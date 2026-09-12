//! The write path, end to end, against a real SQLite file on a real disk.
//!
//! # Why this is a separate target and not more `#[cfg(test)]` modules
//!
//! Every test in this crate before now was an in-crate unit test, which means two
//! things this file deliberately does not inherit:
//!
//! 1. **They can reach private items.** A unit test can construct a `Keystore`
//!    from a fixed seed, poke a `SortKey`, or call a `pub(crate)` decoder. That
//!    makes them good tests of mechanism and useless as evidence that the *public
//!    API* is usable — a crate whose public surface was missing a method entirely
//!    would pass all of them.
//! 2. **They mostly run in memory.** `SqliteOpLog::in_memory()` exercises the real
//!    SQL, which is most of the value, and by construction cannot survive being
//!    dropped. "The store is rebuildable by replay" is a claim about a file.
//!
//! So this target imports `dialectica_core` as an outside consumer would, touches
//! nothing private, and puts every store on disk. If something here needs a
//! private item, that is a finding about the public API rather than a reason to
//! move the test.
//!
//! # The defect family these tests are shaped against
//!
//! This project's every test defect found so far has been **a fixture where two
//! explanations produce the same answer**. Each test below therefore names, in a
//! comment, the mutation it would catch — and each was run against that mutation
//! before being kept. Where a test could NOT be made to fail, that is said rather
//! than implied.
//!
//! The commonest instance here is the one the seeder also has to avoid: asserting
//! that something "came back" when the value asserted on came from the same call
//! that produced it. A read that returns what the write just handed us is the test
//! agreeing with itself.
//!
//! # Mutations run against this suite, and what each killed
//!
//! Every row was applied, the suite run, and the mutation reverted. The point of
//! recording it is that a test nobody has watched fail is a test nobody knows works.
//!
//! | Mutation | Killed by |
//! |---|---|
//! | `create_reply` sets `thread: Some(*parent)` unconditionally | `a_reply_is_filed_under_the_thread_its_parent_belongs_to` — and ONLY that one, because it is the only fixture two levels deep |
//! | `joined_stoa` fabricates a record instead of refusing | `a_post_into_a_stoa_this_peer_has_not_joined_is_refused` |
//! | `read_thread` drops its `verify()` check | `a_forged_op_in_the_store_is_not_rendered_in_a_thread` |
//! | `INSERT OR IGNORE` → `INSERT OR REPLACE` on `stoas` | `joining_a_stoa_this_peer_created_does_not_downgrade_it_to_joined` |
//! | `list_stoas` orders by `rowid DESC` instead of address | `list_stoas_is_in_address_order_and_is_stable_across_a_restart` |
//! | both target checks removed from `postable_target` | `a_reply_across_stoas_and_a_vote_on_a_non_post_are_refused_distinguishably` |
//! | body bound `>` → `>=` (the fencepost) | `an_oversized_body_is_refused_rather_than_signed_or_truncated` — caught by the at-bound half of the pair, which an absurd-value test would not have |
//! | every store becomes `SqliteOpLog::in_memory()` | SIX tests: all four restart tests and both disk-failure tests. This is the one that proves the persistence claims are about a file rather than about process memory |
//!
//! **One mutation was NOT caught by this file and is recorded as such**: the
//! `stoas` half of `check_layout` being deleted is killed by a unit test in
//! `log/sqlite.rs` rather than from here, because building a store with a correct
//! `ops` table and no `stoas` needs the schema DDL, which is private.
//!
//! # A defect this suite found
//!
//! `every_write_handler_refuses_malformed_json_with_the_error_shape` failed on its
//! first run, on the input `[]`. Every handler parsed the request as a
//! `serde_json::Value` and then used `.get(..)`, which returns `None` on an array —
//! indistinguishable from a missing field. For handlers whose fields are all
//! optional that meant an array was served as an empty request: `list_stoas("[]")`
//! returned a successful page. The fix is `wire::parse_request`, which is now the
//! one place a request becomes fields, and it applies to the pre-existing read-path
//! handlers too. See its doc comment for why no existing test caught it.

use dialectica_core::identity::{Address, PublicKey, SecretKey};
use dialectica_core::log::{OpLog, Relation, SqliteOpLog, StoaRegistry};
use dialectica_core::op::{OpId, VoteDirection};
use dialectica_core::publish;
use dialectica_core::stoa::Genesis;

// ─── Fixtures ─────────────────────────────────────────────────────────────

