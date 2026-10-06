//! Inbound messages parked while their channel is being opened.
//!
//! # What a parked message is, and what it is not
//!
//! Delivery can hand over a message on a channel before its answer to the
//! creation reaches this peer. Judged then, the message would be refused as an
//! unknown channel and lost, because SDS already counts it delivered. So the
//! delivery wiring parks it here, and decides it once the open settles
//! (`crate::delivery`, the processor's reviews).
//!
//! **Everything here is unverified and attacker-controlled.** Nothing has
//! decoded or verified a parked payload, so it is kept apart from the op log, in
//! a file of its own: no read of the op log can return it, it moves no Lamport
//! clock, and a judgement of an arriving op cannot find it "already held". That
//! is by construction — the op log never sees these rows — not by a filter a
//! later read could forget.
//!
//! # What a row keeps
//!
//! The payload, the channel identifier it arrived on, and its place in the order
//! delivery handed messages over. **Never the sender identifier or the event's
//! timestamp**: neither decides anything, and both are text the sender chose.
//! The table has no column for either, so there is nowhere to put them.
//!
//! # A file of its own
//!
//! For the reason `sender.rs` gives: each store stamps a layout version and
//! checks its own columns, so a table added to an existing file would need a
//! version bump that makes every existing store unopenable. A separate file adds
//! without migrating. It is also what keeps a parked payload out of reach of
//! every op-log read.
//!
//! # Nothing here panics
//!
//! Outside `#[cfg(test)]` there is no `unwrap`, `expect` or indexing; every
//! `rusqlite` failure becomes a [`ParkError`].

use rusqlite::{Connection, TransactionBehavior};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The storage layout this build writes and understands for parked messages.
///
/// Independent of the other stores' versions. Pinned by a hardcoded test,
/// because `cargo mutants` does not mutate a `const`.
pub const PARKED_LAYOUT_VERSION: i32 = 1;

/// The four bounds on what is parked.
///
/// `op-transport`, "Parked messages are bounded per channel and in total",
/// requires four fixed bounds, each per-channel one no greater than its total,
/// and each byte bound at least the 150 KiB message limit. The values are this
/// design's: see [`PARK_BOUNDS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParkBounds {
    /// Messages parked on one channel.
    pub per_channel_count: usize,
    /// Payload bytes parked on one channel.
    pub per_channel_bytes: usize,
    /// Messages parked in all.
    pub total_count: usize,
    /// Payload bytes parked in all.
    pub total_bytes: usize,
}

/// The bounds the running module parks by.
///
/// `design.md` Decision 4 has the reasoning. In short:
///
/// - **256 messages per channel**, the waiting payloads' own bound
///   ([`crate::delivery::INBOUND_BOUND`]): an open that settles held reviews at
///   most what one full queue could have handed it.
/// - **8 MiB per channel**: a review reads one channel's parked messages into
///   memory at once, so this is what a review holds. 54 payloads at the 150 KiB
///   limit; far more than 256 posts of ordinary size, so for ordinary traffic
///   the count binds first.
/// - **1024 messages and 32 MiB in all**: four channels at their own bounds, on
///   disk. Parked messages live only while an open is in flight, so the total
///   bounds what a peer opening several Stoas at once (a restart) can be made to
///   hold.
pub const PARK_BOUNDS: ParkBounds = ParkBounds {
    per_channel_count: 256,
    per_channel_bytes: 8 * 1024 * 1024,
    total_count: 1024,
    total_bytes: 32 * 1024 * 1024,
};

// The orders `op-transport` requires of the four bounds, held at compile time.
// They cannot hold the 150 KiB itself — lowering `MAX_MESSAGE_BYTES` compiles —
// which `the_park_bounds_are_ordered_as_required` does against a literal.
const _: () = assert!(PARK_BOUNDS.per_channel_count <= PARK_BOUNDS.total_count);
const _: () = assert!(PARK_BOUNDS.per_channel_bytes <= PARK_BOUNDS.total_bytes);
const _: () = assert!(PARK_BOUNDS.per_channel_bytes >= crate::transport::MAX_MESSAGE_BYTES);
const _: () = assert!(PARK_BOUNDS.total_bytes >= crate::transport::MAX_MESSAGE_BYTES);

