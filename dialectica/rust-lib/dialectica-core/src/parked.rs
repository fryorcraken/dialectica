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

use crate::shedding::{choose, Victim};
use rusqlite::{Connection, TransactionBehavior};
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
/// `park-pending-inbound`'s design, Decision 4, has the reasoning. In short:
///
/// - **256 messages per channel**, the waiting payloads' own bound
///   ([`crate::delivery::INBOUND_BOUND`]): one channel may park as many
///   messages as could wait to be taken at once.
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
///
/// Two variants and not a flag beside a count: a payload that is discarded
/// costs no message already parked (`op-transport`: "If the payload being parked
/// is chosen … every message already parked MUST be kept"), so "discarded, and
/// something evicted for it" has no value to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParkOutcome {
    /// The payload is parked, and this many messages that were already parked
    /// were discarded to make room for it.
    Parked { evicted: usize },
    /// The payload is discarded, and every message already parked is kept.
    Discarded,
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
        // What a review has decided leaves neither its bytes nor its size in the
        // file. `secure_delete` overwrites freed content rather than leaving it in
        // a page's free space; it is per connection, so it is set on every open.
        // `auto_vacuum = FULL` returns freed pages to the file system at each
        // commit, so the file shrinks back. SQLite fixes `auto_vacuum` when the
        // first table is created, and ignores it inside a transaction, so it is
        // set here, before `ensure_schema`'s; on a file that already has the
        // table it changes nothing. Without them the file kept a flood's size,
        // and its freed pages the attacker's bytes, after every review had
        // decided them.
        conn.execute_batch("PRAGMA secure_delete = ON; PRAGMA auto_vacuum = FULL;")
            .map_err(storage)?;
        let found = Self::ensure_schema(&conn)?;
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

    /// Make sure the file has a schema — creating it if the file has none — and
    /// return the layout version the file stamps, for the caller to check.
    ///
    /// **The op log's shape, `BEGIN IMMEDIATE` and the version read under the
    /// write lock**, though only the processor thread opens this file: it costs
    /// nothing, and the day a second thread opens it is not a day anyone will
    /// think to come back here (`sender.rs`'s `create_schema` names the race).
    /// `PRAGMA user_version` is last, the commit point of the layout claim.
    fn ensure_schema(conn: &Connection) -> Result<i32, ParkError> {
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
    ) -> Result<ParkOutcome, ParkError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let rows = held_rows(&tx)?;
        let evict = match plan_park(&rows, channel_id, payload.len(), bounds) {
            // Nothing to write: the transaction rolls back on drop.
            Plan::Discard => return Ok(ParkOutcome::Discarded),
            Plan::Park { evict } => evict,
        };
        for seq in &evict {
            tx.execute("DELETE FROM parked WHERE seq = ?1", [seq])
                .map_err(storage)?;
        }
        tx.execute(
            "INSERT INTO parked (channel, payload) VALUES (?1, ?2)",
            rusqlite::params![channel_id, payload],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(ParkOutcome::Parked {
            evicted: evict.len(),
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
        let payloads: Vec<Vec<u8>> = tx
            .prepare("SELECT payload FROM parked WHERE channel = ?1 ORDER BY seq")
            .and_then(|mut select| {
                select
                    .query_map([channel_id], |row| row.get(0))?
                    .collect()
            })
            .map_err(storage)?;
        let taken = payloads
            .into_iter()
            .map(|payload| Parked {
                channel_id: channel_id.to_string(),
                payload,
            })
            .collect();
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

/// What parking one payload will do.
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    /// Park it, discarding these held rows (by `seq`) first.
    Park { evict: Vec<i64> },
    /// Discard it, and keep every held row.
    Discard,
}

/// The discard rule of `op-transport`'s "Parked messages are bounded per channel
/// and in total", as a pure function of what is held.
///
/// 1. **Its own channel first.** A payload that would put its own channel over
///    its count or byte bound is discarded, and nothing held is.
/// 2. **Then the total count, then the total bytes.** While parking would put
///    the parked messages over a total, [`choose`] picks a message — counting
///    the payload with its own channel, and leaving out every message already
///    chosen — until the bound would hold: messages for the count bound first,
///    then for the byte bound.
/// 3. **If the payload itself is chosen, for either bound, it is discarded and
///    every held row is kept**, a row chosen before it included. Only a plan
///    that parks the payload evicts anything: a payload that is lost anyway
///    costs no message already parked.
///
/// `rows` is in hand-over order. The payload is newer than every row.
fn plan_park(rows: &[Row], channel: &str, bytes: usize, bounds: &ParkBounds) -> Plan {
    let own: Vec<&Row> = rows.iter().filter(|r| r.channel == channel).collect();
    let own_bytes: usize = own.iter().map(|r| r.bytes).fold(0, usize::saturating_add);
    if own.len().saturating_add(1) > bounds.per_channel_count
        || own_bytes.saturating_add(bytes) > bounds.per_channel_bytes
    {
        return Plan::Discard;
    }

    let totals = [
        Total {
            measure: |_| 1,
            arriving: 1,
            bound: bounds.total_count as u64,
        },
        Total {
            measure: |r| r.bytes as u64,
            arriving: bytes as u64,
            bound: bounds.total_bytes as u64,
        },
    ];
    let mut kept: Vec<&Row> = rows.iter().collect();
    let mut evict = Vec::new();
    for total in &totals {
        if !total.make_room(&mut kept, &mut evict, channel) {
            return Plan::Discard;
        }
    }
    Plan::Park { evict }
}

/// One total bound, as shedding sees it.
struct Total {
    /// What the bound counts of one held row: 1, or its payload bytes.
    measure: fn(&Row) -> u64,
    /// What it counts of the payload being parked.
    arriving: u64,
    bound: u64,
}

impl Total {
    /// Choose held rows for this bound until parking the payload would keep it,
    /// moving each from `kept` to `evict`, in hand-over order.
    ///
    /// `false` when the payload itself is chosen: the caller then discards it and
    /// evicts nothing, whatever `evict` holds by then.
    fn make_room(&self, kept: &mut Vec<&Row>, evict: &mut Vec<i64>, channel: &str) -> bool {
        loop {
            let held = kept
                .iter()
                .map(|r| (self.measure)(r))
                .fold(0, u64::saturating_add);
            if held.saturating_add(self.arriving) <= self.bound {
                return true;
            }
            let held_rows = kept.iter().map(|r| (r.channel.as_str(), (self.measure)(r)));
            let at = match choose(held_rows, (channel, self.arriving)) {
                Victim::Arrival => return false,
                // `kept` is in hand-over order, so its last row on the channel
                // chosen is that channel's newest.
                Victim::NewestOf(victim) => kept.iter().rposition(|r| r.channel == victim),
            };
            match at {
                Some(at) => evict.push(kept.remove(at).seq),
                // Unreachable: `choose` names only a channel holding a row.
                // Discarding the payload is the answer that keeps the bound and
                // loses nothing already parked.
                None => return false,
            }
        }
    }
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

    impl ParkOutcome {
        fn is_parked(&self) -> bool {
            matches!(self, ParkOutcome::Parked { .. })
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
        // Change these only with `park-pending-inbound`'s design, Decision 4.
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
            assert!(store.park(channel, payload, &PARK_BOUNDS).unwrap().is_parked());
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
            assert!(store.park("a", &[n], &SMALL).unwrap().is_parked());
        }
        assert_eq!(
            store.park("a", b"one more", &SMALL).unwrap(),
            ParkOutcome::Discarded
        );
        assert_eq!(payloads(&mut store, "a"), [[0], [1], [2]]);
    }

    #[test]
    fn a_channel_at_its_byte_bound_discards_the_arrival() {
        let mut store = ParkedStore::in_memory().unwrap();
        store.park("a", &[0; 200], &SMALL).unwrap();
        // 200 + 101 > 300, though 101 fits on its own and the count is under 3.
        assert!(!store.park("a", &[1; 101], &SMALL).unwrap().is_parked());
        assert!(store.park("a", &[1; 100], &SMALL).unwrap().is_parked());
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
            ParkOutcome::Parked { evicted: 1 }
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
            ParkOutcome::Discarded
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
        assert!(store.park("c", b"c1", &bounds).unwrap().is_parked());
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
        assert!(store.park("c", &[5; 100], &SMALL).unwrap().is_parked());
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
    fn a_payload_discarded_for_the_byte_total_evicts_nothing_the_count_total_chose() {
        // `op-transport`, "Parked messages are bounded per channel and in total":
        // "If the payload being parked is chosen, for either total bound, that
        // payload MUST be discarded and every message already parked MUST be
        // kept, a message chosen before it included."
        //
        // Held: `a` 1 B; `b` 150 B twice; `d` 99 B twice — five messages, the
        // count total, and 499 B. `c` brings 300 B, within its own channel's
        // bounds. The count total chooses `d`'s newest (tied with `b` at two,
        // `d`'s newest is the later). The byte total then finds `c` tied with `b`
        // at 300 B, and the arrival is the newest of all: `c` is chosen. Red
        // while the count total's choice is applied anyway: `d` loses a message
        // for an arrival that is not parked.
        let mut store = ParkedStore::in_memory().unwrap();
        for (channel, bytes) in [("a", 1), ("b", 150), ("b", 150), ("d", 99), ("d", 99)] {
            assert!(store.park(channel, &vec![0; bytes], &SMALL).unwrap().is_parked());
        }
        assert_eq!(
            store.park("c", &[1; 300], &SMALL).unwrap(),
            ParkOutcome::Discarded
        );
        for (channel, count) in [("a", 1), ("b", 2), ("c", 0), ("d", 2)] {
            assert_eq!(store.count_on(channel).unwrap(), count, "on `{channel}`");
        }
    }

    #[test]
    fn a_count_eviction_is_not_kept_for_a_payload_the_byte_total_then_discards() {
        // The correctness review's case, as a plan over held rows: three 30-byte
        // messages on `a`, a 90-byte payload for `b`, totals of three messages and
        // 100 bytes. The count total chooses `a`'s newest; the byte total then
        // finds `b`, at 90 bytes, holding more than `a` at 60.
        let bounds = ParkBounds {
            per_channel_count: 5,
            per_channel_bytes: 100,
            total_count: 3,
            total_bytes: 100,
        };
        let rows: Vec<Row> = (1..=3)
            .map(|seq| Row {
                seq,
                channel: "a".to_string(),
                bytes: 30,
            })
            .collect();
        assert_eq!(plan_park(&rows, "b", 90, &bounds), Plan::Discard);
        // And when the byte total holds after the count total's choice, that
        // choice is applied: 60 + 40 is within 100.
        assert_eq!(
            plan_park(&rows, "b", 40, &bounds),
            Plan::Park { evict: vec![3] }
        );
    }

    #[test]
    fn a_review_leaves_neither_the_size_nor_the_bytes_of_what_it_took_in_the_file() {
        // NO SPEC: the spec says what is parked and when it stops being parked,
        // not what the file keeps afterwards. This holds `auto_vacuum = FULL`
        // (the file shrinks back) and `secure_delete` (a freed row's bytes are
        // overwritten, in a page still holding another channel's row), so a
        // flood's bytes do not outlive the reviews that decided them.
        let dir = TempDir::new("shrinks");
        let path = parked_path_in(&dir.0);
        let mut store = ParkedStore::open(&path).unwrap();
        let small = b"zzyzx-small-parked";
        let large_marker = b"zzyzx-large-parked";
        let large: Vec<u8> = large_marker.iter().copied().cycle().take(1024).collect();
        store.park("a", small, &PARK_BOUNDS).unwrap();
        store.park("b", b"stays parked", &PARK_BOUNDS).unwrap();
        for _ in 0..200 {
            store.park("a", &large, &PARK_BOUNDS).unwrap();
        }
        let full = std::fs::metadata(&path).unwrap().len();

        assert_eq!(store.take_channel("a").unwrap().len(), 201);

        let after = std::fs::metadata(&path).unwrap().len();
        assert!(after * 10 < full, "{full} bytes before, {after} after");
        let bytes = std::fs::read(&path).unwrap();
        for marker in [&small[..], &large_marker[..]] {
            assert!(
                !bytes.windows(marker.len()).any(|w| w == marker),
                "{} is still in the file",
                String::from_utf8_lossy(marker)
            );
        }
        assert_eq!(payloads(&mut store, "b"), [b"stays parked"]);
    }
}
