//! Which Stoas this peer is in, and the genesis record it holds for each.
//!
//! # Why this is not the op log
//!
//! The `op-log` capability owns a record of **what arrived**; this owns a record
//! of **what the user chose**. Neither is derivable from the other, and the two
//! disagreeing is ordinary rather than a fault:
//!
//! - A Stoa joined and since silent has **no ops at all**, so an op-derived
//!   answer omits precisely the Stoas a user has just acted on — which is when
//!   they are most certain they are in one.
//! - An op addressed to a Stoa the peer never heard of arrives whether or not
//!   anybody wanted it. `Op::stoa` is a field the **sender** chose, so a peer
//!   that joined a Stoa because an op mentioned it would be a peer any stranger
//!   can enrol.
//!
//! The second direction is why this module and [`crate::log`] share no state and
//! no call: there is no path from [`crate::log::OpLog::append`] to anything here.
//! The requirement is satisfied by the absence of a call rather than by a check
//! that has to be right at every append site.
//!
//! # The record is retained, not only the address
//!
//! A Stoa address is a one-way hash of its genesis record, and
//! [`crate::moderation::Moderators::of`] takes the **record**. So a peer that
//! retained only addresses would hold a list of Stoas whose content it can store
//! and cannot judge — and the deficiency would be invisible until a moderation op
//! arrived, which is the worst moment to discover it.
//!
//! The record is stored as [`Genesis::canonical_bytes`] **verbatim**, never as
//! decomposed columns. The address *is* the hash of that encoding, so
//! re-encoding on read would make every stored address depend on this module's
//! re-encoding agreeing with `stoa.rs`'s forever. Same argument
//! `log/sqlite.rs` makes for storing `op_bytes` rather than fields.
//!
//! # This file, and not a table beside `ops`
//!
//! `design.md` has the table of alternatives; the short version is that
//! [`crate::log::sqlite`]'s `create_schema` runs **only** when `user_version`
//! reads `0`, so a table added to it reaches fresh stores and never an existing
//! one — and the two ways round that are a version bump the spec forbids, or a
//! `CREATE TABLE IF NOT EXISTS` that silently repairs a file `check_layout`
//! exists to refuse.
//!
//! A separate file makes "adding membership does not make an existing store
//! unreadable" hold **by construction**: nothing in this change edits
//! `SqliteOpLog`, so there is no version of the op log's layout that has to mean
//! two things.
//!
//! # Nothing here panics
//!
//! A genesis record arrives from a peer, and PHASE0-FINDINGS §3 measured that an
//! unguarded panic **aborts the module process**. So outside `#[cfg(test)]` there
//! is no `unwrap`, no `expect` and no indexing that could be out of bounds; every
//! `rusqlite` failure and every refusal becomes a [`MembershipError`].

use crate::identity::Address;
use crate::stoa::{Genesis, GenesisError};
use rusqlite::{Connection, OptionalExtension};
use std::fmt;
use std::path::Path;

/// The storage layout this build writes and understands for **memberships**.
///
/// Held in SQLite's own `PRAGMA user_version`, as the op log's is.
///
/// **Independent of [`crate::log::sqlite::LAYOUT_VERSION`] by design**, and
/// independent is not the same as distinguishing. **Both constants are `1`
/// today**, so the version check cannot tell the two stores apart at all: hand
/// [`MembershipStore::open`] an op-log file and `found == 1` equals this value,
/// so it takes the already-stamped branch. What refuses it is `check_layout`,
/// which names `stoa` and `genesis_bytes` and cannot prepare that statement
/// against an `ops` table. **Naming the columns is the whole of the boundary**;
/// do not read the version numbers as holding it.
///
/// What being separate files DOES buy, and it survives both constants being `1`:
/// neither refusal mentions the other, so a peer holding a membership store from
/// a future build cannot list its Stoas and can still read every op it holds.
/// That is the recoverable direction — one file makes an unknown membership
/// layout cost the user their ops.
///
/// **Pinned by a hardcoded assertion**, following `identity.rs`'s wire constants.
/// `cargo mutants` mutates functions and not `const`s, so a wrong version here is
/// invisible to it; this project has already shipped a `VERSION_1` defect that
/// left the whole suite green.
pub const MEMBERSHIP_LAYOUT_VERSION: i32 = 1;

/// One Stoa the peer is in.
///
/// The address and the record are kept as a pair rather than the address being
/// derived on demand, because a caller listing a page wants both and re-deriving
/// per row would hash the record again for an answer the row already carries.
/// [`MembershipStore`] guarantees they agree: nothing is stored whose record does
/// not verify against its address, and
/// `the_retained_record_still_verifies_against_its_address` holds that across a
/// restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    /// The Stoa's address — its identity, and the key the record is filed under.
    pub stoa: Address,
    /// The complete genesis record, decoded from the bytes that were stored.
    pub genesis: Genesis,
}

impl Membership {
    /// The **only** way to build a `Membership` from a caller's address and a
    /// caller's record: by verifying that they are the same Stoa.
    ///
    /// # Why this is a constructor and not a check inside `join`
    ///
    /// It was a check inside [`MembershipStore::join`], and a second identical one
    /// in `wire::genesis_for`. Two guards enforcing one property, and **no test
    /// distinguished them** — deleting either left the suite green, because
    /// whichever remained refused the same inputs. Both directions were measured:
    /// `findings/spec-test.md` entry 2 deleted the store's and 546 of 550 passed;
    /// deleting `genesis_for`'s left **550 of 550** passing.
    ///
    /// The project's rule for that shape is CLAUDE.md's: *"prefer reshaping state
    /// so an invariant holds by construction over adding a branch that checks
    /// it."* So the pair is now a type that cannot be built wrong. `join` takes a
    /// `Membership` and has no guard, because there is no longer an unverified pair
    /// for it to receive — and a caller reaching for one has to come through here,
    /// which is the one place the refusal is tested.
    ///
    /// **Verification consults only its two arguments.** No index, no registry, no
    /// peer, no network call. That is what makes a pasted address
    /// self-authenticating (§4.8): [`Genesis::matches`] re-derives the address from
    /// the record and compares, so a wrong or tampered record cannot survive, and
    /// detecting it needs nobody's cooperation.
    ///
    /// Encoding is checked first because it is the more specific failure: a record
    /// whose title exceeds the genesis cap has no canonical encoding and therefore
    /// no address at all, so it is not "the wrong Stoa" — it is not a Stoa. A
    /// caller telling a user "that record cannot be a Stoa" versus "that record is
    /// not the Stoa you pasted" needs the two apart.
    pub fn verified(stoa: &Address, genesis: &Genesis) -> Result<Self, MembershipError> {
        genesis
            .canonical_bytes()
            .map_err(MembershipError::UnencodableRecord)?;
        if !genesis.matches(stoa) {
            return Err(MembershipError::RecordDoesNotMatchAddress);
        }
        Ok(Membership {
            stoa: *stoa,
            genesis: genesis.clone(),
        })
    }
}

