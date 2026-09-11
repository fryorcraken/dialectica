//! The behavioural contract, asserted of **every** implementation of [`OpLog`].
//!
//! # Why this file exists
//!
//! A trait with two implementors is worth having only if the second is a
//! drop-in, and prose cannot establish that. Each behaviour here is one generic
//! function over `L: OpLog`, called from two `#[test]`s — one handing it a
//! [`MemoryOpLog`], one a [`SqliteOpLog`]. A behaviour that holds for one and not
//! the other is a failing test naming which.
//!
//! # Why plain functions and not a macro
//!
//! A `macro_rules!` generating the `#[test]`s would be shorter and is wrong
//! here, for a reason about CI rather than taste. `ci.yml`'s test-count step
//! counts `^\s*#\[test\]\s*$` textually across the sources and fails the build
//! when cargo's count differs:
//!
//! > cargo ran {ran} tests but the source declares {declared}. A test that is
//! > compiled out (a cfg, a removed mod) does not fail — it silently stops being
//! > checked.
//!
//! One `#[test]` inside a macro invoked twice declares one matching line and
//! runs two tests, so the gate would fail — correctly by its own logic, for a
//! reason having nothing to do with a compiled-out test. That is this project's
//! "a gate can fail for a real reason while naming the wrong cause" family, and
//! writing the pairs out avoids it entirely. Two `#[test]` lines, two tests.
//!
//! It also reads better at a failure: the test name says which implementation
//! broke.
//!
//! # What is NOT here
//!
//! Behaviours no in-memory log can exhibit: persistence across a reopen, a
//! layout version, a storage failure. Those live in `sqlite.rs`'s own tests and
//! the spec states them as applying to "an implementation that persists".

use super::fixtures::*;
use super::{Appended, Entry, MemoryOpLog, OpLog, SqliteOpLog};
use crate::arrival::Arrival;
use crate::identity::sign_op_bytes;
use crate::op::{ModerationAction, Op, OpId, OpKind, SignedOp};

/// A fresh SQLite log with no file behind it.
///
/// Not a double: the same schema, the same `ORDER BY`, the same decode path,
/// with SQLite's `:memory:` backing store. The point of the contract suite is to
/// exercise the real SQL, and the point of `:memory:` is that it does so without
/// a temporary directory per test.
fn sqlite() -> SqliteOpLog {
    SqliteOpLog::in_memory().expect("an in-memory SQLite log always opens")
}

fn memory() -> MemoryOpLog {
    MemoryOpLog::new()
}

/// Collect a read's op ids, so assertions compare sequences rather than entries.
fn ids(entries: &[Entry]) -> Vec<OpId> {
    entries.iter().map(|e| e.id()).collect()
}

// ─── The store decides nothing ────────────────────────────────────────────

fn an_op_with_an_invalid_signature_is_stored_anyway<L: OpLog>(log: &mut L) {
    // §3.3: "The store may hold junk; the reader never trusts it." A log that
    // filtered here would make a forgery attempt indistinguishable from an op
    // that never arrived — and those call for different responses.
    //
    // Asserted of BOTH implementations because persistence is the obvious
    // moment to be tempted into "surely we should not write garbage to disk",
    // and the temptation is the defect.
    let victim = a_key(2);
    let attacker = a_key(3);
    let op = Op {
        author: victim.public_key(),
        ..a_post("forged")
    };
    let forged = SignedOp {
        signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
        op,
    };
    // The fixture must genuinely be a forgery, or this test proves nothing.
    assert!(!forged.verify(), "the fixture must be an actual forgery");

    let id = forged.op.id();
    assert_eq!(
        log.append(forged.clone(), Arrival::unordered()).unwrap(),
        Appended::Stored
    );
    assert_eq!(log.get(&id).unwrap().map(|e| e.op), Some(forged));
    assert_eq!(log.iter().unwrap().len(), 1);
}

#[test]
fn an_op_with_an_invalid_signature_is_stored_anyway_in_memory() {
    an_op_with_an_invalid_signature_is_stored_anyway(&mut memory());
}

#[test]
fn an_op_with_an_invalid_signature_is_stored_anyway_in_sqlite() {
    an_op_with_an_invalid_signature_is_stored_anyway(&mut sqlite());
}

fn a_moderation_op_from_a_peer_with_no_authority_is_stored<L: OpLog>(log: &mut L) {
    // Authenticity is not authority (§6, and `op.rs`'s two tests pinning the
    // distinction). The log holds a moderation op from a random peer exactly
    // as it holds one from a moderator, because deciding between them needs
    // the moderator set at that op's Lamport time — which the log does not
    // have and the resolver does.
    let random_peer = a_key(9);
    let op = Op {
        stoa: a_stoa("Agora"),
        author: random_peer.public_key(),
        kind: OpKind::Moderate {
            target: signed(a_post("victim")).op.id(),
            action: ModerationAction::Hide,
        },
    };
    let hide = op.sign(&random_peer);
    // Authentic — it really is from them — and carries no authority.
    assert!(hide.verify());

    let id = hide.op.id();
    log.append(hide, Arrival::unordered()).unwrap();
    assert!(
        log.get(&id).unwrap().is_some(),
        "an unauthorised moderation is stored"
    );
}

#[test]
fn a_moderation_op_from_a_peer_with_no_authority_is_stored_in_memory() {
    a_moderation_op_from_a_peer_with_no_authority_is_stored(&mut memory());
}

#[test]
fn a_moderation_op_from_a_peer_with_no_authority_is_stored_in_sqlite() {
    a_moderation_op_from_a_peer_with_no_authority_is_stored(&mut sqlite());
}

fn a_stored_op_reads_back_byte_identical<L: OpLog>(log: &mut L) {
    // The op is signed, so any mutation by the store would break the
    // signature — asserted directly rather than trusted.
    //
    // This is the assertion that holds `sqlite.rs` to storing `to_bytes()`
    // verbatim rather than decomposing the op into columns and re-encoding on
    // read. A re-encode would pass every field-level assertion and could still
    // produce different bytes, which is a signature that no longer verifies on
    // a peer that received the original.
    let op = signed(a_post("exact"));
    let id = op.op.id();
    log.append(op.clone(), Arrival::ordered(5, a_message_id(3)))
        .unwrap();

    let entry = log.get(&id).unwrap().unwrap();
    assert_eq!(entry.op, op);
    assert_eq!(entry.op.to_bytes(), op.to_bytes());
    assert_eq!(entry.id(), id);
    assert!(entry.op.verify(), "storage must not disturb the signature");
}

#[test]
fn a_stored_op_reads_back_byte_identical_in_memory() {
    a_stored_op_reads_back_byte_identical(&mut memory());
}

#[test]
fn a_stored_op_reads_back_byte_identical_in_sqlite() {
    a_stored_op_reads_back_byte_identical(&mut sqlite());
}