/// A temporary directory that removes itself.
///
/// Named per test, so a failure leaves one identifiable directory rather than a
/// shared one two tests raced over — and so `cargo test`'s default parallelism
/// does not make two tests share a store.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("dialectica-writepath-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
        TempDir(path)
    }

    fn file(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }

    fn store(&self) -> SqliteOpLog {
        SqliteOpLog::open(&self.file("ops.sqlite")).expect("a fresh store opens")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A signing identity.
///
/// # Generated directly, and that is honest rather than a shortcut
///
/// `SecretKey::generate()` is the whole of what the write path needs from identity
/// — see `root_signing_key` in the module crate for the seam. **Keystore creation
/// is separately owned and deliberately not exercised here**: a test that wrote a
/// keystore would be testing the other agent's surface, and would fail for reasons
/// that have nothing to do with the write path the moment that surface changed.
///
/// What this costs is stated plainly: these tests do not prove a key survives a
/// restart, because the key is not what persists here — the ops and the Stoas are.
/// The restart tests below re-derive the author address from the in-memory key,
/// which is the correct thing to check at this seam, since the key's own durability
/// is the keystore's property and has its own tests in `keystore.rs`.
///
/// Takes no directory argument for that reason: nothing about this touches disk.
fn an_identity() -> SecretKey {
    SecretKey::generate()
}

/// Create a Stoa through the public API and return its address and record.
fn a_stoa(store: &mut SqliteOpLog, creator: &PublicKey, title: &str) -> (Address, Genesis) {
    let joined = publish::create_stoa(store, creator, title).expect("a Stoa is creatable");
    let address = joined.address().expect("a short title encodes");
    (address, joined.genesis)
}

// ─── The happy path, end to end ───────────────────────────────────────────

#[test]
fn identity_then_stoa_then_post_reply_vote_then_read_the_thread_back() {
    // THE END-TO-END CASE. Every step goes through the public API, and every
    // assertion is against a value computed independently of the call that
    // produced it — which is the discipline this project's defect family demands.
    let dir = TempDir::new("end-to-end");
    let key = an_identity();
    let mut store = dir.store();

    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    // The address is the hash of the record, so re-deriving it independently is a
    // real check rather than a restatement: `create_stoa` could have returned any
    // 32 bytes and this would catch it.
    assert_eq!(
        stoa,
        dialectica_core::identity::stoa_address(&genesis.canonical_bytes().unwrap()),
        "the Stoa address must be the hash of its genesis record"
    );

    let post = publish::create_post(&mut store, &key, &stoa, "First", &[])
        .expect("a post into a joined Stoa is publishable");
    let reply = publish::create_reply(&mut store, &key, &stoa, &post.op.id(), "Second", &[])
        .expect("a reply to a stored post is publishable");
    publish::create_vote(&mut store, &key, &stoa, &post.op.id(), VoteDirection::Up)
        .expect("a vote on a stored post is publishable");

    // Read the thread back. The moderator set comes from the genesis record, which
    // is the only way to build one.
    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page = publish::read_thread(&store, &moderators, &stoa, &post.op.id(), 0, 20, false)
        .expect("a thread is readable");

    // Two posts: the root and the reply. The VOTE must not appear — it is not a
    // post, and a thread read that returned it would be rendering an op no
    // resolver approved. Asserting the count is what catches that; asserting only
    // "the root is present" would pass with the vote in the list.
    assert_eq!(
        page.items.len(),
        2,
        "a thread is its root and its replies, and nothing else: {:?}",
        page.items.iter().map(|i| &i.post).collect::<Vec<_>>()
    );

    // The root is first and has no parent. Its absence is the assertion — a
    // `parent` of `Some("")` or `Some(root)` would both be wrong and both would
    // pass an `is_some()`-shaped check.
    assert_eq!(page.items[0].post, post.op.id().to_hex());
    assert_eq!(page.items[0].parent, None, "the thread root has no parent");
    assert_eq!(page.items[0].body.text, "First");

    // The reply names the root as its parent, and its body is its own.
    assert_eq!(page.items[1].post, reply.op.id().to_hex());
    assert_eq!(
        page.items[1].parent,
        Some(post.op.id().to_hex()),
        "a reply must name the post it replies to"
    );
    assert_eq!(page.items[1].body.text, "Second");

    // Both are attributed to the author's address, re-derived here rather than
    // read back from the row.
    let expected_author = key.public_key().address().to_hex();
    for item in &page.items {
        assert_eq!(item.author, expected_author);
        assert!(!item.is_revised);
        assert!(!item.is_hidden);
    }
}

#[test]
fn a_reply_is_filed_under_the_thread_its_parent_belongs_to() {
    // The derived-thread property, and the mutation it catches is specific:
    // `create_reply` could set `thread: Some(*parent)` unconditionally, which is
    // CORRECT for a reply to a root and WRONG for a reply to a reply. A test with
    // only one level of nesting cannot tell the two apart — which is this project's
    // defect family exactly, so the fixture goes two deep.
    let dir = TempDir::new("nested-thread");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let root = publish::create_post(&mut store, &key, &stoa, "Root", &[]).unwrap();
    let first = publish::create_reply(&mut store, &key, &stoa, &root.op.id(), "One", &[]).unwrap();
    // A reply to the REPLY. Its thread must still be the root, not `first`.
    let second =
        publish::create_reply(&mut store, &key, &stoa, &first.op.id(), "Two", &[]).unwrap();

    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page = publish::read_thread(&store, &moderators, &stoa, &root.op.id(), 0, 20, false)
        .unwrap();

    // All three in ONE thread. With `thread: Some(*parent)` the grandchild would
    // be filed under `first` and this read would return two rows.
    assert_eq!(
        page.items.len(),
        3,
        "a reply to a reply belongs to the root's thread, got {:?}",
        page.items.iter().map(|i| &i.post).collect::<Vec<_>>()
    );

    // And the parent chain is preserved — the thread is flat in storage but the
    // tree is recoverable, which is what a view needs to indent.
    let by_id: std::collections::HashMap<_, _> =
        page.items.iter().map(|i| (i.post.clone(), i)).collect();
    assert_eq!(
        by_id[&second.op.id().to_hex()].parent,
        Some(first.op.id().to_hex()),
        "the grandchild's PARENT is the reply, even though its THREAD is the root"
    );
}

// ─── Restart: the store survives, and replay reproduces it ────────────────

#[test]
fn everything_resolves_identically_after_closing_and_reopening_the_store() {
    // PLAN.md says the store is rebuildable by replay. THIS IS THE PROOF, and the
    // shape of it matters: the store is dropped — closing the SQLite connection —
    // and a NEW one is opened at the same path. Nothing is carried across in
    // memory except the op ids, which are hashes computed from the ops themselves.
    //
    // The mutation this catches is any use of `:memory:` or a temporary file in the
    // write path, and any store state held only in RAM. `SqliteOpLog::in_memory`
    // would pass every other test in this file.
    let dir = TempDir::new("restart");
    let key = an_identity();

    let (stoa, genesis, post_id, reply_id, before) = {
        let mut store = dir.store();
        let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
        let post = publish::create_post(&mut store, &key, &stoa, "First", &["cid-one".into()])
            .unwrap();
        let reply =
            publish::create_reply(&mut store, &key, &stoa, &post.op.id(), "Second", &[]).unwrap();
        publish::create_vote(&mut store, &key, &stoa, &post.op.id(), VoteDirection::Up).unwrap();

        let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
        let before =
            publish::read_thread(&store, &moderators, &stoa, &post.op.id(), 0, 20, false).unwrap();
        (stoa, genesis, post.op.id(), reply.op.id(), before)
        // `store` drops here. The connection closes.
    };

    // A NEW store at the same path.
    let store = dir.store();

    // The op count survived. Three ops: post, reply, vote.
    assert_eq!(
        store.len().unwrap(),
        3,
        "every op must survive a restart, not merely the ones a thread read shows"
    );

    // The Stoa registry survived, and with its relation intact. This is the half a
    // test that only checked ops would miss entirely — and the `stoas` table is new,
    // so it is the half most likely to be absent.
    let joined = store
        .get_stoa(&stoa)
        .unwrap()
        .expect("a created Stoa must still be known after a restart");
    assert_eq!(joined.relation, Relation::Created);
    assert_eq!(joined.genesis, genesis, "the genesis record must round-trip");

    // The thread resolves IDENTICALLY. Comparing the whole page rather than a
    // field or two: a restart that lost attachments, or is_revised, or the parent
    // links, would pass a spot check on the bodies.
    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let after =
        publish::read_thread(&store, &moderators, &stoa, &post_id, 0, 20, false).unwrap();
    assert_eq!(
        after, before,
        "a thread must resolve identically across a restart"
    );

    // And the attachment specifically, because it is the field most likely to be
    // dropped silently — it is a list, and an empty list is a plausible-looking
    // answer.
    assert_eq!(
        after.items[0].attachments.len(),
        1,
        "an attachment must survive a restart"
    );
    assert_eq!(after.items[0].attachments[0].text, "cid-one");

    // The reply is still reachable by id, which is what a reply-to-a-reply needs.
    assert!(
        store.get(&reply_id).unwrap().is_some(),
        "a stored op must be retrievable by id after a restart"
    );
}

#[test]
fn a_post_published_before_a_restart_is_repliable_after_one() {
    // Restart is not only a read property. A peer that could read its own history
    // but not build on it would pass the test above — so this replies, after the
    // reopen, to a post written before it.
    let dir = TempDir::new("restart-then-write");
    let key = an_identity();

    let (stoa, genesis, post_id) = {
        let mut store = dir.store();
        let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
        let post = publish::create_post(&mut store, &key, &stoa, "Before", &[]).unwrap();
        (stoa, genesis, post.op.id())
    };

    let mut store = dir.store();
    // The Stoa must still be joined, or this is `NotJoined` rather than a reply —
    // which would be the same failure wearing a different error.
    publish::create_reply(&mut store, &key, &stoa, &post_id, "After", &[])
        .expect("a post from before the restart must be repliable after it");

    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page = publish::read_thread(&store, &moderators, &stoa, &post_id, 0, 20, false).unwrap();
    assert_eq!(page.items.len(), 2);
}

#[test]
fn two_identities_in_one_store_stay_distinct_across_a_restart() {
    // Two authors, one Stoa, one file. The mutation this catches is an author
    // column or an attribution that used ambient state rather than the signing
    // key — which a single-identity fixture cannot see, because one author is
    // indistinguishable from "whoever wrote last".
    let dir = TempDir::new("two-authors");
    let alice = an_identity();
    let bob = an_identity();
    assert_ne!(
        alice.public_key(),
        bob.public_key(),
        "the fixture needs two genuinely different identities"
    );

    let (stoa, genesis, alice_post, bob_reply) = {
        let mut store = dir.store();
        let (stoa, genesis) = a_stoa(&mut store, &alice.public_key(), "Agora");
        let a = publish::create_post(&mut store, &alice, &stoa, "Alice's", &[]).unwrap();
        // Bob replies in a Stoa Alice created. He can, because this peer's store
        // knows the Stoa — `NotJoined` is about the PEER, not about the author.
        let b = publish::create_reply(&mut store, &bob, &stoa, &a.op.id(), "Bob's", &[]).unwrap();
        (stoa, genesis, a.op.id(), b.op.id())
    };

    let store = dir.store();
    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page = publish::read_thread(&store, &moderators, &stoa, &alice_post, 0, 20, false).unwrap();

    let by_id: std::collections::HashMap<_, _> =
        page.items.iter().map(|i| (i.post.clone(), i)).collect();
    assert_eq!(
        by_id[&alice_post.to_hex()].author,
        alice.public_key().address().to_hex()
    );
    assert_eq!(
        by_id[&bob_reply.to_hex()].author,
        bob.public_key().address().to_hex()
    );
    // And they differ, so the test cannot pass by attributing both to one person.
    assert_ne!(
        by_id[&alice_post.to_hex()].author,
        by_id[&bob_reply.to_hex()].author
    );
}

// ─── Empty versus unreadable, through the real API on a real file ─────────

#[test]
fn an_empty_store_and_an_unreadable_one_are_distinguishable() {
    // UI-BRIEF obligation 5, proved through the public API against real files:
    // "An empty feed and 'we could not read the store' look identical and mean
    // opposite things."
    //
    // Both halves are needed and neither is sufficient. A test that only checked
    // the empty case would pass for an implementation that returned empty for
    // EVERYTHING, which is precisely the defect.
    let dir = TempDir::new("empty-vs-unreadable");
    let key = an_identity();

    // EMPTY: a real, valid, freshly-created store with a joined Stoa and no posts.
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Quiet");
    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();

    let empty = dialectica_core::feed::list_threads(&store, &moderators, &stoa, 0, 20, false)
        .expect("a quiet Stoa is a successful read, not a failure");
    assert!(empty.items.is_empty(), "nobody has posted, so there are no threads");
    assert!(!empty.has_more);
    drop(store);

    // UNREADABLE: the same path, corrupted. Not deleted — a deleted file is
    // recreated by `open`, which is the wrong thing to test and would pass
    // trivially. This overwrites the header with bytes that are not a SQLite
    // database, which is what a truncated copy or a bad restore looks like.
    let path = dir.file("ops.sqlite");
    std::fs::write(&path, b"this is not a SQLite database, not even slightly")
        .expect("the file is writable");

    let reopened = SqliteOpLog::open(&path);
    assert!(
        reopened.is_err(),
        "a file that is not a database must not open as an empty store"
    );

    // And the two are distinguishable BY A CALLER, which is the actual obligation
    // — not merely different internally. One is `Ok` with an empty list, the other
    // is `Err` with a message that names the problem.
    let err = reopened.unwrap_err().to_string();
    assert!(
        !err.is_empty(),
        "the failure must carry a reason a view can render"
    );
    assert!(
        !err.contains("items"),
        "a storage failure must not be phrased as a feed, got {err:?}"
    );
}

#[test]
fn a_read_of_a_corrupt_store_is_an_error_at_the_wire_boundary_and_never_an_empty_page() {
    // The same obligation one layer up, where a view actually reads: through the
    // JSON handler. `feed.rs` returning `Err` buys nothing if `wire.rs` flattens
    // it into `{"items":[]}` — and that flattening is a one-line change nothing
    // else here would catch.
    let dir = TempDir::new("corrupt-at-wire");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    publish::create_post(&mut store, &key, &stoa, "First", &[]).unwrap();
    drop(store);

    let genesis_hex = hex::encode(genesis.canonical_bytes().unwrap());
    let request = format!(
        r#"{{"stoa":"{}","genesis":"{}"}}"#,
        stoa.to_hex(),
        genesis_hex
    );

    // While the store is good, the handler answers with a page. This half is what
    // stops the test passing because the REQUEST was malformed.
    let good = dialectica_core::list_threads_from_request(&request, || {
        SqliteOpLog::open(&dir.file("ops.sqlite"))
    });
    let v: serde_json::Value = serde_json::from_str(&good).unwrap();
    assert_eq!(
        v["items"].as_array().map(|a| a.len()),
        Some(1),
        "the fixture must read successfully before it is corrupted, got {good}"
    );

    // Now corrupt it and ask the same question.
    std::fs::write(dir.file("ops.sqlite"), b"not a database at all").unwrap();
    let bad = dialectica_core::list_threads_from_request(&request, || {
        SqliteOpLog::open(&dir.file("ops.sqlite"))
    });
    let v: serde_json::Value = serde_json::from_str(&bad).unwrap();
    assert!(v.get("error").is_some(), "got {bad}");
    assert!(
        v.get("items").is_none(),
        "a failure must never also carry a result — §2.5 forbids the partial \
         success shape, and a view checking `items` first would render an empty \
         feed: {bad}"
    );
}

#[test]
fn list_stoas_distinguishes_no_stoas_from_an_unreadable_store() {
    // Obligation 5 on the FIRST screen a user sees. "You have joined no Stoas" is
    // the state of every new install, so it is the one most likely to be confused
    // with a broken store — and a new user shown a broken peer as an empty one has
    // no way to tell.
    let dir = TempDir::new("list-stoas-empty-vs-broken");

    // A valid, empty registry.
    let store = dir.store();
    let out = dialectica_core::list_stoas("{}", &store);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["items"].as_array().map(|a| a.len()),
        Some(0),
        "a fresh peer is in no Stoas, which is a successful answer: {out}"
    );
    assert!(v.get("error").is_none(), "got {out}");
    drop(store);

    // The same question of a store that cannot be opened at all. Opened here
    // rather than through the handler because the handler takes an open store —
    // which is itself the finding that the ADAPTER owns this boundary, and the
    // adapter is cfg'd out of `cargo test`. So this asserts the half that is
    // reachable: that the failure is a failure.
    std::fs::write(dir.file("ops.sqlite"), b"not a database").unwrap();
    assert!(
        SqliteOpLog::open(&dir.file("ops.sqlite")).is_err(),
        "a corrupt registry must not open as an empty one"
    );
}

