//! The read path, end to end, against real files on a real disk.
//!
//! # Why this target exists, and what the per-change suites structurally cannot do
//!
//! Every other test in this crate is an in-crate `#[cfg(test)]` module, which
//! means two things this file deliberately does not inherit:
//!
//! 1. **They can reach private items.** A unit test constructs a `Keystore` from
//!    a fixed seed, pokes a `SortKey`, calls a `pub(crate)` decoder, or borrows a
//!    fixture from `log::fixtures`. That makes them good tests of mechanism and
//!    no evidence at all that the *public API* is usable: a crate whose public
//!    surface was missing a method entirely would pass every one of them. This
//!    file imports `dialectica_core` as an outside consumer does and touches
//!    nothing private. Where a test here needs a value the crate keeps private
//!    — the field cap, the title cap — the value is **hardcoded**, which is the
//!    stronger assertion anyway: a cap that silently drifted would still refuse
//!    an absurd input and still pass every test that only probes absurd inputs.
//!
//! 2. **They mostly run in memory.** `SqliteOpLog::in_memory()` exercises the
//!    real SQL, which is most of the value, and by construction cannot survive
//!    being dropped. "The store is rebuildable by replay" is a claim about a
//!    *file*, so every store here is a file and every restart is a dropped
//!    connection and a reopened path — never a flag.
//!
//! A third reason is specific to this crate's defect history. The three bugs
//! that reached this session's review were each **cross-layer**: encode versus
//! decode, wire versus store, memory versus disk. A per-change suite cannot see
//! one, because each layer is correct in isolation and the defect lives in the
//! seam. So the tests below prefer crossing a boundary to going deep on one.
//!
//! # The defect family these fixtures are shaped against
//!
//! Every test defect found in this repo so far has been **a fixture where two
//! explanations produce the same answer**. An empty store and a refused write
//! both list nothing; a root parent and a copied thread id give the same thread;
//! `"ab"` and `"abc"` differ whether or not a length prefix is written. So each
//! fixture below is built so that only one explanation survives, and the comment
//! on the test says which rival explanation it excludes.
//!
//! The instance this file has to work hardest to avoid is asserting that
//! something "came back" using a value that came from the same call that produced
//! it. A read that returns what the write just handed us is the test agreeing
//! with itself. Where an expected value can be derived independently — an address
//! is a hash of a record, an author address is a hash of a key — it is derived
//! that way; where it cannot, it is hardcoded.
//!
//! # Mutations run against this file, and what each killed
//!
//! Every row was applied to the implementation, the suite run, and the mutation
//! reverted. A test nobody has watched fail is a test nobody knows works.
//!
//! Eleven mutations, each with the failure PREDICTED before the run and the
//! failure OBSERVED after it. Two rows disagreed, and both disagreements changed
//! this file — see the note under the table.
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `feed::list_threads` drops its `entry.op.verify()` check | forgery test alone | that test alone, `["genuine","forged"]` vs `["genuine"]` — as predicted |
//! | `feed::list_threads` keeps `Post`s with any `parent` | reply test alone | that test alone, `["root","reply"]` vs `["root"]` — as predicted |
//! | `moderation::resolve` checks authority AFTER taking the leading `Moderate` | forged-hide test alone | that test alone, `Unmoderated` vs `Hidden(..)` — as predicted |
//! | `Moderators::contains` returns `true` unconditionally | two tests | **ONE test, and at a fixture guard** — see note 1 |
//! | `SqliteOpLog::append` uses `INSERT OR REPLACE` | re-arrival test, on the Lamport value | that test, **but at the `Appended` assertion** — see note 2 |
//! | `iter_stoa`'s `WHERE stoa = ?` compares a 2-byte prefix | shared-prefix test alone | that test alone, `["right","left"]` leaked into the `left` read — as predicted |
//! | `from_connection` discards `check_layout`'s error | mislabelled-store test alone | that test alone, opened `Ok` — as predicted |
//! | `found != LAYOUT_VERSION` loosened to `found >` | foreign-version test alone | that test alone, degraded to `LayoutDoesNotMatchItsVersion{version:1,why:"no such table: ops"}` — as predicted, including the mechanism |
//! | `sanitise` stops removing invisibles | sanitise test alone | that test alone, `"hello\u{200b}world"` vs `"helloworld"` — as predicted |
//! | the paging slice loses one row per page | paging test alone | that test alone, first page 2 rows vs 3 — as predicted |
//! | `SqliteOpLog::open` ignores its path and opens `:memory:` | ~15 of 20 | **18 of 20** — see note 3 |
//!
//! **Note 1 — a mismatch that was a defect in this file.** The `contains`
//! mutation was predicted to kill two tests. It killed one, and it killed it at
//! `assert!(!moderators.contains(..))` — a FIXTURE GUARD, which aborts its test
//! before the behaviour under test runs. So the suite went red for the right
//! reason by accident: nothing had asserted what `contains` answers as a
//! behaviour, and the unauthorised-hide resolution was never reached.
//! `the_moderator_set_of_a_genesis_record_is_exactly_its_creator` was added in
//! response, asserting both directions of the predicate directly. Re-running the
//! mutation then killed two tests, one of them on the behavioural claim.
//!
//! **Note 2 — a mismatch that reordered a test.** The `INSERT OR REPLACE`
//! mutation killed the right test at the wrong assertion: `Appended::Stored` vs
//! `AlreadyPresent` fired first and the test died before checking the recorded
//! Lamport value, which is the substance of the requirement. The two `Appended`
//! values are now captured and asserted AFTER the metadata, so the mutation fails
//! on `Some(9)` vs `Some(1)` — the claim the test is named for. A test that
//! reports the shallowest of its failures hides the rest.
//!
//! **Note 3 — the load-bearing mutation.** Making every store in-memory kills
//! **18 of 20**, more than the ~15 predicted. The two survivors are the only two
//! tests that deliberately touch no store at all
//! (`the_moderator_set_of_a_genesis_record_is_exactly_its_creator` and
//! `an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa`), which is the
//! correct outcome for pure-value tests. This is what proves the persistence
//! claims here are about a file rather than about process memory: a suite where
//! this mutation killed little would be a suite whose "survives a restart" tests
//! were restarting nothing.
//!
//! # A defect this file found, and did not fix
//!
//! `an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever`
//! documents a live cross-layer defect on `main`: `Op::canonical_bytes` is
//! infallible and writes any length, while `Op::decode` refuses a field over
//! 153,600 bytes. So a 153,601-byte body signs, appends `Ok(Stored)`, and from
//! that moment every `iter`/`iter_stoa` on the store fails `CorruptEntry` —
//! permanently, across a restart, with no public API able to remove the row.
//!
//! **The test asserts the defect as it stands rather than the behaviour that
//! would be correct**, because a test asserting the fix would fail on `main` and
//! this file is not the change that fixes it. It is written so that fixing the
//! encoder flips it loudly: the assertion names the current outcome and the
//! comment names what should replace it.