/// One parked message, as a review takes it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parked {
    pub channel_id: String,
    pub payload: Vec<u8>,
}

/// What parking one payload did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parking {
    /// Whether the payload itself is parked. `false` is a discard of the payload.
    pub parked: bool,
    /// How many messages that were already parked were discarded to make room.
    pub evicted: usize,
}

/// Why the parked messages could not be used.
#[derive(Debug, PartialEq, Eq)]
pub enum ParkError {
    /// The store could not be reached, opened, read or written.
    Storage(String),
    /// The store declares a layout this build does not understand.
    UnknownLayoutVersion { found: i32, expected: i32 },
    /// The store stamps this build's layout number and does not have that layout.
    LayoutDoesNotMatchItsVersion { version: i32, why: String },
}

impl fmt::Display for ParkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParkError::Storage(why) => {
                write!(f, "the parked-message store could not be used: {why}")
            }
            ParkError::UnknownLayoutVersion { found, expected } => write!(
                f,
                "the parked-message store was written with storage layout version \
                 {found}, and this build understands version {expected}"
            ),
            ParkError::LayoutDoesNotMatchItsVersion { version, why } => write!(
                f,
                "the parked-message store declares storage layout version {version} \
                 but does not have that layout: {why}"
            ),
        }
    }
}

impl std::error::Error for ParkError {}

/// The parked messages, on disk.
#[derive(Debug)]
pub struct ParkedStore {
    conn: Connection,
}

impl ParkedStore {
    /// Open or create the store at `path`, refusing a layout this build cannot read.
    pub fn open(path: &Path) -> Result<Self, ParkError> {
        let conn = Connection::open(path).map_err(storage)?;
        Self::from_connection(conn)
    }