/// Why a membership operation did not happen.
///
/// Each variant names a **different** mistake, for the reason `GenesisError`
/// gives: a store that only says "invalid" sends the reader looking in the wrong
/// place.
///
/// **Not `Clone`**, unlike [`crate::log::OpLogError`], and the asymmetry is
/// deliberate rather than an omission: [`MembershipError::UnencodableRecord`]
/// carries a [`GenesisError`], which carries a `KeyError`, and neither is `Clone`.
/// Deriving it up the chain would widen two other modules' surfaces so that this
/// type could gain a trait nothing needs.
#[derive(Debug, PartialEq, Eq)]
pub enum MembershipError {
    /// The store could not be reached, opened, read or written.
    ///
    /// Carries the underlying description as a `String` rather than a database
    /// crate's error type, matching [`crate::log::OpLogError::Storage`].
    Storage(String),
    /// The store declares a layout this build does not understand.
    ///
    /// **Refused in both directions**, as the op log's is: accepting an older
    /// layout is the shape a well-meaning "backwards compatible" edit takes, and
    /// it would read an older file through this build's column positions.
    UnknownLayoutVersion { found: i32, expected: i32 },
    /// The store stamps a layout this build DOES understand, and is not it.
    ///
    /// `PRAGMA user_version` is one integer anything can write, so a file
    /// carrying this build's number is a claim rather than a fact. Refused at
    /// open rather than discovered at the first read, where it would arrive as
    /// `Storage("no such table: stoas")` — an error that blames the disk when the
    /// fact is that the file is mislabelled.
    LayoutDoesNotMatchItsVersion { version: i32, why: String },
    /// The supplied record is not the one the supplied address names.
    ///
    /// **The refusal that makes an address self-authenticating.** It consults
    /// nothing: no registry, no peer, no index. A wrong or tampered record fails
    /// to match, and the failure needs no one's cooperation to detect.
    RecordDoesNotMatchAddress,
    /// The supplied record has no canonical encoding, so it names no Stoa.
    ///
    /// **The ENCODE side, and the name says so because the message a caller sees
    /// is the whole of what it learns.** This was once called
    /// `UndecodableRecord` and rendered "the genesis record could not be read",
    /// which is the opposite operation: the only way to reach it is
    /// [`Genesis::canonical_bytes`] refusing a record — a title over the genesis
    /// cap — so nothing was ever read. A caller passing a 2000-byte title was
    /// told its record "could not be read: title is 2000 bytes".
    ///
    /// Distinct from [`MembershipError::RecordDoesNotMatchAddress`] because they
    /// are different mistakes: this one is a record with no address at all, that
    /// one is a record whose address is not the one claimed. A caller telling a
    /// user "that record cannot be a Stoa" versus "that record is not the Stoa
    /// you pasted" needs the two apart.
    ///
    /// There is no decode-side sibling here, and that is not an omission: a
    /// membership is joined from a `Genesis` this crate already decoded — `wire.rs`
    /// owns that decode and reports it — and a row read back that does not decode
    /// is [`MembershipError::CorruptEntry`], which points at the file rather than
    /// at the caller.
    UnencodableRecord(GenesisError),
    /// A retained row could not be read back as a membership.
    ///
    /// Distinct from [`MembershipError::Storage`] because the store worked
    /// perfectly and what it handed back did not — which points at a corrupted or
    /// hand-edited file rather than at the disk.
    ///
    /// **Reported, never skipped.** Skipping would make a corrupted row
    /// indistinguishable from a Stoa the user never joined, which is
    /// `OpLogError`'s "an empty feed is not an unreadable store" argument applied
    /// to a listing.
    CorruptEntry(String),
}

impl fmt::Display for MembershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MembershipError::Storage(why) => {
                write!(f, "the Stoa membership store could not be used: {why}")
            }
            MembershipError::UnknownLayoutVersion { found, expected } => write!(
                f,
                "this membership store was written with storage layout version {found}, \
                 and this build understands version {expected}; open it with a build that \
                 knows that layout rather than upgrading it in place"
            ),
            MembershipError::LayoutDoesNotMatchItsVersion { version, why } => write!(
                f,
                "this membership store declares storage layout version {version}, which \
                 this build understands, but does not have that layout: {why}; it was not \
                 written by this build and must not be read as though it were"
            ),
            MembershipError::RecordDoesNotMatchAddress => write!(
                f,
                "the genesis record does not hash to the Stoa address it was given with"
            ),
            MembershipError::UnencodableRecord(e) => {
                write!(
                    f,
                    "that genesis record cannot be encoded, so it names no Stoa: {e}"
                )
            }
            MembershipError::CorruptEntry(why) => write!(
                f,
                "a Stoa this peer is in could not be read back: {why}; the storage is \
                 readable but its contents are not what this build wrote"
            ),
        }
    }
}

impl std::error::Error for MembershipError {}

/// One page of memberships.
///
/// Mirrors the ecosystem's pagination shape with no total, for the reason
/// [`crate::feed::FeedPage`] gives: the question "is there another page here" is
/// answerable, and "how many are there anywhere" is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipPage {
    pub items: Vec<Membership>,
    pub page: usize,
    pub has_more: bool,
}

/// The Stoas this peer is in, on disk.
///
/// # The file is handed in, never discovered
///
/// [`MembershipStore::open`] takes a path, exactly as `SqliteOpLog::open` and
/// `Keystore` do, and for the same reason: a fixed path baked into a pure crate is
/// untestable and would mean this crate reading the environment at a moment its
/// caller does not control. The module crate has the host-stamped
/// `instance_persistence_path`.
#[derive(Debug)]
pub struct MembershipStore {
    conn: Connection,
}

impl MembershipStore {
    /// Open or create a membership store at `path`, refusing a layout this build
    /// cannot read.
    pub fn open(path: &Path) -> Result<Self, MembershipError> {
        let conn = Connection::open(path).map_err(storage)?;
        Self::from_connection(conn)
    }

