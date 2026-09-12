//! The record of which derivation path a user chose for each Stoa.
//!
//! The contract is the `identity-onboarding` spec's requirement "A chosen
//! derivation path is recorded, because it cannot be recomputed"; the reasoning
//! behind the shape is in that change's `design.md`. What is repeated here is
//! only what a reader of THIS file needs in order not to undo it.
//!
//! # Why this is its own file rather than a table in the op log
//!
//! `log/sqlite.rs` holds its layout version in `PRAGMA user_version`,
//! `check_layout` proves the declared layout against the columns every read
//! touches, and `create_schema`'s doc comment states outright that **there is no
//! migration path by design**. So adding a table to `ops.sqlite` is a
//! `LAYOUT_VERSION` bump, and a bump refuses every store a prior build wrote.
//!
//! Two further properties fall out of the split and are worth having anyway. The
//! failure domains differ — an op log that will not open is a feed that cannot
//! render, a path record that will not open is a user who cannot post — and the
//! record is separately copyable, which the spec requires nothing prevent:
//! *"no value in it may be derivable only on the machine that wrote it, and none
//! may be unreadable once written."*
//!
//! # What is deliberately NOT here
//!
//! **No secret.** The spec is explicit that the record *"SHALL NOT be required to
//! be secret. It says which path was chosen and reveals nothing that a published
//! identity does not already reveal."* So there is no encryption, no permission
//! check and no `Zeroizing` in this file — and that is a conclusion the spec
//! reached, not an omission. The master key is [`crate::keystore`]'s, and it is
//! what actually needs protecting.
//!
//! **No export.** The owner's sequence is local storage first, export and remote
//! backup later. [`IdentityStore::all_paths`] exists because the spec requires
//! the record be *readable in full* so that export is possible; serialising it to
//! anywhere is a later change.
//!
//! # Nothing here panics
//!
//! A panic aborts the module process (PHASE0-FINDINGS §3). There is no `unwrap`,
//! no `expect` and no indexing outside `#[cfg(test)]`; every `rusqlite` failure
//! becomes an [`IdentityStoreError`].

use crate::identity::Address;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

/// The storage layout this build writes and understands.
///
/// Held in SQLite's own `PRAGMA user_version`, the same place and for the same
/// reason `log/sqlite.rs` holds its own: one `INTEGER` the file format already
/// reserves, so the version needs no table and cannot be lost by a schema change
/// that forgot about it.
///
/// **Pinned by a hardcoded assertion**, following `identity.rs`'s wire constants
/// and `log/sqlite.rs`'s precedent. `cargo mutants` mutates functions and not
/// `const`s, so a wrong version here would be invisible to it, and this project
/// has already shipped a `VERSION_1` defect that left the whole suite green.
pub const LAYOUT_VERSION: i32 = 1;

/// Everything that can go wrong, each arm distinguishable.
///
/// Distinguishable for the reason `KeystoreError` is: the reply a view renders
/// carries one of these messages, and a message can only name a fix if the error
/// said which thing went wrong.
#[derive(Debug, PartialEq, Eq)]
pub enum IdentityStoreError {
    /// The store could not be opened, read or written. The string is SQLite's and
    /// names no path content.
    Storage(String),
    /// A layout version this build does not recognise.
    ///
    /// Refused rather than read hopefully, following `OpLogError`'s posture: a
    /// store written by another build may name paths under a derivation scheme
    /// this build does not implement, and reading one would report an identity the
    /// user does not have.
    UnknownLayoutVersion { found: i32, expected: i32 },
    /// The file carries this build's version but not this build's layout.
    ///
    /// Distinct from [`IdentityStoreError::Storage`] because the two say different
    /// things: one blames the disk, this one says the file is mislabelled. A
    /// half-restored backup or a hand-edited store looks exactly like this.
    LayoutDoesNotMatchItsVersion { version: i32, why: String },
    /// A stored path was not a value this build can represent as a `u32`.
    ///
    /// SQLite's `INTEGER` is an `i64`, so a row can hold a negative or oversized
    /// value that no derivation this build performs could have written. Refused
    /// rather than clamped: coercing would name an identity the user never chose,
    /// which is the one outcome the spec calls unrecoverable.
    PathOutOfRange { stoa: String, found: i64 },
    /// A stored Stoa address was not 32 bytes.
    StoaNotAnAddress { found: usize },
}