    /// An ephemeral store with no file behind it: this exact code over SQLite's
    /// `:memory:`, for tests not about persistence.
    pub fn in_memory() -> Result<Self, ParkError> {
        let conn = Connection::open_in_memory().map_err(storage)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, ParkError> {
        let found = Self::create_schema(&conn)?;
        if found != PARKED_LAYOUT_VERSION {
            return Err(ParkError::UnknownLayoutVersion {
                found,
                expected: PARKED_LAYOUT_VERSION,
            });
        }
        Self::check_layout(&conn)?;
        Ok(ParkedStore { conn })
    }

    /// Prove the store has the layout its `user_version` claims, by naming the
    /// columns — `membership.rs`'s `check_layout`, for the same reason.
    fn check_layout(conn: &Connection) -> Result<(), ParkError> {
        conn.query_row(
            "SELECT seq, channel, payload FROM parked LIMIT 0",
            [],
            |_| Ok(()),
        )
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(()),
            other => Err(ParkError::LayoutDoesNotMatchItsVersion {
                version: PARKED_LAYOUT_VERSION,
                why: other.to_string(),
            }),
        })
    }

    /// Create the schema if the file has none, returning the layout version the
    /// file then stamps.
    ///
    /// **The op log's shape, `BEGIN IMMEDIATE` and the version read under the
    /// write lock**, though only the processor thread opens this file: it costs
    /// nothing, and the day a second thread opens it is not a day anyone will
    /// think to come back here (`sender.rs`'s `create_schema` names the race).
    /// `PRAGMA user_version` is last, the commit point of the layout claim.
    fn create_schema(conn: &Connection) -> Result<i32, ParkError> {
        conn.execute_batch("BEGIN IMMEDIATE;").map_err(storage)?;
        let found: i32 = match conn.query_row("PRAGMA user_version", [], |row| row.get(0)) {
            Ok(found) => found,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(storage(e));
            }
        };
        if found != 0 {
            let _ = conn.execute_batch("COMMIT;");
            return Ok(found);
        }
        let result = conn.execute_batch(&format!(
            "CREATE TABLE parked (
                 -- The order delivery handed messages over. A rowid alias, so a
                 -- new row is numbered past every row present: among the rows
                 -- held, a larger `seq` was handed over later, which is all the
                 -- order and the shedding rule ask of it.
                 seq      INTEGER PRIMARY KEY,
                 -- The channel identifier it arrived on, as delivery gave it.
                 channel  TEXT NOT NULL,
                 -- The payload, verbatim and unjudged.
                 payload  BLOB NOT NULL
                 -- No sender identifier and no event timestamp, deliberately:
                 -- see the module's documentation.
             ) STRICT;
             CREATE INDEX parked_by_channel ON parked (channel, seq);
             PRAGMA user_version = {PARKED_LAYOUT_VERSION};
             COMMIT;"
        ));
        if let Err(e) = result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(storage(e));
        }
        Ok(PARKED_LAYOUT_VERSION)
    }

    /// Park one payload on `channel_id`, discarding under `bounds` as
    /// [`plan_park`] decides, in one transaction.
    ///
    /// Either the whole plan is applied — the evictions and the payload's row
    /// together — or, on any error, nothing is: the transaction rolls back on
    /// drop.
    pub fn park(
        &mut self,
        channel_id: &str,
        payload: &[u8],
        bounds: &ParkBounds,
    ) -> Result<Parking, ParkError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let rows = held_rows(&tx)?;
        let plan = plan_park(&rows, channel_id, payload.len(), bounds);
        for seq in &plan.evict {
            tx.execute("DELETE FROM parked WHERE seq = ?1", [seq])
                .map_err(storage)?;
        }
        if plan.park {
            tx.execute(
                "INSERT INTO parked (channel, payload) VALUES (?1, ?2)",
                rusqlite::params![channel_id, payload],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        Ok(Parking {
            parked: plan.park,
            evicted: plan.evict.len(),
        })
    }

    /// Remove every message parked on `channel_id` and return them, in the order
    /// they were handed over.
    ///
    /// **Removed and returned in one transaction**, so a review decides each
    /// message it reads and no later review can read it again: "decided by
    /// exactly one review" holds by the store, whatever the review then does.
    /// If anything fails, nothing is removed and the messages stay parked for
    /// the channel's next review, as `op-transport` requires.
    pub fn take_channel(&mut self, channel_id: &str) -> Result<Vec<Parked>, ParkError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let taken = {
            let mut select = tx
                .prepare("SELECT payload FROM parked WHERE channel = ?1 ORDER BY seq")
                .map_err(storage)?;
            let rows = select
                .query_map([channel_id], |row| row.get::<_, Vec<u8>>(0))
                .map_err(storage)?;
            let mut taken = Vec::new();
            for payload in rows {
                taken.push(Parked {
                    channel_id: channel_id.to_string(),
                    payload: payload.map_err(storage)?,
                });
            }
            taken
        };
        tx.execute("DELETE FROM parked WHERE channel = ?1", [channel_id])
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(taken)
    }

    /// Every channel identifier something is parked on, once each.
    pub fn channels(&self) -> Result<Vec<String>, ParkError> {
        let mut select = self
            .conn
            .prepare("SELECT DISTINCT channel FROM parked ORDER BY channel")
            .map_err(storage)?;
        let rows = select
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        rows.collect::<Result<_, _>>().map_err(storage)
    }

    /// How many messages are parked on `channel_id`.
    pub fn count_on(&self, channel_id: &str) -> Result<usize, ParkError> {
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM parked WHERE channel = ?1",
                [channel_id],
                |row| row.get(0),
            )
            .map_err(storage)?;
        Ok(usize::try_from(n).unwrap_or(0))
    }
}

/// One parked row as the shedding plan sees it: its place, its channel, its size.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    /// The row's `seq`, as SQLite holds it. Never negative: a rowid this table
    /// assigns starts at 1.
    seq: i64,
    channel: String,
    bytes: usize,
}

fn held_rows(conn: &Connection) -> Result<Vec<Row>, ParkError> {
    let mut select = conn
        .prepare("SELECT seq, channel, LENGTH(payload) FROM parked ORDER BY seq")
        .map_err(storage)?;
    let rows = select
        .query_map([], |row| {
            Ok(Row {
                seq: row.get(0)?,
                channel: row.get(1)?,
                bytes: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
            })
        })
        .map_err(storage)?;
    rows.collect::<Result<_, _>>().map_err(storage)
}

