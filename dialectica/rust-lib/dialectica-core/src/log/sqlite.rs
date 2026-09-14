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

use super::{Appended, Entry, OpLog, OpLogError};
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
/// **Bumped to 2 by the op clock.** The sort columns previously derived from the
/// `Arrival` at write time, and now derive from the op's own counter — so a
/// store written by a version-1 build holds rows whose `sort_*` values answer a
/// different question, and reading them would produce a silently wrong order
/// rather than an error. The existing check refuses an unrecognised layout
/// rather than migrating it, which is the behaviour this bump relies on.
pub const LAYOUT_VERSION: i32 = 2;

/// Map a Lamport counter onto an ascending sort key that reverses it.
///
/// # Why a mapping rather than `ORDER BY sort_counter DESC`
///
/// `cmp_ops` wants descending counter among the ops that carry one. Reversing at
/// **write** time makes one ascending index serve it, where a `DESC` on the
/// stored value would need its own index to be scanned rather than sorted — and
/// the whole point of materialising a sort key is that §2.5's paginated reads
/// become an index walk.
///
/// # Why not simply negate
///
/// `-(u64::MAX as i64)` does not fit. SQLite's `INTEGER` is an `i64` and a
/// counter is a `u64`, so the map has to be a bijection between the two ranges
/// rather than an arithmetic negation. This one is: `u64::MAX - counter`
/// reverses within `u64`, and adding `i64::MIN` reinterprets the result across
/// the signed range. `0` maps to `i64::MAX`, `u64::MAX` maps to `i64::MIN`.
///
/// # The sentinel is NOT in this column
///
/// An earlier draft reserved `i64::MAX` here for "this op carries no counter",
/// and it was wrong: `counter == 0` maps to exactly that value, so a counter-0
/// op would have shared a sort key with every op carrying none and interleaved
/// with them by op id. `cmp_ops` requires every op carrying a counter to precede
/// every op that does not **whatever the counter**, and a counter of zero is
/// precisely where a sentinel-based design breaks.
///
/// Every repair inside one `i64` column fails for the same reason — `u64` and
/// `i64` have the same cardinality, so a bijection leaves no spare value. The
/// fix is [`SortKey::has_counter`], a separate leading column, which costs one
/// integer per row and makes the partition structural instead of arithmetic.
fn counter_sort_key(counter: u64) -> i64 {
    (u64::MAX - counter).wrapping_add(i64::MIN as u64) as i64
}

/// [`cmp_ops`](crate::arrival::cmp_ops), materialised as two stored columns.
///
/// # Derived from the OP, never from the arrival
///
/// This previously took an [`Arrival`] and reversed the transport's Lamport
/// value. It now takes the op, because the op carries the counter that orders
/// it. The recorded arrival is still stored, in its own columns, and no longer
/// reaches the sort key at all — which is the storage-layer form of `OpEntry` no
/// longer carrying an `Arrival`.
///
/// **Two columns rather than the four this used to need.** The pair that carried
/// the transport's message-id tiebreak is gone with the tiebreak, and their
/// disappearance removes the defect they existed to work around: `cmp_ops` used
/// to short-circuit past the message id in its degraded arm while an SQL
/// `ORDER BY` — which has no short-circuit — compared it anyway, so an unordered
/// op carrying a message id sorted ahead of one without. Two honest peers, one
/// in memory and one on disk, rendered one thread differently. There is now no
/// column for SQL to over-consult.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SortKey {
    /// `0` when the op carries a counter, `1` when it does not.
    ///
    /// **The leading column, and ascending over it is `cmp_ops`'s partition.**
    /// A column of its own rather than a sentinel inside [`SortKey::counter`],
    /// because `u64` and `i64` have the same cardinality: a bijection between
    /// them leaves no value spare to mean "absent", and every candidate sentinel
    /// is some real counter's image. `0` before `1` so that ascending is the
    /// right direction for every column in the key, which is what lets one index
    /// serve the whole `ORDER BY`.
    has_counter: i64,
    /// Descending counter, reversed at write time — see [`counter_sort_key`].
    /// Meaningless when `has_counter` is `1`, and never reached, because the
    /// leading column has already decided.
    counter: i64,
}

