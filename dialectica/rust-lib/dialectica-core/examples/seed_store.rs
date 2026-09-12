//! Build a dialectica store at a given path, through the public API only.
//!
//! ```text
//! cargo run --example seed_store -- <directory>
//! ```
//!
//! # What it produces, and why it prints what it prints
//!
//! A SQLite store holding one Stoa's genesis record and a small thread: several
//! posts, replies at two levels of nesting, and votes, from **three different
//! identities** so that author attribution is visible rather than uniform.
//!
//! **It prints the Stoa address and the genesis record, and both are required to
//! use the result.** `list_threads` re-derives the address from the genesis record
//! it is handed and refuses a mismatch, so a caller cannot invent either one:
//!
//! - the **address** is a one-way hash, so it cannot be recovered from the store by
//!   guessing;
//! - the **genesis record** cannot be recovered from the address at all, and every
//!   read needs it, because a moderator set can only be built from a record.
//!
//! Without both printed, the seeded store is usable by nothing but itself. That is
//! the whole reason this example exists rather than a fixture inside a test.
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
//! To seed a running instance, find the real directory first — the module logs it at
//! startup (`dialectica ready: instance … (persistence: …)`) — and pass that.
//!
//! # Identities are generated, not loaded
//!
//! `SecretKey::generate()`, three times. This example deliberately does **not**
//! create or read a keystore: identity creation is separately owned, and a seeder
//! that depended on it could not run until that landed. The consequence is honest
//! and worth stating — **the keys are discarded when this exits**, so the seeded
//! posts cannot be replied to *as their original authors* afterwards. The store is
//! for reading, which is what a seeded store is for.
//!
//! # Public API only, no test-only back doors
//!
//! Every write goes through `publish::*` and every read through the same functions a
//! view calls. Nothing here reaches for a `pub(crate)` item or appends an op
//! directly — if it could, the example would not be evidence that the public API is
//! sufficient to build a forum, which is the second thing it is for.