#[test]
fn a_store_declaring_our_layout_without_the_stoas_table_is_reachable_from_the_public_api() {
    // `LayoutDoesNotMatchItsVersion` existed before this change with no public-API
    // test reaching it. IT IS REACHABLE, and this is the proof — via `open`, which
    // is the only way a caller gets a store.
    //
    // The file built here is the one a version-1 store hand-stamped to 2 looks
    // like, and the one a half-restored backup looks like.
    let dir = TempDir::new("layout-mismatch-public");
    let path = dir.file("ops.sqlite");

    // A real store, then its `stoas` table dropped and the version left in place.
    // Dropping from a REAL store rather than hand-building, so `ops` is genuinely
    // correct and only the second table is missing — otherwise the first check
    // decides and this tests the wrong half.
    {
        let store = dir.store();
        drop(store);
    }
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE stoas;").unwrap();
    drop(conn);

    let err = SqliteOpLog::open(&path).expect_err("a mislabelled store must be refused at open");
    let message = err.to_string();
    // The message must say the file is not the layout it declares, NOT blame the
    // disk. That distinction is the whole reason the variant exists: a disk error
    // is retried and a mislabelled store is not.
    assert!(
        message.contains("declares storage layout version"),
        "the refusal must say the file is mislabelled rather than blaming the \
         disk, got {message:?}"
    );
    assert!(
        message.contains("stoas"),
        "and it must name the part that was missing, got {message:?}"
    );
}

