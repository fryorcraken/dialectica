//! The seeder for the end-to-end successful join, and the check that its output
//! still verifies.
//!
//! # What this is
//!
//! `dialectica-ui/tests/ui/seeded-join.yaml` pastes a Stoa reference into a
//! fresh profile and joins it. For that join to succeed the reference must be
//! one the core verifies: its genesis record decodes, its address is that
//! record's hash, and the Stoa it names is one the profile does not hold. A
//! running Basecamp cannot mint one — a profile that created the Stoa would
//! already hold it — so the reference comes from here.
//!
//! **The reference is committed, as a literal in that spec**, and this file
//! reads it back out of the spec and puts it through the same two core calls
//! the view makes with it: `get_stoa` for the preview and `join_stoa` for the
//! join. The `e2e-successful-join` change's design.md D1 says why committed and
//! not generated per run, and D2 why the check reads the spec rather than a
//! second copy of the literal.
//!
//! # What a red test here means
//!
//! **Read this before regenerating anything.** `stoa-navigation-view` makes the
//! reference encoding a compatibility surface: a reference somebody pasted into
//! a chat last month must still join. This reference was written under the
//! genesis encoding current when it was committed. If the core stops accepting
//! it, every reference in the wild written under that encoding has stopped
//! joining too, and this test is the first thing to say so — in `ci.yml`'s
//! `rust` job, on the PR that made the change, rather than minutes later in
//! `ui-tests.yml`.
//!
//! So a red here is a question for the change that caused it (is breaking old
//! references intended, and where is that decided?), not a fixture to refresh.
//! If it IS intended, the failure message prints the reference the current
//! core would write for the same record, ready to paste into the spec.
//!
//! # The creator's secret is public, on purpose
//!
//! The creator key comes from a fixed 32-byte seed written below, because
//! `stoa-genesis` refuses a creator that is not a valid public key and a real
//! key is the simplest way to have one. Anybody reading this file can sign as
//! this Stoa's moderator. That is harmless for a fixture Stoa nobody posts in,
//! and it is why this reference must never be offered to a user as anything
//! but a test.

use dialectica_core::identity::{Address, SecretKey};
use dialectica_core::log::SqliteOpLog;
use dialectica_core::membership::MembershipStore;
use dialectica_core::stoa::{Genesis, Policy};

/// The spec that pastes the reference, relative to this crate.
///
/// Read at run time rather than with `include_str!`, so that a moved spec is
/// a test that fails and names the path, not a crate that fails to compile
/// with a message about a missing file nobody connects to the UI suite.
const SPEC: &str = "../../../dialectica-ui/tests/ui/seeded-join.yaml";

/// Where a reference starts. The spec's own paste is the only occurrence of
/// this in the file, and [`reference_in_spec`] refuses the file otherwise.
const REFERENCE_OPENS: &str = "{\"stoa\":\"";

/// The seeded Stoa's founding title, which the spec matches on the preview.
///
/// Plain ASCII, so sitometres' text normalisation (curly quotes, dashes and
/// whitespace folded before an exact match) has nothing to fold, and distinct
/// from anything else the run creates — the run creates nothing.
const TITLE: &str = "A Stoa seeded outside this profile";

/// The creator's secret seed. Public by construction: see the module docs.
const CREATOR_SEED: [u8; 32] = [0x5e; 32];

/// The record the reference must carry.
fn seeded_record() -> Genesis {
    Genesis {
        creator: SecretKey::from_bytes(&CREATOR_SEED)
            .expect("every 32-byte string is an Ed25519 seed")
            .public_key(),
        policy: Policy::Open,
        title: TITLE.to_string(),
    }
}

/// The reference the current core writes for [`seeded_record`], in the
/// encoding `stoa-navigation-view` fixes.
///
/// Used only in failure messages. The tests below check the COMMITTED
/// reference against the core; they never compare it with this, because a
/// reference that still joins under a newer encoder is the compatibility
/// surface working, not drift.
fn regenerated() -> String {
    let record = seeded_record();
    let address = record.address().expect("the seeded record encodes");
    let bytes = record.canonical_bytes().expect("the seeded record encodes");
    format!(
        "{{\"stoa\":\"{}\",\"genesis\":\"{}\"}}",
        address.to_hex(),
        hex::encode(bytes)
    )
}

/// The one reference the spec pastes, exactly as sitometres will type it.
///
/// The spec writes it as a YAML single-quoted scalar, whose content is taken
/// verbatim, and the reference holds no quote, brace or escape of its own, so
/// the text between its opening `{` and the next `}` is the value typed.
/// Anything else — no reference, two, or one reworded into an escaped form —
/// fails here rather than being checked as something the run does not type.
fn reference_in_spec() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SPEC);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the seeded join spec at {}: {e}\n\
             If it moved, update SPEC here; the reference it should paste is\n  {}",
            path.display(),
            regenerated()
        )
    });
    let starts: Vec<usize> = text
        .match_indices(REFERENCE_OPENS)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "{} must paste exactly one reference, written as a single-quoted '{{\"stoa\":…}}'; \
         found {}. The reference this core writes for the seeded record is\n  {}",
        path.display(),
        starts.len(),
        regenerated()
    );
    let rest = &text[starts[0]..];
    let end = rest
        .find('}')
        .unwrap_or_else(|| panic!("the reference in {} is not closed", path.display()));
    rest[..=end].to_string()
}