use dialectica_core::identity::SecretKey;
use dialectica_core::log::{OpLog, SqliteOpLog, StoaRegistry};
use dialectica_core::op::VoteDirection;
use dialectica_core::publish;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The path is required. `nth(1)` rather than a flag parser because there is one
    // argument and a dependency for that would be the unexamined widening §2.3 is
    // about.
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!(
            "usage: cargo run --example seed_store -- <directory>\n\
             \n\
             The directory is REQUIRED and is not guessed. Basecamp's is\n\
             module_data/dialectica/<instance-id>, where the instance id is\n\
             host-assigned — a default here would seed a store nothing reads, and\n\
             the failure would be silent. The running module logs its real path at\n\
             startup: \"dialectica ready: instance ... (persistence: ...)\"."
        );
        // A non-zero exit, so a script that forgot the argument fails rather than
        // reporting success over an empty store.
        std::process::exit(2);
    };

    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir)?;
    // The same filename the module's adapter opens. A seeder writing to a different
    // name would produce a store the module never reads — the silent failure this
    // whole preamble is about, one level down.
    let path = dir.join("ops.sqlite");
    let mut store = SqliteOpLog::open(&path)?;

    // Three identities. Named for what they do in the thread rather than by letter,
    // so the printed output reads as a conversation.
    let founder = SecretKey::generate();
    let replier = SecretKey::generate();
    let voter = SecretKey::generate();

    // ── The Stoa ──────────────────────────────────────────────────────────
    //
    // Created by the founder, which makes them its sole moderator (§6) — the
    // creator key is inside the address preimage, so that follows from the record
    // rather than from anything stored separately.
    let stoa = publish::create_stoa(&mut store, &founder.public_key(), "Ἀγορά")?;
    let address = stoa.address()?;
    let genesis_hex = hex::encode(stoa.genesis.canonical_bytes()?);

    // ── A thread with real structure ──────────────────────────────────────
    //
    // Two roots rather than one, so a feed has more than a single row and
    // `hasMore` can be exercised at small page sizes.
    let first_root = publish::create_post(
        &mut store,
        &founder,
        &address,
        "What does it mean for a forum to be decentralized?",
        &[],
    )?;
    let second_root = publish::create_post(
        &mut store,
        &founder,
        &address,
        "On the difference between moderation and censorship",
        &[],
    )?;

    // A reply to the first root, then a reply to THAT reply — two levels, so a
    // thread read has a tree to reconstruct rather than a flat list. A seeder with
    // only one level could not distinguish a correct `thread` field from
    // `thread: parent`.
    let reply = publish::create_reply(
        &mut store,
        &replier,
        &address,
        &first_root.op.id(),
        "That it has no single party who can switch it off.",
        &[],
    )?;
    let nested = publish::create_reply(
        &mut store,
        &founder,
        &address,
        &reply.op.id(),
        "Agreed — though that is a floor rather than the whole of it.",
        &[],
    )?;

    // A reply carrying an attachment, so the attachments field is non-empty
    // somewhere. §4.6 makes a CID's resolvability a fetch outcome rather than a
    // validation question, so a placeholder is the honest thing here: nothing in
    // this store claims it resolves.
    let with_attachment = publish::create_reply(
        &mut store,
        &replier,
        &address,
        &second_root.op.id(),
        "There is a diagram that makes this clearer.",
        &["placeholder-cid-not-resolvable".to_string()],
    )?;

    // ── Votes ─────────────────────────────────────────────────────────────
    //
    // NOTHING READS THESE YET (§7.2 rule 2 ships no score), so they change nothing
    // a reader can see. They are here because the history is what scoring will
    // fold over, and because a store with no vote ops could not exercise the
    // "a vote is not a post" filter that the thread and feed reads both apply.
    //
    // Both directions, and from two identities, so a later tally has something to
    // partition.
    publish::create_vote(
        &mut store,
        &voter,
        &address,
        &first_root.op.id(),
        VoteDirection::Up,
    )?;
    publish::create_vote(
        &mut store,
        &replier,
        &address,
        &first_root.op.id(),
        VoteDirection::Up,
    )?;
    publish::create_vote(
        &mut store,
        &voter,
        &address,
        &second_root.op.id(),
        VoteDirection::Down,
    )?;
    publish::create_vote(&mut store, &voter, &address, &reply.op.id(), VoteDirection::Up)?;

    // ── Report, and verify what is reported ───────────────────────────────
    //
    // Read the thread back through the public API before printing. A seeder that
    // printed an address without checking the store serves it would be the one
    // failure mode that makes the output worse than nothing: a caller would trust
    // a store that does not work.
    let moderators = dialectica_core::moderation::Moderators::of(&stoa.genesis)?;
    let thread = publish::read_thread(
        &store,
        &moderators,
        &address,
        &first_root.op.id(),
        0,
        100,
        false,
    )?;
    let feed = dialectica_core::feed::list_threads(&store, &moderators, &address, 0, 100, false)?;

    // Asserted rather than merely printed, so a broken seeder exits non-zero
    // instead of emitting plausible output. Three posts in the first thread: the
    // root, the reply, the nested reply.
    assert_eq!(
        thread.items.len(),
        3,
        "the seeded thread must hold its root and both replies"
    );
    assert_eq!(feed.items.len(), 2, "the seeded feed must hold two thread heads");

    println!("seeded {}", path.display());
    println!();
    // THE TWO VALUES A CALLER CANNOT DERIVE. Printed first and labelled, because
    // they are the reason this program prints anything at all.
    println!("stoa    {}", address.to_hex());
    println!("genesis {genesis_hex}");
    println!();
    println!("threads {}", feed.items.len());
    println!("ops     {}", store.len()?);
    println!("stoas   {}", store.list_stoas()?.len());
    println!();
    println!("thread  {}", first_root.op.id().to_hex());
    println!("  reply  {}", reply.op.id().to_hex());
    println!("  nested {}", nested.op.id().to_hex());
    println!("thread  {}", second_root.op.id().to_hex());
    println!("  reply  {}", with_attachment.op.id().to_hex());
    println!();
    // The request a caller can paste, since assembling it from the two values above
    // is the first thing anybody will do.
    println!("a listThreads request for this store:");
    println!(
        r#"  {{"stoa":"{}","genesis":"{genesis_hex}"}}"#,
        address.to_hex()
    );

    Ok(())
}