fn nothing_is_filtered_on_the_way_in<L: OpLog>(log: &mut L) {
    // Split out of `storing_adversarial_ops_never_panics`, where this count
    // sat at the bottom of a 70-line test whose name advertised only
    // panic-freedom. It is load-bearing — it is what catches an append that
    // quietly drops a kind, such as a "validate early" filter on `Moderate`
    // — and a comment cannot defend an assertion from a well-intentioned
    // edit the way a name can.
    let author = a_key(2);
    let stoa = a_stoa("Agora");
    for kind in every_op_kind() {
        let op = Op {
            stoa,
            author: author.public_key(),
            kind,
        }
        .sign(&author);
        log.append(op, Arrival::unordered()).unwrap();
    }
    // Hardcoded rather than `every_op_kind().len()`, which would agree with
    // the fixture however wrong the fixture became. Update both together
    // when an op kind lands.
    assert_eq!(
        log.len().unwrap(),
        6,
        "an op kind was filtered on the way in"
    );
}

#[test]
fn nothing_is_filtered_on_the_way_in_in_memory() {
    nothing_is_filtered_on_the_way_in(&mut memory());
}

#[test]
fn nothing_is_filtered_on_the_way_in_in_sqlite() {
    nothing_is_filtered_on_the_way_in(&mut sqlite());
}

// ─── Dedup by op id ───────────────────────────────────────────────────────

fn the_same_op_appended_twice_is_one_entry<L: OpLog>(log: &mut L) {
    // §3.1: ops are "idempotent by `opId`". Retransmission, causal-history
    // backfill and SDS-Repair all deliver ops a peer may already hold, so a
    // second append is ordinary traffic.
    let op = signed(a_post("once"));
    assert_eq!(
        log.append(op.clone(), Arrival::unordered()).unwrap(),
        Appended::Stored
    );
    assert_eq!(
        log.append(op.clone(), Arrival::unordered()).unwrap(),
        Appended::AlreadyPresent
    );
    assert_eq!(log.len().unwrap(), 1);
    assert_eq!(log.iter().unwrap().len(), 1);
}

#[test]
fn the_same_op_appended_twice_is_one_entry_in_memory() {
    the_same_op_appended_twice_is_one_entry(&mut memory());
}

#[test]
fn the_same_op_appended_twice_is_one_entry_in_sqlite() {
    the_same_op_appended_twice_is_one_entry(&mut sqlite());
}

fn the_append_result_says_whether_the_op_was_new<L: OpLog>(log: &mut L) {
    // Hardcoded expectations, not a count of the log before and after: a
    // caller acts on this (gossip onward, rebuild a view) and must not have
    // to derive it.
    let one = signed(a_post("one"));
    let two = signed(a_post("two"));
    assert_eq!(
        log.append(one.clone(), Arrival::unordered()).unwrap(),
        Appended::Stored
    );
    assert_eq!(
        log.append(two, Arrival::unordered()).unwrap(),
        Appended::Stored
    );
    assert_eq!(
        log.append(one, Arrival::unordered()).unwrap(),
        Appended::AlreadyPresent
    );
    assert_eq!(log.len().unwrap(), 2);
}

#[test]
fn the_append_result_says_whether_the_op_was_new_in_memory() {
    the_append_result_says_whether_the_op_was_new(&mut memory());
}

#[test]
fn the_append_result_says_whether_the_op_was_new_in_sqlite() {
    the_append_result_says_whether_the_op_was_new(&mut sqlite());
}

fn two_ops_differing_in_any_field_are_two_entries<L: OpLog>(log: &mut L) {
    // Dedup must be by the WHOLE op, not by author, Stoa or kind. A log that
    // keyed on anything coarser would silently drop a second post.
    let one = signed(a_post("first"));
    let two = signed(a_post("second"));
    let (one_id, two_id) = (one.op.id(), two.op.id());
    assert_ne!(one_id, two_id);

    log.append(one.clone(), Arrival::unordered()).unwrap();
    log.append(two.clone(), Arrival::unordered()).unwrap();
    assert_eq!(log.len().unwrap(), 2);
    assert_eq!(log.get(&one_id).unwrap().map(|e| e.op), Some(one));
    assert_eq!(log.get(&two_id).unwrap().map(|e| e.op), Some(two));
}

#[test]
fn two_ops_differing_in_any_field_are_two_entries_in_memory() {
    two_ops_differing_in_any_field_are_two_entries(&mut memory());
}

#[test]
fn two_ops_differing_in_any_field_are_two_entries_in_sqlite() {
    two_ops_differing_in_any_field_are_two_entries(&mut sqlite());
}

fn dedup_is_by_op_id_and_not_by_signature<L: OpLog>(log: &mut L) {
    // Two DIFFERENT ops from one author share no id; one op signed twice
    // shares one id. Ed25519 signatures here are deterministic, so this
    // pins the key rather than the signature bytes: a log keyed on the
    // signature would behave identically today and diverge the moment a
    // randomised scheme or a re-signed op appeared.
    //
    // For the SQLite implementation this is the test that pins the PRIMARY KEY
    // to the right column. "Remove the primary key" is a mutation that dies
    // easily; "make it `op_bytes` instead of `op_id`" is the one step weaker,
    // and today it would survive every other dedup test here — because
    // identical ops have identical bytes. It dies on this one only because the
    // ops compared are byte-identical AND the assertion is about the id.
    let op = a_post("same bytes");
    let first = op.clone().sign(&a_key(2));
    let second = op.sign(&a_key(2));
    assert_eq!(first.op.id(), second.op.id(), "the fixture is one op");

    log.append(first, Arrival::unordered()).unwrap();
    assert_eq!(
        log.append(second, Arrival::unordered()).unwrap(),
        Appended::AlreadyPresent
    );
    assert_eq!(log.len().unwrap(), 1);
}

#[test]
fn dedup_is_by_op_id_and_not_by_signature_in_memory() {
    dedup_is_by_op_id_and_not_by_signature(&mut memory());
}

#[test]
fn dedup_is_by_op_id_and_not_by_signature_in_sqlite() {
    dedup_is_by_op_id_and_not_by_signature(&mut sqlite());
}

// ─── The first arrival's metadata wins ────────────────────────────────────

fn a_second_arrival_does_not_overwrite_the_recorded_metadata<L: OpLog>(log: &mut L) {
    // Hardcoded: the log must report 5, the value recorded FIRST, and not 9.
    // Keeping the last would make a peer's order depend on how many times
    // each op happened to reach it, which differs per peer — exactly the
    // divergence `arrival.rs` exists to prevent, reintroduced at the store.
    //
    // For SQLite this is what separates `INSERT OR IGNORE` from
    // `INSERT OR REPLACE`.
    let op = signed(a_post("re-delivered"));
    let id = op.op.id();

    log.append(op.clone(), Arrival::ordered(5, a_message_id(1)))
        .unwrap();
    log.append(op, Arrival::ordered(9, a_message_id(2))).unwrap();

    let entry = log.get(&id).unwrap().unwrap();
    assert_eq!(entry.arrival.lamport(), Some(5));
    assert_eq!(entry.arrival.message_id(), Some(&a_message_id(1)));
}

#[test]
fn a_second_arrival_does_not_overwrite_the_recorded_metadata_in_memory() {
    a_second_arrival_does_not_overwrite_the_recorded_metadata(&mut memory());
}

#[test]
fn a_second_arrival_does_not_overwrite_the_recorded_metadata_in_sqlite() {
    a_second_arrival_does_not_overwrite_the_recorded_metadata(&mut sqlite());
}

