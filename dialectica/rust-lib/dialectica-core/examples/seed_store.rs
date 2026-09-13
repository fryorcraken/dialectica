//! Build a whole dialectica peer state at a given path, through the public API
//! only, so that the UI can be looked at with real content in it.
//!
//! ```text
//! cargo run --example seed_store -- <directory>
//! cargo run --example seed_store -- --fresh <directory>
//! ```
//!
//! # What it produces, and why it prints what it prints
//!
//! The four files the adapter opens, under the names the adapter derives, holding
//! one Stoa and a small forum: two threads, replies at two levels of nesting, and
//! votes, from **two identities** so that author attribution is visible rather than
//! uniform.
//!
//! **It prints the Stoa address and the genesis record, and both are required to
//! use the result.** `listThreads` takes both, because
//! [`dialectica_core::moderation::Moderators::of`] is the only way to build a
//! moderator set and it takes a genesis record. Neither value is recoverable by a
//! person looking at the store:
//!
//! - the **address** is SHA-256 over the record's canonical bytes, so it cannot be
//!   guessed;
//! - the **record cannot be recovered from the address at all**, because that is
//!   the same hash run backwards. Every read needs it, since a moderator set can
//!   only be built from a record.
//!
//! Without both printed, the seeded store is usable by nothing but itself. That is
//! the whole reason this is a program rather than a fixture inside a test.
//!
//! # DO NOT GUESS AN OUTPUT PATH — it is a required argument
//!
//! Basecamp's persistence directory is `module_data/dialectica/<instance-id>`,
//! where the instance id is **host-assigned and unpredictable**. A default baked in
//! here would point at a directory the host never uses, and the failure would be
//! silent: a seeded store nothing ever reads, indistinguishable from a forum nobody
//! has posted in. So the path is a parameter with no default, and the program
//! refuses to run without one.
//!
//! To seed a running instance, find the real directory first — the module logs it
//! at startup (`dialectica ready: instance … (persistence: …)`) — and pass that.
//!
//! # It refuses an existing store rather than replacing one
//!
//! A tool that silently destroys a store somebody was using is worse than one that
//! refuses, and the specific thing at stake here is not the posts: it is
//! `identity.key`, which holds the **root secret in exactly one place**. Replacing
//! that file destroys every identity the user has, including their authorship of
//! every op already published to peers, and no error anywhere says so.
//! [`Keystore::create`](dialectica_core::keystore::Keystore::create) refuses to
//! overwrite for that reason; this refuses one step earlier, over all four files, so
//! that a half-seeded directory is not a state this program can produce.
//!
//! `--fresh` is the explicit opposite. It deletes the four files by name, prints
//! each one as it goes, and is the only path here that removes anything.
//!
//! # The identity is REAL, and that is the substantive change from the original
//!
//! The version of this tool that went out with the closed PR #31 used
//! `SecretKey::generate()` three times and discarded the keys on exit, because
//! there was no keystore to mint from. There is now, and using throwaway keys has
//! become actively wrong rather than merely limited:
//!
//! - **A Stoa's creator is its sole moderator, and the creator is fixed inside the
//!   address preimage forever.** Founding the seeded Stoa with a key the running
//!   module does not hold produces a Stoa its own user can never moderate, and no
//!   later action can repair it — the address would have to change.
//! - **`getCapabilities` would report the user cannot post.** The probe reads the
//!   identity record, so a Stoa seeded without one renders with a disabled compose
//!   box, which looks like a bug in whichever UI piece is being examined.
//!
//! So this mints a keystore when there is none, and signs everything with it. The
//! keystore is written **unencrypted** unless `DIALECTICA_PASSPHRASE` is set, which
//! is [`protection_from_env`](dialectica_core::keystore::protection_from_env)'s own
//! behaviour rather than a policy invented here.
//!
//! # Three derivations exist for one user, and this one follows the module
//!
//! `createStoa` names `identity_public_key` as the creator; the publish path signs
//! with `stoa_key`; `getCapabilities` reports `stoa_address_at_path`. That is three
//! derivations for one user, it is known — `ci.yml` carries a named exemption for
//! it — and it is a spec question this example does not get to decide.
//!
//! What this file does is **match the module at each position**, so that a seeded
//! store behaves exactly as one the module built: creator from `identity_*`, ops
//! signed with `stoa_key`, and a path recorded so the probe has one to read.
//!
//! **`keystore.rs`'s own docstrings disagree with the adapter, and this file
//! follows the adapter.** `Keystore::stoa_key` says it is "built and, in the MVP,
//! not called by any handler"; `Keystore::identity_key` says "there is exactly one
//! derivation position, so a creator and a poster cannot be two keys". The adapter
//! *does* call `stoa_key` (`dialectica/rust-lib/src/lib.rs`), so both sentences are
//! false of the code as it stands. This file copies the adapter because a seeder
//! that agreed with the docstrings instead would build stores the module does not —
//! but do not read it as having invented the divergence. Whoever resolves this
//! should fix those two docstrings in the same change
//! (`findings/security.md` entry 4).
//!
//! **The visible consequence, and the reason it is printed rather than hidden:**
//! every seeded post's `author` in the feed is the *signing* address, and
//! `getCapabilities` reports a *different* one. A UI developer who saw only the
//! second would conclude the feed was attributing their own posts to a stranger.
//! Both are printed, side by side and labelled as the known gap, and the program
//! asserts they still disagree — so the day the spec settles it, this fails loudly
//! and tells whoever fixed it that these paragraphs are now stale.
//!
//! # Public API only, no test-only back doors
//!
//! Every write goes through `authoring::*`, `MembershipStore::join` or a `Keystore`
//! method, and every read through the same functions a view reaches. Nothing here
//! touches a `pub(crate)` item or appends an op directly — if it could, this would
//! not be evidence that the public API is sufficient to build a forum, which is the
//! second thing it is for.
//!
//! # CI does compile this, and no workflow change was needed to make it
//!
//! The #31 original was lost when its PR closed, and the obvious worry is that an
//! example rots silently as the API moves underneath it. **It does not, and that
//! was measured rather than assumed.** A deliberate type error was planted here and
//! both of CI's Rust gates failed on it:
//!
//! - `cargo test --manifest-path … -p dialectica -p dialectica-core` — cargo builds
//!   a package's examples as part of `cargo test`, so the `Tests` step already
//!   compiles this file before it runs a single test;
//! - `cargo clippy … --all-targets -- -D warnings` — `--all-targets` includes
//!   `--examples`, so the `Clippy` step compiles it too, and lints it.
//!
//! So this file cannot stop compiling without turning the Rust job red, and adding
//! a third step to build examples would have been a gate duplicating two that
//! already work. What CI cannot see is whether the program still *does anything
//! useful* — nothing runs it — and that is a real limit rather than one this change
//! closes: the assertions below are what makes a run fail loudly, and a run is a
//! person typing the command.
//!
//! # NEVER ADD A `#[test]` TO THIS FILE
//!
//! CI's test-count gate counts `#[test]` attributes across the whole Rust tree with
//! `examples/` in scope, and cargo does **not** run an example's tests under
//! `cargo test`. One declared here makes `declared` exceed `ran` and fails the job
//! — correctly, because a test that cannot run is worse than no test. Anything
//! worth asserting about this code belongs in `tests/end_to_end.rs`; what this file
//! asserts about its own output, it asserts inline with `assert!`, which runs.
//!
//! # Why this returns `Result<(), String>` rather than `Box<dyn Error>`
//!
//! **Six of the crate's eight error types do not implement `std::error::Error`.**
//! Only `OpLogError` and `MembershipError` do, so the obvious
//! `Box<dyn std::error::Error>` signature does not compile against
//! `KeystoreError`, `GenesisError`, `IdentityStoreError`, `Refusal`,
//! `RandomnessUnavailable` or `OnboardingError`.
//!
//! That is a real gap in the public API and it is deliberately **not fixed here**.
//! Adding six trait impls widens the crate's public surface, which this project
//! treats as a decision to take on purpose rather than as a side effect of one
//! caller wanting `?` — and this piece is a developer tool that changes no
//! behaviour, so smuggling an API widening into it is precisely the shape the
//! change flow exists to catch. Every one of the six implements `Display`, so
//! `.map_err(|e| e.to_string())` costs one call and nothing else.
//!
//! Recorded here rather than left as a puzzle: the next person to want `?` across
//! these types should propose the impls as their own change, with the argument
//! that a public error type a caller cannot box is a public error type a caller
//! cannot compose.