#[test]
fn an_unwritable_directory_is_reported_rather_than_silently_losing_ops() {
    // A disk failure mode the write path must not swallow. An op that was accepted
    // and not stored is worse than a refused one: the user believes they posted.
    let dir = TempDir::new("unwritable");
    let key = an_identity();

    // A subdirectory made read-only AFTER the store is created, so the failure is
    // on the write rather than on the open — which is the harder case and the one
    // a caller is least likely to expect.
    let sub = dir.file("ro");
    std::fs::create_dir_all(&sub).unwrap();
    let path = sub.join("ops.sqlite");
    let mut store = SqliteOpLog::open(&path).expect("a store is creatable while writable");
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    // Close the connection before revoking write, so SQLite is not holding a
    // handle that keeps working.
    drop(store);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0500: readable and traversable, not writable. SQLite needs to create a
        // journal beside the database, so this blocks the write without making the
        // file unreadable — which is the realistic "full or read-only filesystem"
        // shape rather than a deleted file.
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o500)).unwrap();

        let result = SqliteOpLog::open(&path).and_then(|mut store| {
            publish::create_post(&mut store, &key, &stoa, "Doomed", &[])
                .map_err(|e| match e {
                    publish::PublishError::Storage(e) => e,
                    // Any other variant means the write failed for a reason that
                    // is not the disk, which would make this test pass for the
                    // wrong reason. Surfaced as a distinctive message rather than
                    // silently accepted.
                    other => dialectica_core::log::OpLogError::CorruptEntry(format!(
                        "expected a storage failure, got {other:?}"
                    )),
                })
                .map(|_| ())
        });

        // Restore permissions BEFORE asserting, so a failure does not leave an
        // undeletable directory behind and break the next run.
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o700)).unwrap();

        let err = result.expect_err("a write to a read-only directory must be reported");
        let message = err.to_string();
        assert!(
            message.contains("storage") || message.contains("readonly"),
            "the failure must name the storage problem, got {message:?}"
        );
    }
}

// ─── Hostile input at every new entry point ───────────────────────────────

#[test]
fn every_write_handler_refuses_malformed_json_with_the_error_shape() {
    // The blanket sweep, over every new handler at once. The property is the
    // contract's, so it is checked as a property rather than as nine happy paths:
    // §2.5 says failure is ALWAYS `{"error":"..."}` and never a partial success.
    //
    // A handler that panicked here would abort the module process
    // (PHASE0-FINDINGS §3), so "does not unwind" is as much the assertion as
    // "reports an error".
    let dir = TempDir::new("hostile-json");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let post = publish::create_post(&mut store, &key, &stoa, "First", &[]).unwrap();

    let bad_inputs = [
        "",
        "not json",
        "[]",
        "null",
        "7",
        r#"{"#,
        r#"{"stoa":}"#,
        // A deeply nested object, which is the shape that overflows a recursive
        // parser. serde_json has its own depth limit; asserting we report rather
        // than abort is the point.
        &"[".repeat(400),
    ];

    for bad in bad_inputs {
        // Each handler in turn. The key and the store are real, so a refusal here
        // is about the INPUT and not about a missing prerequisite.
        let replies = [
            dialectica_core::create_post(bad, &mut store, &key),
            dialectica_core::create_reply(bad, &mut store, &key),
            dialectica_core::create_vote(bad, &mut store, &key),
            dialectica_core::create_stoa(bad, &mut store, &key.public_key()),
            dialectica_core::join_stoa(bad, &mut store),
            dialectica_core::list_stoas(bad, &store),
            dialectica_core::get_thread(bad, &store, &genesis),
        ];
        for reply in replies {
            let v: serde_json::Value = serde_json::from_str(&reply)
                .unwrap_or_else(|e| panic!("a handler emitted invalid JSON ({e}) for {bad:?}: {reply}"));
            assert!(
                v.get("error").is_some(),
                "malformed input {bad:?} must be the error shape, got {reply}"
            );
            // Never a partial success. A reply carrying both an error and an `op`
            // or `items` would render as a success in any view checking those
            // first.
            for forbidden in ["op", "items", "stoa"] {
                assert!(
                    v.get(forbidden).is_none(),
                    "a failure must not also carry `{forbidden}` — §2.5: {reply}"
                );
            }
        }
    }

    // And nothing was written by any of that. The count is the assertion: a
    // handler that refused the input but had already appended an op would pass
    // every check above.
    assert_eq!(
        store.len().unwrap(),
        1,
        "no malformed request may store an op; only the fixture's post should exist"
    );
    let _ = post;
}