fn a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one<L: OpLog>(log: &mut L) {
    // THE test that separates first-wins from richer-wins, and the only
    // direction that does: unordered FIRST, then ordered.
    //
    // Every other re-arrival test here delivers the ordered copy first, and
    // in that direction first-wins and richer-wins agree — so the rule both
    // this module's docs and `arrival.rs` name as "the tempting wrong
    // answer" passed the whole suite without a single failure until this
    // existed. A rule two modules warn about with no test that can catch it
    // is a rule the next refactor walks straight into.
    //
    // Richer-wins is wrong because whether a peer ever receives the richer
    // copy is a per-peer accident: two peers holding the same ops would
    // record different metadata and order them differently, with no error.
    // Convergence beats completeness, the same trade the degraded order
    // makes.
    let op = signed(a_post("poor first, rich second"));
    let id = op.op.id();

    log.append(op.clone(), Arrival::unordered()).unwrap();
    log.append(op, Arrival::ordered(7, a_message_id(1))).unwrap();

    let entry = log.get(&id).unwrap().unwrap();
    assert_eq!(
        entry.arrival.lamport(),
        None,
        "first-wins keeps the poorer arrival; this is richer-wins"
    );
    assert_eq!(entry.arrival.message_id(), None);
    assert!(!entry.arrival.is_ordered_by_transport());
}

#[test]
fn a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one_in_memory() {
    a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one(&mut memory());
}

#[test]
fn a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one_in_sqlite() {
    a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one(&mut sqlite());
}

fn an_unordered_re_arrival_does_not_erase_a_recorded_order<L: OpLog>(log: &mut L) {
    // Ordered first, then nothing. This is the BLUNT direction — a naive
    // "last write wins" fails it, but richer-wins agrees with it, so it
    // cannot distinguish first-wins from richer-wins on its own. The
    // direction that can is
    // `a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one`.
    //
    // Kept because last-write-wins is a real mistake and this is what
    // catches it: it would demote an ordered op to the degraded order and
    // move it in every thread it appears in.
    let op = signed(a_post("ordered once"));
    let id = op.op.id();

    log.append(op.clone(), Arrival::ordered(7, a_message_id(1)))
        .unwrap();
    log.append(op, Arrival::unordered()).unwrap();

    let entry = log.get(&id).unwrap().unwrap();
    assert_eq!(entry.arrival.lamport(), Some(7));
    assert!(entry.arrival.is_ordered_by_transport());
}

#[test]
fn an_unordered_re_arrival_does_not_erase_a_recorded_order_in_memory() {
    an_unordered_re_arrival_does_not_erase_a_recorded_order(&mut memory());
}

#[test]
fn an_unordered_re_arrival_does_not_erase_a_recorded_order_in_sqlite() {
    an_unordered_re_arrival_does_not_erase_a_recorded_order(&mut sqlite());
}

fn a_re_arrival_does_not_move_the_op_in_the_read_order<L: OpLog>(log: &mut L) {
    // The consequence at the level that matters: a thread already rendered
    // must not reorder because a duplicate arrived. Hardcoded expected
    // sequence, both before and after.
    // Lamport values assigned against op-id order, so a log ignoring the
    // metadata would fail the `before` assertion rather than agree with it.
    let (low, high) = two_posts_by_ascending_id();

    log.append(low.clone(), Arrival::ordered(1, a_message_id(1)))
        .unwrap();
    log.append(high.clone(), Arrival::ordered(2, a_message_id(1)))
        .unwrap();

    let before = ids(&log.iter().unwrap());
    assert_eq!(before, vec![high.op.id(), low.op.id()]);

    // Re-deliver the low op claiming a Lamport value that would put it first.
    log.append(low.clone(), Arrival::ordered(99, a_message_id(1)))
        .unwrap();

    let after = ids(&log.iter().unwrap());
    assert_eq!(after, vec![high.op.id(), low.op.id()], "the order moved");
}

#[test]
fn a_re_arrival_does_not_move_the_op_in_the_read_order_in_memory() {
    a_re_arrival_does_not_move_the_op_in_the_read_order(&mut memory());
}

#[test]
fn a_re_arrival_does_not_move_the_op_in_the_read_order_in_sqlite() {
    a_re_arrival_does_not_move_the_op_in_the_read_order(&mut sqlite());
}

fn one_op_arriving_twice_with_different_metadata_is_still_one_entry<L: OpLog>(log: &mut L) {
    // THE case that would otherwise reach `cmp_ops` as an undefined tie.
    //
    // `cmp_ops` is total over DISTINCT ops; two entries sharing an op id but
    // carrying different `Arrival` metadata compare Equal — correctly, since
    // §5.7's rule has nothing to say about one op against itself — but a sort
    // over such a pair would leave their relative order to the sort's
    // stability, which differs with the sequence a peer received in. Two
    // honest peers would then render one thread differently, with no error
    // anywhere.
    //
    // Dedup must therefore happen BEFORE any sort, never as a pass over
    // sorted output. In memory that is the map key; in SQLite it is the
    // PRIMARY KEY, and the ordering index ends in that key so it is unique —
    // no `SELECT ... ORDER BY` over it can emit one op id twice. The `op-log`
    // design named a non-unique ordering index as the shape that would reopen
    // this hole; this is what would catch it.
    let op = signed(a_post("arrives twice"));
    let id = op.op.id();

    // Deliberately the exact pair the probe found: unordered, versus a
    // message id with no Lamport value. Both are unordered by the transport,
    // so both fall through to the op-id comparison and tie.
    let plain = Arrival::unordered();
    let with_id = Arrival::from_parts(None, Some(a_message_id(1)));
    assert_ne!(plain, with_id, "the fixture needs two different arrivals");

    log.append(op.clone(), plain.clone()).unwrap();
    log.append(op, with_id).unwrap();

    assert_eq!(log.len().unwrap(), 1, "one op id is one entry");
    assert_eq!(
        log.iter().unwrap().len(),
        1,
        "no duplicate reaches the ordering"
    );
    // And the first-wins rule decided which arrival survived.
    assert_eq!(log.get(&id).unwrap().unwrap().arrival, plain);
}

#[test]
fn one_op_arriving_twice_with_different_metadata_is_still_one_entry_in_memory() {
    one_op_arriving_twice_with_different_metadata_is_still_one_entry(&mut memory());
}

#[test]
fn one_op_arriving_twice_with_different_metadata_is_still_one_entry_in_sqlite() {
    one_op_arriving_twice_with_different_metadata_is_still_one_entry(&mut sqlite());
}

fn no_two_entries_in_a_read_ever_share_an_op_id<L: OpLog>(log: &mut L) {
    // The general form of the property above, over a log built entirely from
    // re-deliveries with varying metadata. If any read could emit two
    // entries with one op id, the ordering that produced it consulted an
    // undefined order.
    let ops = [
        signed(a_post("a")),
        signed(a_post("b")),
        signed(a_post("c")),
    ];
    let arrivals = [
        Arrival::unordered(),
        Arrival::from_parts(None, Some(a_message_id(1))),
        Arrival::ordered(3, a_message_id(2)),
        Arrival::from_parts(Some(3), None),
    ];
    // Every op delivered under every arrival: 12 appends, 3 distinct ops.
    for arrival in arrivals.iter() {
        for op in ops.iter() {
            log.append(op.clone(), arrival.clone()).unwrap();
        }
    }
    assert_eq!(log.len().unwrap(), 3);

    let mut got = ids(&log.iter().unwrap());
    assert_eq!(got.len(), 3);
    got.sort();
    got.dedup();
    assert_eq!(got.len(), 3, "a read emitted two entries with one op id");
}

