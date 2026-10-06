//! The sender identifier this peer supplies when it opens a Stoa's channel.
//!
//! # What the value has to be, and why each property is there
//!
//! `channelCreate(channelId, contentTopic, senderId)` takes a third string that
//! looks like the other two and has the opposite requirement: the channel id and
//! the topic must be **the same** on every peer, and the sender identifier must be
//! **different** on every peer. `op-transport`'s requirement "The sender identifier
//! this peer supplies is its own, stable, and says nothing about its author" asks
//! four things of it, and each is answered here by construction:
//!
//! - **Differs between installations, including two holding one identity.** SDS
//!   (LIP-109, `anoncomms/raw/sds.md`) says the sending participant "MUST include
//!   its own globally unique identifier", and a receiving participant "SHOULD
//!   ignore the message if it has a `sender_id` matching its own" — so two
//!   installations sharing a value could each discard the other's messages as
//!   their own. That is read from the SDS spec, not measured against delivery;
//!   it is why the value is 32 bytes from the OS random source rather than
//!   anything derived from what an installation holds.
//! - **The same across restarts.** It is retained, in this file's store, and read
//!   back rather than re-minted.
//! - **Differs between two Stoas.** One value per Stoa. SDS traffic carries it on
//!   every message, including to a peer that only reads, so one value per
//!   installation would link that reader's presence across every Stoa it is in.
//! - **Neither a key nor computable from one.** It is random, so there is no
//!   function from any key to it. The rejected alternative — #30's author-derived
//!   `sender_id(author_for_stoa)` — fails this and the first property together:
//!   two installations restored from one keystore would compute the same value.
//!
//! # A file of its own
//!
//! `stoas.sqlite` stamps a layout version and its `check_layout` names exactly its
//! columns, so a table added to it would reach fresh stores only, or need a version
//! bump that makes every existing membership store unopenable. `membership.rs`
//! makes the same argument about the op log; a separate file makes "adding this
//! does not make an existing store unreadable" hold without a migration.
//!
//! # Nothing here panics
//!
//! Outside `#[cfg(test)]` there is no `unwrap`, `expect` or indexing; every
//! `rusqlite` failure becomes a [`SenderError`].

use crate::identity::Address;
use rusqlite::{Connection, OptionalExtension};
use std::fmt;
use std::path::{Path, PathBuf};

/// The storage layout this build writes and understands for sender identifiers.
///
/// Independent of the other stores' versions, for the reason
/// [`crate::membership::MEMBERSHIP_LAYOUT_VERSION`] gives. Pinned by a hardcoded
/// test, because `cargo mutants` does not mutate a `const`.
pub const SENDER_LAYOUT_VERSION: i32 = 1;

/// How many random bytes a sender identifier carries.
const SENDER_BYTES: usize = 32;

/// The rendered form's head.
///
/// Structured for the reason `transport.rs` structures the channel id: the node is
/// shared with every other application in the context, and a bare hex string
/// claims nothing about what minted it. `p` for *participant*, SDS's own word,
/// beside the channel id's `c` and the topic's `s`, so the three strings
/// `channelCreate` takes are told apart by eye in a log.
const SENDER_PREFIX: &str = "/dialectica/1/p/";

/// One Stoa's sender identifier for this installation, as `channelCreate` takes it.
///
/// A newtype rather than a `String` so it cannot be passed where a channel id or a
/// topic is wanted — the three are adjacent parameters of one call.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SenderId(String);