impl std::fmt::Display for IdentityStoreError {
    /// Every message names the fix, following `KeystoreError::Display`'s
    /// documented obligation — these strings reach a view through `whoAmI`'s
    /// reason, and a reason a user cannot act on is a reason not worth carrying.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityStoreError::Storage(e) => write!(
                f,
                "the identity record could not be read or written: {e}; check the \
                 path and its containing directory"
            ),
            IdentityStoreError::UnknownLayoutVersion { found, expected } => write!(
                f,
                "the identity record declares layout version {found} and this build \
                 understands {expected}; upgrade dialectica"
            ),
            IdentityStoreError::LayoutDoesNotMatchItsVersion { version, why } => write!(
                f,
                "the identity record claims layout version {version} but does not \
                 have that layout ({why}); restore it from a backup"
            ),
            IdentityStoreError::PathOutOfRange { stoa, found } => write!(
                f,
                "the identity record holds derivation path {found} for Stoa {stoa}, \
                 which this build cannot have written; restore the record from a \
                 backup"
            ),
            IdentityStoreError::StoaNotAnAddress { found } => write!(
                f,
                "the identity record holds a {found}-byte Stoa key where an address \
                 is 32 bytes; restore it from a backup"
            ),
        }
    }
}

fn storage(e: rusqlite::Error) -> IdentityStoreError {
    IdentityStoreError::Storage(e.to_string())
}

/// One recorded choice: a Stoa, and the path chosen for it.
///
/// A named struct rather than a tuple because [`IdentityStore::all_paths`] exists
/// to be exported later, and a tuple's field order is the sort of thing an export
/// format would inherit by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenPath {
    pub stoa: Address,
    pub path: u32,
}

/// The record of chosen derivation paths, in a SQLite database.
///
/// # The file is handed in, never discovered
///
/// [`IdentityStore::open`] takes a path, exactly as `Keystore` and `SqliteOpLog`
/// do and for the same reason: a fixed path baked into a pure crate is untestable
/// and would mean this crate reading the environment at a moment its caller does
/// not control.
///
/// # One writer, by the spec rather than by a lock
///
/// The spec requires the record have *exactly one writer*, and names what that
/// buys: no reconciliation between divergent records, no question of which
/// device's record is authoritative, no per-device path allocation to keep
/// disjoint. **There is therefore no locking, no vector clock and no merge
/// function in this file**, and their absence is the scope limit being taken
/// rather than a gap. A second writer is a separate capability, and it would
/// arrive as an approved per-device key rather than as a second writer of this
/// table.
#[derive(Debug)]
pub struct IdentityStore {
    conn: Connection,
}

impl IdentityStore {
    /// Where the record lives inside a directory the caller chose.
    ///
    /// The *name* is fixed and the *directory* is not, following
    /// `keystore::default_path_in`. A sibling of `ops.sqlite`, not a table inside
    /// it — see this module's documentation.
    pub fn default_path_in(dir: &Path) -> std::path::PathBuf {
        dir.join("identity.sqlite")
    }

    /// Open or create the record at `path`, refusing a layout this build cannot
    /// read.
    pub fn open(path: &Path) -> Result<Self, IdentityStoreError> {
        let conn = Connection::open(path).map_err(storage)?;
        Self::from_connection(conn)
    }