#[test]
fn no_two_entries_in_a_read_ever_share_an_op_id_in_memory() {
    no_two_entries_in_a_read_ever_share_an_op_id(&mut memory());
}

#[test]
fn no_two_entries_in_a_read_ever_share_an_op_id_in_sqlite() {
    no_two_entries_in_a_read_ever_share_an_op_id(&mut sqlite());
}

// ─── The read order is the ordering rule's ────────────────────────────────

fn reading_returns_ops_in_lamport_order_not_insertion_order<L: OpLog>(log: &mut L) {
    // Hardcoded expected sequence. The ops are appended in exactly the
    // reverse of the order that must come out, so a log returning insertion
    // order fails rather than coincidentally agreeing.
    //
    // The Lamport values are assigned AGAINST op-id order — the lower op id
    // gets the lower timestamp — so a read that fell back to ordering by op
    // id would return the reverse of what is asserted here. Without that,
    // this test would pass for a log that consulted no metadata at all, on
    // whatever order the fixture's hashes happened to land in.
    let (low_id, high_id) = two_posts_by_ascending_id();

    log.append(low_id.clone(), Arrival::ordered(1, a_message_id(1)))
        .unwrap();
    log.append(high_id.clone(), Arrival::ordered(2, a_message_id(1)))
        .unwrap();

    assert_eq!(
        ids(&log.iter().unwrap()),
        vec![high_id.op.id(), low_id.op.id()],
        "highest Lamport must read first, against op-id order"
    );
}

#[test]
fn reading_returns_ops_in_lamport_order_not_insertion_order_in_memory() {
    reading_returns_ops_in_lamport_order_not_insertion_order(&mut memory());
}

#[test]
fn reading_returns_ops_in_lamport_order_not_insertion_order_in_sqlite() {
    reading_returns_ops_in_lamport_order_not_insertion_order(&mut sqlite());
}

fn the_message_id_tiebreak_is_used_and_is_not_the_op_id<L: OpLog>(log: &mut L) {
    // Within one Lamport value, §5.7 breaks ties by ASCENDING message id.
    // The message ids are assigned against op-id order for the same reason
    // as above: a log that ignored the arrival and sorted by op id would
    // return the reverse.
    let (low_id, high_id) = two_posts_by_ascending_id();

    // The op with the LOWER op id carries the HIGHER message id.
    log.append(low_id.clone(), Arrival::ordered(7, a_message_id(9)))
        .unwrap();
    log.append(high_id.clone(), Arrival::ordered(7, a_message_id(1)))
        .unwrap();

    assert_eq!(
        ids(&log.iter().unwrap()),
        vec![high_id.op.id(), low_id.op.id()],
        "the lower message id reads first, against op-id order"
    );
}

#[test]
fn the_message_id_tiebreak_is_used_and_is_not_the_op_id_in_memory() {
    the_message_id_tiebreak_is_used_and_is_not_the_op_id(&mut memory());
}

#[test]
fn the_message_id_tiebreak_is_used_and_is_not_the_op_id_in_sqlite() {
    the_message_id_tiebreak_is_used_and_is_not_the_op_id(&mut sqlite());
}

fn ops_the_transport_did_not_order_read_in_ascending_op_id<L: OpLog>(log: &mut L) {
    // The degraded order, which is the ONLY order in production today. The
    // expected sequence is derived from the op ids alone — if anything
    // peer-local leaked into the read, the two logs would disagree.
    let (low, high) = two_posts_by_ascending_id();

    log.append(high.clone(), Arrival::unordered()).unwrap();
    log.append(low.clone(), Arrival::unordered()).unwrap();

    assert_eq!(
        ids(&log.iter().unwrap()),
        vec![low.op.id(), high.op.id()]
    );
}

#[test]
fn ops_the_transport_did_not_order_read_in_ascending_op_id_in_memory() {
    ops_the_transport_did_not_order_read_in_ascending_op_id(&mut memory());
}

#[test]
fn ops_the_transport_did_not_order_read_in_ascending_op_id_in_sqlite() {
    ops_the_transport_did_not_order_read_in_ascending_op_id(&mut sqlite());
}

fn ordered_and_unordered_ops_coexist_with_the_ordered_ones_first<L: OpLog>(log: &mut L) {
    // The mixture the transport fix will produce: a peer's existing log is
    // all unordered, and new arrivals carry Lamport values. Both must live
    // in one log, and the ordering rule places the ordered ones first
    // whatever their value.
    // The ORDERED op is given the HIGHER op id, so a log that ignored the
    // metadata and fell back to op id would put the unordered one first and
    // fail. Lamport 0 is the sharp value: it must still beat "no metadata".
    let (unordered, ordered_zero) = two_posts_by_ascending_id();

    log.append(unordered.clone(), Arrival::unordered()).unwrap();
    log.append(ordered_zero.clone(), Arrival::ordered(0, a_message_id(1)))
        .unwrap();

    assert_eq!(
        ids(&log.iter().unwrap()),
        vec![ordered_zero.op.id(), unordered.op.id()],
        "even Lamport 0 beats an op the transport did not order"
    );
}

#[test]
fn ordered_and_unordered_ops_coexist_with_the_ordered_ones_first_in_memory() {
    ordered_and_unordered_ops_coexist_with_the_ordered_ones_first(&mut memory());
}

#[test]
fn ordered_and_unordered_ops_coexist_with_the_ordered_ones_first_in_sqlite() {
    ordered_and_unordered_ops_coexist_with_the_ordered_ones_first(&mut sqlite());
}

fn whether_an_arrival_was_ordered_survives_storage<L: OpLog>(log: &mut L) {
    // The seam for the upstream fix: a peer must be able to say "this thread
    // is ordered by the network" or "by op id". A store that dropped the
    // distinction could not.
    let ordered = signed(a_post("ordered"));
    let unordered = signed(a_post("unordered"));
    log.append(ordered.clone(), Arrival::ordered(1, a_message_id(1)))
        .unwrap();
    log.append(unordered.clone(), Arrival::unordered()).unwrap();

    assert!(log
        .get(&ordered.op.id())
        .unwrap()
        .unwrap()
        .arrival
        .is_ordered_by_transport());
    assert!(!log
        .get(&unordered.op.id())
        .unwrap()
        .unwrap()
        .arrival
        .is_ordered_by_transport());
}

#[test]
fn whether_an_arrival_was_ordered_survives_storage_in_memory() {
    whether_an_arrival_was_ordered_survives_storage(&mut memory());
}

#[test]
fn whether_an_arrival_was_ordered_survives_storage_in_sqlite() {
    whether_an_arrival_was_ordered_survives_storage(&mut sqlite());
}