impl SortKey {
    /// Derive the two columns from the op's own clock.
    fn of(op: &SignedOp) -> Self {
        match op.op.clock {
            Some(clock) => SortKey {
                has_counter: 0,
                counter: counter_sort_key(clock.counter),
            },
            // The degraded branch: `cmp_ops` compares op ids and NOTHING else.
            // The counter key is zeroed rather than left meaningful, because a
            // stored value that must not be read is one an `ORDER BY` will
            // eventually read.
            None => SortKey {
                has_counter: 1,
                counter: 0,
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
        conn.query_row(
            &format!(
                "SELECT {SELECT_COLUMNS}, op_id, stoa, target, author, score_epoch,
                        sort_has_counter, sort_counter
                 FROM ops LIMIT 0"
            ),
            [],
            |_| Ok(()),
        )
        // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS
        // case and every other error is the layout being wrong. Matching on it
        // rather than using `optional()` keeps that reading explicit: the
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
                 -- RECORDED, AND IT ORDERS NOTHING. These columns are a peer's
                 -- honest note of one delivery. No read consults them for
                 -- position, and the ORDER BY below does not name them — the
                 -- sort columns come from the op's own counter.
                 --
                 -- Kept rather than dropped because discarding a fact a peer
                 -- genuinely recorded, in order to make an ordering change, is
                 -- throwing away evidence to tidy a schema.
                 arrival_lamport   INTEGER,
                 arrival_msg       BLOB,

                 -- ── The read order, materialised ──────────────────────────
                 -- `cmp_ops` expressed over stored values so it can be
                 -- indexed. Written once at append, from the OP's own clock,
                 -- which is fixed at signing and cannot differ between two
                 -- receipts of one op — both clock fields are inside the
                 -- preimage, so two arrivals of one op carry identical values.

                 -- 0 = this op carries a counter, 1 = it does not.
                 -- THE LEADING COLUMN, and `cmp_ops`'s partition made
                 -- structural: every op carrying a counter precedes every op
                 -- that does not, whatever the counter. A column rather than a
                 -- sentinel inside `sort_counter`, because u64 and i64 have the
                 -- same cardinality and a bijection leaves no value spare. See
                 -- `counter_sort_key`, which records the draft that got this
                 -- wrong and how a counter of 0 exposes it.
                 sort_has_counter  INTEGER NOT NULL,

                 -- Descending counter, reversed at write time. Meaningless when
                 -- `sort_has_counter` is 1, and never reached in that case.
                 sort_counter      INTEGER NOT NULL,

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
                 -- The time a score decays FROM: the op's OWN Lamport counter,
                 -- or NULL where the op carries none.
                 --
                 -- It came from the recorded arrival before the op clock, and
                 -- now comes from the op, which is a strict improvement for
                 -- exactly the reason the note below about wall-clocks gives: a
                 -- decay epoch two peers disagree about is a ranking two peers
                 -- disagree about. The op's counter is identical on every peer
                 -- holding the op; a recorded arrival never was.
                 --
                 -- NULL AND NOT A SENTINEL. `-1` was the first draft and it
                 -- was wrong for the reason `counter_sort_key` gives about its
                 -- own column: `u64::MAX as i64` IS `-1`, so an op carrying no
                 -- counter and one at the maximum stored the same value. A
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
                 -- NOT A WALL-CLOCK READING OF ANY KIND, and that is the whole
                 -- point. Not the receiving peer's clock, which is a per-peer
                 -- CLOCK_REALTIME read — two peers would rank the same ops
                 -- differently because they received them at different
                 -- instants. And NOT the op's author-asserted wall-clock
                 -- either, which is the newly available mistake: it is a field
                 -- the adversary sets, so a decay reading it would let a post
                 -- claiming a future instant pin itself above every honest one
                 -- permanently. PLAN Appendix A measures that exact failure in
                 -- the nearest kin project. The counter is the only value here
                 -- that is both shared across peers and not chooseable for
                 -- rank.
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
                 ON ops (sort_has_counter, sort_counter, op_id);

             -- The two restricted reads, each prefixed by what it restricts on
             -- so that the same ordering is served without a sort.
             CREATE INDEX ops_by_stoa
                 ON ops (stoa, sort_has_counter, sort_counter, op_id);
             CREATE INDEX ops_by_target
                 ON ops (target, sort_has_counter, sort_counter, op_id);

             -- RESERVED, and nothing queries it yet. `(target, author)` is the
             -- shape a vote tally needs: 'the ops about this subject, grouped
             -- by who signed them', which a later ranking change weights
             -- against the moderator set it resolves at read time. It is
             -- created now rather than later because adding an index to a
             -- peer's existing store is cheap only while the store is small,
             -- and because an index nothing uses costs writes and nothing else
             -- — which is the right side of rule 5's trade.
             CREATE INDEX ops_by_target_author ON ops (target, author);

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
    /// The `ORDER BY` is `cmp_ops`: ascending `sort_has_counter` (ops carrying
    /// one first), then ascending `sort_counter` (which is the counter
    /// reversed, so descending counter), then op id as the last resort —
    /// matching `cmp_ops`'s three arms branch for branch.
    ///
    /// **No column of the recorded arrival appears here**, which is the storage
    /// half of "ordering does not consult the transport".
    fn ordered_read(
        &self,
        where_clause: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<Entry>, OpLogError> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS}
             FROM ops
             {where_clause}
             ORDER BY sort_has_counter ASC, sort_counter ASC, op_id ASC"
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
        let key = SortKey::of(&entry.op);
        // The op's OWN counter, never the recorded arrival's Lamport value.
        //
        // `NULL` for "this op carries no counter", never a sentinel. **An
        // earlier draft used `-1` and that was the collision this file's own
        // `counter_sort_key` documentation rejects**: `u64::MAX as i64` IS
        // `-1`, so an op carrying none and one at `u64::MAX` stored the same
        // epoch and became indistinguishable. `u64` and `i64` have the same
        // cardinality, so no in-band sentinel can work; the same reasoning that
        // produced `sort_has_counter`.
        let score_epoch = entry.op.op.clock.map(|c| c.counter as i64);

        let changed = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO ops
                     (op_id, op_bytes, stoa, target, author,
                      arrival_lamport, arrival_msg,
                      sort_has_counter, sort_counter,
                      score_epoch)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id.as_bytes().as_slice(),
                    entry.op.to_bytes(),
                    entry.op.op.stoa.as_bytes().as_slice(),
                    entry.target().map(|t| t.as_bytes().to_vec()),
                    entry.op.op.author.to_bytes().as_slice(),
                    // The arrival as RECORDED — both fields, and nothing
                    // downstream orders by either.
                    entry.arrival.lamport().map(|l| l as i64),
                    entry.arrival.message_id().map(|m| m.as_bytes().to_vec()),
                    key.has_counter,
                    key.counter,
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
        // **2 since the op clock.** The `sort_*` columns used to derive from the
        // recorded `Arrival` and now derive from the op's own counter, so a
        // version-1 file holds rows whose sort values answer a different
        // question. Reading them through this build would produce a silently
        // wrong order rather than an error, which is exactly what the bump — and
        // the refuse-rather-than-migrate check below — exists to prevent.
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
        // A NEGATIVE version, not `LAYOUT_VERSION - 1`. `LAYOUT_VERSION - 1` is
        // `1` today, which is a real layout an earlier build genuinely wrote —
        // and it was `0` when the version was 1, which is SQLite's "never
        // stamped" value and legitimately means a fresh file, so a test using
        // the arithmetic form would have asserted the opposite of what it names.
        // A test whose meaning flips as the version is bumped is not a test of
        // the property it claims.
        //
        // `-1` is below this build's version, is not the fresh sentinel, is not
        // any layout anything ever wrote, and stays below whatever
        // `LAYOUT_VERSION` becomes. That is the property the test needs, stated
        // so a later reader does not "simplify" it back.
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
            other => {
                panic!("a table named `ops` that is not our `ops` must be refused, got {other:?}")
            }
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

        // `lamport().is_some()` rather than the removed
        // `is_ordered_by_transport()`. The predicate went with the ordering
        // role; the RECORD is still a record, and this test is about whether it
        // survives a round trip — which it must, because a peer's note of what
        // it received is a fact whether or not anything orders by it.
        assert!(back(&ordered).lamport().is_some());
        assert!(back(&unordered).lamport().is_none());

        // A message id recorded against an arrival carrying no Lamport value —
        // the shape an SDS ephemeral message produces. The record must keep it,
        // and NOTHING orders by it.
        let recovered = back(&id_only);
        assert!(recovered.lamport().is_none());
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
        // **No ordering assertion here any more, and its removal is the point.**
        // This test used to close by asserting the pair read back in descending
        // Lamport order. That assertion belonged to a rule that no longer holds:
        // the recorded Lamport value orders nothing, and these two ops carry no
        // counter, so `cmp_ops` puts them in ascending op-id order regardless of
        // what either arrival recorded. Re-adding an ordering assertion here
        // would be asserting the op-id fallback in a test named for a round
        // trip; `ops_the_transport_did_not_order_read_in_ascending_op_id` is
        // where that belongs.
        //
        // What IS still worth pinning is that both rows are readable: a round
        // trip that lost one would otherwise leave the `get`s above passing
        // against a store that cannot enumerate them.
        assert_eq!(reopened.len().unwrap(), 2);
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
    fn an_op_with_no_counter_and_one_at_the_maximal_counter_store_different_score_epochs() {
        // **A DEFECT A REVIEW FOUND, and the test that would have caught it.**
        //
        // `score_epoch` was written as `map_or(-1, |c| c as i64)`. But
        // `u64::MAX as i64` IS `-1`, so an op carrying no counter and an op at
        // counter `u64::MAX` stored the SAME value and became indistinguishable
        // — the exact in-band-sentinel collision `counter_sort_key`'s
        // documentation rejects for its own column, and which `sort_has_counter`
        // exists to avoid.
        //
        // It was invisible because the column is reserved: "nothing writes
        // this and no read consults it" is a comment that excuses a column
        // from every behavioural test, while the rows go to every peer's disk.
        // It would have become a §7.2 ranking bug the moment rule 5 landed.
        //
        // `u64::MAX` is in this test specifically because it is the ONLY value
        // whose `as i64` cast collides with a plausible sentinel.
        //
        // **The epoch comes from the OP's counter, not from the recorded
        // arrival**, which is why both ops here are given the SAME arrival: if
        // the arrival still reached the column, the two would store the same
        // epoch and this test would say so.
        let mut log = SqliteOpLog::in_memory().unwrap();
        let no_counter = signed(a_post("no counter at all"));
        let maximal = signed(a_post_at("counter u64::MAX", u64::MAX));

        log.append(no_counter.clone(), Arrival::unordered())
            .unwrap();
        log.append(maximal.clone(), Arrival::unordered()).unwrap();

        let for_no_counter = stored_score_epoch(&log, &no_counter.op.id());
        let for_maximal = stored_score_epoch(&log, &maximal.op.id());

        // Hardcoded on both sides rather than merely asserting they differ: a
        // test that only checked inequality would pass for any two sentinels,
        // including a second collision-prone pair.
        assert_eq!(
            for_no_counter, None,
            "an op carrying no counter must store NULL, not a sentinel"
        );
        assert_eq!(
            for_maximal,
            Some(-1),
            "u64::MAX casts to -1, which is why NULL and not -1 is the absence"
        );
        assert_ne!(
            for_no_counter, for_maximal,
            "absence and u64::MAX must be distinguishable in the reserved column"
        );
    }

    #[test]
    fn the_score_epoch_round_trips_every_counter_boundary() {
        // The reserved column holds the value a later decay will read, so the
        // cast has to be reversible across the whole domain — and the
        // boundaries are the only place a cast breaks.
        let mut log = SqliteOpLog::in_memory().unwrap();
        for counter in [0u64, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX] {
            let op = signed(a_post_at(&format!("counter {counter}"), counter));
            // A RECORDED arrival that says something else entirely, on every
            // iteration: the epoch must be the op's counter and not the
            // transport's Lamport value, and an arrival agreeing with the
            // counter would make the two explanations indistinguishable.
            log.append(op.clone(), Arrival::ordered(42, a_message_id(1)))
                .unwrap();
            let stored = stored_score_epoch(&log, &op.op.id())
                .expect("an op carrying a counter stores an epoch");
            assert_eq!(
                stored as u64, counter,
                "the score epoch must round-trip counter {counter}"
            );
        }
    }

    #[test]
    fn the_score_epoch_is_the_ops_counter_and_not_the_recorded_lamport_value() {
        // The two explanations, separated. Every test above could pass for an
        // implementation that still read `arrival.lamport()` if the arrival ever
        // happened to agree — so this one builds the disagreement directly: an
        // op whose counter and whose recorded Lamport value are different
        // numbers, plus an op carrying no counter whose arrival records one.
        //
        // The second is the sharper half. Under the old rule it stored an epoch;
        // under this one it must store NULL, because the op itself asserts no
        // position and a peer's note of when it arrived is not one.
        let mut log = SqliteOpLog::in_memory().unwrap();
        let disagreeing = signed(a_post_at("counter 5, recorded 900", 5));
        let arrival_only = signed(a_post("no counter, recorded 900"));

        log.append(disagreeing.clone(), Arrival::ordered(900, a_message_id(1)))
            .unwrap();
        log.append(arrival_only.clone(), Arrival::ordered(900, a_message_id(2)))
            .unwrap();

        assert_eq!(
            stored_score_epoch(&log, &disagreeing.op.id()),
            Some(5),
            "the epoch must be the op's counter, not the recorded Lamport value"
        );
        assert_eq!(
            stored_score_epoch(&log, &arrival_only.op.id()),
            None,
            "an op carrying no counter has no epoch, whatever the transport recorded"
        );
    }

    // ─── The sort key ─────────────────────────────────────────────────────

    #[test]
    fn the_counter_sort_key_reverses_the_order_across_the_whole_u64_range() {
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
                counter_sort_key(lower).cmp(&counter_sort_key(higher)),
                Ordering::Greater,
                "the sort key must reverse: {lower} vs {higher}"
            );
        }
        // Injective at the boundary, which the reversal alone does not imply:
        // a saturating map is still weakly decreasing.
        assert_ne!(counter_sort_key(u64::MAX), counter_sort_key(u64::MAX - 1));
        assert_ne!(counter_sort_key(0), counter_sort_key(1));
    }

    #[test]
    fn the_counter_key_collides_with_the_no_counter_filler_and_the_leading_column_is_why_that_is_safe(
    ) {
        // `SortKey::of` fills a no-counter op's `counter` with `0`. That is NOT
        // a spare value — `counter_sort_key(i64::MAX as u64)` is exactly `0`, so
        // a real counter maps onto the filler.
        //
        // The behaviour is correct, because `sort_has_counter` leads the ORDER
        // BY and decides before the counter column is consulted. Asserted as a
        // COLLISION rather than avoided, because the collision is real and the
        // defence is structural. A future edit that removes the leading column
        // fails here, with a name that says what to look at, instead of failing
        // somewhere in a sequence comparison.
        let colliding = i64::MAX as u64;
        assert_eq!(
            counter_sort_key(colliding),
            0,
            "this is the counter whose key collides with the no-counter filler"
        );

        let with = SortKey::of(&signed(a_post_at("carries one", colliding)));
        let without = SortKey::of(&signed(a_post("carries none")));
        assert_eq!(
            with.counter, without.counter,
            "the collision is real: the counter column cannot separate these"
        );
        assert!(
            with.has_counter < without.has_counter,
            "so `sort_has_counter` must, and it is the only thing that does"
        );
    }

    #[test]
    fn an_op_at_the_colliding_counter_still_reads_before_one_carrying_none() {
        // The consequence of the collision above, asserted end to end through a
        // real read rather than only on the key. A sort that consulted
        // `sort_counter` before `sort_has_counter` would interleave these two.
        //
        // The op CARRYING a counter is given the HIGHER op id, so a read falling
        // back to the op id would put the other one first and fail.
        let (lower_id, higher_id) = two_bodies_by_ascending_id();
        let without = signed(a_post(&lower_id));
        let with = signed(a_post_at(&higher_id, i64::MAX as u64));
        assert!(
            with.op.id() > without.op.id(),
            "the fixture must give the counter-carrying op the higher id"
        );

        let mut log = SqliteOpLog::in_memory().unwrap();
        log.append(without.clone(), Arrival::unordered()).unwrap();
        log.append(with.clone(), Arrival::unordered()).unwrap();

        let ids: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![with.op.id(), without.op.id()],
            "the colliding counter must still beat an op carrying none"
        );
    }

