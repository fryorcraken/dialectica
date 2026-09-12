//! The op log on disk: §3.3's "local SQLite store", and the trait's second
//! implementor.
//!
//! # Why this file exists
//!
//! PLAN.md §3.3: "Each peer is to keep a **local SQLite store** holding every op
//! it has seen". §9 lists "the op log and its SQLite projection, query indexing"
//! as one Phase 1 item, of which [`super::MemoryOpLog`] was half. This is the
//! other half, and it is what makes §4.7's middle durability tier — "everything
//! this peer has ever seen" — buy anything across a restart.
//!
//! It is also the answer to a condition the `op-log` change wrote for itself:
//! *"`MemoryOpLog` is the only implementor of `OpLog`; when `grep 'impl OpLog'`
//! still returns one line at the end of Phase 1, the trait did not earn itself."*
//!
//! # This is the log, not the view
//!
//! §3.3 distinguishes the op log (the authority) from the materialised view (a
//! cache that can be rebuilt by replay). Nothing here derives forum state:
//! there is no thread table, no feed, no reply count. There is one table of ops
//! and the indexes that make the defined read order servable.
//!
//! # What decides nothing, still decides nothing
//!
//! §3.3 puts verification on read, and persistence is not an occasion to change
//! that. [`SqliteOpLog::append`] writes an op whose signature does not verify
//! exactly as it writes one that does, for the three reasons [`super`]'s module
//! documentation gives. A store that refused to write what it could not verify
//! would make a forgery indistinguishable from an op that never arrived — the
//! same defect on disk as in memory.
//!
//! # The order lives in two places now, and a test is what holds them together
//!
//! [`crate::arrival::cmp_ops`] is the definition of the read order. This module
//! re-expresses it as an `ORDER BY` over columns computed at append time, so
//! that the order is **indexable** — which the alternative, loading every
//! matching row and sorting in Rust, structurally is not.
//!
//! That is two definitions of one order with no compiler checking them against
//! each other, and it is the genuine cost of this design. What makes it
//! survivable is
//! `both_implementations_agree_across_every_ordering_branch`: it builds a
//! population crossing every branch of `cmp_ops`, appends it to both logs, and
//! asserts the sequences are identical. A divergence is a failing test naming
//! the pair that disagreed, not a rendering difference in production.
//!
//! **`cmp_ops` remains the definition.** The SQL is an implementation of it.
//!
//! # Nothing here panics
//!
//! PHASE0-FINDINGS §3 measured that an unguarded panic **aborts the module
//! process**, and everything stored arrived from a peer. So there is no
//! `unwrap`, no `expect` and no indexing that could be out of bounds outside
//! `#[cfg(test)]`; every `rusqlite` failure becomes an
//! [`OpLogError`](super::OpLogError).

use super::{Appended, Entry, JoinedStoa, OpLog, OpLogError, Relation, StoaRegistry};
use crate::arrival::{Arrival, MessageId};
use crate::identity::Address;
use crate::op::{OpId, SignedOp};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

/// The storage layout this build writes and understands.
///
/// Held in SQLite's own `PRAGMA user_version`, which is one `INTEGER` the file
/// format already reserves — so a version needs no table of its own and cannot
/// be lost by a schema change that forgot about it.
///
/// **Pinned by a hardcoded assertion**, following `identity.rs`'s wire
/// constants. `cargo mutants` mutates functions and not `const`s, so a wrong
/// version here is invisible to it; this project has already shipped a
/// `VERSION_1` defect that left the whole suite green.
///
/// # Version 2 added the `stoas` table, and the bump is not optional
///
/// The write path needs to know which Stoas this peer created or joined, and to
/// hold each one's **genesis record** — because `wire::list_threads` takes a
/// genesis record and `moderation::Moderators::of` cannot be built without one.
/// A version-1 store has no such table.
///
/// **Bumping is what keeps that legible.** Left at 1, a version-1 file would
/// open `Ok` (the pragma matches) and then fail `check_layout`, reported as
/// `LayoutDoesNotMatchItsVersion` — an error whose whole meaning is "this file
/// was not written by this build and is lying about its layout". A store written
/// by the previous build is not lying; it is an older layout, honestly stamped,
/// and `UnknownLayoutVersion` is the variant that says so. Collapsing the two
/// would make a truthful old store indistinguishable from a tampered one, which
/// is the distinction `OpLogError` spends two variants preserving.
///
/// There is still **no migration**, by design: a version-1 store is refused
/// rather than upgraded. What changed is only which refusal it gets.
pub const LAYOUT_VERSION: i32 = 2;

/// Map a Lamport timestamp onto an ascending sort key that reverses it.
///
/// # Why a mapping rather than `ORDER BY arrival_lamport DESC`
///
/// `cmp_ops` wants descending Lamport among the ops the transport ordered.
/// Reversing at **write** time makes one ascending index serve it, where a
/// `DESC` on the stored value would need its own index to be scanned rather than
/// sorted — and the whole point of materialising a sort key is that §2.5's
/// paginated reads become an index walk.
///
/// # Why not simply negate
///
/// `-(u64::MAX as i64)` does not fit. SQLite's `INTEGER` is an `i64` and a
/// Lamport value is a `u64`, so the map has to be a bijection between the two
/// ranges rather than an arithmetic negation. This one is: `u64::MAX - lamport`
/// reverses within `u64`, and adding `i64::MIN` reinterprets the result across
/// the signed range. `0` maps to `i64::MAX`, `u64::MAX` maps to `i64::MIN`.
///
/// # The sentinel is NOT in this column
///
/// An earlier draft reserved `i64::MAX` here for "the transport did not order
/// this", and it was wrong: `lamport == 0` maps to exactly that value, so a
/// Lamport-0 op would have shared a sort key with every unordered op and
/// interleaved with them by op id. `cmp_ops` requires every ordered op to
/// precede every unordered one **whatever the Lamport value**, and
/// `an_op_the_transport_ordered_beats_one_it_did_not` uses Lamport 0 precisely
/// because that is where a sentinel-based design breaks.
///
/// Every repair inside one `i64` column fails for the same reason — `u64` and
/// `i64` have the same cardinality, so a bijection leaves no spare value. The
/// fix is [`SortKey::ordered`], a separate leading column, which costs one
/// integer per row and makes the partition structural instead of arithmetic.
///
/// Order-reversing across the whole domain is asserted by
/// `the_lamport_sort_key_reverses_the_order_across_the_whole_u64_range`, at the
/// boundaries rather than at convenient middle values.
fn lamport_sort_key(lamport: u64) -> i64 {
    (u64::MAX - lamport).wrapping_add(i64::MIN as u64) as i64
}

/// [`cmp_ops`](crate::arrival::cmp_ops), materialised as three stored columns.
///
/// # Why this is one type rather than three expressions at the insert
///
/// It was three expressions, and **the two-implementation agreement test caught
/// them disagreeing with the comparator.** The bug is worth recording because it
/// is not obvious from either side alone:
///
/// `cmp_ops` short-circuits. Its `(None, None)` Lamport arm goes **straight to
/// the op id** — `a.id.cmp(b.id)` — and never reaches `cmp_tiebreak`, so for two
/// ops the transport did not order, **the message id is not consulted at all.**
/// An `ORDER BY sort_lamport, sort_msg_present DESC, sort_msg, op_id` does
/// consult it, because SQL has no short-circuit: the columns are compared in
/// sequence regardless of what the first one held.
///
/// So an unordered op carrying a message id — which is a real shape, the one an
/// SDS ephemeral message produces — sorted ahead of an unordered op without one,
/// where `cmp_ops` would have ordered the pair by op id. Two honest peers, one
/// storing in memory and one on disk, rendered one thread differently.
///
/// **The fix is to make the message id unreachable for an unordered op rather
/// than to remember not to consult it.** [`SortKey::of`] discards it when there
/// is no Lamport value, so the row that reaches the `ORDER BY` cannot express
/// the distinction SQL would otherwise sort on. That is CLAUDE.md's "put the
/// complexity in the data structure, not the logic": the alternative is a
/// `CASE WHEN` in the `ORDER BY` — a guard that must be spelled identically in
/// four places, three indexes and one query, and that silently degrades an index
/// scan into a sort when it is not.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SortKey {
    /// `0` when the transport ordered this op, `1` when it did not.
    ///
    /// **The leading column, and ascending over it is `cmp_ops`'s partition.**
    /// A column of its own rather than a sentinel inside [`SortKey::lamport`],
    /// because `u64` and `i64` have the same cardinality: a bijection between
    /// them leaves no value spare to mean "absent", and every candidate
    /// sentinel is some real Lamport value's image. `0` before `1` so that
    /// ascending is the right direction for every column in the key, which is
    /// what lets one index serve the whole `ORDER BY`.
    ordered: i64,
    /// Descending Lamport, reversed at write time — see [`lamport_sort_key`].
    /// Meaningless when `ordered` is `1`, and never reached, because the
    /// leading column has already decided.
    lamport: i64,
    /// Whether a message id participates in the order. **Not** whether the
    /// arrival recorded one: an unordered arrival's message id is recorded in
    /// the `arrival_msg` column and deliberately absent from its sort key.
    msg_present: i64,
    msg: Option<Vec<u8>>,
}

impl SortKey {
    /// Derive the four columns from what the transport supplied.
    ///
    /// **The match is on the Lamport value, and the message id is only reachable
    /// inside the `Some` arm.** That shape is the invariant: there is no path
    /// through this function that puts a message id into the sort key of an op
    /// the transport did not order.
    fn of(arrival: &Arrival) -> Self {
        match arrival.lamport() {
            Some(lamport) => {
                let msg = arrival.message_id();
                SortKey {
                    ordered: 0,
                    lamport: lamport_sort_key(lamport),
                    // `cmp_tiebreak` puts an op WITH a message id before one
                    // without, within one Lamport value. Its own column because
                    // the EMPTY message id is a legal value, so no byte string
                    // is "below every byte string and not equal to the empty
                    // one" — collapsing the two would be
                    // `absence_is_not_equal_to_a_zero_lamport_timestamp`'s
                    // defect in another costume.
                    msg_present: i64::from(msg.is_some()),
                    msg: msg.map(|m| m.as_bytes().to_vec()),
                }
            }
            // The degraded branch: `cmp_ops` compares op ids and NOTHING else.
            // Both the Lamport key and the message id are zeroed here rather
            // than stored and then not consulted, because a stored value that
            // must not be read is one an `ORDER BY` will eventually read —
            // which is the defect the agreement test caught.
            None => SortKey {
                ordered: 1,
                lamport: 0,
                msg_present: 0,
                msg: None,
            },
        }
    }
}