fn an_empty_message_id_is_recorded_and_is_not_absence<L: OpLog>(log: &mut L) {
    // The EMPTY message id is a legal recorded value, and it is not the same
    // fact as "no message id was supplied".
    //
    // This is `absence_is_not_equal_to_a_zero_lamport_timestamp`'s shape at
    // the store, and for SQLite it is what forces `sort_msg_present` to be its
    // own column: a schema encoding presence as "the blob is not empty" would
    // pass every other ordering test here and collapse these two arrivals,
    // reordering an op carrying an empty id as though the transport had
    // supplied nothing.
    let empty = signed(a_post("empty id"));
    let absent = signed(a_post("no id"));
    log.append(
        empty.clone(),
        Arrival::from_parts(None, Some(crate::arrival::MessageId::new(vec![]))),
    )
    .unwrap();
    log.append(absent.clone(), Arrival::unordered()).unwrap();

    let empty_back = log.get(&empty.op.id()).unwrap().unwrap();
    assert_eq!(
        empty_back.arrival.message_id().map(|m| m.as_bytes().len()),
        Some(0),
        "an empty message id must read back as present and empty"
    );
    assert_eq!(
        log.get(&absent.op.id())
            .unwrap()
            .unwrap()
            .arrival
            .message_id(),
        None,
        "absence must read back as absence"
    );
}

#[test]
fn an_empty_message_id_is_recorded_and_is_not_absence_in_memory() {
    an_empty_message_id_is_recorded_and_is_not_absence(&mut memory());
}

#[test]
fn an_empty_message_id_is_recorded_and_is_not_absence_in_sqlite() {
    an_empty_message_id_is_recorded_and_is_not_absence(&mut sqlite());
}

// ─── Restricted reads ─────────────────────────────────────────────────────

fn a_stoa_restricted_read_excludes_other_stoas<L: OpLog>(log: &mut L) {
    // §4.5 keys on the Stoa ADDRESS, never a channel id. Hardcoded: exactly
    // the one op from the requested Stoa.
    let agora = a_stoa("Agora");
    let lyceum = a_stoa("Lyceum");
    assert_ne!(agora, lyceum);

    let here = signed(a_post("in the agora"));
    let there = signed(Op {
        stoa: lyceum,
        ..a_post("in the lyceum")
    });

    log.append(here.clone(), Arrival::unordered()).unwrap();
    log.append(there.clone(), Arrival::unordered()).unwrap();

    assert_eq!(ids(&log.iter_stoa(&agora).unwrap()), vec![here.op.id()]);
    assert_eq!(ids(&log.iter_stoa(&lyceum).unwrap()), vec![there.op.id()]);
    assert_eq!(
        log.iter().unwrap().len(),
        2,
        "both are in the unrestricted read"
    );
}

#[test]
fn a_stoa_restricted_read_excludes_other_stoas_in_memory() {
    a_stoa_restricted_read_excludes_other_stoas(&mut memory());
}

#[test]
fn a_stoa_restricted_read_excludes_other_stoas_in_sqlite() {
    a_stoa_restricted_read_excludes_other_stoas(&mut sqlite());
}

fn two_stoas_sharing_an_address_prefix_are_not_confused<L: OpLog>(log: &mut L) {
    // Whole-key matching, not merely "some filtering happens".
    //
    // `a_stoa_restricted_read_excludes_other_stoas` uses two Stoas whose
    // addresses differ in the first byte, so a read comparing only a PREFIX
    // passes it. This one cannot be passed that way: the two addresses are
    // chosen to COLLIDE in byte 0 and differ only later.
    //
    // Why it matters here specifically: a prefix-matching `iter_stoa` is a
    // cross-Stoa leak in a censorship-resistant forum — another Stoa's ops
    // bleeding into this Stoa's read. With 32-byte addresses a collision is
    // rare enough never to surface in casual testing and certain enough to
    // surface eventually, which is the worst combination.
    let (one, two) = ("S0", "S178");
    let (addr_one, addr_two) = (a_stoa(one), a_stoa(two));

    // FIXTURE GUARD. These addresses are hash-derived, so the collision is a
    // property of the current genesis encoding rather than of the titles. If
    // an encoding change breaks it, this test must fail loudly here rather
    // than silently stop exercising prefix confusion — the "a new check can
    // retire an old test" hazard, applied to our own fixture.
    assert_eq!(
        addr_one.as_bytes()[0],
        addr_two.as_bytes()[0],
        "fixture must share a first byte, or it tests nothing"
    );
    assert_ne!(addr_one, addr_two, "fixture must be two distinct Stoas");

    let here = signed(Op {
        stoa: addr_one,
        ..a_post("in the first")
    });
    let there = signed(Op {
        stoa: addr_two,
        ..a_post("in the second")
    });

    log.append(here.clone(), Arrival::unordered()).unwrap();
    log.append(there.clone(), Arrival::unordered()).unwrap();

    assert_eq!(
        ids(&log.iter_stoa(&addr_one).unwrap()),
        vec![here.op.id()],
        "a prefix match leaked a Stoa"
    );
    assert_eq!(
        ids(&log.iter_stoa(&addr_two).unwrap()),
        vec![there.op.id()]
    );
}

#[test]
fn two_stoas_sharing_an_address_prefix_are_not_confused_in_memory() {
    two_stoas_sharing_an_address_prefix_are_not_confused(&mut memory());
}

#[test]
fn two_stoas_sharing_an_address_prefix_are_not_confused_in_sqlite() {
    two_stoas_sharing_an_address_prefix_are_not_confused(&mut sqlite());
}

fn two_targets_sharing_an_op_id_prefix_are_not_confused<L: OpLog>(log: &mut L) {
    // The same property for `iter_target`, and the consequence is worse: a
    // moderation resolver handed ops aimed at a DIFFERENT post would apply
    // one post's `Hide` to an unrelated post. Silent, and invisible until
    // two op ids happen to collide.
    let (one, two) = ("p0", "p37");
    let (post_one, post_two) = (signed(a_post(one)), signed(a_post(two)));
    let (target_one, target_two) = (post_one.op.id(), post_two.op.id());

    // FIXTURE GUARD, for the same reason as above: op ids are hashes.
    assert_eq!(
        target_one.as_bytes()[0],
        target_two.as_bytes()[0],
        "fixture must share a first byte, or it tests nothing"
    );
    assert_ne!(target_one, target_two, "fixture must be two distinct ops");

    let author = a_key(2);
    let moderate_one = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Moderate {
            target: target_one,
            action: ModerationAction::Hide,
        },
    }
    .sign(&author);
    let moderate_two = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Moderate {
            target: target_two,
            action: ModerationAction::Unhide,
        },
    }
    .sign(&author);

    for op in [
        post_one,
        post_two,
        moderate_one.clone(),
        moderate_two.clone(),
    ] {
        log.append(op, Arrival::unordered()).unwrap();
    }

    assert_eq!(
        ids(&log.iter_target(&target_one).unwrap()),
        vec![moderate_one.op.id()],
        "a prefix match leaked another post's moderation"
    );
    assert_eq!(
        ids(&log.iter_target(&target_two).unwrap()),
        vec![moderate_two.op.id()]
    );
}