    /// An ephemeral store with no file behind it.
    ///
    /// **Not a test double and not a second implementation.** It is this exact
    /// code — the same schema, the same statements — with SQLite's `:memory:`
    /// backing store, so a behavioural test that is not *about* persistence
    /// exercises the real SQL without a temporary directory or a teardown.
    ///
    /// Tests that ARE about persistence use [`MembershipStore::open`] against a
    /// real file, because this one by construction cannot survive being dropped.
    pub fn in_memory() -> Result<Self, MembershipError> {
        let conn = Connection::open_in_memory().map_err(storage)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, MembershipError> {
        // A fresh database reports 0, which is not a version anything wrote — it
        // is the absence of one, and the only case where creating the schema is
        // correct. It is also the case a store that predates membership takes,
        // which is what makes "adding membership does not make an existing store
        // unreadable" hold without a migration: there is nothing to convert,
        // because this file did not exist.
        let found: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;

        if found == 0 {
            Self::create_schema(&conn)?;
        } else if found != MEMBERSHIP_LAYOUT_VERSION {
            return Err(MembershipError::UnknownLayoutVersion {
                found,
                expected: MEMBERSHIP_LAYOUT_VERSION,
            });
        } else {
            // The version is a CLAIM, and this is the only thing that checks it.
            // Same reasoning as `log/sqlite.rs::check_layout`, and the same
            // ordering guarantee makes it safe: `create_schema` stamps the pragma
            // LAST, so a crash midway through creation leaves version 0 and takes
            // the create branch instead.
            Self::check_layout(&conn)?;
        }

        Ok(MembershipStore { conn })
    }

    /// Prove the store has the layout its `user_version` claims.
    ///
    /// Names the **columns** and not merely the table: `SELECT 1 FROM stoas`
    /// proves a table by that name exists and nothing about its shape, so a store
    /// whose columns were renamed or dropped would still pass.
    ///
    /// `LIMIT 0` reads no row — preparing and running the statement is what
    /// establishes the layout, and the cost does not grow with the store.
    fn check_layout(conn: &Connection) -> Result<(), MembershipError> {
        conn.query_row("SELECT stoa, genesis_bytes FROM stoas LIMIT 0", [], |_| {
            Ok(())
        })
        // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS case
        // and every other error is the layout being wrong. Matching on it rather
        // than using `optional()` keeps that reading explicit: the question asked
        // is "could this statement be prepared and run", not "did it find
        // anything".
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(()),
            other => Err(MembershipError::LayoutDoesNotMatchItsVersion {
                version: MEMBERSHIP_LAYOUT_VERSION,
                why: other.to_string(),
            }),
        })
    }

    /// The whole schema, with every column's reason beside it.
    ///
    /// # `PRAGMA user_version` IS LAST, AND MUST STAY LAST
    ///
    /// It is the commit point for the *layout claim*, and everything it claims has
    /// to be true before it is made. A crash or a failure anywhere before it
    /// leaves a file at version `0`, which `from_connection` reads as "never
    /// stamped" and creates cleanly. Moving it earlier strands a file at version 1
    /// with no tables, which `check_layout` then refuses forever.
    fn create_schema(conn: &Connection) -> Result<(), MembershipError> {
        let result = conn.execute_batch(&format!(
            "BEGIN;
             CREATE TABLE stoas (
                 -- The Stoa ADDRESS, and the identity. `PRIMARY KEY` rather than
                 -- a UNIQUE index over the record bytes, because the address is
                 -- what a Stoa IS — and because it is what makes a repeated join
                 -- idempotent structurally rather than by a branch: with
                 -- `INSERT OR IGNORE`, a second join of one Stoa cannot produce a
                 -- second row and cannot overwrite the first.
                 stoa           BLOB PRIMARY KEY NOT NULL,

                 -- `Genesis::canonical_bytes()` VERBATIM, never decomposed
                 -- columns. The address IS the hash of this encoding, so
                 -- re-encoding on read would make every stored address depend on
                 -- this module's re-encoding agreeing with `stoa.rs`'s forever.
                 --
                 -- It is also what makes the retained record answer the founding
                 -- title and the posting policy with no network call: they are
                 -- fields of what decodes out of these bytes.
                 genesis_bytes  BLOB NOT NULL
             ) STRICT;

             -- LAST, DELIBERATELY. See this function's documentation.
             PRAGMA user_version = {MEMBERSHIP_LAYOUT_VERSION};
             COMMIT;"
        ));

        if let Err(e) = result {
            // EXPLICIT, not left to `Drop`. `execute_batch` returns at the first
            // failing statement with the `BEGIN` still open; rusqlite rolling back
            // on drop makes that sound only for as long as every caller drops the
            // connection on this path. The rollback's own result is discarded on
            // purpose: the caller is owed the error that CAUSED the failure, not a
            // second one from cleaning up after it.
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(storage(e));
        }
        Ok(())
    }

    /// Record a Stoa this peer is in.
    ///
    /// # There is no verification here, and that is the point
    ///
    /// A [`Membership`] cannot be built from an address and a record that disagree
    /// — [`Membership::verified`] is the only constructor that takes a caller's pair
    /// and it refuses a mismatch. So this function has no guard to forget, no guard
    /// to duplicate, and no unverified pair it could be handed.
    ///
    /// It used to check, and `wire::genesis_for` checked the same predicate one
    /// layer up. Two guards, one property, and no test that could tell them apart:
    /// deleting either left the other refusing the same inputs, so the suite stayed
    /// green both ways (`findings/spec-test.md` entry 2 — 546/550 deleting the
    /// store's, and a re-run measured **550/550** deleting the wire's). The pair
    /// became a type instead of the guard becoming a third.
    ///
    /// The address is still the **caller's claim** about what it thinks it is
    /// joining and the record is still the material; that distinction did not move,
    /// it is just now enforced where the pair is made rather than where it is
    /// written.
    ///
    /// # Idempotent, and non-destructive
    ///
    /// `INSERT OR IGNORE`, never `INSERT OR REPLACE`. A pasted address is exactly
    /// the input a user supplies twice: reporting the second attempt as an error
    /// would make a harmless action look broken, and REPLACE would make a join a
    /// way to overwrite what a peer already holds.
    pub fn join(&mut self, membership: &Membership) -> Result<Joined, MembershipError> {
        // Infallible in practice and still handled: `Membership::verified` already
        // encoded this record once, so a record that reaches here has an encoding.
        // Re-encoding rather than carrying the bytes keeps `Membership` a plain
        // pair a caller can read, which is what `list` hands back.
        let bytes = membership
            .genesis
            .canonical_bytes()
            .map_err(MembershipError::UnencodableRecord)?;

        let changed = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO stoas (stoa, genesis_bytes) VALUES (?1, ?2)",
                rusqlite::params![membership.stoa.as_bytes().as_slice(), bytes],
            )
            .map_err(storage)?;

        Ok(if changed == 0 {
            Joined::AlreadyIn
        } else {
            Joined::Recorded
        })
    }

    /// What this peer retains for one Stoa, or absence.
    ///
    /// Absence is a defined answer and not an error: a Stoa the peer is not in is
    /// a question with an answer. A failure means the store could not be
    /// consulted, which is a different fact — the same nesting, and the same
    /// reason, as [`crate::log::OpLog::get`].
    pub fn get(&self, stoa: &Address) -> Result<Option<Membership>, MembershipError> {
        let bytes: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT genesis_bytes FROM stoas WHERE stoa = ?1",
                rusqlite::params![stoa.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;

        match bytes {
            None => Ok(None),
            Some(bytes) => Ok(Some(decode_row(*stoa.as_bytes(), &bytes)?)),
        }
    }

    /// Whether this peer is in a Stoa.
    ///
    /// A question of its own rather than `get(..).is_some()`, because a caller
    /// asking it does not want the record and should not have to decode one to
    /// find out — and because the two answers must not drift.
    pub fn contains(&self, stoa: &Address) -> Result<bool, MembershipError> {
        let found: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM stoas WHERE stoa = ?1",
                rusqlite::params![stoa.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        Ok(found.is_some())
    }

    /// One page of the Stoas this peer is in, ordered by address.
    ///
    /// # Why the address is the order
    ///
    /// The spec requires every Stoa be reachable by paging and none appear twice,
    /// which is a property of the order being **total and stable**. The address is
    /// the primary key, so it is unique and the read is an index walk.
    ///
    /// Insertion order was the alternative and is worse for the reason
    /// [`crate::log`] gives about its own reads: it is per-peer by construction,
    /// and a reader that could reach it would be one refactor away from presenting
    /// it as meaningful. A `joined_at` column would be a local wall-clock reading,
    /// which is what `arrival.rs` refuses for the same reason.
    ///
    /// The order is over 32 bytes of hash and therefore **means nothing** — do not
    /// read it as ranking anything, exactly as `Address`'s own `Ord` says.
    pub fn list(&self, page: usize, per_page: usize) -> Result<MembershipPage, MembershipError> {
        // # A page of zero rows is the last page, and that is the whole of the
        // zero case
        //
        // `has_more` means "paging further reaches a Stoa this page did not show".
        // At `per_page == 0` paging further reaches nothing — every later page is
        // also empty — so the answer is `false` however many Stoas the store holds.
        // Returning early says that once, rather than leaving the arithmetic below
        // to arrive at it: the look-ahead read `LIMIT 0+1` fetched a row, `has_more`
        // was `1 > 0`, and the page was then emptied, so every page came back both
        // empty and not-the-last and paging to exhaustion never terminated.
        //
        // This is a guard, and it is one because the alternative is worse: an
        // arithmetic shape that handles zero is an arithmetic shape whose zero case
        // nobody can read. One function, one job — the zero page is a different
        // question from where a page boundary falls.
        //
        // NO SPEC: the spec does not say what a `per_page` of zero lists, and the
        // wire never produces one (`clamp_per_page` turns 0 into the default). This
        // is the answer that cannot hang a caller.
        if per_page == 0 {
            return Ok(MembershipPage {
                items: Vec::new(),
                page,
                has_more: false,
            });
        }

        // # The two facts about the page boundary are ONE operation
        //
        // A page has to answer both "which rows" and "is there another page", and
        // the look-ahead read is what supplies both without a second `COUNT(*)`
        // that could disagree with it. Computing them separately — `has_more` from
        // what was fetched, the items from a `truncate` — is how a page that shows
        // nothing came to claim a page after it.
        //
        // `split_off` is that boundary as a single cut: what is kept is the page,
        // what comes off is the evidence, and there is no arrangement of the two
        // that contradicts the other. CLAUDE.md's rule — prefer a shape that
        // cannot express the mistake over a guard that checks for it.
        let offset = page.saturating_mul(per_page);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT stoa, genesis_bytes FROM stoas
                 ORDER BY stoa ASC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(storage)?;

        // # Why `per_page` saturates and `page` refuses
        //
        // SQLite's parameters are `i64`, so both have to cross that boundary, and
        // the right answer differs for each:
        //
        // - An **over-large `per_page`** is a caller asking for more rows than
        //   exist, which is answerable: saturating the limit at `i64::MAX` serves
        //   every row there is. Converting `per_page + 1` and giving up on failure
        //   was the bug — the guard tested the INCREMENTED limit, so a `per_page`
        //   the un-incremented one handled fine answered "you are in no Stoa",
        //   indistinguishably from an empty store and with an `Ok`.
        // - An **over-large `page`** is a caller asking for a page that cannot
        //   exist, which is not answerable: a `usize` offset past `i64::MAX` cast
        //   rather than converted becomes negative, and SQLite treats a negative
        //   OFFSET as none at all — serving the FIRST page to a caller who asked
        //   for an impossible one. Empty is the honest reply, so that one refuses.
        let limit = per_page.saturating_add(1).min(i64::MAX as usize) as i64;
        let offset = match i64::try_from(offset) {
            Ok(o) => o,
            Err(_) => {
                return Ok(MembershipPage {
                    items: Vec::new(),
                    page,
                    has_more: false,
                })
            }
        };

        let rows = stmt
            .query_map(rusqlite::params![limit, offset], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(storage)?;

        let mut items = Vec::new();
        for row in rows {
            let (stoa, bytes) = row.map_err(storage)?;
            let stoa: [u8; 32] = stoa.as_slice().try_into().map_err(|_| {
                MembershipError::CorruptEntry(format!(
                    "a stored Stoa address is {} bytes, not 32",
                    stoa.len()
                ))
            })?;
            items.push(decode_row(stoa, &bytes)?);
        }

        // One cut. `split_off` cannot be asked for an index past the length, so the
        // `min` is what makes the call total rather than a panic on a short page —
        // and what comes off is by construction whatever this page does not hold.
        let beyond = items.split_off(per_page.min(items.len()));

        Ok(MembershipPage {
            items,
            page,
            has_more: !beyond.is_empty(),
        })
    }

    /// How many Stoas this peer is in.
    pub fn len(&self) -> Result<usize, MembershipError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM stoas", [], |row| row.get(0))
            .map_err(storage)?;
        // A negative count is not representable by COUNT(*), and a count past
        // `usize` needs more rows than addressable memory. `try_from` rather than
        // `as` so that a platform where it could fail says so instead of wrapping.
        usize::try_from(n).map_err(|_| {
            MembershipError::CorruptEntry(format!(
                "the store reported {n} memberships, which is not a count"
            ))
        })
    }

    /// Whether this peer is in no Stoa.
    ///
    /// Present because clippy's `len_without_is_empty` requires it alongside
    /// [`MembershipStore::len`], and because "is this peer in anything" is a
    /// question a first-run view genuinely asks.
    pub fn is_empty(&self) -> Result<bool, MembershipError> {
        Ok(self.len()? == 0)
    }
}

/// What a [`MembershipStore::join`] did.
///
/// Returned rather than left for the caller to work out by counting, for the
/// reason [`crate::log::Appended`] gives: counting before and after is a two-step
/// check every call site would have to spell the same way. A caller genuinely
/// wants to know — a newly recorded Stoa is one to start following, and a
/// duplicate is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joined {
    /// The peer was not in this Stoa. It is now.
    Recorded,
    /// The peer was already in this Stoa. Nothing changed, including the record
    /// already retained — see [`MembershipStore::join`].
    AlreadyIn,
}