/// The op log in a SQLite database.
///
/// # One connection, owned, no pool
///
/// A peer has one store. A connection pool solves a concurrency problem this
/// type does not have, and would be a dependency taken for a shape nothing here
/// exhibits.
///
/// # The file is handed in, never discovered
///
/// [`SqliteOpLog::open`] takes a path, exactly as `Keystore` is handed a file
/// and for the same reason: a fixed path baked into a pure crate is untestable
/// and would mean this crate reading the environment at a moment its caller does
/// not control. The module crate has the host-stamped
/// `instance_persistence_path`.
#[derive(Debug)]
pub struct SqliteOpLog {
    conn: Connection,
}

impl SqliteOpLog {
    /// Open or create a log at `path`, refusing a layout this build cannot read.
    ///
    /// A store whose `PRAGMA user_version` names a layout this build does not
    /// understand is refused with
    /// [`OpLogError::UnknownLayoutVersion`](super::OpLogError::UnknownLayoutVersion),
    /// naming both numbers, and **no op is read from it**. See that variant for
    /// why refusing beats a best-effort read.
    pub fn open(path: &Path) -> Result<Self, OpLogError> {
        let conn = Connection::open(path).map_err(storage)?;
        Self::from_connection(conn)
    }

    /// An ephemeral log with no file behind it.
    ///
    /// **Not a test double, and not a second implementation of
    /// [`super::MemoryOpLog`].** It is this exact code — the same schema, the
    /// same `ORDER BY`, the same decode path — with SQLite's `:memory:` backing
    /// store. It exists so that every behavioural test that is not *about*
    /// persistence can exercise the real SQL without a temporary directory, a
    /// file permission, or a teardown, which is what the `op-log` change gave as
    /// its third reason for deferring SQLite at all.
    ///
    /// Tests that ARE about persistence use [`SqliteOpLog::open`] against a real
    /// file, because this one by construction cannot survive being dropped.
    pub fn in_memory() -> Result<Self, OpLogError> {
        let conn = Connection::open_in_memory().map_err(storage)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, OpLogError> {
        // A fresh database reports 0, which is not a version anything wrote —
        // it is the absence of one, and the only case where creating the schema
        // is correct. Any other unexpected value is a store somebody else's
        // build wrote.
        let found: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;

        if found == 0 {
            Self::create_schema(&conn)?;
        } else if found != LAYOUT_VERSION {
            return Err(OpLogError::UnknownLayoutVersion {
                found,
                expected: LAYOUT_VERSION,
            });
        } else {
            // THE VERSION IS A CLAIM, AND THIS IS THE ONLY THING THAT CHECKS IT.
            //
            // `PRAGMA user_version` is one integer any writer can stamp. A file
            // carrying our number whose `ops` table is missing or altered took
            // the branch above and opened `Ok`, and then EVERY read failed with
            // `Storage("no such table: ops")` — an error naming the disk when
            // the fact is that the file is not the layout it declares. A caller
            // cannot tell those apart, and they call for different responses.
            //
            // This is the same refuse-rather-than-read-hopefully posture the
            // version check itself has, applied to the half the version check
            // cannot see. `create_schema` writes the pragma LAST, so the one
            // file this could false-positive on — a crash midway through
            // creation — has version 0 and takes the create branch instead.
            //
            // `LIMIT 0` deliberately: it prepares and runs the statement, which
            // is what proves the table and its name exist, and reads no row, so
            // the cost does not grow with the store.
            Self::check_layout(&conn)?;
        }

        Ok(SqliteOpLog { conn })
    }

    /// Prove the store actually has the layout its `user_version` claims.
    ///
    /// # Why the version number alone is not enough
    ///
    /// `PRAGMA user_version` is a single integer with no relationship to the
    /// tables beside it. Anything can stamp it. A file carrying this build's
    /// number with no `ops` table is not a hypothetical — it is what a
    /// half-restored backup, a hand-edited store, or a partially-`DROP`ped file
    /// looks like, and without this it opened `Ok` and failed at the first read
    /// as `Storage("no such table: ops")`. That error blames the disk. The disk
    /// is fine; the file is mislabelled, and that is a different thing to be
    /// told.
    ///
    /// # Why the columns and not merely the table
    ///
    /// `SELECT 1 FROM ops` proves a table called `ops` exists and nothing about
    /// its shape, so a store whose columns were renamed or dropped would still
    /// pass. Naming the columns every read depends on is what makes this a
    /// check on the LAYOUT rather than on the name. The list is
    /// [`SELECT_COLUMNS`] plus the ordering columns — exactly what
    /// [`SqliteOpLog::ordered_read`] and [`SqliteOpLog::get`] touch — so a
    /// column either read or ordered on cannot go missing unnoticed.
    ///
    /// `LIMIT 0` reads no row: preparing and running the statement is what
    /// establishes the layout, and the cost does not grow with the store.
    ///
    /// # This is not a migration, and must not become one
    ///
    /// Refusing is the whole behaviour. `design.md` records that there is no
    /// migration path by design, and a check that repaired what it found would
    /// be one — written against a layout nobody has described, over a file this
    /// build did not write.
    fn check_layout(conn: &Connection) -> Result<(), OpLogError> {
        // BOTH tables, because the layout is both. A file with `ops` and no
        // `stoas` is exactly the half-created or hand-edited store this refuses,
        // and checking only `ops` would let it open and then fail at the first
        // `list_stoas` as `Storage("no such table: stoas")` — the disk-blaming
        // error this whole function exists to replace.
        Self::check_table(
            conn,
            &format!(
                "SELECT {SELECT_COLUMNS}, op_id, stoa, target, author, score_epoch,
                        sort_ordered, sort_lamport, sort_msg_present, sort_msg
                 FROM ops LIMIT 0"
            ),
        )?;
        Self::check_table(
            conn,
            "SELECT stoa, genesis_bytes, relation FROM stoas LIMIT 0",
        )
    }

    /// Prepare and run one `LIMIT 0` statement, mapping any failure to the
    /// mislabelled-layout error.
    ///
    /// Split out so the two tables cannot be checked by two slightly different
    /// pieces of error mapping — CLAUDE.md's "a second call site spelling its own
    /// X is how one of them eventually spells it differently", applied to the
    /// `QueryReturnedNoRows`-is-success subtlety below, which is the easiest half
    /// of this to get wrong twice.
    fn check_table(conn: &Connection, sql: &str) -> Result<(), OpLogError> {
        conn.query_row(sql, [], |_| Ok(()))
            // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS
            // case and every other error is the layout being wrong. Matching on
            // it rather than using `optional()` keeps that reading explicit: the
            // question asked is "could this statement be prepared and run", not
            // "did it find anything".
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(()),
                other => Err(OpLogError::LayoutDoesNotMatchItsVersion {
                    version: LAYOUT_VERSION,
                    why: other.to_string(),
                }),
            })
    }

    /// The whole schema, in one place, with every column's reason beside it.
    ///
    /// # Dedup is the `PRIMARY KEY`, and it was chosen for the precondition
    ///
    /// The `op-log` design was careful to record that its `HashMap` acquired the
    /// anti-duplicate property by accident, and warned the next author against
    /// assuming a new storage shape inherits it. **Here the causality runs the
    /// other way, and saying so is the point**: `op_id BLOB PRIMARY KEY` was
    /// written to satisfy two already-stated requirements at once —
    ///
    /// 1. §3.1's "idempotent by `opId`" — one op id, one row;
    /// 2. `cmp_ops`'s precondition, that no two entries sharing an op id may
    ///    reach the comparator.
    ///
    /// The design named the shape that would give only the first: "a
    /// `SELECT ... ORDER BY` over a table with a **non-unique** index". The
    /// ordering index below ends in `op_id`, which is the primary key, so it is
    /// unique by construction and no ordered read can emit one op id twice.
    ///
    /// # The op is stored as its canonical bytes, not as fields
    ///
    /// `op_bytes` is `SignedOp::to_bytes()` verbatim. The canonical encoding is
    /// the signature preimage (`op.rs`), so storing the op as decomposed columns
    /// and re-encoding on read would make the signature depend on this module's
    /// re-encoding agreeing with `op.rs`'s forever. `a_stored_op_reads_back_byte_identical`
    /// holds it to the byte.
    ///
    /// The columns beside it — `stoa`, `target`, and the three sort columns —
    /// are **derived from those bytes at append time and are indexes, not
    /// authority**. If one ever disagreed with the blob, the blob is right.
    ///
    /// # The failure path rolls back explicitly
    ///
    /// `execute_batch` stops at the first failing statement and returns, with
    /// the `BEGIN` still open. Dropping the connection would make rusqlite roll
    /// it back, so the old code was sound — **by accident rather than by
    /// invariant**, and only while no caller holds the connection past a failed
    /// create. `from_connection` propagates the error and drops it today; a
    /// caller that retried instead would meet "cannot start a transaction
    /// within a transaction" and be told the wrong thing about what went wrong.
    /// The `ROLLBACK` below makes the guarantee this function's own.
    ///
    /// # `PRAGMA user_version` IS LAST, AND MUST STAY LAST
    ///
    /// It is the commit point for the *layout claim*, and every other statement
    /// has to be true before it is made. A crash — or a failure — anywhere
    /// before it leaves a file at version `0`, which `from_connection` reads as
    /// "never stamped" and creates cleanly. Moving the pragma earlier, or
    /// splitting this batch so the pragma lands in a separate one that can
    /// succeed on its own, produces exactly the file `check_layout` exists to
    /// refuse: version `1` stamped over tables that were never created. That
    /// file is then permanently unopenable, because there is no migration path
    /// by design — so the ordering here is not tidiness, it is what keeps a
    /// crash mid-create recoverable.
    ///
    /// A later "add a migration" refactor is the change this warning is
    /// addressed to.
    fn create_schema(conn: &Connection) -> Result<(), OpLogError> {
        let result = conn.execute_batch(&format!(
            "BEGIN;
             CREATE TABLE ops (
                 -- §3.1's idempotence AND `cmp_ops`'s precondition, in one
                 -- constraint. See this function's documentation.
                 op_id             BLOB PRIMARY KEY NOT NULL,

                 -- The signed canonical encoding, verbatim. The authority.
                 op_bytes          BLOB NOT NULL,

                 -- The Stoa ADDRESS, never a channel id (§4.5: 'never let
                 -- channel identity leak into payloads or storage keys', so
                 -- that per-thread channels later become a routing change
                 -- rather than a migration).
                 stoa              BLOB NOT NULL,

                 -- The op this one acts upon, or NULL for a kind that names
                 -- none. A `Post`'s parent is NOT a target: a reply names its
                 -- parent as a reply relationship, not as a subject acted upon.
                 target            BLOB,

                 -- ── The recorded arrival, verbatim ────────────────────────
                 -- What the transport said, exactly as `Arrival` holds it.
                 -- NULL means the transport supplied nothing for that field.
                 --
                 -- SEPARATE FROM THE SORT COLUMNS BELOW, and that separation is
                 -- load-bearing rather than redundant. The two are different
                 -- facts: this is what the peer RECORDED, the sort columns are
                 -- what the ORDER BY compares. They differ in TWO ways, and an
                 -- earlier comment here claimed one — which is the kind of
                 -- exactness claim that invites a future editor to collapse the
                 -- columns:
                 --
                 --   1. An op the transport did not order but for which it
                 --      supplied a message id. The arrival keeps the id; the
                 --      sort key must not carry it, because `cmp_ops` does not
                 --      consult it. See `SortKey`.
                 --   2. EVERY unordered op. `arrival_lamport` is NULL, while
                 --      `sort_lamport` is the filler 0 — and 0 is a real
                 --      Lamport value's image (`lamport_sort_key(i64::MAX)`),
                 --      so the sort column cannot represent absence at all.
                 --      Harmless only because `sort_ordered` leads.
                 --
                 -- Deriving the arrival back OUT of the sort columns was the
                 -- first design and it was wrong: it made the recorded fact and
                 -- the ordering fact one column set, so making the ordering
                 -- correct would have silently discarded a message id the peer
                 -- genuinely received.
                 arrival_lamport   INTEGER,
                 arrival_msg       BLOB,

                 -- ── The read order, materialised ──────────────────────────
                 -- `cmp_ops` expressed over stored values so it can be
                 -- indexed. Written once at append; first-wins makes that
                 -- sound, since the arrival is fixed at first append and no
                 -- code path here rewrites it.

                 -- 0 = the transport ordered this op, 1 = it did not.
                 -- THE LEADING COLUMN, and `cmp_ops`'s partition made
                 -- structural: every ordered op precedes every unordered one
                 -- whatever the Lamport value. A column rather than a sentinel
                 -- inside `sort_lamport`, because u64 and i64 have the same
                 -- cardinality and a bijection leaves no value spare. See
                 -- `lamport_sort_key`, which records the draft that got this
                 -- wrong and how Lamport 0 exposed it.
                 sort_ordered      INTEGER NOT NULL,

                 -- Descending Lamport, reversed at write time. Meaningless when
                 -- `sort_ordered` is 1, and never reached in that case.
                 sort_lamport      INTEGER NOT NULL,

                 -- `cmp_tiebreak` puts an op WITH a message id before one
                 -- without. This is its own column rather than a sentinel
                 -- inside `sort_msg` because the EMPTY message id is a legal
                 -- value, so no byte string is 'below every byte string and
                 -- not equal to the empty one'. Collapsing the two would be
                 -- `absence_is_not_equal_to_a_zero_lamport_timestamp`'s defect
                 -- in another costume.
                 sort_msg_present  INTEGER NOT NULL,
                 sort_msg          BLOB,

                 -- The op's AUTHOR, lifted out of `op_bytes` so it can be
                 -- joined and indexed.
                 --
                 -- Reserved for §7.2's ranking, and the shape is the point.
                 -- Votes are to affect relevance, with a MODERATOR's vote
                 -- weighted more heavily than an ordinary one — and
                 -- `moderation.rs` decides moderator-ness **on read, every
                 -- time, from the genesis record**, caching nothing and
                 -- deciding nothing at append.
                 --
                 -- So a vote's weight is NOT a property of the vote. It
                 -- depends on the moderator set at the moment of reading, and
                 -- a schema that multiplied a weight in at insert would
                 -- contradict the module that decides authority, and would go
                 -- silently stale the moment a moderator set changed.
                 --
                 -- What IS stable at write time is who signed the op. Storing
                 -- the author makes 'count this target's votes, partitioned by
                 -- voter' an indexed query, with the weighting applied at
                 -- READ time against the (small) moderator set — which is the
                 -- only shape consistent with authority being a read-time
                 -- decision.
                 author            BLOB NOT NULL,

                 -- ── Reserved for a relevance score (§7.2 rule 5) ──────────
                 -- NO READ CONSULTS THIS YET.
                 --
                 -- The time a score decays FROM: the op's Lamport timestamp,
                 -- or NULL where the transport supplied none.
                 --
                 -- NULL AND NOT A SENTINEL. `-1` was the first draft and it
                 -- was wrong for the reason `lamport_sort_key` gives about its
                 -- own column: `u64::MAX as i64` IS `-1`, so an unordered op
                 -- and one ordered at the maximum stored the same value. A
                 -- reserved column nothing reads is the easiest place to leave
                 -- a defect, because no test fails — and the rows are written
                 -- to every peer's store long before the read that would
                 -- expose it.
                 --
                 -- This is the durable half of the rule 5 decision. A score
                 -- multiplied by decay BEFORE storage is a function of the
                 -- READ's clock, so no index over it can be correct and
                 -- §2.5's `ORDER BY … LIMIT` degrades to a full scan on every
                 -- page. Storing the epoch instead lets decay be applied in
                 -- the ORDER BY expression over stored values, which an index
                 -- can serve.
                 --
                 -- NOT a wall-clock reading, and that is the whole point.
                 -- `arrival.rs` refuses `channelMessageReceived`'s `timestamp`
                 -- because it is a per-peer CLOCK_REALTIME read — 'recording
                 -- it here would be recording arrival sequence while believing
                 -- we recorded a shared order'. A decay epoch taken from the
                 -- local clock reintroduces that one layer up: two peers would
                 -- rank the same ops differently because they received them at
                 -- different instants, which is divergence from a source §7.2
                 -- rule 1 does not sanction.
                 --
                 -- NO `score` COLUMN, deliberately. An earlier draft reserved
                 -- one and it was wrong: a stored score is a stored WEIGHTING,
                 -- and the weighting is not knowable at append. See the
                 -- `author` column above.
                 score_epoch       INTEGER
             ) STRICT;

             -- THE ORDERING INDEX. Its columns are exactly the ORDER BY's, in
             -- order, so an unrestricted read is an index scan rather than a
             -- materialise-and-sort. It ends in `op_id` — the primary key — so
             -- it is unique, which is what keeps a `SELECT ... ORDER BY` over
             -- it from ever emitting one op id twice.
             CREATE INDEX ops_order
                 ON ops (sort_ordered, sort_lamport,
                         sort_msg_present DESC, sort_msg, op_id);

             -- The two restricted reads, each prefixed by what it restricts on
             -- so that the same ordering is served without a sort.
             CREATE INDEX ops_by_stoa
                 ON ops (stoa, sort_ordered, sort_lamport,
                         sort_msg_present DESC, sort_msg, op_id);
             CREATE INDEX ops_by_target
                 ON ops (target, sort_ordered, sort_lamport,
                         sort_msg_present DESC, sort_msg, op_id);

             -- RESERVED, and nothing queries it yet. `(target, author)` is the
             -- shape a vote tally needs: 'the ops about this subject, grouped
             -- by who signed them', which a later ranking change weights
             -- against the moderator set it resolves at read time. It is
             -- created now rather than later because adding an index to a
             -- peer's existing store is cheap only while the store is small,
             -- and because an index nothing uses costs writes and nothing else
             -- — which is the right side of rule 5's trade.
             CREATE INDEX ops_by_target_author ON ops (target, author);

             -- ── The Stoas this peer created or joined ─────────────────────
             --
             -- NOT derived from the ops table, and that is the point. A Stoa is
             -- a genesis record its creator publishes (§1) and its address is
             -- the hash of that record — so holding ops addressed to a Stoa
             -- says nothing about whether this peer READS it, and holding no
             -- ops says nothing about whether it does. `iter_stoa` answers
             -- 'what have I seen addressed here', which is a different question
             -- from 'which Stoas am I in'. A peer that joined a quiet Stoa must
             -- still list it, and a peer that received a gossiped op for a Stoa
             -- it never joined must not.
             CREATE TABLE stoas (
                 -- The 32-byte Stoa address. The identity, and the primary key,
                 -- so joining twice is idempotent for the same reason appending
                 -- one op twice is: there is one slot.
                 stoa          BLOB PRIMARY KEY NOT NULL,

                 -- THE GENESIS RECORD, verbatim canonical bytes.
                 --
                 -- Stored rather than reconstructed because it CANNOT be
                 -- reconstructed: the address is a one-way hash of it, so a peer
                 -- holding only an address cannot recover the creator key or the
                 -- title. And it must be held because every read needs it —
                 -- `Moderators::of` takes a genesis record and there is no other
                 -- way to build one, so a Stoa whose record this peer lost is a
                 -- Stoa whose feed cannot be moderated and therefore must not be
                 -- served (`wire.rs` makes that structural).
                 --
                 -- Self-authenticating, so storage is safe: the address is the
                 -- hash of these bytes, and `Genesis::matches` re-derives it.
                 -- Nothing trusts this column — every read re-checks it.
                 genesis_bytes BLOB NOT NULL,

                 -- 'created' or 'joined'. One column with two values rather
                 -- than two tables, because they are the same fact about one
                 -- Stoa and every read wants both.
                 --
                 -- Recorded rather than inferred from the creator key. Inferring
                 -- looks equivalent and is not: it would say 'created' for a
                 -- Stoa somebody else made and this peer joined while HOLDING
                 -- the creator key, which is not a state that can arise today
                 -- but is also not a question this column has to leave open. It
                 -- is also the honest answer to 'did I make this' — which is
                 -- what a user asked, not 'do I hold a key that could have'.
                 relation      TEXT NOT NULL
             ) STRICT;

             -- LAST, DELIBERATELY. See this function's documentation: this is
             -- the layout CLAIM, and everything it claims must already be true
             -- when it is made. A crash or failure before this point leaves
             -- version 0, which reopens as a fresh store and creates cleanly.
             -- Moving it earlier — or into a batch of its own — strands a file
             -- at version 1 with no tables, which `check_layout` then refuses
             -- forever, because there is no migration path by design.
             PRAGMA user_version = {LAYOUT_VERSION};
             COMMIT;"
        ));

        if let Err(e) = result {
            // EXPLICIT, not left to `Drop`. `execute_batch` returns at the
            // first failing statement with the `BEGIN` still open; rusqlite
            // rolling back on drop made that sound only for as long as every
            // caller drops the connection on this path. Stating it here makes
            // it a property of this function instead of of its callers.
            //
            // The rollback's own result is discarded on purpose: the caller is
            // owed the error that CAUSED the failure, not a second error from
            // cleaning up after it. A failed `ROLLBACK` here means the
            // transaction was never open — which is the state we wanted.
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(storage(e));
        }
        Ok(())
    }

    /// The one `SELECT`, shared by all three ordered reads.
    ///
    /// Written once for the reason `MemoryOpLog::sorted` gives: a second call
    /// site spelling its own `ORDER BY` is how one of them eventually spells it
    /// differently. The `WHERE` clause is the only thing that varies, and it is
    /// passed in rather than being chosen from an enum here, so a fourth read
    /// does not mean editing this function.
    ///
    /// The `ORDER BY` is `cmp_ops`: ascending `sort_lamport` (ordered ops first,
    /// then descending Lamport), then message-id presence, then message-id
    /// bytes, then op id as the last resort — matching
    /// `cmp_ops` → `cmp_tiebreak` → `a.id.cmp(b.id)` branch for branch.
    fn ordered_read(
        &self,
        where_clause: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<Entry>, OpLogError> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS}
             FROM ops
             {where_clause}
             ORDER BY sort_ordered ASC, sort_lamport ASC,
                      sort_msg_present DESC, sort_msg ASC, op_id ASC"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(storage)?;
        let rows = stmt
            .query_map(params, |row| {
                Ok(Row {
                    op_bytes: row.get(0)?,
                    arrival_lamport: row.get(1)?,
                    arrival_msg: row.get(2)?,
                })
            })
            .map_err(storage)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(decode_entry(row.map_err(storage)?)?);
        }
        Ok(out)
    }
}