impl SenderId {
    fn from_bytes(bytes: &[u8; SENDER_BYTES]) -> Self {
        SenderId(format!("{SENDER_PREFIX}{}", hex::encode(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a sender identifier could not be supplied.
///
/// Any of these means the channel is not opened: `op-transport` forbids opening a
/// channel under an identifier the next start would not supply again.
#[derive(Debug, PartialEq, Eq)]
pub enum SenderError {
    /// The store could not be reached, opened, read or written.
    Storage(String),
    /// The store declares a layout this build does not understand.
    UnknownLayoutVersion { found: i32, expected: i32 },
    /// The store stamps this build's layout number and does not have that layout.
    LayoutDoesNotMatchItsVersion { version: i32, why: String },
    /// The OS random source did not answer, so no identifier could be minted.
    NoRandomness,
    /// A retained row is not an identifier this build wrote.
    CorruptEntry(String),
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SenderError::Storage(why) => {
                write!(f, "the sender identifier store could not be used: {why}")
            }
            SenderError::UnknownLayoutVersion { found, expected } => write!(
                f,
                "the sender identifier store was written with storage layout version \
                 {found}, and this build understands version {expected}"
            ),
            SenderError::LayoutDoesNotMatchItsVersion { version, why } => write!(
                f,
                "the sender identifier store declares storage layout version {version} \
                 but does not have that layout: {why}"
            ),
            SenderError::NoRandomness => write!(
                f,
                "the OS random source did not answer, so no sender identifier was minted"
            ),
            SenderError::CorruptEntry(why) => {
                write!(
                    f,
                    "a retained sender identifier could not be read back: {why}"
                )
            }
        }
    }
}

impl std::error::Error for SenderError {}

/// The retained sender identifiers, one per Stoa, on disk.
#[derive(Debug)]
pub struct SenderStore {
    conn: Connection,
}

impl SenderStore {
    /// Open or create the store at `path`, refusing a layout this build cannot read.
    pub fn open(path: &Path) -> Result<Self, SenderError> {
        let conn = Connection::open(path).map_err(storage)?;
        Self::from_connection(conn)
    }

    /// An ephemeral store with no file behind it: this exact code over SQLite's
    /// `:memory:`, for tests not about persistence.
    pub fn in_memory() -> Result<Self, SenderError> {
        let conn = Connection::open_in_memory().map_err(storage)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, SenderError> {
        let found: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;
        if found == 0 {
            Self::create_schema(&conn)?;
        } else if found != SENDER_LAYOUT_VERSION {
            return Err(SenderError::UnknownLayoutVersion {
                found,
                expected: SENDER_LAYOUT_VERSION,
            });
        } else {
            Self::check_layout(&conn)?;
        }
        Ok(SenderStore { conn })
    }

    /// Prove the store has the layout its `user_version` claims, by naming the
    /// columns — `membership.rs`'s `check_layout`, for the same reason.
    fn check_layout(conn: &Connection) -> Result<(), SenderError> {
        conn.query_row("SELECT stoa, sender FROM senders LIMIT 0", [], |_| Ok(()))
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(()),
                other => Err(SenderError::LayoutDoesNotMatchItsVersion {
                    version: SENDER_LAYOUT_VERSION,
                    why: other.to_string(),
                }),
            })
    }

    /// The schema. `PRAGMA user_version` is last, as the other stores' is: it is
    /// the commit point of the layout claim, so a failure before it leaves a file
    /// at version 0 that the next open creates cleanly.
    ///
    /// **A plain `BEGIN` with the version read outside it, and safe only because
    /// one thread opens this file** — the delivery worker (design Decision 13).
    /// Two connections opening a fresh store at once would both read version 0
    /// and both `CREATE`, the race the op log had. If a second thread ever opens
    /// this store, take the op log's `create_schema` as the model: `BEGIN
    /// IMMEDIATE`, then re-read the version under the write lock.
    fn create_schema(conn: &Connection) -> Result<(), SenderError> {
        let result = conn.execute_batch(&format!(
            "BEGIN;
             CREATE TABLE senders (
                 -- The Stoa address. `PRIMARY KEY` so a second mint for one Stoa
                 -- cannot produce a second row: `INSERT OR IGNORE` keeps the first,
                 -- which is what makes the value stable rather than last-writer.
                 stoa    BLOB PRIMARY KEY NOT NULL,
                 -- The random bytes, not the rendered string, so the prefix can
                 -- never be stored twice or disagree with the one this build uses.
                 sender  BLOB NOT NULL
             ) STRICT;
             PRAGMA user_version = {SENDER_LAYOUT_VERSION};
             COMMIT;"
        ));
        if let Err(e) = result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(storage(e));
        }
        Ok(())
    }

    /// This installation's sender identifier for a Stoa: the retained one, or a
    /// new one retained before it is returned.
    ///
    /// # Retained before returned, which is the requirement
    ///
    /// An identifier handed to `channelCreate` and never written would be replaced
    /// by a different one on the next start — the channel would then see this
    /// installation as a second participant. So a mint that cannot be written is an
    /// error and nothing is returned; the caller does not open the channel.
    ///
    /// The value returned after a mint is **read back** rather than the bytes just
    /// generated: `INSERT OR IGNORE` keeps whichever row got there first, and the
    /// read is what makes the answer the retained one in every case.
    pub fn sender_for(&mut self, stoa: &Address) -> Result<SenderId, SenderError> {
        if let Some(retained) = self.retained(stoa)? {
            return Ok(retained);
        }
        let mut bytes = [0u8; SENDER_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| SenderError::NoRandomness)?;
        self.conn
            .execute(
                "INSERT OR IGNORE INTO senders (stoa, sender) VALUES (?1, ?2)",
                rusqlite::params![stoa.as_bytes().as_slice(), bytes.as_slice()],
            )
            .map_err(storage)?;
        self.retained(stoa)?.ok_or_else(|| {
            SenderError::CorruptEntry(format!(
                "the identifier minted for Stoa {} was written and is not there",
                stoa.to_hex()
            ))
        })
    }

    /// The retained identifier for a Stoa, or absence.
    fn retained(&self, stoa: &Address) -> Result<Option<SenderId>, SenderError> {
        let bytes: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT sender FROM senders WHERE stoa = ?1",
                rusqlite::params![stoa.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match bytes {
            None => Ok(None),
            Some(bytes) => {
                let bytes: [u8; SENDER_BYTES] = bytes.as_slice().try_into().map_err(|_| {
                    SenderError::CorruptEntry(format!(
                        "a retained sender identifier is {} bytes, not {SENDER_BYTES}",
                        bytes.len()
                    ))
                })?;
                Ok(Some(SenderId::from_bytes(&bytes)))
            }
        }
    }
}

/// This store's file name inside a host-supplied directory, beside
/// [`crate::membership::membership_path_in`] and
/// [`crate::log::op_log_path_in`].
pub fn sender_path_in(dir: &Path) -> PathBuf {
    dir.join("senders.sqlite")
}

fn storage(e: rusqlite::Error) -> SenderError {
    SenderError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_stoa(seed: u8) -> Address {
        Address::from_bytes([seed; 32])
    }

    /// A fresh temporary directory, removed on drop. Hand-rolled with `std::fs`,
    /// as the other stores' tests are.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("dialectica-sender-{}-{name}", std::process::id()));
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

    #[test]
    fn the_layout_version_and_file_name_are_pinned() {
        assert_eq!(SENDER_LAYOUT_VERSION, 1);
        assert_eq!(
            sender_path_in(Path::new("/a/dir")),
            PathBuf::from("/a/dir/senders.sqlite")
        );
    }

    #[test]
    fn one_stoa_asked_twice_gets_one_identifier() {
        let mut store = SenderStore::in_memory().unwrap();
        let first = store.sender_for(&a_stoa(1)).unwrap();
        let second = store.sender_for(&a_stoa(1)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn two_stoas_get_two_identifiers() {
        let mut store = SenderStore::in_memory().unwrap();
        assert_ne!(
            store.sender_for(&a_stoa(1)).unwrap(),
            store.sender_for(&a_stoa(2)).unwrap()
        );
    }

    #[test]
    fn a_reopened_store_supplies_the_identifier_it_retained() {
        // A restart is a new process opening the same file; a store that minted
        // afresh on every open would pass every in-memory test above.
        let dir = TempDir::new("reopen");
        let path = sender_path_in(&dir.0);
        let before = SenderStore::open(&path)
            .unwrap()
            .sender_for(&a_stoa(1))
            .unwrap();
        let after = SenderStore::open(&path)
            .unwrap()
            .sender_for(&a_stoa(1))
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn two_installations_supply_different_identifiers_for_one_stoa() {
        // Nothing an installation holds reaches the value, so two stores — two
        // installations — differ even though nothing distinguishes their inputs.
        let mut one = SenderStore::in_memory().unwrap();
        let mut two = SenderStore::in_memory().unwrap();
        assert_ne!(
            one.sender_for(&a_stoa(1)).unwrap(),
            two.sender_for(&a_stoa(1)).unwrap()
        );
    }

    #[test]
    fn a_sender_identifier_is_its_head_and_thirty_two_bytes_in_lowercase_hex() {
        // design.md Decision 7's rendered form, hardcoded: the head names what
        // minted the value beside the channel id's and the topic's, and the tail
        // is the row's 32 bytes. Asserted against literals, not against
        // `SENDER_PREFIX` and `SENDER_BYTES`, which would agree with any value
        // they were changed to. (This is not a spec requirement: the spec asks
        // nothing of the format, and its "nothing is a key" property is
        // `nothing_a_sender_identifier_is_made_from_is_a_key`'s.)
        let mut store = SenderStore::in_memory().unwrap();
        let sender = store.sender_for(&a_stoa(1)).unwrap();
        let tail = sender
            .as_str()
            .strip_prefix("/dialectica/1/p/")
            .expect("the identifier starts with its head");
        assert_eq!(tail.len(), 64);
        assert!(
            tail.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{tail}"
        );
    }

    /// The source of this file outside its tests, with comments and whitespace
    /// removed, so a check on it reads code and not prose.
    fn minting_code() -> String {
        include_str!("sender.rs")
            // The attribute as it opens the test module, not as the header's prose
            // mentions it.
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .expect("the file has a non-test part")
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<String>()
            .split_whitespace()
            .collect()
    }

    #[test]
    fn nothing_a_sender_identifier_is_made_from_is_a_key() {
        // `op-transport`, scenario "Nothing a sender identifier is made from is a
        // key", which the spec says is "checked by reading the code that makes a
        // sender identifier, not by comparing an identifier with a key": no key
        // reaches that code, so a comparison could not fail. This reads it.
        //
        // What it pins is the code's INPUTS, which is what the property is about:
        //  - the minting function takes a Stoa address and nothing else;
        //  - the value's only source is the OS random source;
        //  - nothing in the module names a key, a keystore or a signature, and the
        //    one thing imported from `identity` is `Address`.
        // A change that gave the identifier a key to work from must add one of
        // these, and fails here. It does not, and cannot, see a key laundered
        // through the Stoa address — the address is the row's lookup key and takes
        // no part in the value, which is what "two installations holding one
        // identity supply different identifiers" observes.
        let code = minting_code();
        assert_eq!(
            code.matches("pubfnsender_for(&mutself,stoa:&Address)")
                .count(),
            1,
            "the minting function takes something other than a Stoa address"
        );
        assert_eq!(
            code.matches("getrandom::fill(&mutbytes)").count(),
            1,
            "the value is not filled from the OS random source alone"
        );
        assert_eq!(
            code.matches("crate::identity").count(),
            1,
            "the sender identifier code reaches into `identity` for more than `Address`"
        );
        assert!(code.contains("usecrate::identity::Address;"));
        for key_word in [
            "PublicKey",
            "SecretKey",
            "public_key",
            "secret_key",
            "keystore",
            "Keystore",
            "Signature",
            "sign(",
        ] {
            assert!(
                !code.contains(key_word),
                "the sender identifier code mentions `{key_word}`"
            );
        }
    }

    #[test]
    fn every_error_says_what_went_wrong_in_its_own_words() {
        // The one thing these renderings do is fill the log line "no sender
        // identifier could be retained (<reason>)", which is how an operator learns
        // why a Stoa's channel was not requested. `cargo mutants` found `Display`
        // replaced by an empty string surviving every test. Fragments hardcoded,
        // one per variant, so no two can read alike.
        let cases = [
            (
                SenderError::Storage("disk on fire".to_string()),
                "sender identifier store could not be used: disk on fire",
            ),
            (
                SenderError::UnknownLayoutVersion {
                    found: 7,
                    expected: 1,
                },
                "layout version 7, and this build understands version 1",
            ),
            (
                SenderError::LayoutDoesNotMatchItsVersion {
                    version: 1,
                    why: "no column".to_string(),
                },
                "declares storage layout version 1 but does not have that layout: no column",
            ),
            (SenderError::NoRandomness, "random source did not answer"),
            (
                SenderError::CorruptEntry("short".to_string()),
                "could not be read back: short",
            ),
        ];
        let mut rendered = Vec::new();
        for (error, fragment) in cases {
            let text = error.to_string();
            assert!(text.contains(fragment), "`{text}` lacks `{fragment}`");
            rendered.push(text);
        }
        rendered.sort();
        rendered.dedup();
        assert_eq!(rendered.len(), 5, "two variants render alike");
    }

    #[test]
    fn a_mint_that_cannot_be_written_supplies_nothing() {
        // `query_only` makes every write fail while reads still work, which is
        // exactly "cannot be retained" — and distinguishes it from a store that
        // cannot be opened at all.
        let mut store = SenderStore::in_memory().unwrap();
        let retained = store.sender_for(&a_stoa(1)).unwrap();
        store.conn.execute_batch("PRAGMA query_only = 1").unwrap();

        assert!(matches!(
            store.sender_for(&a_stoa(2)),
            Err(SenderError::Storage(_))
        ));
        // An identifier retained before the store went read-only is still
        // supplied: it needs no write, and it is the one the next start reads.
        assert_eq!(store.sender_for(&a_stoa(1)).unwrap(), retained);
    }

    #[test]
    fn a_store_from_a_future_layout_is_refused() {
        let dir = TempDir::new("future");
        let path = sender_path_in(&dir.0);
        drop(SenderStore::open(&path).unwrap());
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 2")
            .unwrap();
        assert_eq!(
            SenderStore::open(&path).unwrap_err(),
            SenderError::UnknownLayoutVersion {
                found: 2,
                expected: 1
            }
        );
    }

    #[test]
    fn a_mislabelled_file_is_refused_at_open() {
        let dir = TempDir::new("mislabelled");
        let path = sender_path_in(&dir.0);
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE other (x INTEGER); PRAGMA user_version = 1;")
            .unwrap();
        assert!(matches!(
            SenderStore::open(&path),
            Err(SenderError::LayoutDoesNotMatchItsVersion { .. })
        ));
    }
}