/// What parking one payload will do: park it or not, and which held rows to
/// discard first.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    park: bool,
    evict: Vec<i64>,
}

/// The discard rule of `op-transport`'s "Parked messages are bounded per channel
/// and in total", as a pure function of what is held.
///
/// 1. **Its own channel first.** A payload that would put its own channel over
///    its count or byte bound is discarded, and nothing held is.
/// 2. **Then the total count, then the total bytes.** While parking would put the
///    parked messages over a total, the channel [`shed`] picks — counting the
///    payload with its own channel — gives up its newest message. When that is
///    the payload's own channel, its newest is the payload itself: the payload is
///    discarded and nothing more is.
///
/// `rows` is in hand-over order. The payload is newer than every row.
fn plan_park(rows: &[Row], channel: &str, bytes: usize, bounds: &ParkBounds) -> Plan {
    let own: Vec<&Row> = rows.iter().filter(|r| r.channel == channel).collect();
    let own_bytes: usize = own.iter().map(|r| r.bytes).fold(0, usize::saturating_add);
    if own.len().saturating_add(1) > bounds.per_channel_count
        || own_bytes.saturating_add(bytes) > bounds.per_channel_bytes
    {
        return Plan {
            park: false,
            evict: Vec::new(),
        };
    }

    let mut kept: Vec<&Row> = rows.iter().collect();
    let mut evict = Vec::new();
    // The count bound, then the byte bound, each with its own measure.
    let measures: [(fn(&Row) -> u64, u64, usize); 2] = [
        (|_| 1, 1, bounds.total_count),
        (
            |r| r.bytes as u64,
            bytes as u64,
            bounds.total_bytes,
        ),
    ];
    for (measure, arriving, bound) in measures {
        loop {
            let held: u64 = kept.iter().map(|r| measure(r)).sum();
            if held.saturating_add(arriving) <= bound as u64 {
                break;
            }
            let loads = channel_loads(&kept, measure, channel, arriving);
            match shed(loads.iter().map(|(c, l)| (c.as_str(), *l))) {
                Some(victim) if victim != channel => {
                    let newest = kept
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.channel == victim)
                        .max_by_key(|(_, r)| r.seq)
                        .map(|(at, r)| (at, r.seq));
                    match newest {
                        Some((at, seq)) => {
                            kept.remove(at);
                            evict.push(seq);
                        }
                        // Unreachable: `shed` names only a channel with a load.
                        None => return Plan { park: false, evict },
                    }
                }
                // The payload's own channel, or nothing to shed at all.
                _ => return Plan { park: false, evict },
            }
        }
    }
    Plan { park: true, evict }
}

/// Each channel's load under `measure`, with the arriving payload counted on its
/// own channel as the newest of all.
fn channel_loads(
    kept: &[&Row],
    measure: fn(&Row) -> u64,
    arriving_channel: &str,
    arriving: u64,
) -> Vec<(String, Load)> {
    let mut loads: HashMap<&str, Load> = HashMap::new();
    for row in kept {
        let load = loads.entry(row.channel.as_str()).or_default();
        load.measure = load.measure.saturating_add(measure(row));
        load.newest = load.newest.max(u64::try_from(row.seq).unwrap_or(0));
    }
    let own = loads.entry(arriving_channel).or_default();
    own.measure = own.measure.saturating_add(arriving);
    own.newest = u64::MAX;
    loads.into_iter().map(|(c, l)| (c.to_string(), l)).collect()
}

/// How much a channel holds, under whatever a bound counts, and when its newest
/// message was handed over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Load {
    pub measure: u64,
    pub newest: u64,
}