/// One row, as the two reads select it.
///
/// A named struct rather than a tuple because the two `Option`s are easy to
/// transpose and the compiler would not notice — both are `Option`s of
/// different types only by luck.
struct Row {
    op_bytes: Vec<u8>,
    arrival_lamport: Option<i64>,
    arrival_msg: Option<Vec<u8>>,
}

/// The columns both reads select, in the order [`decode_entry`] expects.
///
/// A `const` rather than two string literals, because the `SELECT` list and the
/// index arithmetic in the row mapper have to agree and nothing else checks
/// that they do.
const SELECT_COLUMNS: &str = "op_bytes, arrival_lamport, arrival_msg";

/// Rebuild an [`Entry`] from what a row holds.
///
/// The op comes from its canonical bytes, so the signature is over exactly what
/// was signed.
///
/// **The arrival comes from the `arrival_*` columns and NOT from the sort key.**
/// An earlier draft derived it from the sort columns, on the reasoning that
/// there was nothing in an `Arrival` they did not already carry. That was wrong
/// in two ways, and the schema comment on `arrival_lamport` enumerates both.
/// The sharper one: an op the transport did not order, for which it nonetheless
/// supplied a message id — the shape an SDS ephemeral message produces. The
/// sort key must drop that id (see [`SortKey`]) and the record must keep it, so
/// one column set cannot be both.
fn decode_entry(row: Row) -> Result<Entry, OpLogError> {
    let op = SignedOp::from_bytes(&row.op_bytes)
        .map_err(|e| OpLogError::CorruptEntry(format!("the stored op did not decode: {e}")))?;

    // Stored from a `u64` by a cast that is reversed here. The round trip is
    // exact across the whole domain — including `u64::MAX`, which is stored as
    // `-1` and read back as `u64::MAX` — which
    // `a_maximal_lamport_timestamp_survives_storage` pins at the boundary
    // rather than at a convenient middle value.
    let lamport = row.arrival_lamport.map(|l| l as u64);

    // `NULL` versus a value, not empty versus non-empty: an EMPTY message id is
    // a legal recorded value and must read back as present-and-empty. That is
    // `absence_is_not_equal_to_a_zero_lamport_timestamp`'s distinction, and
    // `an_empty_message_id_is_recorded_and_is_not_absence` pins it here.
    let message_id = row.arrival_msg.map(MessageId::new);

    Ok(Entry {
        op,
        arrival: Arrival::from_parts(lamport, message_id),
    })
}