    /// An ephemeral record with no file behind it.
    ///
    /// **Not a test double.** It is this exact code — the same schema, the same
    /// statements, the same decode path — with SQLite's `:memory:` backing store,
    /// following `SqliteOpLog::in_memory` for the same reason: every test that is
    /// not *about* persistence exercises the real SQL without a temporary
    /// directory or a teardown. Tests that ARE about persistence use
    /// [`IdentityStore::open`] against a real file, because this one by
    /// construction cannot survive being dropped.
    pub fn in_memory() -> Result<Self, IdentityStoreError> {
        let conn = Connection::open_in_memory().map_err(storage)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, IdentityStoreError> {
        // A fresh database reports 0, which is not a version anything wrote — it
        // is the absence of one, and the only case where creating is correct.
        let found: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;

        if found == 0 {
            Self::create_schema(&conn)?;
        } else if found != LAYOUT_VERSION {
            return Err(IdentityStoreError::UnknownLayoutVersion {
                found,
                expected: LAYOUT_VERSION,
            });
        } else {
            Self::check_layout(&conn)?;
        }

        Ok(IdentityStore { conn })
    }

    /// Prove the store has the layout its `user_version` claims.
    ///
    /// `PRAGMA user_version` is one integer with no relationship to the tables
    /// beside it, and anything can stamp it. A file carrying this build's number
    /// with no `chosen_paths` table is what a half-restored backup or a
    /// hand-edited store looks like, and without this check it would open `Ok` and
    /// fail at the first read as `Storage("no such table: chosen_paths")` — an
    /// error that blames the disk when the fact is that the file is mislabelled.
    ///
    /// Both columns are named, not merely the table: `SELECT 1 FROM chosen_paths`
    /// proves a name exists and nothing about its shape. `LIMIT 0` prepares and
    /// runs the statement, which is what establishes the layout, and reads no row,
    /// so the cost does not grow with the store.
    ///
    /// **This is not a migration and must not become one.** Refusing is the whole
    /// behaviour; a check that repaired what it found would be a migration written
    /// against a layout nobody has described.
    fn check_layout(conn: &Connection) -> Result<(), IdentityStoreError> {
        conn.query_row("SELECT stoa, path FROM chosen_paths LIMIT 0", [], |_| Ok(()))
            // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS
            // case and every other error is the layout being wrong.
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(()),
                other => Err(IdentityStoreError::LayoutDoesNotMatchItsVersion {
                    version: LAYOUT_VERSION,
                    why: other.to_string(),
                }),
            })
    }

    /// The whole schema, in one place.
    ///
    /// # The primary key IS "one chosen path per Stoa"
    ///
    /// `stoa BLOB PRIMARY KEY` is what makes the spec's *"Distinct choices for
    /// distinct Stoas are recorded separately"* a property of the schema rather
    /// than of a guard at each write, and what makes a second choice for one Stoa
    /// unrepresentable rather than merely refused somewhere. CLAUDE.md's rule
    /// applied to storage: a branch must be got right at every call site, a data
    /// shape is right everywhere at once.
    ///
    /// # `PRAGMA user_version` IS LAST, AND MUST STAY LAST
    ///
    /// It is the commit point for the layout *claim*, and every other statement
    /// has to be true before it is made. A crash anywhere before it leaves a file
    /// at version `0`, which `from_connection` reads as "never stamped" and
    /// creates cleanly. Moving it earlier produces exactly the file `check_layout`
    /// exists to refuse — version 1 stamped over a table that was never created —
    /// and that file is permanently unopenable, because there is no migration path
    /// by design. The ordering is what keeps a crash mid-create recoverable.
    fn create_schema(conn: &Connection) -> Result<(), IdentityStoreError> {
        let result = conn.execute_batch(&format!(
            "BEGIN;
             CREATE TABLE chosen_paths (
                 -- The Stoa this choice is for, as its 32 raw address bytes.
                 -- PRIMARY KEY is the one-path-per-Stoa invariant; see this
                 -- function's documentation.
                 stoa  BLOB PRIMARY KEY NOT NULL,

                 -- The derivation path the user chose, as written by
                 -- `identity::derive_stoa_key_at_path`. A `u32` widened to
                 -- SQLite's `i64`; a row outside `u32` is refused on read rather
                 -- than clamped, because clamping would name an identity the user
                 -- never chose.
                 path  INTEGER NOT NULL
             );
             PRAGMA user_version = {LAYOUT_VERSION};
             COMMIT;"
        ));
        if let Err(e) = result {
            // `execute_batch` stops at the first failing statement and returns
            // with the `BEGIN` still open. Dropping the connection would make
            // rusqlite roll back, so leaving it would be sound by accident rather
            // than by invariant — the same correction `log/sqlite.rs` records.
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(storage(e));
        }
        Ok(())
    }

    /// Record the path chosen for a Stoa.
    ///
    /// **Refuses to replace an existing choice**, and the refusal is the point
    /// rather than a convenience. The spec requires that keeping a candidate be
    /// refused where an identity already exists, because replacing silently
    /// *"discards every identity derived from it, while the ops those identities
    /// signed remain published and unreachable — and no error anywhere says it
    /// happened."*
    ///
    /// `INSERT` without `OR REPLACE`, so the primary key does the refusing: a
    /// second choice for one Stoa is a constraint violation, not a row this code
    /// decided to keep. That is the refusal being structural rather than a branch
    /// somebody has to remember at every write.
    pub fn record_path(&self, stoa: &Address, path: u32) -> Result<(), IdentityStoreError> {
        let affected = self
            .conn
            .execute(
                "INSERT INTO chosen_paths (stoa, path) VALUES (?1, ?2)",
                rusqlite::params![&stoa.as_bytes()[..], i64::from(path)],
            )
            .map_err(storage)?;
        // `execute` returning 0 would mean an insert that inserted nothing, which
        // this statement cannot produce — there is no `OR IGNORE` here. Checked
        // anyway rather than assumed, because a caller told a write succeeded when
        // no row exists would report an identity that was never recorded.
        if affected != 1 {
            return Err(IdentityStoreError::Storage(format!(
                "recording a chosen path affected {affected} rows instead of 1"
            )));
        }
        Ok(())
    }

    /// The path recorded for a Stoa, or `None` if none is.
    ///
    /// `None` is a real and common state — a master key exists and this Stoa has
    /// no choice recorded for it — and it is distinct from an error. Collapsing
    /// the two would make "you have not chosen an identity here" indistinguishable
    /// from "the record is unreadable", which are different things to tell a user.
    pub fn path_for(&self, stoa: &Address) -> Result<Option<u32>, IdentityStoreError> {
        let found: Option<i64> = self
            .conn
            .query_row(
                "SELECT path FROM chosen_paths WHERE stoa = ?1",
                rusqlite::params![&stoa.as_bytes()[..]],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match found {
            None => Ok(None),
            Some(raw) => Ok(Some(path_from_row(raw, stoa.to_hex())?)),
        }
    }

    /// Every recorded pairing of Stoa and path.
    ///
    /// **This exists because the spec requires the record be readable in full**,
    /// so that it can later be exported and preserved somewhere other than the
    /// machine that made it: *"every recorded pairing of Stoa and path can be read
    /// back, and reading them requires nothing beyond the stored record itself."*
    ///
    /// Export itself is not in this change. What this method discharges is the
    /// obligation that nothing about the storage *prevents* it — there is no value
    /// here that is derivable only on the machine that wrote it, and every row is
    /// readable once written.
    ///
    /// Deliberately not paginated. The record has one row per Stoa a user has
    /// joined, which is a number of Stoas a person reads, and a caller exporting a
    /// backup wants all of it or none.
    pub fn all_paths(&self) -> Result<Vec<ChosenPath>, IdentityStoreError> {
        let mut stmt = self
            .conn
            // Ordered so two exports of one record are byte-identical. Ordering by
            // the address is meaningless as a ranking — it is a hash — but a
            // deterministic order is what makes an export diffable.
            .prepare("SELECT stoa, path FROM chosen_paths ORDER BY stoa")
            .map_err(storage)?;
        let rows = stmt
            .query_map([], |row| {
                let stoa: Vec<u8> = row.get(0)?;
                let path: i64 = row.get(1)?;
                Ok((stoa, path))
            })
            .map_err(storage)?;

        let mut out = Vec::new();
        for row in rows {
            let (stoa_bytes, raw) = row.map_err(storage)?;
            // The stored blob's length is not trusted. It is this module that
            // wrote it, but the file is not under this module's control — it can
            // be hand-edited or half-restored — and a `try_into` that assumed 32
            // bytes would be an `unwrap` on disk content.
            let stoa: [u8; 32] =
                stoa_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| IdentityStoreError::StoaNotAnAddress {
                        found: stoa_bytes.len(),
                    })?;
            let stoa = Address::from_bytes(stoa);
            out.push(ChosenPath {
                path: path_from_row(raw, stoa.to_hex())?,
                stoa,
            });
        }
        Ok(out)
    }
}