#[test]
fn two_targets_sharing_an_op_id_prefix_are_not_confused_in_memory() {
    two_targets_sharing_an_op_id_prefix_are_not_confused(&mut memory());
}

#[test]
fn two_targets_sharing_an_op_id_prefix_are_not_confused_in_sqlite() {
    two_targets_sharing_an_op_id_prefix_are_not_confused(&mut sqlite());
}

fn a_non_empty_log_read_against_an_absent_stoa_is_empty<L: OpLog>(log: &mut L) {
    // The empty-log case is covered elsewhere; this is the populated one,
    // where a read that ignored its argument would return everything rather
    // than nothing.
    log.append(signed(a_post("in the agora")), Arrival::unordered())
        .unwrap();

    let elsewhere = a_stoa("Never Used");
    assert_eq!(log.iter_stoa(&elsewhere).unwrap().len(), 0);
    assert_eq!(log.len().unwrap(), 1, "the log is genuinely non-empty");
}

#[test]
fn a_non_empty_log_read_against_an_absent_stoa_is_empty_in_memory() {
    a_non_empty_log_read_against_an_absent_stoa_is_empty(&mut memory());
}

#[test]
fn a_non_empty_log_read_against_an_absent_stoa_is_empty_in_sqlite() {
    a_non_empty_log_read_against_an_absent_stoa_is_empty(&mut sqlite());
}

fn a_target_restricted_read_returns_every_kind_that_names_the_target<L: OpLog>(log: &mut L) {
    // The shape both resolvers fold over. A revise, a moderate and a vote on
    // one post must all come back, because "the ops about this subject" is
    // one question — a resolver narrows what it is given.
    let post = signed(a_post("the subject"));
    let target = post.op.id();
    let author = a_key(2);

    let revise = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Revise {
            target,
            body: "edited".to_string(),
            attachments: vec![],
        },
    }
    .sign(&author);
    let moderate = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Moderate {
            target,
            action: ModerationAction::Hide,
        },
    }
    .sign(&author);
    let vote = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Vote {
            target,
            direction: crate::op::VoteDirection::Up,
        },
    }
    .sign(&author);

    for op in [post.clone(), revise.clone(), moderate.clone(), vote.clone()] {
        log.append(op, Arrival::unordered()).unwrap();
    }

    let mut got = ids(&log.iter_target(&target).unwrap());
    let mut expected = vec![revise.op.id(), moderate.op.id(), vote.op.id()];
    got.sort();
    expected.sort();
    assert_eq!(got, expected, "all three kinds naming the target");
    assert_eq!(log.len().unwrap(), 4, "the post itself is still stored");
}

#[test]
fn a_target_restricted_read_returns_every_kind_that_names_the_target_in_memory() {
    a_target_restricted_read_returns_every_kind_that_names_the_target(&mut memory());
}

#[test]
fn a_target_restricted_read_returns_every_kind_that_names_the_target_in_sqlite() {
    a_target_restricted_read_returns_every_kind_that_names_the_target(&mut sqlite());
}

fn a_post_names_no_target_and_is_never_returned_by_a_target_read<L: OpLog>(log: &mut L) {
    // A reply's `parent` is a reply relationship, not a subject acted upon.
    // Conflating them would make a moderation resolver see every reply to a
    // post alongside the moderations of it.
    let parent = signed(a_post("the parent"));
    let parent_id = parent.op.id();
    let reply = signed(Op {
        kind: OpKind::Post {
            thread: Some(parent_id),
            parent: Some(parent_id),
            body: "a reply".to_string(),
            attachments: vec![],
        },
        ..a_post("unused")
    });

    log.append(parent, Arrival::unordered()).unwrap();
    log.append(reply.clone(), Arrival::unordered()).unwrap();

    assert_eq!(
        log.iter_target(&parent_id).unwrap().len(),
        0,
        "a reply is not an op acting on its parent"
    );
    assert_eq!(log.len().unwrap(), 2, "both are stored");
}

#[test]
fn a_post_names_no_target_and_is_never_returned_by_a_target_read_in_memory() {
    a_post_names_no_target_and_is_never_returned_by_a_target_read(&mut memory());
}

#[test]
fn a_post_names_no_target_and_is_never_returned_by_a_target_read_in_sqlite() {
    a_post_names_no_target_and_is_never_returned_by_a_target_read(&mut sqlite());
}

fn a_stoa_metadata_op_names_no_target<L: OpLog>(log: &mut L) {
    // A metadata op acts on the Stoa, which it names through `Op::stoa`
    // rather than as a target op id, so no target-restricted read should
    // return it. Pinned because `Entry::target` returning `None` for a kind
    // that should name one fails silently: the op is simply never returned,
    // and no test unaware of the kind notices.
    //
    // It IS stored and IS returned by a Stoa read — the log holds it, it
    // just is not about another op.
    let author = a_key(2);
    let stoa = a_stoa("Agora");
    let post = signed(a_post("some post"));
    let metadata = Op {
        stoa,
        author: author.public_key(),
        kind: OpKind::StoaMetadata {
            title: "Renamed".to_string(),
            description: "now with a description".to_string(),
        },
    }
    .sign(&author);

    log.append(post.clone(), Arrival::unordered()).unwrap();
    log.append(metadata.clone(), Arrival::unordered()).unwrap();

    assert_eq!(
        log.iter_target(&post.op.id()).unwrap().len(),
        0,
        "a metadata op does not act on another op"
    );
    assert!(
        ids(&log.iter_stoa(&stoa).unwrap()).contains(&metadata.op.id()),
        "but it is stored and reachable by Stoa"
    );
}

#[test]
fn a_stoa_metadata_op_names_no_target_in_memory() {
    a_stoa_metadata_op_names_no_target(&mut memory());
}

#[test]
fn a_stoa_metadata_op_names_no_target_in_sqlite() {
    a_stoa_metadata_op_names_no_target(&mut sqlite());
}

fn a_target_read_does_not_return_the_target_itself<L: OpLog>(log: &mut L) {
    // An op is never its own target: `iter_target` returns the ops acting ON
    // it. A log that included the subject would make every resolver's fold
    // start on an entry of the wrong kind.
    let post = signed(a_post("the subject"));
    let target = post.op.id();
    log.append(post, Arrival::unordered()).unwrap();
    assert_eq!(log.iter_target(&target).unwrap().len(), 0);
}

#[test]
fn a_target_read_does_not_return_the_target_itself_in_memory() {
    a_target_read_does_not_return_the_target_itself(&mut memory());
}

#[test]
fn a_target_read_does_not_return_the_target_itself_in_sqlite() {
    a_target_read_does_not_return_the_target_itself(&mut sqlite());
}