use dialectica_core::arrival::{Arrival, MessageId};
use dialectica_core::feed::{self, FeedPage};
use dialectica_core::identity::{Address, PublicKey, SecretKey};
use dialectica_core::keystore::{Keystore, Unlock};
use dialectica_core::log::sqlite::LAYOUT_VERSION;
use dialectica_core::log::{Appended, Entry, OpLog, OpLogError, SqliteOpLog};
use dialectica_core::moderation::{self, Moderation, Moderators};
use dialectica_core::op::{ModerationAction, Op, OpId, OpKind, SignedOp, VoteDirection};
use dialectica_core::revision::current_version;
use dialectica_core::stoa::{Genesis, GenesisError, Policy};
use std::path::{Path, PathBuf};

// ─── Caps this crate keeps private, hardcoded here ────────────────────────

/// `op.rs`'s per-field decode cap, from §4.4's 150 KiB SDS message limit.
///
/// **Hardcoded, and deliberately not imported** — it is private, and importing
/// it would be worse if it were not: a test phrased in terms of the constant
/// moves with the constant, so a cap that drifted upward would still pass. This
/// number is the requirement; the code either meets it or does not.
///
/// **Do not update this to match the code.** If they disagree, one of them is a
/// bug and this file is the half that is not allowed to blink.
const FIELD_CAP: usize = 153_600;

/// `stoa.rs`'s genesis title cap. Hardcoded for the reason `FIELD_CAP` is.
const TITLE_CAP: usize = 1024;

// ─── Fixtures ─────────────────────────────────────────────────────────────