#[test]
fn a_missing_field_and_a_wrong_typed_one_are_different_errors() {
    // Both are refusals, and they are DIFFERENT mistakes — the rule every parser
    // in `wire.rs` follows, because "missing field: body" sent about a field that
    // is right there sends someone looking in the wrong place.
    //
    // The mutation this catches is a parser collapsing to one message, which is the
    // tempting simplification and is invisible to a test that only checks
    // `is_some()` on `error`.
    let dir = TempDir::new("missing-vs-wrong-typed");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let hex = stoa.to_hex();

    let missing = dialectica_core::create_post(&format!(r#"{{"stoa":"{hex}"}}"#), &mut store, &key);
    let wrong_typed = dialectica_core::create_post(
        &format!(r#"{{"stoa":"{hex}","body":42}}"#),
        &mut store,
        &key,
    );

    let m: serde_json::Value = serde_json::from_str(&missing).unwrap();
    let w: serde_json::Value = serde_json::from_str(&wrong_typed).unwrap();
    assert!(
        m["error"].as_str().unwrap().contains("missing"),
        "an absent body must say so, got {missing}"
    );
    assert!(
        w["error"].as_str().unwrap().contains("must be a string"),
        "a wrong-typed body must say so, got {wrong_typed}"
    );
    assert_ne!(
        m["error"], w["error"],
        "the two mistakes must not collapse into one message"
    );
}

#[test]
fn an_oversized_body_is_refused_rather_than_signed_or_truncated() {
    // The encode-side bound. Truncating would change what the author said, which
    // is the one thing a signature exists to prevent — so the test asserts BOTH
    // that it was refused and that nothing was stored.
    let dir = TempDir::new("oversized");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    // One byte over. The BOUNDARY, not an absurd value: a check written as `>=`
    // or applied to the wrong unit passes an absurd-value test and fails this one.
    let too_long = "x".repeat(publish::MAX_BODY_BYTES + 1);
    let err = publish::create_post(&mut store, &key, &stoa, &too_long, &[])
        .expect_err("a body one byte over the bound must be refused");
    assert!(
        matches!(err, publish::PublishError::BodyTooLong { .. }),
        "got {err:?}"
    );
    assert_eq!(store.len().unwrap(), 0, "a refused post must not be stored");

    // And exactly at the bound is ACCEPTED, which is the other half of the pair.
    // Without it the check could refuse everything and the assertion above would
    // still pass.
    let at_bound = "x".repeat(publish::MAX_BODY_BYTES);
    publish::create_post(&mut store, &key, &stoa, &at_bound, &[])
        .expect("a body of exactly MAX_BODY_BYTES must be accepted");
    assert_eq!(store.len().unwrap(), 1);
}

#[test]
fn too_many_attachments_and_an_oversized_one_are_refused_distinguishably() {
    // Two bounds that a single "attachments are too big" error would collapse,
    // and they have different fixes: drop some, versus shorten one.
    let dir = TempDir::new("attachments");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let too_many: Vec<String> = (0..=publish::MAX_ATTACHMENTS).map(|i| i.to_string()).collect();
    let count_err = publish::create_post(&mut store, &key, &stoa, "body", &too_many)
        .expect_err("one attachment over the count bound must be refused");
    assert!(
        matches!(count_err, publish::PublishError::TooManyAttachments { .. }),
        "got {count_err:?}"
    );

    let one_too_long = vec!["x".repeat(publish::MAX_ATTACHMENT_BYTES + 1)];
    let size_err = publish::create_post(&mut store, &key, &stoa, "body", &one_too_long)
        .expect_err("one attachment over the size bound must be refused");
    assert!(
        matches!(size_err, publish::PublishError::AttachmentTooLong { .. }),
        "got {size_err:?}"
    );

    // The two must not be the same error. Asserting the variants above is not
    // enough on its own — the messages are what a user reads.
    assert_ne!(count_err.to_string(), size_err.to_string());
    assert_eq!(store.len().unwrap(), 0, "neither refusal may store an op");

    // And the accepted side of both bounds, so the checks cannot be
    // refuse-everything.
    let at_count: Vec<String> = (0..publish::MAX_ATTACHMENTS).map(|i| i.to_string()).collect();
    publish::create_post(&mut store, &key, &stoa, "body", &at_count)
        .expect("exactly MAX_ATTACHMENTS must be accepted");
}

#[test]
fn a_post_into_a_stoa_this_peer_has_not_joined_is_refused() {
    // THE OUTBOUND AUTHORISATION CHECK. A Stoa this peer has no genesis record for
    // is one it cannot resolve moderators for and cannot read back — so signing
    // into it produces an op the author can never see.
    //
    // The address used is a REAL, well-formed Stoa address, built from a genuine
    // genesis record that simply was not joined. A random 32 bytes would also be
    // refused, and would be refused by any implementation that checked nothing
    // beyond "is this in the table" — the distinction matters because it is the
    // difference between testing the check and testing hex parsing.
    let dir = TempDir::new("not-joined");
    let key = an_identity();
    let mut store = dir.store();

    let elsewhere = Genesis {
        creator: key.public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Somewhere else".to_string(),
    };
    let elsewhere_address = elsewhere.address().unwrap();

    let err = publish::create_post(&mut store, &key, &elsewhere_address, "Hello", &[])
        .expect_err("a post into an unjoined Stoa must be refused");
    assert!(matches!(err, publish::PublishError::NotJoined), "got {err:?}");
    assert_eq!(store.len().unwrap(), 0, "a refused post must not be stored");

    // And after JOINING the very same Stoa, the identical call succeeds. This is
    // what proves the refusal was about membership rather than about anything else
    // in the request — the same bytes, the same key, one difference.
    publish::join_stoa(&mut store, &elsewhere_address, &elsewhere).unwrap();
    publish::create_post(&mut store, &key, &elsewhere_address, "Hello", &[])
        .expect("the same post must succeed once the Stoa is joined");
    assert_eq!(store.len().unwrap(), 1);
}

#[test]
fn a_reply_to_an_op_this_peer_does_not_hold_is_refused() {
    // §3.3 makes a partial op set normal, so the target may be perfectly valid and
    // simply absent. It is refused anyway: a reply under a parent this peer cannot
    // show is, to the user, a reply that went nowhere.
    let dir = TempDir::new("unknown-parent");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    // A well-formed op id that names nothing in this store.
    let nowhere = OpId::from_hex(&"ab".repeat(32)).unwrap();
    let err = publish::create_reply(&mut store, &key, &stoa, &nowhere, "Reply", &[])
        .expect_err("a reply to an absent op must be refused");
    assert!(
        matches!(err, publish::PublishError::TargetNotFound),
        "got {err:?}"
    );
    assert_eq!(store.len().unwrap(), 0);
}

#[test]
fn a_reply_across_stoas_and_a_vote_on_a_non_post_are_refused_distinguishably() {
    // Two target checks that a single "bad target" error would collapse. Both are
    // reachable only with a store holding the right shapes, which is why they are
    // here rather than in a unit test.
    let dir = TempDir::new("bad-targets");
    let key = an_identity();
    let mut store = dir.store();

    let (first, _g1) = a_stoa(&mut store, &key.public_key(), "First");
    // A second Stoa, joined, so `NotJoined` cannot be the reason for either
    // refusal below — which it would be if the fixture only created one.
    let second_genesis = Genesis {
        creator: key.public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Second".to_string(),
    };
    let second = second_genesis.address().unwrap();
    publish::join_stoa(&mut store, &second, &second_genesis).unwrap();

    let post_in_first = publish::create_post(&mut store, &key, &first, "Here", &[]).unwrap();

    // Replying from the SECOND Stoa to a post in the FIRST.
    let cross = publish::create_reply(
        &mut store,
        &key,
        &second,
        &post_in_first.op.id(),
        "Across",
        &[],
    )
    .expect_err("a cross-Stoa reply must be refused");
    assert!(
        matches!(cross, publish::PublishError::TargetInAnotherStoa),
        "got {cross:?}"
    );

    // A vote on a VOTE. The wire format permits naming any op id; nothing
    // resolves this, so signing it produces an op with no observable effect.
    let vote =
        publish::create_vote(&mut store, &key, &first, &post_in_first.op.id(), VoteDirection::Up)
            .unwrap();
    let on_a_vote =
        publish::create_vote(&mut store, &key, &first, &vote.op.id(), VoteDirection::Up)
            .expect_err("a vote on a vote must be refused");
    assert!(
        matches!(on_a_vote, publish::PublishError::TargetNotAPost),
        "got {on_a_vote:?}"
    );

    // The two messages differ, because they send a caller to different places.
    assert_ne!(cross.to_string(), on_a_vote.to_string());
}

#[test]
fn joining_a_stoa_whose_record_does_not_hash_to_the_address_is_refused() {
    // §4.8's self-authenticating property, which is the security property of the
    // whole join flow. A tampered record must not pass for the Stoa an address
    // names, and the check consults nothing — no registry, no peer.
    let dir = TempDir::new("join-mismatch");
    let key = an_identity();
    let mut store = dir.store();

    let real = Genesis {
        creator: key.public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Agora".to_string(),
    };
    let real_address = real.address().unwrap();

    // The impostor differs in the CREATOR, which is the field that matters: a
    // record naming a different creator would install a different sole moderator
    // (§6). That is the attack, not a typo in a title.
    let impostor = Genesis {
        creator: an_identity().public_key(),
        ..real.clone()
    };
    assert_ne!(
        impostor.address().unwrap(),
        real_address,
        "the fixture's impostor must genuinely differ"
    );

    let err = publish::join_stoa(&mut store, &real_address, &impostor)
        .expect_err("a record that does not hash to the address must be refused");
    assert!(matches!(err, publish::PublishError::Genesis(_)), "got {err:?}");
    assert!(
        store.get_stoa(&real_address).unwrap().is_none(),
        "a refused join must not be recorded"
    );

    // The honest record for the same address is accepted, so the refusal was about
    // the mismatch rather than about joining at all.
    publish::join_stoa(&mut store, &real_address, &real)
        .expect("the record that does hash to the address must be accepted");
    assert!(store.get_stoa(&real_address).unwrap().is_some());
}

// NOTE: there is deliberately no test here that creating a second identity is
// refused, and no test of the keystore on disk at all. Keystore creation is
// separately owned; `keystore.rs`'s own suite covers the refuse-to-overwrite
// property, and duplicating it here would mean this target failed whenever that
// surface moved, for reasons unrelated to the write path.

// ─── The registry's own properties ────────────────────────────────────────

#[test]
fn list_stoas_returns_created_and_joined_and_says_which() {
    // Both relations, in one store, distinguished. A registry that reported one
    // relation for everything would pass a test that only listed created Stoas —
    // and "did I make this" is a question a user asked.
    let dir = TempDir::new("relations");
    let key = an_identity();
    let mut store = dir.store();

    let (mine, _g) = a_stoa(&mut store, &key.public_key(), "Mine");

    let theirs_genesis = Genesis {
        creator: an_identity().public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Theirs".to_string(),
    };
    let theirs = theirs_genesis.address().unwrap();
    publish::join_stoa(&mut store, &theirs, &theirs_genesis).unwrap();

    let all = store.list_stoas().unwrap();
    assert_eq!(all.len(), 2);

    let by_address: std::collections::HashMap<_, _> = all
        .iter()
        .map(|j| (j.address().unwrap(), j.relation))
        .collect();
    assert_eq!(by_address[&mine], Relation::Created);
    assert_eq!(by_address[&theirs], Relation::Joined);
}

#[test]
fn joining_a_stoa_this_peer_created_does_not_downgrade_it_to_joined() {
    // The one case first-wins is actually load-bearing for. The genesis record is
    // identical either way — the address is its hash — so the ONLY thing a second
    // write could corrupt is the relation.
    //
    // A user pasting their own Stoa's address is an ordinary thing to do, and being
    // told afterwards that they merely joined the forum they founded is wrong.
    let dir = TempDir::new("no-downgrade");
    let key = an_identity();
    let mut store = dir.store();

    let (mine, genesis) = a_stoa(&mut store, &key.public_key(), "Mine");
    // Join it — the same Stoa this peer created.
    let after = publish::join_stoa(&mut store, &mine, &genesis).unwrap();

    assert_eq!(
        after.relation,
        Relation::Created,
        "joining a Stoa you created must not rewrite the relation"
    );
    // And through the store, so the assertion is about what was PERSISTED rather
    // than about what `join_stoa` happened to return.
    assert_eq!(
        store.get_stoa(&mine).unwrap().unwrap().relation,
        Relation::Created
    );
    assert_eq!(store.list_stoas().unwrap().len(), 1, "and it is still one Stoa");
}

#[test]
fn list_stoas_is_in_address_order_and_is_stable_across_a_restart() {
    // The order is arbitrary and convergent, which is the honest claim — and
    // "stable" is the half a caller depends on, because pagination over an
    // unstable order skips and repeats rows.
    //
    // The fixture creates Stoas in an order deliberately DIFFERENT from address
    // order (titles are sequential, addresses are hashes), so a registry returning
    // insertion order would fail. A test creating them in address order could not
    // tell the two apart.
    let dir = TempDir::new("stoa-order");
    let key = an_identity();

    let mut expected: Vec<Address> = {
        let mut store = dir.store();
        let mut made = Vec::new();
        for i in 0..8 {
            let (a, _g) = a_stoa(&mut store, &key.public_key(), &format!("Stoa {i}"));
            made.push(a);
        }
        made
    };
    expected.sort();

    let before: Vec<Address> = dir
        .store()
        .list_stoas()
        .unwrap()
        .iter()
        .map(|j| j.address().unwrap())
        .collect();
    assert_eq!(before, expected, "the listing must be in address order");

    // The same, after a restart. A `HashMap`-backed registry would pass the
    // in-process check by accident of one iteration and fail this.
    let after: Vec<Address> = dir
        .store()
        .list_stoas()
        .unwrap()
        .iter()
        .map(|j| j.address().unwrap())
        .collect();
    assert_eq!(after, before, "the order must be stable across a restart");
    assert!(
        expected.windows(2).all(|w| w[0] < w[1]),
        "the fixture must have distinct, ordered addresses"
    );
}

#[test]
fn a_stoa_with_ops_is_not_a_stoa_this_peer_joined() {
    // THE DISTINCTION THE `stoas` TABLE EXISTS FOR, and the mutation it catches is
    // the tempting one: deriving the Stoa list from `iter_stoa` over the op log.
    //
    // A peer receives gossiped ops for Stoas it never joined. Listing those as
    // joined would put forums in a user's sidebar that they never chose — and
    // conversely, a joined-but-quiet Stoa must still appear.
    let dir = TempDir::new("ops-are-not-membership");
    let key = an_identity();
    let mut store = dir.store();

    // A joined Stoa with NO ops.
    let (quiet, _g) = a_stoa(&mut store, &key.public_key(), "Quiet");

    // An op addressed to a Stoa that was never joined, appended directly — which
    // is exactly what the ingest path will do when the transport lands, since
    // `append` deliberately validates nothing.
    let stranger_genesis = Genesis {
        creator: key.public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Stranger".to_string(),
    };
    let stranger = stranger_genesis.address().unwrap();
    let op = dialectica_core::op::Op {
        stoa: stranger,
        author: key.public_key(),
        kind: dialectica_core::op::OpKind::Post {
            thread: None,
            parent: None,
            body: "gossiped in".to_string(),
            attachments: vec![],
        },
    }
    .sign(&key);
    store
        .append(op, dialectica_core::arrival::Arrival::unordered())
        .unwrap();

    let listed: Vec<Address> = store
        .list_stoas()
        .unwrap()
        .iter()
        .map(|j| j.address().unwrap())
        .collect();

    assert_eq!(
        listed,
        vec![quiet],
        "membership is the registry, not the op log: a quiet joined Stoa is \
         listed and a gossiped-in one is not"
    );
    // Both halves stated separately, so a failure says which one broke.
    assert!(
        !listed.contains(&stranger),
        "an op arriving for an unjoined Stoa must not create a membership"
    );
    // And the op really is there, so the test is not passing because the append
    // silently failed.
    assert_eq!(store.iter_stoa(&stranger).unwrap().len(), 1);
}

// ─── The thread read's own properties ─────────────────────────────────────

#[test]
fn a_forged_op_in_the_store_is_not_rendered_in_a_thread() {
    // §3.3: the store holds junk and the reader never trusts it. The write path
    // does not weaken that — this appends a forgery DIRECTLY, as the ingest path
    // will, and the thread read must drop it.
    //
    // The forgery is the one that matters: a valid signature by the wrong key,
    // claiming another author. An unsigned or garbage op would be caught by any
    // check at all; only re-deriving the address from the key catches this.
    let dir = TempDir::new("forgery");
    let victim = an_identity();
    let attacker = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &victim.public_key(), "Agora");

    let root = publish::create_post(&mut store, &victim, &stoa, "Mine", &[]).unwrap();

    // An op claiming the victim as author, signed by the attacker.
    let forged_op = dialectica_core::op::Op {
        stoa,
        author: victim.public_key(),
        kind: dialectica_core::op::OpKind::Post {
            thread: Some(root.op.id()),
            parent: Some(root.op.id()),
            body: "I did not write this".to_string(),
            attachments: vec![],
        },
    };
    let forged = dialectica_core::op::SignedOp {
        signature: dialectica_core::identity::sign_op_bytes(
            &attacker,
            &forged_op.canonical_bytes(),
        ),
        op: forged_op,
    };
    assert!(!forged.verify(), "the fixture must be a genuine forgery");
    store
        .append(forged.clone(), dialectica_core::arrival::Arrival::unordered())
        .unwrap();

    // It IS in the store — the log stores junk deliberately, and a test where the
    // append silently failed would pass the assertion below for the wrong reason.
    assert!(store.get(&forged.op.id()).unwrap().is_some());

    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page =
        publish::read_thread(&store, &moderators, &stoa, &root.op.id(), 0, 20, false).unwrap();

    assert_eq!(
        page.items.len(),
        1,
        "a forged reply must not render, got {:?}",
        page.items.iter().map(|i| &i.body.text).collect::<Vec<_>>()
    );
    assert_eq!(page.items[0].post, root.op.id().to_hex());
}

#[test]
fn a_thread_read_paginates_without_skipping_or_repeating() {
    // Pagination over the thread read, which is a new surface. The property is that
    // the pages CONCATENATE to the whole, which is what catches an off-by-one in
    // either bound — a test checking only page lengths would pass for a
    // `start..start+per_page` that dropped a row between pages.
    let dir = TempDir::new("thread-pages");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let root = publish::create_post(&mut store, &key, &stoa, "Root", &[]).unwrap();
    for i in 0..9 {
        publish::create_reply(&mut store, &key, &stoa, &root.op.id(), &format!("r{i}"), &[])
            .unwrap();
    }

    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let whole =
        publish::read_thread(&store, &moderators, &stoa, &root.op.id(), 0, 100, false).unwrap();
    assert_eq!(whole.items.len(), 10, "the root and nine replies");
    assert!(!whole.has_more);

    // Walk it four at a time and reassemble.
    let mut seen = Vec::new();
    let mut page = 0;
    loop {
        let p =
            publish::read_thread(&store, &moderators, &stoa, &root.op.id(), page, 4, false).unwrap();
        seen.extend(p.items.iter().map(|i| i.post.clone()));
        if !p.has_more {
            break;
        }
        page += 1;
        assert!(page < 10, "pagination must terminate");
    }

    assert_eq!(
        seen,
        whole.items.iter().map(|i| i.post.clone()).collect::<Vec<_>>(),
        "the pages must concatenate to the whole thread, in the same order"
    );

    // A page past the end is an empty page, not a panic and not a wrap-around.
    let past = publish::read_thread(&store, &moderators, &stoa, &root.op.id(), 99, 4, false)
        .unwrap();
    assert!(past.items.is_empty());
    assert!(!past.has_more);
}

#[test]
fn a_thread_read_asked_for_the_wrong_stoas_genesis_is_refused() {
    // Pairing a moderator set with the wrong Stoa would apply one forum's authority
    // to another's posts. Refused rather than served with moderation quietly not
    // applying — which is the failure mode that looks like success.
    let dir = TempDir::new("thread-wrong-genesis");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let root = publish::create_post(&mut store, &key, &stoa, "Root", &[]).unwrap();

    let other = Genesis {
        creator: key.public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Elsewhere".to_string(),
    };

    let request = format!(
        r#"{{"stoa":"{}","thread":"{}"}}"#,
        stoa.to_hex(),
        root.op.id().to_hex()
    );
    // The right Stoa, the WRONG record.
    let out = dialectica_core::get_thread(&request, &store, &other);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("error").is_some(), "got {out}");
    assert!(
        v.get("items").is_none(),
        "a failure must never also carry a result — §2.5: {out}"
    );

    // And the matching record works, so the refusal was about the mismatch.
    let ok = dialectica_core::get_thread(&request, &store, &genesis);
    let v: serde_json::Value = serde_json::from_str(&ok).unwrap();
    assert_eq!(v["items"].as_array().map(|a| a.len()), Some(1), "got {ok}");
}

#[test]
fn the_thread_handler_the_adapter_calls_verifies_the_genesis_against_the_address() {
    // `get_thread_from_request` is the hex-taking form the module adapter forwards
    // to, and it is the ONLY one the adapter reaches — so it is where the
    // self-authenticating check has to hold. The decoded-record form's own test
    // above says nothing about this one.
    //
    // The mutation this catches is the tempting one: decoding the genesis record
    // without checking it hashes to the address. That would let a caller supply any
    // record they liked — installing themselves as the Stoa's moderator — and every
    // other test here passes a MATCHING pair, so none of them would notice.
    let dir = TempDir::new("thread-from-request");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let root = publish::create_post(&mut store, &key, &stoa, "Root", &[]).unwrap();
    publish::create_reply(&mut store, &key, &stoa, &root.op.id(), "Reply", &[]).unwrap();

    let thread_hex = root.op.id().to_hex();

    // The honest pair reads.
    let ok = dialectica_core::wire::get_thread_from_request(
        &format!(
            r#"{{"stoa":"{}","genesis":"{}","thread":"{thread_hex}"}}"#,
            stoa.to_hex(),
            hex::encode(genesis.canonical_bytes().unwrap())
        ),
        &store,
    );
    let v: serde_json::Value = serde_json::from_str(&ok).unwrap();
    assert_eq!(
        v["items"].as_array().map(|a| a.len()),
        Some(2),
        "the root and its reply, got {ok}"
    );

    // A record for a DIFFERENT Stoa, presented with this Stoa's address. It decodes
    // perfectly; it simply is not the record this address names.
    let impostor = Genesis {
        creator: an_identity().public_key(),
        policy: dialectica_core::stoa::Policy::Open,
        title: "Impostor".to_string(),
    };
    let bad = dialectica_core::wire::get_thread_from_request(
        &format!(
            r#"{{"stoa":"{}","genesis":"{}","thread":"{thread_hex}"}}"#,
            stoa.to_hex(),
            hex::encode(impostor.canonical_bytes().unwrap())
        ),
        &store,
    );
    let v: serde_json::Value = serde_json::from_str(&bad).unwrap();
    assert!(
        v.get("error").is_some(),
        "a record that does not hash to the address must be refused, got {bad}"
    );
    assert!(
        v.get("items").is_none(),
        "a failure must never also carry a result — §2.5: {bad}"
    );

    // And a non-object request, so the object guard is reached through this
    // handler too rather than only through the ones the unit tests cover.
    let v: serde_json::Value =
        serde_json::from_str(&dialectica_core::wire::get_thread_from_request("[]", &store))
            .unwrap();
    assert!(v.get("error").is_some());
}

#[test]
fn a_thread_whose_root_has_not_arrived_is_empty_rather_than_an_error() {
    // §3.3 makes a partial op set the normal case, so a thread this peer cannot
    // show yet is an ANSWER. The distinction from obligation 5 is the point: this
    // is a successful read that found nothing, and the corrupt-store tests above
    // are a failed read — the two must not be the same reply.
    let dir = TempDir::new("absent-root");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let nowhere = OpId::from_hex(&"cd".repeat(32)).unwrap();
    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let page = publish::read_thread(&store, &moderators, &stoa, &nowhere, 0, 20, false)
        .expect("an unknown thread is an empty answer, not a failure");
    assert!(page.items.is_empty());
    assert!(!page.has_more);
}

// ─── The publish reply shape ──────────────────────────────────────────────

#[test]
fn a_publish_reply_carries_the_op_id_the_store_now_holds() {
    // The reply shape is `{"op":"<hex>"}`, and the id must be the one a reply can
    // then NAME — otherwise a view has to re-read the feed to find what it posted.
    //
    // The assertion goes through the store rather than comparing the handler's
    // answer to itself: the id is looked up, and then used as a parent. A handler
    // returning a plausible-looking hash of the wrong thing would pass a
    // string-shape check and fail this.
    let dir = TempDir::new("publish-reply");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let out = dialectica_core::create_post(
        &format!(r#"{{"stoa":"{}","body":"First"}}"#, stoa.to_hex()),
        &mut store,
        &key,
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let op_hex = v["op"].as_str().unwrap_or_else(|| panic!("got {out}"));
    let op_id = OpId::from_hex(op_hex).expect("the reported op id must parse");

    assert!(
        store.get(&op_id).unwrap().is_some(),
        "the reported op id must name an op the store holds"
    );
    // And it is usable as a parent, which is the reason it is returned at all.
    publish::create_reply(&mut store, &key, &stoa, &op_id, "Second", &[])
        .expect("the reported op id must be repliable");
}

#[test]
fn a_vote_direction_is_a_name_and_an_unknown_one_is_refused() {
    // Never defaulted. Guessing which direction a caller meant is how a downvote
    // is recorded as an upvote — and a vote is attributed to a person.
    let dir = TempDir::new("vote-direction");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let post = publish::create_post(&mut store, &key, &stoa, "First", &[]).unwrap();
    let target = post.op.id().to_hex();
    let hex = stoa.to_hex();

    for (direction, ok) in [
        (r#""up""#, true),
        (r#""down""#, true),
        // The refusals: a plausible synonym, the wire discriminant, and a
        // wrong-typed value. `0` is the interesting one — it is `Up`'s actual
        // wire byte, so an implementation leaking the discriminant would accept it.
        (r#""UP""#, false),
        (r#""upvote""#, false),
        ("0", false),
        ("true", false),
        ("null", false),
    ] {
        let out = dialectica_core::create_vote(
            &format!(r#"{{"stoa":"{hex}","target":"{target}","direction":{direction}}}"#),
            &mut store,
            &key,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        if ok {
            assert!(
                v.get("op").is_some(),
                "direction {direction} must be accepted, got {out}"
            );
        } else {
            assert!(
                v.get("error").is_some(),
                "direction {direction} must be refused rather than guessed, got {out}"
            );
            assert!(v.get("op").is_none(), "got {out}");
        }
    }

    // A missing direction is its own refusal, distinguishable from a wrong one.
    let missing = dialectica_core::create_vote(
        &format!(r#"{{"stoa":"{hex}","target":"{target}"}}"#),
        &mut store,
        &key,
    );
    let v: serde_json::Value = serde_json::from_str(&missing).unwrap();
    assert!(
        v["error"].as_str().unwrap().contains("missing"),
        "got {missing}"
    );
}

#[test]
fn an_up_vote_and_a_down_vote_are_different_ops() {
    // The direction must reach the stored bytes. A vote whose direction was
    // dropped would store one op for both, and `create_vote` would report
    // "already present" for the second — which is invisible unless the ids are
    // compared.
    let dir = TempDir::new("vote-distinct");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");
    let post = publish::create_post(&mut store, &key, &stoa, "First", &[]).unwrap();

    let up = publish::create_vote(&mut store, &key, &stoa, &post.op.id(), VoteDirection::Up)
        .unwrap();
    let down = publish::create_vote(&mut store, &key, &stoa, &post.op.id(), VoteDirection::Down)
        .unwrap();

    assert_ne!(
        up.op.id(),
        down.op.id(),
        "the direction must participate in the op id"
    );
    // Three ops: the post and two votes. This is what catches a second vote that
    // was silently deduplicated away.
    assert_eq!(store.len().unwrap(), 3);
}

#[test]
fn a_published_op_verifies_under_the_authors_own_address() {
    // The whole point of signing, checked end to end: the op the store holds must
    // verify, and must verify as the author it claims.
    //
    // The address is re-derived from the KEY rather than read from the op, so an
    // implementation that wrote the author field from the wrong place would fail.
    let dir = TempDir::new("signature-end-to-end");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, _genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let post = publish::create_post(&mut store, &key, &stoa, "Signed", &[]).unwrap();
    let stored = store.get(&post.op.id()).unwrap().expect("it is stored");

    assert!(stored.op.verify(), "a published op must verify");
    assert_eq!(
        stored.op.op.author.address(),
        key.public_key().address(),
        "and it must be attributed to the signing key's own address"
    );
    // Byte-identical to what was returned, so the store is not re-encoding.
    assert_eq!(stored.op.to_bytes(), post.to_bytes());

    // And a different key does NOT verify against it, so the assertion above is
    // not passing for any key at all.
    let other = an_identity();
    assert!(
        !dialectica_core::identity::verify_authored_op(
            &other.public_key().address(),
            &other.public_key().to_bytes(),
            &stored.op.op.canonical_bytes(),
            &stored.op.signature.to_bytes(),
        ),
        "another identity must not verify as this op's author"
    );
}

#[test]
fn publishing_the_same_post_twice_is_idempotent_by_op_id() {
    // §3.1's idempotence, reached through the write path. Two identical posts have
    // identical canonical bytes and therefore one op id — so the second is a
    // duplicate, not a second post.
    //
    // This is a real consequence a view must handle rather than a curiosity: a user
    // double-clicking submit produces exactly this, and a feed showing one post is
    // correct.
    let dir = TempDir::new("idempotent");
    let key = an_identity();
    let mut store = dir.store();
    let (stoa, genesis) = a_stoa(&mut store, &key.public_key(), "Agora");

    let first = publish::create_post(&mut store, &key, &stoa, "Same", &[]).unwrap();
    let second = publish::create_post(&mut store, &key, &stoa, "Same", &[]).unwrap();

    assert_eq!(first.op.id(), second.op.id());
    assert_eq!(store.len().unwrap(), 1, "a duplicate must not add a row");

    let moderators = dialectica_core::moderation::Moderators::of(&genesis).unwrap();
    let feed =
        dialectica_core::feed::list_threads(&store, &moderators, &stoa, 0, 20, false).unwrap();
    assert_eq!(feed.items.len(), 1, "and it must render once");
}