fn a_restricted_read_preserves_the_unrestricted_relative_order<L: OpLog>(log: &mut L) {
    // A resolver folds over a restricted read and takes the first match, so
    // the restriction must not reorder. Hardcoded expected sequence.
    //
    // "First" means current under `cmp_ops`, which is most-recent only on
    // the ORDERED branch — the one this test's fixture uses, and the one
    // production does not reach today. The degraded branch is ascending op
    // id and carries no recency; `arrival.rs` says so explicitly after the
    // moderation resolver read recency into it.
    let post = signed(a_post("the subject"));
    let target = post.op.id();
    let author = a_key(2);
    let older = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Revise {
            target,
            body: "v2".to_string(),
            attachments: vec![],
        },
    }
    .sign(&author);
    let newer = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Revise {
            target,
            body: "v3".to_string(),
            attachments: vec![],
        },
    }
    .sign(&author);

    log.append(post, Arrival::ordered(1, a_message_id(1)))
        .unwrap();
    log.append(older.clone(), Arrival::ordered(2, a_message_id(1)))
        .unwrap();
    log.append(newer.clone(), Arrival::ordered(3, a_message_id(1)))
        .unwrap();

    assert_eq!(
        ids(&log.iter_target(&target).unwrap()),
        vec![newer.op.id(), older.op.id()],
        "the current version reads first"
    );
}

#[test]
fn a_restricted_read_preserves_the_unrestricted_relative_order_in_memory() {
    a_restricted_read_preserves_the_unrestricted_relative_order(&mut memory());
}

#[test]
fn a_restricted_read_preserves_the_unrestricted_relative_order_in_sqlite() {
    a_restricted_read_preserves_the_unrestricted_relative_order(&mut sqlite());
}

// ─── A partial set is the normal case ─────────────────────────────────────

fn every_read_over_an_empty_log_is_empty_and_not_an_error<L: OpLog>(log: &mut L) {
    // §3.3: a peer cannot establish that its set is complete, so a read that
    // needed completeness could never answer at all.
    assert_eq!(log.len().unwrap(), 0);
    assert!(log.is_empty().unwrap());
    assert_eq!(log.iter().unwrap().len(), 0);
    assert_eq!(log.iter_stoa(&a_stoa("Agora")).unwrap().len(), 0);
    assert_eq!(
        log.iter_target(&signed(a_post("absent")).op.id())
            .unwrap()
            .len(),
        0
    );
    assert_eq!(log.get(&signed(a_post("absent")).op.id()).unwrap(), None);
}

#[test]
fn every_read_over_an_empty_log_is_empty_and_not_an_error_in_memory() {
    every_read_over_an_empty_log_is_empty_and_not_an_error(&mut memory());
}

#[test]
fn every_read_over_an_empty_log_is_empty_and_not_an_error_in_sqlite() {
    every_read_over_an_empty_log_is_empty_and_not_an_error(&mut sqlite());
}

fn a_revision_whose_target_is_absent_is_stored_and_readable<L: OpLog>(log: &mut L) {
    // The dangling-reference case, which is ordinary: the target may simply
    // not have propagated yet. A store that required the target to be
    // present would drop ops that arrive out of order and never recover
    // them.
    //
    // For a database this is the test that forbids a FOREIGN KEY on `target`,
    // which is otherwise the natural thing to write and would reject exactly
    // the ops §4.7's backfill delivers out of order by design.
    let missing_target = signed(a_post("never received")).op.id();
    let author = a_key(2);
    let orphan = Op {
        stoa: a_stoa("Agora"),
        author: author.public_key(),
        kind: OpKind::Revise {
            target: missing_target,
            body: "revises something we lack".to_string(),
            attachments: vec![],
        },
    }
    .sign(&author);

    log.append(orphan.clone(), Arrival::unordered()).unwrap();

    assert_eq!(
        log.get(&missing_target).unwrap(),
        None,
        "the target really is absent"
    );
    assert_eq!(log.iter().unwrap().len(), 1);
    assert_eq!(
        ids(&log.iter_target(&missing_target).unwrap()),
        vec![orphan.op.id()]
    );
}

#[test]
fn a_revision_whose_target_is_absent_is_stored_and_readable_in_memory() {
    a_revision_whose_target_is_absent_is_stored_and_readable(&mut memory());
}

#[test]
fn a_revision_whose_target_is_absent_is_stored_and_readable_in_sqlite() {
    a_revision_whose_target_is_absent_is_stored_and_readable(&mut sqlite());
}

fn a_reply_whose_parent_is_absent_is_stored<L: OpLog>(log: &mut L) {
    let orphan_parent = signed(a_post("never received")).op.id();
    let reply = signed(Op {
        kind: OpKind::Post {
            thread: Some(orphan_parent),
            parent: Some(orphan_parent),
            body: "a reply to nothing we hold".to_string(),
            attachments: vec![],
        },
        ..a_post("unused")
    });
    let id = reply.op.id();
    assert_eq!(
        log.append(reply, Arrival::unordered()).unwrap(),
        Appended::Stored
    );
    assert!(log.get(&id).unwrap().is_some());
}

#[test]
fn a_reply_whose_parent_is_absent_is_stored_in_memory() {
    a_reply_whose_parent_is_absent_is_stored(&mut memory());
}

#[test]
fn a_reply_whose_parent_is_absent_is_stored_in_sqlite() {
    a_reply_whose_parent_is_absent_is_stored(&mut sqlite());
}

fn looking_up_an_absent_op_is_a_defined_absence<L: OpLog>(log: &mut L) {
    log.append(signed(a_post("present")), Arrival::unordered())
        .unwrap();
    assert_eq!(log.get(&signed(a_post("absent")).op.id()).unwrap(), None);
    assert_eq!(
        log.get(&OpId::from_hex(&"00".repeat(32)).unwrap()).unwrap(),
        None
    );
}

#[test]
fn looking_up_an_absent_op_is_a_defined_absence_in_memory() {
    looking_up_an_absent_op_is_a_defined_absence(&mut memory());
}

#[test]
fn looking_up_an_absent_op_is_a_defined_absence_in_sqlite() {
    looking_up_an_absent_op_is_a_defined_absence(&mut sqlite());
}

// ─── Hostile input ────────────────────────────────────────────────────────

fn storing_adversarial_ops_never_panics<L: OpLog>(log: &mut L) {
    // PHASE0-FINDINGS §3: a panic ABORTS the module process, so a malformed
    // op would become a denial of service against the peer that received it.
    // Everything stored arrived from a peer.
    //
    // For SQLite the sharp values are the Lamport boundaries: `0` and
    // `u64::MAX` are where a sort-key mapping that overflows an `i64` would
    // panic in a debug build rather than produce a wrong order.
    let author = a_key(2);
    let stoa = a_stoa("Agora");

    let kinds = every_op_kind();
    let arrivals = [
        Arrival::unordered(),
        Arrival::ordered(0, crate::arrival::MessageId::new(vec![])),
        Arrival::ordered(u64::MAX, crate::arrival::MessageId::new(vec![0xFF; 1024])),
        Arrival::from_parts(None, Some(crate::arrival::MessageId::new(vec![1]))),
        Arrival::from_parts(Some(1), None),
    ];

    for kind in kinds {
        for arrival in arrivals.iter() {
            let op = Op {
                stoa,
                author: author.public_key(),
                kind: kind.clone(),
            }
            .sign(&author);
            let id = op.op.id();
            log.append(op, arrival.clone()).unwrap();
            let _ = log.get(&id).unwrap();
            let _ = log.iter().unwrap();
            let _ = log.iter_stoa(&stoa).unwrap();
            let _ = log.iter_target(&id).unwrap();
            let _ = log.len().unwrap();
        }
    }
    // No count assertion here on purpose. That check is a different job and
    // has its own name — `nothing_is_filtered_on_the_way_in`. This test is
    // about panics only.
}