    /// Two bodies such that `a_post(first)` has the lower id and
    /// `a_post_at(second, i64::MAX as u64)` has the higher.
    ///
    /// Searched rather than hardcoded: which of two bodies hashes lower is not
    /// something a reader should take on trust, and a stale hardcoded guess
    /// would make the test above pass for the wrong reason.
    fn two_bodies_by_ascending_id() -> (String, String) {
        for n in 0..1000u32 {
            let low = format!("no counter {n}");
            let high = format!("with counter {n}");
            if a_post(&low).id() < a_post_at(&high, i64::MAX as u64).id() {
                return (low, high);
            }
        }
        panic!("no such pair in 1000 candidates, which is astronomically unlikely");
    }

    #[test]
    fn an_op_carrying_a_counter_sorts_ahead_of_one_carrying_none_at_every_counter() {
        // `cmp_ops` places every op carrying a counter before every op that does
        // not, WHATEVER the counter, and the sort key has to express that.
        //
        // THIS IS THE TEST THAT CAUGHT THE SENTINEL BUG. An earlier draft
        // reserved `i64::MAX` in the counter column for "carries none" — and
        // `counter_sort_key(0)` is exactly `i64::MAX`, so a counter-0 op
        // interleaved with the others instead of preceding them. Counter 0 is in
        // this list for that reason and must stay.
        let without = SortKey::of(&signed(a_post("none")));
        for counter in [0u64, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX] {
            let with = SortKey::of(&signed(a_post_at("some", counter)));
            assert!(
                with.has_counter < without.has_counter,
                "counter {counter} must still sort ahead of an op carrying none"
            );
        }
    }