/// Rebuild a [`Membership`] from what a row holds.
///
/// **The address comes from the row's key and the record from its bytes, and then
/// the two are checked against each other.** Deriving the address from the record
/// instead would make the check vacuous — it would compare a value against itself
/// — and trusting the key alone would let a hand-edited file file one Stoa's
/// record under another's address, which is precisely the substitution a Stoa
/// address exists to make detectable.
fn decode_row(stoa: [u8; 32], bytes: &[u8]) -> Result<Membership, MembershipError> {
    let stoa = Address::from_bytes(stoa);
    let genesis = Genesis::decode(bytes).map_err(|e| {
        MembershipError::CorruptEntry(format!("a stored genesis record did not decode: {e}"))
    })?;
    if !genesis.matches(&stoa) {
        return Err(MembershipError::CorruptEntry(format!(
            "the record stored for Stoa {} does not hash to that address",
            stoa.to_hex()
        )));
    }
    Ok(Membership { stoa, genesis })
}

/// Every `rusqlite` failure becomes one variant, carrying its own description.
///
/// A free function rather than a `From` impl, deliberately: a `From` would make
/// `?` convert silently at every call site, including ones where a different
/// variant is the right answer — `CorruptEntry` is not a storage failure, and the
/// two are distinguishable only because this conversion is written out.
fn storage(e: rusqlite::Error) -> MembershipError {
    MembershipError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;
    use crate::stoa::Policy;

    fn a_key(seed: u8) -> crate::identity::PublicKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap().public_key()
    }

    fn a_record(title: &str) -> Genesis {
        Genesis {
            creator: a_key(1),
            policy: Policy::Open,
            title: title.to_string(),
        }
    }

    /// A path in a fresh temporary directory, and the directory's guard.
    ///
    /// The guard must be held for the test's lifetime: dropping it removes the
    /// directory. Returned as a pair rather than hidden behind a helper that drops
    /// it, because a test that let it drop early would fail confusingly. Built
    /// with `std::fs` rather than a `tempfile` dependency, following
    /// `log/sqlite.rs`'s own fixture.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "dialectica-membership-{}-{name}",
                std::process::id()
            ));
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

    /// Join a record under the address it actually names.
    ///
    /// The matching-pair case, which is most of them. A test that means to join a
    /// MISMATCHED pair calls `Membership::verified` directly and asserts on its
    /// refusal, because that is now the only place a mismatch can be refused —
    /// `join` takes a `Membership` and there is no unverified pair to hand it.
    fn join_matching(store: &mut MembershipStore, g: &Genesis) -> Result<Joined, MembershipError> {
        store.join(&Membership::verified(&g.address().unwrap(), g)?)
    }

    /// Every Stoa a store holds, paged through to the end.
    ///
    /// A helper rather than one `list(0, large)` call, because "every Stoa is
    /// reachable by paging" is a requirement and a test that read them all in one
    /// page would not exercise it.
    fn every_stoa(store: &MembershipStore, per_page: usize) -> Vec<Membership> {
        let mut out = Vec::new();
        let mut page = 0;
        loop {
            let p = store.list(page, per_page).unwrap();
            out.extend(p.items);
            if !p.has_more {
                return out;
            }
            page += 1;
            assert!(page < 1000, "paging did not terminate");
        }
    }

    // ─── The layout version ───────────────────────────────────────────────

    #[test]
    fn the_membership_layout_version_is_pinned_to_a_known_answer() {
        // Hardcoded, following `identity.rs`'s wire constants. `cargo mutants`
        // mutates functions and not `const`s, so a wrong version here is
        // invisible to it — and this project has already shipped a `VERSION_1`
        // defect that left the whole suite green.
        //
        // Changing this number is changing the on-disk format every peer holds.
        // If this assertion fails, that is the question being asked.
        assert_eq!(MEMBERSHIP_LAYOUT_VERSION, 1);
    }

    #[test]
    fn the_membership_layout_version_is_independent_of_the_op_logs() {
        // The two are separate FILES, and that is the whole reason membership does
        // not make an op store unreadable. Their version numbers do not contribute
        // to it: both are 1 today, so the version check cannot separate the two
        // stores at all — what refuses an op-log file handed to `open` is
        // `check_layout` naming `stoa` and `genesis_bytes`.
        //
        // So a test comparing the two constants would pass for an implementation
        // that read one from the other. What is asserted instead is that the
        // membership store does not CONSULT the op log's constant: its refusal
        // names its own expected version, and a store stamped with the op log's
        // number plus one is refused.
        let dir = TempDir::new("independent-version");
        let path = dir.file("stoas.sqlite");
        let store = MembershipStore::open(&path).unwrap();
        drop(store);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "PRAGMA user_version = {};",
            crate::log::sqlite::LAYOUT_VERSION + 1
        ))
        .unwrap();
        drop(conn);

        match MembershipStore::open(&path) {
            Err(MembershipError::UnknownLayoutVersion { expected, .. }) => {
                assert_eq!(
                    expected, MEMBERSHIP_LAYOUT_VERSION,
                    "the membership store must report ITS OWN expected version"
                );
            }
            other => panic!("an unknown layout must be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_fresh_store_is_created_rather_than_refused() {
        // `0` is what SQLite reports for a file nobody has stamped, and it is the
        // ONE value that must not be refused — it is the absence of a version
        // rather than an unknown one. It is also the case a peer that predates
        // membership takes, so a check that refused it would make the feature
        // unreachable on every existing installation.
        let dir = TempDir::new("fresh");
        let path = dir.file("stoas.sqlite");
        assert!(!path.exists(), "the fixture must start with no file");

        let store = MembershipStore::open(&path).expect("a fresh store must be creatable");
        assert_eq!(store.len().unwrap(), 0);
        assert!(store.is_empty().unwrap());

        let stamped: i32 = store
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stamped, MEMBERSHIP_LAYOUT_VERSION);
    }

    #[test]
    fn a_store_from_an_unknown_layout_version_is_refused_and_names_both_numbers() {
        let dir = TempDir::new("unknown-version");
        let path = dir.file("stoas.sqlite");

        let store = MembershipStore::open(&path).unwrap();
        drop(store);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version = 9999").unwrap();
        drop(conn);

        match MembershipStore::open(&path) {
            Err(MembershipError::UnknownLayoutVersion { found, expected }) => {
                // Hardcoded on both sides: a test asserting `found == 9999` only
                // would pass for an implementation that reported the same number
                // twice.
                assert_eq!(found, 9999);
                assert_eq!(expected, 1);
            }
            other => panic!("an unknown layout version must be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_store_from_an_older_layout_version_is_refused_too() {
        // The mutation one step weaker than "refuse nothing": relaxing the check
        // to `found > MEMBERSHIP_LAYOUT_VERSION` accepts any older layout, which
        // is the shape a well-meaning "backwards compatible" edit takes. The op
        // log has this exact test for the same reason, and mutation testing found
        // the survivor there.
        //
        // A NEGATIVE version, not `MEMBERSHIP_LAYOUT_VERSION - 1`: with the
        // version at 1 those are the same thing, and `0` is SQLite's "never
        // stamped" value which legitimately means a fresh file. `-1` is below
        // this build's version, is not the fresh sentinel, and stays below
        // whatever the version becomes.
        let dir = TempDir::new("older-version");
        let path = dir.file("stoas.sqlite");

        let store = MembershipStore::open(&path).unwrap();
        drop(store);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version = -1").unwrap();
        drop(conn);

        match MembershipStore::open(&path) {
            Err(MembershipError::UnknownLayoutVersion { found, expected }) => {
                assert_eq!(found, -1);
                assert_eq!(expected, MEMBERSHIP_LAYOUT_VERSION);
            }
            other => panic!("an older layout version must be refused too, got {other:?}"),
        }
    }

    #[test]
    fn a_store_stamped_with_our_version_but_missing_the_table_is_refused_at_open() {
        // `PRAGMA user_version` is one integer and anything can write it, so a
        // file carrying OUR number is a claim rather than a fact. Without this
        // check such a file opens `Ok` and fails at the first read with
        // `Storage("no such table: stoas")`, blaming the disk for a mislabelled
        // file.
        //
        // Asserted at OPEN and not merely "some error eventually": that the
        // failure arrives at the moment the claim is made is the whole finding.
        let dir = TempDir::new("stamped-but-empty");
        let path = dir.file("stoas.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "PRAGMA user_version = {MEMBERSHIP_LAYOUT_VERSION};"
        ))
        .unwrap();
        drop(conn);

        match MembershipStore::open(&path) {
            Err(MembershipError::LayoutDoesNotMatchItsVersion { version, why }) => {
                assert_eq!(version, MEMBERSHIP_LAYOUT_VERSION);
                assert!(
                    why.contains("stoas"),
                    "the refusal must name what was missing, got {why:?}"
                );
            }
            other => panic!("a mislabelled store must be refused at open, got {other:?}"),
        }
    }

    #[test]
    fn a_store_whose_table_lost_a_column_is_refused_too() {
        // One step weaker than the test above, and the one a table-name-only
        // check would pass: the table is called `stoas` and is not the `stoas`
        // this build reads.
        let dir = TempDir::new("stamped-wrong-columns");
        let path = dir.file("stoas.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE stoas (stoa BLOB PRIMARY KEY NOT NULL);
             PRAGMA user_version = {MEMBERSHIP_LAYOUT_VERSION};"
        ))
        .unwrap();
        drop(conn);

        match MembershipStore::open(&path) {
            Err(MembershipError::LayoutDoesNotMatchItsVersion { version, .. }) => {
                assert_eq!(version, MEMBERSHIP_LAYOUT_VERSION);
            }
            other => panic!(
                "a table named `stoas` that is not our `stoas` must be refused, got {other:?}"
            ),
        }
    }

    #[test]
    fn a_store_this_build_wrote_passes_its_own_layout_check() {
        // The other side of the boundary, and not a formality: a `check_layout`
        // naming a column the schema does not have would refuse every real store.
        let dir = TempDir::new("stamped-and-correct");
        let path = dir.file("stoas.sqlite");

        let mut store = MembershipStore::open(&path).unwrap();
        let g = a_record("Agora");
        join_matching(&mut store, &g).unwrap();
        drop(store);

        let reopened = MembershipStore::open(&path)
            .expect("a store this build wrote must pass this build's layout check");
        assert_eq!(reopened.len().unwrap(), 1);
    }

    #[test]
    fn every_error_renders_without_leaking_rust_syntax() {
        // These reach the `{"error":"..."}` wire contract, so `format!("{:?}")`
        // would put `UnknownLayoutVersion { found: 9 }` — Rust type syntax — in a
        // user-facing field.
        //
        // The `match` below is what keeps this list from silently falling behind
        // the enum: adding a variant fails to compile until it is listed.
        fn every_variant() -> Vec<MembershipError> {
            let all = vec![
                MembershipError::Storage("disk on fire".into()),
                MembershipError::UnknownLayoutVersion {
                    found: 9,
                    expected: 1,
                },
                MembershipError::LayoutDoesNotMatchItsVersion {
                    version: 1,
                    why: "no such table: stoas".into(),
                },
                MembershipError::RecordDoesNotMatchAddress,
                MembershipError::UnencodableRecord(GenesisError::Truncated),
                MembershipError::CorruptEntry("a stored record did not decode".into()),
            ];
            if let Some(e) = all.first() {
                match e {
                    MembershipError::Storage(_)
                    | MembershipError::UnknownLayoutVersion { .. }
                    | MembershipError::LayoutDoesNotMatchItsVersion { .. }
                    | MembershipError::RecordDoesNotMatchAddress
                    | MembershipError::UnencodableRecord(_)
                    | MembershipError::CorruptEntry(_) => {}
                }
            }
            all
        }

        let errors = every_variant();
        // Distinguishability is the requirement, and the format assertions below
        // do not check it: collapsing three arms to one string passes them.
        let mut seen = std::collections::HashSet::new();
        for e in &errors {
            assert!(
                seen.insert(e.to_string()),
                "{e:?} renders identically to another variant: {e}"
            );
        }
        for e in errors {
            let rendered = e.to_string();
            assert!(!rendered.is_empty(), "{e:?} rendered empty");
            assert!(
                !rendered.contains("::") && !rendered.contains('{'),
                "{e:?} rendered as Rust syntax: {rendered}"
            );
        }
    }

    // ─── Joining ──────────────────────────────────────────────────────────

    #[test]
    fn a_matching_record_is_recorded_and_read_back() {
        let mut store = MembershipStore::in_memory().unwrap();
        let g = a_record("Agora");
        let address = g.address().unwrap();

        assert_eq!(join_matching(&mut store, &g).unwrap(), Joined::Recorded);
        assert!(store.contains(&address).unwrap());
        let held = store.get(&address).unwrap().expect("the Stoa is retained");
        assert_eq!(
            held.genesis, g,
            "the record read back must be the one stored"
        );
        assert_eq!(held.stoa, address);
    }

    #[test]
    fn a_record_differing_in_any_field_is_refused_and_records_nothing() {
        // Every field in turn, not one hand-picked substitution: a check that
        // compared only the creator would pass a title-only substitution, and
        // vice versa. The record's whole encoding reaches its address, so one
        // comparison covers every field — but only a test that varies each one
        // shows that.
        //
        // ASSERTED AGAINST `Membership::verified`, WHICH IS NOW THE ONLY REFUSAL
        // SITE. It used to assert against `store.join`, which checked the same
        // predicate `wire::genesis_for` checked — two guards, and no test could
        // tell them apart, because deleting either left the other refusing the
        // same inputs (`findings/spec-test.md` entry 2; deleting the wire's half
        // left 550/550 green). `join` now takes a `Membership` and cannot be handed
        // a mismatched pair at all, so this is the one place the refusal exists to
        // be tested.
        let real = a_record("Agora");
        let address = real.address().unwrap();

        let other_creator = Genesis {
            creator: a_key(2),
            ..real.clone()
        };
        let other_title = Genesis {
            title: "Stoa".to_string(),
            ..real.clone()
        };

        for impostor in [other_creator, other_title] {
            let store = MembershipStore::in_memory().unwrap();
            assert_eq!(
                Membership::verified(&address, &impostor),
                Err(MembershipError::RecordDoesNotMatchAddress),
                "a substituted record must be refused"
            );
            // Not in the Stoa asked for...
            assert!(!store.contains(&address).unwrap());
            // ...and not in the one the supplied record would have named either,
            // which is the half a store that "helpfully" filed it under the
            // record's own address would fail. Nothing was written because nothing
            // reached the store: there is no `Membership` to hand it.
            assert!(!store.contains(&impostor.address().unwrap()).unwrap());
            assert_eq!(store.len().unwrap(), 0);
        }
    }

    #[test]
    fn an_unencodable_record_is_refused_before_anything_is_written() {
        // A title over the genesis cap has no encoding and therefore no address,
        // so it is not the record any address names. `Genesis` is a plain struct a
        // caller may build directly, so this is reachable without going through
        // the decoder — and `canonical_bytes` is fallible precisely for it.
        let store = MembershipStore::in_memory().unwrap();
        let too_long = Genesis {
            title: "x".repeat(2000),
            ..a_record("Agora")
        };
        let some_address = a_record("Agora").address().unwrap();

        // Refused by `Membership::verified` — the constructor, which is what
        // "before anything is written" now means literally: there is no value to
        // hand `join`, so no statement can run.
        match Membership::verified(&some_address, &too_long) {
            Err(MembershipError::UnencodableRecord(_)) => {}
            other => panic!("an unencodable record must be refused, got {other:?}"),
        }
        assert_eq!(store.len().unwrap(), 0);
        assert!(!store.contains(&some_address).unwrap());
    }

    #[test]
    fn an_encode_failure_does_not_report_itself_as_a_failure_to_read() {
        // REGRESSION on the message, not on the control flow. The variant was
        // called `UndecodableRecord` and rendered "the genesis record could not be
        // read", while the ONLY way to reach it is `canonical_bytes()` — the encode
        // side — failing. A caller passing a 2000-byte title was told "could not be
        // read: title is 2000 bytes, the maximum is 1024"; nothing was read.
        //
        // This reaches `{"error":"..."}`, so the sentence is the whole of what a
        // user sees. Asserted on the rendered string because that is the contract;
        // asserting only on the variant would have passed the wrong wording.
        let too_long = Genesis {
            title: "x".repeat(2000),
            ..a_record("Agora")
        };
        let rendered = Membership::verified(&a_record("Agora").address().unwrap(), &too_long)
            .expect_err("an unencodable record must be refused")
            .to_string();
        assert!(
            !rendered.contains("could not be read"),
            "an encode failure must not be reported as a failure to read: {rendered}"
        );
        assert!(
            rendered.contains("cannot be encoded"),
            "the message must say what actually failed: {rendered}"
        );
        // And the underlying reason survives, which is what makes the message
        // actionable rather than merely accurate.
        assert!(
            rendered.contains("2000"),
            "the genesis error's own reason must reach the caller: {rendered}"
        );
    }

    #[test]
    fn a_repeated_join_is_idempotent_and_leaves_the_record_untouched() {
        // A pasted address is exactly the input a user supplies twice. Reporting
        // the second attempt as an error would make a harmless action look broken,
        // and REPLACE would make a join a way to overwrite what a peer holds.
        //
        // **What actually kills `INSERT OR REPLACE` here is the `Joined` value,
        // and that is worth stating because the state assertions do not.**
        // Measured: with the statement changed to REPLACE, the two `get`
        // comparisons below stay green and so does
        // `a_join_cannot_overwrite_another_stoas_retained_record` — because the
        // verification makes a mismatched pair unreachable, so the only record
        // REPLACE can ever write over one Stoa's row is the byte-identical one.
        // Only `changed == 0` tells the two statements apart.
        //
        // So the state assertions are not redundant but they are not the kill
        // either: they pin the requirement ("exactly one Stoa, retained values
        // unchanged") against a store that deleted and reinserted, or that filed a
        // second row, which REPLACE does not do and a future edit might.
        let mut store = MembershipStore::in_memory().unwrap();
        let g = a_record("Agora");
        let address = g.address().unwrap();

        assert_eq!(join_matching(&mut store, &g).unwrap(), Joined::Recorded);
        let before = store.get(&address).unwrap().unwrap();

        assert_eq!(
            join_matching(&mut store, &g).unwrap(),
            Joined::AlreadyIn,
            "a repeated join must succeed and report that nothing changed"
        );
        assert_eq!(store.len().unwrap(), 1, "exactly one Stoa for that address");
        assert_eq!(
            store.get(&address).unwrap().unwrap(),
            before,
            "a repeated join must not disturb what was retained"
        );
    }

    #[test]
    fn a_join_cannot_overwrite_another_stoas_retained_record() {
        // The REPLACE failure mode in its sharper form. Two Stoas, each joined
        // with its own record; then the first is re-joined. If the write ever
        // became `INSERT OR REPLACE` keyed on something coarser than the address,
        // or if a caller could pass a mismatched pair through, one Stoa's record
        // would land under the other's address. The verification makes that
        // unreachable, and this asserts the state rather than the mechanism.
        let mut store = MembershipStore::in_memory().unwrap();
        let one = a_record("One");
        let two = a_record("Two");
        let a1 = one.address().unwrap();
        let a2 = two.address().unwrap();
        assert_ne!(a1, a2, "the fixture must be two distinct Stoas");

        join_matching(&mut store, &one).unwrap();
        join_matching(&mut store, &two).unwrap();

        // The cross pairing cannot even be BUILT, which is what stops the
        // overwrite — stronger than it being refused at the write, because there is
        // no value a future caller could carry past the check.
        assert_eq!(
            Membership::verified(&a1, &two),
            Err(MembershipError::RecordDoesNotMatchAddress)
        );
        assert_eq!(store.get(&a1).unwrap().unwrap().genesis, one);
        assert_eq!(store.get(&a2).unwrap().unwrap().genesis, two);
        assert_eq!(store.len().unwrap(), 2);
    }

    #[test]
    fn verification_consults_only_the_two_inputs() {
        // The property that makes a pasted address self-authenticating: the
        // decision needs no index, no peer and no network call.
        //
        // SATISFIED BY CONSTRUCTION SINCE THE RESHAPE, and this test is now only
        // half of what it was — said plainly rather than left to look like more.
        // `Membership::verified` is an associated function with no `self`, so there
        // is no store, no connection and no handle it could consult: the property
        // is in the signature, and no fixture can vary what it does not receive.
        // The previous version made the same call against an empty and a populated
        // store and compared the two answers, which WAS a real test while
        // verification lived on `MembershipStore` and had a `self` to reach through.
        //
        // What is left, and it is worth keeping: the refusal itself, against a
        // hardcoded variant. A store is still built and still asserted untouched,
        // because "nothing was written" is the half that remains observable.
        let g = a_record("Agora");
        let address = g.address().unwrap();
        let impostor = Genesis {
            creator: a_key(9),
            ..g.clone()
        };

        assert_eq!(
            Membership::verified(&address, &impostor),
            Err(MembershipError::RecordDoesNotMatchAddress),
            "a record naming another creator must be refused"
        );

        // And a populated store is not a way past it: the refusal is the same, and
        // the store is unchanged, because nothing reached it.
        let mut populated = MembershipStore::in_memory().unwrap();
        join_matching(&mut populated, &g).unwrap();
        assert_eq!(
            Membership::verified(&address, &impostor),
            Err(MembershipError::RecordDoesNotMatchAddress)
        );
        assert_eq!(populated.len().unwrap(), 1);
        assert_eq!(populated.get(&address).unwrap().unwrap().genesis, g);
    }

    // ─── What is retained ─────────────────────────────────────────────────

    #[test]
    fn the_retained_record_still_verifies_against_its_address() {
        // The whole reason the record is stored as canonical bytes: read it back,
        // recompute its address, and get the address it is filed under. A store
        // that re-encoded from decomposed columns would pass this only while its
        // re-encoding agreed with `stoa.rs`'s byte for byte.
        //
        // Across a REOPEN, not only in memory, because the requirement is about
        // what survives a restart.
        let dir = TempDir::new("retained-verifies");
        let path = dir.file("stoas.sqlite");
        let g = a_record("Ἀγορά — the marketplace");
        let address = g.address().unwrap();

        let mut store = MembershipStore::open(&path).unwrap();
        join_matching(&mut store, &g).unwrap();
        drop(store);

        let reopened = MembershipStore::open(&path).unwrap();
        let held = reopened.get(&address).unwrap().unwrap();
        assert_eq!(
            held.genesis.address().unwrap(),
            address,
            "the retained record must recompute to the address it is held under"
        );
        assert!(held.genesis.matches(&address));
    }

    #[test]
    fn the_founding_title_and_policy_are_answerable_from_what_was_retained() {
        // Both fields, and the title as a HARDCODED literal rather than read back
        // from the record the test just built — otherwise the assertion is the
        // test agreeing with itself about what it stored.
        let mut store = MembershipStore::in_memory().unwrap();
        let g = a_record("The Painted Porch");
        let address = g.address().unwrap();
        join_matching(&mut store, &g).unwrap();

        let held = store.get(&address).unwrap().unwrap();
        assert_eq!(held.genesis.title, "The Painted Porch");
        assert_eq!(held.genesis.policy, Policy::Open);
    }

    #[test]
    fn a_stoa_the_peer_is_not_in_is_absence_and_not_a_failure() {
        // Absence is a defined answer. A store that reported it as an error would
        // make "are you in this Stoa" unanswerable without unwrapping a failure.
        let store = MembershipStore::in_memory().unwrap();
        let address = a_record("Nowhere").address().unwrap();
        assert_eq!(store.get(&address).unwrap(), None);
        assert!(!store.contains(&address).unwrap());
    }

    #[test]
    fn a_corrupt_stored_record_is_reported_rather_than_skipped() {
        // Skipping would make a corrupted row indistinguishable from a Stoa the
        // user never joined — `OpLogError`'s "an empty feed is not an unreadable
        // store" argument applied to a listing. A user missing a Stoa they joined
        // needs to be told, not quietly served a shorter list.
        //
        // The row is written BEHIND the store's back, which is the only way to
        // produce it: `join` verifies, so this state is unreachable through the
        // API and that is exactly why it needs a test.
        let dir = TempDir::new("corrupt-row");
        let path = dir.file("stoas.sqlite");
        let address = a_record("Agora").address().unwrap();

        let store = MembershipStore::open(&path).unwrap();
        store
            .conn
            .execute(
                "INSERT INTO stoas (stoa, genesis_bytes) VALUES (?1, ?2)",
                rusqlite::params![address.as_bytes().as_slice(), vec![0xffu8, 0x00, 0x01]],
            )
            .unwrap();

        match store.get(&address) {
            Err(MembershipError::CorruptEntry(why)) => {
                assert!(!why.is_empty(), "the refusal must say what went wrong");
            }
            other => panic!("a corrupt row must be reported, got {other:?}"),
        }
        match store.list(0, 10) {
            Err(MembershipError::CorruptEntry(_)) => {}
            other => panic!("a listing must not skip a corrupt row, got {other:?}"),
        }
    }

    #[test]
    fn a_record_filed_under_the_wrong_address_is_reported_rather_than_trusted() {
        // One step weaker than the test above, and the one a decode-only check
        // would pass: the bytes ARE a valid genesis record, and they are not the
        // record that address names. A hand-edited file is exactly this shape, and
        // trusting the key would let one Stoa's record answer for another's —
        // which is the substitution a Stoa address exists to make detectable.
        let store = MembershipStore::in_memory().unwrap();
        let real = a_record("Agora");
        let elsewhere = a_record("Somewhere else");
        store
            .conn
            .execute(
                "INSERT INTO stoas (stoa, genesis_bytes) VALUES (?1, ?2)",
                rusqlite::params![
                    real.address().unwrap().as_bytes().as_slice(),
                    elsewhere.canonical_bytes().unwrap()
                ],
            )
            .unwrap();

        match store.get(&real.address().unwrap()) {
            Err(MembershipError::CorruptEntry(_)) => {}
            other => panic!("a misfiled record must be reported, got {other:?}"),
        }
    }

    // ─── Persistence ──────────────────────────────────────────────────────

    #[test]
    fn memberships_outlive_the_store_object_that_recorded_them() {
        // The whole point of recording membership rather than deriving it: a peer
        // that forgot its Stoas would ask the user to re-paste every address they
        // had joined, and for a created Stoa there is nobody to re-paste it from.
        let dir = TempDir::new("outlive");
        let path = dir.file("stoas.sqlite");
        let one = a_record("One");
        let two = a_record("Two");

        let mut store = MembershipStore::open(&path).unwrap();
        join_matching(&mut store, &one).unwrap();
        join_matching(&mut store, &two).unwrap();
        drop(store);

        let reopened = MembershipStore::open(&path).unwrap();
        let held = every_stoa(&reopened, 1);
        assert_eq!(held.len(), 2);
        // Each still answers its founding values, asserted against literals.
        let mut titles: Vec<&str> = held.iter().map(|m| m.genesis.title.as_str()).collect();
        titles.sort_unstable();
        assert_eq!(titles, vec!["One", "Two"]);
        for m in &held {
            assert_eq!(m.genesis.policy, Policy::Open);
            assert!(m.genesis.matches(&m.stoa));
        }
    }

    #[test]
    fn a_refused_join_leaves_nothing_behind_a_restart() {
        let dir = TempDir::new("refused-restart");
        let path = dir.file("stoas.sqlite");
        let real = a_record("Agora");
        let address = real.address().unwrap();
        let impostor = Genesis {
            creator: a_key(3),
            ..real.clone()
        };

        // The refusal is at the constructor, so the store is opened, handed
        // nothing, and closed — which is a sharper version of "leaves nothing
        // behind" than the one this test had: the file is touched by the open and
        // by no write, because there was no `Membership` to write.
        let store = MembershipStore::open(&path).unwrap();
        assert!(Membership::verified(&address, &impostor).is_err());
        drop(store);

        let reopened = MembershipStore::open(&path).unwrap();
        assert!(!reopened.contains(&address).unwrap());
        assert!(!reopened.contains(&impostor.address().unwrap()).unwrap());
        assert_eq!(reopened.len().unwrap(), 0);
    }

    // ─── Listing ──────────────────────────────────────────────────────────

    #[test]
    fn a_peer_in_no_stoa_lists_nothing_and_does_not_fail() {
        let store = MembershipStore::in_memory().unwrap();
        let page = store.list(0, 20).unwrap();
        assert_eq!(page.items, vec![]);
        assert_eq!(page.page, 0);
        // NO SPEC: the spec says an empty listing reports no failure and does not
        // say what `hasMore` holds. `false` is chosen — there is no further page.
        assert!(!page.has_more);
    }

    #[test]
    fn every_stoa_is_reachable_by_paging_and_appears_exactly_once() {
        // The requirement, over a population LARGER than one page and with a page
        // size that does not divide it — an off-by-one in the offset arithmetic
        // or in `has_more` is invisible when the last page happens to be full.
        let mut store = MembershipStore::in_memory().unwrap();
        let mut expected = Vec::new();
        for n in 0..7 {
            let g = a_record(&format!("Stoa {n}"));
            let address = g.address().unwrap();
            join_matching(&mut store, &g).unwrap();
            expected.push(address);
        }
        expected.sort_unstable();

        let seen: Vec<Address> = every_stoa(&store, 3).iter().map(|m| m.stoa).collect();
        assert_eq!(
            seen.len(),
            7,
            "every Stoa must be reachable by paging, got {}",
            seen.len()
        );
        assert_eq!(
            seen, expected,
            "each Stoa must appear once, in a total order"
        );
    }

    #[test]
    fn the_order_is_total_and_does_not_depend_on_the_sequence_of_joins() {
        // The property paging rests on. Two stores given the same Stoas in
        // opposite sequences must list them identically — a store ordering by
        // insertion would return reversed sequences here, and then a page boundary
        // would mean two different things on two peers.
        let records: Vec<Genesis> = (0..5).map(|n| a_record(&format!("S{n}"))).collect();

        let mut forwards = MembershipStore::in_memory().unwrap();
        for g in records.iter() {
            join_matching(&mut forwards, g).unwrap();
        }
        let mut backwards = MembershipStore::in_memory().unwrap();
        for g in records.iter().rev() {
            join_matching(&mut backwards, g).unwrap();
        }

        let a: Vec<Address> = every_stoa(&forwards, 2).iter().map(|m| m.stoa).collect();
        let b: Vec<Address> = every_stoa(&backwards, 2).iter().map(|m| m.stoa).collect();
        assert_eq!(a.len(), 5, "the fixture must reach the comparison");
        assert_eq!(a, b, "the listing order changed with the order of joins");
    }

    #[test]
    fn has_more_is_false_on_the_last_page_and_true_before_it() {
        // The boundary pair. A `has_more` that was always true pages forever; one
        // that was always false loses every Stoa past the first page. Both halves
        // are asserted, with a population that fills exactly two pages — the case
        // where "look one past the end" and "compare against a count" disagree.
        let mut store = MembershipStore::in_memory().unwrap();
        for n in 0..4 {
            let g = a_record(&format!("S{n}"));
            join_matching(&mut store, &g).unwrap();
        }
        let first = store.list(0, 2).unwrap();
        assert_eq!(first.items.len(), 2);
        assert!(first.has_more, "a further page exists");

        let second = store.list(1, 2).unwrap();
        assert_eq!(second.items.len(), 2);
        assert!(
            !second.has_more,
            "an exactly-full last page must not claim a further one"
        );

        let past_the_end = store.list(2, 2).unwrap();
        assert_eq!(past_the_end.items, vec![]);
        assert!(!past_the_end.has_more);
    }

    #[test]
    fn a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page() {
        // The trap the `i64::try_from` exists for: a `usize` offset past
        // `i64::MAX` cast rather than converted becomes negative, and SQLite
        // treats a negative OFFSET as none at all — so a caller asking for an
        // impossible page would be served the FIRST one and would have no way to
        // tell. Answering empty is the honest reply.
        let mut store = MembershipStore::in_memory().unwrap();
        let g = a_record("Agora");
        join_matching(&mut store, &g).unwrap();

        let page = store.list(usize::MAX, 20).unwrap();
        assert_eq!(
            page.items,
            vec![],
            "an unreachable page must be empty, never the first page"
        );
        assert!(!page.has_more);
        assert_eq!(page.page, usize::MAX, "the page asked for is reported back");
    }

    #[test]
    fn a_per_page_at_the_conversion_boundary_still_lists_the_whole_store() {
        // REGRESSION. `limit = per_page + 1` was converted to `i64` and a failed
        // conversion answered empty — so a `per_page` the un-incremented limit
        // would have handled fine fell into the failure window, and page 0 of a
        // store holding three Stoas reported "you are in no Stoa".
        //
        // The pair is the whole test: one below the boundary listed all three, one
        // at it listed none, and nothing distinguishes that reply from an empty
        // store. `list_stoas` states the obligation for its own `Err` arm — "a
        // storage failure is the error shape and NEVER an empty listing" — and this
        // path broke it while returning `Ok`.
        let mut store = MembershipStore::in_memory().unwrap();
        for n in 0..3 {
            let g = a_record(&format!("S{n}"));
            join_matching(&mut store, &g).unwrap();
        }

        let below = store.list(0, (i64::MAX as usize) - 1).unwrap();
        assert_eq!(
            below.items.len(),
            3,
            "the fixture must reach the comparison"
        );

        let at = store.list(0, i64::MAX as usize).unwrap();
        assert_eq!(
            at.items.len(),
            3,
            "a per_page one larger must not turn three memberships into none"
        );
        assert!(
            !at.has_more,
            "a page holding everything has no page after it"
        );
    }

    #[test]
    fn a_per_page_of_zero_terminates_rather_than_paging_forever() {
        // REGRESSION. `limit = 0 + 1 = 1` fetched a row, `has_more = 1 > 0` was
        // true, and `truncate(0)` then emptied the page — so every page was both
        // empty and not-the-last, and `every_stoa` ran to its own 1000-page guard
        // and would have blamed the fixture.
        //
        // The two facts about one boundary disagreed: `has_more` was computed from
        // what was FETCHED and the page from what was KEPT. Asserting only
        // `items.is_empty()` would pass the broken code, so the assertion that
        // matters is on `has_more`.
        let mut store = MembershipStore::in_memory().unwrap();
        for n in 0..3 {
            let g = a_record(&format!("S{n}"));
            join_matching(&mut store, &g).unwrap();
        }

        // NO SPEC: the spec does not say what a `per_page` of zero lists. The wire
        // never produces one — `clamp_per_page` turns 0 into the default — so this
        // is a decision about the crate's own API, and it is the one that cannot
        // hang a caller: an empty page with nothing after it.
        for page in [0, 1, 5] {
            let p = store.list(page, 0).unwrap();
            assert_eq!(p.items, vec![], "a page of zero holds nothing");
            assert!(
                !p.has_more,
                "a page of zero must not promise a further page, or paging to \
                 exhaustion never terminates"
            );
        }

        // And paging actually terminates, which is the consequence the assertion
        // above exists for rather than a restatement of it.
        assert_eq!(every_stoa(&store, 0), vec![]);
    }

    #[test]
    fn an_empty_store_and_a_populated_one_disagree_about_being_empty() {
        // `is_empty` survived `cargo mutants` replaced by `Ok(true)`: the only
        // assertion on it was against a FRESH store, so nothing ever asked for the
        // `false` case. A permanent `true` renders every user as having joined
        // nothing, and its own docstring says a first-run view asks this.
        let mut store = MembershipStore::in_memory().unwrap();
        assert!(store.is_empty().unwrap(), "a fresh store is empty");

        let g = a_record("Agora");
        join_matching(&mut store, &g).unwrap();
        assert!(
            !store.is_empty().unwrap(),
            "a store holding a membership is not empty"
        );
        // And it agrees with `len`, which is the drift the two exist as a pair to
        // avoid — `is_empty` reporting the opposite of a non-zero `len` is the
        // shape the mutation produced.
        assert_eq!(store.len().unwrap(), 1);
    }

    #[test]
    fn a_listing_reports_the_stoas_membership_holds_and_nothing_else() {
        // The contents requirement: exactly what membership records. There is no
        // path from this store to the op log, so there is nothing else it COULD
        // report — which is the point, and is asserted here as the set rather than
        // as the absence of a call.
        let mut store = MembershipStore::in_memory().unwrap();
        let joined = a_record("Joined");
        let never = a_record("Never joined");
        join_matching(&mut store, &joined).unwrap();

        let seen: Vec<Address> = every_stoa(&store, 10).iter().map(|m| m.stoa).collect();
        assert_eq!(seen, vec![joined.address().unwrap()]);
        assert!(!seen.contains(&never.address().unwrap()));
    }

    // `membership_is_not_lost_because_a_stoa_has_no_ops` WAS HERE, AND IS DELETED.
    //
    // `findings/spec-test.md` entry 4: it built three memberships and asserted the
    // listing held three, which is what
    // `every_stoa_is_reachable_by_paging_and_appears_exactly_once` already asserts
    // over a larger population and a page size that does not divide it. The only
    // thing its name added was the op-log claim, and that claim is not testable at
    // this layer — so the test told a reader the property was covered while
    // exercising nothing of it.
    //
    // SATISFIED BY CONSTRUCTION at this layer, and what makes the absence real:
    // `list`'s only material is `self.conn`, and `open` / `in_memory` are the only
    // constructors — neither takes a log, a path to one, or anything through which
    // one could be reached. `grep -n "OpLog\|ops.sqlite" membership.rs` returns
    // nothing outside comments, so there is no code path to break.
    //
    // The version of this requirement that CAN fail lives at the wire, where a real
    // op store and a real membership store share one directory and an
    // implementation that went looking would find the ops:
    // `wire.rs::an_empty_op_log_does_not_empty_the_listing` and
    // `wire.rs::an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership`.

    #[test]
    fn an_empty_title_is_recordable_and_reads_back_empty() {
        // NO SPEC — well, the spec DOES require an empty title be accepted at
        // creation; what is unstated is that the store treats it as an ordinary
        // value rather than as absence. `Genesis` has no minimum length and the
        // title is not an identifier, so a store that coerced an empty title to
        // NULL or refused it would make a record other peers decode and verify
        // without complaint unreachable.
        let mut store = MembershipStore::in_memory().unwrap();
        let g = a_record("");
        let address = g.address().unwrap();
        join_matching(&mut store, &g).unwrap();
        assert_eq!(store.get(&address).unwrap().unwrap().genesis.title, "");
    }

    #[test]
    fn a_title_carrying_bidi_and_zero_width_characters_is_retained_unchanged() {
        // The record is hashed to produce the address, so normalising a title
        // would change the address and split one Stoa into two that cannot see
        // each other. `op.rs` takes the same position for display text: preserve
        // exactly, never normalise. Rendering it safely is the view's obligation.
        let mut store = MembershipStore::in_memory().unwrap();
        let nasty = "Agora\u{202E}\u{200B}\u{0430}";
        let g = a_record(nasty);
        let address = g.address().unwrap();
        join_matching(&mut store, &g).unwrap();

        let held = store.get(&address).unwrap().unwrap();
        assert_eq!(
            held.genesis.title, nasty,
            "the founding title must be retained byte for byte"
        );
        assert!(held.genesis.title.contains('\u{202E}'));
        assert!(held.genesis.title.contains('\u{200B}'));
    }
}