#[test]
fn storing_adversarial_ops_never_panics_in_memory() {
    storing_adversarial_ops_never_panics(&mut memory());
}

#[test]
fn storing_adversarial_ops_never_panics_in_sqlite() {
    storing_adversarial_ops_never_panics(&mut sqlite());
}

// ─── What the trait buys ──────────────────────────────────────────────────

fn a_resolver_can_be_written_against_the_trait_alone<L: OpLog>(log: &mut L) {
    // NO SPEC: the spec requires a read API the resolvers can fold over; it
    // does not require that they be written generically over the trait
    // rather than against the concrete type. Chosen: generic, and pinned
    // here, because the SQLite implementation §3.3 names is the reason the
    // trait exists — a resolver that named `MemoryOpLog` would have to be
    // rewritten rather than relinked.
    //
    // This stands in for both real resolvers: it is the fold each performs,
    // over the read each uses, and it is now exercised against BOTH
    // implementations — which is the claim "swapping the store underneath a
    // resolver is a change to one line" made checkable.
    fn latest_moderation<L: OpLog>(
        log: &L,
        target: &OpId,
    ) -> Result<Option<ModerationAction>, super::OpLogError> {
        Ok(log
            .iter_target(target)?
            .into_iter()
            .find_map(|e| match &e.op.op.kind {
                OpKind::Moderate { action, .. } => Some(*action),
                _ => None,
            }))
    }

    let post = signed(a_post("the subject"));
    let target = post.op.id();
    let moderator = a_key(1);
    let hide = Op {
        stoa: a_stoa("Agora"),
        author: moderator.public_key(),
        kind: OpKind::Moderate {
            target,
            action: ModerationAction::Hide,
        },
    }
    .sign(&moderator);
    let unhide = Op {
        stoa: a_stoa("Agora"),
        author: moderator.public_key(),
        kind: OpKind::Moderate {
            target,
            action: ModerationAction::Unhide,
        },
    }
    .sign(&moderator);

    log.append(post, Arrival::ordered(1, a_message_id(1)))
        .unwrap();
    log.append(hide, Arrival::ordered(2, a_message_id(1)))
        .unwrap();
    log.append(unhide, Arrival::ordered(3, a_message_id(1)))
        .unwrap();

    // Last write wins by Lamport order (§5.7): the unhide is current.
    assert_eq!(
        latest_moderation(log, &target).unwrap(),
        Some(ModerationAction::Unhide)
    );
}

#[test]
fn a_resolver_can_be_written_against_the_trait_alone_in_memory() {
    a_resolver_can_be_written_against_the_trait_alone(&mut memory());
}

#[test]
fn a_resolver_can_be_written_against_the_trait_alone_in_sqlite() {
    a_resolver_can_be_written_against_the_trait_alone(&mut sqlite());
}

// ─── The two implementations agree ────────────────────────────────────────

#[test]
fn both_implementations_agree_across_every_ordering_branch() {
    // **THE test the second implementation exists to make possible**, and the
    // one thing holding the SQL `ORDER BY` to `cmp_ops`.
    //
    // `sqlite.rs` re-expresses the order as an ORDER BY over columns computed
    // at append time, because a comparator applied in Rust cannot be indexed
    // and §2.5's paginated reads need an index. That leaves two definitions of
    // one order with no compiler checking them against each other — which is
    // exactly the "fourth slightly-different guard" CLAUDE.md warns about.
    //
    // This is what makes that survivable. `every_ordering_shape` crosses every
    // branch `cmp_ops` distinguishes — ordered against unordered, Lamport at
    // `0` and `u64::MAX`, equal Lamport with differing message ids, equal
    // Lamport with EQUAL message ids (forcing the op-id last resort), a
    // message id with no Lamport value, a Lamport value with no message id —
    // and asserts the two sequences are identical element for element.
    //
    // A divergence is a failing test naming the pair, not a rendering
    // difference between two honest peers with no error anywhere.
    let population = every_ordering_shape();

    let mut mem = memory();
    let mut sql = sqlite();
    for (op, arrival) in &population {
        mem.append(op.clone(), arrival.clone()).unwrap();
        sql.append(op.clone(), arrival.clone()).unwrap();
    }

    let from_memory = ids(&mem.iter().unwrap());
    let from_sqlite = ids(&sql.iter().unwrap());
    assert_eq!(
        from_memory.len(),
        population.len(),
        "the fixture must reach the comparison intact"
    );
    assert_eq!(
        from_memory, from_sqlite,
        "the SQL ORDER BY and cmp_ops disagree"
    );

    // And the same for each restricted read, because each uses its own index
    // and a wrong index column would reorder one read and not the others.
    let stoa = a_stoa("Agora");
    assert_eq!(
        ids(&mem.iter_stoa(&stoa).unwrap()),
        ids(&sql.iter_stoa(&stoa).unwrap()),
        "the Stoa-restricted reads disagree"
    );
    assert_eq!(mem.len().unwrap(), sql.len().unwrap());
}

#[test]
fn both_implementations_agree_on_a_target_restricted_read() {
    // The restricted read the resolvers actually fold over, given a
    // population that crosses the ordering branches. Separate from the
    // unrestricted comparison because `ops_by_target` is a different index
    // and its column order could be wrong on its own.
    let post = signed(a_post("the subject"));
    let target = post.op.id();
    let author = a_key(2);

    // One moderation per arrival shape, all naming one target, so the
    // target-restricted read has something to order.
    let arrivals = [
        Arrival::unordered(),
        Arrival::from_parts(None, Some(a_message_id(2))),
        Arrival::from_parts(Some(4), None),
        Arrival::ordered(4, a_message_id(1)),
        Arrival::ordered(4, a_message_id(8)),
        Arrival::ordered(0, a_message_id(1)),
        Arrival::ordered(u64::MAX, a_message_id(1)),
    ];
    let ops: Vec<(SignedOp, Arrival)> = arrivals
        .iter()
        .enumerate()
        .map(|(i, arrival)| {
            let op = Op {
                stoa: a_stoa("Agora"),
                author: author.public_key(),
                kind: OpKind::Revise {
                    target,
                    body: format!("v{i}"),
                    attachments: vec![],
                },
            }
            .sign(&author);
            (op, arrival.clone())
        })
        .collect();

    let mut mem = memory();
    let mut sql = sqlite();
    for log_pair in [
        (&mut mem as &mut dyn OpLog, ()),
        (&mut sql as &mut dyn OpLog, ()),
    ] {
        let (log, ()) = log_pair;
        log.append(post.clone(), Arrival::unordered()).unwrap();
        for (op, arrival) in &ops {
            log.append(op.clone(), arrival.clone()).unwrap();
        }
    }

    let from_memory = ids(&mem.iter_target(&target).unwrap());
    let from_sqlite = ids(&sql.iter_target(&target).unwrap());
    assert_eq!(
        from_memory.len(),
        ops.len(),
        "every revision must reach the comparison"
    );
    assert_eq!(
        from_memory, from_sqlite,
        "the target-restricted reads disagree"
    );
}