/// A temporary directory that removes itself, named per test.
///
/// Named per test so that `cargo test`'s default parallelism cannot make two
/// tests share a store, and so a failure leaves one identifiable directory
/// rather than a shared one two tests raced over.
///
/// Mode `0o700` is not tidiness: `Keystore::create` refuses a keystore whose
/// containing directory is group- or other-writable, so a default-mode temp
/// directory would fail the keystore tests for a reason that has nothing to do
/// with what they assert.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("dialectica-e2e-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                .expect("the temporary directory's mode is settable");
        }
        TempDir(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A store at the conventional path in this directory.
    fn store(&self) -> SqliteOpLog {
        SqliteOpLog::open(&self.file("ops.sqlite")).expect("a fresh store opens")
    }

    /// Drop a store and reopen the same path — **this is what "a restart" means
    /// here.** Not a flag, not a method on the store: the connection goes away
    /// and the file is opened again from scratch, which is the only thing that
    /// distinguishes a claim about a file from a claim about process memory.
    fn reopen(&self, store: SqliteOpLog) -> SqliteOpLog {
        drop(store);
        SqliteOpLog::open(&self.file("ops.sqlite")).expect("an existing store reopens")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A deterministic signing identity.
///
/// `from_bytes` over a fixed seed rather than `generate()`, so a failure is
/// reproducible and so two identities in one test are distinguishable by name
/// rather than by luck. Every 32-byte string is a valid Ed25519 seed, so this is
/// infallible in substance and the `expect` is a formality.
fn a_key(seed: u8) -> SecretKey {
    SecretKey::from_bytes(&[seed; 32]).expect("every 32-byte string is a valid Ed25519 seed")
}

/// A Stoa's genesis record, with `creator` as its sole moderator.
fn a_genesis(creator: &PublicKey, title: &str) -> Genesis {
    Genesis {
        creator: creator.clone(),
        policy: Policy::Open,
        title: title.to_string(),
    }
}

/// A thread head: a `Post` with no parent.
fn a_post(stoa: &Address, author: &SecretKey, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
        author: author.public_key(),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(author)
}

/// A reply: a `Post` naming a parent.
fn a_reply(stoa: &Address, author: &SecretKey, parent: &OpId, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
        author: author.public_key(),
        kind: OpKind::Post {
            thread: Some(*parent),
            parent: Some(*parent),
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(author)
}

fn a_moderation(
    stoa: &Address,
    author: &SecretKey,
    target: &OpId,
    action: ModerationAction,
) -> SignedOp {
    Op {
        stoa: *stoa,
        author: author.public_key(),
        kind: OpKind::Moderate {
            target: *target,
            action,
        },
    }
    .sign(author)
}

/// A `Post` signed by one key and **attributed to another** — a forgery.
///
/// Built by signing with `signer` after setting `author` to somebody else's key,
/// which is exactly what a hostile peer does. The log stores it deliberately
/// (§3.3), so this is the fixture that proves the reader is the thing refusing
/// it.
fn a_forged_post(stoa: &Address, claimed: &PublicKey, signer: &SecretKey, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
        author: claimed.clone(),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(signer)
}

/// The bodies a feed page renders, in order. The shape most assertions compare.
fn bodies(page: &FeedPage) -> Vec<&str> {
    page.items.iter().map(|r| r.body.text.as_str()).collect()
}

// ─── The whole chain: a keystore file, a store file, and a restart ─────────

#[test]
fn a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it() {
    // THE END-TO-END CASE, and the one that crosses the most boundaries: a
    // keystore file on disk mints the identity, the derived key signs a post, the
    // post goes into a SQLite file, both connections are dropped, and both files
    // are reopened from their paths before anything is asserted.
    //
    // The rival explanation this fixture excludes: that the author address came
    // out of the row we just wrote. It does not — the expected address is
    // re-derived from the keystore REOPENED FROM DISK, so the assertion compares
    // two independent paths to the same value. A store that stashed the address
    // as a column and handed it back would pass a weaker version of this test and
    // fail this one, because the keystore half would still have to agree.
    let dir = TempDir::new("keystore-to-feed");
    let key_path = dir.file("identity.key");

    // Mint and write the keystore. Nothing is read back from this handle.
    let minted = Keystore::generate();
    minted
        .create(&key_path, &Unlock::Unencrypted)
        .expect("a keystore is creatable in a 0700 directory");

    // The Stoa's genesis record. Its creator is a separate identity, so that
    // "the author" and "the moderator" are not the same key by accident — the
    // commonest way a moderation test passes for the wrong reason.
    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // The address is the hash of the record, so re-deriving it independently is a
    // real check rather than a restatement: `Genesis::address` could have returned
    // any 32 bytes and this would catch it.
    assert_eq!(
        stoa,
        dialectica_core::identity::stoa_address(
            &genesis.canonical_bytes().expect("a short title encodes")
        ),
        "a Stoa's address must be the hash of its genesis record"
    );

    // Sign with the key derived for THIS Stoa, from the keystore we just wrote.
    let author_key = minted.stoa_key(&stoa);
    let post = a_post(&stoa, &author_key, "First");
    let post_id = post.op.id();

    let mut store = dir.store();
    assert_eq!(
        store.append(post, Arrival::unordered()),
        Ok(Appended::Stored),
        "a fresh op is newly stored"
    );

    // THE RESTART. Both handles go away; both files are reopened from their paths.
    let store = dir.reopen(store);
    drop(minted);
    let reopened_keystore =
        Keystore::open(&key_path, &Unlock::Unencrypted).expect("the keystore reopens unencrypted");

    // The expected author address, derived from the keystore that came back off
    // disk — NOT read from the feed row being asserted on.
    let expected_author = reopened_keystore.stoa_address(&stoa).to_hex();

    let moderators = Moderators::of(&genesis).expect("a genesis record yields its moderator set");
    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false)
        .expect("a feed is readable from a reopened store");

    assert_eq!(
        bodies(&page),
        vec!["First"],
        "the post survives the restart"
    );
    assert_eq!(
        page.items[0].author, expected_author,
        "the surviving post is attributed to the address the reopened keystore derives"
    );
    // The thread id is the root post's op id, computed before the store ever saw
    // it. A store that re-derived an id from what it stored would differ here.
    assert_eq!(page.items[0].thread, post_id.to_hex());
    // Unrevised, so the rendered version IS the root. Asserting equality rather
    // than `is_some` is what would catch a resolver that returned a different op.
    assert_eq!(page.items[0].current_version, post_id.to_hex());
    assert!(!page.items[0].is_revised);
    assert!(!page.items[0].is_hidden);
}

#[test]
fn the_same_keystore_posts_under_different_addresses_in_two_stoas() {
    // §5.2's per-Stoa unlinkability, proved through the store rather than at the
    // derivation function. The rival explanation excluded: that the two feeds
    // differ because two different keystores wrote them. There is ONE keystore
    // here, reopened from one file, and it is asked for two Stoas' keys.
    //
    // Asserting the two addresses merely differ would be weak — two hashes differ
    // by default. So this also pins that each feed reports the address that
    // keystore derives FOR THAT STOA, which a derivation ignoring its Stoa
    // argument would fail.
    let dir = TempDir::new("per-stoa-address");
    let key_path = dir.file("identity.key");
    Keystore::generate()
        .create(&key_path, &Unlock::Unencrypted)
        .expect("a keystore is creatable");
    let ks = Keystore::open(&key_path, &Unlock::Unencrypted).expect("the keystore opens");

    let founder = a_key(1);
    let first = a_genesis(&founder.public_key(), "Agora");
    let second = a_genesis(&founder.public_key(), "Lyceum");
    let a = first.address().expect("a short title encodes");
    let b = second.address().expect("a short title encodes");
    assert_ne!(
        a, b,
        "two titles must give two Stoas, or this proves nothing"
    );

    let mut store = dir.store();
    store
        .append(
            a_post(&a, &ks.stoa_key(&a), "in Agora"),
            Arrival::unordered(),
        )
        .expect("a post is storable");
    store
        .append(
            a_post(&b, &ks.stoa_key(&b), "in Lyceum"),
            Arrival::unordered(),
        )
        .expect("a post is storable");
    let store = dir.reopen(store);

    let page_a = feed::list_threads(
        &store,
        &Moderators::of(&first).expect("moderators"),
        &a,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    let page_b = feed::list_threads(
        &store,
        &Moderators::of(&second).expect("moderators"),
        &b,
        0,
        20,
        false,
    )
    .expect("a feed is readable");

    assert_eq!(bodies(&page_a), vec!["in Agora"]);
    assert_eq!(bodies(&page_b), vec!["in Lyceum"]);

    // Each feed reports the address this keystore derives for that Stoa — the
    // independent derivation, not the row.
    assert_eq!(page_a.items[0].author, ks.stoa_address(&a).to_hex());
    assert_eq!(page_b.items[0].author, ks.stoa_address(&b).to_hex());
    assert_ne!(
        page_a.items[0].author, page_b.items[0].author,
        "one identity must present two addresses across two Stoas (§5.2)"
    );
}

// ─── Empty versus unreadable, at three boundaries ──────────────────────────
//
// This is the project's defect family at its sharpest: an empty store and a
// broken one both produce an empty listing, so a test that only checks "the feed
// is empty" cannot tell them apart. Each pair below asserts the two outcomes are
// DIFFERENT KINDS of answer, not merely different values.

#[test]
fn an_empty_store_answers_every_read_and_a_missing_file_is_a_created_one() {
    // A fresh path is CREATED rather than refused — so "no file yet" is not an
    // error, and every read over it is an empty answer rather than a failure.
    // The rival explanation excluded: that the reads returned empty because they
    // failed. They are asserted as `Ok`, and the error case is the next test.
    let dir = TempDir::new("empty-store");
    let path = dir.file("ops.sqlite");
    assert!(!path.exists(), "the fixture must start with no file");

    let store = SqliteOpLog::open(&path).expect("a missing store is created, not refused");
    assert!(path.exists(), "opening a fresh path writes the file");

    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    assert_eq!(store.len(), Ok(0));
    assert_eq!(store.is_empty(), Ok(true));
    assert_eq!(store.iter(), Ok(vec![]));
    assert_eq!(store.iter_stoa(&stoa), Ok(vec![]));
    // A target read over an op id nothing names. `from_hex` is the only public
    // route to an `OpId`, so the value is a literal rather than a hash of
    // anything the store produced.
    let absent = OpId::from_hex(&"7a".repeat(32)).expect("64 hex characters is an op id");
    assert_eq!(store.iter_target(&absent), Ok(vec![]));
    assert_eq!(store.get(&absent), Ok(None));

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("an empty store yields an empty feed, not an error");
    assert_eq!(page.items, vec![]);
    assert!(!page.has_more, "there is no further page of nothing");
    assert_eq!(page.page, 0);
}

#[test]
fn a_store_written_by_another_layout_version_is_refused_naming_both_numbers() {
    // The empty-versus-unreadable pair's second half, at the OPEN boundary: this
    // store is not empty and not readable, and it says so as a DIFFERENT variant
    // from the mislabelled case below. A build that accepted any version would
    // open it and report an empty feed — indistinguishable from the test above.
    //
    // A NEGATIVE version, deliberately, rather than `LAYOUT_VERSION + 1`: a
    // negative number is guaranteed to be a version nothing will ever legitimately
    // stamp, so this fixture stays valid as `LAYOUT_VERSION` grows. A test phrased
    // as `+ 1` becomes a test of the NEXT version the moment the cap moves.
    let dir = TempDir::new("foreign-version");
    let path = dir.file("foreign.sqlite");
    let foreign_version: i32 = -7;

    let conn = rusqlite_stamp(&path, foreign_version, None);
    drop(conn);

    match SqliteOpLog::open(&path) {
        Err(OpLogError::UnknownLayoutVersion { found, expected }) => {
            assert_eq!(
                found, foreign_version,
                "the refusal names the version found"
            );
            // `expected` is the build's own number, so comparing it to
            // LAYOUT_VERSION is the one place reading the constant is right: the
            // claim is that the error reports the build's version, not that the
            // version is any particular value.
            assert_eq!(
                expected, LAYOUT_VERSION,
                "the refusal names the version this build understands"
            );
        }
        other => panic!("a foreign layout version must be refused as unknown, got {other:?}"),
    }
}

#[test]
fn a_store_stamping_our_layout_without_our_tables_is_refused_as_mislabelled() {
    // The third boundary, and the one the discarded suite could NOT reach from a
    // test target — it recorded that building such a file needed the private
    // schema DDL. It does not: the file only has to carry our version number and
    // NOT our table, and `rusqlite` is a dependency of this crate so a test can
    // write one directly.
    //
    // The distinction being pinned is the whole point: a store claiming our
    // layout and not having it is `LayoutDoesNotMatchItsVersion`, NOT
    // `UnknownLayoutVersion` (that is a store with somebody else's honest number)
    // and NOT `Storage` (that would blame the disk, which is fine). Three
    // explanations, three variants — and a build without the layout check would
    // open this Ok and fail later as `Storage("no such table: ops")`.
    let dir = TempDir::new("mislabelled");
    let path = dir.file("mislabelled.sqlite");

    // Our version, stamped over a table that is not ours.
    let conn = rusqlite_stamp(
        &path,
        LAYOUT_VERSION,
        Some("CREATE TABLE something_else (x INTEGER);"),
    );
    drop(conn);

    match SqliteOpLog::open(&path) {
        Err(OpLogError::LayoutDoesNotMatchItsVersion { version, why }) => {
            assert_eq!(
                version, LAYOUT_VERSION,
                "the refusal names the version the file claimed"
            );
            assert!(
                !why.is_empty(),
                "the refusal must say what was wrong, not merely that something was"
            );
        }
        other => panic!("a store claiming our layout without our tables must be refused as mislabelled, got {other:?}"),
    }
}

#[test]
fn a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed() {
    // The fourth way a read can fail, and the one most likely to be swallowed:
    // the path exists and holds bytes that are not SQLite at all. This must not
    // come back as an empty feed, which is §11.1 obligation 5 — "an empty feed is
    // indistinguishable from a Stoa nobody has posted in".
    //
    // Paired with the empty-store test above: same API calls, same shape of
    // answer wanted, and the two must differ in KIND. That pairing is what makes
    // this more than an error-message test.
    let dir = TempDir::new("not-a-database");
    let path = dir.file("garbage.sqlite");
    // A SQLite file begins "SQLite format 3\0". These bytes deliberately do not,
    // and are long enough that the header check is what refuses them rather than
    // the file being too short to have a header at all.
    std::fs::write(
        &path,
        b"this is not a database, it is a text file\n".repeat(8),
    )
    .expect("a file is writable");

    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // The failure may surface at open or at the first read depending on when
    // SQLite reads the header; either is correct, and what matters is that it is
    // a failure rather than an empty answer. So the assertion covers both routes
    // and refuses the one outcome that would be wrong.
    let outcome = SqliteOpLog::open(&path).and_then(|store| {
        feed::list_threads(
            &store,
            &Moderators::of(&genesis).expect("moderators"),
            &stoa,
            0,
            20,
            false,
        )
    });
    match outcome {
        Err(_) => {}
        Ok(page) => panic!(
            "a file that is not a database must fail rather than read as a quiet Stoa; \
             got a page of {} item(s)",
            page.items.len()
        ),
    }
}

/// Write a SQLite file carrying `version`, plus an optional statement batch.
///
/// A test helper rather than a fixture from the crate, because the crate's schema
/// DDL is private and this deliberately does not want it: the point is to build a
/// file the crate did NOT write.
fn rusqlite_stamp(path: &Path, version: i32, ddl: Option<&str>) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("a database is creatable");
    if let Some(ddl) = ddl {
        conn.execute_batch(ddl).expect("the DDL applies");
    }
    conn.execute_batch(&format!("PRAGMA user_version = {version};"))
        .expect("the version is stampable");
    conn
}

// ─── What a reader refuses, with the store holding it anyway ────────────────

#[test]
fn a_forged_post_is_not_rendered_even_though_the_store_holds_it() {
    // §3.3's whole design in one test: the log stores junk, the reader refuses it.
    //
    // The rival explanation this fixture excludes: that the forgery is absent
    // from the feed because it was never stored. So the test asserts BOTH halves
    // against the SAME store — `iter_stoa` returns two entries and the feed
    // returns one. A test that only checked the feed would pass against a log
    // that had silently dropped the forgery on append, which is behaviour §3.3
    // specifically forbids.
    let dir = TempDir::new("forgery");
    let victim = a_key(1);
    let attacker = a_key(2);
    let genesis = a_genesis(&victim.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    store
        .append(a_post(&stoa, &victim, "genuine"), Arrival::unordered())
        .expect("a genuine post is storable");
    // Attributed to the victim, signed by the attacker.
    store
        .append(
            a_forged_post(&stoa, &victim.public_key(), &attacker, "forged"),
            Arrival::unordered(),
        )
        .expect("the log stores a forgery deliberately");
    let store = dir.reopen(store);

    assert_eq!(
        store.iter_stoa(&stoa).expect("the store is readable").len(),
        2,
        "the LOG must hold both — it is not the log's job to filter (§3.3)"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    assert_eq!(
        bodies(&page),
        vec!["genuine"],
        "the READER must refuse the forgery"
    );
}

#[test]
fn a_reply_is_not_a_thread_head_in_the_feed() {
    // A `Post` with a parent is a reply, and the feed lists heads. The rival
    // explanation excluded: that the reply is missing because it failed to store
    // or failed to verify. Both are ruled out by asserting it IS in the store and
    // IS resolvable as a current version — it is present, valid, and correctly
    // not a head.
    let dir = TempDir::new("reply-not-head");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let root = a_post(&stoa, &author, "root");
    let root_id = root.op.id();
    let reply = a_reply(&stoa, &author, &root_id, "reply");
    let reply_id = reply.op.id();

    let mut store = dir.store();
    store.append(root, Arrival::unordered()).expect("storable");
    store.append(reply, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    assert_eq!(store.len(), Ok(2), "both ops are in the store");
    assert!(
        current_version(&store, &reply_id)
            .expect("the store is readable")
            .is_some(),
        "the reply is a valid, resolvable post — it is simply not a head"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    assert_eq!(
        bodies(&page),
        vec!["root"],
        "a feed lists thread heads, and a reply is not one"
    );
    // The head's thread id is its own op id, and the reply's is not rendered at
    // all. Naming both ids excludes the fixture where one is substituted for the
    // other and the count still comes to one.
    assert_eq!(page.items[0].thread, root_id.to_hex());
    assert_ne!(
        page.items[0].thread,
        reply_id.to_hex(),
        "the two ops must be distinguishable, or the count above proves nothing"
    );
}

// ─── Moderation across the store boundary ──────────────────────────────────

#[test]
fn a_hide_by_the_moderator_removes_a_thread_from_the_default_feed_and_marks_it_in_the_other() {
    // Moderation resolved on READ, from a reopened file. The fixture carries TWO
    // threads so that "hidden" is distinguishable from "the feed is empty" — the
    // defect family again: a resolver that hid everything, and one that hid the
    // right one, both produce a shorter list, and only a survivor tells them
    // apart.
    let dir = TempDir::new("hide");
    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let doomed = a_post(&stoa, &founder, "doomed");
    let doomed_id = doomed.op.id();
    let spared = a_post(&stoa, &founder, "spared");

    let mut store = dir.store();
    store
        .append(doomed, Arrival::unordered())
        .expect("storable");
    store
        .append(spared, Arrival::unordered())
        .expect("storable");
    store
        .append(
            a_moderation(&stoa, &founder, &doomed_id, ModerationAction::Hide),
            Arrival::unordered(),
        )
        .expect("storable");
    let store = dir.reopen(store);

    let moderators = Moderators::of(&genesis).expect("moderators");

    // The resolver's own answer, so that a feed change and a resolver change are
    // distinguishable rather than both showing up only as a shorter list.
    let resolved = moderation::resolve(&store, &moderators, &doomed_id).expect("readable");
    assert!(
        resolved.is_hidden(),
        "the founder is the sole moderator and hid this op"
    );
    assert!(
        resolved.deciding_op().is_some(),
        "a hide names the op that decided it"
    );

    let default = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&default),
        vec!["spared"],
        "the default feed omits the hidden thread and keeps the other"
    );

    let shown = feed::list_threads(&store, &moderators, &stoa, 0, 20, true).expect("readable");
    // Both come back, and the hidden one is FLAGGED. §9.1: "a reader who asked to
    // see what was hidden is owed the knowledge of which ones those were." A
    // include_hidden that returned both unflagged would pass a count-only check.
    let mut shown_bodies = bodies(&shown);
    shown_bodies.sort_unstable();
    assert_eq!(shown_bodies, vec!["doomed", "spared"]);
    let doomed_row = shown
        .items
        .iter()
        .find(|r| r.body.text == "doomed")
        .expect("the hidden thread is in the include_hidden feed");
    let spared_row = shown
        .items
        .iter()
        .find(|r| r.body.text == "spared")
        .expect("the visible thread is too");
    assert!(doomed_row.is_hidden, "the hidden one is marked hidden");
    assert!(!spared_row.is_hidden, "the other is not");
}

#[test]
fn the_moderator_set_of_a_genesis_record_is_exactly_its_creator() {
    // `Moderators::contains` is public because a UI asks it to decide whether to
    // offer a hide button, so it is asserted in its own right rather than only as
    // a fixture guard in the tests below. A mutation making it answer `true`
    // unconditionally was caught ONLY by a guard assertion before this test
    // existed — and a guard aborts its test before the behaviour under test runs,
    // so the suite reported the right count for the wrong reason.
    //
    // Both directions are asserted, which is what a membership predicate needs: a
    // `contains` returning `true` always and one returning `false` always are
    // different bugs, and a one-sided test catches only one of them.
    let founder = a_key(1);
    let outsider = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let moderators = Moderators::of(&genesis).expect("moderators");

    assert!(
        moderators.contains(&founder.public_key()),
        "the creator named in the record is a moderator"
    );
    assert!(
        !moderators.contains(&outsider.public_key()),
        "nobody else is — the initial set is the creator alone (§6)"
    );
    // The set belongs to the Stoa the record addresses. Re-derived from the
    // record rather than read off the `Moderators`, so a `stoa()` returning
    // something else would be caught.
    assert_eq!(
        moderators.stoa(),
        &genesis.address().expect("a short title encodes"),
        "a moderator set is scoped to one Stoa"
    );
}

#[test]
fn a_hide_by_a_non_moderator_leaves_the_thread_visible() {
    // Authority, not merely signature validity. This hide is PROPERLY SIGNED by a
    // key that is simply not a moderator — so a resolver checking only `verify()`
    // would honour it. The rival explanation excluded: that the hide was ignored
    // because it was malformed or forged. It is neither; it is unauthorised.
    let dir = TempDir::new("unauthorised-hide");
    let founder = a_key(1);
    let rando = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &founder, "survives");
    let post_id = post.op.id();
    let hide = a_moderation(&stoa, &rando, &post_id, ModerationAction::Hide);
    // The hide is genuinely valid as a signature — that is what makes this test
    // about authority. Asserting it excludes the reading that it was skipped as a
    // forgery.
    assert!(
        hide.verify(),
        "the unauthorised hide must be validly signed, or this tests the wrong thing"
    );

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    store.append(hide, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    let moderators = Moderators::of(&genesis).expect("moderators");
    assert!(
        !moderators.contains(&rando.public_key()),
        "the fixture's outsider must not be a moderator"
    );
    assert_eq!(
        moderation::resolve(&store, &moderators, &post_id),
        Ok(Moderation::Unmoderated),
        "a hide from a non-moderator binds nothing"
    );

    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&page),
        vec!["survives"],
        "an unauthorised hide must not remove a thread"
    );
}

#[test]
fn a_forged_hide_does_not_displace_the_genuine_one_that_sorts_after_it() {
    // The censorship-resistance case, and the reason `moderation::resolve` skips
    // non-binding ops rather than taking the leading moderation and then checking
    // it. A resolver doing the latter would report `Unmoderated` here — the
    // attacker's op sorts FIRST and is invalid, so checking-after-taking discards
    // the genuine hide behind it.
    //
    // The fixture's load-bearing part is the ORDER, and it is arranged rather
    // than hoped for: the forged op is given a higher Lamport timestamp, which
    // `cmp_ops` places first. Without that the test would pass or fail on a hash
    // coincidence — exactly the "hunted a shared byte" failure `log/mod.rs`'s own
    // test module records.
    let dir = TempDir::new("forged-hide-ordering");
    let founder = a_key(1);
    let attacker = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &founder, "hidden by a moderator");
    let post_id = post.op.id();

    // Signed by the attacker, attributed to the FOUNDER — so a resolver checking
    // `contains(author)` without `verify()` would honour it.
    let forged_unhide = Op {
        stoa,
        author: founder.public_key(),
        kind: OpKind::Moderate {
            target: post_id,
            action: ModerationAction::Unhide,
        },
    }
    .sign(&attacker);
    assert!(
        !forged_unhide.verify(),
        "the fixture's forgery must not verify, or it is not a forgery"
    );

    let genuine_hide = a_moderation(&stoa, &founder, &post_id, ModerationAction::Hide);

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    // The forgery sorts FIRST: ordered ops lead, and among them the higher
    // Lamport value comes first.
    store
        .append(
            forged_unhide.clone(),
            Arrival::ordered(9, MessageId::new(vec![1])),
        )
        .expect("storable");
    store
        .append(
            genuine_hide.clone(),
            Arrival::ordered(1, MessageId::new(vec![2])),
        )
        .expect("storable");
    let store = dir.reopen(store);

    // Prove the ordering the test depends on, rather than assuming it. If this
    // ever stops holding, this test stops testing what it claims and says so here
    // instead of passing quietly.
    let about: Vec<OpId> = store
        .iter_target(&post_id)
        .expect("readable")
        .iter()
        .map(Entry::id)
        .collect();
    assert_eq!(
        about,
        vec![forged_unhide.op.id(), genuine_hide.op.id()],
        "the fixture requires the forgery to sort ahead of the genuine hide"
    );

    let moderators = Moderators::of(&genesis).expect("moderators");
    let resolved = moderation::resolve(&store, &moderators, &post_id).expect("readable");
    assert_eq!(
        resolved,
        Moderation::Hidden(
            store
                .get(&genuine_hide.op.id())
                .expect("readable")
                .expect("the genuine hide is stored")
        ),
        "a forgery sorting first must be skipped, not allowed to displace a genuine hide"
    );

    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&page),
        Vec::<&str>::new(),
        "the genuine hide still governs what is rendered"
    );
}