/// Which channel gives up its newest message when a total bound is over: the
/// channel holding the most, and among several holding the most, the one whose
/// newest message was handed over latest.
///
/// **The arriving message is counted with its own channel, as that channel's
/// newest — the newest of all.** So "the arriving payload's own channel is chosen
/// if it is among those holding the most", which `op-transport` states for both
/// the waiting payloads and the parked messages, is this same tie-break and not
/// a second rule: the arrival's channel always has the latest newest. And when
/// the channel chosen is the arrival's, its newest — the one given up — is the
/// arrival.
///
/// One function for both bounds, because the spec gives them one rule: "This is
/// the rule 'Parked messages are bounded per channel and in total' applies to its
/// total bounds", in the waiting payloads' requirement.
pub(crate) fn shed<'a>(loads: impl IntoIterator<Item = (&'a str, Load)>) -> Option<&'a str> {
    loads
        .into_iter()
        .max_by(|(_, a), (_, b)| {
            a.measure
                .cmp(&b.measure)
                .then_with(|| a.newest.cmp(&b.newest))
        })
        .map(|(channel, _)| channel)
}

/// This store's file name inside a host-supplied directory, beside
/// [`crate::sender::sender_path_in`] and [`crate::log::op_log_path_in`].
pub fn parked_path_in(dir: &Path) -> PathBuf {
    dir.join("parked.sqlite")
}