/// Every `rusqlite` failure becomes one variant, carrying its own description.
///
/// A free function rather than a `From` impl, deliberately: a `From` would make
/// `?` convert silently at every call site, including ones where a different
/// variant is the right answer — `CorruptEntry` is not a storage failure, and
/// the two are distinguishable only because this conversion is written out.
fn storage(e: rusqlite::Error) -> OpLogError {
    OpLogError::Storage(e.to_string())
}

impl OpLog for SqliteOpLog {
    /// Writes whatever it is given, once, keeping the first arrival's metadata.
    ///
    /// **`INSERT OR IGNORE`, never `INSERT OR REPLACE`.** Replace would
    /// overwrite the stored arrival with the later one — richer-wins by the back
    /// door, which is the rule `arrival.rs` spends a section of module
    /// documentation warning consumers off, because whether a peer receives the
    /// richer copy is a per-peer accident and two peers would then sort the same
    /// ops differently with no error anywhere.
    ///
    /// `changes()` reports which happened, so the caller is told rather than
    /// having to count the log before and after.
    fn append(&mut self, op: SignedOp, arrival: Arrival) -> Result<Appended, OpLogError> {
        let entry = Entry { op, arrival };
        let id = entry.id();
        let key = SortKey::of(&entry.arrival);
        // `NULL` for "the transport supplied no Lamport value", never a
        // sentinel. **An earlier draft used `-1` and that was the collision
        // this file's own `lamport_sort_key` documentation rejects**:
        // `u64::MAX as i64` IS `-1`, so an unordered op and one ordered at
        // `u64::MAX` stored the same epoch and became indistinguishable.
        //
        // It was inert — nothing reads this column yet — which is exactly why
        // it was worth fixing now: it would have become a §7.2 ranking defect
        // the moment rule 5 landed, in rows already written to every peer's
        // store. `u64` and `i64` have the same cardinality, so no in-band
        // sentinel can work; the same reasoning that produced `sort_ordered`.
        let score_epoch = entry.arrival.lamport().map(|l| l as i64);

        let changed = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO ops
                     (op_id, op_bytes, stoa, target, author,
                      arrival_lamport, arrival_msg,
                      sort_ordered, sort_lamport, sort_msg_present, sort_msg,
                      score_epoch)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id.as_bytes().as_slice(),
                    entry.op.to_bytes(),
                    entry.op.op.stoa.as_bytes().as_slice(),
                    entry.target().map(|t| t.as_bytes().to_vec()),
                    entry.op.op.author.to_bytes().as_slice(),
                    // The arrival as RECORDED — both fields, whatever the sort
                    // key does with them.
                    entry.arrival.lamport().map(|l| l as i64),
                    entry.arrival.message_id().map(|m| m.as_bytes().to_vec()),
                    key.ordered,
                    key.lamport,
                    key.msg_present,
                    key.msg,
                    score_epoch,
                ],
            )
            .map_err(storage)?;

        Ok(if changed == 0 {
            Appended::AlreadyPresent
        } else {
            Appended::Stored
        })
    }

    fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError> {
        let row = self
            .conn
            .query_row(
                &format!("SELECT {SELECT_COLUMNS} FROM ops WHERE op_id = ?1"),
                rusqlite::params![id.as_bytes().as_slice()],
                |row| {
                    Ok(Row {
                        op_bytes: row.get(0)?,
                        arrival_lamport: row.get(1)?,
                        arrival_msg: row.get(2)?,
                    })
                },
            )
            // `optional` turns SQLite's "no rows" into `None` rather than an
            // error, which is the whole point of the nesting: absence is a
            // defined answer and a failure is a different one.
            .optional()
            .map_err(storage)?;

        row.map(decode_entry).transpose()
    }

    fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
        self.ordered_read("", &[])
    }

    fn iter_stoa(&self, stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
        // `= ?1` on the whole 32-byte blob, never a prefix or a LIKE. A
        // prefix-matching restricted read is a cross-Stoa leak in a
        // censorship-resistant forum, and with 32-byte addresses a collision is
        // rare enough never to surface in casual testing and certain enough to
        // surface eventually.
        self.ordered_read(
            "WHERE stoa = ?1",
            &[&stoa.as_bytes().as_slice() as &dyn rusqlite::ToSql],
        )
    }

    fn iter_target(&self, target: &OpId) -> Result<Vec<Entry>, OpLogError> {
        // `target IS NOT NULL` is implied by the equality — SQL's `=` never
        // matches NULL — so an op naming no target can never be returned by a
        // target read, without a second clause that has to be remembered.
        self.ordered_read(
            "WHERE target = ?1",
            &[&target.as_bytes().as_slice() as &dyn rusqlite::ToSql],
        )
    }

    fn len(&self) -> Result<usize, OpLogError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ops", [], |row| row.get(0))
            .map_err(storage)?;
        // A negative count is not representable by COUNT(*), and a count past
        // `usize` needs more ops than addressable memory. `try_into` rather than
        // `as` so that a platform where it could fail says so instead of
        // wrapping.
        usize::try_from(n).map_err(|_| {
            OpLogError::CorruptEntry(format!("the store reported {n} ops, which is not a count"))
        })
    }
}