// ─── Persistence properties that are about a FILE ──────────────────────────

#[test]
fn a_stored_op_reads_back_byte_identical_across_a_restart() {
    // §3.3's byte-stability claim, against a file. The expected value is the
    // op's bytes taken BEFORE the store existed, so the comparison is against
    // something the store did not produce — the failure mode the project's
    // defect list names as `assert_eq!(bytes[0], VERSION_1)`, avoided by holding
    // a copy from outside.
    let dir = TempDir::new("byte-identical");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let op = a_post(&stoa, &author, "exactly these bytes");
    let expected_bytes = op.to_bytes();
    let expected_id = op.op.id();
    let expected_signature = op.signature.to_bytes();

    let mut store = dir.store();
    store.append(op, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    let back = store
        .get(&expected_id)
        .expect("readable")
        .expect("the op is in the store");
    assert_eq!(
        back.op.to_bytes(),
        expected_bytes,
        "the stored op must read back byte-identical"
    );
    assert_eq!(back.op.signature.to_bytes(), expected_signature);
    assert_eq!(back.id(), expected_id, "and its id is unchanged");
    assert!(back.op.verify(), "and it still verifies after a round trip");
}

#[test]
fn a_second_arrival_does_not_overwrite_the_first_ones_recorded_metadata() {
    // First-arrival-wins, across a restart. The rival explanation excluded: that
    // the second append was refused. It is not — it reports `AlreadyPresent`,
    // which is asserted, so the op WAS offered again and the metadata still did
    // not move.
    //
    // The two Lamport values are 1 and 9, and the SECOND is the higher, so a
    // richer-wins or last-wins implementation would show 9 here. A fixture whose
    // second value was lower would pass under both rules.
    let dir = TempDir::new("first-arrival-wins");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let op = a_post(&stoa, &author, "arrives twice");
    let id = op.op.id();

    let mut store = dir.store();
    let first = store.append(op.clone(), Arrival::ordered(1, MessageId::new(vec![0xAA])));
    let second = store.append(op, Arrival::ordered(9, MessageId::new(vec![0xBB])));
    let store = dir.reopen(store);

    // THE METADATA CLAIM IS ASSERTED FIRST, and the two `Appended` values after
    // it. That ordering is deliberate: an `INSERT OR REPLACE` mutation reports
    // `Stored` for the second append, and with the report asserted first the test
    // died there without ever checking the metadata — the substance of the
    // requirement went unexercised while the suite still showed one red test.
    // A test that reports the shallowest of its failures hides the rest.
    assert_eq!(store.len(), Ok(1), "one op id is one row");
    let back = store.get(&id).expect("readable").expect("stored");
    assert_eq!(
        back.arrival.lamport(),
        Some(1),
        "the FIRST arrival's Lamport value survives, not the higher second one"
    );
    assert_eq!(
        back.arrival.message_id().map(MessageId::as_bytes),
        Some(&[0xAA][..]),
        "and the first arrival's message id with it"
    );

    assert_eq!(first, Ok(Appended::Stored), "the first append stores");
    assert_eq!(
        second,
        Ok(Appended::AlreadyPresent),
        "a re-append is expected traffic, reported rather than refused"
    );
}

#[test]
fn two_stoas_whose_addresses_share_a_leading_byte_do_not_leak_into_each_other() {
    // A prefix-comparison bug in `iter_stoa` would pass a test using two
    // unrelated addresses, because unrelated addresses differ in byte 0. So this
    // CONSTRUCTS the shared prefix rather than hunting for titles that happen to
    // collide — the failure `log/mod.rs`'s test module records as having passed a
    // 2-byte prefix leak.
    //
    // `Address::from_bytes` takes a fixed array, so the two addresses here are
    // literals differing only in their LAST byte. They are not derived from any
    // genesis record, which is fine: `iter_stoa` filters on the address in the
    // op, and nothing in this test reads a genesis.
    let dir = TempDir::new("shared-prefix");
    let author = a_key(1);
    let mut left_bytes = [0x5Au8; 32];
    let mut right_bytes = [0x5Au8; 32];
    left_bytes[31] = 0x01;
    right_bytes[31] = 0x02;
    let left = Address::from_bytes(left_bytes);
    let right = Address::from_bytes(right_bytes);
    assert_eq!(
        left.as_bytes()[..31],
        right.as_bytes()[..31],
        "the fixture requires a 31-byte shared prefix"
    );
    assert_ne!(left, right);

    let mut store = dir.store();
    store
        .append(a_post(&left, &author, "left"), Arrival::unordered())
        .expect("storable");
    store
        .append(a_post(&right, &author, "right"), Arrival::unordered())
        .expect("storable");
    let store = dir.reopen(store);

    let in_left: Vec<String> = store
        .iter_stoa(&left)
        .expect("readable")
        .iter()
        .map(|e| match &e.op.op.kind {
            OpKind::Post { body, .. } => body.clone(),
            other => panic!("the fixture stores only posts, got {other:?}"),
        })
        .collect();
    assert_eq!(
        in_left,
        vec!["left".to_string()],
        "a read restricted to one Stoa must match the COMPLETE address"
    );

    let in_right: Vec<String> = store
        .iter_stoa(&right)
        .expect("readable")
        .iter()
        .map(|e| match &e.op.op.kind {
            OpKind::Post { body, .. } => body.clone(),
            other => panic!("the fixture stores only posts, got {other:?}"),
        })
        .collect();
    assert_eq!(in_right, vec!["right".to_string()]);
}

#[test]
fn a_page_past_the_end_is_an_empty_page_and_the_pages_before_it_tile_the_feed() {
    // Paging over a real file, and the case most likely to panic on a slice. The
    // rival explanation excluded: that page 1 is empty because paging is broken
    // rather than because the feed ends. Page 0 is asserted FULL and the two
    // pages are asserted to tile — every body appears exactly once across them —
    // so an off-by-one that dropped or duplicated a row fails here even though
    // each page's length would still look plausible.
    let dir = TempDir::new("paging");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    // Five distinguishable bodies, so a dropped or duplicated row is visible by
    // name rather than only as a count.
    for n in 0..5u8 {
        store
            .append(
                a_post(&stoa, &author, &format!("post {n}")),
                Arrival::unordered(),
            )
            .expect("storable");
    }
    let store = dir.reopen(store);
    let moderators = Moderators::of(&genesis).expect("moderators");

    let first = feed::list_threads(&store, &moderators, &stoa, 0, 3, false).expect("readable");
    assert_eq!(first.items.len(), 3);
    assert_eq!(first.page, 0);
    assert!(first.has_more, "two of five remain after a page of three");

    let second = feed::list_threads(&store, &moderators, &stoa, 1, 3, false).expect("readable");
    assert_eq!(second.items.len(), 2);
    assert_eq!(second.page, 1);
    assert!(!second.has_more, "nothing follows the last two");

    // The two pages tile the feed: every body once, none twice, none missing.
    let mut seen: Vec<&str> = bodies(&first);
    seen.extend(bodies(&second));
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec!["post 0", "post 1", "post 2", "post 3", "post 4"],
        "the pages must tile the feed exactly — no gap, no repeat"
    );

    // Past the end: an empty page, and NOT a panic and NOT a wrapped first page.
    let past = feed::list_threads(&store, &moderators, &stoa, 99, 3, false).expect("readable");
    assert_eq!(past.items, vec![], "a page past the end is empty");
    assert_eq!(past.page, 99, "and reports the page that was asked for");
    assert!(!past.has_more);
}