fn storage(e: rusqlite::Error) -> ParkError {
    ParkError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: ParkBounds = ParkBounds {
        per_channel_count: 3,
        per_channel_bytes: 300,
        total_count: 5,
        total_bytes: 500,
    };

    /// A fresh temporary directory, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("dialectica-parked-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn payloads(store: &mut ParkedStore, channel: &str) -> Vec<Vec<u8>> {
        store
            .take_channel(channel)
            .unwrap()
            .into_iter()
            .map(|p| p.payload)
            .collect()
    }

    #[test]
    fn the_layout_version_and_file_name_are_pinned() {
        assert_eq!(PARKED_LAYOUT_VERSION, 1);
        assert_eq!(
            parked_path_in(Path::new("/a/dir")),
            PathBuf::from("/a/dir/parked.sqlite")
        );
    }

    #[test]
    fn the_park_bounds_are_pinned() {
        // Known answers, hardcoded: `cargo mutants` does not mutate a `const`.
        // Change these only with design.md's Decision on the bounds.
        assert_eq!(
            PARK_BOUNDS,
            ParkBounds {
                per_channel_count: 256,
                per_channel_bytes: 8_388_608,
                total_count: 1024,
                total_bytes: 33_554_432,
            }
        );
    }

    #[test]
    fn the_park_bounds_are_ordered_as_required() {
        // `op-transport`, scenario "The bounds are ordered as required", with the
        // message limit written as a literal, independently of the code.
        let limit = 150 * 1024;
        let b = PARK_BOUNDS;
        assert!(b.per_channel_count <= b.total_count);
        assert!(b.per_channel_bytes <= b.total_bytes);
        assert!(b.per_channel_bytes >= limit);
        assert!(b.total_bytes >= limit);
    }

    #[test]
    fn a_review_takes_a_channels_messages_in_hand_over_order_and_leaves_none() {
        let mut store = ParkedStore::in_memory().unwrap();
        for (channel, payload) in [("a", b"1"), ("b", b"x"), ("a", b"2"), ("a", b"3")] {
            assert!(store.park(channel, payload, &PARK_BOUNDS).unwrap().parked);
        }
        assert_eq!(payloads(&mut store, "a"), [b"1", b"2", b"3"]);
        assert_eq!(store.count_on("a").unwrap(), 0);
        assert!(payloads(&mut store, "a").is_empty(), "taken twice");
        assert_eq!(store.channels().unwrap(), ["b"]);
    }

    #[test]
    fn parked_messages_survive_a_reopen() {
        let dir = TempDir::new("reopen");
        let path = parked_path_in(&dir.0);
        ParkedStore::open(&path)
            .unwrap()
            .park("a", b"kept", &PARK_BOUNDS)
            .unwrap();
        let mut again = ParkedStore::open(&path).unwrap();
        assert_eq!(payloads(&mut again, "a"), [b"kept"]);
    }

    #[test]
    fn nothing_parked_carries_the_sender_identifier_or_the_timestamp() {
        // `op-transport`, scenario "Nothing parked carries the sender identifier
        // or the event's timestamp", read off the file's bytes: the table has no
        // column for either, so neither can be in it. `park` takes neither, which
        // the compiler holds; this holds the schema.
        let dir = TempDir::new("no-sender");
        let path = parked_path_in(&dir.0);
        ParkedStore::open(&path)
            .unwrap()
            .park("a", b"the payload", &PARK_BOUNDS)
            .unwrap();
        let conn = Connection::open(&path).unwrap();
        let mut columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('parked')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        columns.sort();
        assert_eq!(columns, ["channel", "payload", "seq"]);
    }

    #[test]
    fn a_channel_at_its_count_bound_discards_the_arrival_and_keeps_what_it_parked() {
        let mut store = ParkedStore::in_memory().unwrap();
        for n in 0..3u8 {
            assert!(store.park("a", &[n], &SMALL).unwrap().parked);
        }
        assert_eq!(
            store.park("a", b"one more", &SMALL).unwrap(),
            Parking {
                parked: false,
                evicted: 0
            }
        );
        assert_eq!(payloads(&mut store, "a"), [[0], [1], [2]]);
    }

    #[test]
    fn a_channel_at_its_byte_bound_discards_the_arrival() {
        let mut store = ParkedStore::in_memory().unwrap();
        store.park("a", &[0; 200], &SMALL).unwrap();
        // 200 + 101 > 300, though 101 fits on its own and the count is under 3.
        assert!(!store.park("a", &[1; 101], &SMALL).unwrap().parked);
        assert!(store.park("a", &[1; 100], &SMALL).unwrap().parked);
        assert_eq!(store.count_on("a").unwrap(), 2);
    }

    #[test]
    fn over_the_total_count_the_channel_holding_the_most_gives_up_its_newest() {
        let mut store = ParkedStore::in_memory().unwrap();
        for p in [b"a1", b"a2", b"a3"] {
            store.park("a", p, &SMALL).unwrap();
        }
        store.park("b", b"b1", &SMALL).unwrap();
        store.park("b", b"b2", &SMALL).unwrap(); // the total, 5
        assert_eq!(
            store.park("c", b"c1", &SMALL).unwrap(),
            Parking {
                parked: true,
                evicted: 1
            }
        );
        assert_eq!(payloads(&mut store, "a"), [b"a1", b"a2"]);
        assert_eq!(payloads(&mut store, "b"), [b"b1", b"b2"]);
        assert_eq!(payloads(&mut store, "c"), [b"c1"]);
    }

    #[test]
    fn the_arriving_payloads_channel_loses_a_tie_for_the_most_parked() {
        let mut store = ParkedStore::in_memory().unwrap();
        for p in [b"a1", b"a2"] {
            store.park("a", p, &SMALL).unwrap();
        }
        for p in [b"b1", b"b2", b"b3"] {
            store.park("b", p, &SMALL).unwrap(); // the total, 5
        }
        // Counting the arrival, `a` holds 3 and `b` holds 3.
        assert_eq!(
            store.park("a", b"a3", &SMALL).unwrap(),
            Parking {
                parked: false,
                evicted: 0
            }
        );
        assert_eq!(payloads(&mut store, "a"), [b"a1", b"a2"]);
        assert_eq!(payloads(&mut store, "b"), [b"b1", b"b2", b"b3"]);
    }

    #[test]
    fn among_other_channels_tied_for_the_most_the_one_with_the_latest_newest_gives_it_up() {
        // No scenario pins this tie-break between two channels neither of which
        // is the arrival's; the requirement's text does ("otherwise the one among
        // them whose newest parked message was handed over latest").
        let bounds = ParkBounds {
            total_count: 4,
            ..SMALL
        };
        let mut store = ParkedStore::in_memory().unwrap();
        store.park("a", b"a1", &bounds).unwrap();
        store.park("b", b"b1", &bounds).unwrap();
        store.park("b", b"b2", &bounds).unwrap();
        store.park("a", b"a2", &bounds).unwrap(); // a's newest is the latest
        assert!(store.park("c", b"c1", &bounds).unwrap().parked);
        assert_eq!(payloads(&mut store, "a"), [b"a1"]);
        assert_eq!(payloads(&mut store, "b"), [b"b1", b"b2"]);
    }

    #[test]
    fn over_the_total_bytes_the_channel_holding_the_most_bytes_gives_up_its_newest() {
        let mut store = ParkedStore::in_memory().unwrap();
        store.park("a", &[1; 150], &SMALL).unwrap();
        store.park("a", &[2; 150], &SMALL).unwrap();
        store.park("b", &[3; 100], &SMALL).unwrap();
        store.park("b", &[4; 50], &SMALL).unwrap(); // 450 of 500
        // `c` holds the fewest messages but 100 more bytes put the total over.
        assert!(store.park("c", &[5; 100], &SMALL).unwrap().parked);
        assert_eq!(payloads(&mut store, "a"), [vec![1; 150]]);
        assert_eq!(store.count_on("b").unwrap(), 2);
    }

    #[test]
    fn a_park_that_cannot_be_written_parks_nothing() {
        let mut store = ParkedStore::in_memory().unwrap();
        store.park("a", b"before", &PARK_BOUNDS).unwrap();
        store.conn.execute_batch("PRAGMA query_only = 1").unwrap();
        assert!(matches!(
            store.park("a", b"refused", &PARK_BOUNDS),
            Err(ParkError::Storage(_))
        ));
        assert!(matches!(
            store.take_channel("a"),
            Err(ParkError::Storage(_))
        ));
        store.conn.execute_batch("PRAGMA query_only = 0").unwrap();
        // The failed take removed nothing; the failed park added nothing.
        assert_eq!(payloads(&mut store, "a"), [b"before"]);
    }

    #[test]
    fn a_store_from_a_future_layout_is_refused() {
        let dir = TempDir::new("future");
        let path = parked_path_in(&dir.0);
        drop(ParkedStore::open(&path).unwrap());
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 2")
            .unwrap();
        assert_eq!(
            ParkedStore::open(&path).unwrap_err(),
            ParkError::UnknownLayoutVersion {
                found: 2,
                expected: 1
            }
        );
    }

    #[test]
    fn a_mislabelled_file_is_refused_at_open() {
        let dir = TempDir::new("mislabelled");
        let path = parked_path_in(&dir.0);
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE other (x INTEGER); PRAGMA user_version = 1;")
            .unwrap();
        assert!(matches!(
            ParkedStore::open(&path),
            Err(ParkError::LayoutDoesNotMatchItsVersion { .. })
        ));
    }

    #[test]
    fn every_error_says_what_went_wrong_in_its_own_words() {
        // These renderings fill the storage-failure lines a park or a review
        // logs; `cargo mutants` found `Display` replaced by an empty string
        // surviving every other test. Fragments hardcoded, one per variant.
        let cases = [
            (
                ParkError::Storage("disk on fire".to_string()),
                "parked-message store could not be used: disk on fire",
            ),
            (
                ParkError::UnknownLayoutVersion {
                    found: 7,
                    expected: 1,
                },
                "layout version 7, and this build understands version 1",
            ),
            (
                ParkError::LayoutDoesNotMatchItsVersion {
                    version: 1,
                    why: "no column".to_string(),
                },
                "declares storage layout version 1 but does not have that layout: no column",
            ),
        ];
        for (error, fragment) in cases {
            let text = error.to_string();
            assert!(text.contains(fragment), "`{text}` lacks `{fragment}`");
        }
    }

    #[test]
    fn shed_picks_the_most_then_the_latest_newest() {
        let load = |measure, newest| Load { measure, newest };
        assert_eq!(
            shed([("a", load(3, 1)), ("b", load(2, 9))]),
            Some("a"),
            "the most wins over the latest"
        );
        assert_eq!(
            shed([("a", load(3, 1)), ("b", load(3, 9))]),
            Some("b"),
            "among the most, the latest newest"
        );
        assert_eq!(shed(std::iter::empty()), None);
    }
}