use std::path::{Path, PathBuf};

use dialectica_core::authoring;
use dialectica_core::identity::SecretKey;
use dialectica_core::identity_store::IdentityStore;
use dialectica_core::keystore::{self, Keystore};
use dialectica_core::log::{OpLog, SqliteOpLog};
use dialectica_core::membership::{self, Membership, MembershipStore};
use dialectica_core::moderation::Moderators;
use dialectica_core::op::VoteDirection;
use dialectica_core::stoa::{Genesis, Policy};

/// The derivation path recorded for the seeded Stoa.
///
/// Zero is the first candidate of a slate, which is what a user pressing through
/// onboarding without deliberating would land on. The value matters only in that
/// the seeder and the probe must agree, and they agree because both read this
/// record rather than assuming a number.
const SEEDED_PATH: u32 = 0;

/// Every file this program writes, as the adapter names them.
///
/// One function rather than four `dir.join(...)` calls scattered about, because the
/// names are not this program's to choose: each comes from the `core` function the
/// adapter calls, so a rename upstream moves this with it. A seeder writing to a
/// filename the module does not open would produce a store nothing reads, which is
/// the silent failure the whole preamble is about, one level down.
fn store_files(dir: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("keystore", keystore::default_path_in(dir)),
        ("identity record", IdentityStore::default_path_in(dir)),
        ("membership store", membership::membership_path_in(dir)),
        // The op log's name is the one value here with no `core` accessor: the
        // adapter spells `dir.join("ops.sqlite")` inline. Copied rather than
        // derived, and flagged as such — if that ever moves, this moves by hand.
        ("op log", dir.join("ops.sqlite")),
    ]
}