// ─── Hostile input reached from the public API ─────────────────────────────

#[test]
fn an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever() {
    // A LIVE CROSS-LAYER DEFECT, asserted as it stands. See this file's header.
    //
    // `Op::canonical_bytes` is infallible and writes a 4-byte length prefix for
    // any body; `Op::decode` refuses a field over 153,600 bytes. So one byte over
    // the cap signs, appends `Ok(Stored)`, and every subsequent ordered read of
    // the store fails `CorruptEntry` — across a restart, permanently, with no
    // public API able to remove the row. A peer that accepted such a body from
    // its own UI would brick every feed read it has.
    //
    // The AT-CAP half of the pair is what makes this a fencepost test rather than
    // an absurd-value test: exactly 153,600 must round-trip, so a cap tightened to
    // `>=` fails here. A test using only `u32::MAX` would pass under that
    // tightening — which is the second entry on this project's list of tests that
    // could not fail for the reason they named.
    let dir = TempDir::new("over-cap-body");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // At the cap: encodes, stores, reads back, renders.
    {
        let at_cap = a_post(&stoa, &author, &"x".repeat(FIELD_CAP));
        let id = at_cap.op.id();
        let mut store = SqliteOpLog::open(&dir.file("at-cap.sqlite")).expect("opens");
        store
            .append(at_cap, Arrival::unordered())
            .expect("storable");
        drop(store);
        let store = SqliteOpLog::open(&dir.file("at-cap.sqlite")).expect("reopens");
        let back = store
            .get(&id)
            .expect("a body of exactly the cap must read back")
            .expect("stored");
        match &back.op.op.kind {
            OpKind::Post { body, .. } => assert_eq!(
                body.len(),
                FIELD_CAP,
                "a field of exactly the cap survives the round trip"
            ),
            other => panic!("expected a post, got {other:?}"),
        }
        assert_eq!(
            feed::list_threads(
                &store,
                &Moderators::of(&genesis).expect("moderators"),
                &stoa,
                0,
                20,
                false
            )
            .expect("a feed over an at-cap body is readable")
            .items
            .len(),
            1
        );
    }

    // One byte over: signs, appends, and the store is unreadable from then on.
    let over = a_post(&stoa, &author, &"x".repeat(FIELD_CAP + 1));
    let over_id = over.op.id();
    // The encoder produced bytes the decoder refuses. Asserting both directions
    // is what names this as an ASYMMETRY rather than as a decoder limit: the
    // encode succeeded, so the two halves disagree.
    let encoded = over.to_bytes();
    assert!(
        encoded.len() > FIELD_CAP,
        "the fixture must actually exceed the cap"
    );
    assert!(
        SignedOp::from_bytes(&encoded).is_err(),
        "this peer's own encoder produced bytes its own decoder refuses — the defect"
    );

    let mut store = SqliteOpLog::open(&dir.file("over-cap.sqlite")).expect("opens");
    assert_eq!(
        store.append(over, Arrival::unordered()),
        Ok(Appended::Stored),
        "the append SUCCEEDS, which is what makes this unrecoverable"
    );

    // Every ordered read now fails, before and after a restart.
    drop(store);
    let store = SqliteOpLog::open(&dir.file("over-cap.sqlite")).expect("the file still opens");
    match store.iter() {
        Err(OpLogError::CorruptEntry(_)) => {}
        other => panic!(
            "EXPECTED-DEFECT: an over-cap row must currently poison iter(); got {other:?}. \
             If the encoder now refuses an over-cap body, this test should be replaced by one \
             asserting the refusal happens BEFORE the append."
        ),
    }
    match store.iter_stoa(&stoa) {
        Err(OpLogError::CorruptEntry(_)) => {}
        other => panic!("EXPECTED-DEFECT: the Stoa read is poisoned too; got {other:?}"),
    }
    // `len` counts rows without decoding them, so the store still reports one op
    // it cannot hand back — which is what makes the row invisible to any repair a
    // caller could attempt through this API.
    assert_eq!(
        store.len(),
        Ok(1),
        "the row is counted but undecodable, and no public method can remove it"
    );
    assert!(
        matches!(store.get(&over_id), Err(OpLogError::CorruptEntry(_))),
        "not even a direct get can retrieve it"
    );
}