/// Narrow a stored `i64` back to the `u32` a path is.
///
/// A named function rather than an inline `try_into` at each of the two read
/// sites, so that "a stored path outside `u32` is refused and never clamped" is
/// one rule with one answer to "is it applied everywhere?" — CLAUDE.md's "a guard
/// is a job".
///
/// Refusing rather than clamping matters more here than it usually would: a
/// clamped path derives a *valid* key, so the user would be handed a working
/// identity that is not the one they chose, with no error anywhere. The spec calls
/// storing an identity the user did not choose unrecoverable, because the choice
/// cannot be recomputed.
fn path_from_row(raw: i64, stoa_hex: String) -> Result<u32, IdentityStoreError> {
    u32::try_from(raw).map_err(|_| IdentityStoreError::PathOutOfRange {
        stoa: stoa_hex,
        found: raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::stoa_address;

    fn a_stoa(tag: &[u8]) -> Address {
        stoa_address(tag)
    }

    #[test]
    fn a_recorded_path_reads_back() {
        let store = IdentityStore::in_memory().unwrap();
        let stoa = a_stoa(b"one");
        store.record_path(&stoa, 7).unwrap();
        // The expectation is the literal 7, not something read back from the
        // write — otherwise the assertion is the test agreeing with itself.
        assert_eq!(store.path_for(&stoa).unwrap(), Some(7));
    }

    #[test]
    fn an_unrecorded_stoa_reads_as_none_rather_than_an_error() {
        // A real and common state: a master key exists and this Stoa has no
        // choice recorded. Distinct from the record being unreadable, which is
        // what makes `whoAmI` able to tell a user which of the two they are in.
        let store = IdentityStore::in_memory().unwrap();
        store.record_path(&a_stoa(b"one"), 7).unwrap();
        assert_eq!(store.path_for(&a_stoa(b"two")).unwrap(), None);
    }

    #[test]
    fn distinct_stoas_record_distinct_paths() {
        // The spec's "Distinct choices for distinct Stoas are recorded
        // separately". The two paths differ AND the two Stoas differ, so a
        // store that keyed on nothing would fail on the second assertion and one
        // that overwrote would fail on the first.
        let store = IdentityStore::in_memory().unwrap();
        store.record_path(&a_stoa(b"one"), 1).unwrap();
        store.record_path(&a_stoa(b"two"), 2).unwrap();
        assert_eq!(store.path_for(&a_stoa(b"one")).unwrap(), Some(1));
        assert_eq!(store.path_for(&a_stoa(b"two")).unwrap(), Some(2));
    }

    #[test]
    fn a_second_choice_for_one_stoa_is_refused_and_changes_nothing() {
        // The spec's "Keeping an identity does not replace an existing one",
        // enforced by the primary key rather than by a branch. The second
        // assertion is the load-bearing one: a refusal that left the row
        // replaced anyway would satisfy the first and still lose the identity.
        let store = IdentityStore::in_memory().unwrap();
        let stoa = a_stoa(b"one");
        store.record_path(&stoa, 1).unwrap();
        assert!(store.record_path(&stoa, 2).is_err());
        assert_eq!(
            store.path_for(&stoa).unwrap(),
            Some(1),
            "a refused second choice must leave the first in place"
        );
    }

    #[test]
    fn every_recorded_pairing_reads_back_in_full() {
        // The spec's "The record is fully readable once written", which is what
        // makes a later export possible. Asserted against a hardcoded expected
        // set rather than against whatever `all_paths` happened to return.
        let store = IdentityStore::in_memory().unwrap();
        store.record_path(&a_stoa(b"one"), 11).unwrap();
        store.record_path(&a_stoa(b"two"), 22).unwrap();
        store.record_path(&a_stoa(b"three"), 33).unwrap();

        let all = store.all_paths().unwrap();
        assert_eq!(all.len(), 3, "got {all:?}");
        for (tag, path) in [(&b"one"[..], 11u32), (b"two", 22), (b"three", 33)] {
            assert!(
                all.contains(&ChosenPath {
                    stoa: a_stoa(tag),
                    path
                }),
                "the pairing for {} is missing from {all:?}",
                String::from_utf8_lossy(tag)
            );
        }
    }

    #[test]
    fn reading_the_record_back_needs_nothing_beyond_the_record() {
        // The spec: "reading them requires nothing beyond the stored record
        // itself" — no value may be derivable only on the machine that wrote it.
        //
        // Checked by reading the raw table with a SECOND connection that shares
        // nothing with the writer but the bytes: no environment, no key, no
        // in-process state. What comes back is the Stoa bytes and an integer, and
        // the test reconstructs the pairing from those alone.
        let dir = TempDir::new("readable-record");
        let path = dir.store_path();
        let stoa = a_stoa(b"one");
        {
            let store = IdentityStore::open(&path).unwrap();
            store.record_path(&stoa, 9).unwrap();
        }

        let raw = Connection::open(&path).unwrap();
        let (stored_stoa, stored_path): (Vec<u8>, i64) = raw
            .query_row("SELECT stoa, path FROM chosen_paths", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(stored_stoa, stoa.as_bytes().to_vec());
        assert_eq!(stored_path, 9);
        // And the table holds ONLY those two columns, so there is nothing else an
        // export would have to carry — and nothing machine-local hiding in a
        // column this test did not name.
        let columns: Vec<String> = raw
            .prepare("SELECT name FROM pragma_table_info('chosen_paths')")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(columns, vec!["stoa".to_string(), "path".to_string()]);
    }

    #[test]
    fn a_recorded_path_survives_a_restart() {
        // The spec's "A chosen path is readable after a restart". An in-memory
        // store cannot answer this by construction, which is why this one uses a
        // real file and drops the handle between the write and the read.
        let dir = TempDir::new("survives-restart");
        let path = dir.store_path();
        let stoa = a_stoa(b"one");
        {
            IdentityStore::open(&path)
                .unwrap()
                .record_path(&stoa, 5)
                .unwrap();
        }
        // Twice, because the spec requires it survive MORE than one restart — a
        // store that consumed its content on read would pass a single reload.
        for reload in 0..2 {
            let store = IdentityStore::open(&path).unwrap();
            assert_eq!(
                store.path_for(&stoa).unwrap(),
                Some(5),
                "reload {reload} lost the recorded path"
            );
        }
    }

    #[test]
    fn an_unknown_layout_version_is_refused_by_name() {
        // The same posture `SqliteOpLog` takes: a store written by a build whose
        // derivation scheme this one does not implement would name identities the
        // user does not have. Refused, naming both numbers.
        let dir = TempDir::new("unknown-version");
        let path = dir.store_path();
        IdentityStore::open(&path).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 9999")
            .unwrap();

        assert_eq!(
            IdentityStore::open(&path).unwrap_err(),
            IdentityStoreError::UnknownLayoutVersion {
                found: 9999,
                expected: LAYOUT_VERSION,
            }
        );
    }

    #[test]
    fn a_version_stamped_over_a_missing_table_is_refused_rather_than_read() {
        // The gap the version check cannot see, and the reason `check_layout`
        // exists: `PRAGMA user_version` is one integer anything can stamp. This
        // is what a half-restored backup looks like, and without the check it
        // opened `Ok` and failed at the first read blaming the disk.
        let dir = TempDir::new("mislabelled");
        let path = dir.store_path();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!("PRAGMA user_version = {LAYOUT_VERSION};"))
            .unwrap();
        drop(conn);

        match IdentityStore::open(&path) {
            Err(IdentityStoreError::LayoutDoesNotMatchItsVersion { version, .. }) => {
                assert_eq!(version, LAYOUT_VERSION);
            }
            other => panic!("expected a mislabelled-layout refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_table_missing_a_column_is_refused_rather_than_read() {
        // `SELECT 1 FROM chosen_paths` would pass here, which is why
        // `check_layout` names both columns. A store whose `path` column was
        // renamed or dropped is a store whose reads all fail; refusing at open
        // says so once instead of at every call.
        let dir = TempDir::new("missing-column");
        let path = dir.store_path();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE chosen_paths (stoa BLOB PRIMARY KEY NOT NULL);
             PRAGMA user_version = {LAYOUT_VERSION};"
        ))
        .unwrap();
        drop(conn);

        assert!(matches!(
            IdentityStore::open(&path),
            Err(IdentityStoreError::LayoutDoesNotMatchItsVersion { .. })
        ));
    }

    #[test]
    fn arbitrary_file_content_is_refused_without_a_panic() {
        // The store's file is not under this module's control — it can be corrupt
        // from an interrupted write or hostile from another local process. A
        // panic here aborts the module process (PHASE0-FINDINGS §3), so every
        // shape must be an error.
        let dir = TempDir::new("arbitrary-content");
        let path = dir.store_path();
        for content in [
            vec![],
            vec![0u8; 1],
            vec![0xffu8; 100],
            b"SQLite format 3\0not really".to_vec(),
            (0..=255u8).collect::<Vec<u8>>(),
        ] {
            std::fs::write(&path, &content).unwrap();
            // A panic here does not fail this test, it ABORTS the binary — which
            // is the ordering to write against, because the failure is visible
            // either way and neither way is green.
            //
            // Beyond not panicking: if the open SUCCEEDS, every read must still
            // answer rather than panic, because an accepted file is a file this
            // code goes on to query. The empty file is the case that makes this
            // assertion load-bearing — SQLite treats a zero-length file as a
            // fresh database, so it opens, and the reads then have to work.
            if let Ok(store) = IdentityStore::open(&path) {
                let _ = store.path_for(&a_stoa(b"one"));
                let _ = store.all_paths();
            }
        }
    }

    #[test]
    fn a_truncated_store_is_refused_rather_than_read() {
        // A truncation at ANY length, which is what an interrupted write leaves.
        // Separate from the arbitrary-bytes sweep because a truncated *valid*
        // store is the shape most likely to parse halfway — the header is real,
        // so a decoder that trusted it would read past the end.
        let dir = TempDir::new("truncated");
        let path = dir.store_path();
        IdentityStore::open(&path)
            .unwrap()
            .record_path(&a_stoa(b"one"), 1)
            .unwrap();
        let whole = std::fs::read(&path).unwrap();
        assert!(whole.len() > 100, "a real store should be more than a header");

        for cut in [1usize, 16, 100, whole.len() / 2, whole.len() - 1] {
            std::fs::write(&path, &whole[..cut]).unwrap();
            if let Ok(store) = IdentityStore::open(&path) {
                let _ = store.path_for(&a_stoa(b"one"));
                let _ = store.all_paths();
            }
        }
    }

    #[test]
    fn a_stored_path_outside_u32_is_refused_rather_than_clamped() {
        // NO SPEC: the spec does not say what a stored path outside `u32` means,
        // because nothing this build writes can produce one. It is reachable only
        // by editing the file, and the choice taken is to REFUSE.
        //
        // Clamping is the alternative and it is the dangerous one: a clamped path
        // derives a perfectly valid key, so the user would be handed a working
        // identity that is not the one they chose, with no error anywhere. The
        // spec calls storing an identity the user did not choose unrecoverable.
        let dir = TempDir::new("path-out-of-range");
        let path = dir.store_path();
        let stoa = a_stoa(b"one");
        IdentityStore::open(&path)
            .unwrap()
            .record_path(&stoa, 1)
            .unwrap();

        for bad in [-1i64, i64::from(u32::MAX) + 1, i64::MAX, i64::MIN] {
            Connection::open(&path)
                .unwrap()
                .execute(
                    "UPDATE chosen_paths SET path = ?1",
                    rusqlite::params![bad],
                )
                .unwrap();
            let store = IdentityStore::open(&path).unwrap();
            assert!(
                matches!(
                    store.path_for(&stoa),
                    Err(IdentityStoreError::PathOutOfRange { found, .. }) if found == bad
                ),
                "path {bad} was not refused by path_for"
            );
            // Through the full read as well, since an export must not silently
            // drop or coerce a row either.
            assert!(
                matches!(
                    store.all_paths(),
                    Err(IdentityStoreError::PathOutOfRange { found, .. }) if found == bad
                ),
                "path {bad} was not refused by all_paths"
            );
        }
    }

    #[test]
    fn a_stored_stoa_of_the_wrong_length_is_refused() {
        // NO SPEC: the spec says nothing about a malformed stored Stoa key,
        // because nothing this build writes produces one. Refused rather than
        // padded or truncated, for the same reason as the path: a coerced address
        // names a different Stoa, and no error would say so.
        let dir = TempDir::new("short-stoa");
        let path = dir.store_path();
        IdentityStore::open(&path)
            .unwrap()
            .record_path(&a_stoa(b"one"), 1)
            .unwrap();
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE chosen_paths SET stoa = ?1",
                rusqlite::params![&b"short"[..]],
            )
            .unwrap();

        assert_eq!(
            IdentityStore::open(&path).unwrap().all_paths().unwrap_err(),
            IdentityStoreError::StoaNotAnAddress { found: 5 }
        );
    }

    #[test]
    fn every_error_message_names_a_fix() {
        // The obligation `KeystoreError::Display` carries, applied here: these
        // strings reach a view as `whoAmI`'s reason, and a reason a user cannot
        // act on is a reason not worth carrying.
        //
        // Checked by requiring an imperative the reader can follow, not by
        // matching the exact prose — the `posting-capability` spec is explicit
        // that reason wording is not part of the contract.
        let errors = [
            IdentityStoreError::Storage("disk on fire".into()),
            IdentityStoreError::UnknownLayoutVersion {
                found: 9,
                expected: 1,
            },
            IdentityStoreError::LayoutDoesNotMatchItsVersion {
                version: 1,
                why: "no such table".into(),
            },
            IdentityStoreError::PathOutOfRange {
                stoa: "ab".into(),
                found: -1,
            },
            IdentityStoreError::StoaNotAnAddress { found: 5 },
        ];
        for e in errors {
            let message = e.to_string();
            assert!(
                ["check", "upgrade", "restore"]
                    .iter()
                    .any(|verb| message.contains(verb)),
                "this message names no fix: {message}"
            );
        }
    }

    #[test]
    fn the_layout_version_is_pinned_to_a_known_answer() {
        // `cargo mutants` mutates functions and not `const`s, and this project
        // has already shipped a `VERSION_1` defect that left the whole suite
        // green. A hardcoded literal is the only thing that sees a bump nobody
        // meant — and a bump is not cosmetic here, because there is no migration
        // path: it makes every existing record unopenable.
        assert_eq!(LAYOUT_VERSION, 1);
    }

    /// A fresh temporary directory, and its guard.
    ///
    /// Copied in shape from `log/sqlite.rs`'s `TempDir` rather than invented, so
    /// that two stores' tests clean up the same way: the guard must be held for
    /// the test's lifetime, and dropping it removes the directory. `std::fs`
    /// rather than a `tempfile` dependency, for the reason that file records —
    /// one need, in tests, is not worth a crate.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            // The process id and the test's own name keep two tests in one run
            // from colliding, and two runs from inheriting each other's files.
            path.push(format!("dialectica-idstore-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
            TempDir(path)
        }

        fn store_path(&self) -> std::path::PathBuf {
            IdentityStore::default_path_in(&self.0)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