/// The reference's two fields, as the view's `DStoaReference.parse` reads them.
fn fields(reference: &str) -> (String, String) {
    let v: serde_json::Value = serde_json::from_str(reference)
        .unwrap_or_else(|e| panic!("the spec's reference is not JSON ({e}): {reference}"));
    let get = |k: &str| {
        v.get(k)
            .and_then(|s| s.as_str())
            .unwrap_or_else(|| panic!("the spec's reference has no string `{k}`: {reference}"))
            .to_string()
    };
    (get("stoa"), get("genesis"))
}

fn reply(json: &str) -> serde_json::Value {
    serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("the core answered non-JSON ({e}): {json}"))
}

/// The record decodes under the current encoding, is the seeded one, and is
/// what its address names.
///
/// Decoded and compared as a RECORD, not as bytes against [`regenerated`]: a
/// later encoding that still decodes this one keeps this green, and that is
/// the case a compatibility surface exists for.
#[test]
fn the_committed_reference_is_the_seeded_record_at_its_own_address() {
    let reference = reference_in_spec();
    let (stoa, genesis) = fields(&reference);

    let bytes = hex::decode(&genesis)
        .unwrap_or_else(|e| panic!("the spec's genesis is not hex ({e}): {reference}"));
    let record = Genesis::decode(&bytes).unwrap_or_else(|e| {
        panic!(
            "the committed reference no longer decodes: {e}.\n\
             Every reference written under the encoding it was written under has \
             stopped joining too (see this file's docs before regenerating).\n\
             The current core writes\n  {}",
            regenerated()
        )
    });
    assert_eq!(
        record,
        seeded_record(),
        "the committed record is not the seeded one; the current core writes\n  {}",
        regenerated()
    );

    let address = Address::from_hex(&stoa)
        .unwrap_or_else(|e| panic!("the spec's address is not an address ({e:?}): {reference}"));
    assert!(
        record.matches(&address),
        "the committed address is not the hash of the committed record, so the \
         core would refuse the join; the current core writes\n  {}",
        regenerated()
    );
}

/// The preview's lookup: a peer holding no ops for the Stoa is answered with
/// the founding title as a fallback, which is what the spec's preview step
/// matches on screen.
#[test]
fn the_preview_of_the_committed_reference_falls_back_to_the_seeded_title() {
    let reference = reference_in_spec();
    let (stoa, _) = fields(&reference);

    let answer = reply(&dialectica_core::get_stoa(
        &reference,
        SqliteOpLog::in_memory,
    ));

    assert_eq!(answer.get("error"), None, "getStoa refused it: {answer}");
    assert_eq!(answer["stoa"], stoa.as_str(), "{answer}");
    assert_eq!(
        answer["isGenesisFallback"], true,
        "an empty store holds no metadata op, so the answer must be the fallback: {answer}"
    );
    assert_eq!(answer["title"], TITLE, "{answer}");
}

/// The join: a Stoa the peer was not in is joined, the reply names it with its
/// founding title and record, and the listing then holds it and nothing else.
///
/// Against an EMPTY store, asserted empty first: joining a Stoa already held
/// also succeeds (`stoa-membership`), so a store that already held it would let
/// this pass without the join being the new membership the spec depends on.
#[test]
fn the_core_joins_the_committed_reference_as_a_new_membership() {
    let reference = reference_in_spec();
    let (stoa, genesis) = fields(&reference);
    let mut store = MembershipStore::in_memory().expect("an in-memory membership store opens");

    let before = reply(&dialectica_core::list_stoas(
        r#"{"page":0,"perPage":10}"#,
        &store,
    ));
    assert_eq!(
        before["items"],
        serde_json::json!([]),
        "the store starts empty: {before}"
    );

    let joined = reply(&dialectica_core::join_stoa(&reference, &mut store));
    assert_eq!(joined.get("error"), None, "joinStoa refused it: {joined}");
    assert_eq!(joined["stoa"], stoa.as_str(), "{joined}");
    assert_eq!(joined["foundingTitle"], TITLE, "{joined}");
    assert_eq!(joined["genesis"], genesis.as_str(), "{joined}");

    let after = reply(&dialectica_core::list_stoas(
        r#"{"page":0,"perPage":10}"#,
        &store,
    ));
    let listed: Vec<&str> = after["items"]
        .as_array()
        .unwrap_or_else(|| panic!("the listing has no items: {after}"))
        .iter()
        .filter_map(|item| item["stoa"].as_str())
        .collect();
    assert_eq!(listed, vec![stoa.as_str()], "{after}");
}