#[test]
fn an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa() {
    // The genesis record's cap, and the contrast that makes the defect above a
    // defect: here the encode is FALLIBLE, so an over-long title is refused at
    // `canonical_bytes` and never reaches an address. `Op::canonical_bytes` is
    // the one that is not.
    //
    // The at-cap half is the fencepost again: exactly 1024 must encode.
    let founder = a_key(1);

    let at_cap = a_genesis(&founder.public_key(), &"t".repeat(TITLE_CAP));
    assert!(
        at_cap.canonical_bytes().is_ok(),
        "a title of exactly the cap must encode"
    );
    assert!(
        at_cap.address().is_ok(),
        "and must therefore have an address"
    );

    let over = a_genesis(&founder.public_key(), &"t".repeat(TITLE_CAP + 1));
    assert_eq!(
        over.canonical_bytes(),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "one byte over the cap is refused, naming the length found"
    );
    assert_eq!(
        over.address(),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "so it never acquires an address"
    );
    assert_eq!(
        Moderators::of(&over),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "and no moderator set can be built from it"
    );
}

#[test]
fn a_vote_is_stored_and_is_rendered_by_nothing() {
    // What a vote currently does, pinned so that the UI brief's claim — votes are
    // stored and read by nothing, so there is no score to render — is checkable
    // rather than asserted in prose.
    //
    // The rival explanation excluded: that the vote is absent from the feed
    // because it was not stored. `iter_target` is asserted to return it, so it IS
    // in the store and IS associated with the post; the feed simply does not
    // render it and no public API reports a count.
    //
    // NO SPEC: no live requirement in `openspec/specs/` says what a vote does to
    // a feed — `relevance-ordering` is an unarchived change. This pins today's
    // behaviour so that adding a score is a visible change rather than a silent
    // one.
    let dir = TempDir::new("vote");
    let author = a_key(1);
    let voter = a_key(2);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &author, "voted on");
    let post_id = post.op.id();
    let vote = Op {
        stoa,
        author: voter.public_key(),
        kind: OpKind::Vote {
            target: post_id,
            direction: VoteDirection::Up,
        },
    }
    .sign(&voter);
    let vote_id = vote.op.id();

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    store.append(vote, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    // The vote is in the store and names the post.
    assert_eq!(
        store
            .iter_target(&post_id)
            .expect("readable")
            .iter()
            .map(Entry::id)
            .collect::<Vec<_>>(),
        vec![vote_id],
        "a target read returns every op naming that target, whatever its kind"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("readable");
    assert_eq!(
        bodies(&page),
        vec!["voted on"],
        "a vote is not a thread head and does not appear as a row"
    );
    // The row carries no score field at all — there is nothing for a view to
    // render. Pinned by the row's own shape: if a score is ever added, this
    // comparison against a fully-specified row fails and someone has to decide
    // what the view does with it.
    assert!(!page.items[0].is_revised);
    assert!(!page.items[0].is_hidden);
    assert_eq!(page.items[0].attachments, vec![]);
}

#[test]
fn a_body_carrying_invisible_characters_is_sanitised_on_the_way_out_of_the_store() {
    // Sanitisation across the store boundary: the body goes onto disk verbatim
    // (it is inside the signature preimage and must not be altered) and comes out
    // of the FEED cleaned, with a count of what was removed.
    //
    // The rival explanation excluded: that the store altered the body. The stored
    // op is read back and asserted to hold the ORIGINAL bytes, so the change is
    // provably the feed's and not the store's. A test that only checked the feed
    // would pass if the store had silently rewritten the row — which would break
    // every signature.
    let dir = TempDir::new("sanitise");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // A zero-width space (U+200B) between two visible words.
    let raw = "hello\u{200B}world";
    let post = a_post(&stoa, &author, raw);
    let id = post.op.id();
    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    // The STORE keeps the body exactly as signed.
    let back = store.get(&id).expect("readable").expect("stored");
    match &back.op.op.kind {
        OpKind::Post { body, .. } => assert_eq!(
            body, raw,
            "the stored body is inside the signature preimage and must be untouched"
        ),
        other => panic!("expected a post, got {other:?}"),
    }
    assert!(
        back.op.verify(),
        "and it still verifies, which it would not if the store had rewritten it"
    );

    // The FEED renders it cleaned, and says how much it removed.
    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("readable");
    assert_eq!(
        page.items[0].body.text, "helloworld",
        "the invisible character is removed for display"
    );
    assert_eq!(
        page.items[0].body.removed, 1,
        "and counted, so a view can mark it"
    );
    assert!(!page.items[0].body.is_clean());
}