fn usage() -> String {
    format!(
        "usage: cargo run --example seed_store -- [--fresh] <directory>\n\
         \n\
         The directory is REQUIRED and is not guessed. Basecamp's is\n\
         module_data/dialectica/<instance-id>, where the instance id is\n\
         host-assigned — a default here would seed a store nothing reads, and\n\
         the failure would be silent. The running module logs its real path at\n\
         startup: \"dialectica ready: instance ... (persistence: ...)\".\n\
         \n\
         --fresh deletes the four files this tool writes ({}) before seeding.\n\
         Without it, an existing store is REFUSED and nothing is written.\n\
         \n\
         --fresh DELETES FIRST AND CHECKS AFTERWARDS. The assertions that make a\n\
         bad run fail run at the end, over ops already written, so a run that\n\
         fails one leaves a half-seeded directory and the old store is gone. The\n\
         no---fresh path promises \"nothing was written\"; this one cannot.",
        store_files(Path::new(""))
            .iter()
            .map(|(_, p)| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Any of this crate's error types, as a message, with what was being attempted.
///
/// The context prefix is what makes the one-line failure usable: every store here
/// is a SQLite file and several of them fail with the same `rusqlite` wording, so
/// "unable to open database file" without a prefix names none of the four.
fn why<T, E: std::fmt::Display>(what: &str, r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{what}: {e}"))
}

fn main() -> Result<(), String> {
    // Two arguments at most, and a hand-rolled parse rather than a flag crate: a
    // dependency for one optional flag would be exactly the unexamined widening
    // this crate's dependency posture is about.
    let mut fresh = false;
    let mut dir: Option<String> = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--fresh" => fresh = true,
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option {other}\n\n{}", usage());
                std::process::exit(2);
            }
            other => {
                if dir.is_some() {
                    eprintln!("more than one directory given\n\n{}", usage());
                    std::process::exit(2);
                }
                dir = Some(other.to_string());
            }
        }
    }
    let Some(dir) = dir else {
        eprintln!("{}", usage());
        // A non-zero exit, so a script that forgot the argument fails rather than
        // reporting success over an empty store.
        std::process::exit(2);
    };
    let dir = PathBuf::from(dir);
    why("creating the directory", std::fs::create_dir_all(&dir))?;

    // ── Refuse, or delete on purpose ──────────────────────────────────────
    //
    // Before anything is opened. `SqliteOpLog::open` and friends CREATE their file,
    // so a check made after the first open would be checking a file this program
    // had just written.
    let existing: Vec<_> = store_files(&dir)
        .into_iter()
        .filter(|(_, p)| p.exists())
        .collect();
    if !existing.is_empty() {
        if !fresh {
            eprintln!("refusing to seed {}: it already holds", dir.display());
            for (what, path) in &existing {
                eprintln!("  {what:<16} {}", path.display());
            }
            eprintln!(
                "\nNothing was written. The keystore in particular holds the root secret in\n\
                 exactly one place, so replacing it destroys every identity this peer has —\n\
                 including its authorship of ops other peers already hold.\n\
                 \n\
                 Seed an empty directory, or pass --fresh to delete these and start over."
            );
            std::process::exit(1);
        }
        // BEFORE the first deletion, not during it. The four names are generic
        // enough to collide with unrelated data — `identity.key`, `ops.sqlite` —
        // and a mistyped path would otherwise destroy four arbitrary files. Naming
        // each as it goes is honest but after the fact; this refuses first.
        //
        // Only `identity.key` is checked, and only it is worth checking: it is the
        // one file here whose loss is UNRECOVERABLE, because the root secret exists
        // in exactly one place. The three SQLite stores are rebuildable content.
        //
        // `Keystore::is_encrypted` is the check rather than a magic-byte comparison
        // written here, and that is the difference between reusing a parser and
        // copying a constant. `MAGIC` is private to `keystore.rs`; spelling `0xD4`
        // in this file would be a second copy of a format constant that no test
        // covers, which is the defect family this repo records. `is_encrypted`
        // parses the real header and answers `NotAKeystore` for anything else.
        let keystore_path = keystore::default_path_in(&dir);
        if keystore_path.exists() {
            why(
                "refusing --fresh: identity.key is not a keystore, so this is \
                 probably not a dialectica store and nothing was deleted",
                Keystore::is_encrypted(&keystore_path).map(|_| ()),
            )?;
        }

        for (what, path) in &existing {
            // Named one at a time as they go, so a person who passed --fresh by
            // mistake can see in the scrollback exactly what they lost.
            println!("--fresh: deleting {what} {}", path.display());
            why(&format!("deleting the {what}"), std::fs::remove_file(path))?;
        }
        println!();
    }

    // ── The identity ──────────────────────────────────────────────────────
    //
    // Minted here because there is nothing to open: the refusal above guarantees no
    // keystore exists at this point. `protection_from_env` rather than a hardcoded
    // `Unlock::Unencrypted`, so that a passphrase in the environment encrypts the
    // file exactly as `keepIdentity` would — a seeder that wrote plaintext under a
    // set passphrase would produce a keystore the module then refuses to open.
    let keystore_path = keystore::default_path_in(&dir);
    let keystore = why("minting a root secret", Keystore::generate())?;
    let protection = keystore::protection_from_env();
    why(
        "writing the keystore",
        keystore.create(&keystore_path, &protection),
    )?;

    // ── Read the keystore straight back, and fail on what the module would ──
    //
    // **Writing a keystore is not the same as writing one the module will open**,
    // and the gap between those is silent. `create` checks only that the file does
    // not exist; the directory-permission guard lives on the READ path, in
    // `read_checked`. So `create_dir_all` under a permissive umask, or a directory
    // the user made `chmod 777` earlier, produces a real root secret in a place any
    // local user can replace it — with every assertion here passing and the full
    // report printing, while the module refuses the result with
    // "the keystore's directory is writable by others (mode 0777)".
    //
    // Measured, not supposed: seeding a `mkdir -m 777` directory succeeded and
    // printed a full report, and `keystore::open_in` on the result then refused it
    // (`findings/security.md` entry 1).
    //
    // This ASKS THE KEYSTORE rather than re-implementing the check. A mode test
    // written here would be a second copy of a guard that already exists and is
    // already tested, and the two would eventually disagree about which bits matter
    // — with this copy being the one no test covers. `open_in` is the same call the
    // adapter makes, so what it accepts is exactly what the module accepts.
    why("the keystore this wrote is not one the module can open", {
        keystore::open_in(&dir).map(|_| ())
    })?;

    // The creator, as `createStoa` names it. `identity_public_key` and not a
    // per-Stoa derivation: a Stoa's creator is its sole moderator and the creator is
    // inside the address preimage forever, so this must be the key the module would
    // have used or the seeded Stoa is one its own user cannot moderate.
    let creator = keystore.identity_public_key();

    // ── The Stoa ──────────────────────────────────────────────────────────
    let genesis = Genesis {
        creator,
        // The only variant `stoa.rs` defines, and the only one `createStoa` can
        // produce.
        policy: Policy::Open,
        title: "Ἀγορά".to_string(),
    };
    let address = why("deriving the Stoa address", genesis.address())?;
    let genesis_hex = hex::encode(why(
        "encoding the genesis record",
        genesis.canonical_bytes(),
    )?);

    // Recorded in the membership store through `Membership::verified`, which is the
    // only constructor `join` accepts — the same write `createStoa` and a pasted
    // address both go through. Not bypassed even though the pair cannot fail here
    // (the address came from this very record), because a constructor a caller may
    // skip is not an invariant.
    let mut memberships = why(
        "opening the membership store",
        MembershipStore::open(&membership::membership_path_in(&dir)),
    )?;
    let membership = why(
        "verifying the record against its address",
        Membership::verified(&address, &genesis),
    )?;
    why("recording the membership", memberships.join(&membership))?;

    // The chosen path, so `getCapabilities` reports an identity for this Stoa rather
    // than "no identity has been chosen". Without this row the probe answers
    // `CannotPost` and every UI piece renders a disabled compose box, which reads as
    // a bug in the piece rather than as an unseeded record.
    let paths = why(
        "opening the identity record",
        IdentityStore::open(&IdentityStore::default_path_in(&dir)),
    )?;
    why(
        "recording the chosen path",
        paths.record_path(&address, SEEDED_PATH),
    )?;

    // ── The authors ───────────────────────────────────────────────────────
    //
    // The founder signs with `stoa_key`, which is what the module's publish path
    // signs with today. The second author is a generated key with no keystore behind
    // it, and that asymmetry is deliberate: a peer holds exactly one root secret, so
    // a SECOND local identity is not a state the module can be in. What a real store
    // holds is one identity of its own plus ops that arrived from other peers — and
    // an op from another peer is, as far as the log is concerned, an op signed by a
    // key this peer does not hold. That is what this reproduces.
    let founder = keystore.stoa_key(&address);
    let visitor = why("minting the visitor's key", SecretKey::generate())?;

    let mut log = why(
        "opening the op log",
        SqliteOpLog::open(&dir.join("ops.sqlite")),
    )?;

    // ── A forum with real structure ───────────────────────────────────────
    //
    // Two roots rather than one, so a feed has more than a single row and a small
    // `perPage` has something to page.
    let first_root = why(
        "publishing the first root",
        authoring::post(
            &mut log,
            &founder,
            address,
            "What does it mean for a forum to be decentralized?".to_string(),
        ),
    )?;
    // **The second root is the VISITOR's, and that is what makes the two-identity
    // claim visible.** Both roots were the founder's, so both feed rows carried one
    // author — which made the author assertion below index `items[0]` as though the
    // choice mattered when `items[1]` asserted the identical thing, and left the
    // visitor signing only replies and votes, neither of which `list_threads`
    // returns. So a docstring promising "two identities so that author attribution
    // is visible" described output where it was not visible at all
    // (`findings/correctness.md` entry 3). One root each fixes both.
    let second_root = why(
        "publishing the second root",
        authoring::post(
            &mut log,
            &visitor,
            address,
            "On the difference between moderation and censorship".to_string(),
        ),
    )?;

    // A reply to the first root, then a reply to THAT reply — two levels, so a
    // thread read has a tree to reconstruct rather than a flat list. With only one
    // level, a correct `thread` field and a `thread: parent` bug give the same
    // answer, so a seeded store that stopped at one level could not show the
    // difference on screen either.
    let reply = why(
        "replying to the first root",
        authoring::reply(
            &mut log,
            &visitor,
            address,
            first_root.id,
            "That it has no single party who can switch it off.".to_string(),
        ),
    )?;
    let nested = why(
        "replying to that reply",
        authoring::reply(
            &mut log,
            &founder,
            address,
            reply.id,
            "Agreed — though that is a floor rather than the whole of it.".to_string(),
        ),
    )?;
    let other_reply = why(
        "replying to the second root",
        authoring::reply(
            &mut log,
            &visitor,
            address,
            second_root.id,
            "One is a Stoa deciding what it is; the other is deciding for everyone else."
                .to_string(),
        ),
    )?;

    // ── Votes ─────────────────────────────────────────────────────────────
    //
    // NOTHING READS THESE YET — `feed.rs` says in as many words that no ordering in
    // the current contract reads a `Vote` op — so they change nothing a reader can
    // see, and a UI showing a score off the back of a seeded store would be showing
    // a number it invented. They are here because the history is what a scorer will
    // eventually fold over, and because a store with no vote ops cannot exercise the
    // "a vote is not a post" filter that the feed and thread reads both apply.
    //
    // Both directions, from both identities, so a later tally has something to
    // partition. A table and a loop rather than four near-identical calls: the four
    // differ only in three values, and written out longhand the fourth is where a
    // wrong target or a copied direction hides.
    for (voter, target, direction) in [
        (&visitor, first_root.id, VoteDirection::Up),
        (&founder, first_root.id, VoteDirection::Up),
        (&visitor, second_root.id, VoteDirection::Down),
        (&founder, reply.id, VoteDirection::Up),
    ] {
        why(
            "publishing a vote",
            authoring::vote(&mut log, voter, address, target, direction),
        )?;
    }

    // ── Read it back before claiming anything ─────────────────────────────
    //
    // Through the same public functions a view reaches. A seeder that printed an
    // address without checking the store serves it would be the one failure mode
    // that makes the output worse than nothing: a caller would trust a store that
    // does not work, and would go looking for the bug in the UI.
    let moderators = why("building the moderator set", Moderators::of(&genesis))?;
    let feed = why(
        "reading the feed back",
        dialectica_core::feed::list_threads(&log, &moderators, &address, 0, 100, false),
    )?;
    let ops = why("counting the ops", log.len())?;
    let stoas = why("counting the memberships", memberships.len())?;

    // Asserted rather than merely printed, so a broken seeder exits non-zero instead
    // of emitting plausible output. Both numbers are hardcoded from the writes above
    // rather than read back from the same call being checked.
    assert_eq!(
        feed.items.len(),
        2,
        "the seeded feed must hold two thread heads, not {}",
        feed.items.len()
    );
    assert_eq!(
        ops, 9,
        "two roots, three replies and four votes is nine ops, not {ops}"
    );

    // ── The nesting, which the two counts above cannot see ────────────────
    //
    // `list_threads` returns thread HEADS, so no assertion over it observes the
    // reply structure at all: a store where `nested` hung off the wrong parent, or
    // where all three replies hung off `first_root`, satisfies both counts
    // (`findings/correctness.md` entry 2). The indented tree in the report is
    // presentation, not a check — and the nesting is the one thing this tool exists
    // to produce, because a UI cannot render a tree that is not there.
    //
    // **Read back through `OpLog::get`, not from the `Published` values above.** The
    // publish calls returned ids; asking the store what it actually holds under each
    // is what makes this a check on the store rather than on local variables.
    //
    // NOT through a thread read, because `dialectica-core` has none: `feed.rs`
    // exposes `list_threads` and nothing else, and the thread read is `piece/thread-read`,
    // still in flight. When it lands, this is the assertion to move onto it.
    for (what, id, expected_parent, expected_thread) in [
        ("reply", reply.id, first_root.id, first_root.id),
        // The one that discriminates. At two levels "the parent's id" and "the
        // parent's thread" are the same value, so a `thread: parent` bug is
        // invisible; three levels separate them — `nested`'s parent is `reply`, and
        // its thread is the ROOT's.
        ("nested", nested.id, reply.id, first_root.id),
        (
            "other_reply",
            other_reply.id,
            second_root.id,
            second_root.id,
        ),
    ] {
        let entry = why(&format!("reading {what} back"), log.get(&id))?
            .ok_or_else(|| format!("reading {what} back: the store does not hold it"))?;
        match &entry.op.op.kind {
            dialectica_core::op::OpKind::Post { parent, thread, .. } => {
                assert_eq!(
                    *parent,
                    Some(expected_parent),
                    "{what} must name its parent"
                );
                assert_eq!(
                    *thread,
                    Some(expected_thread),
                    "{what} must belong to the root's thread, not its parent's id"
                );
            }
            other => panic!("{what} must be a post, got {other:?}"),
        }
    }
    // ── The moderation gap, asserted as it actually is ────────────────────
    //
    // This used to be `assert!(moderators.contains(&genesis.creator))`, with a
    // comment claiming it caught the derivation trap head-on. **It caught nothing.**
    // `Moderators::of` sets `creator: genesis.creator.clone()` and `contains` is
    // `&self.creator == key`, so the assertion reduced to
    // `genesis.creator == genesis.creator` and held for any value in that field —
    // this repo's recorded "asks the implementation what it did and agrees" defect,
    // in a file whose whole job is to be evidence (`findings/correctness.md` entry 1).
    //
    // What the assertion claimed to rule out is TRUE TODAY, and asserting the real
    // property is what makes it visible. `Moderators::authorises` gates on
    // `entry.op.op.author` — the SIGNING author — and the adapter signs every publish
    // with `keystore.stoa_key(&stoa)` while the record names `identity_public_key`.
    // Those are two keys, so:
    assert!(
        moderators.contains(&genesis.creator),
        "the record's creator must moderate its own Stoa"
    );
    assert!(
        !moderators.contains(&founder.public_key()),
        "the signing key has BECOME a moderator — the three-derivations gap is \
         closed, which is good news. Delete this assertion, the `moderation` line \
         in the report, and the paragraph in design.md that documents the gap."
    );
    // **So a hide published through the module against a seeded Stoa is REFUSED.**
    // That is the outcome `design.md` says the real keystore was adopted to prevent,
    // and adopting it did not prevent this half: the creator is a key the peer holds,
    // but not the key it signs with. Measured, not inferred — pointing the first
    // assertion at `founder.public_key()` fails, which was run.
    //
    // The tool cannot fix it. WHICH key a publish signs with is the spec question
    // `ci.yml` carries a named exemption for, and closing it here would mean the
    // seeder disagreeing with the module — which is the one thing that would make a
    // seeded store stop being evidence of anything. So it is asserted, reported and
    // left, in the shape that cannot rot: the day it is fixed, this fails and says
    // what to delete.

    // The probe's half: the address `getCapabilities` will report for this Stoa must
    // be one the store has a path for. Read back from the record rather than reusing
    // the constant, so a path that cannot be read fails here instead of on screen.
    let recorded =
        why("reading the chosen path back", paths.path_for(&address))?.ok_or_else(|| {
            "reading the chosen path back: the path just recorded is absent".to_string()
        })?;
    let posting_address = keystore.stoa_address_at_path(&address, recorded);
    let signing_address = keystore.stoa_public_key(&address).address();

    // The feed's `author` is the SIGNING address, not the one the probe reports.
    // Asserted rather than assumed, and asserted against a value this program did
    // not compute for the occasion: the row comes back out of the store.
    //
    // This holds the known three-derivations gap in place, deliberately. If it ever
    // fails, the gap has been CLOSED — that is good news — and the thing to do is
    // delete this assertion along with the paragraph in the report below, not to
    // adjust the comparison until it passes again.
    // Over BOTH rows, by looking each up rather than indexing. `items[0]` used to
    // carry this alone, and while the two roots shared an author that was an
    // assertion whose index could not matter — `items[1]` said the same thing. Now
    // the roots have one author each, so this checks two different values and the
    // feed's ordering is not baked in either.
    let authors: Vec<&str> = feed.items.iter().map(|r| r.author.as_str()).collect();
    assert!(
        authors.contains(&signing_address.to_hex().as_str()),
        "the founder's root must be attributed to the key it was signed with; got {authors:?}"
    );
    assert!(
        authors.contains(&visitor.public_key().address().to_hex().as_str()),
        "the visitor's root must be attributed to the visitor; got {authors:?}"
    );
    assert_ne!(
        posting_address, signing_address,
        "the probe and the publish path have stopped disagreeing — the \
         three-derivations gap is closed, so this assertion and the paragraph it \
         documents should both go"
    );

    // ── Report ────────────────────────────────────────────────────────────
    println!("seeded {}", dir.display());
    println!();
    // THE TWO VALUES A CALLER CANNOT DERIVE. Printed first and labelled, because
    // they are the reason this program prints anything at all.
    println!("stoa     {}", address.to_hex());
    println!("genesis  {genesis_hex}");
    println!("title    {}", genesis.title);
    println!();
    println!("threads    {}", feed.items.len());
    println!("ops        {ops}");
    println!("stoas      {stoas}");
    // WHICH of the two protections was written, because the default writes a real
    // Ed25519 root secret in the clear and the run otherwise says nothing about it.
    // The default is correct — it is `protection_from_env`'s own contract, inherited
    // rather than reinvented — but `keepIdentity` reports protection precisely so
    // that plaintext is a state a caller can NAME rather than a silent default, and
    // a developer seeding a laptop should be able to tell from the scrollback.
    //
    // Matched on the value actually used to write the file, not re-read from the
    // environment: a second `var_os` here could disagree with the first.
    println!(
        "protection {}",
        match protection {
            keystore::Unlock::Unencrypted =>
                "UNENCRYPTED — the root secret is in the clear \
                 (set DIALECTICA_PASSPHRASE to encrypt)",
            keystore::Unlock::Passphrase(_) => "encrypted with DIALECTICA_PASSPHRASE",
        }
    );
    println!();
    // BOTH author addresses, because they DISAGREE and a reader who saw only one
    // would spend an afternoon on it. `getCapabilities` reports the first, and every
    // seeded op is authored by the second, because the module derives a posting
    // identity at one position and signs at another — the three-derivations gap
    // `ci.yml` carries a named exemption for. Nothing here can close it: which key a
    // publish signs with is a spec question.
    //
    // Verified by running it, not inferred: the feed read below reports the signing
    // address as each row's `author`, and it is not the probe's.
    println!("author addresses, which do not agree — this is the known gap:");
    println!("  getCapabilities reports  {}", posting_address.to_hex());
    println!("  every seeded op is by    {}", signing_address.to_hex());
    println!(
        "  record names as creator  {}",
        genesis.creator.address().to_hex()
    );
    // THE CONSEQUENCE, because the three addresses above are only interesting for
    // what they cause. `Moderators::authorises` gates on the signing author, the
    // record names a different key, so no moderation this peer publishes binds.
    // A UI developer whose hide button does nothing needs to read this line rather
    // than debug their own screen.
    println!(
        "  => MODERATION DOES NOT WORK on a seeded Stoa: a hide published through\n\
         \x20    the module is refused, because the signing key is not the creator\n\
         \x20    the record names. Not a defect of this tool; see design.md."
    );
    println!();
    println!("thread  {}", first_root.id.to_hex());
    println!("  reply  {}", reply.id.to_hex());
    println!("  nested {}", nested.id.to_hex());
    println!("thread  {}", second_root.id.to_hex());
    println!("  reply  {}", other_reply.id.to_hex());
    println!();
    // The request a caller can paste, since assembling it from the two values above
    // is the first thing anybody will do. The field names are `listThreads`'s own.
    println!("a listThreads request for this store:");
    println!(
        r#"  {{"stoa":"{}","genesis":"{genesis_hex}"}}"#,
        address.to_hex()
    );
    println!();
    // And the three QML properties, spelled as `Main.qml` declares them, because
    // pasting them there is the actual next step for the person running this.
    println!("Main.qml properties:");
    println!(r#"  stoaAddress: "{}""#, address.to_hex());
    println!(r#"  stoaTitle:   "{}""#, genesis.title);
    println!(r#"  stoaGenesis: "{genesis_hex}""#);

    Ok(())
}