    #[test]
    fn the_recorded_arrival_does_not_reach_the_sort_key_at_all() {
        // The structural half of "ordering does not consult the transport".
        // `SortKey::of` takes a `SignedOp` and cannot see an `Arrival`, so this
        // asserts the consequence: one op yields ONE sort key however wildly the
        // recorded arrival varies.
        //
        // Stated as a test rather than left to the type, because the type is
        // exactly what a future edit would widen — and this fails with a name
        // that says what was undone.
        let op = signed(a_post_at("one op", 5));
        let key = SortKey::of(&op);

        // Every arrival shape the recorder can produce. None of them is an
        // input to `SortKey::of`, so none can change the answer.
        for arrival in [
            Arrival::unordered(),
            Arrival::ordered(0, a_message_id(1)),
            Arrival::ordered(u64::MAX, MessageId::new(vec![])),
            Arrival::from_parts(None, Some(a_message_id(9))),
            Arrival::from_parts(Some(12_345), None),
        ] {
            let mut log = SqliteOpLog::in_memory().unwrap();
            log.append(op.clone(), arrival).unwrap();
            // Read the stored sort columns back, which is what the ORDER BY
            // compares — a stronger statement than re-calling `SortKey::of`.
            let (has_counter, counter): (i64, i64) = log
                .conn
                .query_row(
                    "SELECT sort_has_counter, sort_counter FROM ops WHERE op_id = ?1",
                    rusqlite::params![op.op.id().as_bytes().as_slice()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                (has_counter, counter),
                (key.has_counter, key.counter),
                "the recorded arrival must not reach the stored sort key"
            );
        }
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
        // see.** A review confirmed it on the four-column key this replaced:
        // flipping one of `ops_order`'s columns to the opposite direction
        // passed the entire suite, and `EXPLAIN QUERY PLAN` turned
        // `SCAN USING INDEX` into `... USE TEMP B-TREE FOR LAST N TERMS`.
        //
        // That is exactly the degradation this file's header says the sort key
        // exists to prevent — §2.5's paginated reads becoming a
        // materialise-and-sort — and it is invisible to every gate because the
        // RESULTS are identical. Only the cost changes, and only at a size no
        // test builds.
        //
        // **The key is ascending in every column on purpose**, which is what
        // reversing the counter at WRITE time buys (see `counter_sort_key`): one
        // ascending index serves the whole `ORDER BY`. A `DESC` on any column
        // here would need an index of its own or reintroduce the temp B-tree.
        //
        // Asserted on the absence of a temp B-tree rather than on the exact
        // plan text, because plan wording is SQLite's to change between
        // versions and the property is "no sort", not "this sentence".
        let log = SqliteOpLog::in_memory().unwrap();
        // The string this pins must be `ordered_read`'s, verbatim. It is
        // duplicated rather than shared because a test reading the production
        // `ORDER BY` out of the production function would pass for any string
        // both agreed on — including a wrong one.
        let order_by = "ORDER BY sort_has_counter ASC, sort_counter ASC, op_id ASC";

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