impl StoaRegistry for SqliteOpLog {
    /// `INSERT OR IGNORE`, never `REPLACE` — first-wins, for the reason
    /// [`OpLog::append`] gives.
    ///
    /// Here it is additionally near-vacuous and worth saying so: the address is
    /// the hash of `genesis_bytes`, so two rows competing for one address carry
    /// byte-identical records. What the rule actually preserves is `relation`,
    /// which is NOT a function of the record — a peer that created a Stoa and
    /// later pastes its own address must not have "created" rewritten to
    /// "joined".
    fn remember_stoa(
        &mut self,
        genesis: &crate::stoa::Genesis,
        relation: Relation,
    ) -> Result<Appended, OpLogError> {
        // Encode once, and let the failure be reported. `canonical_bytes` refuses
        // an over-long title, so this is the encoder and the decoder agreeing
        // about what is storable — the symmetry `stoa.rs` enforces on both sides.
        let bytes = genesis
            .canonical_bytes()
            .map_err(|e| OpLogError::CorruptEntry(format!("genesis record: {e}")))?;
        // From the bytes just encoded rather than from a second call to
        // `address()`, so the stored key and the stored record cannot disagree.
        let address = crate::identity::stoa_address(&bytes);

        let changed = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO stoas (stoa, genesis_bytes, relation)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    address.as_bytes().as_slice(),
                    bytes,
                    relation.as_str(),
                ],
            )
            .map_err(storage)?;

        Ok(if changed == 0 {
            Appended::AlreadyPresent
        } else {
            Appended::Stored
        })
    }

    fn get_stoa(&self, stoa: &Address) -> Result<Option<JoinedStoa>, OpLogError> {
        let row = self
            .conn
            .query_row(
                "SELECT genesis_bytes, relation FROM stoas WHERE stoa = ?1",
                rusqlite::params![stoa.as_bytes().as_slice()],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage)?;

        row.map(|(bytes, relation)| decode_stoa(&bytes, &relation))
            .transpose()
    }

    fn list_stoas(&self) -> Result<Vec<JoinedStoa>, OpLogError> {
        // `ORDER BY stoa` is the promised address order, served by the primary
        // key's own index. The trait states the order; this is where it is kept.
        let mut stmt = self
            .conn
            .prepare("SELECT genesis_bytes, relation FROM stoas ORDER BY stoa ASC")
            .map_err(storage)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;

        let mut out = Vec::new();
        for row in rows {
            let (bytes, relation) = row.map_err(storage)?;
            out.push(decode_stoa(&bytes, &relation)?);
        }
        Ok(out)
    }
}

/// Rebuild a [`JoinedStoa`] from what a row holds.
///
/// Both columns are decoded strictly, and a failure is [`OpLogError::CorruptEntry`]
/// rather than [`OpLogError::Storage`] for the reason [`decode_entry`] gives: the
/// store worked and what it handed back did not, which points at the file rather
/// than at the disk.
///
/// **The genesis record is re-decoded rather than trusted.** It is a blob in a
/// file that another process may have edited, so it goes through
/// `Genesis::decode` — the same strict decoder a record arriving from a peer
/// meets. Nothing about being on our own disk makes these bytes trustworthy;
/// `keystore.rs` makes the same argument about the keystore file.
fn decode_stoa(bytes: &[u8], relation: &str) -> Result<JoinedStoa, OpLogError> {
    let genesis = crate::stoa::Genesis::decode(bytes).map_err(|e| {
        OpLogError::CorruptEntry(format!("the stored genesis record did not decode: {e}"))
    })?;
    let relation = Relation::parse_str(relation).ok_or_else(|| {
        OpLogError::CorruptEntry(format!(
            "the stored Stoa relation {relation:?} is neither \"created\" nor \"joined\""
        ))
    })?;
    Ok(JoinedStoa { genesis, relation })
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use crate::log::Appended;
    use std::cmp::Ordering;

    /// A path in a fresh temporary directory, and the directory's guard.
    ///
    /// The guard must be held for the test's lifetime: dropping it removes the
    /// directory. Returned as a pair rather than hidden behind a helper that
    /// drops it, because a test that let it drop early would fail confusingly.
    ///
    /// Built with `std::fs` rather than a `tempfile` dependency, for the same
    /// reason the keystore change hand-rolled its nine-line predicate: one need,
    /// in tests, is not worth a crate.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            // The process id and the test's own name keep two tests in one run
            // from colliding, and two runs from inheriting each other's files.
            path.push(format!("dialectica-oplog-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
            TempDir(path)
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // ─── The layout version ───────────────────────────────────────────────

    #[test]
    fn the_layout_version_is_pinned_to_a_known_answer() {
        // Hardcoded, following `identity.rs`'s wire constants. `cargo mutants`
        // mutates functions and not `const`s, so a wrong version here is
        // invisible to it — and this project has already shipped a `VERSION_1`
        // defect that left the whole suite green.
        //
        // Changing this number is changing the on-disk format every peer holds.
        // If this assertion fails, that is the question being asked.
        //
        // IT HAS FAILED ONCE, AND THE ANSWER IS RECORDED RATHER THAN THE NUMBER
        // QUIETLY EDITED. Version 2 added the `stoas` table, which the write path
        // needs because a feed cannot be served without the Stoa's genesis record
        // and a version-1 store has nowhere to keep one. The bump is what makes an
        // old store report `UnknownLayoutVersion` — "an older layout, honestly
        // stamped" — rather than `LayoutDoesNotMatchItsVersion`, which means "this
        // file is lying about its layout". A version-1 store is not lying, and
        // telling its owner that it is would send them looking for tampering.
        //
        // There is still no migration: a version-1 store is refused, not upgraded.
        assert_eq!(LAYOUT_VERSION, 2);
    }

    #[test]
    fn a_store_from_an_unknown_layout_version_is_refused_and_names_both_numbers() {
        // Ops are the authority for all forum state (§3.3), so a layout misread
        // yields a forum that is wrong with no error anywhere. Refusing is
        // visible; reading best-effort is not.
        let dir = TempDir::new("unknown-version");
        let path = dir.file("log.sqlite");

        // A store this build wrote, then stamped with a version it does not
        // know. Stamped rather than hand-built, so the test exercises the
        // version check and not a malformed file.
        let log = SqliteOpLog::open(&path).unwrap();
        drop(log);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version = 9999").unwrap();
        drop(conn);

        match SqliteOpLog::open(&path) {
            Err(OpLogError::UnknownLayoutVersion { found, expected }) => {
                // Hardcoded on both sides: a test asserting `found == 9999`
                // only would pass for an implementation that reported the same
                // number twice.
                //
                // `expected` stays a LITERAL rather than becoming
                // `LAYOUT_VERSION`, which is the tempting edit when a bump makes
                // this fail. Substituting the constant would make both sides of
                // the comparison come from the implementation, and the test would
                // then agree with any version the code happened to report —
                // including a wrong one. A literal is what keeps this a check.
                assert_eq!(found, 9999);
                assert_eq!(expected, 2);
            }
            other => panic!("an unknown layout version must be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_store_from_an_older_layout_version_is_refused_too() {
        // **A SURVIVOR THAT MUTATION TESTING FOUND.** Relaxing the check from
        // `found != LAYOUT_VERSION` to `found > LAYOUT_VERSION` — accepting any
        // layout older than this build's — passed the entire suite, because
        // every other version test uses a HIGHER version (9999).
        //
        // "Refuse nothing" dies instantly. "Refuse only the future" is the
        // mutation one step weaker, and it is both plausible and wrong:
        // accepting an older layout is exactly the shape a well-meaning
        // "backwards compatible" edit takes, and it would have this build read
        // a version-0-shaped file through version-1 column positions. Ops are
        // the authority for all forum state, so that is a forum that is wrong
        // with no error anywhere.
        //
        // There is no migration path here by design (see `design.md`): a
        // layout this build does not understand is refused in BOTH directions,
        // and an older store is upgraded by a change that says so, not by
        // being read hopefully.
        let dir = TempDir::new("older-version");
        let path = dir.file("log.sqlite");

        let log = SqliteOpLog::open(&path).unwrap();
        drop(log);
        let conn = Connection::open(&path).unwrap();
        // A NEGATIVE version, not `LAYOUT_VERSION - 1`. With `LAYOUT_VERSION`
        // at 1 those are the same thing, and `0` is SQLite's "never stamped"
        // value which legitimately means a fresh file — so a test using it
        // would assert the opposite of what it names, and would start failing
        // for the wrong reason the moment the version is bumped.
        //
        // `-1` is below this build's version, is not the fresh sentinel, and
        // stays below whatever `LAYOUT_VERSION` becomes. That is the property
        // the test needs, stated so a later reader does not "simplify" it back.
        conn.execute_batch("PRAGMA user_version = -1").unwrap();
        drop(conn);

        match SqliteOpLog::open(&path) {
            Err(OpLogError::UnknownLayoutVersion { found, expected }) => {
                assert_eq!(found, -1);
                assert_eq!(expected, LAYOUT_VERSION);
            }
            other => panic!("an older layout version must be refused too, got {other:?}"),
        }
    }

    #[test]
    fn a_fresh_store_is_created_rather_than_refused() {
        // The boundary on the other side of the version check: `0` is what
        // SQLite reports for a file nobody has stamped, and it is the ONE value
        // that must not be refused — it is the absence of a version rather than
        // an unknown one. A check that refused it would make the log
        // unopenable on first run.
        let dir = TempDir::new("fresh");
        let path = dir.file("log.sqlite");
        assert!(!path.exists(), "the fixture must start with no file");

        let log = SqliteOpLog::open(&path).expect("a fresh store must be creatable");
        assert_eq!(log.len().unwrap(), 0);

        // And it stamped the version, so the next open takes the other branch.
        let stamped: i32 = log
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stamped, LAYOUT_VERSION);
    }

    #[test]
    fn a_store_stamped_with_our_version_but_missing_the_tables_is_refused_at_open() {
        // The gap the version check cannot see. `PRAGMA user_version` is one
        // integer and anything can write it, so a file carrying OUR number is a
        // claim rather than a fact — and before `check_layout` such a file
        // opened `Ok` and failed at the first read with
        // `Storage("no such table: ops")`, blaming the disk for a mislabelled
        // file.
        //
        // Asserted at OPEN and not merely "some error eventually": that the
        // failure arrives at the moment the claim is made is the whole finding.
        let dir = TempDir::new("stamped-but-empty");
        let path = dir.file("log.sqlite");

        // An otherwise-empty database stamped with this build's version. Built
        // by hand rather than by creating and dropping, so nothing about the
        // real schema is involved — this is what a half-restored backup or a
        // hand-edited file looks like.
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!("PRAGMA user_version = {LAYOUT_VERSION};"))
            .unwrap();
        drop(conn);

        match SqliteOpLog::open(&path) {
            Err(OpLogError::LayoutDoesNotMatchItsVersion { version, why }) => {
                assert_eq!(version, LAYOUT_VERSION);
                // The underlying reason is carried through rather than
                // swallowed: a reader holding this file needs to know WHICH
                // part of the layout was missing.
                assert!(
                    why.contains("ops"),
                    "the refusal must name what was missing, got {why:?}"
                );
            }
            other => panic!(
                "a store stamped with our version but without our layout must be \
                 refused at open, got {other:?}"
            ),
        }
    }

    #[test]
    fn a_store_whose_ops_table_lost_a_column_is_refused_too() {
        // One step weaker than the test above and the one a table-name-only
        // check would pass: the table is called `ops` and is not the `ops` this
        // build reads. `SELECT 1 FROM ops LIMIT 0` cannot tell the difference,
        // which is why `check_layout` names the columns.
        let dir = TempDir::new("stamped-wrong-columns");
        let path = dir.file("log.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE ops (op_id BLOB PRIMARY KEY NOT NULL, op_bytes BLOB NOT NULL);
             PRAGMA user_version = {LAYOUT_VERSION};"
        ))
        .unwrap();
        drop(conn);

        match SqliteOpLog::open(&path) {
            Err(OpLogError::LayoutDoesNotMatchItsVersion { version, .. }) => {
                assert_eq!(version, LAYOUT_VERSION);
            }
            other => panic!(
                "a table named `ops` that is not our `ops` must be refused, got {other:?}"
            ),
        }
    }

    #[test]
    fn a_store_with_ops_but_no_stoas_table_is_refused_too() {
        // THE TEST WITHOUT WHICH THE `stoas` HALF OF `check_layout` IS FREE TO
        // DELETE. Every other layout test builds a file with no `ops` table or a
        // wrong one, so all of them are decided by the FIRST of the two checks —
        // measured, not assumed: with the `stoas` check commented out, every one
        // of them stays green.
        //
        // The file this constructs is the one that distinguishes them: a complete,
        // correct `ops` table stamped with this build's version, and no `stoas`.
        // It is exactly what a version-1 store hand-stamped to 2 looks like, and
        // what a partially-restored backup looks like. Without the second check it
        // opens `Ok` and fails at the first `list_stoas` as
        // `Storage("no such table: stoas")` — the disk-blaming error the whole
        // function exists to replace.
        //
        // The `ops` DDL is copied from `create_schema` rather than produced by it,
        // because producing it would also produce `stoas` and there would be
        // nothing to test. That duplication is the point of the fixture: it is the
        // only way to express "the half-built store".
        let dir = TempDir::new("ops-without-stoas");
        let path = dir.file("log.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE ops (
                 op_id             BLOB PRIMARY KEY NOT NULL,
                 op_bytes          BLOB NOT NULL,
                 stoa              BLOB NOT NULL,
                 target            BLOB,
                 arrival_lamport   INTEGER,
                 arrival_msg       BLOB,
                 sort_ordered      INTEGER NOT NULL,
                 sort_lamport      INTEGER NOT NULL,
                 sort_msg_present  INTEGER NOT NULL,
                 sort_msg          BLOB,
                 author            BLOB NOT NULL,
                 score_epoch       INTEGER
             ) STRICT;
             PRAGMA user_version = {LAYOUT_VERSION};"
        ))
        .unwrap();
        drop(conn);

        match SqliteOpLog::open(&path) {
            Err(OpLogError::LayoutDoesNotMatchItsVersion { version, why }) => {
                assert_eq!(version, LAYOUT_VERSION);
                // It must name the MISSING table, not the one that was fine.
                // Without this the assertion would pass for a check that refused
                // the file while reporting `ops` — sending a reader to the one
                // table that is correct.
                assert!(
                    why.contains("stoas"),
                    "the refusal must name the missing table, got {why:?}"
                );
            }
            other => panic!(
                "a store with a correct `ops` and no `stoas` must be refused at \
                 open, got {other:?}"
            ),
        }
    }

    #[test]
    fn a_store_this_build_wrote_passes_its_own_layout_check() {
        // The other side of the boundary, and not a formality: a `check_layout`
        // naming a column the schema does not have would refuse every real
        // store, and every other test here opens a store this build just
        // created — so the two would fail together and name the wrong cause.
        // This one is about the reopen specifically.
        let dir = TempDir::new("stamped-and-correct");
        let path = dir.file("log.sqlite");

        let mut log = SqliteOpLog::open(&path).unwrap();
        log.append(signed(a_post("survives")), Arrival::unordered())
            .unwrap();
        drop(log);

        let reopened = SqliteOpLog::open(&path)
            .expect("a store this build wrote must pass this build's layout check");
        assert_eq!(reopened.len().unwrap(), 1);
    }

    #[test]
    fn the_layout_mismatch_refusal_names_the_version_and_the_reason() {
        // The variant is one thing; what a human reads is another. This message
        // has to distinguish itself from `UnknownLayoutVersion`'s, or a reader
        // holding a mislabelled file is told to find "a build that knows that
        // layout" — advice that cannot help, because this build IS that build.
        let rendered = OpLogError::LayoutDoesNotMatchItsVersion {
            version: 1,
            why: "no such table: ops".to_string(),
        }
        .to_string();
        assert!(rendered.contains('1'), "the declared version is not named");
        assert!(
            rendered.contains("no such table: ops"),
            "the underlying reason is not named"
        );
    }

    #[test]
    fn the_refusal_names_the_version_in_its_message() {
        // The variant is one thing; what a human reads is another. A refusal
        // that did not name the numbers would leave the reader unable to tell
        // which build wrote the file they are holding.
        let rendered = OpLogError::UnknownLayoutVersion {
            found: 7,
            expected: 1,
        }
        .to_string();
        assert!(rendered.contains('7'), "the found version is not named");
        assert!(rendered.contains('1'), "the expected version is not named");
    }

    // ─── Dedup is keyed on the identity column ────────────────────────────

    #[test]
    fn the_primary_key_is_the_op_id_and_not_any_other_column() {
        // **A SURVIVOR THAT MUTATION TESTING FOUND, and the test that kills
        // it.** Moving the `PRIMARY KEY` from `op_id` to `op_bytes` passed the
        // ENTIRE suite — every dedup test, every contract test, both
        // implementations.
        //
        // It survives because the two columns agree on every input the public
        // API can construct: identical ops have identical bytes, so `op_bytes`
        // deduplicates exactly where `op_id` does. "Remove the primary key"
        // dies instantly and proves nothing; this is the mutation one step
        // weaker, and it is the one that was live.
        //
        // Why it would matter. §3.1 makes the op id the identity, and it is a
        // DOMAIN-SEPARATED HASH of the canonical bytes rather than the bytes
        // themselves (`an_op_id_is_not_a_bare_hash_of_the_canonical_bytes`).
        // Keying on the encoding rather than on the identity means the store's
        // notion of "the same op" is the encoding's, so any future encoding
        // change that is not identity-preserving — a version bump, a canonical
        // form that admits two spellings — silently splits one op into two
        // rows, and `cmp_ops`'s precondition is violated with no error.
        //
        // Asserted against the schema rather than through the API, because the
        // API cannot construct the distinguishing case: that is precisely why
        // the mutation survived every behavioural test.
        let log = SqliteOpLog::in_memory().unwrap();
        let mut stmt = log
            .conn
            .prepare("SELECT name, pk FROM pragma_table_info('ops')")
            .unwrap();
        let columns: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(!columns.is_empty(), "the table must have columns");
        let key: Vec<&str> = columns
            .iter()
            .filter(|(_, pk)| *pk > 0)
            .map(|(name, _)| name.as_str())
            .collect();
        // Hardcoded, and exactly one column: a composite key including `op_id`
        // would also satisfy a `contains` check while deduplicating on
        // something coarser.
        assert_eq!(
            key,
            vec!["op_id"],
            "the primary key must be the op id alone"
        );
    }

    #[test]
    fn a_store_this_version_wrote_opens_and_keeps_its_ops() {
        let dir = TempDir::new("same-version");
        let path = dir.file("log.sqlite");

        let op = signed(a_post("persisted"));
        let id = op.op.id();
        let mut log = SqliteOpLog::open(&path).unwrap();
        log.append(op, Arrival::unordered()).unwrap();
        drop(log);

        let reopened = SqliteOpLog::open(&path).unwrap();
        assert!(reopened.get(&id).unwrap().is_some());
    }

    // ─── Persistence ──────────────────────────────────────────────────────

    #[test]
    fn ops_outlive_the_log_object_that_stored_them() {
        // §4.7's middle durability tier, which bought nothing before this
        // existed: "everything this peer has ever seen".
        let dir = TempDir::new("outlive");
        let path = dir.file("log.sqlite");

        let population = every_ordering_shape();
        let mut log = SqliteOpLog::open(&path).unwrap();
        for (op, arrival) in &population {
            log.append(op.clone(), arrival.clone()).unwrap();
        }
        let before: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
        drop(log);

        let reopened = SqliteOpLog::open(&path).unwrap();
        let after: Vec<OpId> = reopened.iter().unwrap().iter().map(|e| e.id()).collect();

        assert_eq!(
            before.len(),
            population.len(),
            "the fixture must reach the comparison"
        );
        // The SEQUENCE, not the set: a store that persisted every op but
        // rebuilt its sort key differently on reopen would pass a set
        // comparison and render a thread in a new order after a restart.
        assert_eq!(before, after, "the read order changed across a restart");
    }

    #[test]
    fn a_persisted_op_is_byte_identical_after_a_restart() {
        // The op is signed, so anything the store did to it on the way to disk
        // or back would break the signature. Asserted rather than trusted.
        let dir = TempDir::new("byte-identical");
        let path = dir.file("log.sqlite");

        let op = signed(a_post("exact across a restart"));
        let id = op.op.id();
        let arrival = Arrival::ordered(5, a_message_id(3));

        let mut log = SqliteOpLog::open(&path).unwrap();
        log.append(op.clone(), arrival.clone()).unwrap();
        drop(log);

        let reopened = SqliteOpLog::open(&path).unwrap();
        let entry = reopened.get(&id).unwrap().unwrap();
        assert_eq!(entry.op.to_bytes(), op.to_bytes());
        assert_eq!(entry.op, op);
        assert!(entry.op.verify(), "storage must not disturb the signature");
        assert_eq!(entry.arrival, arrival, "the recorded arrival must survive");
    }

    #[test]
    fn deduplication_survives_a_restart() {
        // §3.1's idempotence is a property of the STORE, not of one process's
        // memory. A peer that re-received an op after a restart and stored it
        // twice would put a duplicate into `cmp_ops`, whose precondition
        // forbids exactly that.
        let dir = TempDir::new("dedup-restart");
        let path = dir.file("log.sqlite");

        let op = signed(a_post("arrives, restarts, arrives again"));
        let mut log = SqliteOpLog::open(&path).unwrap();
        assert_eq!(
            log.append(op.clone(), Arrival::ordered(1, a_message_id(1)))
                .unwrap(),
            Appended::Stored
        );
        drop(log);

        let mut reopened = SqliteOpLog::open(&path).unwrap();
        assert_eq!(
            reopened
                .append(op.clone(), Arrival::ordered(9, a_message_id(2)))
                .unwrap(),
            Appended::AlreadyPresent,
            "a restart must not make a known op look new"
        );
        assert_eq!(reopened.len().unwrap(), 1);
        // And first-wins held across the restart too: the metadata recorded
        // before the drop is the one that survived.
        let entry = reopened.get(&op.op.id()).unwrap().unwrap();
        assert_eq!(entry.arrival.lamport(), Some(1));
    }

    #[test]
    fn whether_an_arrival_was_ordered_survives_a_restart() {
        // The seam for the upstream transport fix: a peer must still be able to
        // say "this thread is ordered by the network" or "by op id" after it
        // restarts. A store that rebuilt the distinction from a sort key alone
        // would lose the message id of an unordered arrival, which is the
        // defect `SortKey` exists to prevent.
        let dir = TempDir::new("ordered-survives");
        let path = dir.file("log.sqlite");

        let ordered = signed(a_post("ordered"));
        let unordered = signed(a_post("unordered"));
        // The sharp one: unordered by the transport, but carrying a message id.
        let id_only = signed(a_post("a message id and no lamport"));

        let mut log = SqliteOpLog::open(&path).unwrap();
        log.append(ordered.clone(), Arrival::ordered(1, a_message_id(1)))
            .unwrap();
        log.append(unordered.clone(), Arrival::unordered()).unwrap();
        log.append(
            id_only.clone(),
            Arrival::from_parts(None, Some(a_message_id(7))),
        )
        .unwrap();
        drop(log);

        let reopened = SqliteOpLog::open(&path).unwrap();
        let back = |op: &crate::op::SignedOp| reopened.get(&op.op.id()).unwrap().unwrap().arrival;

        assert!(back(&ordered).is_ordered_by_transport());
        assert!(!back(&unordered).is_ordered_by_transport());

        // THE case the sort key cannot carry: unordered, and yet a message id
        // was genuinely received. The record must keep it even though the
        // ordering must not consult it.
        let recovered = back(&id_only);
        assert!(!recovered.is_ordered_by_transport());
        assert_eq!(
            recovered.message_id(),
            Some(&a_message_id(7)),
            "an unordered arrival's message id must survive storage"
        );
    }

    #[test]
    fn a_maximal_lamport_timestamp_survives_storage() {
        // `u64::MAX` does not fit in SQLite's `INTEGER`, which is an `i64`, so
        // it is stored through a cast and read back through its inverse. The
        // boundary is where that round trip breaks if it is going to.
        let dir = TempDir::new("maximal-lamport");
        let path = dir.file("log.sqlite");

        let mut log = SqliteOpLog::open(&path).unwrap();
        // A PAIR at the boundary: the largest value, and one below it. One
        // alone would pass for an implementation that saturated.
        let at_max = signed(a_post("at the maximum"));
        let below = signed(a_post("one below the maximum"));
        log.append(at_max.clone(), Arrival::ordered(u64::MAX, a_message_id(1)))
            .unwrap();
        log.append(
            below.clone(),
            Arrival::ordered(u64::MAX - 1, a_message_id(1)),
        )
        .unwrap();
        drop(log);

        let reopened = SqliteOpLog::open(&path).unwrap();
        assert_eq!(
            reopened
                .get(&at_max.op.id())
                .unwrap()
                .unwrap()
                .arrival
                .lamport(),
            Some(u64::MAX)
        );
        assert_eq!(
            reopened
                .get(&below.op.id())
                .unwrap()
                .unwrap()
                .arrival
                .lamport(),
            Some(u64::MAX - 1),
            "a saturating round trip would collapse these two"
        );
        // And the order between them is still descending Lamport.
        let ids: Vec<OpId> = reopened.iter().unwrap().iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![at_max.op.id(), below.op.id()]);
    }

    // ─── The reserved score epoch ─────────────────────────────────────────

    /// Read the `score_epoch` column for one op, as it is actually stored.
    ///
    /// The column is reserved and no read consults it, so there is no API path
    /// to it. Reaching in directly is the only way to test a reserved column —
    /// and NOT testing it is how the `-1` collision below survived review.
    fn stored_score_epoch(log: &SqliteOpLog, id: &OpId) -> Option<i64> {
        log.conn
            .query_row(
                "SELECT score_epoch FROM ops WHERE op_id = ?1",
                rusqlite::params![id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn an_unordered_op_and_a_maximal_lamport_one_store_different_score_epochs() {
        // **A DEFECT A REVIEW FOUND, and the test that would have caught it.**
        //
        // `score_epoch` was written as `map_or(-1, |l| l as i64)`. But
        // `u64::MAX as i64` IS `-1`, so an op the transport did not order and
        // an op ordered at `u64::MAX` stored the SAME value and became
        // indistinguishable — the exact in-band-sentinel collision
        // `lamport_sort_key`'s documentation rejects for its own column, and
        // which `sort_ordered` exists to avoid.
        //
        // It was invisible because the column is reserved: "nothing writes
        // this and no read consults it" is a comment that excuses a column
        // from every behavioural test, while the rows go to every peer's disk.
        // It would have become a §7.2 ranking bug the moment rule 5 landed.
        //
        // `u64::MAX` is in this test specifically because it is the ONLY value
        // whose `as i64` cast collides with a plausible sentinel.
        let mut log = SqliteOpLog::in_memory().unwrap();
        let unordered = signed(a_post("no lamport at all"));
        let maximal = signed(a_post("lamport u64::MAX"));

        log.append(unordered.clone(), Arrival::unordered()).unwrap();
        log.append(maximal.clone(), Arrival::ordered(u64::MAX, a_message_id(1)))
            .unwrap();

        let for_unordered = stored_score_epoch(&log, &unordered.op.id());
        let for_maximal = stored_score_epoch(&log, &maximal.op.id());

        // Hardcoded on both sides rather than merely asserting they differ: a
        // test that only checked inequality would pass for any two sentinels,
        // including a second collision-prone pair.
        assert_eq!(
            for_unordered, None,
            "an unordered arrival must store NULL, not a sentinel"
        );
        assert_eq!(
            for_maximal,
            Some(-1),
            "u64::MAX casts to -1, which is why NULL and not -1 is the absence"
        );
        assert_ne!(
            for_unordered, for_maximal,
            "absence and u64::MAX must be distinguishable in the reserved column"
        );
    }

    #[test]
    fn the_score_epoch_round_trips_every_lamport_boundary() {
        // The reserved column holds the value a later decay will read, so the
        // cast has to be reversible across the whole domain — and the
        // boundaries are the only place a cast breaks.
        let mut log = SqliteOpLog::in_memory().unwrap();
        for lamport in [0u64, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX] {
            let op = signed(a_post(&format!("lamport {lamport}")));
            log.append(op.clone(), Arrival::ordered(lamport, a_message_id(1)))
                .unwrap();
            let stored = stored_score_epoch(&log, &op.op.id())
                .expect("an ordered arrival stores an epoch");
            assert_eq!(
                stored as u64, lamport,
                "the score epoch must round-trip lamport {lamport}"
            );
        }
    }

    // ─── The sort key ─────────────────────────────────────────────────────

    #[test]
    fn the_lamport_sort_key_reverses_the_order_across_the_whole_u64_range() {
        // Boundaries, not convenient middle values. An implementation that
        // overflows or saturates does so at the ends and nowhere else, so a
        // test sampling the middle would report green for a key that collapses
        // `u64::MAX` onto its neighbour.
        //
        // Asserted as a PROPERTY — a < b implies key(a) > key(b) — over every
        // adjacent pair in a boundary-heavy sample, rather than against
        // hardcoded key values, because the values are an encoding detail and
        // the reversal is the contract.
        let values = [
            0u64,
            1,
            2,
            // THE SIGN BOUNDARY, which a review found untested. `u64::MAX / 2`
            // is `i64::MAX as u64`, and it is where the `wrapping_add` crosses
            // from negative to non-negative — the one place in the domain
            // where a cast that is almost right stops being right.
            i64::MAX as u64 - 1,
            i64::MAX as u64,
            i64::MAX as u64 + 1,
            u64::MAX - 2,
            u64::MAX - 1,
            u64::MAX,
        ];
        for window in values.windows(2) {
            let (lower, higher) = (window[0], window[1]);
            assert!(lower < higher, "the sample must be ascending");
            assert_eq!(
                lamport_sort_key(lower).cmp(&lamport_sort_key(higher)),
                Ordering::Greater,
                "the sort key must reverse: {lower} vs {higher}"
            );
        }
        // Injective at the boundary, which the reversal alone does not imply:
        // a saturating map is still weakly decreasing.
        assert_ne!(lamport_sort_key(u64::MAX), lamport_sort_key(u64::MAX - 1));
        assert_ne!(lamport_sort_key(0), lamport_sort_key(1));
    }

    #[test]
    fn the_lamport_key_collides_with_the_unordered_filler_and_sort_ordered_is_why_that_is_safe() {
        // `SortKey::of` fills an unordered op's `lamport` with `0`. That is
        // NOT a spare value — `lamport_sort_key(i64::MAX as u64)` is exactly
        // `0`, so a real Lamport timestamp maps onto the filler.
        //
        // The behaviour is correct, because `sort_ordered` leads the ORDER BY
        // and decides before the Lamport column is consulted. But a review
        // found that **no test used that Lamport value**, so the safety rested
        // on a comment: deleting `sort_ordered` and relying on `sort_lamport`
        // alone would have broken this one pair and nothing would have failed.
        //
        // Asserted as a COLLISION rather than avoided, because the collision
        // is real and the defence is structural. A future edit that removes
        // the leading column fails here, with a name that says what to look
        // at, instead of failing somewhere in a sequence comparison.
        let colliding = i64::MAX as u64;
        assert_eq!(
            lamport_sort_key(colliding),
            0,
            "this is the value whose key collides with the unordered filler"
        );

        let ordered = SortKey::of(&Arrival::ordered(colliding, a_message_id(1)));
        let unordered = SortKey::of(&Arrival::unordered());
        assert_eq!(
            ordered.lamport, unordered.lamport,
            "the collision is real: the Lamport column cannot separate these"
        );
        assert!(
            ordered.ordered < unordered.ordered,
            "so `sort_ordered` must, and it is the only thing that does"
        );
    }

    #[test]
    fn an_op_at_the_colliding_lamport_value_still_reads_before_an_unordered_one() {
        // The consequence of the collision above, asserted end to end through
        // a real read rather than only on the key. A sort that consulted
        // `sort_lamport` before `sort_ordered` would interleave these two.
        let mut log = SqliteOpLog::in_memory().unwrap();
        let (unordered, ordered) = two_posts_by_ascending_id();
        // The ORDERED op is given the HIGHER op id, so a read falling back to
        // the op id would put the unordered one first and fail.
        log.append(unordered.clone(), Arrival::unordered()).unwrap();
        log.append(
            ordered.clone(),
            Arrival::ordered(i64::MAX as u64, a_message_id(1)),
        )
        .unwrap();

        let ids: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![ordered.op.id(), unordered.op.id()],
            "the colliding Lamport value must still beat an unordered op"
        );
    }

    #[test]
    fn an_ordered_op_sorts_ahead_of_an_unordered_one_at_every_lamport_value() {
        // `cmp_ops` places every ordered op before every unordered one WHATEVER
        // the Lamport value, and the sort key has to express that.
        //
        // THIS IS THE TEST THAT CAUGHT THE SENTINEL BUG. An earlier draft
        // reserved `i64::MAX` in the Lamport column for "unordered" — and
        // `lamport_sort_key(0)` is exactly `i64::MAX`, so a Lamport-0 op
        // interleaved with the unordered ones instead of preceding them.
        // Lamport 0 is in this list for that reason and must stay.
        let unordered = SortKey::of(&Arrival::unordered());
        for lamport in [0u64, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX] {
            let ordered = SortKey::of(&Arrival::ordered(lamport, a_message_id(1)));
            assert!(
                ordered.ordered < unordered.ordered,
                "lamport {lamport} must still sort ahead of an unordered op"
            );
        }
    }

    #[test]
    fn an_unordered_arrivals_message_id_is_absent_from_its_sort_key() {
        // `cmp_ops`'s `(None, None)` Lamport arm goes STRAIGHT to the op id and
        // never consults the message id. SQL has no short-circuit, so a sort
        // key that carried the id would order two unordered ops by it — which
        // is the divergence the two-implementation agreement test caught.
        //
        // Pinned here as well as there because this is the structural half: the
        // agreement test says the two implementations match, and this says WHY,
        // so a future edit that reintroduced the id would fail with a name that
        // points at the cause.
        let with_id = SortKey::of(&Arrival::from_parts(None, Some(a_message_id(7))));
        let without = SortKey::of(&Arrival::unordered());
        assert_eq!(with_id.msg_present, 0);
        assert_eq!(with_id.msg, None);
        assert_eq!(
            with_id, without,
            "two unordered arrivals must have identical sort keys, \
             so only the op id can separate them"
        );
    }

    #[test]
    fn an_ordered_arrivals_message_id_is_present_in_its_sort_key() {
        // The other direction, so the test above cannot be satisfied by a key
        // that simply never carries a message id.
        let with_id = SortKey::of(&Arrival::ordered(5, a_message_id(7)));
        let without = SortKey::of(&Arrival::from_parts(Some(5), None));
        assert_eq!(with_id.msg_present, 1);
        assert_eq!(with_id.msg, Some(a_message_id(7).as_bytes().to_vec()));
        assert_eq!(without.msg_present, 0);
        // And presence sorts first, which is `cmp_tiebreak`'s rule: the column
        // is ordered DESC, so 1 precedes 0.
        assert!(with_id.msg_present > without.msg_present);
    }

    #[test]
    fn an_empty_message_id_is_present_rather_than_absent_in_the_sort_key() {
        // The empty message id is a legal value, so presence cannot be encoded
        // as "the blob is non-empty". Collapsing them would reorder an op
        // carrying an empty id as though the transport had supplied none.
        let empty = SortKey::of(&Arrival::ordered(5, MessageId::new(vec![])));
        assert_eq!(empty.msg_present, 1, "an empty message id is still present");
        assert_eq!(empty.msg, Some(vec![]));
    }

    // ─── The indexes actually serve the order ─────────────────────────────

    /// SQLite's plan for a query, as one string.
    fn query_plan(log: &SqliteOpLog, sql: &str) -> String {
        let mut stmt = log
            .conn
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        rows.join(" | ")
    }

    #[test]
    fn every_ordered_read_is_served_by_an_index_rather_than_a_sort() {
        // **The one property of this module that no behavioural test can
        // see.** A review confirmed it: changing `ops_order`'s
        // `sort_msg_present DESC` to plain ascending passes all 443 tests, and
        // `EXPLAIN QUERY PLAN` turns `SCAN USING INDEX` into
        // `... USE TEMP B-TREE FOR LAST 3 TERMS`.
        //
        // That is exactly the degradation this file's header says the sort key
        // exists to prevent — §2.5's paginated reads becoming a
        // materialise-and-sort — and it is invisible to every gate because the
        // RESULTS are identical. Only the cost changes, and only at a size no
        // test builds.
        //
        // Asserted on the absence of a temp B-tree rather than on the exact
        // plan text, because plan wording is SQLite's to change between
        // versions and the property is "no sort", not "this sentence".
        let log = SqliteOpLog::in_memory().unwrap();
        let order_by = "ORDER BY sort_ordered ASC, sort_lamport ASC, \
                        sort_msg_present DESC, sort_msg ASC, op_id ASC";

        let reads = [
            ("unrestricted", format!("SELECT op_id FROM ops {order_by}")),
            (
                "by Stoa",
                format!("SELECT op_id FROM ops WHERE stoa = x'00' {order_by}"),
            ),
            (
                "by target",
                format!("SELECT op_id FROM ops WHERE target = x'00' {order_by}"),
            ),
        ];

        for (name, sql) in reads {
            let plan = query_plan(&log, &sql);
            assert!(
                !plan.to_uppercase().contains("TEMP B-TREE"),
                "the {name} read is sorted rather than index-walked: {plan}"
            );
            assert!(
                plan.to_uppercase().contains("INDEX"),
                "the {name} read does not use an index at all: {plan}"
            );
        }
    }

    // ─── Nothing here panics ──────────────────────────────────────────────

    #[test]
    fn opening_a_path_that_cannot_be_a_database_is_an_error_and_not_a_panic() {
        // PHASE0-FINDINGS §3: a panic aborts the module process. The store's
        // path comes from the host, so a bad one must be an answer.
        let dir = TempDir::new("bad-path");
        // A directory where a file is expected: SQLite cannot open it, and the
        // failure must arrive as a `Result`.
        let as_dir = dir.file("a-directory");
        std::fs::create_dir_all(&as_dir).unwrap();

        match SqliteOpLog::open(&as_dir) {
            Err(OpLogError::Storage(_)) => {}
            other => panic!("opening a directory must be a storage error, got {other:?}"),
        }
    }

    #[test]
    fn a_corrupt_stored_op_is_reported_rather_than_decoded() {
        // The store worked and what it returned did not — a different fact from
        // a disk failure, and one that points at a hand-edited file rather than
        // at the hardware.
        let dir = TempDir::new("corrupt");
        let path = dir.file("log.sqlite");

        let op = signed(a_post("about to be corrupted"));
        let id = op.op.id();
        let mut log = SqliteOpLog::open(&path).unwrap();
        log.append(op, Arrival::unordered()).unwrap();
        drop(log);

        // Replace the stored bytes with something that is not an op. The row is
        // otherwise intact, so this exercises the decode and not the schema.
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE ops SET op_bytes = ?1",
            rusqlite::params![vec![0xFFu8; 8]],
        )
        .unwrap();
        drop(conn);

        let reopened = SqliteOpLog::open(&path).unwrap();
        match reopened.get(&id) {
            Err(OpLogError::CorruptEntry(_)) => {}
            other => panic!("a corrupt op must be reported as corrupt, got {other:?}"),
        }
        // And the ordered read reports it too, rather than silently skipping
        // the row — a read that dropped undecodable entries would quietly
        // shrink a peer's history.
        match reopened.iter() {
            Err(OpLogError::CorruptEntry(_)) => {}
            other => panic!("a corrupt op must not be skipped by a read, got {other:?}"),
        }
    }

    #[test]
    fn every_error_variant_renders_differently() {
        // Two errors that read the same are one error with two names, and a
        // reader cannot act on the difference. `keystore.rs` pins the same
        // property for the same reason.
        let rendered = [
            OpLogError::Storage("disk".to_string()).to_string(),
            OpLogError::UnknownLayoutVersion {
                found: 2,
                expected: 1,
            }
            .to_string(),
            // The two layout errors are the pair most at risk of reading the
            // same, and they are the pair a reader most needs told apart:
            // `UnknownLayoutVersion` says "find the build that wrote this",
            // which is advice this one cannot act on — this build IS that
            // build, and the file is mislabelled.
            OpLogError::LayoutDoesNotMatchItsVersion {
                version: 1,
                why: "no such table: ops".to_string(),
            }
            .to_string(),
            OpLogError::CorruptEntry("bytes".to_string()).to_string(),
        ];
        for (i, a) in rendered.iter().enumerate() {
            for b in rendered.iter().skip(i + 1) {
                assert_ne!(a, b, "two error variants render identically");
            }
        }
    }
}
