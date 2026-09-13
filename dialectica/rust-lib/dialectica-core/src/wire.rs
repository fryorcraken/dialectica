//! The wire contract: the guard, and the handler bodies behind it.
//!
//! This is the module's public surface (PLAN.md §2.5) — every method takes JSON
//! and returns JSON, and failure is always `{"error":"..."}`. The forum's
//! semantics live in the sibling modules; this one is the boundary they are
//! reached through.
//!
//! Phase 0 wrote this as `core.rs` inside the module crate, against the day it
//! would become the pure inner crate PLAN.md §9 describes. That day is this
//! commit, and the seam held: the move was a rename, not a rewrite.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// The request envelope, deliberately in a file of its own.
///
/// [`Request`]'s guarantee is that a handler holding one went through the
/// envelope check — and a tuple struct's private field is private to its
/// **defining module**, not its defining type. While the type lived in this
/// file, every handler here could write `Request(map)` and skip the check; the
/// claim in its doc comment was false for exactly the population it named. The
/// boundary is the fix, and it holds only while **this file contains no
/// constructor and that file contains no handler**.
mod request;

pub use request::{Request, MAX_REQUEST_BYTES, REQUEST_NOT_AN_OBJECT};

/// The one failure shape (PLAN.md §2.5). Everything that goes wrong comes back
/// through here, so a view has exactly one error branch to render — never a
/// partial success.
pub fn error_json(message: &str) -> String {
    // `json!` escapes the message. Hand-rolled string concatenation would emit
    // malformed JSON for a message containing a quote or a newline, turning a
    // diagnosable error into a parse failure at the view.
    serde_json::json!({ "error": message }).to_string()
}

/// The panic guard. No handler may unwind.
///
/// The SDK ships no `catch_unwind` — verified at the builder's pinned rev and
/// at upstream HEAD (see `docs/PHASE0-FINDINGS.md`). Every generated
/// `extern "C"` dispatch calls straight into author code, so an unwind crosses
/// an `extern "C"` frame, which is undefined behaviour.
///
/// The worse half is not the UB. **PHASE0-FINDINGS §3 measured what actually
/// happens: the module process ABORTS** — `failed to initiate panic, error 5`,
/// SIGABRT — the caller waits out its 20s timeout, and every later call reports
/// `MODULE_NOT_LOADED`. That is why this guard wraps every handler rather than
/// only the ones that look risky.
///
/// An earlier version of this comment predicted mutex poisoning instead: the
/// generated dispatch does hold `INSTANCE.0.lock().unwrap()` across the call, so
/// a surviving panic would poison it. That code is real and is never reached,
/// because the abort comes first. Recorded because the prediction was
/// reasonable and wrong, and the difference matters — poisoning would be
/// recoverable by tolerating it, and an abort is not recoverable at all.
///
/// `AssertUnwindSafe` is load-bearing rather than a silencer: the closure
/// borrows `&mut` state, which is not `UnwindSafe` by default. The assertion we
/// are making is that a handler does not leave *observable* state half-written
/// before panicking — which is a reason to keep handler bodies free of partial
/// mutation, not a reason to drop the guard. Without the guard the alternative
/// is not "safe state", it is a dead module process.
pub fn guarded<F: FnOnce() -> String>(method: &str, f: F) -> String {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(payload) => {
            // `panic!("literal")` yields a `&str` payload; `panic!("{}", x)`
            // yields a `String`. Handling only one silently loses the message
            // for every panic of the other kind — which is most of the
            // interesting ones.
            let detail = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_string());
            error_json(&format!("panic in {}: {}", method, detail))
        }
    }
}

/// `{"version":"X.Y.Z"}`.
pub fn version(crate_version: &str) -> String {
    guarded("version", || {
        serde_json::json!({ "version": crate_version }).to_string()
    })
}

/// `{"payload":<any>}` -> `{"pong":<any>}`.
///
/// Trivial by design, but it validates at the boundary, which is the habit the
/// security posture asks for: inbound JSON is attacker-controlled and is
/// rejected here rather than deeper in.
///
/// # `payload` is the surface's one `<any>` field, and that decides its `null`
///
/// The contract keys a field's `null` reading to its declared type and
/// optionality, and gives three readings. `payload` takes **reading 1**: a field
/// documented as carrying any JSON value carries a `null` through as that value,
/// and that reading takes precedence over the required-field one. So
/// `{"payload":null}` is served as `{"pong":null}` rather than refused —
/// `payload` is required, but for a field whose type admits `null` the `null` is
/// not a malformed parameter, it *is* the parameter.
///
/// That precedence is the part worth stating here rather than leaving to be
/// derived: without it, `payload` is reachable by two readings that disagree, and
/// this method is where they meet. Pinned by
/// `pings_payload_carries_an_explicit_null_through_as_a_value`.
pub fn ping(request: &str) -> String {
    guarded("ping", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        // Note what is NOT here: a `Some(Value::Null)` arm collapsing a null into
        // the missing-field refusal. A `<any>` field's null is a value (reading
        // 1), so the only absence is a genuine one.
        let Some(payload) = parsed.get("payload") else {
            return error_json("missing field: payload");
        };
        serde_json::json!({ "pong": payload }).to_string()
    })
}

/// Panics on purpose, so the guard is provable rather than merely asserted.
///
/// A guard with nothing that exercises it is a claim no gate can see. This
/// method is the thing that makes `panic_probe_returns_the_error_shape` a real
/// test, and it is also how Phase 0 answers "what does a panic in a dispatch
/// handler actually do?" against a running host rather than by inference.
pub fn panic_probe(request: &str) -> String {
    guarded("panic_probe", || {
        panic!("panic_probe was asked to panic with: {}", request)
    })
}

// ─── The capability probe ─────────────────────────────────────────────────
//
// The contract is the `posting-capability` spec; the reasoning behind these
// shapes is in that change's `design.md`. What is repeated here is only what a
// reader of THIS code needs in order not to undo it.

/// What a caller may do right now, and why not if not.
///
/// **The two states are one enum rather than a struct with two `Option`s**, and
/// that is the whole reason this type exists instead of the handler building
/// JSON inline. The contract is
/// `{"canPost":bool, "identity":"…" | "reason":"…"}` — the `|` is exclusive —
/// and §2.5 forbids a reply that is partly a success. A
/// `{ can_post, identity: Option, reason: Option }` can express three states
/// the contract does not have (both, neither, and a `canPost:true` with no
/// identity), and each would then have to be prevented at every construction
/// site.
///
/// CLAUDE.md's standing instruction is to put the complexity in the data
/// structure: here that is an enum with one payload each, so the illegal
/// states cannot be written down. Serialisation is total and the handler has
/// no branch to get wrong.
#[derive(Debug, PartialEq, Eq)]
pub enum Capability {
    /// Posting is possible, under this author address (hex).
    CanPost { identity: String },
    /// Posting is not possible. The reason **names the fix** — the
    /// `posting-capability` spec requires it, and it is the difference between
    /// a view that can help a user and one that can only display
    /// "unlocked: false".
    CannotPost { reason: String },
}

impl Capability {
    /// The wire form. Exactly one of `identity` and `reason` is present, by
    /// construction rather than by a check here.
    pub fn to_json(&self) -> String {
        match self {
            Capability::CanPost { identity } => {
                serde_json::json!({ "canPost": true, "identity": identity }).to_string()
            }
            Capability::CannotPost { reason } => {
                serde_json::json!({ "canPost": false, "reason": reason }).to_string()
            }
        }
    }
}

/// The probe handler: `{}` in, `{"canPost":…}` out.
///
/// # This answers; it does not fail
///
/// **Every state comes back as a `Capability`, including the ones that are
/// plainly errors** — an unreadable keystore, a malformed file, a wrong
/// passphrase. That is a deliberate departure from the usual shape, and the
/// reason is what the caller would otherwise have to do: a view handling both
/// "you cannot post, because X" and "I could not determine whether you can
/// post" has two negative branches, and the second has no sensible rendering.
/// Collapsing them means a view checks exactly one thing.
///
/// §2.5's `{"error":"..."}` is still reachable, and only for the one failure
/// that is not about capability: input this function could not interpret. A
/// malformed *request* is a caller bug; a malformed *keystore* is a user state.
///
/// # What it is given, and why
///
/// Both the unlock state and the Stoa are the caller's to supply. This crate
/// cannot read the environment or know the host's persistence path, and a probe
/// that went looking would be doing discovery at a moment its caller does not
/// control — the adapter has both and passes them in. The `stoa` is needed
/// because there is no such thing as "the" identity: §5.2 gives a user one
/// identity *per Stoa*, so "who would post" has no answer until a Stoa is
/// named.
///
/// # `Fn`, not `FnOnce`
///
/// The spec says the probe is callable repeatedly — a view asks it whenever it
/// renders, not once per process. Review flagged `FnOnce` as making that
/// scenario unsatisfiable.
///
/// **That turned out to be half right, and the correction is worth recording
/// because the obvious reading is wrong.** `FnOnce` is a supertrait of `Fn`,
/// so a `&F` where `F: Fn` satisfies it — the repeatability scenario *was*
/// testable under `FnOnce`, by passing a reference. Verified by reverting the
/// signature and watching the test still pass, twice, including with a
/// capturing closure.
///
/// `Fn` stays because it is the **honest** constraint rather than the minimum
/// one: nothing here consumes the lookup, `FnOnce` said it might, and a caller
/// reading the signature would reasonably conclude it had to build a fresh
/// closure per call. A signature that overstates what it takes is a signature
/// callers work around.
///
/// What this does NOT do is make a mutation detectable — reverting to `FnOnce`
/// leaves the suite green, and the mutation table says so rather than claiming
/// a kill it does not have.
///
/// # The lookup's error is a reason, not a `KeystoreError`
///
/// It was `Result<String, KeystoreError>`, which said the only thing that can stop a
/// user posting is the keystore. That stopped being true when the probe began
/// consulting the path record: "a master key exists and this Stoa has no choice
/// recorded" is a real `CannotPost` state and not a keystore failure at all. The
/// alternative was an `Other(String)` arm on `KeystoreError`, rejected because that
/// enum's arms are documented as distinguishable *so that a reason can name a fix*,
/// and a catch-all carrying another module's failure is what that doctrine exists to
/// prevent. A `String` is what `capability_for` reduced the error to on the very next
/// line anyway.
pub fn get_capabilities(
    request: &str,
    lookup: impl Fn(&crate::identity::Address) -> Result<String, String>,
) -> String {
    guarded("get_capabilities", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(s) => s,
            Err(e) => return e,
        };
        capability_for(&stoa, lookup).to_json()
    })
}

/// The probe's decision, separated from its JSON and its guard.
///
/// Split out because this is the part worth testing directly: the mapping from
/// "what the keystore said" to "what a view is told" is where the requirement
/// that a reason names a fix actually lives, and asserting on it through a JSON
/// string would be asserting on serialisation at the same time.
pub fn capability_for(
    stoa: &crate::identity::Address,
    lookup: impl Fn(&crate::identity::Address) -> Result<String, String>,
) -> Capability {
    match lookup(stoa) {
        Ok(identity) => Capability::CanPost { identity },
        // The reason IS the lookup's own message, not a rewording of it. Where that
        // message comes from `KeystoreError::Display` or `IdentityStoreError::Display`,
        // each already names the fix for its own case — a documented obligation on
        // both, with tests — so paraphrasing here would mean maintaining the same
        // guidance in two places and watching the two drift.
        Err(reason) => Capability::CannotPost { reason },
    }
}

/// The address an op published now would be attributed to, for the probe to report.
///
/// # This exists because two methods were answering "who posts here" differently
///
/// `posting-capability`'s spec requires *"the identity reported SHALL be the one an
/// op published now would be attributed to, derived from the key that would actually
/// sign it"*, and names the failure it is guarding: *"the user sees one handle and
/// posts under another."* That was the live state. `whoAmI` used the path-taking
/// derivation under salt `/dialectica/2/…`; the probe's lookup was still the
/// pathless one under `/dialectica/1/…`, and `identity.rs`'s own test asserts the
/// two schemes **must** disagree. So the onboarding flow showed one address and the
/// posting gate reported another, for one user in one Stoa, with no field in either
/// reply to tell them apart.
///
/// The salt bump was right; what it left behind was this caller. Architecture and
/// security review found it independently, and `proposal.md`'s claim that
/// `posting-capability` is *"not modified, deliberately — the probe's … derivation
/// [is] untouched"* is exactly how it happened: the shape was untouched and the
/// contract was broken.
///
/// # Why it is here rather than in the adapter's closure
///
/// The pathless derivation was chosen in `dialectica/rust-lib/src/lib.rs`, which is
/// `#[cfg(logos_scaffold)]` and therefore never compiled by `cargo test`. Every
/// probe test injects a stub returning a literal, so no test could compare the
/// probe's identity to `whoAmI`'s and none could be written while the choice lived
/// there. Moving the choice into `core` is what makes
/// `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` possible.
///
/// # The record is consulted, and its absence is not an error
///
/// A master key with no recorded choice for this Stoa cannot post *as anyone*: there
/// is no path, so there is no identity, so there is nothing to attribute an op to.
/// That is reported as `CannotPost` with a reason naming the missing choice — the
/// same state `whoAmI`'s fourth row names, so the two methods agree about it rather
/// than one inventing an identity the other does not have.
pub fn posting_identity(
    stoa: &crate::identity::Address,
    keystore: &crate::keystore::Keystore,
    paths: &crate::identity_store::IdentityStore,
) -> Result<String, String> {
    match paths.path_for(stoa) {
        Ok(Some(path)) => Ok(keystore.stoa_address_at_path(stoa, path).to_hex()),
        Ok(None) => Err(NO_CHOICE_FOR_THIS_STOA.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// The secret key a publish into this Stoa signs with.
///
/// # This is the same derivation [`posting_identity`] reports, and that is the
/// whole requirement
///
/// `content-authoring` contracts it directly — *"WHEN the posting-capability
/// probe reports an identity for a Stoa and a post is then published into that
/// Stoa, THEN the published op's author is the identity the probe reported"* —
/// and the probe reports [`posting_identity`], which is
/// `stoa_address_at_path(stoa, recorded_path)`.
///
/// The publish path signed with `keystore.stoa_key(&stoa)` instead: the
/// **pathless** per-Stoa scheme, under a different salt. `identity.rs`'s own
/// test asserts the two schemes must disagree, so a user's posts were signed by
/// an identity neither `getCapabilities` nor `whoAmI` would ever name — the
/// exact failure `posting-capability` calls out as *"the user sees one handle
/// and posts under another"*, one layer deeper, because here it is what actually
/// reaches the network rather than what a screen displays.
///
/// **The spec had decided this and the code had not caught up.** CI carried a
/// named exemption for `keystore.stoa_key(&stoa)` in the adapter, on the
/// reasoning that *"WHICH key a publish signs with is a spec question this gate
/// cannot answer"*, ending *"Delete this exemption when the spec decides."* The
/// scenario above is the spec deciding, so the exemption is gone in the same
/// change as the call it fenced.
///
/// # Why it lives here rather than in the adapter
///
/// The same reason [`posting_identity`] does, and the reason is measured: the
/// choice was in `dialectica/rust-lib/src/lib.rs`, which is
/// `#[cfg(logos_scaffold)]` and compiled by no `cargo test`, by no clippy run
/// and by no fmt run. Nothing could compare the key a publish signs with against
/// the identity the probe reports while the two lived on opposite sides of that
/// boundary. Moving it here is what makes
/// `the_key_a_publish_signs_with_is_the_identity_the_probe_reports` possible.
///
/// # A missing choice is a refusal, not a fallback
///
/// [`posting_identity`] reports `CannotPost` when no path is recorded for this
/// Stoa, because there is no identity to attribute an op to. Signing with
/// *anything* here would contradict that: the probe would say the user cannot
/// post and the publish would succeed, under a key the probe refuses to name.
/// So the same state is the same answer, carrying
/// [`NO_CHOICE_FOR_THIS_STOA`] — one constant, because it is one state.
pub fn publishing_key(
    stoa: &crate::identity::Address,
    keystore: &crate::keystore::Keystore,
    paths: &crate::identity_store::IdentityStore,
) -> Result<crate::identity::SecretKey, String> {
    match paths.path_for(stoa) {
        Ok(Some(path)) => Ok(keystore.stoa_key_at_path(stoa, path)),
        Ok(None) => Err(NO_CHOICE_FOR_THIS_STOA.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// The reason both `getCapabilities` and `whoAmI` give for "a master key exists and
/// this Stoa has no choice recorded".
///
/// One constant because it is one state, and the two methods reporting it in
/// different words would be two methods disagreeing about the user's situation in the
/// one place they are meant to agree. The state itself is what the two-store split
/// creates, and the wording names the fix rather than the fault, per
/// `KeystoreError::Display`'s obligation.
pub const NO_CHOICE_FOR_THIS_STOA: &str =
    "a master key exists but no identity has been chosen for this Stoa; \
     generate a slate and keep one of its candidates";

/// `{"stoa":"<hex>"}` -> whether the user can post there, from the two stores.
///
/// The shape [`get_capabilities`] has, with the *derivation* moved inside so it can
/// be tested. See [`posting_identity`] for why that move was necessary and what it
/// fixed. The openers are closures for the reason every other one in this file is:
/// this crate cannot read the environment or know the host's layout.
pub fn get_capabilities_from_stores(
    request: &str,
    open_keystore: impl Fn() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
    open_paths: impl Fn() -> Result<
        crate::identity_store::IdentityStore,
        crate::identity_store::IdentityStoreError,
    >,
) -> String {
    get_capabilities(request, |stoa| {
        // Each error keeps its OWN type's message rather than being folded into one
        // of them. Adding an `Other(String)` arm to `KeystoreError` so this could
        // return that type was the obvious move and is rejected: that enum's arms
        // are documented as distinguishable so a reason can name a fix, and a
        // catch-all carrying another module's failure is the collapse the
        // distinguishability doctrine exists to prevent.
        let keystore = open_keystore().map_err(|e| e.to_string())?;
        let paths = open_paths().map_err(|e| e.to_string())?;
        posting_identity(stoa, &keystore, &paths)
    })
}

// ─── Onboarding: the slate, keeping one, and who the user is ──────────────
//
// The contract is the `identity-onboarding` spec; the reasoning behind these
// shapes is in that change's `design.md`. What is repeated here is only what a
// reader of THIS code needs in order not to undo it.

/// The `stoa` field, parsed. One job, because **every** handler that takes a Stoa
/// needs it and a second copy would eventually disagree with the first about whether
/// a missing field and a wrong-typed one are the same mistake.
///
/// The `Err` arm is already the wire reply, following `parse_channel_id`: a caller
/// cannot accidentally invent a second error shape while converting one.
///
/// # Six call sites, and for a while it was three
///
/// This was extracted for the three handlers `identity-onboarding` added, and the
/// three that already existed — `get_capabilities`, `list_threads_inner`,
/// `list_threads_from_request` — were left on their own inline copies. Architecture
/// review named the cost precisely: they were behaviourally identical, so nothing
/// failed, and the bill arrives on the next change that tightens the parse (a length
/// bound, a lowercase-hex rule), which would land in one place while three handlers
/// kept the old behaviour and every one of their tests kept passing.
///
/// CLAUDE.md's rule is that the fourth slightly-different copy of a guard is the
/// signal to reshape. The signal was read — the helper exists — and then the reshape
/// stopped at the new call sites. All six now use it.
///
/// # It takes a `&Request`, not a `&Value`, and that is what carries the envelope
///
/// This took a `&serde_json::Value` until `main`'s request envelope merged in. The
/// change of parameter type is the whole reason the merge is not a textual one: a
/// `Value` answers `None` to `get("stoa")` for an **array** exactly as it does for
/// an object with no `stoa`, so this helper could not tell those apart and every
/// handler behind it inherited that blindness. A `&Request` can only have come from
/// [`Request::parse`], so by the time this reads a field the non-object refusal and
/// the size cap have both already happened.
///
/// That makes the six call sites a feature rather than a liability. The envelope's
/// own `design.md` records what its type does **not** buy — "a handler need not hold
/// a `Request` at all", measured against a sixth method that served `[]` with the
/// suite green — and a shared field reader taking `&Request` is the one shape that
/// shrinks that gap: a new Stoa-taking handler cannot reach `stoa` without a
/// `Request` in hand, because this is the only place that reads the field.
fn parse_stoa(parsed: &Request) -> Result<crate::identity::Address, String> {
    match parsed.get("stoa") {
        Some(serde_json::Value::String(s)) => {
            crate::identity::Address::from_hex(s).map_err(|e| error_json(&format!("stoa: {e}")))
        }
        Some(_) => Err(error_json("stoa must be a string")),
        None => Err(error_json("missing field: stoa")),
    }
}

/// The onboarding state that spans two wire calls, and the decision about which
/// master key a slate was offered under.
///
/// # Why this type exists at all
///
/// A slate is not a nonce. It is a **`(master key, nonce)` pair** — the nonce
/// fixes the five *paths* and the master key fixes the five *identities* at those
/// paths. `derive_path` takes only the nonce, so two different master keys and one
/// nonce give the same five paths and five completely different addresses.
///
/// The first version of this change held only the nonce, and the adapter minted a
/// fresh master key on each of the two calls. Every guard fired correctly and the
/// outcome was still wrong: the slate showed candidates of key *A*, the keep wrote
/// key *B*, reported `kept: true`, and named an address the user had never seen.
/// Three reviewers reached it independently. The spec calls storing an identity the
/// user did not choose unrecoverable *"because the choice cannot be recomputed"*,
/// and an address is *"the only unforgeable way to tell two candidates apart"* —
/// so the value the choice was made on was precisely the value that changed.
///
/// The nonce check cannot catch it. Both slates are equally live and the nonce is
/// the same; what differs is a thing the nonce says nothing about.
///
/// # Why the key is held rather than committed to
///
/// The alternative was to carry a commitment to the master key in the slate reply
/// and have the keep refuse when the key it is about to write does not match. That
/// detects the divergence; it does not give the user the identity they chose — it
/// turns a wrong answer into a refusal the user can do nothing about, because the
/// key that produced their slate is already gone.
///
/// Holding the minted keystore for the module's lifetime is what makes the right
/// answer available. It also makes two properties true that the per-call mint made
/// false, and both are user-visible: refreshing a slate now offers **more
/// candidates of one identity's key** rather than candidates of a different key
/// each press, which is what `docs/UI-BRIEF.md` tells a designer the flow does; and
/// keeping a candidate for a *second* Stoa reuses the master key the first keep
/// wrote, which is what makes one master key per install mean anything.
///
/// **The exposure this costs is a root secret in memory for the module's lifetime
/// rather than for one call.** That is the same lifetime a kept keystore's root
/// has — `keep` writes it and every later call opens it — so the window widens
/// only on the fresh-install path, and only until the user keeps or the module
/// stops. Nothing is written: `mint_or_open` writes no file, which is the spec's
/// *"Generating a slate SHALL NOT write to storage"*, and `Keystore`'s root is
/// `Zeroizing`, so the held copy is wiped on drop rather than by a line somebody
/// has to remember.
///
/// # Why it is in `core` and not in the adapter
///
/// It was in the adapter, as a 12-line `master_key` helper, and that is where the
/// defect lived. `dialectica/rust-lib/src/lib.rs` is `#[cfg(logos_scaffold)]` and
/// `build.rs` sets that cfg only when the generated provider exists, which it never
/// does under `cargo test` — so the one function deciding which master key a slate
/// and a keep each saw was compiled out of the only gate that runs any logic. The
/// adapter's own comment says a body there that grows past one line *"is logic no
/// test can reach"*, and it was right.
///
/// What is left in the adapter is the part that genuinely cannot move: the host's
/// directory, and the environment the protection is read from.
#[derive(Default)]
pub struct OnboardingSession {
    /// The keystore this module is working with, opened from disk or minted once.
    ///
    /// `None` until the first call that needs one. Minted at most once per module
    /// lifetime, which is the whole point of the field.
    keystore: Option<crate::keystore::Keystore>,
    /// The nonce of the slate a keep may quote. `None` when no slate is live.
    live_slate: Option<crate::onboarding::SlateNonce>,
}

impl OnboardingSession {
    /// A session with nothing opened and no slate live.
    pub fn new() -> Self {
        Self::default()
    }

    /// The keystore to work with: the one on disk, or one minted and remembered.
    ///
    /// **Minted at most once.** A second call returns the same key, which is the
    /// property the two-call onboarding flow rests on and the one a per-call mint
    /// did not have.
    ///
    /// `open` is tried on **every** call rather than only the first, and that is
    /// deliberate: a keep writes the keystore, so the call after a keep finds a file
    /// where the call before it found none. Preferring the file over the held copy
    /// means the identity a later call reports is the one on disk — the authority —
    /// rather than a minted key that was never written.
    ///
    /// Any error other than "no keystore" propagates. A keystore that exists and
    /// cannot be opened must not be silently replaced by a fresh key, which is how
    /// every identity a user has gets discarded with no error saying so.
    fn keystore_for(
        &mut self,
        open: impl FnOnce() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
    ) -> Result<&crate::keystore::Keystore, crate::keystore::KeystoreError> {
        match open() {
            Ok(ks) => {
                // The file wins over anything held. See above.
                self.keystore = Some(ks);
            }
            Err(crate::keystore::KeystoreError::NotFound) => {
                if self.keystore.is_none() {
                    // `?` rather than an `expect`. Minting asks the OS for entropy,
                    // and this function is reached from a dispatch handler on every
                    // fresh-install slate and keep — so a panic here aborts the
                    // module process rather than failing one call. See
                    // `identity::SecretKey::generate` for what that measured.
                    self.keystore = Some(crate::keystore::Keystore::generate()?);
                }
            }
            Err(e) => return Err(e),
        }
        // Not reachable as `None`: every arm above either sets the field or
        // returns. An error rather than an `expect`, because an `expect` on a
        // handler path is a panic in a module process.
        self.keystore
            .as_ref()
            .ok_or(crate::keystore::KeystoreError::NotFound)
    }

    /// Whether a slate is live, and which. For the adapter's own assertions only.
    pub fn live_slate(&self) -> Option<crate::onboarding::SlateNonce> {
        self.live_slate
    }

    /// Put a specific nonce — or none — in the live slot.
    ///
    /// `#[cfg(test)]` and `pub(crate)`, both load-bearing, following
    /// `Keystore::from_root_for_test`. A keep has to be reachable with a nonce that
    /// is stale, forged or absent, and those states cannot be produced by generating
    /// a slate, because generating one makes its nonce live by definition.
    ///
    /// It is **not** public, and that matters beyond tidiness: a public setter would
    /// let a caller declare a slate live that was never offered, which is the
    /// superseded-selection refusal disabled from outside.
    #[cfg(test)]
    pub(crate) fn set_live_slate_for_test(&mut self, nonce: Option<crate::onboarding::SlateNonce>) {
        self.live_slate = nonce;
    }
}

/// `{"stoa":"<hex>"}` -> a slate of candidate identities.
///
/// # The count is not a parameter, and that is a security property
///
/// The spec requires the number of candidates be *"fixed by the implementation
/// and reported with the set, rather than requested by the caller"*, because a
/// caller-supplied count is *"a number that decides how much key derivation this
/// module performs"*. So there is no `count` field to pass, which is the strongest
/// form of that: the request has nowhere to put one.
///
/// It is still **reported**, as `count`, so a view rendering a slate does not
/// hardcode five.
///
/// # Nothing is written
///
/// The spec: *"Generating a slate SHALL NOT write to storage."* This handler takes
/// the master key and returns JSON; there is no store parameter for it to write
/// to, so the requirement holds by the signature rather than by a line somebody
/// has to not add.
///
/// # The keystore is opened by a closure, and the session decides what to do with it
///
/// `open` is a closure for the reason [`get_capabilities`]' lookup is one: this
/// crate cannot read the environment or know the host's persistence path, and a
/// handler that went looking would be doing discovery at a moment its caller does
/// not control. It hands back the keystore rather than the raw root, so the root is
/// never a value this function names.
///
/// What the closure does **not** decide is what "no keystore yet" means. That is
/// [`OnboardingSession::keystore_for`]'s, and it lives there rather than in the
/// adapter because the adapter is not compiled by `cargo test` — see that type's
/// documentation for what the earlier arrangement cost.
///
/// # The live slate is set here, and only on success
///
/// Recorded after the slate is built, so a derivation failure does not supersede a
/// slate the user is still looking at.
pub fn generate_identity_slate(
    session: &mut OnboardingSession,
    request: &str,
    open: impl FnOnce() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
) -> String {
    guarded("generate_identity_slate", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let slate = {
            let keystore = match session.keystore_for(open) {
                Ok(k) => k,
                Err(e) => return error_json(&e.to_string()),
            };
            match keystore.slate_for(&stoa) {
                Ok(s) => s,
                Err(e) => return error_json(&e.to_string()),
            }
        };
        session.live_slate = Some(slate.nonce);
        slate_json(&slate)
    })
}

/// A slate as the view receives it.
///
/// Pinned by `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
/// against a hardcoded **key set**, for the reason
/// `the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies` gives: a
/// view is written against these exact names and renaming one is a breaking change
/// no type checker would catch.
///
/// The key *set*, not merely each key's presence — an added field fails that test as
/// well as a removed one. This comment previously cited the capability test's reason
/// while the slate test checked only presence, which invited a reader to expect the
/// sibling's strength; the spec-test reviewer measured the gap by adding a
/// `displayName` to every candidate and watching the suite stay green. The spec
/// requires that no candidate carry a display name or a visual mark, so presence-only
/// could not fail on the one scenario this shape most needs to be held to.
///
/// **`path` is present**, and its presence is a decision rather than an oversight.
/// It is not secret — the spec says the record *"reveals nothing that a published
/// identity does not already reveal"* — and a view that can show the user which
/// path they are about to keep is a view that can render the recovery warning
/// truthfully. What is NOT here is any secret, which the spec requires by name.
fn slate_json(slate: &crate::onboarding::Slate) -> String {
    let candidates: Vec<serde_json::Value> = slate
        .candidates
        .iter()
        .map(|c| {
            serde_json::json!({
                "index": c.index,
                "path": c.path,
                "address": c.address.to_hex(),
                "publicKey": c.public_key.to_hex(),
            })
        })
        .collect();
    serde_json::json!({
        "slate": slate.nonce.to_hex(),
        "count": candidates.len(),
        "candidates": candidates,
    })
    .to_string()
}

/// What keeping a candidate needed from storage, and what it produced.
///
/// **An enum with one payload each rather than a struct of `Option`s**, following
/// [`Capability`] and for the identical reason: the contract has exactly two
/// shapes, and a struct could express states it does not have — a success with no
/// identity, or a failure with one.
#[derive(Debug, PartialEq, Eq)]
pub enum Kept {
    /// The identity is stored. Carries what it is, and whether the master key was
    /// encrypted at rest.
    Stored {
        address: String,
        public_key: String,
        path: u32,
        encrypted: bool,
    },
    /// Nothing was stored. The reason names the fix.
    Refused { reason: String },
}

impl Kept {
    /// The wire form. Exactly one of the two shapes, by construction.
    pub fn to_json(&self) -> String {
        match self {
            Kept::Stored {
                address,
                public_key,
                path,
                encrypted,
            } => serde_json::json!({
                "kept": true,
                "address": address,
                "publicKey": public_key,
                "path": path,
                "encrypted": encrypted,
            })
            .to_string(),
            Kept::Refused { reason } => {
                serde_json::json!({ "kept": false, "reason": reason }).to_string()
            }
        }
    }
}

/// What a keep needs from the world, gathered so the decision below has one
/// argument rather than four.
///
/// A struct rather than four parameters because `keep_selection` would otherwise be
/// a function whose call sites differ only in argument order — and the path, the
/// unlock and the record are a unit: they are the state a keep acts on.
///
/// **The keystore is not here.** It belongs to [`OnboardingSession`], because the
/// key a keep writes must be the key the slate was derived from and a caller that
/// could pass a different one is a caller that can reach the defect this change
/// fixed. Taking it from the session rather than from the argument is that made
/// unrepresentable.
pub struct KeepTargets<'a> {
    /// Where the master key goes.
    pub keystore_path: &'a std::path::Path,
    /// How it is protected. The spec deliberately does not settle where this
    /// comes from; what it requires is that whichever protection applies be
    /// recorded in the file and reportable, which [`Kept::Stored`]'s `encrypted`
    /// discharges.
    pub unlock: &'a crate::keystore::Unlock,
    /// The record of chosen paths.
    pub paths: &'a crate::identity_store::IdentityStore,
}

/// `{"stoa":"…","slate":"…","index":N}` -> the identity that was kept.
///
/// # The selection is checked against the slate it was made against
///
/// The request carries the slate's nonce, and it must equal the live one. That is
/// how the spec's *"A selection made against a superseded set is refused"* is met,
/// and the superseded case is the **same code path** as a nonce that never
/// existed — so there is no second path to get wrong. See
/// [`crate::onboarding`]'s module documentation for why the slate is a nonce.
///
/// # Nothing is coerced
///
/// An out-of-range index is refused, never clamped. The spec is explicit that
/// coercing *"would store an identity the user did not choose — which is
/// unrecoverable, because the choice cannot be recomputed"*.
///
/// # The write order is the atomicity story
///
/// Keystore first, path record second. The keystore write is the irreversible half
/// and writes atomically, so a failure there leaves nothing anywhere. The reverse
/// order would leave a recorded path naming a master key that does not exist.
/// `design.md` records what this does and does not claim.
///
/// # The second-keep refusal is the path record's, not the keystore's
///
/// The first version of this used `Keystore::create`'s `AlreadyExists` as the
/// refusal for a second keep. That is one mechanism doing two jobs, and review
/// measured what the second one broke: the keystore is **one file per install**, so
/// a user who kept an identity in Stoa A and then tried to keep one in Stoa B was
/// refused at `create` before `record_path` was ever called — with a reason naming a
/// keystore they did not know they had. `chosen_paths` could therefore never hold a
/// second row through any wire call, which made the spec's *"Distinct choices for
/// distinct Stoas are recorded separately"* unreachable through the API, and made
/// `design.md`'s claim that the primary key discharges that scenario false.
///
/// So the two refusals are separated, each to the thing that actually knows:
///
/// - **A master key already on disk** is not an error. It is the expected state for
///   every Stoa after the first, and the keystore is reused rather than rewritten.
/// - **A choice already recorded for this Stoa** is the refusal, and it comes from
///   `chosen_paths`' primary key — the same structural refusal `record_path`'s doc
///   comment already argued for, now actually load-bearing.
pub fn keep_identity(
    session: &mut OnboardingSession,
    request: &str,
    open: impl FnOnce() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
    targets: KeepTargets<'_>,
) -> String {
    guarded("keep_identity", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let nonce = match parsed.get("slate") {
            Some(serde_json::Value::String(s)) => {
                match crate::onboarding::SlateNonce::from_hex(s) {
                    Ok(n) => n,
                    // A malformed nonce is a malformed REQUEST, so it is §2.5's error
                    // shape rather than a `Kept::Refused` — the same line
                    // `get_capabilities` draws between a caller bug and a user state.
                    Err(e) => return error_json(&format!("slate: {e}")),
                }
            }
            Some(_) => return error_json("slate must be a string"),
            None => return error_json("missing field: slate"),
        };
        let index = match parse_index(&parsed, "index") {
            Ok(Some(i)) => i,
            // Absent rather than defaulted to 0. A default here would keep the
            // first candidate for a caller who named none, which is storing an
            // identity nobody chose.
            Ok(None) => return error_json("missing field: index"),
            Err(e) => return e,
        };

        // Read before the keystore is taken, because taking it borrows the session
        // mutably. The value is a `Copy` 32 bytes, so this is a read and not a
        // second source of truth.
        let live = session.live_slate;
        let keystore = match session.keystore_for(open) {
            Ok(k) => k,
            Err(e) => return error_json(&e.to_string()),
        };
        keep_selection(&stoa, nonce, index, live, keystore, targets).to_json()
    })
}

/// The keep's decision, separated from its JSON and its guard.
///
/// Split out for the reason [`capability_for`] is: the mapping from "what the
/// stores said" to "what a view is told" is where the requirements actually live,
/// and asserting on it through a JSON string would be asserting on serialisation
/// at the same time.
///
/// The keystore is a parameter here rather than reached through the session,
/// because this function is the *decision* and the session is state — separating
/// them is what lets a test hand this one a specific key and assert on what it did
/// with it.
pub fn keep_selection(
    stoa: &crate::identity::Address,
    nonce: crate::onboarding::SlateNonce,
    index: usize,
    live_nonce: Option<crate::onboarding::SlateNonce>,
    keystore: &crate::keystore::Keystore,
    targets: KeepTargets<'_>,
) -> Kept {
    use crate::onboarding::OnboardingError;

    let refused = |e: OnboardingError| Kept::Refused {
        reason: e.to_string(),
    };

    // The nonce check comes FIRST, before anything is derived or written. A
    // selection against a slate that is not live must cost nothing.
    match live_nonce {
        None => return refused(OnboardingError::NoLiveSlate),
        Some(live) if live != nonce => return refused(OnboardingError::NonceIsNotTheLiveSlate),
        Some(_) => {}
    }

    // Reproduced from the nonce rather than looked up — see `crate::onboarding`.
    // This is the SAME keystore the slate was derived from, because the session
    // holds it across the two calls; when it was minted per call, this line
    // reproduced the five paths against a different key and the candidate it
    // returned was one the user had never been shown.
    let slate = match keystore.slate_from_nonce(stoa, nonce) {
        Ok(s) => s,
        Err(e) => return refused(e),
    };
    let candidate = match slate.candidate(index) {
        Ok(c) => c,
        Err(e) => return refused(e),
    };

    // THE ORDER. The keystore is the irreversible half and writes atomically, so a
    // failure there leaves nothing anywhere.
    //
    // `create` where no file exists, and nothing where one does. A master key
    // already on disk is the expected state for every Stoa after the first, so it
    // is not a refusal — see this function's caller for why using `AlreadyExists`
    // as the second-keep guard made a second Stoa unreachable.
    //
    // `encrypted` is settled HERE rather than at the end, because the truthful
    // source differs between the two branches and only this code knows which
    // branch it took. Deciding it below would mean re-deriving that, which is the
    // second copy of a fact that CLAUDE.md's guard rule is about.
    let encrypted = if targets.keystore_path.exists() {
        // This call wrote nothing, so the protection that is true is the FILE's,
        // and reading it is the only way to know. Note this is not the re-read the
        // `design.md` decision rejects: that one is about re-reading a file this
        // call just wrote, where the value this code used is the authority. Here
        // there is no value this code used — the file predates the call.
        match crate::keystore::Keystore::is_encrypted(targets.keystore_path) {
            Ok(v) => v,
            // The keystore is on disk and unreadable. Refusing rather than
            // guessing: a keep that reported an identity while unable to tell
            // whether its master key is protected has answered a question it does
            // not know the answer to.
            Err(e) => {
                return Kept::Refused {
                    reason: e.to_string(),
                }
            }
        }
    } else {
        if let Err(e) = keystore.create(targets.keystore_path, targets.unlock) {
            return Kept::Refused {
                reason: e.to_string(),
            };
        }
        // Taken from the unlock this keep USED, not from re-reading the file.
        // Re-reading would report the protection of whatever is at the path now,
        // which on a directory an attacker can write to is not necessarily the
        // file just written. The value that is true is the one this code used.
        matches!(targets.unlock, crate::keystore::Unlock::Passphrase(_))
    };

    // The refusal for a second choice in ONE Stoa. `chosen_paths`' primary key
    // does the refusing, so it is a property of the schema rather than a branch
    // here — and it is per-Stoa, which is the scope the spec asks for and the
    // keystore's one-file-per-install scope could not express.
    if let Err(e) = targets.paths.record_path(stoa, candidate.path) {
        return Kept::Refused {
            reason: e.to_string(),
        };
    }

    Kept::Stored {
        address: candidate.address.to_hex(),
        public_key: candidate.public_key.to_hex(),
        path: candidate.path,
        encrypted,
    }
}

/// Who the user is in a Stoa, or why there is nobody.
///
/// **An enum with one payload each**, following [`Capability`]. The spec requires
/// the reply carry *"an identity or a reason, never both and never neither —
/// matching the posting probe's shape rather than introducing a second convention
/// for the same job"*.
#[derive(Debug, PartialEq, Eq)]
pub enum Whoami {
    /// There is an identity.
    Identity {
        address: String,
        public_key: String,
        path: u32,
        /// Whether recovering this identity needs more than the master key.
        ///
        /// **Always `true` in this change**, because no export or remote backup
        /// exists — so the recorded path lives only in local storage and losing
        /// that store loses the identity even with the master key preserved. The
        /// spec confines the requirement to *"what is checkable now: that the
        /// module reports the unbacked state"*.
        ///
        /// A boolean rather than only prose, so the change that implements backup
        /// flips a value rather than changing a shape.
        recovery_needs_the_record: bool,
    },
    /// There is nobody. The reason names the fix.
    Nobody { reason: String },
}

impl Whoami {
    /// The wire form. Exactly one of the two shapes, by construction.
    pub fn to_json(&self) -> String {
        match self {
            Whoami::Identity {
                address,
                public_key,
                path,
                recovery_needs_the_record,
            } => serde_json::json!({
                "hasIdentity": true,
                "address": address,
                "publicKey": public_key,
                "path": path,
                "recoveryNeedsTheRecord": recovery_needs_the_record,
            })
            .to_string(),
            Whoami::Nobody { reason } => {
                serde_json::json!({ "hasIdentity": false, "reason": reason }).to_string()
            }
        }
    }
}

/// `{"stoa":"<hex>"}` -> who the user is there.
///
/// # This is a different question from whether posting is possible
///
/// The spec is explicit, and the two *"can honestly disagree: a stored identity
/// whose keystore permissions are too open is a real identity that cannot
/// currently be used."* A caller with only the posting probe would have to render
/// "you are nobody" to a user who has an identity and a fixable problem.
///
/// This handler therefore reports the identity where one is recorded and the
/// keystore opens, and a **distinguishable reason** in each of the four ways that
/// can fail: the keystore does not open, the record does not open, the record read
/// fails, and — the one the two-store split creates — a master key with no recorded
/// path for this Stoa. That last one's reason names the record, so it does not read
/// as "you are nobody", and it is the same string `getCapabilities` gives for the
/// same state.
pub fn who_am_i(
    request: &str,
    master: impl Fn() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
    paths: impl Fn() -> Result<
        crate::identity_store::IdentityStore,
        crate::identity_store::IdentityStoreError,
    >,
) -> String {
    guarded("who_am_i", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(s) => s,
            Err(e) => return e,
        };
        whoami_for(&stoa, master, paths).to_json()
    })
}

/// The identity question's decision, separated from its JSON and its guard.
///
/// Split out for the reason [`capability_for`] is.
///
/// **Every state is an answer, never the error shape**, following the posting
/// probe: a caller handling both "you are nobody, because X" and "I could not
/// determine who you are" has two negative branches and the second has no
/// sensible rendering. §2.5's error shape stays reachable for the one failure that
/// is not about identity — a request this code could not interpret — which is
/// [`who_am_i`]'s job rather than this one's.
pub fn whoami_for(
    stoa: &crate::identity::Address,
    master: impl Fn() -> Result<crate::keystore::Keystore, crate::keystore::KeystoreError>,
    paths: impl Fn() -> Result<
        crate::identity_store::IdentityStore,
        crate::identity_store::IdentityStoreError,
    >,
) -> Whoami {
    // The keystore is asked first, because "there is no master key" is the state a
    // fresh install is in and it needs no record consulted to establish. Asking
    // the record first would report a missing record for a user who has no
    // identity at all, which sends them to fix the wrong thing.
    let keystore = match master() {
        Ok(k) => k,
        // The reason IS the error's message. `KeystoreError::Display` already
        // names the fix for each case with a test holding it to that, so
        // paraphrasing here would maintain the same guidance twice and watch the
        // two drift — the argument `capability_for` records.
        Err(e) => {
            return Whoami::Nobody {
                reason: e.to_string(),
            }
        }
    };
    let store = match paths() {
        Ok(s) => s,
        Err(e) => {
            return Whoami::Nobody {
                reason: e.to_string(),
            }
        }
    };
    let path = match store.path_for(stoa) {
        Ok(Some(p)) => p,
        // The state the two-store split creates: a master key exists and this Stoa
        // has no choice recorded. Named as such rather than reported as "no
        // identity", because the fix is different — choose one here, not create a
        // key.
        Ok(None) => {
            return Whoami::Nobody {
                // The same constant `getCapabilities` reports for this state, so
                // the two methods cannot describe one situation in two ways.
                reason: NO_CHOICE_FOR_THIS_STOA.to_string(),
            };
        }
        Err(e) => {
            return Whoami::Nobody {
                reason: e.to_string(),
            }
        }
    };

    let public_key = keystore.stoa_public_key_at_path(stoa, path);
    Whoami::Identity {
        address: public_key.address().to_hex(),
        public_key: public_key.to_hex(),
        path,
        // Always true in this change: no export or remote backup exists, so the
        // record lives only here. See the field's own documentation.
        recovery_needs_the_record: true,
    }
}

// ─── The feed ─────────────────────────────────────────────────────────────

/// `{"stoa":"…", "page":N, "perPage":N, "includeHidden":bool}` -> one page.
///
/// The reply is the ecosystem's pagination shape —
/// `{"items":[…],"page":N,"hasMore":bool}` — and this is the first method in the
/// project to implement it, so it sets the precedent §9.1 says it would.
///
/// # There is no `order` parameter, and that is the decision
///
/// §9.1 proposes `order` with `new` and `active`, and then records that both are
/// defined by a Lamport timestamp that does not reach us, so both would today be
/// served as ascending op id. **An accepted-but-degraded parameter is a method
/// telling its caller a falsehood** — §9.1's own words are that "a view asking
/// for `top` and silently getting `new` has been told a falsehood no test will
/// catch", and the same objection applies with equal force to `new` itself.
///
/// So the method serves the one order core can honestly compute and does not
/// take an argument naming it. Adding a second ordering later adds the parameter
/// then, when there is a second answer for it to select between. See
/// [`crate::feed`] for what "convergent" claims and the much larger thing it
/// does not.
///
/// # Every argument is optional except the Stoa
///
/// `page` defaults to 0 and `perPage` to the module's default, because a view
/// rendering a first page should not have to spell both. The Stoa has no
/// sensible default — §5.2 makes identity per-Stoa, so "which Stoa" is not a
/// question this peer can answer on the caller's behalf.
///
/// # The genesis record is a parameter, because authority cannot be guessed
///
/// [`Moderators::of`](crate::moderation::Moderators::of) is the only way to
/// build a moderator set and it takes a genesis record, which is the fail-closed
/// property made structural. This handler inherits it: a caller with no genesis
/// record cannot ask for a feed, rather than getting one with moderation
/// silently not applied.
pub fn list_threads<L: crate::log::OpLog>(
    request: &str,
    log: &L,
    genesis: &crate::stoa::Genesis,
) -> String {
    guarded("list_threads", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        list_threads_inner(&parsed, log, genesis)
    })
}

/// Decode a genesis record from a request's hex form as the verified pair it and
/// the address make.
///
/// # Why the caller supplies the record at all
///
/// [`Moderators::of`](crate::moderation::Moderators::of) needs a genesis record
/// and `list_threads` has nowhere else to get one: a membership now retains the
/// record per Stoa ([`crate::membership::MembershipStore::get`]), so "there is
/// nowhere else" is no longer true — but this capability deliberately does not
/// read it yet, which is a scope decision rather than an impossibility. Until it
/// does, the record travels with the request.
///
/// **That is not a weakening, because the record is self-authenticating.** §4.8:
/// an address *is* the hash of the genesis record, so a wrong or tampered record
/// fails to match the address it claims. A caller cannot use this to install
/// themselves as a Stoa's moderator: changing the creator changes the record,
/// which changes the address, which no longer matches the Stoa whose posts are
/// being read.
///
/// # It returns a `Membership`, so the verification happens exactly once
///
/// This used to verify here and return a bare `Genesis`, which
/// [`crate::membership::MembershipStore::join`] then verified again — two guards
/// on one predicate, and **no test could tell them apart**: deleting either left
/// the suite green (`findings/spec-test.md` entry 2, and a re-run measuring
/// 550/550 for the wire's half). Returning the verified pair means a caller that
/// wants to store what it decoded already holds the only type `join` accepts,
/// with nothing left to re-check and nothing left to forget.
pub fn genesis_for(
    parsed: &Request,
    stoa: &crate::identity::Address,
) -> Result<crate::membership::Membership, String> {
    let hex_str = match parsed.get("genesis") {
        Some(serde_json::Value::String(s)) => s,
        Some(_) => return Err(error_json("genesis must be a string")),
        None => return Err(error_json("missing field: genesis")),
    };
    // BEFORE the decode. `hex::decode` allocates `hex_str.len() / 2` bytes from
    // a length the caller chose, and a genesis record has a known maximum — so a
    // 64 MiB hex string can be refused for nothing rather than decoded into a
    // 32 MiB `Vec` that `Genesis::decode` then rejects. The bound comes from
    // `stoa` rather than being spelled out here: the largest record the format
    // can hold is that module's knowledge, and a number copied over would drift
    // from it silently.
    if hex_str.len() > crate::stoa::MAX_CANONICAL_BYTES * 2 {
        return Err(error_json(&format!(
            "genesis is {} hex characters, over the {} the format allows",
            hex_str.len(),
            crate::stoa::MAX_CANONICAL_BYTES * 2
        )));
    }
    let bytes = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(_) => return Err(error_json("genesis is not valid hex")),
    };
    let genesis = match crate::stoa::Genesis::decode(&bytes) {
        Ok(g) => g,
        Err(e) => return Err(error_json(&format!("genesis: {e}"))),
    };
    crate::membership::Membership::verified(stoa, &genesis).map_err(|e| error_json(&e.to_string()))
}

/// The feed read, from a request that is already parsed.
///
/// **Takes a `&Request` rather than a `&str`, and that is the fix to a double
/// parse rather than a tidy-up.** `list_threads_from_request` parsed the request
/// to read `stoa` and `genesis`, then handed the raw `&str` on to
/// [`list_threads`], which parsed it again — two `Value` trees live at once, two
/// nested `guarded` frames, one call. Harmless in output and not harmless in
/// cost: it doubled the price of the very request-size lever
/// [`MAX_REQUEST_BYTES`] exists to close, on the one path that already holds the
/// larger of the two payloads.
///
/// The shape that prevents it recurring is the signature. A `&str` here is an
/// invitation to parse; a `&Request` can only have come from a parse that already
/// happened, so the second one is not merely discouraged but unspellable without
/// widening this signature on purpose.
///
/// **No `guarded` frame of its own**, for the same reason: the two public entry
/// points each carry one, and a third nested inside them would catch nothing
/// either of them does not. `guarded` is idempotent, so the old nesting was
/// harmless — but "two frames for one call" is the kind of thing that reads as
/// intent and gets copied.
fn list_threads_inner<L: crate::log::OpLog>(
    parsed: &Request,
    log: &L,
    genesis: &crate::stoa::Genesis,
) -> String {
    let stoa = match parse_stoa(parsed) {
        Ok(s) => s,
        Err(e) => return e,
    };

    // The Stoa asked for must be the one the genesis record names, or the
    // moderator set being applied governs a different Stoa than the posts
    // being filtered. That is check 3 of `moderation.rs`'s three, at the
    // one place a caller could otherwise pair them wrongly.
    let genesis_address = match genesis.address() {
        Ok(a) => a,
        Err(e) => return error_json(&format!("genesis: {e}")),
    };
    if genesis_address != stoa {
        return error_json("the genesis record does not describe the Stoa this feed was asked for");
    }

    let moderators = match crate::moderation::Moderators::of(genesis) {
        Ok(m) => m,
        Err(e) => return error_json(&format!("genesis: {e}")),
    };

    // A present-but-wrong-typed field is a different mistake from an absent
    // one, and a negative or fractional page is neither — each is refused by
    // name rather than coerced, because coercing would answer a question the
    // caller did not ask.
    let page = match parse_index(parsed, "page") {
        Ok(v) => v.unwrap_or(0),
        Err(e) => return e,
    };
    let per_page = match parse_index(parsed, "perPage") {
        Ok(v) => crate::feed::clamp_per_page(v),
        Err(e) => return e,
    };

    // The contract's reading 2 again, and this is the field the contract
    // names as its worked example: an optional flag whose `null` reads as
    // absent BECAUSE `false` is the restrictive default. Hidden content stays
    // excluded, so no caller reaches a wider answer by naming the field with
    // no value.
    //
    // Flip the default to `true` and this arm becomes the authorisation
    // bypass the contract's `SHALL NOT` forbids — the `null` would have to be
    // refused as a wrong type instead. See `parse_index`'s doc for the full
    // statement of the limit; it is one rule with two instances, not two
    // local habits.
    let include_hidden = match parsed.get("includeHidden") {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::Bool(b)) => *b,
        Some(_) => return error_json("includeHidden must be a boolean"),
    };

    match crate::feed::list_threads(log, &moderators, &stoa, page, per_page, include_hidden) {
        Ok(page) => feed_page_json(&page),
        // §11.1 obligation 5: a storage failure is the error shape and NEVER
        // an empty feed. The two mean opposite things and render identically
        // if this arm is ever softened.
        Err(e) => error_json(&e.to_string()),
    }
}

/// The feed handler as the module actually calls it: genesis record included.
///
/// [`list_threads`] takes a decoded [`Genesis`](crate::stoa::Genesis) because
/// that is the shape worth testing directly — the moderator set is the thing
/// under test and threading a hex string through every fixture would be
/// asserting on decoding at the same time. This is the one-argument form the
/// adapter forwards to, and it is thin on purpose: parse, verify, delegate.
///
/// The `store` closure supplies the log. The adapter holds a persistence path
/// and opens a store per call; passing a closure rather than a path keeps this
/// crate free of any opinion about where storage lives, which is the same reason
/// [`get_capabilities`] takes a lookup.
pub fn list_threads_from_request<L: crate::log::OpLog>(
    request: &str,
    store: impl FnOnce() -> Result<L, crate::log::OpLogError>,
) -> String {
    guarded("list_threads", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(s) => s,
            Err(e) => return e,
        };
        // The verified pair; this path wants only the record half.
        let genesis = match genesis_for(&parsed, &stoa) {
            Ok(m) => m.genesis,
            Err(e) => return e,
        };
        // Opening the store is itself fallible, and a failure here is §2.5's
        // error shape rather than an empty feed — the same rule the read path
        // follows, applied one step earlier where it is just as easy to get
        // wrong.
        let log = match store() {
            Ok(l) => l,
            Err(e) => return error_json(&e.to_string()),
        };
        // The request this already holds, not the raw `&str` again. Handing the
        // string to `list_threads` parsed it a second time — two `Value` trees
        // live at once, two nested `guarded` frames, one call. The `&Request`
        // signature on `list_threads_inner` is what makes the mistake
        // unspellable rather than merely fixed.
        list_threads_inner(&parsed, &log, &genesis)
    })
}

/// A non-negative integer field, absent, or a refusal already in the wire shape.
///
/// Separated out because its **three** callers are the same parsing job with the
/// same three failure modes, and a second copy would eventually disagree with the
/// first about whether `-1` is an error or a zero. The callers are `page` and
/// `perPage` in the feed, and `index` in [`keep_identity`].
///
/// # The three callers do not have the same stakes, and the severe one governs
///
/// This comment used to name only the two pagination callers and argue entirely in
/// their terms — *"a page of -1 is not a page"*. Readability review pointed out
/// what that costs: a reader arriving from `keep_identity` asks why this refuses
/// rather than coerces and is told about serving the wrong page of a feed.
///
/// The real answer is the `index` caller's, and it is much stronger. A coerced
/// index keeps the **first candidate** for a caller who named something else, which
/// stores an identity nobody chose — an outcome the spec calls unrecoverable,
/// *"because the choice cannot be recomputed"*. CLAUDE.md's named tell is "do not
/// let a function quietly acquire a second caller with different needs"; the needs
/// here differ in consequence, and the shared guard has to be written for the worst
/// of them.
///
/// The name is also now wrong in a small way worth flagging rather than churning:
/// `index` is what the slate caller's *field* is called, meaning a slate position,
/// while this function's name came from pagination. Renaming it touches three
/// handlers for no behaviour change and is left for whoever next has a reason to
/// touch them.
///
/// # WHY AN EXPLICIT `null` IS ABSENT HERE, AND WHEN COPYING THAT IS WRONG
///
/// [`Request::get`] distinguishes `{"page":null}` from `{}` faithfully —
/// `Some(Null)` against `None` — and this reader deliberately collapses them.
/// That is the contract's **reading 2**: an optional field treats a `null` as
/// absent and acts on its restrictive default. `page` defaults to 0 and
/// `perPage` to the module's own value, so a null-sending caller gets strictly no
/// more than it would have got by omitting the field.
///
/// **The restrictive direction is the licence, and it does not travel with the
/// spelling.** The contract states the limit as a `SHALL NOT`: a field must not
/// read `null` as absent where the resulting default is the *permissive* choice,
/// and such a field refuses the `null` as a wrong type instead (reading 3). So
/// this spelling is the template the next optional field gets written from, and
/// the property that makes it safe is not in the spelling.
///
/// A future `asModerator`, `includeRemoved` or `bypassPolicy` written as
/// `None | Some(Null) => <permissive default>` would let `{"bypassPolicy":null}`
/// reach the permissive branch by naming a field with no value, while a
/// presence-checking validator upstream sees the field as set — the two
/// disagreeing about whether the caller asked for anything. **Before copying this
/// match arm, check which way your default leans; if it leans permissive, the
/// contract requires you to refuse the null rather than default it.**
fn parse_index(parsed: &Request, field: &str) -> Result<Option<usize>, String> {
    match parsed.get(field) {
        // A null reads as absent HERE because the default it falls to is the
        // restrictive one. That is the load-bearing half, not the collapse — see
        // the doc above before copying this arm to a field whose default widens
        // what the caller may see.
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => match n.as_u64() {
            // `as_u64` refuses a negative and a fractional number, which is
            // exactly the set that should be refused. For `page`, clamping -1 to 0
            // would serve the first page to a caller who asked for something
            // impossible; for `index`, it would keep the first candidate, which is
            // storing an identity the user did not choose.
            //
            // **It refuses by SPELLING rather than by value, and the message says
            // so.** `1e2` is JSON for exactly 100 and `0.0` for exactly 0 — both
            // non-negative, both whole — and `as_u64` returns `None` for each,
            // because serde parses any number carrying a `.` or an `e` as `f64`.
            // Several JSON serialisers emit `1e2` for 100, so this is a spelling a
            // legitimate caller can send. The ACCEPTANCE is deliberately
            // unchanged: widening it to accept an exactly-integral float is a
            // contract question the spec does not answer. Filed rather than fixed —
            // a caller told the truth can restring its number today.
            //
            // `try_from` rather than `as usize`. On a 64-bit target the cast is
            // lossless and this is belt-and-braces; on a 32-bit one it TRUNCATES,
            // so `{"index": 4294967296}` would arrive as `0` and keep candidate 0 —
            // exactly the coercion the paragraph above forbids, reached by a cast
            // rather than by a decision. CI builds no 32-bit target, so the refusal
            // is unreachable today and the point is that the property no longer
            // depends on which target it is built for.
            //
            // `stoa-lifecycle` reached the same conclusion from the other end, and
            // it is the same rule `MembershipStore` states two files away — *"`try_from`
            // rather than `as` so that a platform where it could fail says so instead
            // of wrapping."* Its `findings/security.md` entry 4 names the consequence
            // for `page` specifically: a narrowed `{"page":4294967296}` would serve
            // page 0 while reporting `"page":0`, which is the wrong answer
            // `a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page`
            // exists to prevent one layer down.
            Some(v) => match usize::try_from(v) {
                Ok(i) => Ok(Some(i)),
                Err(_) => Err(error_json(&format!(
                    "{field} is larger than this build can represent"
                ))),
            },
            None => Err(error_json(&format!(
                "{field} must be a non-negative integer written without a decimal \
                 point or exponent"
            ))),
        },
        Some(_) => Err(error_json(&format!("{field} must be a number"))),
    }
}

/// One sanitised string as the view receives it.
///
/// **An object rather than a bare string, always** — even when nothing was
/// found. A shape that was sometimes a string and sometimes an object would make
/// every view branch on the type before rendering, and the branch would be
/// written once and forgotten at the second call site.
fn sanitised_json(s: &crate::sanitise::Sanitised) -> serde_json::Value {
    serde_json::json!({
        "text": s.text,
        "removed": s.removed,
        "marked": s.marked,
    })
}

/// The pagination shape, built in one place.
fn feed_page_json(page: &crate::feed::FeedPage) -> String {
    let items: Vec<serde_json::Value> = page
        .items
        .iter()
        .map(|row| {
            serde_json::json!({
                "thread": row.thread,
                "currentVersion": row.current_version,
                "author": row.author,
                "body": sanitised_json(&row.body),
                "attachments": row.attachments.iter().map(sanitised_json).collect::<Vec<_>>(),
                "isRevised": row.is_revised,
                "isHidden": row.is_hidden,
            })
        })
        .collect();
    serde_json::json!({
        "items": items,
        "page": page.page,
        "hasMore": page.has_more,
    })
    .to_string()
}

// ─── Creating, joining and listing Stoas ──────────────────────────────────
//
// The contract is the `stoa-membership` spec; the reasoning is in that change's
// `design.md`. What is repeated here is only what a reader of THIS code needs in
// order not to undo it.

/// The founding title's field name on the wire, in one place.
///
/// **`foundingTitle` and never `title`**, and the name is the requirement rather
/// than a preference. `stoa-metadata` puts the *current* title in a
/// moderator-signed op that nothing resolves yet, so every title this capability
/// reports is the founding one — what the Stoa was created as, possibly long ago.
///
/// The alternative shape, `{"title":…,"isFounding":true}`, was rejected: it leaves
/// a view one forgotten branch away from rendering a founding title as current,
/// and makes `title` mean two things depending on a sibling field. A name that
/// cannot be misread costs nothing.
///
/// When metadata resolution lands it adds `title` and `isGenesisFallback` **beside**
/// this field rather than redefining it — PLAN.md §9.1's own shape for `getStoa`.
const FOUNDING_TITLE: &str = "foundingTitle";

/// What a create or a join reports about the Stoa it settled on.
///
/// One function rather than two spellings, because the spec requires both replies
/// name their title as founding and a second call site is how one of them
/// eventually spells it differently. The `policy` is here and deliberately not on
/// a list item: the spec fixes a list item as carrying the address and the
/// founding title, and widening the paginated envelope's item shape is a decision
/// for whoever needs it.
fn stoa_reply(stoa: &crate::identity::Address, genesis: &crate::stoa::Genesis) -> String {
    serde_json::json!({
        "stoa": stoa.to_hex(),
        FOUNDING_TITLE: genesis.title,
        "policy": policy_name(genesis.policy),
    })
    .to_string()
}

// ─── The publish path ─────────────────────────────────────────────────────
//
// The contract is the `content-authoring` spec; the reasoning is in that
// change's `design.md`. `crate::authoring` decides; this parses, and converts a
// refusal into §2.5's one failure shape. What is repeated here is only what a
// reader of THIS code needs in order not to undo it.

/// What a successful publish tells the caller.
///
/// # Two fields, and the second is not decoration
///
/// An op id is a function of the op's own bytes, which carry no timestamp and no
/// nonce, so one identity publishing the same content into the same Stoa twice
/// publishes **one op** and the second call reports the first's id. Both reach a
/// caller as a success naming one id, and `wasNew` is the only thing that tells
/// them apart — a double-submitted form deduplicated, against a person
/// deliberately posting the same reply twice. `op-log`'s append is what knows
/// the answer, and this passes it on rather than discarding it.
///
/// # What is deliberately absent, on every one of the three
///
/// **No score, count, tally, rank or position — on a vote reply least of all.**
/// Nothing in the current contract reads a `Vote` op ([`crate::feed`] says so in
/// as many words), so a field describing an effect would be a caller inferring
/// one that does not exist. This is the same shape for a post, a reply and a
/// vote, which is what makes that absence structural rather than remembered.
///
/// It is also **not** a statement that any peer received the op. The append
/// completed; delivery's outcome arrives later and is not waited on.
fn published_json(published: &crate::authoring::Published) -> String {
    serde_json::json!({
        "opId": published.id.to_hex(),
        "wasNew": published.was_new(),
    })
    .to_string()
}

/// A posting policy's name on the wire.
///
/// **Exhaustive with no wildcard arm**, so a new variant forces a decision here
/// rather than defaulting to a name that describes a different policy — the same
/// position `stoa.rs` takes about the policy *discriminant*, applied to the
/// display form. `Policy::from_byte` refuses an unknown discriminant rather than
/// treating it as `Open`; a wildcard here would undo that one layer up, telling a
/// view a token-gated Stoa is world-postable.
fn policy_name(policy: crate::stoa::Policy) -> &'static str {
    match policy {
        // NO SPEC: the spec requires the posting policy be answerable and fixes no
        // spelling for it, so this lowercase literal is this change's choice. It is
        // a lasting one — a view branches on the string, so changing it is a
        // breaking change to the module surface. `design.md` carries it.
        crate::stoa::Policy::Open => "open",
    }
}

/// Create a Stoa: `{"title":"…"}` -> `{"stoa":…,"foundingTitle":…,"policy":…}`.
///
/// # The creator key is not a parameter, and cannot be
///
/// It arrives through `creator`, a closure the adapter supplies. A call that
/// accepted a creator key would be a call that can be asked to create a Stoa
/// moderated by somebody else — a Stoa the caller cannot moderate, did not mean to
/// make, and whose address cannot be un-minted, because the creator is fixed
/// inside the address preimage forever.
///
/// **A `PublicKey`, not a `SecretKey` and not a `Keystore`.** A genesis record is
/// not an op and carries no signature, so the creator's public key is the whole of
/// what is needed. Taking a secret would be taking authority the operation does
/// not use.
///
/// # Creation fails without a key rather than inventing one
///
/// There is no path from here to `Keystore::generate()`. The closure's failure is
/// surfaced as its own message and this handler adds no reason vocabulary of its
/// own — whether a key is usable and why not is the `posting-capability` probe's,
/// and paraphrasing it here would mean maintaining the same guidance twice.
///
/// # The title is refused before anything is recorded
///
/// **By this handler's own `genesis.address()` call**, which is
/// `stoa_address(&self.canonical_bytes()?)` — fallible for a title over the genesis
/// cap. It returns `{"error":"title: …"}` below, before the store is reached at
/// all, which is what makes "a failed creation leaves nothing behind" structural
/// rather than a rule to remember.
///
/// This used to credit `MembershipStore::join`'s encode-before-write ordering, and
/// that was false: `join` is never reached for an over-long title, and its refusal
/// renders a different sentence ("that genesis record cannot be encoded, so it
/// names no Stoa: …") which does not appear. Verified by running it — the reply for
/// a 1025-byte title is `{"error":"title: title is 1025 bytes, the maximum is
/// 1024"}`, this function's own prefix (`findings/readability.md` entry 1). The
/// guarantee was real and the mechanism named was the wrong one, which sends a
/// reader editing the store to preserve a property that lives here.
///
/// **No bound the genesis record does not have**, which specifically means an
/// empty title is accepted: the record has no minimum length, the title is not an
/// identifier, and refusing one here would make a record other peers decode and
/// verify without complaint unreachable through this surface.
///
/// # Creating the same title twice is one Stoa
///
/// A genesis record carries no per-peer state — no nonce, no timestamp — so the
/// same creator and the same title *is* the same record and therefore the same
/// address. The second call reports that address and leaves one membership,
/// because it goes through the same `join` a paste does and that write is
/// `INSERT OR IGNORE`. A user who wants two Stoas gives them two titles.
pub fn create_stoa(
    request: &str,
    creator: impl FnOnce() -> Result<crate::identity::PublicKey, crate::keystore::KeystoreError>,
    store: &mut crate::membership::MembershipStore,
) -> String {
    guarded("create_stoa", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let title = match parsed.get("title") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(_) => return error_json("title must be a string"),
            None => return error_json("missing field: title"),
        };

        // The key first, so a peer with no usable key is told so before anything
        // is built or opened. It is also the ONE failure here that is about the
        // user's state rather than their request, and surfacing the keystore's own
        // message is what keeps the reason actionable.
        let creator = match creator() {
            Ok(k) => k,
            Err(e) => return error_json(&e.to_string()),
        };

        let genesis = crate::stoa::Genesis {
            creator,
            // NO SPEC: the spec does not say which policy a created Stoa declares
            // and creation accepts no policy parameter. `Open` is the only variant
            // `stoa.rs` defines, so it is the only honest answer — and a parameter
            // selecting between one value would be a parameter that cannot select.
            policy: crate::stoa::Policy::Open,
            title,
        };
        // Refused HERE, before the store is even opened: a record with no encoding
        // has no address, so there is nothing to record it under.
        let stoa = match genesis.address() {
            Ok(a) => a,
            Err(e) => return error_json(&format!("title: {e}")),
        };

        // The SAME write a paste goes through. Two spec requirements fall out of
        // there being one write path rather than two that have to agree:
        // "creating the same title twice yields one Stoa", and "creating and then
        // joining the same Stoa is one membership".
        //
        // `verified` cannot refuse this pair — the address came from this very
        // record two lines up — and it is still called rather than bypassed,
        // because a constructor a caller may skip is not an invariant.
        let membership = match crate::membership::Membership::verified(&stoa, &genesis) {
            Ok(m) => m,
            Err(e) => return error_json(&e.to_string()),
        };
        match store.join(&membership) {
            Ok(_) => stoa_reply(&stoa, &genesis),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// Hand the published op to delivery, then report it as published — and do not
/// let delivery's failure become the publish's.
///
/// # Why the handoff gets its own `catch_unwind` inside an already-guarded handler
///
/// The op is in the log before this is called, and the spec says so: *"WHEN
/// delivery refuses or errors on the handoff, THEN the reply reports the op as
/// published and names its op id"*. A panicking sink is the most violent form of
/// "errors on the handoff", so it must not change the reply.
///
/// Without this, it changed the reply completely. [`guarded`] wraps the whole
/// handler, so a panic in the sink unwound past the `published_json` that had
/// already been computed and the caller received
/// `{"error":"panic in publish_post: …"}` — no `opId`, for an op that **is**
/// published. That is the one thing the requirement forbids: a publish reported
/// as having failed on the strength of a delivery outcome.
///
/// **The outer guard stays.** Moving `deliver` outside it was the other candidate
/// and is rejected: PHASE0-FINDINGS §3 measured what an unguarded panic costs —
/// the module process aborts (`failed to initiate panic, error 5`, SIGABRT), the
/// caller waits out a 20-second timeout, and every later call reports
/// `MODULE_NOT_LOADED`. Two nested guards is the cheap way to keep a panic
/// contained *and* keep the reply truthful.
///
/// # A caught panic is not swallowed silently
///
/// It goes to stderr, because the alternative is a transport defect that no
/// operator can see. It deliberately does **not** reach the reply: there is no
/// field for it. Per `delivery_module.lidl` the real outcome is asynchronous —
/// `channelMessageSent`, `channelMessageError`, `messagePropagated` all arrive
/// after this call has returned — so a synchronous reply could not carry a
/// delivery outcome even if the contract wanted one. Accepting an op is
/// `channelMessageSent` and says nothing about whether a peer received it.
///
/// Making an op that errors or never propagates visible is therefore
/// `op-transport`'s obligation, not this function's. Until it lands, this
/// conversion of a loud failure into a logged one is the known cost, recorded in
/// `docs/PLAN.md` §9.2.
fn delivered_and_published(
    published: &crate::authoring::Published,
    deliver: &mut dyn FnMut(&crate::op::OpId),
) -> String {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| deliver(&published.id))) {
        let detail = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_string());
        eprintln!(
            "dialectica: delivery panicked handing off {} — the op is published and stays published: {}",
            published.id.to_hex(),
            detail
        );
    }
    published_json(published)
}

/// The wire reply for a publish that never reached a handler, because no identity
/// was available to sign with.
///
/// # Why this exists rather than the adapter formatting its own message
///
/// The adapter is the only caller — it is the only code that can open a keystore,
/// which is the only way to discover that there is no usable identity, and
/// `dialectica-core` structurally cannot reach one. So the refusal can only be
/// *raised* there.
///
/// It must not also be *worded* there. The adapter is behind
/// `cfg(logos_scaffold)`, which no `cargo test` sets, so a message written in
/// that file is a message no gate in this repo compiles, let alone asserts on.
/// The first version of this change had exactly that: [`crate::authoring::Refusal::NoIdentity`]
/// carried the text and nothing constructed the variant, while a hand-written
/// `format!` in the adapter carried a second copy of the same sentence. Two
/// copies of one message with no test tying them, and the copy that shipped was
/// the one no test could see.
///
/// One line here fixes it in the direction that gains coverage rather than losing
/// it: the text has one home, in `Refusal`'s `Display`, which `cargo test` does
/// compile and `a_refusal_names_the_id_or_stoa_it_is_about` does assert on.
pub fn no_identity(why: &str) -> String {
    error_json(&crate::authoring::Refusal::NoIdentity(why.to_string()).to_string())
}

/// Field names no publish request may carry, and the reason each is refused.
///
/// **Refused rather than ignored**, which is the opposite of what
/// [`list_threads`] does with an `order` field, and the difference is what the
/// caller believes. A caller passing `order` believes it is selecting between
/// orderings that exist; a caller passing `author` believes it is choosing who
/// signs, and it is not — the identity falls out of the Stoa, so no operation can
/// be asked to sign as someone it is not. A caller passing `thread` believes it
/// is filing a reply somewhere, and the thread is derived from the parent.
///
/// Silently ignoring either leaves a caller acting on a belief the module has
/// quietly declined to honour.
///
/// NO SPEC: the spec requires `thread` to be refused on a **reply**. Refusing it
/// on a post and a vote too is chosen here — a caller who sent one has the same
/// wrong model whichever operation they sent it to — and is marked in
/// `a_forbidden_field_is_refused_on_every_operation`.
const FORBIDDEN_FIELDS: [(&str, &str); 5] = [
    (
        "author",
        "the identity that signs is derived from the Stoa and is never a parameter",
    ),
    (
        "identity",
        "the identity that signs is derived from the Stoa and is never a parameter",
    ),
    (
        "key",
        "the identity that signs is derived from the Stoa and is never a parameter",
    ),
    (
        "address",
        "the identity that signs is derived from the Stoa and is never a parameter",
    ),
    (
        "thread",
        "a reply's thread is derived from its parent and is never a parameter",
    ),
];

/// Refuse a request carrying a field that names something the caller may not
/// choose.
///
/// One guard over a list rather than a check per operation: CLAUDE.md keeps a
/// guard as its own job, so "is it called everywhere?" stays a question with an
/// answer. There are three callers and the list is the union across all three.
fn reject_forbidden_fields(parsed: &Request) -> Result<(), String> {
    for (field, why) in FORBIDDEN_FIELDS {
        if parsed.get(field).is_some() {
            return Err(error_json(&format!("{field} is not accepted: {why}")));
        }
    }
    Ok(())
}

/// A required string field, or a refusal that says which mistake was made.
///
/// A present-but-wrong-typed field is a different mistake from an absent one and
/// the message has to say which — "missing field: body" sends someone looking for
/// a field that is right there, holding a number.
fn required_string<'a>(parsed: &'a Request, field: &str) -> Result<&'a str, String> {
    match parsed.get(field) {
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(_) => Err(error_json(&format!("{field} must be a string"))),
        None => Err(error_json(&format!("missing field: {field}"))),
    }
}

// `required_stoa` used to live here, reading `stoa` through `required_string`
// and hex-decoding it. It is gone rather than converted to take a `&Request`:
// `parse_stoa` gives the same three answers for the same three cases, already
// takes one, and its own doc argues for being the ONLY place the surface reads
// that field — *"a new Stoa-taking handler cannot reach `stoa` without a
// `Request` in hand, because this is the only place that reads the field"*. Two
// readers of one field is the shape that eventually disagrees about whether a
// missing field and a wrong-typed one are one mistake, and keeping the publish
// path on its own copy is what kept it outside the envelope in the first place.

/// An op id field — a parent, or a vote's target.
fn required_op_id(parsed: &Request, field: &str) -> Result<crate::op::OpId, String> {
    let hex_str = required_string(parsed, field)?;
    crate::op::OpId::from_hex(hex_str).map_err(|e| error_json(&format!("{field}: {e}")))
}

/// A vote's direction, by name.
///
/// # The names are on the wire, and an unrecognised one is never mapped
///
/// `"up"` raises and `"down"` lowers. Anything else is refused **naming what was
/// supplied**, and is not defaulted onto a recognised direction: a caller whose
/// `"upvote"` silently became `"down"` would have published the opposite of what
/// it asked for, and nothing would error.
fn required_direction(parsed: &Request) -> Result<crate::op::VoteDirection, String> {
    let name = required_string(parsed, "direction")?;
    match name {
        "up" => Ok(crate::op::VoteDirection::Up),
        "down" => Ok(crate::op::VoteDirection::Down),
        other => Err(error_json(&format!(
            "direction must be \"up\" or \"down\", got \"{other}\""
        ))),
    }
}

/// A publish request that has been through the parse, the forbidden-field guard
/// and the `stoa` read — because it cannot be built without them.
///
/// # Why the prologue is a type and not three copies of four statements
///
/// `publish_post`, `publish_reply` and `publish_vote` each carried the same
/// opening — parse, reject forbidden fields, read `stoa` — and CLAUDE.md names
/// exactly this: *"when you find yourself writing the fourth slightly-different
/// copy of a guard, that is the signal to reshape rather than to add a fourth
/// test"*. `publish_moderation` is the fourth, and there a missed guard is an
/// **authorisation** defect rather than a wrong reply, which is why PLAN.md §9.2
/// calls this reshape its precondition.
///
/// So the guards are not called by each handler; they are what constructing the
/// value *is*. A handler holding a `PublishRequest` provably went through all
/// three, and a fourth publish operation inherits them without its author
/// knowing this decision happened.
///
/// # It is built on [`Request`], and that is what puts the publish path inside
/// the envelope
///
/// The three handlers parsed with a bare `serde_json::from_str`, so they were
/// outside every obligation `module-wire-contract` places on a request. All
/// three were measured failing, not inferred:
///
/// - **A non-object was served as a missing field.** `serde_json::Value::get`
///   answers `None` for an array exactly as it does for an object with no
///   `stoa`, so `[]` came back as `{"error":"missing field: stoa"}` — the
///   spec's "three caller mistakes, three messages" collapsed into one.
/// - **There was no size cap.** A request over
///   [`MAX_REQUEST_BYTES`] was parsed, at roughly 2N transient heap; per
///   `docs/PHASE0-FINDINGS.md` §3 an allocation failure in a dispatch handler
///   aborts the module process rather than returning an error.
/// - **And the three were absent from `every_request_taking_method`**, the sweep
///   whose whole job is applying these rules to the surface — so five sweeps ran
///   green over eleven methods while the surface had fourteen.
///
/// Going through [`Request::parse`] is the whole of the first two fixes, and it
/// is one line here rather than three at three call sites, which is what the
/// preceding refactor bought. The third is
/// `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`.
///
/// # Parse the whole request first, then act. The ordering is the requirement.
///
/// Each handler reads every field it needs before [`crate::authoring`] is
/// reached, so "a refused publish appends nothing and delivery was not invoked"
/// is structural: there is nothing to append until the last field has parsed.
/// Under validate-as-you-go that property would be an artefact of the order the
/// statements happen to be in. This type preserves that: it hands back the
/// request for the operation's own fields, and the handler reads all of them
/// before publishing.
struct PublishRequest {
    /// The Stoa every publish names, parsed once.
    stoa: crate::identity::Address,
    /// The request itself, for the fields this operation adds.
    fields: Request,
}

impl PublishRequest {
    /// Run the envelope, the forbidden-field guard and the `stoa` read, in that
    /// order.
    ///
    /// **The order is not arbitrary, and the first step is the load-bearing
    /// one.** [`Request::parse`] refuses an oversized request *before* paying
    /// for the parse, which is the whole point of a cap whose job is bounding
    /// allocation; and it refuses a non-object with a message the other two
    /// refusals can be told from. The forbidden-field guard then precedes the
    /// `stoa` read because a caller who supplied `author` has a wrong model of
    /// the API that a message about a malformed Stoa would not correct.
    fn parse(request: &str) -> Result<Self, String> {
        let fields = Request::parse(request)?;
        reject_forbidden_fields(&fields)?;
        let stoa = parse_stoa(&fields)?;
        Ok(PublishRequest { stoa, fields })
    }
}

/// The Stoa a publish request names, for a caller that must know it **before**
/// the handler runs — which is the adapter, and only the adapter.
///
/// # Why this exists, and why it is not a convenience
///
/// The per-Stoa signing key cannot be derived without the Stoa, and only the
/// adapter can open a keystore ([`crate::keystore`] needs a host-supplied path
/// this crate cannot know). So the adapter genuinely has to read one field
/// before it can supply the `key` argument the handlers take.
///
/// It was reading it with its own bare `serde_json::from_str` and its own
/// four-arm `stoa` ladder, and **that shadowed every envelope fix in this
/// change on the shipped module**: an array was answered `missing field: stoa`
/// by the adapter at a line no `cargo test` compiles, and an N-byte request was
/// fully parsed at ~2N transient heap before [`MAX_REQUEST_BYTES`] was ever
/// evaluated — so the cap bounded only a *second* parse of bytes already paid
/// for, and the PHASE0-FINDINGS §3 abort it exists to prevent was untouched.
/// `dialectica-core` was correct in isolation and the module was not.
///
/// This is the same envelope, reached through the same [`Request::parse`], so
/// the adapter's early read and the handler's later one **cannot disagree**:
/// the size cap is evaluated on the adapter's call because it is the first
/// thing `Request::parse` does, and a non-object earns [`REQUEST_NOT_AN_OBJECT`]
/// here exactly as it does inside a handler.
///
/// # It reads the Stoa and nothing else, deliberately
///
/// It does **not** run the forbidden-field guard or any required-field read.
/// Those stay the handler's, so there is exactly one place that decides what a
/// publish request must contain — an adapter that validated a second time is
/// the two-readers-of-one-field shape that produced this defect. What the
/// adapter gets is the one value it structurally cannot proceed without.
///
/// **The request is parsed twice and that is accepted rather than hidden.**
/// Both parses are now bounded by the cap, so the cost is CPU on a request
/// already proved small, not unbounded allocation. Removing the second would
/// mean the handlers taking a key *supplier* rather than a key — a reshape
/// `design.md` defers with its argument.
pub fn stoa_of(request: &str) -> Result<crate::identity::Address, String> {
    let parsed = Request::parse(request)?;
    parse_stoa(&parsed)
}

/// The shape all three publish handlers have: guard, prologue, the operation's
/// own fields, then the tail.
///
/// # What the caller is left with, and why that is the whole of it
///
/// `run` receives a [`PublishRequest`] — already parsed, guarded and with its
/// Stoa read — and returns the [`crate::authoring::Published`]. Everything
/// either side of that is here: the panic guard, the three prologue steps, and
/// the delivery handoff with its own `catch_unwind`.
///
/// So a fourth publish operation is a closure reading its own fields. It cannot
/// forget a guard, because it never runs one.
///
/// # The error arm is the wire reply already, not a type to convert
///
/// Both things a `run` closure can fail on already produce one: a field reader
/// (`required_string` and friends) returns the error shape, and a
/// [`crate::authoring::Refusal`] is turned into one by [`refused`]. Following
/// [`Request::parse`]'s own convention here — *"the `Err` arm is already the
/// wire reply, so a caller cannot invent a second error shape while converting
/// one"* — means the two arrive by the same route and no closure has to decide
/// which wrapper to use.
fn publishing(
    method: &'static str,
    request: &str,
    deliver: &mut dyn FnMut(&crate::op::OpId),
    run: impl FnOnce(&PublishRequest) -> Result<crate::authoring::Published, String>,
) -> String {
    guarded(method, || {
        let parsed = match PublishRequest::parse(request) {
            Ok(p) => p,
            Err(e) => return e,
        };
        match run(&parsed) {
            Ok(published) => delivered_and_published(&published, deliver),
            Err(already_the_wire_reply) => already_the_wire_reply,
        }
    })
}

/// An authoring refusal, as the wire reply.
///
/// One function rather than `.map_err(|r| error_json(&r.to_string()))` at three
/// call sites: the wording of a refusal is the `Refusal` type's own job (see
/// [`no_identity`] for why that matters — the adapter is compiled by no test, so
/// a message written outside `Refusal`'s `Display` is a message no gate sees),
/// and three copies of the conversion is three places for one of them to start
/// wording it differently.
fn refused(refusal: crate::authoring::Refusal) -> String {
    error_json(&refusal.to_string())
}

/// `{"stoa":"…","body":"…"}` -> `{"opId":"…","wasNew":bool}`.
///
/// # `deliver` is a sink, and its RETURN TYPE carries three requirements at once
///
/// It returns **nothing**. So:
///
/// - the append has completed before it is called, because it is called after
///   [`crate::authoring::post`] returns;
/// - a declined or erroring handoff leaves the op published, because there is no
///   outcome to inspect and none to act on;
/// - a publish cannot be deferred until delivery reports, because there is
///   nothing to report — a call that waited on one could not be written here.
///
/// It is also **not called on a refusal**, and structurally rather than by a
/// guard: it sits on the success arm of the `Result` and no refusal path reaches
/// it.
///
/// # `&mut dyn FnMut` rather than `impl FnOnce`, and the reason is a compile
/// error nothing else could see
///
/// `impl FnOnce(&OpId)` was written first, because "called at most once" is the
/// honest bound on what a sink is for.
///
/// What rules it out is the requirement that all three handlers be usable
/// through **one** function-pointer type, which
/// `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over` pins
/// with a `type Handler = fn(…)`. A generic `impl FnOnce` monomorphises per call
/// site, so the three would be three types with no shared pointer, and coercing
/// them fails on a higher-ranked lifetime.
///
/// **Be precise about where that constraint comes from, because an earlier
/// version of this comment was not.** It said the *adapter* needs one pointer
/// type. It does not: `Dialectica::publishing` is generic over the handler
/// (`F: FnOnce(…)`), so it monomorphises per call site and would accept a
/// generic sink. The single-pointer requirement is the test's, deliberately —
/// the adapter is behind `cfg(logos_scaffold)`, which no `cargo test` sets, so
/// pinning the three signatures as interchangeable in *this* crate is what stops
/// a signature drifting into an error that would surface only in the builder's
/// build, the one that runs last and reports worst.
///
/// So recovering `FnOnce` is a live option, not a closed one: it costs changing
/// that test's `Handler` type, and buys back the at-most-once bound.
///
/// Nothing is lost that a requirement rests on. The return type `()` is what
/// makes a delivery outcome unwaitable; `FnOnce` only added that the sink could
/// not be called twice, which no requirement asks for and which the one call
/// site makes true anyway.
pub fn publish_post<L: crate::log::OpLog>(
    request: &str,
    log: &mut L,
    key: &crate::identity::SecretKey,
    deliver: &mut dyn FnMut(&crate::op::OpId),
) -> String {
    publishing("publish_post", request, deliver, |parsed| {
        let body = required_string(&parsed.fields, "body")?.to_string();
        crate::authoring::post(log, key, parsed.stoa, body).map_err(refused)
    })
}

/// Join a Stoa: `{"stoa":"<hex>","genesis":"<hex>"}` -> the same reply shape.
///
/// # It takes the record as well as the address, and that is a property of the
/// address
///
/// A Stoa address is a one-way hash of its genesis record: sufficient to **verify**
/// a record somebody hands over, and insufficient to **reconstruct** one. Since a
/// membership must retain the record — `moderation-resolution` requires the record
/// before a reader may decide whether any moderation of that Stoa's content binds
/// — the record has to arrive with the address, because there is nowhere else for
/// it to come from.
///
/// So a bare address is not joinable, and this signature says so rather than
/// leaving a caller to discover it after joining.
/// [`crate::membership::Membership::verified`] does the verification — once, and it
/// consults nothing but its two arguments.
///
/// # A repeated join is not a failure
///
/// A pasted address is exactly the input a user supplies twice. The reply is the
/// same either way and carries no "was this new" flag: the spec asks that the
/// second attempt succeed and change nothing, and a view that rendered "already
/// joined" differently would be rendering a distinction the user did not make.
pub fn join_stoa(request: &str, store: &mut crate::membership::MembershipStore) -> String {
    guarded("join_stoa", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(a) => a,
            Err(e) => return e,
        };
        // `genesis_for` is the feed path's decoder and it returns the VERIFIED
        // pair, which is the only type `join` accepts. Reused rather than
        // reimplemented: a second decoder would be a second place for the
        // verification to be forgotten, and this one carries the argument for why
        // a caller-supplied record is safe.
        //
        // ONE verification, at one place. This handler used to verify here and the
        // store used to verify again, and no test could tell the two apart —
        // deleting either left the suite green. There is nothing to re-check now,
        // because a `Membership` that disagrees with itself cannot be built.
        let membership = match genesis_for(&parsed, &stoa) {
            Ok(m) => m,
            Err(e) => return e,
        };
        match store.join(&membership) {
            Ok(_) => stoa_reply(&membership.stoa, &membership.genesis),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// One page of the Stoas this peer is in.
///
/// `{"page":N,"perPage":N}` -> `{"items":[{"stoa":…,"foundingTitle":…}],"page":N,"hasMore":bool}`.
///
/// # Exactly what membership records, and nothing derived from ops
///
/// This handler cannot reach the op log: it is handed a `MembershipStore` and
/// there is no path from one to the other. So "the listing contains the Stoas the
/// peer is in and no others" holds by construction — a Stoa the peer created with
/// no ops is listed, and a Stoa for which ops arrived but nobody joined is not.
///
/// # `page` and `perPage` behave exactly as the feed's do
///
/// [`parse_index`] and [`crate::feed::clamp_per_page`] are reused unchanged, so a
/// negative page is refused here for the same reason and with the same message as
/// there. A second interpretation of those arguments is how one method eventually
/// clamps what the other refuses.
pub fn list_stoas(request: &str, store: &crate::membership::MembershipStore) -> String {
    guarded("list_stoas", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let page = match parse_index(&parsed, "page") {
            Ok(v) => v.unwrap_or(0),
            Err(e) => return e,
        };
        let per_page = match parse_index(&parsed, "perPage") {
            Ok(v) => crate::feed::clamp_per_page(v),
            Err(e) => return e,
        };

        match store.list(page, per_page) {
            Ok(page) => membership_page_json(&page),
            // A storage failure is the error shape and NEVER an empty listing. The
            // two mean opposite things — "this peer is in no Stoa" versus "this
            // peer's store is unreadable" — and render identically if this arm is
            // ever softened. Same obligation `list_threads` carries for a feed.
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// Open a membership store for writing and hand it to a handler.
///
/// # Why the handlers take a store and this takes a path
///
/// `create_stoa` and `join_stoa` take `&mut MembershipStore` because that is the
/// shape worth testing: creating a Stoa and then listing it is one store used
/// twice, and an opener closure would make an in-memory store — the one that needs
/// no temporary directory and no teardown — unusable for exactly the tests that
/// matter most.
///
/// **`list_stoas` takes `&`, not `&mut`, and reaches its store through
/// [`with_membership_store_read`].** This doc used to say all three handlers take
/// `&mut`, which was false of `list_stoas` from the moment it was written
/// (`findings/readability.md` entry 2) — the call compiled because `&mut T` coerces
/// to `&T`, so nothing failed and the type at the seam simply asserted that all
/// three write. `findings/architecture.md` entry 4 is the same observation as a
/// shape finding: the read path was indistinguishable from the write path at the
/// seam, which is the first thing that has to be distinguishable when two instances
/// contend for one file and readers should proceed while a writer holds the lock.
///
/// # There is no `method` parameter, and that is the fix for a parameter nobody could keep right
///
/// This used to take the method name and pass it to [`guarded`], while each of the
/// three handlers *also* called `guarded` with its own hardcoded name — so the
/// guard was nested and the outer name was a second, caller-supplied copy of
/// something the inner one already knew. It could disagree and nothing noticed:
/// `with_membership_store("list_stoas", …, |s| create_stoa(…, s))` compiled, ran,
/// and reported a panic in `open` as `panic in list_stoas`
/// (`findings/architecture.md` entry 5). Five hand-written pairs had to be kept in
/// step for no gain.
///
/// The outer guard stays and owns a generic label. A panic in
/// `MembershipStore::open` is the only thing it can report that the inner guard
/// cannot — every other panic is inside the handler, where the handler's own guard
/// names the method — so the label is accurate for everything it can ever catch.
///
/// **A failed open is the error shape and never an empty answer.** `SqliteOpLog`'s
/// own documentation makes the argument: an empty listing is indistinguishable
/// from a peer that is in no Stoa, so flattening this would render a peer whose
/// store is broken as a peer that has joined nothing — and the user would be
/// invited to re-paste every address they hold.
///
/// The guard wraps the open too, rather than only the handler inside it: a panic
/// while opening a store is a panic in a dispatch handler like any other, and it
/// aborts the module process the same way.
pub fn with_membership_store(
    path: &std::path::Path,
    handler: impl FnOnce(&mut crate::membership::MembershipStore) -> String,
) -> String {
    guarded(
        "opening the Stoa membership store",
        || match crate::membership::MembershipStore::open(path) {
            Ok(mut store) => handler(&mut store),
            Err(e) => error_json(&e.to_string()),
        },
    )
}

/// Open a membership store for reading and hand it to a handler.
///
/// The read half of [`with_membership_store`], separate so that `list_stoas`'s
/// read-only-ness survives the seam it is reached through rather than being widened
/// to `&mut` at it (`findings/architecture.md` entry 4).
///
/// This buys no concurrency today — `rusqlite::Connection` is not `Sync`, and both
/// halves still open per call. What it buys is that the type stops asserting
/// something false, and that letting readers proceed while a writer holds the write
/// lock becomes a change to this function rather than to every arm of the adapter.
pub fn with_membership_store_read(
    path: &std::path::Path,
    handler: impl FnOnce(&crate::membership::MembershipStore) -> String,
) -> String {
    guarded(
        "opening the Stoa membership store",
        || match crate::membership::MembershipStore::open(path) {
            Ok(store) => handler(&store),
            Err(e) => error_json(&e.to_string()),
        },
    )
}

/// Re-exported so the adapter reaches it as `core::membership_path_in`, beside the
/// handlers it is passed to.
///
/// **It lives in [`crate::membership`]**, which is the module that owns the file.
/// It was defined here, in the module whose stated job is the wire contract — and a
/// file name on disk is not wire contract. Its own docstring said it followed
/// `keystore::default_path_in` because "the naming convention belongs with the thing
/// named", and then did not (`findings/architecture.md` entry 2). Three stores had
/// three conventions in three layers; two of them now agree, and the third — the op
/// log's `dir.join("ops.sqlite")`, inline in the adapter — is named in `design.md`
/// as the remaining one.
pub use crate::membership::membership_path_in;

/// The pagination shape for a membership listing, built in one place.
fn membership_page_json(page: &crate::membership::MembershipPage) -> String {
    let items: Vec<serde_json::Value> = page
        .items
        .iter()
        .map(|m| {
            serde_json::json!({
                "stoa": m.stoa.to_hex(),
                FOUNDING_TITLE: m.genesis.title,
            })
        })
        .collect();
    serde_json::json!({
        "items": items,
        "page": page.page,
        "hasMore": page.has_more,
    })
    .to_string()
}

/// `{"stoa":"…","parent":"…","body":"…"}` -> `{"opId":"…","wasNew":bool}`.
///
/// **There is no `thread` parameter**, and one supplied is refused rather than
/// ignored. The thread is derived from the parent, which makes "a reply filed
/// under a thread its parent does not belong to" unrepresentable rather than
/// checked — see [`crate::authoring::reply`] for the derivation and what it
/// trusts.
pub fn publish_reply<L: crate::log::OpLog>(
    request: &str,
    log: &mut L,
    key: &crate::identity::SecretKey,
    deliver: &mut dyn FnMut(&crate::op::OpId),
) -> String {
    publishing("publish_reply", request, deliver, |parsed| {
        let parent = required_op_id(&parsed.fields, "parent")?;
        let body = required_string(&parsed.fields, "body")?.to_string();
        crate::authoring::reply(log, key, parsed.stoa, parent, body).map_err(refused)
    })
}

/// `{"stoa":"…","target":"…","direction":"up"|"down"}` ->
/// `{"opId":"…","wasNew":bool}`.
///
/// The reply carries an op id and nothing that describes an effect. Nothing in
/// the current contract reads a `Vote` op, so there is no score to report and
/// reporting one would be a falsehood a caller would act on.
pub fn publish_vote<L: crate::log::OpLog>(
    request: &str,
    log: &mut L,
    key: &crate::identity::SecretKey,
    deliver: &mut dyn FnMut(&crate::op::OpId),
) -> String {
    publishing("publish_vote", request, deliver, |parsed| {
        let target = required_op_id(&parsed.fields, "target")?;
        let direction = required_direction(&parsed.fields)?;
        crate::authoring::vote(log, key, parsed.stoa, target, direction).map_err(refused)
    })
}

// ─── The two halves of the delivery bridge that CAN be tested ─────────────
//
// `modules().delivery_module` cannot appear in this file: it calls `lp_*`
// symbols undefined in a test binary. So the call itself stays in the adapter
// and everything on either side of it lives here, where a test can reach it.
// This is the Phase 1 shape (PLAN.md §9) arriving early because Phase 0 needed
// it anyway.

/// Pull `channelId` out of a request, or return the error shape to send back.
///
/// The `Result` is `String` on both sides on purpose: the error arm is already
/// the wire reply, so a caller cannot accidentally invent a second error shape
/// while converting one.
pub fn parse_channel_id(request: &str) -> Result<String, String> {
    let parsed = Request::parse(request)?;
    match parsed.get("channelId") {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        // A present-but-wrong-typed field is a different mistake from a missing
        // one, and saying which halves the time it takes to fix.
        Some(_) => Err(error_json("channelId must be a string")),
        None => Err(error_json("missing field: channelId")),
    }
}

/// Delivery's error message, if this reply is a callee's error envelope.
///
/// A typed cross-module client's happy path and its error path are **separate
/// contracts**, and only the happy one is visible in the generated signature.
/// When delivery declines a call it answers `Ok(...)` at the Rust level with an
/// envelope body — observed live as
/// `{"error":"Context not initialized","success":false,"value":null}` when
/// nothing had yet called `createNode`. A decoder that only knows the success
/// shapes reads that as junk.
///
/// **What this keys off, and why it is `error` alone.** PLAN.md §2.5 makes
/// `{"error":"..."}` the ecosystem's one failure shape; `success` and `value`
/// are delivery's own extras. Requiring the fuller triple would silently miss a
/// callee that spells its envelope with `error` only — and missing a real error
/// is the expensive direction, since it lands back in the catch-all and
/// reproduces exactly the bug this exists to fix.
///
/// The guard against the opposite risk — misreading a legitimate *value* as an
/// error — is the two conditions here: the reply must be a JSON **object**, and
/// `error` must hold a **string**. No success reply in this contract is an
/// object (`channelExists` answers a bool or the FFI's `"true"`/`"false"`), so
/// there is nothing for this to shadow. A callee whose success shape ever is an
/// object with a genuine `error` field would need its own decoder, and that is
/// a contract worth noticing rather than papering over.
pub fn callee_error(reply: &serde_json::Value) -> Option<&str> {
    reply.as_object()?.get("error")?.as_str()
}

/// Wrap delivery's reply in our own shape.
///
/// Two facts meet here and neither is obvious:
///
/// - The contract types `channelExists` as `-> result`, so the generated Rust
///   client hands back a `serde_json::Value`, not a `bool`.
/// - delivery v0.2.1 answers with **the FFI string verbatim** — `"true"` or
///   `"false"`, per its own docstring — so the Value is typically a JSON
///   *string*, not a JSON boolean.
///
/// Both spellings are therefore normalised to a real boolean, and anything
/// else is an error rather than a default. Coercing an unrecognised value to
/// `false` would report "the channel is not open" for a reply that never said
/// that — the failure mode PLAN.md §2.5 forbids, where a broken call is
/// indistinguishable from a successful negative answer.
///
/// An error envelope is recognised *before* the boolean match, so delivery's
/// own message reaches the caller unchanged rather than quoted inside a
/// complaint about our failure to parse it.
pub fn channel_exists_reply(reply: &serde_json::Value) -> String {
    // Before deciding what the value means, decide whether there is a value at
    // all. Ordering is the whole fix: run this after the match and every
    // decline still falls into the catch-all.
    if let Some(message) = callee_error(reply) {
        return error_json(message);
    }
    let exists = match reply {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) if s == "true" => true,
        serde_json::Value::String(s) if s == "false" => false,
        other => {
            return error_json(&format!(
                "delivery_module.channelExists returned an unrecognised value: {other}"
            ))
        }
    };
    serde_json::json!({ "exists": exists }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_converts_a_panic_into_the_error_shape() {
        // Delete `guarded` from `panic_probe` and this test does not fail —
        // it ABORTS the test binary. That is the ordering the guard must be
        // written against: the failure is visible either way, and neither way
        // is green.
        let out = guarded("probe", || panic!("boom"));
        let v: serde_json::Value = serde_json::from_str(&out).expect("guard emitted valid JSON");
        assert!(
            v.get("error").is_some(),
            "a caught panic must surface as the error shape, got {out}"
        );
        assert!(
            v["error"].as_str().unwrap().contains("boom"),
            "the guard must carry the panic message through, got {out}"
        );
    }

    #[test]
    fn guard_carries_a_formatted_panic_payload() {
        // `panic!("{}", x)` produces a String payload, not a &str. A downcast
        // that handles only &str reports "non-string panic payload" here and
        // throws away the one piece of information worth having.
        let out = guarded("probe", || panic!("{}", format!("detail {}", 42)));
        assert!(out.contains("detail 42"), "got {out}");
    }

    #[test]
    fn guard_names_the_method_that_panicked() {
        // With one guard per handler and one error shape, the method name is
        // the only thing distinguishing "which handler died".
        let out = guarded("some_method", || panic!("boom"));
        assert!(out.contains("some_method"), "got {out}");
    }

    #[test]
    fn guard_passes_a_success_through_untouched() {
        let out = guarded("probe", || r#"{"ok":true}"#.to_string());
        assert_eq!(out, r#"{"ok":true}"#);
    }

    #[test]
    fn guard_output_survives_a_panic_payload_containing_json_metacharacters() {
        // A panic payload is attacker-influenced the moment a handler formats
        // untrusted input into it. Naive concatenation emits malformed JSON,
        // and the view sees a parse error instead of the failure.
        let out = guarded("probe", || panic!("{}", r#"he said "hi"\ and left"#));
        serde_json::from_str::<serde_json::Value>(&out)
            .expect("the error shape must stay valid JSON for any panic payload");
    }

    #[test]
    fn ping_echoes_its_payload() {
        let v: serde_json::Value = serde_json::from_str(&ping(r#"{"payload":{"n":1}}"#)).unwrap();
        assert_eq!(v["pong"]["n"], 1);
    }

    #[test]
    fn ping_rejects_malformed_json_without_unwinding() {
        let v: serde_json::Value = serde_json::from_str(&ping("not json")).unwrap();
        assert!(v.get("error").is_some());
    }

    #[test]
    fn ping_rejects_a_missing_field_rather_than_defaulting_it() {
        let out = ping(r#"{"other":1}"#);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v.get("pong").is_none(),
            "a failure must never also carry a result — that is the partial-success shape §2.5 forbids"
        );
    }

    #[test]
    fn panic_probe_returns_the_error_shape_instead_of_unwinding() {
        let out = panic_probe(r#"{}"#);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
    }

    #[test]
    fn version_reports_what_it_was_given() {
        let v: serde_json::Value = serde_json::from_str(&version("9.9.9")).unwrap();
        assert_eq!(v["version"], "9.9.9");
    }

    #[test]
    fn parse_channel_id_accepts_a_string() {
        assert_eq!(
            parse_channel_id(r#"{"channelId":"stoa-abc/e7"}"#).unwrap(),
            "stoa-abc/e7"
        );
    }

    #[test]
    fn parse_channel_id_distinguishes_missing_from_wrong_typed() {
        // Both are errors, but they are different mistakes and the message has
        // to say which — otherwise "missing field" sends someone looking for a
        // field that is right there.
        let missing = parse_channel_id(r#"{}"#).unwrap_err();
        assert!(missing.contains("missing"), "got {missing}");

        let wrong = parse_channel_id(r#"{"channelId":7}"#).unwrap_err();
        assert!(wrong.contains("must be a string"), "got {wrong}");
    }

    #[test]
    fn parse_channel_id_errors_are_already_the_wire_shape() {
        for bad in [r#"{}"#, r#"{"channelId":7}"#, "not json"] {
            let err = parse_channel_id(bad).unwrap_err();
            let v: serde_json::Value = serde_json::from_str(&err)
                .unwrap_or_else(|e| panic!("error arm must be valid JSON ({e}): {err}"));
            assert!(v.get("error").is_some(), "got {err}");
        }
    }

    #[test]
    fn channel_exists_normalises_deliverys_verbatim_string_to_a_boolean() {
        // delivery v0.2.1 answers with the FFI string, so this is the shape
        // that actually arrives — a JSON string, through a `-> result` method.
        for (reply, want) in [("true", true), ("false", false)] {
            let out = channel_exists_reply(&serde_json::json!(reply));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert_eq!(v["exists"], want, "for delivery reply {reply:?}");
        }
    }

    #[test]
    fn channel_exists_also_accepts_a_real_boolean() {
        // Cheap insurance against the upstream tightening its return type: if
        // delivery ever answers with a JSON bool, this keeps working rather
        // than reporting every channel as unrecognised.
        let out = channel_exists_reply(&serde_json::json!(true));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["exists"], true);
    }

    #[test]
    fn channel_exists_propagates_deliverys_own_error_rather_than_wrapping_it() {
        // The exact envelope a live delivery sent when nothing had called
        // `createNode` yet (PHASE0-FINDINGS §6). Before the fix this fell
        // through to the catch-all, and the one useful string —
        // "Context not initialized" — reached the view only as quoted text
        // inside a parser complaint about an "unrecognised value".
        let out = channel_exists_reply(&serde_json::json!({
            "error": "Context not initialized",
            "success": false,
            "value": null
        }));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["error"], "Context not initialized",
            "delivery's own message must arrive unchanged, got {out}"
        );
        assert!(
            v.get("exists").is_none(),
            "a failure must never also carry a result — §2.5"
        );
    }

    #[test]
    fn callee_error_reads_an_envelope_without_demanding_deliverys_extra_fields() {
        // Keying off the fuller {error, success, value} triple would miss a
        // callee that spells its envelope with `error` alone — and a missed
        // error is the expensive direction, because it lands in the catch-all
        // and reproduces the bug.
        assert_eq!(
            callee_error(&serde_json::json!({"error": "boom"})),
            Some("boom")
        );
        assert_eq!(
            callee_error(&serde_json::json!({
                "error": "boom", "success": false, "value": null
            })),
            Some("boom")
        );
    }

    #[test]
    fn callee_error_does_not_claim_an_error_for_a_legitimate_value() {
        // The other direction of the same tradeoff: too loose a check would
        // read a real reply as a failure. A success in this contract is never
        // an object, and a non-string `error` is not a message we could
        // propagate unchanged anyway.
        for not_an_envelope in [
            serde_json::json!(true),
            serde_json::json!("false"),
            serde_json::json!(null),
            serde_json::json!(["error"]),
            serde_json::json!({"exists": true}),
            serde_json::json!({"error": 500}),
            serde_json::json!({"error": null}),
        ] {
            assert_eq!(
                callee_error(&not_an_envelope),
                None,
                "for {not_an_envelope}"
            );
        }
    }

    #[test]
    fn channel_exists_refuses_to_guess_at_an_unrecognised_reply() {
        // The trap this is here to prevent: coercing anything unrecognised to
        // `false` would make a broken call read as "the channel is not open",
        // and a caller would act on an answer delivery never gave.
        for reply in [
            serde_json::json!("maybe"),
            serde_json::json!(1),
            serde_json::json!(null),
        ] {
            let out = channel_exists_reply(&reply);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for reply {reply}, got {out}");
            assert!(
                v.get("exists").is_none(),
                "a failure must not also carry a result"
            );
        }
    }

    // ─── The capability probe ─────────────────────────────────────────────

    use crate::identity::{stoa_address, Address};
    use crate::keystore::KeystoreError;

    fn a_stoa() -> Address {
        stoa_address(b"a genesis record")
    }

    /// The probe with a lookup that succeeds.
    fn probe_ok(request: &str, identity: &str) -> serde_json::Value {
        // Cloned per call rather than moved, now that `lookup` is `Fn`. That
        // is the helper paying the cost of the property being testable at all
        // — see `the_probe_is_callable_repeatedly_with_one_lookup`.
        serde_json::from_str(&get_capabilities(request, |_| Ok(identity.to_string()))).unwrap()
    }

    /// The probe with a lookup that fails for this reason.
    ///
    /// Takes a FACTORY rather than an error, because `lookup` is `Fn` and
    /// `KeystoreError` is not `Clone`. Deriving `Clone` on it to satisfy a
    /// test helper would be widening the library's surface for the
    /// convenience of testing it — the same trade `err_of` exists to avoid
    /// over `Debug`.
    ///
    /// The factory still yields a `KeystoreError` and the message is taken from its
    /// `Display` here, which keeps these tests asserting on the SAME strings the
    /// keystore's own "every message names the fix" obligation covers. The lookup's
    /// error type widened to `String` when the probe began consulting the path
    /// record; what these tests are about did not change.
    fn probe_err_with(request: &str, make: impl Fn() -> KeystoreError) -> serde_json::Value {
        serde_json::from_str(&get_capabilities(request, |_| Err(make().to_string()))).unwrap()
    }

    #[test]
    fn the_probe_reports_an_identity_when_posting_is_possible() {
        // The expected identity is a literal, not something read back from the
        // lookup — otherwise the assertion is the test agreeing with itself.
        let v = probe_ok(
            &format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()),
            "ff00ff00",
        );
        assert_eq!(v["canPost"], true);
        assert_eq!(v["identity"], "ff00ff00");
    }

    #[test]
    fn the_probe_is_callable_repeatedly_with_one_lookup() {
        // A view asks whenever it renders, not once per process. This was
        // **unsatisfiable through this API** until `lookup` became `Fn`:
        // `FnOnce` meant the probe could not be called twice with the same
        // closure, so the scenario could never have been tested. Review caught
        // a spec requirement that the code made impossible to check.
        let request = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());
        let calls = std::cell::Cell::new(0usize);
        let lookup = |_: &Address| {
            calls.set(calls.get() + 1);
            Ok("abcd".to_string())
        };

        // NO SPEC: this test does not distinguish `Fn` from `FnOnce`, and
        // saying so is the point. `FnOnce` is a supertrait of `Fn`, so `&F`
        // satisfies it — reverting the signature leaves this green, checked
        // rather than assumed. What the test pins is the OBSERVABLE half of
        // the requirement: repeated calls agree, and each one re-consults the
        // lookup rather than caching. The signature choice is argued in
        // `get_capabilities`' doc comment and is not mutation-detectable.
        let first = get_capabilities(&request, lookup);
        let second = get_capabilities(&request, lookup);
        let third = get_capabilities(&request, lookup);

        assert_eq!(first, second, "the probe's answer must be stable");
        assert_eq!(second, third);
        assert_eq!(calls.get(), 3, "each call must consult the lookup afresh");

        // A lookup that OWNS something, which is the shape a real adapter has
        // — it holds the keystore path, or the keystore itself. Included
        // because it is the realistic case, not because it distinguishes the
        // two bounds; `&owning` satisfies either.
        let owned_path = std::path::PathBuf::from("/keys/identity.key");
        let owning = move |_: &Address| {
            Ok(owned_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned())
        };
        let a = get_capabilities(&request, &owning);
        let b = get_capabilities(&request, &owning);
        assert_eq!(a, b, "a lookup owning its state must be reusable");
    }

    #[test]
    fn the_reported_identity_is_the_one_an_op_is_actually_signed_under() {
        // END TO END, through a real keystore and a real signature. Every
        // other probe test hands back the literal "abcd", so none of them
        // could see the probe reporting one identity while the user posted
        // under another — which is the thing this requirement exists to stop.
        //
        // The keystore's root is fixed, so the expected address comes from a
        // derivation this test performs independently rather than from the
        // probe's own answer.
        use crate::identity::{derive_stoa_key, sign_op_bytes, verify_authored_op};

        let root = [7u8; 32];
        let stoa = a_stoa();
        let request = format!(r#"{{"stoa":"{}"}}"#, stoa.to_hex());

        let out = get_capabilities(&request, |s| {
            Ok(derive_stoa_key(&root, s).public_key().address().to_hex())
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["canPost"], true, "got {out}");
        let reported = v["identity"].as_str().unwrap();

        // Now sign something as that identity would, and verify the op is
        // attributed to the address the probe named.
        let key = derive_stoa_key(&root, &stoa);
        let sig = sign_op_bytes(&key, b"a post");
        let author = Address::from_hex(reported).expect("the probe reports a parseable address");
        assert!(
            verify_authored_op(
                &author,
                &key.public_key().to_bytes(),
                b"a post",
                &sig.to_bytes()
            ),
            "an op signed by this identity is not attributed to the address the \
             probe reported"
        );

        // And the negative: a DIFFERENT Stoa's key must not verify against the
        // reported address, or the assertion above would hold for any key.
        let other = derive_stoa_key(&root, &stoa_address(b"some other stoa"));
        let other_sig = sign_op_bytes(&other, b"a post");
        assert!(
            !verify_authored_op(
                &author,
                &other.public_key().to_bytes(),
                b"a post",
                &other_sig.to_bytes()
            ),
            "another Stoa's key verified against this Stoa's reported identity"
        );
    }

    #[test]
    fn a_successful_probe_carries_no_reason_and_a_failed_one_no_identity() {
        // The contract is `"identity":"…" | "reason":"…"` — the bar is
        // exclusive. §2.5 forbids a reply that is partly a success, and a
        // compose box gated on `canPost` while a stale `identity` field sits
        // beside it is exactly how the wrong one gets rendered.
        let stoa = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());

        let yes = probe_ok(&stoa, "abcd");
        assert!(yes.get("reason").is_none(), "got {yes}");

        let no = probe_err_with(&stoa, || KeystoreError::NotFound);
        assert_eq!(no["canPost"], false);
        assert!(no.get("identity").is_none(), "got {no}");
    }

    #[test]
    fn each_failure_state_produces_a_distinguishable_reason() {
        // The probe's whole value to a view is that the five states a user can
        // actually be in are told apart. Collapsing any two means one of the
        // two reasons is wrong, and a wrong reason sends someone to fix
        // something that is not broken.
        let stoa = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());
        let mut seen = Vec::new();
        let makers: [fn() -> KeystoreError; 7] = [
            || KeystoreError::NotFound,
            || KeystoreError::Locked,
            || KeystoreError::WrongPassphrase,
            || KeystoreError::PermissionsTooOpen { mode: 0o644 },
            || KeystoreError::DirectoryWritableByOthers { mode: 0o777 },
            || KeystoreError::NotAKeystore,
            || KeystoreError::Truncated,
        ];
        for make in makers {
            let v = probe_err_with(&stoa, make);
            let reason = v["reason"].as_str().unwrap().to_string();
            assert!(
                !seen.contains(&reason),
                "two keystore states produced the same reason: {reason}"
            );
            seen.push(reason);
        }
    }

    #[test]
    fn no_passphrase_and_a_wrong_passphrase_are_different_reasons() {
        // Named separately from the sweep above because this is the pair a
        // reader is most likely to think is one state. "Set the variable" and
        // "you set it to the wrong thing" send a user to completely different
        // places, and merging them leaves someone re-typing a passphrase that
        // was never being read.
        let stoa = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());
        let locked = probe_err_with(&stoa, || KeystoreError::Locked);
        let wrong = probe_err_with(&stoa, || KeystoreError::WrongPassphrase);
        assert_ne!(locked["reason"], wrong["reason"]);
        // And each says enough to act on: one names the variable to set, the
        // other says the supplied value was rejected.
        assert!(
            locked["reason"]
                .as_str()
                .unwrap()
                .contains("DIALECTICA_PASSPHRASE"),
            "got {locked}"
        );
        assert!(
            wrong["reason"].as_str().unwrap().contains("rejected"),
            "got {wrong}"
        );
    }

    #[test]
    fn a_keystore_failure_is_an_answer_and_not_an_error_reply() {
        // The deliberate departure. A view handling both "you cannot post,
        // because X" and "I could not determine whether you can post" has two
        // negative branches, and the second has no sensible rendering.
        let stoa = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());
        let makers: [fn() -> KeystoreError; 4] = [
            || KeystoreError::NotFound,
            || KeystoreError::Io("disk on fire".into()),
            || KeystoreError::NotAKeystore,
            || KeystoreError::PermissionsTooOpen { mode: 0o777 },
        ];
        for make in makers {
            let v = probe_err_with(&stoa, make);
            assert!(
                v.get("error").is_none(),
                "a keystore state must not become the error shape, got {v}"
            );
            assert_eq!(v["canPost"], false);
        }
    }

    #[test]
    fn a_malformed_request_is_the_error_shape_rather_than_a_capability() {
        // The other side of the same line: a malformed REQUEST is a caller bug
        // and not a user state, so it is the one thing here that is still
        // §2.5's error shape. Answering `canPost:false` to unparseable input
        // would tell a view something about the keystore that was never
        // checked.
        for bad in [
            "not json",
            r#"{}"#,
            r#"{"stoa":7}"#,
            r#"{"stoa":"nothex"}"#,
            r#"{"stoa":"00ff"}"#,
        ] {
            let out = get_capabilities(bad, |_| Ok("abcd".to_string()));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v.get("canPost").is_none(),
                "a failure must never also carry a result — §2.5"
            );
        }
    }

    #[test]
    fn the_probe_reports_the_identity_for_the_stoa_it_was_asked_about() {
        // §5.2 gives a user one identity PER STOA, so "who would post" has no
        // answer until a Stoa is named. A probe that ignored the field would
        // report one Stoa's pseudonym while the user posted under another's.
        let seen = std::cell::RefCell::new(None);
        let asked = stoa_address(b"stoa two");
        let _ = get_capabilities(&format!(r#"{{"stoa":"{}"}}"#, asked.to_hex()), |s| {
            *seen.borrow_mut() = Some(*s);
            Ok("abcd".to_string())
        });
        assert_eq!(seen.into_inner(), Some(asked));
    }

    #[test]
    fn the_probe_is_never_a_panic_even_when_the_lookup_panics() {
        // The guard, on the one handler a view calls before rendering
        // anything. A panic here does not make one button unavailable — it
        // aborts the module process (PHASE0-FINDINGS §3) and the entire
        // interface is unrenderable.
        let out = get_capabilities(&format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()), |_| {
            panic!("the keystore layer exploded")
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("canPost").is_none());
    }

    #[test]
    fn the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies() {
        // Hardcoded strings, both of them. The contract is
        // `{"canPost":bool, "identity":"…" | "reason":"…"}`, and a view is
        // written against those exact key names — renaming one is a breaking
        // change that no type checker would catch.
        assert_eq!(
            Capability::CanPost {
                identity: "abcd".into()
            }
            .to_json(),
            r#"{"canPost":true,"identity":"abcd"}"#
        );
        assert_eq!(
            Capability::CannotPost {
                reason: "no keystore".into()
            }
            .to_json(),
            r#"{"canPost":false,"reason":"no keystore"}"#
        );
    }

    #[test]
    fn a_reason_containing_json_metacharacters_stays_valid_json() {
        // A reason carries an OS error message, which can carry a path, which
        // a user chose. Naive concatenation emits malformed JSON and the view
        // sees a parse error instead of the reason.
        let out = Capability::CannotPost {
            reason: r#"at "C:\keys" — he said "no""#.into(),
        }
        .to_json();
        let v: serde_json::Value = serde_json::from_str(&out).expect("must stay valid JSON");
        assert_eq!(v["reason"], r#"at "C:\keys" — he said "no""#);
    }

    // ─── Onboarding ───────────────────────────────────────────────────────

    use crate::identity_store::IdentityStore;
    use crate::keystore::{Keystore, Passphrase, Unlock};
    use crate::onboarding::{SlateNonce, SLATE_SIZE};

    /// A fresh temporary directory, and its guard.
    ///
    /// Copied in shape from `log/sqlite.rs`'s `TempDir`, for the reason it records:
    /// one need, in tests, is not worth a `tempfile` dependency. The guard must be
    /// held for the test's lifetime.
    ///
    /// **`log/sqlite.rs`'s and not `keystore.rs`'s**, which this comment used to
    /// credit as well. They are two different collision strategies: this one and
    /// `log/sqlite.rs`'s put `std::process::id()` in the name and so need the
    /// pre-emptive `remove_dir_all` below, because a name is reused within one run;
    /// `keystore.rs`'s takes 8 random bytes from `getrandom` and needs no removal.
    /// A reader told they are "the same shape" and asked to change one has been
    /// pointed at the wrong precedent — readability review caught it, and noted that
    /// `identity_store.rs`'s equivalent comment gets this right, in the same change.
    ///
    /// This is the **fourth** near-copy of the helper in the crate (`keystore.rs`,
    /// `log/sqlite.rs`, `identity_store.rs`, here). Four is where CLAUDE.md's "the
    /// fourth slightly-different copy of a guard" starts to apply, and "one need, in
    /// tests" was the argument for hand-rolling the first. Flagged rather than
    /// unified: whoever needs a fifth should decide deliberately instead of adding
    /// it, and unifying four test fixtures is not this change's business.
    struct OnboardingDir(std::path::PathBuf);

    impl OnboardingDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("dialectica-onboard-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
            OnboardingDir(path)
        }

        fn keystore_path(&self) -> std::path::PathBuf {
            crate::keystore::default_path_in(&self.0)
        }

        fn paths(&self) -> IdentityStore {
            IdentityStore::open(&IdentityStore::default_path_in(&self.0))
                .expect("a fresh identity record opens")
        }
    }

    impl Drop for OnboardingDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A keystore with a FIXED root, so expectations can be derived independently
    /// of what the code under test produced.
    fn a_master_key() -> Keystore {
        Keystore::from_root_for_test([7u8; 32])
    }

    fn slate_request() -> String {
        format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex())
    }

    /// A session whose keystore is the FIXED test root, as if one were on disk.
    ///
    /// The opener returns `Ok`, so `keystore_for` takes the "a keystore exists"
    /// branch and never mints — which is what most tests want, because they assert
    /// against values derived independently from `[7u8; 32]`.
    fn a_session() -> OnboardingSession {
        OnboardingSession::new()
    }

    /// Generate a slate through the wire handler, returning the reply and the
    /// nonce the handler chose to remember.
    fn slate_through_the_wire() -> (serde_json::Value, Option<SlateNonce>) {
        let mut session = a_session();
        let out = generate_identity_slate(&mut session, &slate_request(), || Ok(a_master_key()));
        let v = serde_json::from_str(&out)
            .unwrap_or_else(|e| panic!("the slate reply must be valid JSON ({e}): {out}"));
        (v, session.live_slate())
    }

    #[test]
    fn a_slate_reply_carries_the_fixed_count_and_that_many_candidates() {
        // The spec: "the reply carries the fixed number of candidates, AND states
        // that number alongside them". Both halves, and the count is checked
        // against the hardcoded 5 rather than against the array's own length —
        // otherwise the assertion is the reply agreeing with itself.
        let (v, _) = slate_through_the_wire();
        assert_eq!(v["count"], 5, "got {v}");
        assert_eq!(v["candidates"].as_array().unwrap().len(), 5, "got {v}");
        assert_eq!(SLATE_SIZE, 5, "the fixed count and the constant must agree");
    }

    #[test]
    fn a_slate_reply_takes_no_count_from_the_caller() {
        // The spec's reason is a security one: a caller-supplied count is "a
        // number that decides how much key derivation this module performs". There
        // is no field to pass, so the check is that offering one changes nothing.
        let ignored = format!(r#"{{"stoa":"{}","count":500}}"#, a_stoa().to_hex());
        let out = generate_identity_slate(&mut a_session(), &ignored, || Ok(a_master_key()));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["count"], 5, "a caller-supplied count was honoured: {out}");
        assert_eq!(v["candidates"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against() {
        // Hardcoded key names, following
        // `the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies`:
        // a view reads these exact names and renaming one is a breaking change no
        // type checker would catch.
        //
        // # The key set is asserted EXACTLY, not merely for presence
        //
        // This checked `is_some()` per expected field and nothing about extra ones,
        // while its name and its doc comment both claimed "the exact shape" — so it
        // had one fewer guarantee than the three sibling reply shapes, which pin
        // their whole serialised string, and the difference was invisible from
        // either. The spec-test reviewer measured the hole: adding a `displayName`
        // to every candidate left all 553 tests green.
        //
        // That matters here specifically because of what the spec says about it. The
        // scenario "A generated name and a mark are not settled by this capability"
        // requires that **no candidate carries a display name or a visual mark**, on
        // the reasoning that a caller written against one "would be written against a
        // name this capability never defined". A presence-only check cannot fail on
        // that scenario at all.
        //
        // The key set rather than the whole string, because a candidate's values are
        // derived and a whole-string pin would be asserting on the derivation too.
        // `assert_eq!` on a sorted `Vec` rather than `contains` per name, so an
        // ADDED key fails and not only a removed one.
        //
        // NO SPEC: `path` is in this set, and no requirement or scenario in
        // `specs/identity-onboarding/spec.md` names a reply field for it — every
        // mention of "path" there is about derivation, recording, or the record's
        // readability. Exposing it is a decision: it is not secret (the spec says
        // the record "reveals nothing that a published identity does not already
        // reveal"), and a view that can show which path is about to be kept is one
        // that can render the recovery warning truthfully. Marked because it is a
        // widening of the core API that the contract does not require, so a later
        // reader can decide whether it was right rather than inheriting it by
        // silence. It appears in the keep and whoami replies too.
        let (v, _) = slate_through_the_wire();
        let mut top: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        top.sort_unstable();
        assert_eq!(
            top,
            ["candidates", "count", "slate"],
            "the slate reply's top-level key set changed: {v}"
        );
        for candidate in v["candidates"].as_array().unwrap() {
            let mut keys: Vec<&str> = candidate
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                ["address", "index", "path", "publicKey"],
                "a candidate's key set changed. An ADDED key fails here too, which \
                 is the point: the spec requires no candidate carry a display name \
                 or a visual mark, and this capability does not define either. {v}"
            );
        }
        // And the candidates are indexed 0..5 in order, because a caller selects
        // by index and an index that did not match the position would select the
        // wrong candidate.
        for (position, candidate) in v["candidates"].as_array().unwrap().iter().enumerate() {
            assert_eq!(candidate["index"], position);
        }
    }

    #[test]
    fn a_slate_reply_carries_an_address_and_a_public_key_for_every_candidate() {
        // The spec requires both, with a reason for each: the address "is the only
        // unforgeable way to tell two candidates apart", and the public key
        // because "the generated display name is derived from the public key
        // rather than from the address".
        //
        // Checked as PARSEABLE values of the right length, not merely present — a
        // field holding the empty string would satisfy a presence check and be
        // useless to a view.
        let (v, _) = slate_through_the_wire();
        for candidate in v["candidates"].as_array().unwrap() {
            let address = candidate["address"].as_str().unwrap();
            assert!(
                crate::identity::Address::from_hex(address).is_ok(),
                "a candidate's address does not parse: {address}"
            );
            let key = hex::decode(candidate["publicKey"].as_str().unwrap()).unwrap();
            assert!(
                crate::identity::PublicKey::from_bytes(&key).is_ok(),
                "a candidate's public key does not parse"
            );
        }
    }

    #[test]
    fn no_secret_appears_anywhere_in_a_slate_reply() {
        // The spec, at the boundary the view actually reads from rather than only
        // in the slate type's own tests. Searched over the raw REPLY STRING, which
        // is stronger than checking fields by name because it catches a field
        // somebody adds later.
        //
        // The master key is `[7; 32]`, so the hex it would appear as is `07` x 32.
        // Checked in both cases, because a reply is lowercase hex and a future one
        // might not be.
        let out =
            generate_identity_slate(&mut a_session(), &slate_request(), || Ok(a_master_key()));
        let master_hex = "07".repeat(32);
        assert!(
            !out.contains(&master_hex) && !out.contains(&master_hex.to_uppercase()),
            "the master key's hex appears in the slate reply: {out}"
        );

        // And every candidate's secret key, derived independently here.
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        for candidate in v["candidates"].as_array().unwrap() {
            let path = candidate["path"].as_u64().unwrap() as u32;
            let secret = hex::encode(
                crate::identity::derive_stoa_key_at_path(&[7u8; 32], &a_stoa(), path).to_bytes(),
            );
            assert!(
                !out.contains(&secret),
                "a candidate's secret key hex appears in the slate reply: {out}"
            );
        }

        // The detection must work, or the assertions above prove nothing: a value
        // that IS in the reply must be found.
        let present = v["candidates"][0]["address"].as_str().unwrap();
        assert!(
            out.contains(present),
            "the search is broken, so the assertions above prove nothing"
        );
    }

    #[test]
    fn generating_a_slate_writes_nothing() {
        // The spec: "Generating a slate SHALL NOT write to storage", and
        // "a caller asking who the user is still finds none".
        //
        // Checked by generating several slates against a real directory and then
        // asserting the directory is still EMPTY — which is stronger than
        // asserting a particular file is absent, because it catches a write to a
        // name this test did not think of.
        let dir = OnboardingDir::new("slate-writes-nothing");
        // ONE session across the three slates, which is what the module has. A
        // session per iteration would also pass and would be testing less: the
        // property is that generating repeatedly writes nothing, and a fresh
        // session each time hides whether the held key is the thing being written.
        let mut session = a_session();
        for _ in 0..3 {
            let out =
                generate_identity_slate(&mut session, &slate_request(), || Ok(a_master_key()));
            assert!(
                serde_json::from_str::<serde_json::Value>(&out)
                    .unwrap()
                    .get("candidates")
                    .is_some(),
                "got {out}"
            );
        }
        let entries: Vec<_> = std::fs::read_dir(&dir.0).unwrap().collect();
        assert!(
            entries.is_empty(),
            "generating a slate wrote {} entries",
            entries.len()
        );

        // And who-am-i still finds nobody, which is the half a view would notice.
        let v: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
            || Ok(dir.paths()),
        ))
        .unwrap();
        assert_eq!(v["hasIdentity"], false, "got {v}");
    }

    #[test]
    fn two_slates_in_a_row_offer_different_candidates() {
        // The spec: "no candidate in the second set has a public key from the
        // first". Through the wire rather than only through the slate type,
        // because a handler that cached its reply would satisfy the type's test
        // and fail this one.
        let (first, _) = slate_through_the_wire();
        let (second, _) = slate_through_the_wire();
        let keys = |v: &serde_json::Value| {
            v["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["publicKey"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        for key in keys(&first) {
            assert!(
                !keys(&second).contains(&key),
                "the second slate reoffered {key}"
            );
        }
        assert_ne!(first["slate"], second["slate"], "the nonce must be fresh");
    }

    #[test]
    fn a_malformed_slate_request_is_the_error_shape_and_carries_no_candidates() {
        // §2.5: never a partial success. A reply carrying both an error and an
        // empty `candidates` list would render as "no identities available" in any
        // view that checked `candidates` first.
        for bad in [
            "not json",
            r#"{}"#,
            r#"{"stoa":7}"#,
            r#"{"stoa":"nothex"}"#,
            r#"{"stoa":"00ff"}"#,
            r#"{"stoa":null}"#,
        ] {
            let out = generate_identity_slate(&mut a_session(), bad, || Ok(a_master_key()));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v.get("candidates").is_none() && v.get("count").is_none(),
                "a failure must never also carry a result — §2.5, got {out}"
            );
        }
    }

    #[test]
    fn a_malformed_slate_request_does_not_supersede_the_live_slate() {
        // The subtle half of the above: a refused request must not have set the live
        // slate, or a malformed call would invalidate a slate the user is still
        // looking at — and their next selection would be refused for a reason that
        // had nothing to do with them.
        //
        // Asserted against a real, live slate rather than against `None`, which is
        // the stronger form: a handler that cleared the slot on failure and one that
        // left it alone both leave `None` behind when nothing was ever live, so
        // starting from `None` cannot tell them apart.
        let mut session = a_session();
        let good = generate_identity_slate(&mut session, &slate_request(), || Ok(a_master_key()));
        assert!(
            serde_json::from_str::<serde_json::Value>(&good)
                .unwrap()
                .get("candidates")
                .is_some(),
            "the setup slate must succeed, got {good}"
        );
        let live = session
            .live_slate()
            .expect("a successful slate makes its nonce live");

        for bad in ["not json", r#"{}"#, r#"{"stoa":"nothex"}"#] {
            let _ = generate_identity_slate(&mut session, bad, || Ok(a_master_key()));
            assert_eq!(
                session.live_slate(),
                Some(live),
                "the refused request {bad:?} disturbed the live slate"
            );
        }
    }

    #[test]
    fn a_keystore_that_cannot_be_opened_is_the_error_shape_rather_than_an_empty_slate() {
        // A slate needs the master key, so a keystore failure is a failure to
        // answer rather than a slate with nothing in it. The reason must reach the
        // view, since `KeystoreError::Display` is what names the fix.
        let out = generate_identity_slate(&mut a_session(), &slate_request(), || {
            Err(crate::keystore::KeystoreError::PermissionsTooOpen { mode: 0o644 })
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("candidates").is_none());
        assert!(
            v["error"].as_str().unwrap().contains("chmod 600"),
            "the fix must reach the view, got {out}"
        );
    }

    // ─── Keeping a candidate ──────────────────────────────────────────────

    /// Keep a candidate through the wire, against a real keystore path and record.
    fn keep_through_the_wire(
        dir: &OnboardingDir,
        nonce: SlateNonce,
        live: Option<SlateNonce>,
        index: i64,
        unlock: &Unlock,
    ) -> serde_json::Value {
        let mut session = a_session();
        // Set the live slate directly rather than by generating one, so a test can
        // present a nonce that is stale, forged or absent — the cases this helper
        // exists to reach.
        session.set_live_slate_for_test(live);
        let paths = dir.paths();
        let request = format!(
            r#"{{"stoa":"{}","slate":"{}","index":{index}}}"#,
            a_stoa().to_hex(),
            nonce.to_hex()
        );
        let out = keep_identity(
            &mut session,
            &request,
            || Ok(a_master_key()),
            KeepTargets {
                keystore_path: &dir.keystore_path(),
                unlock,
                paths: &paths,
            },
        );
        serde_json::from_str(&out)
            .unwrap_or_else(|e| panic!("the keep reply must be valid JSON ({e}): {out}"))
    }

    #[test]
    fn keeping_a_candidate_stores_it_and_reports_what_was_kept() {
        let dir = OnboardingDir::new("keep-stores");
        let nonce = SlateNonce::generate().unwrap();
        let v = keep_through_the_wire(&dir, nonce, Some(nonce), 2, &Unlock::Unencrypted);
        assert_eq!(v["kept"], true, "got {v}");

        // The expected address is derived HERE, independently, from the fixed
        // master key and the path the reply named — not read back from the reply's
        // own address field.
        let path = v["path"].as_u64().unwrap() as u32;
        let expected = crate::identity::derive_stoa_key_at_path(&[7u8; 32], &a_stoa(), path)
            .public_key()
            .address()
            .to_hex();
        assert_eq!(v["address"], expected, "got {v}");
        assert!(v.get("reason").is_none(), "got {v}");

        // And the path that reached the record is the one reported, checked
        // through the store rather than through the reply.
        assert_eq!(dir.paths().path_for(&a_stoa()).unwrap(), Some(path));
    }

    #[test]
    fn a_kept_identity_survives_a_restart_and_is_the_one_reported() {
        // The spec: "the identity reported is the one that was kept", after "the
        // stored state is then loaded afresh" — and twice, because the spec
        // requires it survive more than one restart and a store that consumed its
        // content on read would pass a single reload.
        let dir = OnboardingDir::new("keep-survives");
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&dir, nonce, Some(nonce), 1, &Unlock::Unencrypted);
        let kept_address = kept["address"].as_str().unwrap().to_string();

        for reload in 0..2 {
            let v: serde_json::Value = serde_json::from_str(&who_am_i(
                &slate_request(),
                || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
                || Ok(dir.paths()),
            ))
            .unwrap();
            assert_eq!(v["hasIdentity"], true, "reload {reload}: {v}");
            assert_eq!(
                v["address"], kept_address,
                "reload {reload} reported a different identity than was kept"
            );
        }
    }

    #[test]
    fn a_kept_identity_can_sign_as_the_identity_it_reported() {
        // The spec: "an op signed by the identity it yields has as its author the
        // identity that keeping it reported". END TO END through a real keystore
        // on disk and a real signature — every other keep test compares addresses,
        // so none of them could see a keep that reported one identity while the
        // user posted under another.
        use crate::identity::{sign_op_bytes, verify_authored_op, Address};

        let dir = OnboardingDir::new("keep-signs");
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&dir, nonce, Some(nonce), 0, &Unlock::Unencrypted);
        let reported =
            Address::from_hex(kept["address"].as_str().unwrap()).expect("a parseable address");
        let path = kept["path"].as_u64().unwrap() as u32;

        // Reopened from disk, not the in-memory keystore the keep used — the
        // question is whether what was PERSISTED signs as what was reported.
        let reloaded = Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted).unwrap();
        let key = reloaded.stoa_key_at_path(&a_stoa(), path);
        let sig = sign_op_bytes(&key, b"a post");
        assert!(
            verify_authored_op(
                &reported,
                &key.public_key().to_bytes(),
                b"a post",
                &sig.to_bytes()
            ),
            "an op signed by the kept identity is not attributed to the reported address"
        );

        // The negative: another path's key must not verify, or the assertion
        // above would hold for any key.
        let other = reloaded.stoa_key_at_path(&a_stoa(), path.wrapping_add(1));
        assert!(!verify_authored_op(
            &reported,
            &other.public_key().to_bytes(),
            b"a post",
            &sign_op_bytes(&other, b"a post").to_bytes()
        ));
    }

    #[test]
    fn a_selection_against_a_superseded_slate_is_refused_rather_than_satisfied() {
        // The spec's sharpest requirement on this path: the attempt "is refused
        // rather than storing a candidate from the second" set.
        //
        // So the assertion is not merely that an error came back — it is that
        // NOTHING was stored. A handler that refused and wrote anyway would pass a
        // weaker version of this test.
        let dir = OnboardingDir::new("superseded");
        let first = SlateNonce::generate().unwrap();
        let second = SlateNonce::generate().unwrap();
        assert_ne!(first, second);

        let v = keep_through_the_wire(&dir, first, Some(second), 0, &Unlock::Unencrypted);
        assert_eq!(v["kept"], false, "got {v}");
        assert!(v.get("address").is_none(), "got {v}");
        assert_eq!(
            dir.paths().path_for(&a_stoa()).unwrap(),
            None,
            "a superseded selection recorded a path"
        );
        assert!(
            !dir.keystore_path().exists(),
            "a superseded selection wrote a keystore"
        );
    }

    #[test]
    fn a_selection_with_no_live_slate_at_all_is_refused() {
        // Distinct from the superseded case in how a user reaches it — a restart
        // between generating and keeping — and the same refusal, because there is
        // one useful answer to both: generate a slate and choose from it.
        let dir = OnboardingDir::new("no-live-slate");
        let nonce = SlateNonce::generate().unwrap();
        let v = keep_through_the_wire(&dir, nonce, None, 0, &Unlock::Unencrypted);
        assert_eq!(v["kept"], false, "got {v}");
        assert!(
            v["reason"].as_str().unwrap().contains("generate"),
            "the reason must name the fix, got {v}"
        );
        assert!(!dir.keystore_path().exists());
    }

    #[test]
    fn a_selection_outside_the_set_is_refused_and_stores_nothing() {
        // The spec: "the attempt is refused, AND no identity is stored". Coercing
        // an out-of-range selection would store an identity the user did not
        // choose, which the spec calls unrecoverable.
        let dir = OnboardingDir::new("out-of-range");
        let nonce = SlateNonce::generate().unwrap();
        for index in [SLATE_SIZE as i64, SLATE_SIZE as i64 + 1, 99, 100_000] {
            let v = keep_through_the_wire(&dir, nonce, Some(nonce), index, &Unlock::Unencrypted);
            assert_eq!(v["kept"], false, "index {index}: {v}");
            assert!(v.get("address").is_none(), "index {index}: {v}");
            assert!(
                !dir.keystore_path().exists(),
                "index {index} wrote a keystore"
            );
            assert_eq!(dir.paths().path_for(&a_stoa()).unwrap(), None);
        }
        // And every in-range index IS accepted, or the refusals above could be
        // unconditional and these assertions would still pass.
        let ok = keep_through_the_wire(&dir, nonce, Some(nonce), 4, &Unlock::Unencrypted);
        assert_eq!(ok["kept"], true, "got {ok}");
    }

    #[test]
    fn a_second_keep_is_refused_and_leaves_the_stored_identity_unchanged() {
        // The spec: "the attempt is refused, AND the stored identity is
        // unchanged". The second assertion is the load-bearing one — a refusal
        // that replaced the master key anyway would satisfy the first and destroy
        // every identity derived from the old one, with no error saying so.
        let dir = OnboardingDir::new("second-keep");
        let nonce = SlateNonce::generate().unwrap();
        let first = keep_through_the_wire(&dir, nonce, Some(nonce), 0, &Unlock::Unencrypted);
        assert_eq!(first["kept"], true, "got {first}");
        let before = std::fs::read(dir.keystore_path()).unwrap();

        let second = keep_through_the_wire(&dir, nonce, Some(nonce), 3, &Unlock::Unencrypted);
        assert_eq!(second["kept"], false, "got {second}");
        assert_eq!(
            std::fs::read(dir.keystore_path()).unwrap(),
            before,
            "a refused second keep rewrote the keystore"
        );
        assert_eq!(
            dir.paths().path_for(&a_stoa()).unwrap(),
            Some(first["path"].as_u64().unwrap() as u32),
            "a refused second keep changed the recorded path"
        );
    }

    #[test]
    fn a_keep_whose_keystore_write_fails_records_no_path() {
        // THE WRITE ORDER, and nothing else in this file pins it. Reversing the
        // two writes left the whole suite green until this test existed —
        // measured, not assumed.
        //
        // The observable difference is exactly the bad state `design.md` names: a
        // recorded path naming a master key that does not exist, which the NEXT
        // keep — with a different master key — would silently inherit.
        //
        // The keystore write is made to fail by putting a DIRECTORY where the
        // keystore file goes, so `create` cannot write there. That is a failure of
        // the keystore write specifically, with the path record perfectly healthy,
        // which is the only fixture that separates the two orders.
        let dir = OnboardingDir::new("keystore-write-fails");
        std::fs::create_dir_all(dir.keystore_path()).unwrap();
        let paths = dir.paths();
        let nonce = SlateNonce::generate().unwrap();
        let request = format!(
            r#"{{"stoa":"{}","slate":"{}","index":0}}"#,
            a_stoa().to_hex(),
            nonce.to_hex()
        );

        let mut session = a_session();
        session.set_live_slate_for_test(Some(nonce));
        let out = keep_identity(
            &mut session,
            &request,
            || Ok(a_master_key()),
            KeepTargets {
                keystore_path: &dir.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &paths,
            },
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["kept"], false,
            "the fixture must fail the keystore write, got {out}"
        );

        // The assertion the ordering exists for: nothing reached the record.
        assert_eq!(
            paths.path_for(&a_stoa()).unwrap(),
            None,
            "a failed keystore write left a recorded path behind — the writes are \
             in the wrong order"
        );

        // And the spec's "a subsequent load finds no identity that was not there
        // before": who-am-i must still find nobody.
        let who: serde_json::Value = serde_json::from_str(
            &whoami_for(&a_stoa(), || Ok(a_master_key()), || Ok(dir.paths())).to_json(),
        )
        .unwrap();
        assert_eq!(who["hasIdentity"], false, "got {who}");
    }

    #[test]
    fn the_second_keep_refusal_is_distinguishable_from_other_failures() {
        // The spec: the reason must be "distinguishable from a malformed request
        // and from a storage failure". Three states, three distinct outcomes —
        // and the malformed one is the ERROR shape rather than a refusal, which is
        // the strongest form of distinguishable.
        let dir = OnboardingDir::new("refusal-kinds");
        let nonce = SlateNonce::generate().unwrap();
        keep_through_the_wire(&dir, nonce, Some(nonce), 0, &Unlock::Unencrypted);

        let already = keep_through_the_wire(&dir, nonce, Some(nonce), 1, &Unlock::Unencrypted);
        let already_reason = already["reason"].as_str().unwrap();

        // A storage failure: the record's own file replaced by a directory, so
        // opening it fails. Reached through a different `OnboardingDir` so the
        // already-exists case is not also in play.
        let broken = OnboardingDir::new("refusal-storage");
        let record_path = IdentityStore::default_path_in(&broken.0);
        std::fs::create_dir_all(&record_path).unwrap();
        let storage_failure = IdentityStore::open(&record_path);
        assert!(
            storage_failure.is_err(),
            "the fixture must actually fail, or this test proves nothing"
        );
        let storage_reason = storage_failure.unwrap_err().to_string();

        assert_ne!(
            already_reason, storage_reason,
            "an existing identity and a storage failure must not read alike"
        );

        // And a malformed request is §2.5's error shape, not a refusal at all.
        let mut session = a_session();
        session.set_live_slate_for_test(Some(nonce));
        let malformed = keep_identity(
            &mut session,
            r#"{"stoa":"nothex"}"#,
            || Ok(a_master_key()),
            KeepTargets {
                keystore_path: &broken.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &dir.paths(),
            },
        );
        let v: serde_json::Value = serde_json::from_str(&malformed).unwrap();
        assert!(v.get("error").is_some(), "got {malformed}");
        assert!(v.get("kept").is_none(), "got {malformed}");
    }

    #[test]
    fn the_keep_reply_reports_whether_the_master_key_was_encrypted() {
        // The spec: an encrypted store reports encrypted, an unencrypted one
        // reports unencrypted. Both directions, because a field hardcoded to
        // either value would satisfy one of them.
        let plain = OnboardingDir::new("report-plain");
        let nonce = SlateNonce::generate().unwrap();
        let v = keep_through_the_wire(&plain, nonce, Some(nonce), 0, &Unlock::Unencrypted);
        assert_eq!(v["kept"], true, "got {v}");
        assert_eq!(
            v["encrypted"], false,
            "an unencrypted store must report unencrypted: {v}"
        );

        let encrypted = OnboardingDir::new("report-encrypted");
        let unlock = Unlock::Passphrase(Passphrase::new(b"a real passphrase"));
        let v = keep_through_the_wire(&encrypted, nonce, Some(nonce), 0, &unlock);
        assert_eq!(v["kept"], true, "got {v}");
        assert_eq!(
            v["encrypted"], true,
            "an encrypted store must report encrypted: {v}"
        );

        // And the report agrees with the FILE, not merely with the argument: the
        // keystore's own inspection must say the same thing. This is what would
        // catch a reply whose boolean had drifted from what was written.
        assert!(Keystore::is_encrypted(&encrypted.keystore_path()).unwrap());
        assert!(!Keystore::is_encrypted(&plain.keystore_path()).unwrap());
    }

    #[test]
    fn the_keep_json_is_pinned_to_the_exact_shape_a_view_is_written_against() {
        // Hardcoded strings, both shapes, following the capability probe's
        // precedent.
        assert_eq!(
            Kept::Stored {
                address: "aa".into(),
                public_key: "bb".into(),
                path: 7,
                encrypted: true,
            }
            .to_json(),
            r#"{"address":"aa","encrypted":true,"kept":true,"path":7,"publicKey":"bb"}"#
        );
        assert_eq!(
            Kept::Refused {
                reason: "no slate".into()
            }
            .to_json(),
            r#"{"kept":false,"reason":"no slate"}"#
        );
    }

    #[test]
    fn a_malformed_keep_request_is_the_error_shape_and_carries_no_result() {
        // §2.5, on the method where a partial success would be worst: a reply
        // carrying an error and a `kept:true` beside it would tell a view an
        // identity exists that was never written.
        let dir = OnboardingDir::new("keep-malformed");
        let nonce = SlateNonce::generate().unwrap();
        let paths = dir.paths();
        let stoa = a_stoa().to_hex();
        for bad in [
            "not json".to_string(),
            r#"{}"#.to_string(),
            format!(r#"{{"stoa":"{stoa}"}}"#),
            format!(r#"{{"stoa":"{stoa}","slate":7}}"#),
            format!(r#"{{"stoa":"{stoa}","slate":"nothex"}}"#),
            format!(r#"{{"stoa":"{stoa}","slate":"00ff"}}"#),
            format!(r#"{{"stoa":"{stoa}","slate":"{}"}}"#, nonce.to_hex()),
            format!(
                r#"{{"stoa":"{stoa}","slate":"{}","index":-1}}"#,
                nonce.to_hex()
            ),
            format!(
                r#"{{"stoa":"{stoa}","slate":"{}","index":1.5}}"#,
                nonce.to_hex()
            ),
            format!(
                r#"{{"stoa":"{stoa}","slate":"{}","index":"two"}}"#,
                nonce.to_hex()
            ),
        ] {
            let mut session = a_session();
            session.set_live_slate_for_test(Some(nonce));
            let out = keep_identity(
                &mut session,
                &bad,
                || Ok(a_master_key()),
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            );
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v.get("kept").is_none() && v.get("address").is_none(),
                "a failure must never also carry a result — §2.5, got {out}"
            );
            assert!(
                !dir.keystore_path().exists(),
                "a malformed request for {bad:?} wrote a keystore"
            );
        }
    }

    // ─── Who the user is ──────────────────────────────────────────────────

    #[test]
    fn who_am_i_reports_an_identity_with_its_address_and_public_key() {
        // The spec: "the reply states that there is an identity, AND carries its
        // address and its public key, AND carries no reason". All three.
        let dir = OnboardingDir::new("whoami-yes");
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&dir, nonce, Some(nonce), 2, &Unlock::Unencrypted);

        let v: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
            || Ok(dir.paths()),
        ))
        .unwrap();
        assert_eq!(v["hasIdentity"], true, "got {v}");
        assert_eq!(v["address"], kept["address"], "got {v}");
        assert!(v.get("reason").is_none(), "got {v}");
        // The public key must be present AND must be the one the address derives
        // from, which is the only way the "display name is derived from the public
        // key" requirement is useful.
        let key = hex::decode(v["publicKey"].as_str().unwrap()).unwrap();
        assert_eq!(
            crate::identity::PublicKey::from_bytes(&key)
                .unwrap()
                .address()
                .to_hex(),
            v["address"].as_str().unwrap(),
            "the reported public key does not derive the reported address: {v}"
        );
    }

    #[test]
    fn who_am_i_reports_nobody_with_a_reason_when_no_identity_is_stored() {
        // The spec: "the reply states that there is none, AND carries a reason,
        // AND carries no identity".
        let dir = OnboardingDir::new("whoami-none");
        let v: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
            || Ok(dir.paths()),
        ))
        .unwrap();
        assert_eq!(v["hasIdentity"], false, "got {v}");
        assert!(v.get("address").is_none(), "got {v}");
        assert!(v.get("publicKey").is_none(), "got {v}");
        assert!(!v["reason"].as_str().unwrap().is_empty(), "got {v}");
    }

    #[test]
    fn an_unusable_identity_is_distinguishable_from_an_absent_one() {
        // The spec's requirement, and the reason the method exists separately from
        // the posting probe: "a stored identity whose keystore permissions are too
        // open is a real identity that cannot currently be used", and a caller
        // told "you are nobody" would render the wrong thing.
        //
        // Four states, and all four reasons must differ.
        let absent = whoami_for(
            &a_stoa(),
            || Err(crate::keystore::KeystoreError::NotFound),
            || Ok(IdentityStore::in_memory().unwrap()),
        );
        let unusable = whoami_for(
            &a_stoa(),
            || Err(crate::keystore::KeystoreError::PermissionsTooOpen { mode: 0o644 }),
            || Ok(IdentityStore::in_memory().unwrap()),
        );
        let locked = whoami_for(
            &a_stoa(),
            || Err(crate::keystore::KeystoreError::Locked),
            || Ok(IdentityStore::in_memory().unwrap()),
        );
        // The state the two-store split creates: a master key with no recorded
        // path for this Stoa.
        let unchosen = whoami_for(
            &a_stoa(),
            || Ok(a_master_key()),
            || Ok(IdentityStore::in_memory().unwrap()),
        );

        let reason = |w: &Whoami| match w {
            Whoami::Nobody { reason } => reason.clone(),
            Whoami::Identity { .. } => panic!("expected nobody, got an identity"),
        };
        let reasons = [
            reason(&absent),
            reason(&unusable),
            reason(&locked),
            reason(&unchosen),
        ];
        for (i, a) in reasons.iter().enumerate() {
            for b in reasons.iter().skip(i + 1) {
                assert_ne!(a, b, "two states produced the same reason");
            }
        }
        // And the unchosen state's reason names the record rather than reading as
        // "you have no identity", which is the whole point of listing it.
        assert!(
            reason(&unchosen).contains("chosen") || reason(&unchosen).contains("slate"),
            "the unchosen reason must name what is missing, got {}",
            reason(&unchosen)
        );
    }

    #[test]
    fn who_am_i_reports_that_recovery_needs_more_than_the_master_key() {
        // The spec: "a caller asking whether recovery needs more than the master
        // key is told that it does", while paths are recorded and no export
        // exists. A user who believes their exported master key is a complete
        // backup "has been misled by omission", and the caller has no filesystem
        // access to discover it for itself.
        let dir = OnboardingDir::new("whoami-recovery");
        let nonce = SlateNonce::generate().unwrap();
        keep_through_the_wire(&dir, nonce, Some(nonce), 0, &Unlock::Unencrypted);

        let v: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
            || Ok(dir.paths()),
        ))
        .unwrap();
        assert_eq!(
            v["recoveryNeedsTheRecord"], true,
            "the unbacked state must be reported, got {v}"
        );
    }

    #[test]
    fn distinct_stoas_report_distinct_identities() {
        // §5.2 gives a user one identity PER STOA, and the path record is keyed by
        // Stoa. A handler that ignored the field would report one Stoa's identity
        // while the user posted under another's.
        let dir = OnboardingDir::new("whoami-per-stoa");
        let store = dir.paths();
        let here = a_stoa();
        let elsewhere = stoa_address(b"another stoa");
        store.record_path(&here, 1).unwrap();
        store.record_path(&elsewhere, 2).unwrap();

        let ask = |stoa: &Address| whoami_for(stoa, || Ok(a_master_key()), || Ok(dir.paths()));
        let (a, b) = (ask(&here), ask(&elsewhere));
        match (&a, &b) {
            (
                Whoami::Identity {
                    address: addr_a,
                    path: path_a,
                    ..
                },
                Whoami::Identity {
                    address: addr_b,
                    path: path_b,
                    ..
                },
            ) => {
                assert_eq!(*path_a, 1);
                assert_eq!(*path_b, 2);
                assert_ne!(addr_a, addr_b, "two Stoas reported one address");
                // And each address is the one the recorded path derives, computed
                // here rather than read from the reply.
                for (stoa, path, addr) in [(&here, 1u32, addr_a), (&elsewhere, 2, addr_b)] {
                    assert_eq!(
                        addr,
                        &crate::identity::derive_stoa_key_at_path(&[7u8; 32], stoa, path)
                            .public_key()
                            .address()
                            .to_hex()
                    );
                }
            }
            other => panic!("expected two identities, got {other:?}"),
        }
    }

    #[test]
    fn the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against() {
        assert_eq!(
            Whoami::Identity {
                address: "aa".into(),
                public_key: "bb".into(),
                path: 7,
                recovery_needs_the_record: true,
            }
            .to_json(),
            r#"{"address":"aa","hasIdentity":true,"path":7,"publicKey":"bb","recoveryNeedsTheRecord":true}"#
        );
        assert_eq!(
            Whoami::Nobody {
                reason: "no keystore".into()
            }
            .to_json(),
            r#"{"hasIdentity":false,"reason":"no keystore"}"#
        );
    }

    #[test]
    fn a_malformed_whoami_request_is_the_error_shape_and_carries_no_identity() {
        for bad in [
            "not json",
            r#"{}"#,
            r#"{"stoa":7}"#,
            r#"{"stoa":"nothex"}"#,
            r#"{"stoa":"00ff"}"#,
        ] {
            let out = who_am_i(
                bad,
                || Ok(a_master_key()),
                || Ok(IdentityStore::in_memory().unwrap()),
            );
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v.get("hasIdentity").is_none() && v.get("address").is_none(),
                "a failure must never also carry a result — §2.5, got {out}"
            );
        }
    }

    #[test]
    fn who_am_i_is_an_answer_and_not_an_error_for_every_storage_state() {
        // The posting probe's posture, applied here: a state that prevents naming
        // an identity is reported as "nobody, because X" rather than as the call
        // failing. A view handling both would have two negative branches and the
        // second has no sensible rendering.
        let makers: [fn() -> crate::keystore::KeystoreError; 5] = [
            || crate::keystore::KeystoreError::NotFound,
            || crate::keystore::KeystoreError::Io("disk on fire".into()),
            || crate::keystore::KeystoreError::NotAKeystore,
            || crate::keystore::KeystoreError::WrongPassphrase,
            || crate::keystore::KeystoreError::Truncated,
        ];
        for make in makers {
            let out = who_am_i(
                &slate_request(),
                || Err(make()),
                || Ok(IdentityStore::in_memory().unwrap()),
            );
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("error").is_none(),
                "a storage state became the error shape: {out}"
            );
            assert_eq!(v["hasIdentity"], false, "got {out}");
        }

        // And a failure of the RECORD, not only of the keystore.
        let out = who_am_i(
            &slate_request(),
            || Ok(a_master_key()),
            || {
                Err(crate::identity_store::IdentityStoreError::Storage(
                    "unable to open database file".into(),
                ))
            },
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "got {out}");
        assert_eq!(v["hasIdentity"], false);
        assert!(
            v["reason"]
                .as_str()
                .unwrap()
                .contains("unable to open database file"),
            "the reason must reach the view, got {out}"
        );
    }

    #[test]
    fn no_onboarding_handler_panics_whatever_it_is_given_or_whatever_fails() {
        // The guard, on three methods a view calls during onboarding. A panic here
        // does not make one button unavailable — it aborts the module process
        // (PHASE0-FINDINGS §3) and the entire interface is unrenderable.
        //
        // Two axes: arbitrary INPUT, and a dependency that panics. The second is
        // the one a request-shaped sweep would miss.
        let dir = OnboardingDir::new("no-panics");
        let paths = dir.paths();
        let stoa = a_stoa().to_hex();
        let nonce = SlateNonce::generate().unwrap();

        for input in [
            "",
            "not json",
            "null",
            "[]",
            "0",
            r#""a string""#,
            r#"{}"#,
            r#"{"stoa":null,"slate":null,"index":null}"#,
            r#"{"stoa":[],"slate":{},"index":[]}"#,
            &format!(
                r#"{{"stoa":"{stoa}","slate":"{}","index":18446744073709551616}}"#,
                nonce.to_hex()
            ),
            "\u{0}\u{1}\u{2}",
        ] {
            let mut slate_session = a_session();
            let mut keep_session = a_session();
            keep_session.set_live_slate_for_test(Some(nonce));
            for out in [
                generate_identity_slate(&mut slate_session, input, || Ok(a_master_key())),
                keep_identity(
                    &mut keep_session,
                    input,
                    || Ok(a_master_key()),
                    KeepTargets {
                        keystore_path: &dir.keystore_path(),
                        unlock: &Unlock::Unencrypted,
                        paths: &paths,
                    },
                ),
                who_am_i(
                    input,
                    || Ok(a_master_key()),
                    || Ok(IdentityStore::in_memory().unwrap()),
                ),
            ] {
                serde_json::from_str::<serde_json::Value>(&out).unwrap_or_else(|e| {
                    panic!("a handler emitted invalid JSON for {input:?} ({e}): {out}")
                });
            }
        }

        // A panicking dependency, which the guard has to convert rather than let
        // through.
        let out = generate_identity_slate(&mut a_session(), &slate_request(), || {
            panic!("the keystore layer exploded")
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("candidates").is_none());

        // The keep's opener is a dependency too, and it is the one the session
        // reaches through — so a panic there is inside the session's borrow, which
        // is the arrangement most likely to be got wrong.
        let mut panicking = a_session();
        panicking.set_live_slate_for_test(Some(nonce));
        let out = keep_identity(
            &mut panicking,
            &format!(
                r#"{{"stoa":"{stoa}","slate":"{}","index":0}}"#,
                nonce.to_hex()
            ),
            || panic!("the keystore layer exploded during a keep"),
            KeepTargets {
                keystore_path: &dir.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &paths,
            },
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v.get("kept").is_none(),
            "§2.5: never a partial success, got {out}"
        );

        let out = who_am_i(
            &slate_request(),
            || Ok(a_master_key()),
            || panic!("the record layer exploded"),
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("hasIdentity").is_none());
    }

    // ─── Onboarding: the tester's independent coverage ─────────────────────
    //
    // Written from the `identity-onboarding` spec rather than from the code, and
    // each one was watched to fail under a named mutation before it was kept.
    // Where a test overlaps one above, the reason is stated.

    #[test]
    fn a_keep_whose_path_record_fails_reports_failure_and_names_no_identity() {
        // THE OTHER HALF OF THE WRITE ORDER, and nothing in this file covered it.
        // `a_keep_whose_keystore_write_fails_records_no_path` pins the direction
        // where the FIRST write fails, which is the clean case: nothing was
        // written anywhere. It cannot see the case `design.md` calls "the one
        // partial state" — the keystore succeeded and the record did not.
        //
        // That case is what the spec's "A failed keep records nothing" actually
        // costs, and the spec is checkable on it: "no identity is reported as
        // kept, AND a subsequent load finds no identity that was not there
        // before". Both halves are asserted here.
        //
        // The record write is made to fail by handing the keep a store whose
        // TABLE has been dropped out from under it — the connection is live, so
        // `record_path` reaches SQLite and SQLite refuses. Putting a directory
        // where the file goes would fail at `open`, which is a different arm and
        // would never reach `record_path` at all.
        let dir = OnboardingDir::new("record-write-fails");
        let paths = dir.paths();
        {
            // A second connection to the same file, dropping the table. The
            // keep's own store keeps its handle, so the failure happens at the
            // write rather than at the open.
            let saboteur = rusqlite::Connection::open(IdentityStore::default_path_in(&dir.0))
                .expect("the record file opens");
            saboteur
                .execute_batch("DROP TABLE chosen_paths;")
                .expect("the table is droppable");
        }

        let nonce = SlateNonce::generate().unwrap();
        let request = format!(
            r#"{{"stoa":"{}","slate":"{}","index":0}}"#,
            a_stoa().to_hex(),
            nonce.to_hex()
        );
        let mut session = a_session();
        session.set_live_slate_for_test(Some(nonce));
        let out = keep_identity(
            &mut session,
            &request,
            || Ok(a_master_key()),
            KeepTargets {
                keystore_path: &dir.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &paths,
            },
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        // The fixture must actually fail the RECORD write and not something
        // earlier, or this test proves nothing. The keystore file existing is
        // what says the first write got through.
        assert!(
            dir.keystore_path().exists(),
            "the fixture failed before the keystore write, so it does not \
             exercise the partial state: {out}"
        );

        // The spec: "no identity is reported as kept".
        assert_eq!(
            v["kept"], false,
            "a keep whose record write failed reported success: {out}"
        );
        assert!(v.get("address").is_none(), "got {out}");
        assert!(v.get("path").is_none(), "got {out}");

        // The spec: "a subsequent load finds no identity that was not there
        // before". `whoAmI` must not name one — which is only true because it
        // reads the RECORD, not the keystore.
        //
        // The store is opened here rather than through `OnboardingDir::paths`,
        // because the sabotaged file no longer opens and `paths` panics on that.
        // A load that CANNOT read the record is still a load that must not name
        // an identity, so the failure is handed to the handler as the answer it
        // is — which is the state a real user would be in.
        let record_path = IdentityStore::default_path_in(&dir.0);
        let who: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted),
            || IdentityStore::open(&record_path),
        ))
        .unwrap();
        assert_eq!(
            who["hasIdentity"], false,
            "a keep that did not complete left an identity reportable: {who}"
        );
        assert!(who.get("address").is_none(), "got {who}");
    }

    #[test]
    fn each_keep_refusal_reason_is_pinned_to_its_own_situation() {
        // `the_second_keep_refusal_is_distinguishable_from_other_failures`
        // asserts that two reasons DIFFER without pinning either, and its
        // storage-failure reason is built outside the handler — so it never
        // observes what a keep actually says about a storage failure. Both
        // reasons could become unhelpful in different ways and it would pass.
        //
        // This pins each reason to a substring chosen from the SPEC's own
        // vocabulary for the situation, so a message that stopped naming its
        // situation fails even while remaining distinct from the others.
        //
        // Three situations, one table — CLAUDE.md's rule, and it is also what
        // makes the pairwise-distinctness check below free.
        let dir = OnboardingDir::new("pinned-refusals");
        let nonce = SlateNonce::generate().unwrap();
        let other = SlateNonce::generate().unwrap();
        assert_ne!(nonce, other);

        // First keep succeeds, so the second reaches the already-exists arm.
        let first = keep_through_the_wire(&dir, nonce, Some(nonce), 0, &Unlock::Unencrypted);
        assert_eq!(first["kept"], true, "got {first}");

        // A storage failure reached THROUGH the handler: the record's table is
        // dropped, in a directory with no keystore yet, so the keystore write
        // succeeds and the record write fails. That is the only way to observe
        // what a keep says about a storage failure.
        let broken = OnboardingDir::new("pinned-refusals-storage");
        let broken_paths = broken.paths();
        {
            let saboteur =
                rusqlite::Connection::open(IdentityStore::default_path_in(&broken.0)).unwrap();
            saboteur.execute_batch("DROP TABLE chosen_paths;").unwrap();
        }
        let mut broken_session = a_session();
        broken_session.set_live_slate_for_test(Some(nonce));
        let storage_out = keep_identity(
            &mut broken_session,
            &format!(
                r#"{{"stoa":"{}","slate":"{}","index":0}}"#,
                a_stoa().to_hex(),
                nonce.to_hex()
            ),
            || Ok(a_master_key()),
            KeepTargets {
                keystore_path: &broken.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &broken_paths,
            },
        );
        let storage: serde_json::Value = serde_json::from_str(&storage_out).unwrap();
        assert_eq!(storage["kept"], false, "got {storage_out}");

        // (situation, reply, a word the reason must carry)
        //
        // Each expected word is hardcoded from what the SITUATION is, not read
        // back from the message: an existing identity is about a keystore that
        // already exists, a superseded slate is about generating a fresh one, an
        // out-of-range selection is about choosing from the set, and a storage
        // failure is about the record being unreadable.
        let cases = [
            (
                "an identity already exists",
                keep_through_the_wire(&dir, nonce, Some(nonce), 1, &Unlock::Unencrypted),
                "already exists",
            ),
            (
                "the slate was superseded",
                keep_through_the_wire(&dir, other, Some(nonce), 0, &Unlock::Unencrypted),
                "no longer the current one",
            ),
            (
                "the selection names no candidate",
                keep_through_the_wire(&dir, nonce, Some(nonce), 99, &Unlock::Unencrypted),
                "there is no candidate 99",
            ),
            (
                "the record could not be written",
                storage,
                "read or written",
            ),
        ];

        let mut reasons: Vec<String> = Vec::new();
        for (situation, reply, must_contain) in &cases {
            assert_eq!(reply["kept"], false, "{situation}: {reply}");
            let reason = reply["reason"]
                .as_str()
                .unwrap_or_else(|| panic!("{situation} carried no reason: {reply}"))
                .to_string();
            assert!(
                reason.contains(must_contain),
                "{situation}: the reason does not name the situation. Expected a \
                 reason containing {must_contain:?}, got {reason:?}"
            );
            reasons.push(reason);
        }

        // And still pairwise distinct, which pinning each one already implies but
        // which is the spec's own wording ("distinguishable from").
        for (i, a) in reasons.iter().enumerate() {
            for b in reasons.iter().skip(i + 1) {
                assert_ne!(a, b, "two situations produced the same reason");
            }
        }
    }

    #[test]
    fn a_failed_keep_leaves_the_record_no_fuller_than_it_found_it() {
        // The spec's "A failed keep records nothing" has a second clause the
        // tests above read past: "a subsequent load finds no identity THAT WAS
        // NOT THERE BEFORE". That is a statement about the record as a whole, not
        // about the Stoa being kept — so it is checkable by counting rows across
        // a failure, which nothing else here does.
        //
        // The fixture puts a pre-existing choice for a DIFFERENT Stoa in the
        // record, so a handler that cleared or rewrote the record on failure
        // would be caught, and so would one that recorded the failed Stoa.
        let dir = OnboardingDir::new("failed-keep-leaves-record");
        let paths = dir.paths();
        let elsewhere = stoa_address(b"a stoa kept earlier");
        paths.record_path(&elsewhere, 11).unwrap();

        // The expected content is hardcoded, not read back from the store.
        let before = vec![crate::identity_store::ChosenPath {
            stoa: elsewhere,
            path: 11,
        }];
        assert_eq!(paths.all_paths().unwrap(), before);

        let nonce = SlateNonce::generate().unwrap();
        let stale = SlateNonce::generate().unwrap();
        assert_ne!(nonce, stale);

        // Three ways to fail a keep for `a_stoa()`, none of which may touch the
        // record: a superseded nonce, no live slate, and an out-of-range index.
        for (situation, live, index) in [
            ("a superseded slate", Some(stale), 0i64),
            ("no live slate", None, 0),
            ("an index outside the set", Some(nonce), 77),
        ] {
            let mut session = a_session();
            session.set_live_slate_for_test(live);
            let out = keep_identity(
                &mut session,
                &format!(
                    r#"{{"stoa":"{}","slate":"{}","index":{index}}}"#,
                    a_stoa().to_hex(),
                    nonce.to_hex()
                ),
                || Ok(a_master_key()),
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            );
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert_eq!(v["kept"], false, "{situation}: {out}");
            assert_eq!(
                paths.all_paths().unwrap(),
                before,
                "{situation} changed the record"
            );
        }
    }

    #[test]
    fn a_slate_is_offered_for_the_stoa_it_was_asked_about() {
        // The spec makes identity per-Stoa, and a slate is generated for one
        // Stoa. Nothing in this file checks that the handler uses the `stoa`
        // field it parsed rather than some other value — the existing slate tests
        // all ask about one Stoa, so a handler that hardcoded a Stoa would pass
        // every one of them.
        //
        // Asserted against candidates derived HERE from the fixed master key and
        // the paths the reply named, so the expectation does not come from the
        // reply's own address field.
        let here = a_stoa();
        let elsewhere = stoa_address(b"a different stoa entirely");
        assert_ne!(here, elsewhere);

        for stoa in [here, elsewhere] {
            let request = format!(r#"{{"stoa":"{}"}}"#, stoa.to_hex());
            let out = generate_identity_slate(&mut a_session(), &request, || Ok(a_master_key()));
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("the reply must be JSON ({e}): {out}"));
            for candidate in v["candidates"].as_array().unwrap() {
                let path = candidate["path"].as_u64().unwrap() as u32;
                let expected =
                    crate::identity::derive_stoa_key_at_path(&[7u8; 32], &stoa, path).public_key();
                assert_eq!(
                    candidate["publicKey"],
                    expected.to_hex(),
                    "a candidate is not derived for the Stoa that was asked about: {v}"
                );
                assert_eq!(candidate["address"], expected.address().to_hex(), "got {v}");
            }
        }
    }

    #[test]
    fn a_kept_identity_is_the_candidate_the_slate_offered_at_that_index() {
        // The spec's keep scenarios say the identity kept is the one that was
        // chosen, and the selection is BY INDEX. Every keep test above derives
        // its expectation from the path the REPLY named — which cannot see a keep
        // that stored index 3's path while reporting it as the kept one, because
        // reply and record would agree with each other.
        //
        // This instead generates the slate, reads candidate `n`'s public key from
        // the SLATE reply, and requires the keep at index `n` to report exactly
        // that. The two replies come from two calls, so agreeing is a property of
        // the code rather than of one value being copied.
        for index in 0..SLATE_SIZE {
            let dir = OnboardingDir::new(&format!("keep-is-the-offered-one-{index}"));
            let mut session = a_session();
            let slate_out =
                generate_identity_slate(&mut session, &slate_request(), || Ok(a_master_key()));
            let slate: serde_json::Value = serde_json::from_str(&slate_out).unwrap();
            let nonce = session.live_slate().expect("a slate was remembered");
            let offered = slate["candidates"][index].clone();

            let kept =
                keep_through_the_wire(&dir, nonce, Some(nonce), index as i64, &Unlock::Unencrypted);
            assert_eq!(kept["kept"], true, "index {index}: {kept}");
            assert_eq!(
                kept["publicKey"], offered["publicKey"],
                "index {index}: the keep stored a candidate the slate did not \
                 offer at that index. Offered {offered}, kept {kept}"
            );
            assert_eq!(kept["address"], offered["address"], "index {index}");
            assert_eq!(kept["path"], offered["path"], "index {index}");
        }
    }

    #[test]
    fn on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed() {
        // THE REGRESSION TEST for the defect three reviewers found independently,
        // and the reason `OnboardingSession` exists.
        //
        // `a_kept_identity_is_the_candidate_the_slate_offered_at_that_index` above
        // is the test written for this property and it could not see the defect,
        // because it hands BOTH calls the same fixed `[7u8; 32]` keystore. A fixture
        // that supplies one key to both cannot distinguish "the slate and the keep
        // agree about the master key" from "the harness gave them the same one" —
        // this project's recorded defect family, two explanations for one answer,
        // here at the fixture boundary rather than inside a fixture.
        //
        // So this test does the one thing that fixture cannot: the opener reports
        // NO KEYSTORE, which is the fresh install that onboarding exists for and
        // the only state where minting happens at all. If the key is minted per
        // call, the slate shows candidates of key A, the keep writes key B, and
        // the addresses differ — which is exactly what was measured.
        //
        // No fixed root anywhere. The assertion is a relationship between two
        // replies, not a comparison against a constant, so it holds whatever the
        // minted key turns out to be — and it cannot be satisfied by the fixture
        // handing the same value to both sides, because the fixture supplies none.
        for index in 0..SLATE_SIZE {
            let dir = OnboardingDir::new(&format!("fresh-install-keep-{index}"));
            let paths = dir.paths();
            let mut session = a_session();

            // A fresh install: no keystore file, so the session mints.
            let slate_out = generate_identity_slate(&mut session, &slate_request(), || {
                Err(crate::keystore::KeystoreError::NotFound)
            });
            let slate: serde_json::Value = serde_json::from_str(&slate_out)
                .unwrap_or_else(|e| panic!("the slate reply must be JSON ({e}): {slate_out}"));
            assert!(
                slate.get("candidates").is_some(),
                "a slate must be available on a fresh install, got {slate_out}"
            );
            let offered = slate["candidates"][index].clone();
            let nonce = session.live_slate().expect("a slate was remembered");

            // The keep, through the SAME session — which is the whole fix — and with
            // the keystore still absent from disk, as it is until this call writes
            // it.
            let kept_out = keep_identity(
                &mut session,
                &format!(
                    r#"{{"stoa":"{}","slate":"{}","index":{index}}}"#,
                    a_stoa().to_hex(),
                    nonce.to_hex()
                ),
                || Err(crate::keystore::KeystoreError::NotFound),
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            );
            let kept: serde_json::Value = serde_json::from_str(&kept_out)
                .unwrap_or_else(|e| panic!("the keep reply must be JSON ({e}): {kept_out}"));

            assert_eq!(kept["kept"], true, "index {index}: {kept_out}");
            assert_eq!(
                kept["address"], offered["address"],
                "index {index}: on a fresh install the identity KEPT is not the \
                 candidate the slate SHOWED. The user chose {offered} and was given \
                 {kept}. The address is the only unforgeable way to tell two \
                 candidates apart, so this is the choice being silently replaced."
            );
            assert_eq!(kept["publicKey"], offered["publicKey"], "index {index}");
            assert_eq!(kept["path"], offered["path"], "index {index}");

            // And the identity that is actually ON DISK is that one too, read back
            // through a keystore opened from the written file rather than from the
            // session — so a session reporting its held key while writing another
            // is caught as well.
            let written = crate::keystore::Keystore::open(
                &dir.keystore_path(),
                &crate::keystore::Unlock::Unencrypted,
            )
            .expect("the keep wrote an openable keystore");
            let recorded = paths
                .path_for(&a_stoa())
                .expect("the record reads")
                .expect("the keep recorded a path");
            assert_eq!(
                written.stoa_address_at_path(&a_stoa(), recorded).to_hex(),
                offered["address"].as_str().unwrap(),
                "index {index}: the keystore on disk does not derive the identity \
                 the slate showed at the path that was recorded"
            );
        }
    }

    #[test]
    fn refreshing_a_slate_offers_candidates_of_one_master_key() {
        // The other half of holding the key: `docs/UI-BRIEF.md` tells a designer the
        // flow is "five identities, pick one, refresh for more". With a per-call
        // mint that was false — each refresh offered candidates of a DIFFERENT
        // master key, so "more" was the wrong word for what the button did.
        //
        // Asserted as a relationship rather than against a constant: two slates from
        // one session must differ in their paths (a fresh nonce each time) and agree
        // about the key those paths are derived under. The second half is checked by
        // deriving the first slate's candidate at the SECOND slate's path and
        // requiring it to match — which is only possible if one key produced both.
        let mut session = a_session();

        let first_out = generate_identity_slate(&mut session, &slate_request(), || {
            Err(crate::keystore::KeystoreError::NotFound)
        });
        let first: serde_json::Value = serde_json::from_str(&first_out).unwrap();
        let first_nonce = session.live_slate().expect("a slate is live");

        let second_out = generate_identity_slate(&mut session, &slate_request(), || {
            Err(crate::keystore::KeystoreError::NotFound)
        });
        let second: serde_json::Value = serde_json::from_str(&second_out).unwrap();
        let second_nonce = session.live_slate().expect("a slate is live");

        assert_ne!(
            first_nonce, second_nonce,
            "a refresh must offer a different set, so the nonce must move"
        );

        // The proof that one key produced both: reproduce the SECOND slate from the
        // FIRST slate's nonce-independent ingredient — the session's key — by asking
        // the session to keep a candidate of the second slate and checking the
        // address it reports is derived from the same root the first slate's
        // addresses were. The only value both slates share is that root, so a
        // per-call mint makes this impossible to satisfy.
        //
        // Done by elimination rather than by reading the root, which is deliberately
        // not accessible: for each candidate of the second slate, its address must
        // NOT appear in the first slate (different paths) while both slates' paths
        // must derive from one key — established by the keep below, which writes the
        // held key and lets the written file re-derive the first slate's candidates.
        let dir = OnboardingDir::new("refresh-one-key");
        let paths = dir.paths();
        let kept_out = keep_identity(
            &mut session,
            &format!(
                r#"{{"stoa":"{}","slate":"{}","index":0}}"#,
                a_stoa().to_hex(),
                second_nonce.to_hex()
            ),
            || Err(crate::keystore::KeystoreError::NotFound),
            KeepTargets {
                keystore_path: &dir.keystore_path(),
                unlock: &Unlock::Unencrypted,
                paths: &paths,
            },
        );
        let kept: serde_json::Value = serde_json::from_str(&kept_out).unwrap();
        assert_eq!(kept["kept"], true, "got {kept_out}");

        let written = crate::keystore::Keystore::open(
            &dir.keystore_path(),
            &crate::keystore::Unlock::Unencrypted,
        )
        .expect("the keep wrote an openable keystore");

        // The written key re-derives EVERY candidate of the FIRST slate. That is the
        // assertion: the key the second slate was kept under is the key the first
        // slate was offered under, which is what "refresh for more candidates of one
        // identity" means.
        for candidate in first["candidates"].as_array().unwrap() {
            let path = candidate["path"].as_u64().unwrap() as u32;
            assert_eq!(
                written.stoa_address_at_path(&a_stoa(), path).to_hex(),
                candidate["address"].as_str().unwrap(),
                "the first slate's candidate at path {path} is not derivable from \
                 the key the second slate's keep wrote — a refresh minted a new \
                 master key. First {first}, second {second}"
            );
        }
    }

    #[test]
    fn keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key() {
        // The regression test for the design review's first finding: `create`'s
        // `AlreadyExists` was doing double duty as the second-keep guard AND as the
        // per-Stoa gate, and the keystore is ONE FILE PER INSTALL — so a user who
        // kept an identity in Stoa A was refused in Stoa B, before `record_path` was
        // ever called, with a reason naming a keystore they did not know they had.
        //
        // `chosen_paths` could therefore never hold a second row through any wire
        // call, which made the spec's "Distinct choices for distinct Stoas are
        // recorded separately" unreachable through the API — and made `design.md`'s
        // claim that the primary key discharges that scenario false.
        //
        // Both multi-Stoa tests in this file write the second row with
        // `store.record_path(...)` directly, bypassing the handler, which is why
        // nothing saw it. This one goes through the handler twice.
        let dir = OnboardingDir::new("second-stoa");
        let paths = dir.paths();
        let first_stoa = a_stoa();
        let second_stoa = stoa_address(b"the second stoa");
        assert_ne!(first_stoa, second_stoa);

        let mut session = a_session();
        let keep_in = |session: &mut OnboardingSession, stoa: &Address, index: usize| {
            // The opener reports whatever is actually on disk, which is the honest
            // fixture: absent for the first keep, present for the second. A stub
            // that always said `NotFound` would let the second keep mint a fresh
            // key and hide the very thing this test is about.
            let slate_out = generate_identity_slate(
                session,
                &format!(r#"{{"stoa":"{}"}}"#, stoa.to_hex()),
                || {
                    crate::keystore::Keystore::open(
                        &dir.keystore_path(),
                        &crate::keystore::Unlock::Unencrypted,
                    )
                },
            );
            let slate: serde_json::Value = serde_json::from_str(&slate_out)
                .unwrap_or_else(|e| panic!("slate must be JSON ({e}): {slate_out}"));
            let offered = slate["candidates"][index].clone();
            let nonce = session.live_slate().expect("a slate is live");
            let kept_out = keep_identity(
                session,
                &format!(
                    r#"{{"stoa":"{}","slate":"{}","index":{index}}}"#,
                    stoa.to_hex(),
                    nonce.to_hex()
                ),
                || {
                    crate::keystore::Keystore::open(
                        &dir.keystore_path(),
                        &crate::keystore::Unlock::Unencrypted,
                    )
                },
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            );
            let kept: serde_json::Value = serde_json::from_str(&kept_out)
                .unwrap_or_else(|e| panic!("keep must be JSON ({e}): {kept_out}"));
            (offered, kept)
        };

        let (first_offered, first_kept) = keep_in(&mut session, &first_stoa, 0);
        assert_eq!(
            first_kept["kept"], true,
            "the first keep must succeed: {first_kept}"
        );

        let (second_offered, second_kept) = keep_in(&mut session, &second_stoa, 2);
        assert_eq!(
            second_kept["kept"], true,
            "keeping an identity in a SECOND Stoa was refused — the keystore's \
             one-file-per-install scope is being used as a per-Stoa gate. Got \
             {second_kept}"
        );

        // Each Stoa got the candidate its own slate showed.
        assert_eq!(second_kept["address"], second_offered["address"]);
        assert_eq!(first_kept["address"], first_offered["address"]);

        // Two rows, one per Stoa — the scenario the primary key is supposed to
        // discharge, now actually reachable. Expectations hardcoded from the
        // replies' own paths would be circular, so the check is on the SET of
        // Stoas and that the two paths differ.
        let all = paths.all_paths().expect("the record reads");
        assert_eq!(
            all.len(),
            2,
            "the record must hold one row per Stoa: {all:?}"
        );
        let mut stoas: Vec<_> = all.iter().map(|c| c.stoa.to_hex()).collect();
        stoas.sort();
        let mut expected = vec![first_stoa.to_hex(), second_stoa.to_hex()];
        expected.sort();
        assert_eq!(stoas, expected);

        // And the two identities are genuinely different, which is what makes
        // per-Stoa identity mean anything.
        assert_ne!(
            first_kept["address"], second_kept["address"],
            "two Stoas must not report one identity"
        );

        // One master key, reused rather than replaced: the written keystore
        // re-derives BOTH kept addresses at their recorded paths. A second keep that
        // had minted and written a new key would break the first Stoa's identity
        // silently, which is the failure the old `AlreadyExists` refusal was
        // (accidentally) preventing — so this is the assertion that has to replace
        // it.
        let written = crate::keystore::Keystore::open(
            &dir.keystore_path(),
            &crate::keystore::Unlock::Unencrypted,
        )
        .expect("the keystore on disk opens");
        for (stoa, kept) in [(&first_stoa, &first_kept), (&second_stoa, &second_kept)] {
            let path = paths
                .path_for(stoa)
                .expect("the record reads")
                .expect("a path is recorded");
            assert_eq!(
                written.stoa_address_at_path(stoa, path).to_hex(),
                kept["address"].as_str().unwrap(),
                "the keystore on disk does not derive the identity kept for Stoa {}",
                stoa.to_hex()
            );
        }
    }

    #[test]
    fn a_second_keep_for_one_stoa_is_still_refused_after_the_second_stoa_fix() {
        // The other side of the change above, and the reason it is not a
        // weakening. The spec requires that keeping an identity NOT replace an
        // existing one, because replacing "discards every identity derived from it,
        // while the ops those identities signed remain published and unreachable".
        //
        // That refusal moved from `Keystore::create` to `chosen_paths`' primary key.
        // If the move had lost it, the second-Stoa fix would have bought a
        // reachable second Stoa at the price of a silently replaceable identity —
        // which is far worse than the bug it fixed. So this test exists beside that
        // one deliberately: they constrain each other.
        let dir = OnboardingDir::new("second-keep-one-stoa");
        let paths = dir.paths();
        let mut session = a_session();

        let open = || {
            crate::keystore::Keystore::open(
                &dir.keystore_path(),
                &crate::keystore::Unlock::Unencrypted,
            )
        };
        let slate_out = generate_identity_slate(&mut session, &slate_request(), open);
        assert!(
            serde_json::from_str::<serde_json::Value>(&slate_out)
                .unwrap()
                .get("candidates")
                .is_some(),
            "got {slate_out}"
        );
        let nonce = session.live_slate().expect("a slate is live");

        let keep_at = |session: &mut OnboardingSession, index: usize| -> serde_json::Value {
            let out = keep_identity(
                session,
                &format!(
                    r#"{{"stoa":"{}","slate":"{}","index":{index}}}"#,
                    a_stoa().to_hex(),
                    nonce.to_hex()
                ),
                open,
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            );
            serde_json::from_str(&out).unwrap_or_else(|e| panic!("JSON ({e}): {out}"))
        };

        let first = keep_at(&mut session, 0);
        assert_eq!(first["kept"], true, "got {first}");
        let stored_path = first["path"].as_u64().unwrap() as u32;
        let keystore_bytes = std::fs::read(dir.keystore_path()).expect("the keystore is readable");

        // A different index, so a refusal that silently replaced would be visible in
        // the recorded path.
        let second = keep_at(&mut session, 3);
        assert_eq!(
            second["kept"], false,
            "a second choice for ONE Stoa must be refused: {second}"
        );
        assert!(
            second["reason"].as_str().unwrap_or_default().len() > 10,
            "the refusal must carry a reason a user can act on: {second}"
        );

        // Nothing changed: not the record, and not the master key.
        assert_eq!(
            paths.path_for(&a_stoa()).expect("the record reads"),
            Some(stored_path),
            "a refused second keep changed the recorded path"
        );
        assert_eq!(
            std::fs::read(dir.keystore_path()).unwrap(),
            keystore_bytes,
            "a refused second keep rewrote the master key"
        );
    }

    #[test]
    fn the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa() {
        // The regression test for the finding architecture and security review
        // reached independently: `getCapabilities` derived the reported identity
        // through the PATHLESS trio under salt `/dialectica/1/…` while `whoAmI` used
        // the PATH-TAKING one under `/dialectica/2/…`. `identity.rs`'s own test
        // asserts those two schemes must disagree — so two shipped wire methods
        // answered "who posts here" with two different addresses for one user in one
        // Stoa, and no field in either reply told them apart.
        //
        // `posting-capability`'s requirement is explicit that the probe's identity
        // "SHALL be the one an op published now would be attributed to, derived from
        // the key that would actually sign it", and names the failure: "the user
        // sees one handle and posts under another."
        //
        // No test could see it while the derivation lived in the adapter's closure:
        // that file is `#[cfg(logos_scaffold)]` and every probe test injects a stub
        // returning a literal. Moving the choice into `posting_identity` is what
        // makes this assertable, and the assertion is between two methods rather
        // than against a constant.
        let dir = OnboardingDir::new("probe-agrees-with-whoami");
        let paths = dir.paths();
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&dir, nonce, Some(nonce), 2, &Unlock::Unencrypted);
        assert_eq!(kept["kept"], true, "got {kept}");

        let request = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());

        let probe_out =
            get_capabilities_from_stores(&request, || Ok(a_master_key()), || Ok(dir.paths()));
        let probe: serde_json::Value = serde_json::from_str(&probe_out)
            .unwrap_or_else(|e| panic!("the probe reply must be JSON ({e}): {probe_out}"));

        let who_out = who_am_i(&request, || Ok(a_master_key()), || Ok(dir.paths()));
        let who: serde_json::Value = serde_json::from_str(&who_out)
            .unwrap_or_else(|e| panic!("the whoami reply must be JSON ({e}): {who_out}"));

        assert_eq!(probe["canPost"], true, "got {probe_out}");
        assert_eq!(who["hasIdentity"], true, "got {who_out}");
        assert_eq!(
            probe["identity"], who["address"],
            "getCapabilities and whoAmI report DIFFERENT addresses for one user in \
             one Stoa. The probe says {probe}, whoAmI says {who}. A view rendering \
             'you are posting as X' from the probe and 'you are X' from whoAmI shows \
             two identities and has no way to decide which one signs."
        );

        // And both agree with what was actually kept, so "they agree" cannot be
        // satisfied by both being wrong in the same way.
        assert_eq!(
            probe["identity"], kept["address"],
            "the probe's identity is not the one the keep stored"
        );

        // The strongest form: an op signed by the key at the recorded path verifies
        // against the address the probe reported. That is the requirement's own
        // wording — "derived from the key that would actually sign it" — rather than
        // a comparison of two derivations that could both be wrong.
        let recorded = paths
            .path_for(&a_stoa())
            .expect("the record reads")
            .expect("a path is recorded");
        let signing = a_master_key().stoa_key_at_path(&a_stoa(), recorded);
        let sig = crate::identity::sign_op_bytes(&signing, b"a post");
        let author = Address::from_hex(probe["identity"].as_str().unwrap())
            .expect("the probe reports a parseable address");
        assert!(
            crate::identity::verify_authored_op(
                &author,
                &signing.public_key().to_bytes(),
                b"a post",
                &sig.to_bytes()
            ),
            "an op signed by the key at the recorded path is not attributed to the \
             address the probe reported"
        );
    }

    #[test]
    fn the_key_a_publish_signs_with_is_the_identity_the_probe_reports() {
        // THE REGRESSION TEST for the second defect this change closes, and it
        // is the requirement's own scenario: "WHEN the posting-capability probe
        // reports an identity for a Stoa and a post is then published into that
        // Stoa, THEN the published op's author is the identity the probe
        // reported."
        //
        // The publish path signed with `keystore.stoa_key(&stoa)` — the
        // PATHLESS per-Stoa scheme — while the probe reports
        // `stoa_address_at_path`. `identity.rs`'s
        // `the_path_taking_scheme_does_not_collide_with_the_pathless_one`
        // asserts the two MUST disagree, so this is not a near-miss: every op a
        // user published was authored by an identity no method on the surface
        // would ever name.
        //
        // It could not be tested where it lived. The call was in
        // `dialectica/rust-lib/src/lib.rs`, which `cargo test` does not
        // compile, and CI fenced it with a named exemption reading "Delete this
        // exemption when the spec decides". The spec had decided; this test is
        // what the choice moving into `core` makes possible.
        let dir = OnboardingDir::new("publish-signs-as-the-probe-says");
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&dir, nonce, Some(nonce), 2, &Unlock::Unencrypted);
        assert_eq!(kept["kept"], true, "got {kept}");

        let request = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());
        let probe_out =
            get_capabilities_from_stores(&request, || Ok(a_master_key()), || Ok(dir.paths()));
        let probe: serde_json::Value = serde_json::from_str(&probe_out)
            .unwrap_or_else(|e| panic!("the probe reply must be JSON ({e}): {probe_out}"));
        assert_eq!(probe["canPost"], true, "got {probe_out}");

        let signing = publishing_key(&a_stoa(), &a_master_key(), &dir.paths())
            .expect("a kept identity must yield a signing key");

        // Through the WIRE, not through `publishing_key` twice. Asking the
        // function that was just called what it returns would agree with itself
        // whatever it returns; what has to hold is that an op the publish
        // handler actually wrote carries the probe's address as its author.
        let mut log = MemoryOpLog::new();
        let publish = format!(
            r#"{{"stoa":"{}","body":"who signed this"}}"#,
            a_stoa().to_hex()
        );
        let out = publish_post(&publish, &mut log, &signing, &mut ignored_delivery);
        let published = as_json(&out);
        assert!(published.get("error").is_none(), "got {out}");

        let id = crate::op::OpId::from_hex(published["opId"].as_str().unwrap()).unwrap();
        let entry = log.get(&id).unwrap().expect("the op must be in the log");
        assert_eq!(
            entry.op.op.author.address().to_hex(),
            probe["identity"].as_str().unwrap(),
            "the published op's author is NOT the identity the probe reports. \
             The probe says {probe}, the op was signed as {}. A user is posting \
             under a handle no method on this surface will ever show them.",
            entry.op.op.author.address().to_hex()
        );

        // And the pathless key is the WRONG answer, asserted rather than
        // assumed — without this the test passes against an implementation
        // where the two schemes happen to coincide, which is the defect family
        // this project has recorded: two explanations giving one answer.
        let pathless = a_master_key().stoa_key(&a_stoa());
        assert_ne!(
            pathless.public_key().address().to_hex(),
            probe["identity"].as_str().unwrap(),
            "the pathless scheme agrees with the probe, so this test cannot \
             distinguish the fix from the defect"
        );
    }

    #[test]
    fn a_publish_is_refused_when_no_identity_has_been_chosen_for_the_stoa() {
        // The other half, and the half that stops the fix being "sign with
        // something". `posting_identity` reports CannotPost when no path is
        // recorded; if `publishing_key` fell back to any key, the probe would
        // say the user cannot post while the publish succeeded under a key the
        // probe refuses to name — which is a worse disagreement than the one
        // being fixed, because it is silent on the publishing side.
        //
        // One state, one reason: the same constant both other methods give.
        let dir = OnboardingDir::new("publish-with-no-choice");
        // `.err()` rather than `.expect_err()`: `SecretKey` has no `Debug`, on
        // purpose — a secret that can be formatted is a secret that reaches a
        // log — and `expect_err` requires one on the `Ok` type.
        let refused = publishing_key(&a_stoa(), &a_master_key(), &dir.paths())
            .err()
            .expect("no recorded path must not yield a key");
        assert_eq!(
            refused, NO_CHOICE_FOR_THIS_STOA,
            "a publish with no chosen identity must give the same reason the \
             probe and whoAmI give, got {refused:?}"
        );
    }

    #[test]
    fn the_probe_and_whoami_give_one_reason_when_no_choice_is_recorded_for_this_stoa() {
        // The state the two-store split creates: a master key exists, this Stoa has
        // no choice recorded. Both methods must name it, and name it the SAME way —
        // two methods describing one situation in two vocabularies is the split
        // re-appearing at the wire.
        //
        // `canPost:false` here is the substantive half: the probe previously
        // answered `true` with a pathless address, asserting posting ability for an
        // identity that has no recorded path and that nothing in the signing path
        // would ever use.
        let dir = OnboardingDir::new("probe-no-choice");
        let request = format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex());

        let probe_out =
            get_capabilities_from_stores(&request, || Ok(a_master_key()), || Ok(dir.paths()));
        let probe: serde_json::Value = serde_json::from_str(&probe_out).unwrap();
        let who_out = who_am_i(&request, || Ok(a_master_key()), || Ok(dir.paths()));
        let who: serde_json::Value = serde_json::from_str(&who_out).unwrap();

        assert_eq!(
            probe["canPost"], false,
            "a master key with no recorded choice for this Stoa cannot post as \
             anyone: {probe_out}"
        );
        assert!(probe.get("identity").is_none(), "got {probe_out}");
        assert_eq!(who["hasIdentity"], false, "got {who_out}");
        assert_eq!(
            probe["reason"], who["reason"],
            "the two methods describe one state in two ways: probe {probe}, \
             whoAmI {who}"
        );
        // Pinned to the constant, so a message that stopped naming the fix fails
        // even while the two still agree with each other.
        assert_eq!(probe["reason"], NO_CHOICE_FOR_THIS_STOA, "got {probe_out}");
    }

    #[test]
    fn a_record_restored_beside_a_master_key_names_the_identities_in_use() {
        // The spec's "A restore targets the device holding the master key": "the
        // identities in use are those the record names, AND no other device's
        // record participates". Nothing in this change covered it — the restore
        // path is not a code path, it is the property that a record and a master
        // key which never met each other in one process still agree.
        //
        // The fixture is a restore in the only sense that is checkable now: a
        // record file written by one store, COPIED to a fresh directory, and read
        // beside a keystore file also copied there. Neither handle nor connection
        // is shared, which is what makes it a restore rather than a reuse.
        let origin = OnboardingDir::new("restore-origin");
        let nonce = SlateNonce::generate().unwrap();
        let kept = keep_through_the_wire(&origin, nonce, Some(nonce), 3, &Unlock::Unencrypted);
        assert_eq!(kept["kept"], true, "got {kept}");
        let kept_address = kept["address"].as_str().unwrap().to_string();
        let kept_path = kept["path"].as_u64().unwrap() as u32;

        // The expected identity, derived HERE from the fixed master key and the
        // path — not read back from either file.
        let expected = crate::identity::derive_stoa_key_at_path(&[7u8; 32], &a_stoa(), kept_path)
            .public_key()
            .address()
            .to_hex();
        assert_eq!(kept_address, expected, "got {kept}");

        let restored = OnboardingDir::new("restore-target");
        // The target starts empty, or the "restore" would be reading what was
        // already there.
        assert!(
            std::fs::read_dir(&restored.0).unwrap().next().is_none(),
            "the restore target must start empty"
        );
        std::fs::copy(origin.keystore_path(), restored.keystore_path()).unwrap();
        std::fs::copy(
            IdentityStore::default_path_in(&origin.0),
            IdentityStore::default_path_in(&restored.0),
        )
        .unwrap();

        let v: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&restored.keystore_path(), &Unlock::Unencrypted),
            || Ok(restored.paths()),
        ))
        .unwrap();
        assert_eq!(v["hasIdentity"], true, "got {v}");
        assert_eq!(
            v["path"], kept_path,
            "the restored record names a different path: {v}"
        );
        assert_eq!(
            v["address"], expected,
            "the restored identity is not the one the record names: {v}"
        );

        // "No other device's record participates": a SECOND restore target given
        // the same master key but a record naming a DIFFERENT path must report
        // that path's identity, not the first's. Without this, a handler ignoring
        // the record entirely would satisfy everything above.
        let other = OnboardingDir::new("restore-other-device");
        std::fs::copy(origin.keystore_path(), other.keystore_path()).unwrap();
        let other_path = kept_path.wrapping_add(1);
        other.paths().record_path(&a_stoa(), other_path).unwrap();

        let w: serde_json::Value = serde_json::from_str(&who_am_i(
            &slate_request(),
            || Keystore::open(&other.keystore_path(), &Unlock::Unencrypted),
            || Ok(other.paths()),
        ))
        .unwrap();
        assert_eq!(w["hasIdentity"], true, "got {w}");
        assert_eq!(w["path"], other_path, "got {w}");
        assert_ne!(
            w["address"], v["address"],
            "two records naming different paths reported one identity, so the \
             record is not being read: {w}"
        );
        // And the expectation for it is also derived here.
        assert_eq!(
            w["address"],
            crate::identity::derive_stoa_key_at_path(&[7u8; 32], &a_stoa(), other_path)
                .public_key()
                .address()
                .to_hex(),
            "got {w}"
        );
    }

    // ─── The feed handler ─────────────────────────────────────────────────

    use crate::arrival::Arrival;
    use crate::log::{MemoryOpLog, OpLog};
    use crate::op::{Op, OpKind};
    use crate::stoa::{Genesis, Policy};

    fn feed_key(seed: u8) -> crate::identity::SecretKey {
        crate::identity::SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    fn feed_genesis() -> Genesis {
        Genesis {
            creator: feed_key(1).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
    }

    /// A log holding one thread head with this body.
    fn log_with_body(body: &str) -> MemoryOpLog {
        let key = feed_key(2);
        let op = Op {
            stoa: feed_genesis().address().unwrap(),
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let mut log = MemoryOpLog::new();
        log.append(op, Arrival::unordered()).unwrap();
        log
    }

    /// A log holding two threads, one of them hidden by the Stoa's moderator.
    ///
    /// Needed because `includeHidden` is the contract's worked example for a
    /// null-reads-as-absent field, and "the null took the restrictive default"
    /// cannot be asserted against a log where the flag changes nothing — both
    /// answers would be identical and the test would pass for the wrong reason.
    ///
    /// The hider is `feed_key(1)`, which is [`feed_genesis`]'s creator and
    /// therefore the Stoa's only moderator: a hide op signed by anyone else is
    /// unauthorised and filtered on read, so the thread would stay visible and
    /// the fixture would silently be the one-visible-thread case again.
    fn log_with_a_hidden_thread() -> MemoryOpLog {
        let stoa = feed_genesis().address().unwrap();
        let poster = feed_key(2);
        let visible = Op {
            stoa,
            author: poster.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "visible".to_string(),
                attachments: vec![],
            },
        }
        .sign(&poster);
        let to_hide = Op {
            stoa,
            author: poster.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "hidden".to_string(),
                attachments: vec![],
            },
        }
        .sign(&poster);
        let moderator = feed_key(1);
        let hide = Op {
            stoa,
            author: moderator.public_key(),
            kind: OpKind::Moderate {
                target: to_hide.op.id(),
                action: crate::op::ModerationAction::Hide,
            },
        }
        .sign(&moderator);

        let mut log = MemoryOpLog::new();
        log.append(visible, Arrival::unordered()).unwrap();
        log.append(to_hide, Arrival::unordered()).unwrap();
        log.append(hide, Arrival::unordered()).unwrap();
        log
    }

    fn feed_request(extra: &str) -> String {
        let stoa = feed_genesis().address().unwrap().to_hex();
        if extra.is_empty() {
            format!(r#"{{"stoa":"{stoa}"}}"#)
        } else {
            format!(r#"{{"stoa":"{stoa}",{extra}}}"#)
        }
    }

    #[test]
    fn the_feed_reply_is_the_ecosystems_pagination_shape() {
        // The precedent-setting shape, pinned by key name. A view is written
        // against these exact names and renaming one is a breaking change no
        // type checker would catch.
        let log = log_with_body("hello");
        let out = list_threads(&feed_request(""), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["items"].is_array(), "got {out}");
        assert_eq!(v["page"], 0);
        assert_eq!(v["hasMore"], false);

        let row = &v["items"][0];
        for field in [
            "thread",
            "currentVersion",
            "author",
            "body",
            "attachments",
            "isRevised",
            "isHidden",
        ] {
            assert!(row.get(field).is_some(), "row is missing {field}: {out}");
        }
        assert_eq!(row["body"]["text"], "hello");
    }

    #[test]
    fn a_sanitised_string_is_always_an_object_even_when_nothing_was_found() {
        // A shape that was sometimes a string and sometimes an object would
        // make every view branch on the type before rendering.
        let log = log_with_body("perfectly ordinary");
        let out = list_threads(&feed_request(""), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let body = &v["items"][0]["body"];
        assert!(body.is_object(), "got {out}");
        assert_eq!(body["removed"], 0);
        assert_eq!(body["marked"], 0);
    }

    #[test]
    fn the_feed_hands_the_view_sanitised_text_and_the_counts_beside_it() {
        // End to end through the wire: the obligation is met at the boundary
        // the view actually reads from, not only in the sanitiser's own tests.
        let log = log_with_body("p\u{0430}ypal\u{202E}x");
        let out = list_threads(&feed_request(""), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let body = &v["items"][0]["body"];
        assert_eq!(body["removed"], 1);
        assert_eq!(body["marked"], 1);
        assert!(
            !body["text"].as_str().unwrap().contains('\u{202E}'),
            "an override reached the view: {out}"
        );
    }

    #[test]
    fn the_feed_takes_no_ordering_parameter_and_ignores_one_offered() {
        // The KISS decision, pinned. An `order` field must not select anything,
        // because there is only one order and accepting a name for a second
        // would be telling the caller a falsehood. It is ignored rather than
        // refused: an unknown field is not a caller error.
        let log = log_with_body("hello");
        let plain = list_threads(&feed_request(""), &log, &feed_genesis());
        let with_order = list_threads(&feed_request(r#""order":"top""#), &log, &feed_genesis());
        assert_eq!(
            plain, with_order,
            "an ordering argument must not change the answer while there is one ordering"
        );
    }

    #[test]
    fn a_malformed_feed_request_is_the_error_shape_and_carries_no_items() {
        // §2.5: never a partial success. A reply carrying both an error and an
        // empty `items` list would render as an empty feed in any view that
        // checked `items` first.
        let log = log_with_body("hello");
        let stoa = feed_genesis().address().unwrap().to_hex();
        for bad in [
            "not json".to_string(),
            r#"{}"#.to_string(),
            r#"{"stoa":7}"#.to_string(),
            r#"{"stoa":"nothex"}"#.to_string(),
            r#"{"stoa":"00ff"}"#.to_string(),
            format!(r#"{{"stoa":"{stoa}","page":-1}}"#),
            format!(r#"{{"stoa":"{stoa}","page":1.5}}"#),
            format!(r#"{{"stoa":"{stoa}","perPage":"many"}}"#),
            format!(r#"{{"stoa":"{stoa}","includeHidden":"yes"}}"#),
        ] {
            let out = list_threads(&bad, &log, &feed_genesis());
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v.get("items").is_none(),
                "a failure must never also carry a result — §2.5, got {out}"
            );
        }
    }

    #[test]
    fn a_feed_asked_for_a_stoa_the_genesis_record_does_not_describe_is_refused() {
        // Pairing a moderator set with the wrong Stoa would apply one Stoa's
        // authority to another's posts. Refused rather than served with
        // moderation quietly not applying.
        let log = log_with_body("hello");
        let elsewhere = Genesis {
            creator: feed_key(1).public_key(),
            policy: Policy::Open,
            title: "Somewhere else".to_string(),
        }
        .address()
        .unwrap();
        let out = list_threads(
            &format!(r#"{{"stoa":"{}"}}"#, elsewhere.to_hex()),
            &log,
            &feed_genesis(),
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("items").is_none());
    }

    #[test]
    fn an_absent_page_and_per_page_default_rather_than_failing() {
        // A view rendering a first page should not have to spell both.
        let log = log_with_body("hello");
        let out = list_threads(&feed_request(""), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["page"], 0);
        assert_eq!(v["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn an_oversized_per_page_is_clamped_rather_than_refused() {
        // A caller asking for a million rows is not attacking anything, and
        // refusing the page outright would be a worse answer than a smaller one
        // — but the reply must not actually be built at that size.
        let log = log_with_body("hello");
        let out = list_threads(&feed_request(r#""perPage":1000000"#), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "got {out}");
        assert_eq!(v["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_store_failure_reaches_the_view_as_the_error_shape_and_not_as_an_empty_feed() {
        // §11.1 obligation 5 at the wire, which is the layer the view reads.
        // This is the whole of screen 07's correctness: an empty feed and a
        // broken store must not produce the same reply.
        struct BrokenLog;
        impl OpLog for BrokenLog {
            fn append(
                &mut self,
                _op: crate::op::SignedOp,
                _arrival: Arrival,
            ) -> Result<crate::log::Appended, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
            fn get(
                &self,
                _id: &crate::op::OpId,
            ) -> Result<Option<crate::log::Entry>, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
            fn iter(&self) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
            fn iter_stoa(
                &self,
                _stoa: &Address,
            ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
            fn iter_target(
                &self,
                _target: &crate::op::OpId,
            ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
            fn len(&self) -> Result<usize, crate::log::OpLogError> {
                Err(crate::log::OpLogError::Storage("database is locked".into()))
            }
        }

        let out = list_threads(&feed_request(""), &BrokenLog, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v.get("items").is_none(),
            "a broken store must not render as a quiet Stoa"
        );
        assert!(
            v["error"].as_str().unwrap().contains("database is locked"),
            "the reason must survive so the view can name it, got {out}"
        );

        // And the contrast that makes screen 07 possible: an EMPTY store with
        // the same request produces a success with an empty list. The two
        // replies must be distinguishable, which is the whole requirement.
        let empty = list_threads(&feed_request(""), &MemoryOpLog::new(), &feed_genesis());
        let ev: serde_json::Value = serde_json::from_str(&empty).unwrap();
        assert!(ev.get("error").is_none(), "got {empty}");
        assert_eq!(ev["items"].as_array().unwrap().len(), 0);
        assert_ne!(
            out, empty,
            "empty and unreadable must never be the same reply"
        );
    }

    #[test]
    fn the_feed_handler_is_never_a_panic() {
        // The guard, on a handler that runs over attacker-supplied content. A
        // panic here aborts the module process rather than failing one call.
        struct PanickingLog;
        impl OpLog for PanickingLog {
            fn append(
                &mut self,
                _op: crate::op::SignedOp,
                _arrival: Arrival,
            ) -> Result<crate::log::Appended, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
            fn get(
                &self,
                _id: &crate::op::OpId,
            ) -> Result<Option<crate::log::Entry>, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
            fn iter(&self) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
            fn iter_stoa(
                &self,
                _stoa: &Address,
            ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
            fn iter_target(
                &self,
                _target: &crate::op::OpId,
            ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
            fn len(&self) -> Result<usize, crate::log::OpLogError> {
                panic!("the storage layer exploded")
            }
        }

        let out = list_threads(&feed_request(""), &PanickingLog, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("items").is_none());
    }

    // ─── The genesis record travelling with the request ───────────────────

    fn genesis_hex() -> String {
        hex::encode(feed_genesis().canonical_bytes().unwrap())
    }

    fn full_request() -> String {
        format!(
            r#"{{"stoa":"{}","genesis":"{}"}}"#,
            feed_genesis().address().unwrap().to_hex(),
            genesis_hex()
        )
    }

    #[test]
    fn a_request_carrying_its_genesis_record_reads_the_feed() {
        let log = log_with_body("hello");
        let out =
            list_threads_from_request(&full_request(), || Ok::<_, crate::log::OpLogError>(log));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "got {out}");
        assert_eq!(v["items"].as_array().unwrap().len(), 1);
        assert_eq!(v["items"][0]["body"]["text"], "hello");
    }

    #[test]
    fn a_genesis_record_that_does_not_hash_to_the_stoa_is_refused() {
        // THE security property §4.8 rests on: an address IS the hash of the
        // genesis record, so a caller cannot supply a record naming themselves
        // as creator and have it accepted for someone else's Stoa. Without this
        // check, the moderator set is whatever the caller says it is.
        let attacker = Genesis {
            creator: feed_key(9).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        };
        // The attacker's record is perfectly well-formed — it just describes a
        // different Stoa.
        assert!(attacker.address().is_ok());

        let request = format!(
            r#"{{"stoa":"{}","genesis":"{}"}}"#,
            feed_genesis().address().unwrap().to_hex(),
            hex::encode(attacker.canonical_bytes().unwrap())
        );
        let out = list_threads_from_request(&request, || {
            Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("error").is_some(),
            "a record that does not hash to the address must be refused, got {out}"
        );
        assert!(v.get("items").is_none());
    }

    #[test]
    fn a_missing_or_malformed_genesis_record_is_refused_by_name() {
        let stoa = feed_genesis().address().unwrap().to_hex();
        for (bad, why) in [
            (format!(r#"{{"stoa":"{stoa}"}}"#), "missing"),
            (
                format!(r#"{{"stoa":"{stoa}","genesis":7}}"#),
                "must be a string",
            ),
            (format!(r#"{{"stoa":"{stoa}","genesis":"nothex!"}}"#), "hex"),
            (format!(r#"{{"stoa":"{stoa}","genesis":""}}"#), "genesis"),
        ] {
            let out = list_threads_from_request(&bad, || {
                Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
            });
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v["error"].as_str().unwrap().contains(why),
                "the message must say WHICH mistake ({why}), got {out}"
            );
            assert!(v.get("items").is_none());
        }
    }

    #[test]
    fn an_over_long_genesis_hex_string_is_refused_before_it_is_decoded() {
        // NO SPEC: the spec set bounds no field's length. This is the same
        // absent decision `MAX_REQUEST_BYTES` is, one layer in — and it is kept
        // beside the request cap rather than folded into it because they refuse
        // different things: the request cap bounds what any request may cost,
        // and this bounds what THIS field may allocate no matter how small the
        // request around it is.
        //
        // The assertion is about ORDERING, which is the only part that matters:
        // the fixture is over-long AND not valid hex. An implementation that
        // decoded first answers "genesis is not valid hex"; only one that checks
        // the length first can answer for the length. Swap the two lines in
        // `genesis_for` and this goes red while every other genesis test stays
        // green.
        let stoa = feed_genesis().address().unwrap().to_hex();
        let over_long = "z".repeat(crate::stoa::MAX_CANONICAL_BYTES * 2 + 1);
        let request = format!(r#"{{"stoa":"{stoa}","genesis":"{over_long}"}}"#);
        assert!(
            request.len() < MAX_REQUEST_BYTES,
            "the request must be well under the envelope cap, or THAT is what refuses it"
        );

        let out = list_threads_from_request(&request, || {
            Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
        });
        let message = error_message(&out);
        assert!(
            message.contains("over the") && message.contains("hex characters"),
            "an over-long genesis must be refused for its length, got {message:?}"
        );
        assert!(
            !message.contains("not valid hex"),
            "the length must be checked BEFORE the decode, got {message:?}"
        );

        // The boundary from the other side: a hex string of exactly the largest
        // record the format allows must still reach the decode, so a `>` written
        // as `>=` is caught. Junk of that length is "not valid hex", which is the
        // refusal it has always earned.
        let exactly_at_bound = "z".repeat(crate::stoa::MAX_CANONICAL_BYTES * 2);
        let at_bound_request = format!(r#"{{"stoa":"{stoa}","genesis":"{exactly_at_bound}"}}"#);
        let at_bound = error_message(&list_threads_from_request(&at_bound_request, || {
            Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
        }));
        assert!(
            at_bound.contains("not valid hex"),
            "a hex string at the format's own bound must still be decoded, got {at_bound:?}"
        );
    }

    #[test]
    fn a_store_that_cannot_be_opened_is_the_error_shape_and_not_an_empty_feed() {
        // The failure one step earlier than the read: opening the store. It is
        // just as easy to flatten into an empty page here, and it renders
        // identically if it is.
        let out = list_threads_from_request(&full_request(), || {
            Err::<MemoryOpLog, _>(crate::log::OpLogError::Storage(
                "unable to open database file".into(),
            ))
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("items").is_none());
        assert!(
            v["error"]
                .as_str()
                .unwrap()
                .contains("unable to open database file"),
            "the reason must reach the view so it can be named, got {out}"
        );
    }

    // ─── The publish path ─────────────────────────────────────────────────

    /// The root secret a keystore would hold, fixed so derived addresses are
    /// reproducible.
    const PUBLISH_ROOT: [u8; 32] = [7u8; 32];

    fn publish_stoa() -> Address {
        feed_genesis().address().unwrap()
    }

    /// The per-Stoa signing key, derived exactly as the keystore derives it.
    fn publish_key() -> crate::identity::SecretKey {
        crate::identity::derive_stoa_key(&PUBLISH_ROOT, &publish_stoa())
    }

    /// A publish request naming this Stoa plus whatever else is given.
    fn publish_request(extra: &str) -> String {
        let stoa = publish_stoa().to_hex();
        if extra.is_empty() {
            format!(r#"{{"stoa":"{stoa}"}}"#)
        } else {
            format!(r#"{{"stoa":"{stoa}",{extra}}}"#)
        }
    }

    /// A delivery sink that records nothing — for tests not about delivery.
    ///
    /// A plain `fn` item, so `&mut ignored_delivery` at a call site is a fresh
    /// temporary whose borrow ends with the statement. A shared
    /// `let mut sink = |_| {}` would borrow for the rest of the test and collide
    /// with the handler's `&mut` log.
    fn ignored_delivery(_id: &crate::op::OpId) {}

    fn as_json(out: &str) -> serde_json::Value {
        serde_json::from_str(out)
            .unwrap_or_else(|e| panic!("a handler must emit valid JSON ({e}): {out}"))
    }

    #[test]
    fn a_published_post_reply_names_the_op_and_whether_it_was_new() {
        // Pinned by key name and by value. A view is written against these exact
        // names, and `wasNew` is the only thing that tells a deduplicated
        // publish from a first one.
        let mut log = MemoryOpLog::new();
        let key = publish_key();

        let out = publish_post(
            &publish_request(r#""body":"First""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        );
        let v = as_json(&out);
        assert!(v.get("error").is_none(), "got {out}");
        assert_eq!(v["wasNew"], true);

        // The op id names the op now in the log, read back through the log
        // rather than compared against itself.
        let id = crate::op::OpId::from_hex(v["opId"].as_str().unwrap()).unwrap();
        let entry = log.get(&id).unwrap().expect("the op must be in the log");
        assert_eq!(entry.id(), id);
        match &entry.op.op.kind {
            OpKind::Post { body, parent, .. } => {
                assert_eq!(body, "First");
                assert_eq!(*parent, None, "a post names no parent");
            }
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn a_second_publish_of_one_body_says_it_was_not_new_and_names_the_same_op() {
        // The contracted duplication behaviour, at the wire, where a caller has
        // no other way to tell the two apart.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let request = publish_request(r#""body":"twice""#);

        let first = as_json(&publish_post(
            &request,
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let second = as_json(&publish_post(
            &request,
            &mut log,
            &key,
            &mut ignored_delivery,
        ));

        assert_eq!(first["opId"], second["opId"]);
        assert_eq!(first["wasNew"], true, "the first publish stored it");
        assert_eq!(
            second["wasNew"], false,
            "the second must report already-present rather than failing"
        );
        assert!(
            second.get("error").is_none(),
            "a repeated publish is not a refusal: it is the retried-submission case"
        );
        assert_eq!(log.len().unwrap(), 1, "one op");
    }

    #[test]
    fn a_published_reply_is_derived_into_its_parents_thread_through_the_wire() {
        // End to end, three levels deep, because at two levels "the parent's id"
        // and "the parent's thread" are the same value and a copy-the-parent
        // implementation would agree.
        let mut log = MemoryOpLog::new();
        let key = publish_key();

        let root = as_json(&publish_post(
            &publish_request(r#""body":"the head""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let root_id = root["opId"].as_str().unwrap().to_string();

        let middle = as_json(&publish_reply(
            &publish_request(&format!(r#""parent":"{root_id}","body":"a reply""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let middle_id = middle["opId"].as_str().unwrap().to_string();
        assert_ne!(root_id, middle_id, "the fixture needs distinct ops");

        let leaf = as_json(&publish_reply(
            &publish_request(&format!(r#""parent":"{middle_id}","body":"and again""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        assert!(leaf.get("error").is_none(), "got {leaf}");

        let id = crate::op::OpId::from_hex(leaf["opId"].as_str().unwrap()).unwrap();
        let entry = log.get(&id).unwrap().unwrap();
        match &entry.op.op.kind {
            OpKind::Post { thread, parent, .. } => {
                assert_eq!(
                    parent.map(|p| p.to_hex()),
                    Some(middle_id),
                    "the parent is the one named"
                );
                assert_eq!(
                    thread.map(|t| t.to_hex()),
                    Some(root_id),
                    "the thread is the ROOT's, derived rather than copied from the parent"
                );
            }
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn a_request_supplying_a_thread_is_refused_and_not_ignored() {
        // A caller passing `thread` believes it is filing the reply somewhere.
        // Ignoring the field would leave that belief unhonoured and unreported —
        // which is the opposite of the `order` field's treatment, deliberately.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let root = as_json(&publish_post(
            &publish_request(r#""body":"the head""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let root_id = root["opId"].as_str().unwrap();
        let before = log.len().unwrap();

        let out = publish_reply(
            &publish_request(&format!(
                r#""parent":"{root_id}","thread":"{root_id}","body":"filed by hand""#
            )),
            &mut log,
            &key,
            &mut ignored_delivery,
        );
        let v = as_json(&out);
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v["error"].as_str().unwrap().contains("thread"),
            "the message must name the field, got {out}"
        );
        assert!(v.get("opId").is_none(), "a refusal carries no op id — §2.5");
        assert_eq!(
            log.len().unwrap(),
            before,
            "a refused publish appends nothing"
        );
    }

    #[test]
    fn a_forbidden_field_is_refused_on_every_operation() {
        // NO SPEC: the spec's requirement names `author`, `identity`, `key` AND
        // `address` as never-a-parameter on any publish, so four of the five are
        // specified. (Its scenario one screen down names only the first three;
        // the requirement is the contract.) `thread` is the unspecified one — the
        // spec requires it refused on a REPLY only, and refusing it on a post and
        // a vote too is chosen here, because a caller who sent one has the same
        // wrong model whichever operation it reached. That choice is what this
        // test pins.
        //
        // The trap avoided: a guard called from one handler and forgotten in the
        // other two. That is invisible without checking all three.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let victim = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let target = victim["opId"].as_str().unwrap().to_string();
        let before = log.len().unwrap();

        for field in ["author", "identity", "key", "address", "thread"] {
            let forged = format!(r#""{field}":"00ff""#);
            let requests = [
                publish_request(&format!(r#""body":"x",{forged}"#)),
                publish_request(&format!(r#""parent":"{target}","body":"x",{forged}"#)),
                publish_request(&format!(r#""target":"{target}","direction":"up",{forged}"#)),
            ];
            let outs = [
                publish_post(&requests[0], &mut log, &key, &mut ignored_delivery),
                publish_reply(&requests[1], &mut log, &key, &mut ignored_delivery),
                publish_vote(&requests[2], &mut log, &key, &mut ignored_delivery),
            ];
            for (out, request) in outs.iter().zip(requests.iter()) {
                let v = as_json(out);
                assert!(
                    v.get("error").is_some(),
                    "{field} must be refused, not ignored, for {request}: got {out}"
                );
                assert!(v.get("opId").is_none(), "got {out}");
            }
        }
        assert_eq!(
            log.len().unwrap(),
            before,
            "no refused publish appended anything"
        );
    }

    #[test]
    fn a_publish_requests_missing_field_is_named_and_is_not_defaulted() {
        // Each of the six required fields, absent. A handler that defaulted a
        // body to empty or a direction to `up` would publish something the
        // caller never asked for.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let target = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();
        let stoa = publish_stoa().to_hex();
        let before = log.len().unwrap();

        let cases: [(&str, String, &str); 6] = [
            ("post", r#"{"body":"x"}"#.to_string(), "stoa"),
            ("post", format!(r#"{{"stoa":"{stoa}"}}"#), "body"),
            (
                "reply",
                format!(r#"{{"stoa":"{stoa}","body":"x"}}"#),
                "parent",
            ),
            (
                "reply",
                format!(r#"{{"stoa":"{stoa}","parent":"{target}"}}"#),
                "body",
            ),
            (
                "vote",
                format!(r#"{{"stoa":"{stoa}","direction":"up"}}"#),
                "target",
            ),
            (
                "vote",
                format!(r#"{{"stoa":"{stoa}","target":"{target}"}}"#),
                "direction",
            ),
        ];
        for (op, request, field) in cases {
            let out = match op {
                "post" => publish_post(&request, &mut log, &key, &mut ignored_delivery),
                "reply" => publish_reply(&request, &mut log, &key, &mut ignored_delivery),
                _ => publish_vote(&request, &mut log, &key, &mut ignored_delivery),
            };
            let v = as_json(&out);
            assert!(v.get("error").is_some(), "for {request}, got {out}");
            let message = v["error"].as_str().unwrap();
            assert!(
                message.contains("missing") && message.contains(field),
                "the message must say WHICH field is missing ({field}), got {out}"
            );
            assert!(v.get("opId").is_none());
        }
        assert_eq!(log.len().unwrap(), before);
    }

    #[test]
    fn a_wrong_typed_direction_is_refused_by_its_type_with_a_valid_stoa_and_target() {
        // THE GAP `findings/security.md` S2 NAMES, closed with the fixture it says
        // is missing: a VALID Stoa and a VALID target, so the request survives
        // parser one and two and the direction parser is actually entered.
        //
        // Why the existing coverage did not reach here. `stoa` is parsed first in
        // all three handlers, and every wrong-typed `direction` fixture in the
        // repo also malformed `stoa` — so all three handlers refused at parser one.
        // Measured: `panic!` on `required_string`'s wrong-typed arm for
        // `direction`, reached only from `required_direction`, left all 566 tests
        // green.
        //
        // What this asserts, and why not merely `error.is_some()`: an array or an
        // object in `direction` is an error EITHER WAY — `required_direction` would
        // refuse it, and so would a handler that panicked, and so would one that
        // read the field as absent. Three explanations, one answer, which is this
        // repo's recurring defect family. So the assertions are:
        //
        //   1. the message is the WRONG-TYPE message, against the hardcoded
        //      literal `required_string` writes — NOT "differs from the missing
        //      one", which a panic marker would also satisfy;
        //   2. it is not a panic marker;
        //   3. it does not say "missing", because a present-but-wrong-typed field
        //      reported as missing sends a caller looking for a field that is
        //      right there.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let stoa = publish_stoa().to_hex();
        let target = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();
        let before = log.len().unwrap();

        // Every JSON type that is not a string, `null` INCLUDED.
        //
        // I first excluded null here on the assumption that the envelope collapses
        // it to absent, which would make it the MISSING mistake. That was wrong,
        // and `wire/request.rs:229` says so in as many words: *"An explicit `null`
        // is `Some(Value::Null)` and never `None` … a field holding an explicit
        // `null` is present, not absent"*, and the envelope deliberately decides
        // nothing further, leaving the reading to the reader. `required_string` is
        // that reader here and refuses a null as a wrong TYPE. Confirmed by the
        // sweep failing on the null fixture under the mutation below, which is what
        // sent me to read the envelope rather than assume.
        //
        // So null belongs in this list, and pinning it here is what keeps the
        // envelope's "a null is present" rule from being quietly reversed by a
        // reader that starts treating it as absent — which, for an optional field,
        // is the defaulting reading `parse_index`'s doc calls out as the half that
        // can become an authorisation bypass.
        for wrong in [
            serde_json::Value::Null,
            serde_json::json!(true),
            serde_json::json!(false),
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!({}),
            serde_json::json!([]),
            serde_json::json!(["up"]),
            serde_json::json!({"direction": "up"}),
        ] {
            let request = serde_json::json!({
                "stoa": stoa, "target": target, "direction": wrong
            })
            .to_string();
            let out = publish_vote(&request, &mut log, &key, &mut ignored_delivery);
            let v = as_json(&out);
            let message = v["error"]
                .as_str()
                .unwrap_or_else(|| panic!("expected the error shape, got {out}"));

            assert!(
                !message.starts_with("panic in "),
                "a wrong-typed direction must be refused, not panicked on, for {request}: {out}"
            );
            // The hardcoded literal, not a value read back out of the code.
            assert_eq!(
                message, "direction must be a string",
                "the refusal must name the TYPE mistake, for {request}"
            );
            assert!(
                !message.contains("missing"),
                "a field that is present must not be reported as missing, for {request}"
            );
            assert!(v.get("opId").is_none(), "got {out}");
        }
        assert_eq!(
            log.len().unwrap(),
            before,
            "no refused vote appended anything"
        );
    }

    #[test]
    fn a_wrong_typed_field_is_distinguishable_from_a_missing_one() {
        // Both are errors, and they are different mistakes: "missing field:
        // body" sends someone looking for a field that is right there holding a
        // number. Asserted as the two messages DIFFERING and each naming its own
        // mistake, not merely as two errors.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let stoa = publish_stoa().to_hex();

        let missing = as_json(&publish_post(
            &format!(r#"{{"stoa":"{stoa}"}}"#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let wrong_typed = as_json(&publish_post(
            &format!(r#"{{"stoa":"{stoa}","body":7}}"#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));

        let missing_msg = missing["error"].as_str().unwrap();
        let wrong_msg = wrong_typed["error"].as_str().unwrap();
        assert_ne!(missing_msg, wrong_msg);
        assert!(missing_msg.contains("missing"), "got {missing_msg}");
        assert!(
            wrong_msg.contains("must be a string"),
            "a wrong type must not report as missing, got {wrong_msg}"
        );
        assert!(!wrong_msg.contains("missing"), "got {wrong_msg}");

        // The same distinction on the Stoa field.
        let stoa_missing = as_json(&publish_post(
            r#"{"body":"x"}"#,
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let stoa_wrong = as_json(&publish_post(
            r#"{"stoa":7,"body":"x"}"#,
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        assert!(stoa_missing["error"].as_str().unwrap().contains("missing"));
        assert!(stoa_wrong["error"]
            .as_str()
            .unwrap()
            .contains("must be a string"));
        assert_eq!(log.len().unwrap(), 0, "nothing was published");
    }

    #[test]
    fn an_empty_body_publishes_through_the_wire() {
        // `op-format` contracts an empty variable-length field as a value, so
        // this must be a success and not "missing field: body".
        let mut log = MemoryOpLog::new();
        let out = publish_post(
            &publish_request(r#""body":"""#),
            &mut log,
            &publish_key(),
            &mut ignored_delivery,
        );
        let v = as_json(&out);
        assert!(v.get("error").is_none(), "got {out}");
        let id = crate::op::OpId::from_hex(v["opId"].as_str().unwrap()).unwrap();
        match &log.get(&id).unwrap().unwrap().op.op.kind {
            OpKind::Post { body, .. } => assert_eq!(body, ""),
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn both_vote_directions_publish_and_an_unrecognised_one_is_refused_naming_it() {
        // The sharp property: an unrecognised direction must NOT be mapped onto
        // a recognised one. A caller whose "upvote" silently became "down" would
        // have published the opposite of what it asked for, with no error.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let target = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();

        let mut ids = Vec::new();
        for (name, expected) in [
            ("up", crate::op::VoteDirection::Up),
            ("down", crate::op::VoteDirection::Down),
        ] {
            let out = publish_vote(
                &publish_request(&format!(r#""target":"{target}","direction":"{name}""#)),
                &mut log,
                &key,
                &mut ignored_delivery,
            );
            let v = as_json(&out);
            assert!(v.get("error").is_none(), "for {name}, got {out}");
            let id = crate::op::OpId::from_hex(v["opId"].as_str().unwrap()).unwrap();
            match log.get(&id).unwrap().unwrap().op.op.kind {
                crate::op::OpKind::Vote { direction, .. } => assert_eq!(direction, expected),
                ref other => panic!("expected a vote, got {other:?}"),
            }
            ids.push(id);
        }
        assert_ne!(ids[0], ids[1], "the two directions are two ops");

        // Every plausible near-miss, including the casing and pluralisation a
        // caller would actually get wrong.
        let before = log.len().unwrap();
        for bad in ["UP", "Up", "upvote", "raise", "+1", "", "u p", "1", "down "] {
            let out = publish_vote(
                &publish_request(&format!(r#""target":"{target}","direction":"{bad}""#)),
                &mut log,
                &key,
                &mut ignored_delivery,
            );
            let v = as_json(&out);
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(
                v["error"].as_str().unwrap().contains(bad),
                "the refusal must name the direction supplied, got {out}"
            );
            assert!(v.get("opId").is_none());
        }
        assert_eq!(
            log.len().unwrap(),
            before,
            "no vote was published in either direction by a refused request"
        );
    }

    #[test]
    fn a_vote_reply_carries_no_score_count_tally_rank_or_position() {
        // Nothing in the current contract reads a `Vote` op, so a field
        // describing an effect would be a falsehood a caller would act on.
        // Checked as an exhaustive key list rather than a spot-check of one
        // name, so a new field cannot slip in unnoticed.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let target = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();

        let out = publish_vote(
            &publish_request(&format!(r#""target":"{target}","direction":"up""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        );
        let v = as_json(&out);
        assert!(v.get("error").is_none(), "got {out}");
        assert!(v["opId"].is_string(), "the reply carries the op id");

        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            vec!["opId", "wasNew"],
            "a vote reply must carry the op id and nothing describing an effect, got {out}"
        );
        for forbidden in [
            "score", "count", "tally", "rank", "position", "votes", "total", "weight",
        ] {
            assert!(
                v.get(forbidden).is_none(),
                "a vote reply must not carry {forbidden}, got {out}"
            );
        }
    }

    #[test]
    fn a_vote_leaves_the_feed_row_of_its_target_identical() {
        // The other half of "nothing reads a vote", at the layer a view actually
        // reads from. A score computed anywhere would show up here.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let posted = as_json(&publish_post(
            &publish_request(r#""body":"unaffected""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let target = posted["opId"].as_str().unwrap().to_string();

        let before = list_threads(&feed_request(""), &log, &feed_genesis());
        for direction in ["up", "down"] {
            publish_vote(
                &publish_request(&format!(r#""target":"{target}","direction":"{direction}""#)),
                &mut log,
                &key,
                &mut ignored_delivery,
            );
        }
        let after = list_threads(&feed_request(""), &log, &feed_genesis());
        assert_eq!(
            before, after,
            "voting must change nothing a reader is told about the post"
        );
        // And the fixture really had a row, so this is not two empty feeds
        // agreeing.
        assert_eq!(as_json(&before)["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal() {
        // Two requirements: the sink receives the op that was published, and a
        // refusal never reaches it at all.
        //
        // What this test does not see is the ORDERING — that the append
        // happened before the sink was called. This test observes only that the
        // sink got the right id, and that a refusal never reaches it.
        //
        // The ordering IS observable, and
        // `the_append_completes_before_delivery_is_invoked_on_all_three_handlers`
        // below observes it: a journal shared by a wrapping `OpLog` and the sink
        // records a hardcoded `["append", "deliver"]`. An earlier version of
        // this comment claimed no test through this API could see it, on the
        // grounds that the sink cannot read the log the handler holds mutably.
        // That is true of the log and false of the ordering — two clones of one
        // `Rc<RefCell<Vec<_>>>` borrow nothing from each other. The claim was
        // load-bearing while it stood, because it was the stated reason this
        // weaker test was accepted as sufficient.
        let mut log = MemoryOpLog::new();
        let key = publish_key();

        let mut delivered: Vec<crate::op::OpId> = Vec::new();
        let out = publish_post(
            &publish_request(r#""body":"ordered""#),
            &mut log,
            &key,
            &mut |id: &crate::op::OpId| delivered.push(*id),
        );
        let v = as_json(&out);
        let id = crate::op::OpId::from_hex(v["opId"].as_str().unwrap()).unwrap();
        assert_eq!(
            delivered.as_slice(),
            &[id],
            "delivery must be handed the op that was published, and only it"
        );
        assert!(
            log.get(&id).unwrap().is_some(),
            "and the op is in the log by the time the call returns"
        );

        // A refusal must not reach the sink. An absent parent is the cheapest
        // refusal to construct, and the sink PANICS if reached — so a handler
        // that delivered on the refusal path fails loudly rather than by a count
        // nobody reads.
        let absent = crate::op::OpId::from_hex(&"cc".repeat(32))
            .unwrap()
            .to_hex();
        let out = publish_reply(
            &publish_request(&format!(r#""parent":"{absent}","body":"x""#)),
            &mut log,
            &key,
            &mut |_: &crate::op::OpId| panic!("delivery was invoked for a refused publish"),
        );
        let v = as_json(&out);
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            !v["error"]
                .as_str()
                .unwrap()
                .contains("delivery was invoked"),
            "the sink must not have been reached, got {out}"
        );
        assert!(
            v["error"].as_str().unwrap().contains("does not hold"),
            "the refusal must be the one the fixture built, got {out}"
        );
    }

    /// An `OpLog` that records the order in which it was called, in a journal it
    /// SHARES with a delivery sink.
    ///
    /// # Why this exists, when the change's own notes say the ordering is
    /// unobservable
    ///
    /// `tasks.md` §10 records "that the append precedes delivery" as something
    /// no test through this API can see, on the grounds that the sink cannot read
    /// the log because the handler holds it mutably for the call's duration. That
    /// is true of the *log*, and it is not true of the *ordering*: an
    /// `Rc<RefCell<Vec<_>>>` held by BOTH the log wrapper and the sink is two
    /// clones of one handle, so neither has to borrow the other. The log appends
    /// its own name when `append` runs; the sink appends its own when it runs;
    /// the resulting sequence is evidence, not an argument.
    ///
    /// So "sign, append, hand off — **in that order**" becomes a test that can
    /// fail, which it could not while the only thing pinning it was the order two
    /// statements happen to be in.
    struct JournallingLog {
        inner: MemoryOpLog,
        journal: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
    }

    impl crate::log::OpLog for JournallingLog {
        fn append(
            &mut self,
            op: crate::op::SignedOp,
            arrival: crate::arrival::Arrival,
        ) -> Result<crate::log::Appended, crate::log::OpLogError> {
            let out = self.inner.append(op, arrival);
            // Recorded AFTER the inner append returns, so the journal entry
            // means "the op is stored", not "an append was attempted".
            self.journal.borrow_mut().push("append");
            out
        }
        fn get(
            &self,
            id: &crate::op::OpId,
        ) -> Result<Option<crate::log::Entry>, crate::log::OpLogError> {
            self.inner.get(id)
        }
        fn iter(&self) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
            self.inner.iter()
        }
        fn iter_stoa(
            &self,
            stoa: &Address,
        ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
            self.inner.iter_stoa(stoa)
        }
        fn iter_target(
            &self,
            target: &crate::op::OpId,
        ) -> Result<Vec<crate::log::Entry>, crate::log::OpLogError> {
            self.inner.iter_target(target)
        }
        fn len(&self) -> Result<usize, crate::log::OpLogError> {
            self.inner.len()
        }
    }

    #[test]
    fn the_append_completes_before_delivery_is_invoked_on_all_three_handlers() {
        // The spec's ordering requirement, observed rather than argued: "Each
        // publish operation SHALL sign an op, append it to the local op log, and
        // hand it to delivery. The append SHALL complete before delivery is
        // invoked."
        //
        // The expected sequence is HARDCODED below and is not read back from
        // anything the handlers produced. A handler that called the sink first
        // yields `["deliver", "append"]` and this fails on the comparison.
        //
        // All three handlers, because the ordering is a property of each call
        // site and one of them getting it right says nothing about the other two
        // — which is the same "is the guard called everywhere?" shape as
        // `a_forbidden_field_is_refused_on_every_operation`.
        let key = publish_key();
        let stoa = publish_stoa().to_hex();

        for which in ["post", "reply", "vote"] {
            let journal = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut log = JournallingLog {
                inner: MemoryOpLog::new(),
                journal: std::rc::Rc::clone(&journal),
            };

            // A seed post for the reply and the vote to point at. It goes
            // through the same log, so the journal is CLEARED afterwards and
            // only the measured call's sequence is asserted on.
            let seed = as_json(&publish_post(
                &publish_request(r#""body":"the subject""#),
                &mut log,
                &key,
                &mut ignored_delivery,
            ))["opId"]
                .as_str()
                .unwrap()
                .to_string();
            journal.borrow_mut().clear();

            let request =
                match which {
                    "post" => serde_json::json!({"stoa": stoa, "body": "ordered"}).to_string(),
                    "reply" => serde_json::json!({"stoa": stoa, "parent": seed, "body": "ordered"})
                        .to_string(),
                    _ => serde_json::json!({"stoa": stoa, "target": seed, "direction": "up"})
                        .to_string(),
                };
            let sink_journal = std::rc::Rc::clone(&journal);
            let out = match which {
                "post" => publish_post(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
                "reply" => publish_reply(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
                _ => publish_vote(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
            };
            assert!(as_json(&out).get("error").is_none(), "for {which}: {out}");
            assert_eq!(
                journal.borrow().as_slice(),
                ["append", "deliver"],
                "{which} must append before it hands off, and hand off exactly once"
            );
        }
    }

    #[test]
    fn a_refused_publish_reaches_neither_the_append_nor_delivery() {
        // The other half of the same requirement: "A publish that is refused
        // appends nothing — AND delivery was not invoked." Asserted as an EMPTY
        // journal, so it distinguishes "nothing happened" from "an append was
        // rolled back", which counting the log afterwards cannot.
        //
        // The four refusals sit at three different depths: two fail in the parse
        // (before `authoring` is reached at all), one in `authoring`'s presence
        // check, and one in its cross-Stoa check — so a handler that reached the
        // store or the sink on any of those paths is caught rather than one of
        // them standing in for all three.
        //
        // Each case also asserts the refusal MESSAGE, not just that a refusal
        // happened. That is what keeps the depths honest: the cross-Stoa fixture
        // previously named an absent target against an empty log, so it refused at
        // the presence check and was a second copy of the case above it.
        let key = publish_key();
        let stoa = publish_stoa().to_hex();
        let elsewhere = crate::identity::Address::from_hex(&"4d".repeat(32))
            .unwrap()
            .to_hex();
        let absent = crate::op::OpId::from_hex(&"9b".repeat(32))
            .unwrap()
            .to_hex();

        let cases: [(&str, String, &str); 4] = [
            // Refused in the parse: a forbidden field.
            (
                "post",
                serde_json::json!({"stoa": stoa, "body": "x", "author": "00ff"}).to_string(),
                "a forbidden field",
            ),
            // Refused in the parse: a direction the wire does not recognise.
            (
                "vote",
                serde_json::json!({"stoa": stoa, "target": absent, "direction": "upvote"})
                    .to_string(),
                "an unrecognised direction",
            ),
            // Refused in `authoring`: the parent is not held.
            (
                "reply",
                serde_json::json!({"stoa": stoa, "parent": absent, "body": "x"}).to_string(),
                "an absent parent",
            ),
            // Refused in `authoring`'s CROSS-STOA check, which is a different
            // depth from the one above and needs a target that IS held. The
            // fixture seeds a post into `stoa` and then votes on it naming
            // `elsewhere`.
            //
            // This case used to name an ABSENT target in another Stoa, against an
            // empty log — so `log.get` returned `None` and it refused at `NotHeld`,
            // the same path and the same depth as the case above it. The comment
            // claimed three depths and the fixtures delivered two; a handler that
            // reached the sink on the cross-Stoa path only would have left this
            // test green (found by review, findings/correctness.md C2).
            (
                "vote-cross-stoa",
                serde_json::json!({"stoa": elsewhere, "direction": "up"}).to_string(),
                "a held target in another Stoa",
            ),
        ];

        for (which, request, why) in cases {
            let journal = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut log = JournallingLog {
                inner: MemoryOpLog::new(),
                journal: std::rc::Rc::clone(&journal),
            };

            // The cross-Stoa case is the only one needing a seeded target, and it
            // must be seeded BEFORE the journal is cleared, so the seed's own
            // append does not count as the refusal's.
            let request = if which == "vote-cross-stoa" {
                let seed = publish_post(
                    &publish_request(r#""body":"the target""#),
                    &mut log,
                    &key,
                    &mut ignored_delivery,
                );
                let target = as_json(&seed)["opId"].as_str().unwrap().to_string();
                journal.borrow_mut().clear();
                serde_json::json!({"stoa": elsewhere, "target": target, "direction": "up"})
                    .to_string()
            } else {
                request
            };
            let sink_journal = std::rc::Rc::clone(&journal);
            let out = match which {
                "post" => publish_post(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
                "reply" => publish_reply(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
                _ => publish_vote(&request, &mut log, &key, &mut |_| {
                    sink_journal.borrow_mut().push("deliver")
                }),
            };
            let error = as_json(&out)["error"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            assert!(!error.is_empty(), "{why} must be refused, got {out}");

            // Each case must be refused for the reason it was chosen to exercise,
            // not merely refused. Without this the cross-Stoa fixture silently
            // degraded into a second copy of the absent-target one, which is
            // exactly what had happened.
            let expected_fragment = match why {
                "a forbidden field" => "author",
                "an unrecognised direction" => "direction",
                "an absent parent" => "does not hold",
                "a held target in another Stoa" => "belongs to Stoa",
                other => unreachable!("unnamed case: {other}"),
            };
            assert!(
                error.contains(expected_fragment),
                "{why}: expected a refusal mentioning {expected_fragment:?}, got {out}"
            );

            assert!(
                journal.borrow().is_empty(),
                "{why}: a refused publish must reach neither the store nor delivery, \
                 and it reached {:?}",
                journal.borrow()
            );
        }
    }

    #[test]
    fn a_publish_whose_delivery_panics_leaves_the_op_in_the_log() {
        // A panicking sink is the most violent form of "delivery errors on the
        // handoff". What this pins is that the op STAYS: the append completed
        // before delivery was reached and nothing rolls it back.
        //
        // The reply is pinned by
        // `a_panicking_delivery_sink_still_reports_the_op_as_published_on_all_three_handlers`,
        // which is the other half of this and covers all three handlers. This one
        // is kept because the two assert different things: that one reads the
        // reply, this one reads the LOG, and an implementation that reported
        // success while rolling the op back would satisfy the reply assertion
        // alone.
        //
        // This was an open question when the reviewers found it — `guarded` wrapped
        // the `deliver` call, so a panicking sink turned a successful publish into
        // `{"error":"panic in publish_post: …"}` with no `opId` while the op was in
        // the log, against "a publish SHALL NOT be reported as having failed on the
        // strength of a delivery outcome". The owner settled it: catch the panic at
        // the handoff and report the publish as successful. See
        // `delivered_and_published` for why the outer guard stays, and `docs/PLAN.md`
        // §9.2 for the obligation this hands to `op-transport`.
        //
        // This test was named `…_still_reports_the_op_as_published` and asserted
        // no such thing — the name claimed the requirement while the body checked
        // only the log. Renamed to what it actually pins.
        let mut log = MemoryOpLog::new();
        let key = publish_key();

        // The id the publish will produce, computed independently so the
        // assertion does not depend on a reply the panic prevented.
        let expected = crate::op::Op {
            stoa: publish_stoa(),
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "survives a broken delivery".to_string(),
                attachments: vec![],
            },
        }
        .id();

        let out = publish_post(
            &publish_request(r#""body":"survives a broken delivery""#),
            &mut log,
            &key,
            &mut |_: &crate::op::OpId| panic!("delivery refused the handoff"),
        );
        // The handoff guard caught it, so this is a reply rather than an aborted
        // process. Parsed for well-formedness only; what the reply SAYS is the
        // sibling test's assertion, and what this test is named for is the log.
        as_json(&out);
        assert!(
            log.get(&expected).unwrap().is_some(),
            "a declined handoff must leave the op in the log, got {out}"
        );
        // The op the publish created, and nothing the panic added or rolled back.
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn a_panicking_delivery_sink_still_reports_the_op_as_published_on_all_three_handlers() {
        // "A handoff that fails outright leaves the op published": WHEN delivery
        // fails at the handoff in the most abrupt way the interface permits, THEN
        // the reply reports the op as published and names its op id, the op is
        // readable from the log, and the reply carries no error. A panicking sink
        // IS the most abrupt way the interface permits — the sink returns `()`, so
        // there is no error value it could return instead — which is what makes
        // this test the scenario's own fixture rather than an approximation of it.
        //
        // The quoted name was "A declined handoff leaves the op published", which
        // the spec no longer contains; the wording above is the live scenario's.
        // The test itself needed no change, only the citation.
        //
        // This is the assertion the sibling test
        // `a_publish_whose_delivery_panics_leaves_the_op_in_the_log` deliberately
        // did NOT make while the question was open. It fails before the fix: with
        // the sink call inside `guarded`, a panic discards the already-computed
        // reply and yields `{"error":"panic in publish_post: …"}` with no `opId`
        // for an op that IS in the log.
        //
        // Asserted on all three handlers because the sink is called from three
        // places, so one fixed and two missed is the failure a single-handler
        // test could not see.
        let key = publish_key();

        // A parent to reply to and a target to vote on, published with a sink
        // that does not panic, so the fixtures are not themselves under test.
        let mut log = MemoryOpLog::new();
        let seed = as_json(&publish_post(
            &publish_request(r#""body":"the parent""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let parent = seed["opId"].as_str().unwrap().to_string();

        type Handler = fn(
            &str,
            &mut MemoryOpLog,
            &crate::identity::SecretKey,
            &mut dyn FnMut(&crate::op::OpId),
        ) -> String;

        let cases: [(String, Handler); 3] = [
            (
                publish_request(r#""body":"a post""#),
                publish_post as Handler,
            ),
            (
                publish_request(&format!(r#""parent":"{parent}","body":"a reply""#)),
                publish_reply as Handler,
            ),
            (
                publish_request(&format!(r#""target":"{parent}","direction":"up""#)),
                publish_vote as Handler,
            ),
        ];

        for (request, handler) in &cases {
            let out = handler(request, &mut log, &key, &mut |_: &crate::op::OpId| {
                panic!("delivery refused the handoff")
            });
            let v = as_json(&out);
            assert!(
                v.get("error").is_none(),
                "a panicking sink must not be reported as a failed publish, got {out}"
            );
            let id = v["opId"]
                .as_str()
                .unwrap_or_else(|| panic!("the reply must name its op id, got {out}"));
            let id = crate::op::OpId::from_hex(id).expect("the reply's op id must parse");
            assert!(
                log.get(&id).unwrap().is_some(),
                "the op the reply names must be readable from the log, got {out}"
            );
        }
    }

    #[test]
    fn a_reply_names_the_op_and_whether_it_was_new_and_no_delivery_outcome() {
        // REPOINTED, and the old test is worth saying what was wrong with it.
        //
        // It was `a_delivery_that_reports_nothing_and_one_that_reports_promptly_give_one_reply`,
        // and its comment quoted a scenario — "A publish returns while delivery is
        // still outstanding" — that the spec no longer contains (the `spec-writer`
        // flagged it; `grep` over `specs/content-authoring/spec.md` finds no such
        // text). So it was a test with no requirement behind it.
        //
        // It was also close to vacuous on its own terms. Both sinks were
        // synchronous and the interface hands delivery a `&mut dyn FnMut` returning
        // `()`, so "the reply does not depend on what delivery did" was true by the
        // signature: there is no value a sink could return for a reply to depend
        // on. Asserting two replies equal proved the sink cannot speak, which the
        // type already guarantees, and nothing about the reply's CONTENT.
        //
        // What the spec does still require, and what had no test at all, is the
        // scenario "The reply describes no delivery outcome": the reply carries the
        // op id and whether the op was newly stored, "AND it carries no field
        // describing whether the op was sent, accepted, delivered or propagated".
        // That is an assertion about the reply's keys, and it can fail — a future
        // field named `delivered` would trip it, which is the whole point, since the
        // requirement's argument is that a reply carrying the weaker fact sits
        // exactly where a reader looks for the stronger one.
        //
        // Not a duplicate of `a_vote_reply_carries_no_score…`, which pins the same
        // key set: that one is about VOTE semantics (its forbidden list is score /
        // tally / rank) on `publish_vote`. This one is about the DELIVERY outcome on
        // `publish_post`. Same shape of assertion, two different requirements, and a
        // publish-side delivery field would slip past the vote-side test entirely.
        let key = publish_key();
        let mut log = MemoryOpLog::new();
        let out = publish_post(
            &publish_request(r#""body":"whatever delivery does""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        );
        let v = as_json(&out);
        assert!(v.get("error").is_none(), "got {out}");

        // The keys the reply DOES carry, as an exact set rather than a
        // `contains_key` pair — an exact set is what makes a NEW key fail this
        // test, and a new key is the thing the scenario forbids. Hardcoded, not
        // read back from the reply.
        let mut keys: Vec<&str> = v
            .as_object()
            .expect("a reply is an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["opId", "wasNew"],
            "the reply carries exactly the op id and whether the op was new, got {out}"
        );

        // And named individually, so the failure says WHICH outcome word appeared
        // rather than only that the set changed. These are the four the scenario
        // lists, plus the shapes a delivery outcome would plausibly take.
        for forbidden in [
            "sent",
            "accepted",
            "delivered",
            "propagated",
            "delivery",
            "peers",
            "outcome",
        ] {
            assert!(
                v.get(forbidden).is_none(),
                "the reply must describe no delivery outcome, but carries {forbidden:?}: {out}"
            );
        }

        // The two it must carry are the two the requirement names, checked for
        // type and not merely presence.
        assert!(v["opId"].is_string(), "got {out}");
        assert!(v["wasNew"].is_boolean(), "got {out}");
    }

    #[test]
    fn a_publish_refused_for_an_absent_parent_says_which_and_not_that_it_is_the_wrong_kind() {
        // The two refusals the spec requires be told apart, at the wire. A
        // caller distinguishes a propagation gap it should wait out from a
        // category mistake it must fix.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let post_id = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();
        let vote_id = as_json(&publish_vote(
            &publish_request(&format!(r#""target":"{post_id}","direction":"up""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();
        let absent = crate::op::OpId::from_hex(&"7f".repeat(32))
            .unwrap()
            .to_hex();

        let not_held = as_json(&publish_reply(
            &publish_request(&format!(r#""parent":"{absent}","body":"x""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let wrong_kind = as_json(&publish_reply(
            &publish_request(&format!(r#""parent":"{vote_id}","body":"x""#)),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));

        let absent_msg = not_held["error"].as_str().unwrap();
        let kind_msg = wrong_kind["error"].as_str().unwrap();
        assert_ne!(absent_msg, kind_msg, "the two refusals must be told apart");
        assert!(absent_msg.contains("does not hold"), "got {absent_msg}");
        assert!(kind_msg.contains("not a post"), "got {kind_msg}");
        assert!(
            !kind_msg.contains("does not hold"),
            "a held op must not be reported as absent, got {kind_msg}"
        );
    }

    #[test]
    fn the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text() {
        // The refusal that only the adapter can RAISE, worded in the one place a
        // gate can read.
        //
        // Before this existed, `Refusal::NoIdentity` was constructed by nothing
        // outside its own `Display` test while the adapter hand-wrote a second
        // copy of the same sentence — and the adapter is behind
        // `cfg(logos_scaffold)`, which no `cargo test` sets. So there were two
        // copies of one message, no test tying them, and the copy that shipped
        // was the one nothing compiled.
        //
        // This asserts the two are ONE string rather than two that happen to
        // agree: the expected value is built from the variant, so the only way
        // both sides can pass is by `no_identity` going through it. A
        // `no_identity` that formatted its own text — even the same text — fails
        // the moment the variant's wording moves, which is exactly the drift the
        // old arrangement could not detect.
        let why = "the keystore file is not there";
        let out = no_identity(why);
        let v = as_json(&out);
        assert_eq!(
            v["error"]
                .as_str()
                .expect("the error shape carries a string"),
            crate::authoring::Refusal::NoIdentity(why.to_string()).to_string(),
            "the wire reply must be the VARIANT's wording, not a second copy of it"
        );
        assert!(
            v.get("opId").is_none() && v.get("wasNew").is_none(),
            "a refusal is never a partial success — §2.5, got {out}"
        );

        // And the reason it was handed survives into the reply, so a reader is
        // told what is missing rather than only that something is. A
        // `no_identity` that discarded its argument passes the equality above.
        let message = v["error"].as_str().unwrap();
        assert!(message.contains(why), "the reason must survive, got {out}");
        // NO SPEC: the spec's "A refused publish creates no key material" scenario
        // is structural — no keystore or key exists that did not before — and does
        // not require the refusal to say so. Saying it is chosen; see the matching
        // marker on `a_refusal_names_the_id_or_stoa_it_is_about` in `authoring.rs`,
        // and note this pins the sentence rather than the structural claim.
        assert!(
            message.contains("no key was created"),
            "the refusal must say no key material was created, got {out}"
        );
    }

    #[test]
    fn every_publish_refusal_is_the_error_shape_and_carries_no_op_id() {
        // §2.5: never a partial success. A reply carrying both an error and an
        // op id would render as a published post in any view that read `opId`
        // first — and the caller would then link to an op that does not exist.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let stoa = publish_stoa().to_hex();
        let absent = crate::op::OpId::from_hex(&"3a".repeat(32))
            .unwrap()
            .to_hex();

        let cases: [(&str, String); 12] = [
            ("post", "not json".to_string()),
            ("post", r#"{}"#.to_string()),
            ("post", r#"{"stoa":"nothex","body":"x"}"#.to_string()),
            ("post", r#"{"stoa":"00ff","body":"x"}"#.to_string()),
            ("post", format!(r#"{{"stoa":"{stoa}","body":[]}}"#)),
            ("reply", "not json".to_string()),
            (
                "reply",
                format!(r#"{{"stoa":"{stoa}","parent":"nothex","body":"x"}}"#),
            ),
            (
                "reply",
                format!(r#"{{"stoa":"{stoa}","parent":"{absent}","body":"x"}}"#),
            ),
            ("vote", "not json".to_string()),
            (
                "vote",
                format!(r#"{{"stoa":"{stoa}","target":"{absent}","direction":"up"}}"#),
            ),
            (
                "vote",
                format!(r#"{{"stoa":"{stoa}","target":"{absent}","direction":true}}"#),
            ),
            (
                "vote",
                format!(r#"{{"stoa":"{stoa}","target":7,"direction":"up"}}"#),
            ),
        ];
        for (op, request) in cases {
            let out = match op {
                "post" => publish_post(&request, &mut log, &key, &mut ignored_delivery),
                "reply" => publish_reply(&request, &mut log, &key, &mut ignored_delivery),
                _ => publish_vote(&request, &mut log, &key, &mut ignored_delivery),
            };
            let v = as_json(&out);
            assert!(v.get("error").is_some(), "for {request}, got {out}");
            // `error.is_some()` CANNOT TELL A REFUSAL FROM A PANIC, and this test
            // is the only one that reaches some of these parsers' refusal arms.
            // `guarded` catches an unwind and returns
            // `{"error":"panic in <method>: …"}` — which satisfies every other
            // assertion in this loop — so without this line a handler that
            // panicked on all twelve fixtures would pass, and the test would
            // report "every refusal is the error shape" having checked only that
            // the guard in front of the handler works.
            //
            // `findings/security.md` S2 measured exactly this: with a `panic!` on
            // `required_direction`'s `Err` arm, this test passed. The sweep
            // `hostile_publish_input_is_never_a_panic` already carries this
            // assertion; it was missing from the one test that gets here.
            assert!(
                !v["error"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("panic in "),
                "a handler panicked rather than refusing, for {request}: {out}"
            );
            assert!(
                v.get("opId").is_none(),
                "a failure must never also carry an op id — §2.5, got {out}"
            );
            assert!(v.get("wasNew").is_none(), "got {out}");
        }
        assert_eq!(
            log.len().unwrap(),
            0,
            "no refused publish appended anything"
        );
    }

    #[test]
    fn hostile_publish_input_is_never_a_panic() {
        // A panic ABORTS the module process (PHASE0-FINDINGS §3), so an
        // unparseable or adversarial request would be a denial of service
        // against the peer. Every field type, absent fields, maximal lengths and
        // adversarially chosen text, through all three handlers.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let stoa = publish_stoa().to_hex();
        let id = crate::op::OpId::from_hex(&"5e".repeat(32))
            .unwrap()
            .to_hex();

        let mut requests: Vec<String> = vec![
            "".to_string(),
            "not json".to_string(),
            "null".to_string(),
            "[]".to_string(),
            "7".to_string(),
            r#""a string""#.to_string(),
            r#"{}"#.to_string(),
            r#"{"stoa":null,"body":null,"parent":null,"target":null,"direction":null}"#.to_string(),
            r#"{"stoa":{},"body":{},"parent":{},"target":{},"direction":{}}"#.to_string(),
            r#"{"stoa":[1],"body":[1],"parent":[1],"target":[1],"direction":[1]}"#.to_string(),
            r#"{"stoa":true,"body":false,"parent":1.5,"target":-1,"direction":0}"#.to_string(),
        ];
        for text in ["\u{202E}\u{202C}\u{200B}", "\0\0\0", "🏛🏛🏛", "Ἀγορά", "\"}]"] {
            requests.push(
                serde_json::json!({
                    "stoa": stoa, "body": text, "parent": id, "target": id, "direction": text
                })
                .to_string(),
            );
        }
        // Maximal field lengths: at the format's per-field cap, and past it.
        for len in [150 * 1024, 150 * 1024 + 1] {
            requests.push(
                serde_json::json!({
                    "stoa": stoa,
                    "body": "x".repeat(len),
                    "parent": id,
                    "target": id,
                    "direction": "up"
                })
                .to_string(),
            );
        }
        // A hex string of every wrong length, since the op-id parser is reached
        // with attacker-chosen text.
        for len in [0, 1, 63, 64, 65, 128] {
            requests.push(
                serde_json::json!({
                    "stoa": "a".repeat(len),
                    "body": "x",
                    "parent": "b".repeat(len),
                    "target": "c".repeat(len),
                    "direction": "up"
                })
                .to_string(),
            );
        }
        // The same wrong lengths with a VALID Stoa, which is what actually
        // reaches the op-id parser.
        //
        // Every case above malforms the Stoa too, and `stoa` is parsed first in
        // all three handlers — so each of them refuses before `parent` or
        // `target` is ever read, and the op-id parser was reached with nothing
        // but well-formed hex. Verified: an `expect` on `OpId::from_hex` left the
        // whole suite green, this test included, until these cases existed.
        //
        // Non-hex characters as well as wrong lengths, because "64 characters"
        // and "64 HEX characters" are different acceptances and only the second
        // is the parser's.
        for text in [
            "".to_string(),
            "0".to_string(),
            "z".repeat(64),
            "g".repeat(64),
            "0".repeat(63),
            "0".repeat(65),
            "0".repeat(128),
            "ff ".repeat(21),
            "0x".to_string() + &"0".repeat(64),
            "\u{200B}".repeat(64),
            "Ἀγορά".to_string(),
        ] {
            requests.push(
                serde_json::json!({
                    "stoa": stoa,
                    "body": "x",
                    "parent": text,
                    "target": text,
                    "direction": "up"
                })
                .to_string(),
            );
        }
        // A VALID Stoa and a VALID target with a WRONG-TYPED `direction` and
        // `body`, which is the only way to reach those parsers' wrong-type arms.
        //
        // This is the same ordering hazard the op-id block above exists for, one
        // parser further along — and it was found the same way. `stoa` is parsed
        // first in all three handlers, so every wrong-typed fixture earlier in
        // this sweep malforms `stoa` too and dies at parser one; and the two
        // blocks that DO carry a valid Stoa both pin `direction: "up"`. The
        // result was that `required_direction`'s wrong-typed arm was never
        // entered by any test in the repo.
        //
        // Measured, per `findings/security.md` S2: with a `panic!` on that arm
        // alone, the whole 566-test suite stayed green. With these fixtures it
        // does not. Note the absent arm was NOT the gap — a missing `direction`
        // is caught by `a_publish_requests_missing_field_is_named_and_is_not_defaulted`,
        // which asserts on the message naming the field, and a panic message
        // does not contain it. The wrong-typed arm is the one nothing reached.
        //
        // `null` is included deliberately, and it is a wrong TYPE rather than an
        // absence: `wire/request.rs:229` keeps an explicit null as
        // `Some(Value::Null)` on purpose, so `required_string` refuses it by type.
        // Pinned in the message-level test beside this one.
        for wrong in [
            serde_json::Value::Null,
            serde_json::json!(true),
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!({}),
            serde_json::json!([]),
            serde_json::json!(["up"]),
            serde_json::json!({"direction": "up"}),
        ] {
            requests.push(
                serde_json::json!({
                    "stoa": stoa,
                    "body": wrong,
                    "parent": id,
                    "target": id,
                    "direction": wrong
                })
                .to_string(),
            );
        }

        for request in &requests {
            for out in [
                publish_post(request, &mut log, &key, &mut ignored_delivery),
                publish_reply(request, &mut log, &key, &mut ignored_delivery),
                publish_vote(request, &mut log, &key, &mut ignored_delivery),
            ] {
                let v = as_json(&out);
                assert!(
                    v.is_object(),
                    "every reply is a JSON object, got {out} for {request}"
                );
                // `is_object()` ALONE cannot see a panic, and that is the whole
                // point of this test. `guarded` catches the unwind and returns
                // `{"error":"panic in <method>: …"}` — a perfectly well-formed
                // object — so a handler that panicked on every one of these
                // inputs would satisfy the assertion above.
                //
                // The marker `guarded` writes is what tells the two apart, and it
                // is the only thing that does. Without this line the requirement
                // "a publish SHALL NOT panic for any request" has no test, only a
                // test of the guard that stands in front of it.
                assert!(
                    !v["error"]
                        .as_str()
                        .unwrap_or_default()
                        .starts_with("panic in "),
                    "a handler panicked rather than refusing, for {request}: {out}"
                );
            }
        }

        // Whatever this sweep DID accept must be readable back. "No panic" is not
        // the only way hostile input can win: an input that succeeds and stores an
        // op the decoder refuses is worse, because it is reported to the caller as
        // a publish and then kills every read on the store.
        //
        // That is not hypothetical — it is how the over-cap body defect survived
        // this test. The sweep already fed it `MAX_FIELD_LEN + 1`, asserted "an
        // object and not a panic", and passed, because the handler *succeeded*.
        // Asserting the absence of a panic cannot see a wrongful success.
        for entry in log.iter().expect("the sweep's log must still be readable") {
            crate::op::SignedOp::from_bytes(&entry.op.to_bytes())
                .expect("an op a publish accepted must decode again");
        }
    }

    #[test]
    fn a_body_reaches_the_op_through_the_wire_exactly_as_supplied() {
        // No normalisation, no trimming, no case-folding. `op-format` contracts
        // an accepted encoding as re-encoding to itself, so a transformation
        // here would mean the op published is not the content the caller
        // supplied — and sanitisation is a RENDERING concern `feed.rs` applies
        // on the way out.
        let key = publish_key();
        for body in [
            "  padded  ",
            "MiXeD",
            "caf\u{00E9}",
            "cafe\u{0301}",
            "\u{202E}reversed",
            "zero\u{200B}width",
            "line\nbreak",
        ] {
            let mut log = MemoryOpLog::new();
            let request =
                serde_json::json!({ "stoa": publish_stoa().to_hex(), "body": body }).to_string();
            let out = publish_post(&request, &mut log, &key, &mut ignored_delivery);
            let v = as_json(&out);
            assert!(v.get("error").is_none(), "for {body:?}, got {out}");
            let id = crate::op::OpId::from_hex(v["opId"].as_str().unwrap()).unwrap();
            match &log.get(&id).unwrap().unwrap().op.op.kind {
                OpKind::Post { body: stored, .. } => assert_eq!(
                    stored.as_bytes(),
                    body.as_bytes(),
                    "the body must reach the op byte for byte"
                ),
                other => panic!("expected a post, got {other:?}"),
            }
        }
    }

    #[test]
    fn two_bodies_differing_only_by_normalisation_publish_as_two_ops_through_the_wire() {
        // The sharp case for "no normalisation": NFC "é" against NFD "e"+U+0301
        // render identically and are different bytes. A wire layer that
        // normalised would collapse them into one op.
        let mut log = MemoryOpLog::new();
        let key = publish_key();
        let stoa = publish_stoa().to_hex();

        let composed = as_json(&publish_post(
            &serde_json::json!({"stoa": stoa, "body": "caf\u{00E9}"}).to_string(),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        let decomposed = as_json(&publish_post(
            &serde_json::json!({"stoa": stoa, "body": "cafe\u{0301}"}).to_string(),
            &mut log,
            &key,
            &mut ignored_delivery,
        ));
        assert_ne!(composed["opId"], decomposed["opId"]);
        assert_eq!(log.len().unwrap(), 2);
    }

    #[test]
    fn the_three_handlers_share_one_signature_the_adapter_can_dispatch_over() {
        // All three handlers must be interchangeable: same shape, same sink
        // type, so one can be substituted for another without a signature
        // change. This test pins that by naming ONE function-pointer type and
        // requiring all three to coerce into it.
        //
        // The single pointer type is THIS TEST'S choice, not a constraint the
        // adapter imposes — `Dialectica::publishing` is generic over the handler
        // and would accept three distinct types. The reason to pin it here is
        // that the adapter lives behind `cfg(logos_scaffold)`, which no
        // `cargo test` ever sets (`lib.rs` explains why at length), so a
        // signature that drifted out of line would fail in the BUILDER's build —
        // the one that runs last and reports worst. This is the only gate that
        // can catch that drift early, which is why the erased `&mut dyn FnMut`
        // is worth its cost.
        type Handler = fn(
            &str,
            &mut MemoryOpLog,
            &crate::identity::SecretKey,
            &mut dyn FnMut(&crate::op::OpId),
        ) -> String;

        let key = publish_key();
        let stoa = publish_stoa().to_hex();
        let mut log = MemoryOpLog::new();
        let seed = as_json(&publish_post(
            &publish_request(r#""body":"the subject""#),
            &mut log,
            &key,
            &mut ignored_delivery,
        ))["opId"]
            .as_str()
            .unwrap()
            .to_string();

        let cases: [(Handler, String); 3] = [
            (
                publish_post,
                serde_json::json!({"stoa": stoa, "body": "through a pointer"}).to_string(),
            ),
            (
                publish_reply,
                serde_json::json!({"stoa": stoa, "parent": seed, "body": "likewise"}).to_string(),
            ),
            (
                publish_vote,
                serde_json::json!({"stoa": stoa, "target": seed, "direction": "up"}).to_string(),
            ),
        ];
        let mut delivered = Vec::new();
        for (handler, request) in cases {
            let out = handler(&request, &mut log, &key, &mut |id| delivered.push(*id));
            let v = as_json(&out);
            assert!(v.get("error").is_none(), "for {request}, got {out}");
            assert!(v["opId"].is_string(), "got {out}");
        }
        assert_eq!(
            delivered.len(),
            3,
            "each dispatched handler must reach the shared sink"
        );
    }

    #[test]
    fn the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does() {
        // THE REGRESSION TEST FOR THE DEFECT THAT MADE THIS PIECE'S HEADLINE
        // CLAIM TRUE OF THE CRATE AND FALSE OF THE MODULE.
        //
        // `Dialectica::publishing` must read `stoa` before it can derive a
        // per-Stoa signing key, and only the adapter can open a keystore — so
        // that early read is structural, not incidental. It was doing it with
        // its own bare `serde_json::from_str` and its own four-arm ladder, at a
        // line `cargo test` does not compile. The effect was that every
        // envelope fix in this change was shadowed on the shipped path:
        //
        //   - `[]` was answered `{"error":"missing field: stoa"}` by the
        //     adapter, never reaching REQUEST_NOT_AN_OBJECT;
        //   - an N-byte request was fully parsed at ~2N transient heap BEFORE
        //     MAX_REQUEST_BYTES was evaluated, so the cap bounded only a second
        //     parse of bytes already paid for — and per PHASE0-FINDINGS §3 an
        //     allocation failure there aborts the module process.
        //
        // This is the same kind of test as
        // `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`
        // directly above, and for the same reason: the adapter is behind
        // `cfg(logos_scaffold)`, so the only way to pin a property of it from a
        // gate that runs is to pin the `core` function it is obliged to use.
        // Build LGX proves the adapter compiles; it asserts nothing about the
        // order of operations inside it. CI's adapter-derivation gate is what
        // holds the adapter to CALLING this — the two halves together are the
        // claim.
        let stoa = publish_stoa();

        // 1. A non-object earns the ENVELOPE's refusal, not a missing-field
        //    one. This is the assertion that fails against a bare `from_str`,
        //    and it is on the message rather than on `is_err` because an array
        //    is an error either way — the two-explanations-one-answer shape.
        for not_an_object in ["[]", "7", r#""s""#, "true", "null"] {
            let refused = stoa_of(not_an_object)
                .expect_err("the adapter's Stoa read must refuse a non-object");
            assert_eq!(
                refused,
                error_json(REQUEST_NOT_AN_OBJECT),
                "the adapter's early read must refuse {not_an_object} for its \
                 SHAPE. A bare `serde_json::from_str` answers \
                 `missing field: stoa` here, which is the defect."
            );
        }

        // 2. The size cap is evaluated BEFORE the parse, which is the whole
        //    point of a cap that exists to bound allocation. Asserted the only
        //    way it is observable from a return value — the same way
        //    `an_oversized_request_is_refused_before_it_is_parsed` does it
        //    inside `Request::parse`: feed in something both oversized AND
        //    unparseable, and check which refusal comes back. Only an
        //    implementation that measures length before calling `from_str` can
        //    answer with the size refusal.
        let oversized_and_unparseable = "{".repeat(MAX_REQUEST_BYTES + 1);
        let refused =
            stoa_of(&oversized_and_unparseable).expect_err("an oversized request must be refused");
        assert!(
            refused.contains("over the") && refused.contains("byte limit"),
            "the adapter's early read must refuse an oversized request for its \
             SIZE, got {refused}"
        );
        assert!(
            !refused.contains("invalid JSON"),
            "the size check must run BEFORE the parse on the adapter's path \
             too, or the allocation the cap exists to prevent has already \
             happened, got {refused}"
        );

        // 3. And it still answers the question the adapter actually asked, so
        //    none of the above is satisfied by a function that refuses
        //    everything.
        let served = stoa_of(&publish_request(r#""body":"anything""#))
            .expect("a well-formed publish request must yield its Stoa");
        assert_eq!(served, stoa, "the Stoa read must be the Stoa named");

        // 4. It reads the Stoa and NOTHING else. A forbidden field and a
        //    missing `body` are the handler's to refuse, and an adapter that
        //    refused them too would be a second reader of what a publish must
        //    contain — the shape that produced this defect in the first place.
        let with_forbidden = stoa_of(&publish_request(r#""author":"someone else""#))
            .expect("the adapter's read must not run the handler's guards");
        assert_eq!(with_forbidden, stoa);
    }

    // ─── The request envelope ─────────────────────────────────────────────
    //
    // WHY THESE TESTS LOOK OVERBUILT. Every hostile-input fixture already in
    // this file is refused *whether or not* the envelope is checked:
    // `"not json"` dies at the parse, and `{}` is refused for its missing
    // field. Two explanations, one answer — so a test asserting only
    // `error.is_some()` for `[]` passes against the unfixed code, because
    // `parsed.get("stoa")` returns `None` for an array exactly as it does for
    // an object without the field.
    //
    // The distinguishing assertion is therefore on the MESSAGE, and against a
    // hardcoded expectation rather than "differs from the other one".

    /// A directory name unique to this call, for the sweep fixtures that need one.
    ///
    /// **`OnboardingDir::new` is not safe to call twice with one name**, and the
    /// sweeps are what make that reachable. Its name carries only
    /// `std::process::id()`, and it `remove_dir_all`s the path before creating it —
    /// which is what makes a *reused* name work across runs. Two live directories
    /// of the same name inside one binary is the case that breaks: the second
    /// call's pre-emptive removal deletes the first's, and the first's open SQLite
    /// handle then fails with `Storage("disk I/O error")`.
    ///
    /// That is not hypothetical. `who_am_i` and `keep_identity` each open a record
    /// per call and the sweeps below call them many times over, in tests that run
    /// on parallel threads. With a fixed name per fixture, two of the sweeps
    /// panicked on exactly that message — and they panicked only under a mutation,
    /// so a fixed name would have passed here and failed on some later change for
    /// a reason nobody would have connected to this one.
    ///
    /// The counter is per-binary and monotonic, so no two calls collide however the
    /// harness schedules them.
    fn sweep_dir_name(role: &str) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("sweep-{role}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// A method named, and callable with a raw request string.
    type NamedMethod = (&'static str, fn(&str) -> String);

    /// The body of the root post the publish sweeps seed their log with.
    ///
    /// Named once because two functions have to agree about it: the one that
    /// seeds the log and the one that names the resulting op id. They agree
    /// because an op id is derived from the signed bytes and nothing varying
    /// between runs participates — `content-authoring`'s "one identity
    /// publishes the same Stoa and the same body" scenario is that property.
    const SWEPT_ROOT_BODY: &str = "a root for the sweeps to reply to";

    /// A fresh log holding one root post by the publish key, so a reply or a
    /// vote in the sweeps has something that exists to name.
    fn a_log_seeded_with_a_root() -> MemoryOpLog {
        let mut log = MemoryOpLog::new();
        crate::authoring::post(
            &mut log,
            &publish_key(),
            publish_stoa(),
            SWEPT_ROOT_BODY.to_string(),
        )
        .expect("seeding a root post must succeed");
        log
    }

    /// The op id [`a_log_seeded_with_a_root`] produces, as hex.
    ///
    /// Derived by running the same publish into a throwaway log rather than
    /// hardcoded: a hardcoded id would be a value read back from the
    /// implementation once and then never checked again, and the property that
    /// matters here is that the two logs agree — which re-deriving asserts and
    /// a literal does not.
    fn a_seeded_root_id() -> String {
        crate::authoring::post(
            &mut MemoryOpLog::new(),
            &publish_key(),
            publish_stoa(),
            SWEPT_ROOT_BODY.to_string(),
        )
        .expect("deriving the seeded root id must succeed")
        .id
        .to_hex()
    }

    /// Every method whose request has **at least one required field**, so `{}` is a
    /// refusal for it and the two missing-field sweeps can assert on the message.
    ///
    /// # Why this is a list and not a branch inside each sweep
    ///
    /// `list_stoas` is the surface's first method whose every field is optional:
    /// `{}` is a *served* request for it, answering page 0 of the peer's Stoas. Two
    /// sweeps — `the_three_refusals_a_caller_can_earn_are_three_different_messages`
    /// and `an_empty_object_is_refused_for_its_missing_field_and_never_for_its_shape`
    /// — read `{}` as the third caller mistake, which it is for every other listed
    /// method and is not for that one.
    ///
    /// The alternative was `if name != "list_stoas"` inside each sweep, rejected for
    /// CLAUDE.md's reason: two call sites that have to agree about which methods are
    /// exempt is the shape where the third sweep gets it wrong. Naming the property
    /// once puts it in the data, and a new all-optional method is added by leaving it
    /// out of this list rather than by editing a sweep.
    ///
    /// **This does NOT exempt anything from the envelope.** The shape sweep and the
    /// size sweep still run over every method in
    /// [`every_request_taking_method`] — `list_stoas` included, and both of them
    /// caught it bypassing `Request::parse`.
    fn every_method_with_a_required_field() -> Vec<NamedMethod> {
        every_request_taking_method()
            .into_iter()
            .filter(|(name, _)| *name != "list_stoas")
            .collect()
    }

    /// A named field, the request that supplies it, and the call that reads it.
    ///
    /// A boxed closure rather than a `fn` pointer because each case captures a
    /// different fixture — a `log`, a lookup, a genesis record — and a plain `fn`
    /// cannot close over any of them. The alias is for `clippy::type_complexity`,
    /// the same reason `NamedMethod` exists.
    type NullReadingCase = (&'static str, String, Box<dyn Fn(&str) -> String>);

    /// Every method that **reads a field of its request**, behind one uniform
    /// call, so a new method is added to the sweep in one place rather than to
    /// each test.
    ///
    /// The name is the contract's scope, which is the field read and not the
    /// parameter: reading one field is enough to be inside the envelope rule, and
    /// requiring none is not enough to be outside it.
    ///
    /// # ADD YOUR METHOD HERE
    ///
    /// **If you are adding a method to this crate's wire surface that reads a
    /// field of its request, add it to this list and give it a fixture in
    /// [`a_served_request`]. That is an obligation, not a courtesy.**
    ///
    /// Nothing checks it, and the cost was measured rather than imagined: a
    /// reviewer built a sixth method — a handler parsing `Value` directly with
    /// all-optional fields, serving `[]` as a request that named nothing — and the
    /// whole suite passed. An unlisted method is silently unswept, every sweep
    /// below goes green without it, and a guarantee about five methods reads as a
    /// guarantee about the surface.
    ///
    /// The compiler cannot force this. Moving `Request` behind a module boundary
    /// makes it impossible to hold one without the check, but nothing obliges a
    /// handler to hold one at all — see
    /// `the_sixth_method_the_boundary_does_not_stop`, which builds that method and
    /// demonstrates it. And a source-scanning test was rejected for failing on
    /// unrelated things (see `design.md`'s rejected alternatives). So the
    /// obligation is written here, where an author adding a method has to be in
    /// order to add it.
    ///
    /// Two absences are deliberate rather than forgotten, and both are now
    /// governed by the spec rather than chosen here:
    ///
    /// - `panic_probe` takes a request and reads no field of it, passing it
    ///   through as opaque text. The envelope rule's third case puts it outside,
    ///   and its own requirement gives it a contract instead (design.md §4).
    /// - `version` takes no request at all — the rule's second case.
    fn every_request_taking_method() -> Vec<NamedMethod> {
        fn ping_m(r: &str) -> String {
            ping(r)
        }
        fn caps_m(r: &str) -> String {
            get_capabilities(r, |_| Ok("abcd".to_string()))
        }
        fn feed_m(r: &str) -> String {
            list_threads(r, &log_with_body("hello"), &feed_genesis())
        }
        fn feed_req_m(r: &str) -> String {
            list_threads_from_request(r, || {
                Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
            })
        }
        fn channel_m(r: &str) -> String {
            // `parse_channel_id` returns the wire shape on both arms, so an
            // `Ok` is folded into a reply in order to be swept uniformly. The
            // sweep asserts on refusals, so the success arm's exact shape does
            // not matter — only that it is not an error.
            match parse_channel_id(r) {
                Ok(id) => serde_json::json!({ "channelId": id }).to_string(),
                Err(e) => e,
            }
        }
        // The three `identity-onboarding` added. Each reads `stoa`, so each is
        // inside the envelope rule's first case, and each is here because the
        // obligation above is an obligation: when `main`'s envelope merged in, all
        // three still called `serde_json::from_str` themselves and served `[]` as a
        // request that named no Stoa. Adding them to this list is what turned the
        // five sweeps below red and named the three handlers doing it.
        //
        // Each takes a session and a closure, so each is a `fn` wrapping them with
        // a fresh session per call — the sweeps call these repeatedly and a session
        // shared across calls would make one sweep's slate live for the next.
        fn slate_m(r: &str) -> String {
            generate_identity_slate(&mut OnboardingSession::new(), r, || {
                Ok(Keystore::from_root_for_test([7u8; 32]))
            })
        }
        fn keep_m(r: &str) -> String {
            let dir = OnboardingDir::new(&sweep_dir_name("keep"));
            let paths = dir.paths();
            keep_identity(
                &mut OnboardingSession::new(),
                r,
                || Ok(Keystore::from_root_for_test([7u8; 32])),
                KeepTargets {
                    keystore_path: &dir.keystore_path(),
                    unlock: &Unlock::Unencrypted,
                    paths: &paths,
                },
            )
        }
        fn whoami_m(r: &str) -> String {
            // The `paths` opener is `impl Fn`, called once per handler call but
            // typed as re-callable, so it opens the record rather than moving one
            // in. The directory guard outlives the call.
            let dir = OnboardingDir::new(&sweep_dir_name("whoami"));
            who_am_i(
                r,
                || Ok(Keystore::from_root_for_test([7u8; 32])),
                || Ok(dir.paths()),
            )
        }
        // The three `stoa-lifecycle` added, listed for the reason the three above
        // are: when `main`'s envelope merged into this piece, all three still called
        // `serde_json::from_str` themselves. `create_stoa` reads `title` and the
        // other two read `stoa`, so all three are inside the envelope rule's first
        // case, and all three served `[]` as a request naming no field.
        //
        // Each takes a store, so each is a `fn` building a fresh one per call — the
        // sweeps call these repeatedly and a store shared across calls would make
        // one sweep's Stoa visible to the next.
        fn create_m(r: &str) -> String {
            create_stoa(
                r,
                || Ok(feed_key(1).public_key()),
                &mut a_membership_store(),
            )
        }
        fn join_m(r: &str) -> String {
            join_stoa(r, &mut a_membership_store())
        }
        fn list_stoas_m(r: &str) -> String {
            list_stoas(r, &a_membership_store())
        }
        // The three `content-authoring` added, and the reason this list exists:
        // they entered the crate's wire surface and did not enter this list, so
        // every sweep below ran green without them while all three bypassed the
        // envelope entirely. Each reads `stoa`, so each is inside the envelope
        // rule's first case.
        //
        // Each takes a log, a key and a delivery sink, so each is a `fn`
        // building a fresh log per call — the sweeps call these repeatedly and
        // a log shared across calls would make one sweep's op visible to the
        // next.
        //
        // `publish_reply` and `publish_vote` need their parent and target to be
        // IN that fresh log, or their served fixture is refused for an absent
        // parent rather than served — which would make
        // `an_object_supplying_only_its_required_fields_is_served` assert
        // against a refusal it cannot tell from an envelope one. So each seeds
        // the root post first, and `a_seeded_root_id` names what seeding makes.
        fn publish_post_m(r: &str) -> String {
            publish_post(
                r,
                &mut MemoryOpLog::new(),
                &publish_key(),
                &mut ignored_delivery,
            )
        }
        fn publish_reply_m(r: &str) -> String {
            let mut log = a_log_seeded_with_a_root();
            publish_reply(r, &mut log, &publish_key(), &mut ignored_delivery)
        }
        fn publish_vote_m(r: &str) -> String {
            let mut log = a_log_seeded_with_a_root();
            publish_vote(r, &mut log, &publish_key(), &mut ignored_delivery)
        }
        vec![
            ("ping", ping_m),
            ("get_capabilities", caps_m),
            ("list_threads", feed_m),
            ("list_threads_from_request", feed_req_m),
            ("parse_channel_id", channel_m),
            ("generate_identity_slate", slate_m),
            ("keep_identity", keep_m),
            ("who_am_i", whoami_m),
            ("create_stoa", create_m),
            ("join_stoa", join_m),
            ("list_stoas", list_stoas_m),
            ("publish_post", publish_post_m),
            ("publish_reply", publish_reply_m),
            ("publish_vote", publish_vote_m),
        ]
    }

    /// The adapter's source, read as text at compile time.
    ///
    /// `../../src/lib.rs` from this file is `rust-lib/src/lib.rs`, the module
    /// crate's adapter. Reading it is a *file* operation, so none of what makes
    /// that file untestable applies: `include_str!` does not compile it, does
    /// not link it, and is not gated by `cfg(logos_scaffold)`.
    ///
    /// **The path survives the Nix build.** `mkLogosModule.nix` stages
    /// `codegen.rust.crate` — `rust-lib/` — with `cp -r`, and this crate is
    /// nested inside it, so `rust-lib/src/lib.rs` and
    /// `rust-lib/dialectica-core/src/wire.rs` are staged together and the
    /// relative path between them is unchanged. That nesting is load-bearing
    /// for the build already (see `dialectica-core/Cargo.toml`'s placement
    /// note); this inherits it rather than adding a new requirement.
    const ADAPTER_SOURCE: &str = include_str!("../../src/lib.rs");

    /// Every method the dispatch trait declares as taking a request, read out
    /// of the trait declaration rather than retyped.
    ///
    /// # Why this is the surface, and why a signature is the discriminator
    ///
    /// `DialecticaModule` is the dispatch contract: `interface: "universal"`
    /// derives the RPC table from it, so a method not declared there is not on
    /// the wire and a method that is, is. Within it the shape is uniform and
    /// total — `fn <name>(&mut self, request: String) -> String` takes a
    /// request, `fn <name>(&mut self) -> String` takes none, and
    /// `on_context_ready` takes a context.
    ///
    /// **So this is not the source-scanning test that was rejected.** That one
    /// looked for `serde_json::from_str`, which appears in doc comments, in
    /// test helpers and in reply *decoders* — three ways to go red for a reason
    /// it does not name, which is why the `wire-request-envelope` change's
    /// `design.md` ruled it out. A parameter list is none of those: a doc
    /// comment does not contain one, and a helper is not declared in this
    /// trait.
    ///
    /// **Nor is it the trait-driven sweep that change called structurally
    /// impossible.** That one wanted the trait as a *type*, which needs the
    /// `dialectica` crate (the dependency points the wrong way) and needs
    /// `cfg(logos_scaffold)` (which no `cargo test` sets). Both objections are
    /// about *compiling* the trait. Reading its declaration as text needs
    /// neither — which is what makes this reachable from here, and is the part
    /// that document did not consider rather than a part it got wrong.
    ///
    /// # It CLASSIFIES every method rather than filtering for one shape
    ///
    /// This is the correction for a defect review measured rather than
    /// imagined. The first version matched
    /// `"&mut self, request: String) -> String;"` against a single trimmed
    /// line, and a reviewer defeated it **two ways, both silent**, by adding
    /// `publish_moderation` to the trait and watching this gate stay green:
    ///
    /// - a signature long enough that rustfmt wraps it across four lines, so no
    ///   single line carried the pattern; and
    /// - a parameter named `req` rather than `request`, which is a
    ///   byte-for-byte identical dispatch surface because a parameter name has
    ///   no compiler or codegen consequence.
    ///
    /// A filter cannot catch either, because **a filter's failure mode is
    /// silence**: an unrecognised method is simply absent from the result, and
    /// the `!found.is_empty()` backstop never fires while the other fourteen
    /// still parse. That backstop only ever caught a *total* change of shape,
    /// never the partial one that actually happens — one method written
    /// differently from the rest.
    ///
    /// So this enumerates **every** `fn` in the trait and puts each into
    /// exactly one bucket: takes a request, takes none, or takes something
    /// else. A method matching no bucket is a **loud panic naming it**, not an
    /// omission. That is what turns the parser's own blind spot from a silent
    /// pass into a failure, and it is why the two evasions above are now both
    /// caught by one mechanism rather than by two patches.
    ///
    /// # Its preconditions, stated because a guard with undocumented limits is
    /// worse than one known to be partial
    ///
    /// It assumes the trait declaration is Rust that rustfmt produced, that
    /// each method declaration ends in `;`, and that a request parameter is
    /// typed `String` by value. A method taking `&str`, or `impl Into<String>`,
    /// or two parameters, lands in the "something else" bucket and **fails
    /// loudly** rather than passing — which is the correct direction for a
    /// shape nobody has considered, and is the property the first version
    /// lacked.
    fn the_dispatch_traits_request_taking_methods() -> Vec<String> {
        // The trait DECLARATION, not the impl — the impl repeats every
        // signature, and counting both would double every name. The declaration
        // is also the authority: a method declared and not implemented does not
        // compile, so it cannot be the shorter of the two.
        let (_, after) = ADAPTER_SOURCE
            .split_once("pub trait DialecticaModule")
            .expect("the adapter must declare the dispatch trait");
        let (body, _) = after
            .split_once("\n}\n")
            .expect("the trait declaration must be closed by a `}` at column 0");

        // Strip doc comments and line comments BEFORE anything else: a `fn` or
        // a `;` inside prose would otherwise be read as code. Then collapse all
        // whitespace, so a signature rustfmt wrapped across four lines and one
        // it left on a single line are the same string by the time it is
        // matched. This is what closes the wrapped-signature evasion, and it
        // closes it for every future method rather than for the one that was
        // probed.
        let code: String = body
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.starts_with("//"))
            .collect::<Vec<_>>()
            .join(" ");
        let normalised = code.split_whitespace().collect::<Vec<_>>().join(" ");

        let mut found = Vec::new();
        let mut unclassified = Vec::new();

        // Split on `fn ` rather than on lines, so wrapping is irrelevant.
        for piece in normalised.split("fn ").skip(1) {
            let Some((name, rest)) = piece.split_once('(') else {
                continue;
            };
            let name = name.trim();

            // A defaulted method carries a body, so its declaration does not
            // end at a `;`. `on_context_ready` is the only one today, and this
            // keeps it out without naming it.
            let Some((params_and_ret, _)) = rest.split_once(';') else {
                continue;
            };
            // Everything between the parens, whitespace already normalised.
            let Some((params, ret)) = params_and_ret.split_once(')') else {
                continue;
            };
            if ret.trim() != "-> String" {
                unclassified.push(format!("{name} (returns `{}`)", ret.trim()));
                continue;
            }

            // Drop the receiver; what is left is the parameter list.
            //
            // The TRAILING COMMA is stripped because rustfmt emits one on every
            // wrapped signature — `fn f(\n &mut self,\n request: String,\n)` —
            // and it is the same declaration as the unwrapped form. Not
            // stripping it was not merely cosmetic: it put a perfectly ordinary
            // wrapped method into the unclassified bucket, so the gate failed
            // for the wrong reason and an author would have "fixed" it by
            // teaching the parser a shape it already understood.
            let params = params.trim().trim_end_matches(',').trim();
            let rest_of_params = match params.strip_prefix("&mut self") {
                Some(r) => r.trim_start().trim_start_matches(',').trim(),
                None => {
                    unclassified.push(format!("{name} (receiver `{params}`)"));
                    continue;
                }
            };

            if rest_of_params.is_empty() {
                // Takes no request — the envelope rule's second case. Outside
                // the rule with nothing to check, and `version` is the one.
                continue;
            }

            // MATCHED ON THE TYPE, NOT THE NAME. `request: String` and
            // `req: String` are the same dispatch surface, so keying on the
            // name is what let a one-word rename evade this. The name is
            // ignored entirely; only `: String` decides.
            match rest_of_params.split_once(':') {
                Some((_param_name, ty)) if ty.trim() == "String" => {
                    found.push(name.to_string());
                }
                _ => unclassified.push(format!("{name} (parameters `{rest_of_params}`)")),
            }
        }

        // THE LOUD FAILURE THAT REPLACES A SILENT OMISSION. A method whose
        // shape this parser does not recognise is the exact case that made the
        // first version evadable, so it panics NAMING the method rather than
        // leaving it out of the result. Erring toward a red test on an
        // unfamiliar shape is the correct direction: the cost is an author
        // teaching this function one new shape, and the alternative is a
        // dispatch method reaching the wire unswept.
        assert!(
            unclassified.is_empty(),
            "the dispatch trait declares method(s) whose shape this parser does \
             not recognise: {unclassified:?}\n\nIt cannot tell whether they read \
             a request, so it will not silently assume they do not. Teach \
             `the_dispatch_traits_request_taking_methods` the new shape, and add \
             the method to `every_request_taking_method` if it reads a field of \
             its request."
        );

        found.sort();
        assert!(
            !found.is_empty(),
            "no request-taking method was found in the trait declaration — the \
             signature shape this test reads by has changed, so it is now \
             measuring nothing rather than failing"
        );
        found
    }

    #[test]
    fn the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares() {
        // THE TEST THAT MAKES THE LIST UNABLE TO GO STALE, and the defect it
        // exists for is measured rather than imagined: `publish_post`,
        // `publish_reply` and `publish_vote` entered this crate's wire surface
        // and did not enter `every_request_taking_method`, so five sweeps ran
        // green over eleven methods while the surface had fourteen — and all
        // three bypassed the envelope.
        //
        // It is the fourth copy of one guard becoming a data structure. That
        // list's doc said "ADD YOUR METHOD HERE ... Nothing checks it".
        // Something checks it now, and what it checks against is the dispatch
        // trait rather than a second hand-written list that could go stale the
        // same way.
        //
        // The exclusion is NAMED rather than filtered silently, because the
        // spec names it: the envelope rule's third case, a method that takes a
        // request and reads no field of it. `version` is the second case and is
        // outside by signature, so it never reaches this list at all.
        //
        // A new method arriving in the trait is therefore a RED TEST naming it,
        // not a silent drop in coverage.
        const OUTSIDE_THE_ENVELOPE_RULE: [&str; 1] = [
            // "A method that takes a request and reads no field of it —
            // passing it through as opaque text — is outside it." The probe's
            // own requirement obliges it to reach its panic for every request
            // shape, so the two rules cannot both reach it and that one wins.
            "panic_probe",
        ];

        let expected: Vec<String> = the_dispatch_traits_request_taking_methods()
            .into_iter()
            .filter(|name| !OUTSIDE_THE_ENVELOPE_RULE.contains(&name.as_str()))
            .collect();
        let swept: Vec<String> = every_request_taking_method()
            .into_iter()
            .map(|(name, _)| name.to_string())
            .collect();

        // `parse_channel_id` is in the sweep and is not a trait method: it is
        // the request-reading half of `delivery_channel_exists`, where that
        // method's envelope check actually lives and the whole of it a test
        // binary can reach (`modules()` calls `lp_*` symbols undefined here).
        // So the sweep covers the trait method THROUGH it, and the mapping is
        // stated rather than left to look like a name mismatch.
        let covered_under_another_name = |name: &str| match name {
            "delivery_channel_exists" => swept.iter().any(|s| s == "parse_channel_id"),
            _ => false,
        };

        let missing: Vec<&String> = expected
            .iter()
            .filter(|name| !swept.contains(name) && !covered_under_another_name(name))
            .collect();
        assert!(
            missing.is_empty(),
            "these methods are on the dispatch surface and are NOT swept for \
             the request envelope: {missing:?}\n\nAdd each to \
             `every_request_taking_method` and give it a fixture in \
             `a_served_request`. An unswept method is one no test checks the \
             size cap, the non-object refusal or the three distinct messages \
             for."
        );
    }

    /// A request each method would serve, so a refusal in the sweeps below is
    /// attributable to the thing being varied and not to a missing field.
    fn a_served_request(method: &str) -> String {
        match method {
            "ping" => r#"{"payload":1}"#.to_string(),
            "get_capabilities" => format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()),
            "list_threads" => feed_request(""),
            "list_threads_from_request" => full_request(),
            "parse_channel_id" => r#"{"channelId":"stoa-abc/e7"}"#.to_string(),
            "generate_identity_slate" | "who_am_i" => {
                format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex())
            }
            // `slate` and `index` are required FIELDS, so they must be present or
            // the sweeps that assert a served request see a missing-field error.
            // The nonce is a well-formed one that is not live, which makes the
            // reply `{"kept":false,"reason":…}` — an answer and not the error
            // shape, which is what these sweeps check. That `Kept::Refused` is not
            // `{"error":…}` is this handler's own contract, asserted where that
            // contract is tested; here it is only what makes the fixture served.
            // The nonce is spelled as a hex literal rather than built from bytes:
            // `SlateNonce` has no `from_bytes`, and adding one to the production
            // API so a test fixture can name a value is widening the surface for a
            // test's convenience.
            "keep_identity" => format!(
                r#"{{"stoa":"{}","slate":"{}","index":0}}"#,
                a_stoa().to_hex(),
                "03".repeat(32)
            ),
            // `create_stoa` reads `title`; the other two are the `stoa`/`genesis`
            // pair and the pagination shape. `join_stoa` is served with a record
            // that MATCHES its address, because a mismatch is a refusal and these
            // sweeps need the fixture to be served.
            "create_stoa" => r#"{"title":"Agora"}"#.to_string(),
            "join_stoa" => {
                let genesis = a_joinable_record("Swept");
                join_request(&genesis, &genesis.address().unwrap())
            }
            "list_stoas" => "{}".to_string(),
            // The three publish handlers. `publish_post` needs only a body; the
            // other two also name an op that must EXIST in the log their
            // wrapper seeds, which `a_seeded_root_id` supplies.
            "publish_post" => publish_request(r#""body":"swept""#),
            "publish_reply" => publish_request(&format!(
                r#""parent":"{}","body":"swept reply""#,
                a_seeded_root_id()
            )),
            "publish_vote" => publish_request(&format!(
                r#""target":"{}","direction":"up""#,
                a_seeded_root_id()
            )),
            other => panic!("no served request known for {other}"),
        }
    }

    fn error_message(out: &str) -> String {
        let v: serde_json::Value = serde_json::from_str(out)
            .unwrap_or_else(|e| panic!("reply must be valid JSON ({e}): {out}"));
        v.get("error")
            .unwrap_or_else(|| panic!("expected the error shape, got {out}"))
            .as_str()
            .unwrap_or_else(|| panic!("an error message must be a string, got {out}"))
            .to_string()
    }

    #[test]
    fn a_request_that_is_not_an_object_is_refused_for_its_shape() {
        // THE test this change exists for. Note what it does NOT assert:
        // `error.is_some()`, which is already true of `[]` on every method,
        // because a required field is absent from an array just as it is from
        // `{}`. What it asserts is the message, against the literal constant —
        // so it fails on the unfixed code with the missing-field message, and
        // it fails again if the constant is ever reworded without the spec
        // being revisited.
        for (name, method) in every_request_taking_method() {
            for not_an_object in [
                "[]",
                r#"[{"stoa":"00"}]"#,
                "7",
                r#""a string""#,
                "true",
                "null",
            ] {
                let out = method(not_an_object);
                assert_eq!(
                    error_message(&out),
                    REQUEST_NOT_AN_OBJECT,
                    "{name} must refuse {not_an_object} for its shape, got {out}"
                );
            }
        }
    }

    #[test]
    fn the_three_refusals_a_caller_can_earn_are_three_different_messages() {
        // The spec's crux: three caller mistakes, three messages. Asserting
        // only that they differ would be satisfied by any accident; each is
        // pinned to what it must SAY, so a reword that collapses two is caught.
        //
        // The third mistake is "an object omitting a required field", so this sweeps
        // the methods that HAVE one — see `every_method_with_a_required_field`. The
        // first two mistakes are swept over every method by
        // `a_request_that_is_not_an_object_is_refused_for_its_shape`.
        for (name, method) in every_method_with_a_required_field() {
            let not_an_object = error_message(&method("[]"));
            let unparseable = error_message(&method("not json at all"));
            let missing_field = error_message(&method("{}"));

            assert_eq!(not_an_object, REQUEST_NOT_AN_OBJECT, "for {name}");
            assert!(
                unparseable.starts_with("invalid JSON"),
                "{name}: an unparseable request must say so, got {unparseable:?}"
            );
            assert!(
                missing_field.contains("missing field"),
                "{name}: an object omitting a required field must name it, got \
                 {missing_field:?}"
            );

            // And the pairwise statement, so the requirement is asserted as
            // well as each message being pinned.
            assert_ne!(not_an_object, unparseable, "for {name}");
            assert_ne!(not_an_object, missing_field, "for {name}");
            assert_ne!(unparseable, missing_field, "for {name}");
        }
    }

    #[test]
    fn an_empty_object_is_refused_for_its_missing_field_and_never_for_its_shape() {
        // `{}` is an object, so the envelope check must pass it through to the
        // method's own field checks. A check written as "refuse anything that
        // is not a non-empty object" would break exactly here, and every other
        // test in this file would stay green.
        //
        // Over the methods that have a required field, because the assertion is that
        // `{}` earns the MISSING-FIELD refusal — a method with no required field has
        // none to miss, and `list_stoas` serves `{}`. The "never for its shape" half
        // still holds for it and is covered below.
        for (name, method) in every_method_with_a_required_field() {
            let message = error_message(&method("{}"));
            assert_ne!(
                message, REQUEST_NOT_AN_OBJECT,
                "{name} refused `{{}}` for its shape rather than its missing field"
            );
            assert!(message.contains("missing field"), "{name}: got {message:?}");
        }

        // And the half that holds for a method with NO required field: `{}` must be
        // SERVED, not refused for its shape. Without this, excluding `list_stoas`
        // from the loop above would have excluded it from the envelope claim
        // entirely — which is the failure mode the exclusion has to avoid, since a
        // `Request::parse` that refused every empty object would pass every sweep
        // that remains.
        let out = list_stoas("{}", &a_membership_store());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("error").is_none(),
            "list_stoas has no required field, so `{{}}` must be served: {out}"
        );
        assert_eq!(v["page"], 0, "an omitted page must default to 0: {out}");
    }

    #[test]
    fn a_non_object_refusal_carries_no_result_field() {
        // §2.5: never a partial success. A reply carrying both the refusal and
        // an empty `items` renders as an empty feed in any view that checks
        // `items` first.
        for (name, method) in every_request_taking_method() {
            for not_an_object in ["[]", "7", "null"] {
                let out = method(not_an_object);
                let v: serde_json::Value = serde_json::from_str(&out).unwrap();
                for result_field in ["items", "pong", "canPost", "identity", "channelId"] {
                    assert!(
                        v.get(result_field).is_none(),
                        "{name} carried both an error and {result_field} for \
                         {not_an_object}: {out}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_object_supplying_only_its_required_fields_is_served() {
        // The other half of the check: it must refuse a wrong TYPE and not an
        // absent optional field. Without this, a check that refused every
        // request lacking `page` would satisfy every test above.
        for (name, method) in every_request_taking_method() {
            let out = method(&a_served_request(name));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("error").is_none(),
                "{name} refused a request it must serve: {out}"
            );
        }
    }

    /// A served request with one extra key added, built through `serde_json`.
    ///
    /// The earlier spelling of this spliced text — `trim_end_matches('}')` then
    /// append — was fragile to how a neighbouring fixture happened to be
    /// written, and silently tested something else when it broke. Respelling
    /// `a_served_request("ping")` from `{"payload":1}` to the equally valid
    /// `{"payload":{"n":1}}` made `trim_end_matches` strip BOTH closing braces,
    /// and the test then failed with `invalid JSON: EOF while parsing an object`
    /// — reporting a refusal of the extra field that never happened. Parsing
    /// into a `Map` and inserting cannot produce malformed JSON at all, so the
    /// assertion is about the extra field and only about the extra field.
    fn with_extra_field(served: &str, key: &str, value: serde_json::Value) -> String {
        let mut map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(served)
            .unwrap_or_else(|e| panic!("a served fixture must be a JSON object ({e}): {served}"));
        assert!(
            map.insert(key.to_string(), value).is_none(),
            "{key} is already a field of {served}, so adding it tests nothing"
        );
        serde_json::to_string(&map).expect("a Map always serialises")
    }

    #[test]
    fn an_unrecognised_field_does_not_refuse_the_request() {
        // Out of scope by decision, not by omission — the proposal argues that
        // strictness is a compatibility policy and not an envelope rule. Pinned
        // so tightening it later is a deliberate act that breaks a test.
        for (name, method) in every_request_taking_method() {
            let served = a_served_request(name);
            let with_extra = with_extra_field(
                &served,
                "somethingNoMethodReads",
                serde_json::json!({"nested": [1, 2, 3]}),
            );
            // The fixture must still be the request it was, plus one key —
            // otherwise a broken construction is what the assertion below
            // reports. This is the check the spliced spelling could not make.
            let round_trip: serde_json::Value =
                serde_json::from_str(&with_extra).unwrap_or_else(|e| {
                    panic!("{name}: fixture is not valid JSON ({e}): {with_extra}")
                });
            let original: serde_json::Value = serde_json::from_str(&served).unwrap();
            for (field, want) in original.as_object().unwrap() {
                assert_eq!(
                    round_trip.get(field),
                    Some(want),
                    "{name}: adding a field altered {field}"
                );
            }

            let out = method(&with_extra);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("error").is_none(),
                "{name} refused an unrecognised field: {out}"
            );
        }
    }

    #[test]
    fn panic_probe_still_panics_on_a_non_object_rather_than_refusing_it() {
        // SPECIFIED, and the `NO SPEC:` marker that stood here is gone rather
        // than reworded around: the envelope rule is now scoped by "reads a field
        // of its request", and the probe's own requirement — "A panic in a
        // handler becomes the error shape and the module keeps serving" — gives
        // the probe a contract of its own. It treats its request as opaque text,
        // reaches its panic for every request shape including a non-object and
        // including text that is not JSON, refuses none, and carries the request
        // in its message.
        //
        // The exclusion is still worth a comment, because the reason it is not a
        // gap is not visible from the code: a probe that can refuse a request is
        // a probe there are requests the guard is not exercised against, so the
        // two rules cannot both reach it and the panic-guard one wins. That is
        // the spec's own words, not this test's reasoning.
        let out = panic_probe("[]");
        let message = error_message(&out);
        assert!(
            message.contains("panic in panic_probe"),
            "panic_probe must still reach its panic, got {message:?}"
        );
        assert_ne!(message, REQUEST_NOT_AN_OBJECT);
        // The probe's own requirement also obliges the request to reach the
        // message, which is "the observable difference between passing the text
        // through and decoding it". Asserted here because the exclusion and the
        // pass-through are one claim: a probe that decoded its request in order
        // to refuse it could not carry the raw text.
        assert!(
            message.contains("[]"),
            "the probe must carry the request it was given, got {message:?}"
        );
    }

    #[test]
    fn every_request_taking_method_refuses_an_oversized_request() {
        // NO SPEC: the spec set says nothing about a size limit on a request —
        // not that there is one, not that there is not. This is therefore an
        // ABSENT decision rather than a rejected one, and the number is
        // `dev-writer`'s choice pending the spec-writer: 4 MiB, derived in
        // `MAX_REQUEST_BYTES`'s doc from what a legitimate composed op can
        // carry.
        //
        // What made it necessary is measured rather than theorised: a 64 MiB
        // request padded with one ignored field was ACCEPTED and served, at
        // 92.6 ms and ~2N transient heap, for a 373-byte reply — an inverted
        // amplification nothing downstream can notice. `ping` echoes `payload`,
        // so 32 MiB in produced a 33,554,443-byte reply.
        //
        // Swept across every method rather than asserted once on
        // `Request::parse`, because the claim being made is about the SURFACE:
        // the envelope bounds every request-taking method, including one written
        // next month. If this test ever has to be edited to exempt a method,
        // that is the signal that the method reached around the type.
        let oversized = format!(r#"{{"junk":"{}"}}"#, "x".repeat(MAX_REQUEST_BYTES));
        assert!(oversized.len() > MAX_REQUEST_BYTES);
        for (name, method) in every_request_taking_method() {
            let message = error_message(&method(&oversized));
            assert!(
                message.contains("over the") && message.contains("byte limit"),
                "{name} must refuse an oversized request for its size, got {message:?}"
            );
        }
    }

    #[test]
    fn a_request_within_the_cap_is_still_served() {
        // The other half, and the one that stops a cap of zero from satisfying
        // the sweep above. Every method's own served fixture is far under the
        // cap, so this is really asserting that the check did not fire at all —
        // which is what `an_object_supplying_only_its_required_fields_is_served`
        // would also catch, except that a cap mistakenly written as
        // `request.len() < MAX_REQUEST_BYTES` (refusing everything SMALL) would
        // break both and only this one names the reason.
        for (name, method) in every_request_taking_method() {
            let served = a_served_request(name);
            assert!(
                served.len() < MAX_REQUEST_BYTES,
                "{name}'s fixture must be under the cap for this test to mean anything"
            );
            let out = method(&served);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("error").is_none(),
                "{name} refused a request well under the cap: {out}"
            );
        }
    }

    #[test]
    fn pings_payload_carries_an_explicit_null_through_as_a_value() {
        // Reading 1, and the surface's only instance of it: `payload` is
        // documented `<any>`, so a `null` is the value rather than a malformed
        // parameter — and that reading takes precedence over the required-field
        // one, which `payload` also satisfies.
        //
        // The assertion is that `pong` is PRESENT and holds `null`, which is a
        // different statement from `v["pong"].is_null()`: indexing a missing key
        // in `serde_json` yields `Value::Null` too, so the weaker spelling passes
        // against a reply that dropped the field entirely. That is the two-
        // explanations-one-answer shape this project keeps finding.
        let out = ping(r#"{"payload":null}"#);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("error").is_none(),
            "a `<any>` field's null is a value, not a refusal: {out}"
        );
        assert_eq!(
            v.as_object().and_then(|o| o.get("pong")),
            Some(&serde_json::Value::Null),
            "the null must be CARRIED, not dropped: {out}"
        );
        // And the whole reply, hardcoded, because the shape is the contract.
        assert_eq!(out, r#"{"pong":null}"#);

        // The two it must be told apart from. Omitting the field is the missing
        // one, not a null value — so reading 1 has not been implemented by
        // treating a null as absent.
        assert_eq!(error_message(&ping("{}")), "missing field: payload");
    }

    #[test]
    fn a_null_optional_field_takes_the_restrictive_default() {
        // Reading 2, and the assertion the contract actually names: not merely
        // "a null is accepted" but that the reply EQUALS the one omitting the
        // field gives, and that it is the restrictive reply.
        //
        // `includeHidden` is the worked example — a hidden thread must stay
        // hidden for `null`. Comparing against the omitted-field reply is what
        // makes this fail if `null` were ever read as `true`: both replies would
        // still parse, both would still be error-free, and only the comparison
        // sees the difference.
        let log = log_with_body("hello");
        let omitted = list_threads(&feed_request(""), &log, &feed_genesis());
        let null_valued = list_threads(
            &feed_request(r#""includeHidden":null"#),
            &log,
            &feed_genesis(),
        );
        assert_eq!(
            null_valued, omitted,
            "a null optional field must answer exactly as omitting it does"
        );
        // And it is the restrictive side of the flag, not just the same side.
        // `true` must differ from both, or the comparison above is satisfied by
        // a handler that ignores the flag altogether.
        let explicitly_true = list_threads(
            &feed_request(r#""includeHidden":true"#),
            &log,
            &feed_genesis(),
        );
        let hidden_log = log_with_a_hidden_thread();
        let with_hidden_excluded = list_threads(
            &feed_request(r#""includeHidden":null"#),
            &hidden_log,
            &feed_genesis(),
        );
        let with_hidden_included = list_threads(
            &feed_request(r#""includeHidden":true"#),
            &hidden_log,
            &feed_genesis(),
        );
        assert_ne!(
            with_hidden_excluded, with_hidden_included,
            "the flag must actually change the answer, or this test proves nothing \
             about which side a null lands on ({explicitly_true})"
        );
        let excluded: serde_json::Value = serde_json::from_str(&with_hidden_excluded).unwrap();
        let included: serde_json::Value = serde_json::from_str(&with_hidden_included).unwrap();
        assert!(
            excluded["items"].as_array().unwrap().len()
                < included["items"].as_array().unwrap().len(),
            "a null must land on the side that shows LESS: {with_hidden_excluded} \
             vs {with_hidden_included}"
        );

        // `page` and `perPage` are the same reading through `parse_index`, and
        // the same assertion: identical to omission.
        for field in [r#""page":null"#, r#""perPage":null"#] {
            assert_eq!(
                list_threads(&feed_request(field), &log, &feed_genesis()),
                omitted,
                "{field} must answer exactly as omitting it does"
            );
        }
    }

    #[test]
    fn a_null_required_field_is_refused_as_a_wrong_type_and_not_as_missing() {
        // Reading 3, and the distinction the contract makes explicit: the caller
        // DID name the field, so "missing" would describe a request it did not
        // make. Asserted on both halves — what the message says, and what it must
        // not say — because "an error came back" is true of both readings.
        let stoa = feed_genesis().address().unwrap().to_hex();
        for (request, field, method) in [
            (
                r#"{"stoa":null}"#.to_string(),
                "stoa",
                "get_capabilities" as &str,
            ),
            (
                format!(r#"{{"stoa":"{stoa}","genesis":null}}"#),
                "genesis",
                "list_threads_from_request",
            ),
            (
                r#"{"channelId":null}"#.to_string(),
                "channelId",
                "parse_channel_id",
            ),
        ] {
            let out = match method {
                "get_capabilities" => get_capabilities(&request, |_| Ok("abcd".to_string())),
                "list_threads_from_request" => list_threads_from_request(&request, || {
                    Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
                }),
                "parse_channel_id" => match parse_channel_id(&request) {
                    Ok(id) => serde_json::json!({ "channelId": id }).to_string(),
                    Err(e) => e,
                },
                other => panic!("unhandled method {other}"),
            };
            let message = error_message(&out);
            assert_eq!(
                message,
                format!("{field} must be a string"),
                "{method}: a null required field is a wrong type, got {message:?}"
            );
            assert!(
                !message.contains("missing"),
                "{method}: a named field must not be reported as missing, got {message:?}"
            );
        }
    }

    #[test]
    fn one_field_has_one_null_reading() {
        // The contract's "and never two for one field", asserted as the property
        // rather than field by field: every field the surface reads, supplied as
        // `null`, produces exactly ONE of the three outcomes.
        //
        // What this catches that the three tests above do not: a field acquiring
        // a second reading later. A future `Some(Value::Null)` arm added to
        // `ping` would leave all three of those tests green for `payload` if it
        // returned the same answer by a different route — but a field landing in
        // two buckets here is a count, and the count is what is asserted.
        let stoa = feed_genesis().address().unwrap().to_hex();
        let cases: Vec<NullReadingCase> = vec![
            (
                "payload",
                r#"{"payload":null}"#.to_string(),
                Box::new(|r: &str| ping(r)),
            ),
            (
                "stoa",
                r#"{"stoa":null}"#.to_string(),
                Box::new(|r: &str| get_capabilities(r, |_| Ok("abcd".to_string()))),
            ),
            (
                "genesis",
                format!(r#"{{"stoa":"{stoa}","genesis":null}}"#),
                Box::new(|r: &str| {
                    list_threads_from_request(r, || {
                        Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
                    })
                }),
            ),
            (
                "channelId",
                r#"{"channelId":null}"#.to_string(),
                Box::new(|r: &str| match parse_channel_id(r) {
                    Ok(id) => serde_json::json!({ "channelId": id }).to_string(),
                    Err(e) => e,
                }),
            ),
            (
                "page",
                feed_request(r#""page":null"#),
                Box::new(|r: &str| list_threads(r, &log_with_body("hello"), &feed_genesis())),
            ),
            (
                "perPage",
                feed_request(r#""perPage":null"#),
                Box::new(|r: &str| list_threads(r, &log_with_body("hello"), &feed_genesis())),
            ),
            (
                "includeHidden",
                feed_request(r#""includeHidden":null"#),
                Box::new(|r: &str| list_threads(r, &log_with_body("hello"), &feed_genesis())),
            ),
            // The publish path's own required fields, added with the handlers
            // themselves. All three are strings whose declared type does not
            // admit `null`, so all three are reading 3 — and the assertion that
            // matters is the one this sweep makes for every field: never
            // reported as missing, and exactly one outcome, so a field cannot
            // quietly acquire a second reading later.
            //
            // These PASSED on the unfixed code, which is worth saying: the
            // three handlers' null readings were already right. What was wrong
            // was the envelope around them, and this sweep is coverage of a
            // property rather than a regression test for a defect.
            (
                "body",
                publish_request(r#""body":null"#),
                Box::new(|r: &str| {
                    publish_post(
                        r,
                        &mut MemoryOpLog::new(),
                        &publish_key(),
                        &mut ignored_delivery,
                    )
                }),
            ),
            (
                "parent",
                publish_request(r#""parent":null,"body":"x""#),
                Box::new(|r: &str| {
                    publish_reply(
                        r,
                        &mut a_log_seeded_with_a_root(),
                        &publish_key(),
                        &mut ignored_delivery,
                    )
                }),
            ),
            (
                "direction",
                publish_request(&format!(
                    r#""target":"{}","direction":null"#,
                    a_seeded_root_id()
                )),
                Box::new(|r: &str| {
                    publish_vote(
                        r,
                        &mut a_log_seeded_with_a_root(),
                        &publish_key(),
                        &mut ignored_delivery,
                    )
                }),
            ),
        ];

        for (field, request, call) in cases {
            let out = call(&request);
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("{field}: reply must be JSON ({e}): {out}"));

            let refused_as_wrong_type = v
                .get("error")
                .and_then(|e| e.as_str())
                .is_some_and(|m| m.contains("must be"));
            let refused_as_missing = v
                .get("error")
                .and_then(|e| e.as_str())
                .is_some_and(|m| m.contains("missing"));
            let served = v.get("error").is_none();

            // A null must NEVER be reported as missing — that is the one outcome
            // the contract rules out for every field, whichever reading applies.
            assert!(
                !refused_as_missing,
                "{field}: a null was reported as missing: {out}"
            );
            let outcomes = [refused_as_wrong_type, served]
                .iter()
                .filter(|b| **b)
                .count();
            assert_eq!(
                outcomes, 1,
                "{field} must produce exactly one outcome, got {out}"
            );
        }
    }

    #[test]
    fn the_sixth_method_the_boundary_does_not_stop() {
        // WHAT THE MODULE BOUNDARY DOES NOT FIX, built rather than asserted,
        // because the honest scope of the fix is the thing most likely to be
        // overclaimed.
        //
        // The reviewer's sixth method — a handler parsing `Value` directly with
        // all-optional fields — was built and served `[]` as a request that named
        // nothing, with the whole suite green. The question put to this change was
        // whether the module boundary stops it.
        //
        // IT DOES NOT. Verified here: this compiles and serves `[]` with
        // `Request` moved out of reach. The boundary closes exactly one hole —
        // constructing a `Request` without going through `parse` — and it cannot
        // close this one, because nothing in the type system obliges a handler to
        // hold a `Request` at all. A handler that never mentions the type is
        // never constrained by it.
        //
        // So the guarantee is precisely: a handler that reads fields THROUGH
        // `Request` went through the envelope check. It is not "every handler is
        // checked", and design.md says so in those words.
        //
        // What remains against this is the sweep — `every_request_taking_method`,
        // whose doc now states the obligation — and review. Both are human, and
        // that is the residual.
        fn a_handler_that_never_holds_a_request(request: &str) -> String {
            let parsed: serde_json::Value = match serde_json::from_str(request) {
                Ok(v) => v,
                Err(e) => return error_json(&format!("invalid JSON: {e}")),
            };
            let page = parsed.get("page").and_then(|v| v.as_u64()).unwrap_or(0);
            serde_json::json!({ "items": [], "page": page, "hasMore": false }).to_string()
        }

        let served_an_array = a_handler_that_never_holds_a_request("[]");
        let v: serde_json::Value = serde_json::from_str(&served_an_array).unwrap();
        assert!(
            v.get("error").is_none() && v.get("items").is_some(),
            "if this ever FAILS, the compiler gained a way to force a handler \
             through the envelope and design.md's residual is stale — which is a \
             better outcome than this test passing: {served_an_array}"
        );

        // And the contrast, which is what the boundary did buy: the same handler
        // written through `Request` cannot do this, and needs no author to
        // remember why.
        fn the_same_handler_through_the_type(request: &str) -> String {
            let parsed = match Request::parse(request) {
                Ok(r) => r,
                Err(e) => return e,
            };
            let page = match parse_index(&parsed, "page") {
                Ok(v) => v.unwrap_or(0),
                Err(e) => return e,
            };
            serde_json::json!({ "items": [], "page": page, "hasMore": false }).to_string()
        }
        assert_eq!(
            error_message(&the_same_handler_through_the_type("[]")),
            REQUEST_NOT_AN_OBJECT,
            "the type is what makes the difference, and it is the only thing that does"
        );
    }

    #[test]
    fn the_index_refusal_says_what_it_actually_refuses() {
        // The message was factually wrong and is now factually narrow. `as_u64`
        // refuses by SPELLING, not by value: `1e2` and `0.0` are both exactly
        // whole non-negative numbers, and both are refused because serde parses
        // them as `f64`. A message saying "must be a non-negative whole number"
        // told such a caller its 100 was not a whole number.
        //
        // What is asserted here is the message for each of the four spellings
        // that earn it, against a hardcoded literal — because what let this
        // through is that nothing read the message beside the input that
        // produced it. A test asserting `error.is_some()` for `{"page":1e2}`
        // is satisfied by a message saying anything at all.
        //
        // An earlier version of this comment said the message "was wrong for
        // two years". It was wrong for hours: `git log -S` finds it introduced
        // in `0538c0d` and fixed in `134240b`, both 2026-09-12, and this
        // repository's first commit is five days older than that. The duration
        // was doing the persuading and it was invented — which is this
        // project's recorded "persuasive citations get fabricated" trap wearing
        // a number instead of a reference. The structural reason above is the
        // real one and needs no duration.
        let log = log_with_body("hello");
        let refused = "page must be a non-negative integer written without a \
                       decimal point or exponent";
        for form in [
            // The two the old message described correctly.
            r#""page":-1"#,
            r#""page":1.5"#,
            // The two it described wrongly: exactly 100, and exactly 0.
            r#""page":1e2"#,
            r#""page":0.0"#,
        ] {
            let out = list_threads(&feed_request(form), &log, &feed_genesis());
            assert_eq!(error_message(&out), refused, "for {form}, got {out}");
        }

        // And the acceptance is UNCHANGED — the fix is the message, not the set.
        // `100` is served, so this test cannot be satisfied by a function that
        // refuses every number.
        let served = list_threads(&feed_request(r#""page":100"#), &log, &feed_genesis());
        let v: serde_json::Value = serde_json::from_str(&served).unwrap();
        assert!(v.get("error").is_none(), "got {served}");
        assert_eq!(v["page"], 100);
    }

    #[test]
    fn the_two_feed_entry_points_agree_after_the_parse_moved() {
        // NAMED FOR WHAT IT ASSERTS, which is NOT what its old name
        // (`the_feed_path_parses_its_request_once`) claimed. Nothing below
        // counts parses, and nothing below could.
        //
        // The double parse was: `list_threads_from_request` parsed the request to
        // read `stoa` and `genesis`, then handed the raw `&str` to `list_threads`,
        // which parsed it again. Both replies were identical, so NO assertion on
        // output can see the difference — which is why it survived on main.
        //
        // THE SINGLE PARSE IS ENFORCED BY THE COMPILER, NOT BY THIS TEST.
        // `list_threads_inner` takes `&Request`, so there is no `&str` in scope
        // for a second parse to consume. Reverting that signature to `&str` is
        // what would let the bug back in, and it is a compile-visible change to a
        // private function that this test would stay green through. If you are
        // looking for the thing that guards the fix, it is the signature.
        //
        // So this test's job is the narrower one the refactor DID have to
        // preserve: that moving the parse and dropping a nested `guarded` frame
        // changed no reply. It is a refactor-safety net, and worth keeping as
        // one — it is what would have caught the move going wrong — but it
        // proves nothing about how many times anything is parsed.
        //
        // What the sweep below actually does, since the old comment said "three
        // request shapes each, compared against each other" and both halves were
        // wrong: FIVE shapes, and within the loop the two entry points are NOT
        // compared to each other. Each is only checked to be a JSON object with
        // no doubled guard frame — because for a malformed request the two are
        // not obliged to agree (only one of them consults the genesis). The
        // cross-entry-point equality is asserted once, after the loop, for the
        // well-formed request alone, which is the one case where they must.
        let log = log_with_body("hello");
        let genesis = feed_genesis();

        for request in [
            full_request(),
            "[]".to_string(),
            "not json".to_string(),
            "{}".to_string(),
            feed_request(r#""page":1"#),
        ] {
            let through_the_decoded_form = list_threads(&request, &log, &genesis);
            let through_the_hex_form = list_threads_from_request(&request, || {
                Ok::<_, crate::log::OpLogError>(log_with_body("hello"))
            });

            // Both must be JSON objects, and neither may be a doubled-up envelope
            // — a nested `guarded` that caught something would show as an error
            // naming `list_threads` twice.
            for (which, out) in [
                ("list_threads", &through_the_decoded_form),
                ("list_threads_from_request", &through_the_hex_form),
            ] {
                let v: serde_json::Value = serde_json::from_str(out)
                    .unwrap_or_else(|e| panic!("{which} for {request}: not JSON ({e}): {out}"));
                assert!(v.is_object(), "{which} for {request}: {out}");
                if let Some(message) = v.get("error").and_then(|e| e.as_str()) {
                    assert_eq!(
                        message.matches("panic in list_threads").count(),
                        0,
                        "{which} for {request}: a guard fired, so the frames are \
                         not equivalent: {out}"
                    );
                }
            }
        }

        // And the one case where the two entry points must agree exactly: a
        // well-formed request. They read the same fields from the same request, so
        // a divergence here means the parse that was removed was doing something.
        assert_eq!(
            list_threads(&full_request(), &log, &genesis),
            list_threads_from_request(&full_request(), || Ok::<_, crate::log::OpLogError>(
                log_with_body("hello")
            )),
            "the two entry points must serve the same request identically"
        );
    }

    #[test]
    fn the_bypass_this_module_boundary_closes() {
        // NOT A TEST OF BEHAVIOUR, and said so plainly: this is the honest half
        // of the guarantee, because what it asserts cannot be asserted at
        // runtime at all.
        //
        // The claim in `Request`'s doc is that a handler holding one went
        // through the check. While `Request` was defined IN THIS FILE that was
        // false, because a tuple struct's private field is private to its
        // defining MODULE and every handler lives here. Verified before the fix
        // by compiling, from this very `mod tests`:
        //
        //     let bypass = Request(serde_json::Map::new());
        //     let inner_read = bypass.0.len();     // compiled, ran, returned 0
        //
        // Neither line contains a `from_str`, so the mitigation originally
        // recorded — "a second parse is visible in review as an anomaly" — never
        // applied to it.
        //
        // After the move to `wire::request` the first line fails to compile
        // here. Verified, verbatim:
        //
        //     error[E0423]: cannot initialize a tuple struct which contains
        //                   private fields
        //       --> dialectica-core/src/wire.rs
        //       note: constructor is not visible here due to private fields
        //       --> dialectica-core/src/wire/request.rs
        //
        // That is a compile error, so it cannot be written as a `#[test]` in
        // this file. A `compile_fail` doctest would not prove this either: a
        // doctest compiles as an EXTERNAL consumer, so it would show the field
        // is private across crates — a weaker statement than the one at issue,
        // which is about `wire.rs` itself. Recording the verified error is
        // therefore the whole proof, and it is deliberately stated as such
        // rather than dressed up as a test that passes for a weaker reason.
        //
        // A side effect worth knowing: this crate's doctest run is empty, and
        // CI's count-the-tests gate relies on that — it counts `#[test]`
        // attributes in the source, which no doctest has. So a fenced block
        // marked `ignore` rather than `text` fails that gate. Twice now; see
        // `wire::request`'s module doc.
        //
        // What IS testable, and is: the positive half lives in
        // `wire::request::tests::request_is_constructible_here_because_this_module_defines_it`,
        // which compiles the same line inside the defining module. Together they
        // say the refusal above is about the boundary and not about a typo.
        //
        // And the runtime half of the guarantee — that the only constructor
        // reachable from here refuses a non-object — is
        // `request_parse_refuses_every_non_object_json_value`, below.
        //
        // What this does NOT buy:
        // `the_sixth_method_the_boundary_does_not_stop`, above, builds a handler
        // that never mentions `Request` and serves an array. The claim is about
        // handlers that hold a `Request`, not about every handler, and that is the
        // claim design.md now makes.
        let through_the_constructor = Request::parse(r#"{"payload":1}"#);
        assert!(
            through_the_constructor.is_ok(),
            "the only reachable way in must still work"
        );
    }

    #[test]
    fn request_parse_refuses_every_non_object_json_value() {
        // NAMED FOR WHAT THE BODY FALSIFIES, and it was not always. This test
        // was called `request_parse_is_the_only_way_to_reach_a_field_read`,
        // which asserted exclusivity that the body does not check and that is
        // FALSE in the sense the name implies —
        // `the_sixth_method_the_boundary_does_not_stop`, in this same file,
        // reaches a field read without `Request::parse` at all. A name that
        // contradicts a neighbouring passing test is worse than a vague one.
        //
        // What is true and is asserted: a `Request` cannot be built from a
        // non-object, so a handler holding one cannot have skipped the check.
        // That is what makes the guard inherited by a method nobody has written
        // yet rather than something each author must remember — for handlers
        // that hold a `Request`. See `wire::request`'s module doc for the
        // boundary's exact scope.
        // `err_of` rather than `unwrap_err`, which would require `Debug` on
        // `Request` — widening the library's surface for a test's convenience,
        // the same trade this file already declines for `KeystoreError: Clone`.
        fn err_of(r: Result<Request, String>) -> Option<String> {
            r.err()
        }
        // Every non-object variant `serde_json::Value` has, not just the array:
        // the refusal is one `_` arm, so an implementation that enumerated the
        // variants and forgot one would be caught here rather than only through
        // whichever handler happened to be swept.
        for not_an_object in [
            "[]", "[1,2]", "7", "-1", "1.5", r#""s""#, "true", "false", "null",
        ] {
            assert_eq!(
                err_of(Request::parse(not_an_object)),
                Some(error_json(REQUEST_NOT_AN_OBJECT)),
                "Request::parse accepted {not_an_object}"
            );
        }
        assert!(err_of(Request::parse("{}")).is_none());
        let unparseable = err_of(Request::parse("not json")).expect("must be refused");
        assert!(unparseable.contains("invalid JSON"), "got {unparseable}");
        // And the two failures are not the same failure.
        assert_ne!(unparseable, error_json(REQUEST_NOT_AN_OBJECT));
    }

    #[test]
    fn a_parsed_request_hands_back_the_fields_it_was_given_and_only_those() {
        // The OTHER way "read as an object in which every field is absent" can
        // come back, and the one no handler test can see: `parse` accepting an
        // object and then handing on an EMPTY map. Every envelope test above
        // asserts on refusals, and `an_object_supplying_only_its_required_fields_is_served`
        // asserts only that no error came back — so a `parse` that discarded the
        // map would be caught by the feed's own content tests, but nothing would
        // say the envelope was where it went wrong.
        //
        // The expected values are literals written here, not values read back
        // out of the parse and compared with themselves.
        let parsed = match Request::parse(r#"{"s":"x","n":7,"b":true,"z":null,"o":{"k":[1]}}"#) {
            Ok(r) => r,
            Err(e) => panic!("an object must parse: {e}"),
        };
        assert_eq!(parsed.get("s"), Some(&serde_json::json!("x")));
        assert_eq!(parsed.get("n"), Some(&serde_json::json!(7)));
        assert_eq!(parsed.get("b"), Some(&serde_json::json!(true)));
        // An explicit `null` is PRESENT, and that is now the contract's own words
        // rather than this test's inference: "A field holding an explicit `null`
        // is present, not absent", with three readings keyed to the field's
        // declared type and optionality. The envelope must therefore preserve the
        // distinction and decide none of it.
        //
        // WHICH READERS ACTUALLY OBSERVE IT, corrected — an earlier version of
        // this comment named `parse_index` and `includeHidden` as the two that
        // "both distinguish them", and that is exactly backwards. Those two are
        // the only readers that DON'T: reading 2 collapses a null into absent on
        // purpose. Four of the seven production field reads in this file do
        // distinguish, measured on both sides of a `get` mutated to drop nulls
        // (`.filter(|v| !v.is_null())`):
        //
        //   {"payload":null}   {"pong":null}                -> missing field: payload
        //   {"channelId":null} channelId must be a string    -> missing field: channelId
        //   {"stoa":null}      stoa must be a string         -> missing field: stoa
        //   {"genesis":null}   genesis must be a string      -> missing field: genesis
        //
        // `ping` flips from SUCCESS to error, which is a behaviour change and not
        // a reworded message; the other three collapse the wrong-type-against-
        // missing distinction this very contract requires. So the mutation
        // surviving 486 of 487 tests was a gap in the handler sweeps, never
        // evidence that nothing observes the difference — the inference this
        // comment used to draw.
        //
        // All four are now pinned at handler level, one fixture each, in
        // `pings_payload_carries_an_explicit_null_through_as_a_value` and
        // `a_null_required_field_is_refused_as_a_wrong_type_and_not_as_missing`,
        // with the collapsing pair in
        // `a_null_optional_field_takes_the_restrictive_default`. Four independent
        // kills rather than one test's word.
        assert_eq!(parsed.get("z"), Some(&serde_json::json!(null)));
        assert_eq!(parsed.get("o"), Some(&serde_json::json!({"k": [1]})));

        // And `None` means exactly one thing: this object has no such key. That
        // is the ambiguity the type exists to remove, so it is asserted rather
        // than assumed.
        assert_eq!(parsed.get("neverSupplied"), None);
        assert_eq!(Request::parse("{}").ok().unwrap().get("s"), None);
    }

    #[test]
    fn a_handler_whose_fields_are_all_optional_refuses_a_non_object() {
        // THE reachable form of the defect, which no method on today's surface
        // exhibits — every one requires `stoa`, `payload` or `channelId`, so
        // every one refuses an array as a side effect of that field being
        // absent from it. That side effect is not this rule and does not
        // survive the field becoming optional, which is exactly what the spec
        // says.
        //
        // So the case is built here: a handler shaped like the ones the
        // parallel branches are adding, whose fields are ALL optional. Written
        // against `Request::parse` — the same constructor every real handler
        // uses — so it demonstrates the property the type provides rather than
        // a property of a test double.
        fn all_fields_optional(request: &str) -> String {
            let parsed = match Request::parse(request) {
                Ok(r) => r,
                Err(e) => return e,
            };
            // Every field defaulted. Under the unfixed code this body served
            // `[]`, `7` and `null` as "a request that named nothing" — a
            // successful reply to a request the caller never made.
            let page = match parse_index(&parsed, "page") {
                Ok(v) => v.unwrap_or(0),
                Err(e) => return e,
            };
            serde_json::json!({ "items": [], "page": page, "hasMore": false }).to_string()
        }

        // The served cases first, so the refusals below are attributable to the
        // envelope and not to this double refusing everything.
        for served in ["{}", r#"{"page":3}"#, r#"{"unknown":true}"#] {
            let out = all_fields_optional(served);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("error").is_none(),
                "a request with no required field must be served: {served} -> {out}"
            );
        }

        // And now the thing that was silently served.
        for not_an_object in ["[]", "7", r#""s""#, "true", "null"] {
            let out = all_fields_optional(not_an_object);
            assert_eq!(
                error_message(&out),
                REQUEST_NOT_AN_OBJECT,
                "for {not_an_object}, got {out}"
            );
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(
                v.get("items").is_none(),
                "a refused request must not also carry a page of results: {out}"
            );
        }
    }

    #[test]
    fn the_non_object_message_does_not_read_as_either_refusal_it_must_be_told_from() {
        // Why this exists BESIDE the pin below, and is not the same test.
        //
        // `the_three_refusals_a_caller_can_earn_are_three_different_messages`
        // compares whole strings with `assert_ne!`, and that is not the
        // requirement. The spec says a caller must not be "told its array
        // failed to parse" — and a message reading
        // `"invalid JSON: the request must be a JSON object"` tells it exactly
        // that while comparing unequal to the parse failure's own text. Checked
        // by mutation: reworded to that, and to
        // `"missing field: the request must be a JSON object"`, the three-refusals
        // test stayed GREEN both times. Only the literal pin went red — and a
        // pin fails for "the string changed", which is not the reason this
        // requirement names.
        //
        // So the property is asserted directly: the non-object message must not
        // BEGIN with the phrase either neighbour opens on. The two prefixes are
        // written out here rather than read from the code, because reading them
        // from the code is how a reword makes both sides agree and the check
        // evaporate.
        for neighbour in ["invalid JSON", "missing field"] {
            assert!(
                !REQUEST_NOT_AN_OBJECT.starts_with(neighbour),
                "the non-object refusal opens on {neighbour:?}, which is how a \
                 caller reads a different mistake: {REQUEST_NOT_AN_OBJECT:?}"
            );
        }

        // And the two prefixes are the right ones to have written down: each is
        // what the neighbouring refusal actually says. Without this the test
        // above could be guarding against phrases no message uses.
        assert!(
            error_message(&ping("not json")).starts_with("invalid JSON"),
            "the unparseable refusal no longer opens on \"invalid JSON\", so the \
             prefix this test guards against is the wrong one"
        );
        assert!(
            error_message(&ping("{}")).starts_with("missing field"),
            "the missing-field refusal no longer opens on \"missing field\", so \
             the prefix this test guards against is the wrong one"
        );
    }

    #[test]
    fn the_non_object_message_is_pinned_to_a_known_answer() {
        // Hardcoded, because a test that reads the constant and compares it to
        // itself is the defect family this project has recorded three times. A
        // view may render this string; changing it is a contract change.
        assert_eq!(REQUEST_NOT_AN_OBJECT, "the request must be a JSON object");
    }

    #[test]
    fn every_handler_answers_with_a_json_object_for_any_request_shape() {
        // NAMED FOR WHAT IS ASSERTED. The old name claimed the reply carries
        // "exactly one top-level shape", which nothing here checks: the body
        // asserts `is_object()`, and asserting "exactly one shape" would mean
        // asserting the key set — which these sweeps deliberately do not, since
        // the success shapes differ per method (`pong`, `channelId`,
        // `items`/`page`/`hasMore`).
        //
        // The wire contract is only useful if it holds for EVERY method, so
        // check the property rather than each method's happy path again.
        let mut publish_log = MemoryOpLog::new();
        let key = publish_key();
        for out in [
            version("1.0.0"),
            ping(r#"{"payload":1}"#),
            ping("garbage"),
            panic_probe("{}"),
            get_capabilities(&format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()), |_| {
                Ok("abcd".to_string())
            }),
            get_capabilities("garbage", |_| Ok("abcd".to_string())),
            list_threads(&feed_request(""), &log_with_body("hello"), &feed_genesis()),
            list_threads("garbage", &log_with_body("hello"), &feed_genesis()),
            create_stoa(
                r#"{"title":"Agora"}"#,
                || Ok(feed_key(1).public_key()),
                &mut a_membership_store(),
            ),
            create_stoa(
                "garbage",
                || Ok(feed_key(1).public_key()),
                &mut a_membership_store(),
            ),
            join_stoa("garbage", &mut a_membership_store()),
            list_stoas("{}", &a_membership_store()),
            list_stoas("garbage", &a_membership_store()),
            publish_post(
                &publish_request(r#""body":"x""#),
                &mut publish_log,
                &key,
                &mut ignored_delivery,
            ),
            publish_post("garbage", &mut publish_log, &key, &mut ignored_delivery),
            publish_reply("garbage", &mut publish_log, &key, &mut ignored_delivery),
            publish_vote("garbage", &mut publish_log, &key, &mut ignored_delivery),
        ] {
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("handler emitted invalid JSON ({e}): {out}"));
            assert!(v.is_object(), "every reply is a JSON object, got {out}");
        }

        // The reply half must hold for a REFUSED request too, and the scenario
        // says so: "with a well-formed or a malformed request". A refusal built
        // by hand rather than through `error_json` is the way this breaks — an
        // array in, an array out.
        for (name, method) in every_request_taking_method() {
            for request in ["[]", "7", "null", "garbage", "{}"] {
                let out = method(request);
                let v: serde_json::Value = serde_json::from_str(&out).unwrap_or_else(|e| {
                    panic!("{name} emitted invalid JSON for {request} ({e}): {out}")
                });
                assert!(
                    v.is_object(),
                    "{name} answered {request} with a non-object: {out}"
                );
            }
        }
    }

    // ─── Creating, joining and listing Stoas ──────────────────────────────

    use crate::membership::MembershipStore;

    /// A membership store with nothing in it.
    ///
    /// In-memory rather than a file, for the reason `MembershipStore::in_memory`'s
    /// own documentation gives: it is the same code and the same SQL, and a test
    /// that is not *about* persistence should not need a temporary directory. The
    /// persistence tests in `membership.rs` use a real file.
    fn a_membership_store() -> MembershipStore {
        MembershipStore::in_memory().expect("an in-memory membership store is creatable")
    }

    /// The key a creation is performed under, in these tests.
    fn creator_key() -> crate::identity::PublicKey {
        feed_key(1).public_key()
    }

    /// A creation that succeeds in finding a key.
    fn create(store: &mut MembershipStore, title: &str) -> serde_json::Value {
        let request = serde_json::json!({ "title": title }).to_string();
        let out = create_stoa(&request, || Ok(creator_key()), store);
        serde_json::from_str(&out)
            .unwrap_or_else(|e| panic!("create_stoa emitted invalid JSON ({e}): {out}"))
    }

    /// Every Stoa a store lists, paged through at `per_page`.
    fn listed(store: &MembershipStore, per_page: usize) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let mut page = 0;
        loop {
            let request = serde_json::json!({ "page": page, "perPage": per_page }).to_string();
            let reply = list_stoas(&request, store);
            let v: serde_json::Value = serde_json::from_str(&reply)
                .unwrap_or_else(|e| panic!("list_stoas emitted invalid JSON ({e}): {reply}"));
            let items = v["items"]
                .as_array()
                .unwrap_or_else(|| panic!("a listing must carry items: {reply}"));
            out.extend(items.iter().cloned());
            if v["hasMore"] != true {
                return out;
            }
            page += 1;
            assert!(page < 1000, "paging did not terminate");
        }
    }

    #[test]
    fn creation_returns_the_address_of_the_record_it_built() {
        // The address must be RETURNED: a creation reporting only success leaves
        // the caller unable to name, share or read what it just made.
        //
        // The expected address is derived INDEPENDENTLY here — a record this test
        // builds from the same title and the same key — rather than read back out
        // of the reply and agreed with. That is what makes this fail if the handler
        // returned some other Stoa's address, or the hash of something else.
        let mut store = a_membership_store();
        let reply = create(&mut store, "Agora");

        let expected = crate::stoa::Genesis {
            creator: creator_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
        .address()
        .unwrap();
        assert_eq!(
            reply["stoa"].as_str().unwrap(),
            expected.to_hex(),
            "the reply must name the address the record just built verifies against"
        );

        // And the returned address is one the retained record verifies against,
        // read back out of the store rather than recomputed from the reply.
        let held = store.get(&expected).unwrap().expect("the Stoa is retained");
        assert!(held.genesis.matches(&expected));
    }

    #[test]
    fn the_creator_key_is_whatever_the_lookup_supplies_and_this_handler_chooses_none() {
        // WHICH key the adapter supplies is settled — `Keystore::identity_key`,
        // the root used directly, the same key the capability probe reports — and
        // `keystore.rs` pins that pair with
        // `the_creator_of_a_stoa_this_keystore_made_can_moderate_it`. It is not
        // this handler's decision to make: `core` cannot read the environment or
        // know the host's layout, so the key arrives through a closure exactly as
        // the probe's identity does.
        //
        // What this test pins is the part this handler IS responsible for: the
        // record's creator is the lookup's key exactly, unmodified — no
        // re-derivation, no substitution, no fallback. Two different lookups
        // produce two different Stoas from one title, which is what shows the key
        // reaches the record rather than a constant doing so.
        let mut one = a_membership_store();
        let mut two = a_membership_store();
        let a = create_stoa(
            r#"{"title":"Agora"}"#,
            || Ok(feed_key(1).public_key()),
            &mut one,
        );
        let b = create_stoa(
            r#"{"title":"Agora"}"#,
            || Ok(feed_key(2).public_key()),
            &mut two,
        );
        let va: serde_json::Value = serde_json::from_str(&a).unwrap();
        let vb: serde_json::Value = serde_json::from_str(&b).unwrap();
        assert_ne!(
            va["stoa"], vb["stoa"],
            "the creator key must reach the address, so two keys are two Stoas"
        );

        // And the stored creator is byte-identical to what the lookup returned.
        let address = crate::identity::Address::from_hex(va["stoa"].as_str().unwrap()).unwrap();
        assert_eq!(
            one.get(&address).unwrap().unwrap().genesis.creator.to_hex(),
            feed_key(1).public_key().to_hex()
        );
    }

    #[test]
    fn the_creator_is_the_callers_own_key_and_no_creator_is_accepted_from_the_request() {
        // A call that accepted a creator key would be a call that can be asked to
        // create a Stoa moderated by somebody else — a Stoa the caller cannot
        // moderate and whose address cannot be un-minted.
        //
        // Shown two ways, because the first alone is weak: the retained record's
        // creator IS the lookup's key, and a request OFFERING a different creator
        // is ignored rather than honoured.
        let mut store = a_membership_store();
        let request = serde_json::json!({
            "title": "Agora",
            "creator": feed_key(9).public_key().to_hex(),
        })
        .to_string();
        let out = create_stoa(&request, || Ok(creator_key()), &mut store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        let address = crate::identity::Address::from_hex(v["stoa"].as_str().unwrap()).unwrap();
        let held = store.get(&address).unwrap().unwrap();
        assert_eq!(
            held.genesis.creator.to_hex(),
            creator_key().to_hex(),
            "the creator must be the key the caller would sign with"
        );
        assert_ne!(
            held.genesis.creator.to_hex(),
            feed_key(9).public_key().to_hex(),
            "a creator offered in the request must not reach the record"
        );

        // The offered creator's Stoa is a DIFFERENT Stoa, and the peer is not in
        // it — which is what makes the assertion above about more than one field.
        let attackers = crate::stoa::Genesis {
            creator: feed_key(9).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
        .address()
        .unwrap();
        assert!(!store.contains(&attackers).unwrap());
    }

    #[test]
    fn a_created_stoa_is_listed_immediately_with_no_op_having_arrived() {
        // The op-log boundary at the wire, in the direction that matters most:
        // a freshly created Stoa has no ops BY CONSTRUCTION, so an answer derived
        // from the log would omit precisely the Stoa the user just made.
        //
        // There is no op log in this test at all, which is the point — the
        // listing's material is membership and nothing else.
        let mut store = a_membership_store();
        let reply = create(&mut store, "Agora");
        let address = reply["stoa"].as_str().unwrap().to_string();

        let items = listed(&store, 20);
        assert_eq!(items.len(), 1, "the created Stoa must be listed");
        assert_eq!(items[0]["stoa"].as_str().unwrap(), address);
    }

    #[test]
    fn creation_without_a_usable_key_fails_and_records_nothing() {
        // A Stoa created under a key the user does not hold is a Stoa nobody can
        // moderate and whose address cannot be un-minted. So creation refuses, and
        // MUST NOT proceed by generating a key for the occasion.
        //
        // Every keystore state, because the reason has to survive each one — and
        // because a handler that special-cased `NotFound` and mishandled `Locked`
        // would pass a single-variant test.
        let makers: [fn() -> crate::keystore::KeystoreError; 4] = [
            || KeystoreError::NotFound,
            || KeystoreError::Locked,
            || KeystoreError::WrongPassphrase,
            || KeystoreError::NotAKeystore,
        ];
        for make in makers {
            let mut store = a_membership_store();
            let out = create_stoa(r#"{"title":"Agora"}"#, || Err(make()), &mut store);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "got {out}");
            assert!(
                v.get("stoa").is_none(),
                "a failure must never also carry a result — §2.5, got {out}"
            );
            // The keystore's own message reaches the caller, rather than a
            // paraphrase this handler would have to keep in step with it.
            assert_eq!(v["error"].as_str().unwrap(), make().to_string());
            // And nothing was recorded. This is the half that would still pass if
            // the handler had minted a key: no Stoa exists for the peer to be in.
            assert_eq!(
                store.len().unwrap(),
                0,
                "a failed creation must leave the peer in no new Stoa"
            );
        }
    }

    #[test]
    fn an_over_long_title_creates_nothing() {
        // The refusal happens BEFORE any membership is recorded, which is the
        // ordering this requirement adds on top of `stoa-genesis`'s bound.
        let mut store = a_membership_store();
        let request = serde_json::json!({ "title": "x".repeat(1025) }).to_string();
        let out = create_stoa(&request, || Ok(creator_key()), &mut store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(v.get("stoa").is_none());
        assert_eq!(
            store.len().unwrap(),
            0,
            "the peer must be in no new Stoa after a refused creation"
        );
    }

    #[test]
    fn a_title_at_the_maximum_length_creates_a_stoa() {
        // The boundary's other half. Without this, a fencepost error in either
        // direction is invisible — both still "refuse something long" and every
        // other test passes.
        //
        // 1024 is HARDCODED rather than read from `stoa::MAX_TITLE_BYTES` (which is
        // private anyway): the bound is network-visible, and a test recomputing it
        // from the constant survives a change to it.
        let mut store = a_membership_store();
        let title = "x".repeat(1024);
        let reply = create(&mut store, &title);
        assert!(
            reply.get("error").is_none(),
            "a title of exactly the maximum must create a Stoa, got {reply}"
        );
        let address = crate::identity::Address::from_hex(reply["stoa"].as_str().unwrap()).unwrap();
        assert!(store.contains(&address).unwrap());
        assert_eq!(reply[FOUNDING_TITLE].as_str().unwrap().len(), 1024);
    }

    #[test]
    fn an_empty_title_is_accepted_rather_than_refused() {
        // The record has no minimum length, the title is not an identifier, and
        // refusing one here would make a record other peers decode and verify
        // without complaint unreachable through this surface.
        let mut store = a_membership_store();
        let reply = create(&mut store, "");
        assert!(reply.get("error").is_none(), "got {reply}");
        assert_eq!(
            reply[FOUNDING_TITLE], "",
            "the founding title must be reported as the empty string it is"
        );
        let address = crate::identity::Address::from_hex(reply["stoa"].as_str().unwrap()).unwrap();
        assert!(store.contains(&address).unwrap());
    }

    #[test]
    fn a_title_carrying_control_or_bidirectional_characters_is_not_rejected_for_that_reason() {
        // Not rejected, and not altered. The record is hashed to produce the
        // address, so normalising a title would change the address and split one
        // Stoa into two that cannot see each other — the same position `op.rs`
        // takes about display text. Rendering it safely is the view's obligation,
        // which `docs/UI-BRIEF.md` carries.
        let mut store = a_membership_store();
        let nasty = "Agora\u{202E}\u{200B}\u{202D}";
        let reply = create(&mut store, nasty);
        assert!(reply.get("error").is_none(), "got {reply}");
        assert_eq!(
            reply[FOUNDING_TITLE].as_str().unwrap(),
            nasty,
            "the founding title must carry those characters unchanged"
        );

        // And through a listing too, which is the path a view actually reads from.
        let items = listed(&store, 20);
        assert_eq!(items[0][FOUNDING_TITLE].as_str().unwrap(), nasty);
    }

    #[test]
    fn the_same_creator_and_title_reach_the_same_stoa() {
        // The intuitive expectation is the opposite one, which is why this is
        // pinned. A genesis record carries no nonce and no timestamp, so the same
        // creator making a record with the same title makes the SAME record — and
        // promising uniqueness the encoding cannot provide would be contradicting
        // `stoa-genesis`'s "the record carries no per-peer state".
        let mut store = a_membership_store();
        let first = create(&mut store, "Agora");
        let second = create(&mut store, "Agora");

        assert_eq!(
            first["stoa"], second["stoa"],
            "the same creator and title must reach the same address"
        );
        assert_eq!(
            store.len().unwrap(),
            1,
            "the peer must be in exactly one Stoa for that address"
        );
        assert_eq!(listed(&store, 20).len(), 1);
    }

    #[test]
    fn two_titles_are_two_stoas() {
        // The other side of the requirement above, and the answer a user who wants
        // two Stoas needs: give them two titles.
        let mut store = a_membership_store();
        let one = create(&mut store, "Agora");
        let two = create(&mut store, "Lyceum");
        assert_ne!(one["stoa"], two["stoa"], "two titles must be two addresses");
        assert_eq!(store.len().unwrap(), 2, "the peer must be in both");
        assert_eq!(listed(&store, 1).len(), 2);
    }

    /// A record and its address, as a joinable request.
    fn join_request(genesis: &crate::stoa::Genesis, claim: &crate::identity::Address) -> String {
        serde_json::json!({
            "stoa": claim.to_hex(),
            "genesis": hex::encode(genesis.canonical_bytes().unwrap()),
        })
        .to_string()
    }

    fn a_joinable_record(title: &str) -> crate::stoa::Genesis {
        crate::stoa::Genesis {
            creator: feed_key(5).public_key(),
            policy: Policy::Open,
            title: title.to_string(),
        }
    }

    #[test]
    fn a_matching_record_joins_and_the_reply_carries_the_address_and_founding_title() {
        let mut store = a_membership_store();
        let g = a_joinable_record("Somebody else's Stoa");
        let address = g.address().unwrap();

        let out = join_stoa(&join_request(&g, &address), &mut store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "got {out}");
        // Both fields, against literals — the reply is what lets a view show what
        // was joined, and a reply carrying only a title has shown the reader the
        // forgeable half.
        assert_eq!(v["stoa"].as_str().unwrap(), address.to_hex());
        assert_eq!(v[FOUNDING_TITLE].as_str().unwrap(), "Somebody else's Stoa");

        // And the Stoa is among the ones the peer is in.
        let items = listed(&store, 20);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["stoa"].as_str().unwrap(), address.to_hex());
    }

    #[test]
    fn a_record_that_does_not_match_the_address_is_refused_and_joins_neither_stoa() {
        // The self-authenticating check, at the wire. Every field in turn, because
        // a check comparing only one would pass a substitution in the other.
        let real = a_joinable_record("Agora");
        let address = real.address().unwrap();

        let other_creator = crate::stoa::Genesis {
            creator: feed_key(9).public_key(),
            ..real.clone()
        };
        let other_title = crate::stoa::Genesis {
            title: "Not Agora".to_string(),
            ..real.clone()
        };

        for impostor in [other_creator, other_title] {
            let mut store = a_membership_store();
            let out = join_stoa(&join_request(&impostor, &address), &mut store);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "got {out}");
            assert!(
                v.get("stoa").is_none(),
                "a failure must never also carry a result — §2.5"
            );
            // Not in the Stoa asked for...
            assert!(!store.contains(&address).unwrap());
            // ...and not in the one the SUPPLIED RECORD would name either, which is
            // the half a handler that "helpfully" joined what it was handed would
            // fail.
            assert!(!store.contains(&impostor.address().unwrap()).unwrap());
            assert_eq!(store.len().unwrap(), 0);
        }
    }

    #[test]
    fn a_malformed_record_is_refused_without_a_membership() {
        // Bytes the genesis encoding refuses to decode, as distinct from bytes that
        // decode and describe another Stoa. Both are refusals and they are
        // different mistakes.
        // Each case is a DIFFERENT decoder refusal, and the messages are asserted
        // to differ. Without that, four cases that all died as "truncated" would
        // look like coverage of four paths while exercising one — the fixture trap
        // this project keeps paying for.
        let mut store = a_membership_store();
        let address = a_joinable_record("Agora").address().unwrap();
        let mut messages = Vec::new();
        for (bad, why) in [
            // Nothing at all: the version byte is already missing.
            ("".to_string(), "ended mid-field"),
            // A version byte and then nothing: truncated at the creator key.
            ("01".to_string(), "ended mid-field"),
            // A version this build does not know — "newer client", not "corrupt".
            ("ff00".to_string(), "unknown genesis record version"),
            // Right shape, and an all-zero creator: a low-order point that
            // decompresses and can never verify a signature. The dangerous case,
            // and the one a decoder checking only well-formedness would accept.
            (
                format!("01{}00{}", "00".repeat(32), "00000000"),
                "creator key",
            ),
            // Right shape, a valid creator, and trailing bytes after a complete
            // record — refused because accepting them would let two byte strings
            // decode to one record while hashing to different addresses.
            (
                format!(
                    "{}ff",
                    hex::encode(a_joinable_record("Agora").canonical_bytes().unwrap())
                ),
                "trailing bytes",
            ),
        ] {
            let request =
                serde_json::json!({ "stoa": address.to_hex(), "genesis": bad }).to_string();
            let out = join_stoa(&request, &mut store);
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("error").is_some(), "for {bad:?}, got {out}");
            assert!(v.get("stoa").is_none());
            let message = v["error"].as_str().unwrap().to_string();
            assert!(
                message.contains(why),
                "for {bad:?} the refusal must say {why:?}, got {message}"
            );
            messages.push(message);
        }
        // Three distinct refusals across five cases (two are both truncations),
        // which is what proves these are not all one path.
        messages.sort();
        messages.dedup();
        assert_eq!(
            messages.len(),
            4,
            "the cases must exercise distinguishable decoder refusals, got {messages:?}"
        );
        assert_eq!(store.len().unwrap(), 0, "the peer must be in no new Stoa");
    }

    #[test]
    fn a_repeated_join_succeeds_and_leaves_one_membership_unchanged() {
        // A pasted address is exactly the input a user supplies twice. Reporting
        // the second attempt as an error would make a harmless action look broken.
        let mut store = a_membership_store();
        let g = a_joinable_record("Agora");
        let address = g.address().unwrap();
        let request = join_request(&g, &address);

        let first = join_stoa(&request, &mut store);
        let second = join_stoa(&request, &mut store);
        let v: serde_json::Value = serde_json::from_str(&second).unwrap();
        assert!(
            v.get("error").is_none(),
            "a repeated join must succeed: {second}"
        );
        assert_eq!(
            first, second,
            "the two replies must agree — a view has no 'already joined' branch"
        );
        assert_eq!(store.len().unwrap(), 1);
        assert_eq!(listed(&store, 20).len(), 1);
        // The founding values reported afterwards are unchanged.
        assert_eq!(
            listed(&store, 20)[0][FOUNDING_TITLE].as_str().unwrap(),
            "Agora"
        );
    }

    #[test]
    fn creating_and_then_joining_the_same_stoa_is_one_membership() {
        // Two different calls reaching one write path. This is the property that
        // would break first if creation grew a write of its own.
        let mut store = a_membership_store();
        let created = create(&mut store, "Agora");
        let address =
            crate::identity::Address::from_hex(created["stoa"].as_str().unwrap()).unwrap();
        let record = store.get(&address).unwrap().unwrap().genesis;

        let out = join_stoa(&join_request(&record, &address), &mut store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "got {out}");
        assert_eq!(
            store.len().unwrap(),
            1,
            "the peer must be in exactly one Stoa for that address"
        );
    }

    #[test]
    fn the_listing_envelope_is_the_ecosystems_pagination_shape() {
        // The precedent-setting shape, pinned by exact key NAME. A view is written
        // against these names and renaming one is a breaking change no type
        // checker would catch.
        let mut store = a_membership_store();
        create(&mut store, "Agora");

        let out = list_stoas(r#"{"page":0,"perPage":20}"#, &store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["items"].is_array(), "got {out}");
        assert_eq!(v["page"], 0);
        assert_eq!(v["hasMore"], false);
        // Each item carries the address, not only the title — the address is the
        // identity and the title is decoration.
        let row = &v["items"][0];
        assert!(
            row["stoa"].is_string(),
            "an item must carry its address: {out}"
        );
        assert!(
            row[FOUNDING_TITLE].is_string(),
            "an item must carry its founding title: {out}"
        );
    }

    #[test]
    fn a_listed_title_is_named_as_founding_and_never_as_a_bare_title() {
        // The requirement is that the reply make a founding title distinguishable
        // from a current one resolved from a metadata op. The FIELD NAME is how
        // this implementation does it, so the absence of a bare `title` key is as
        // load-bearing as the presence of `foundingTitle` — a reply carrying both
        // would put a view one forgotten branch from rendering the wrong one.
        //
        // Hardcoded key names on both sides: this is the assertion that fails if
        // someone "tidies" the field back to `title`.
        let mut store = a_membership_store();
        create(&mut store, "Agora");
        let g = a_joinable_record("Elsewhere");
        let joined = join_stoa(&join_request(&g, &g.address().unwrap()), &mut store);

        let list: serde_json::Value = serde_json::from_str(&list_stoas("{}", &store)).unwrap();
        let join: serde_json::Value = serde_json::from_str(&joined).unwrap();

        for v in [&list["items"][0], &join] {
            assert!(
                v.get("foundingTitle").is_some(),
                "the title must be named as founding: {v}"
            );
            assert!(
                v.get("title").is_none(),
                "a bare `title` would assert a currency nothing has checked: {v}"
            );
        }
    }

    #[test]
    fn a_peer_in_no_stoa_lists_nothing_and_reports_no_failure() {
        let store = a_membership_store();
        let out = list_stoas("{}", &store);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("error").is_none(),
            "an empty listing is not a failure: {out}"
        );
        assert_eq!(v["items"].as_array().unwrap().len(), 0);
        assert_eq!(v["page"], 0);
        // NO SPEC: the spec does not say what `hasMore` holds for an empty listing.
        // `false` — there is no further page.
        assert_eq!(v["hasMore"], false);
    }

    #[test]
    fn every_stoa_is_reachable_by_paging_and_appears_once() {
        // A population larger than one page, with a page size that does not divide
        // it: an off-by-one in the offset or in `hasMore` is invisible when the
        // last page happens to be full.
        let mut store = a_membership_store();
        let mut expected = Vec::new();
        for n in 0..7 {
            let reply = create(&mut store, &format!("Stoa {n}"));
            expected.push(reply["stoa"].as_str().unwrap().to_string());
        }
        expected.sort();

        let mut seen: Vec<String> = listed(&store, 3)
            .iter()
            .map(|row| row["stoa"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(seen.len(), 7, "every Stoa must be reachable by paging");
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 7, "no Stoa may appear twice");
        assert_eq!(seen, expected);
    }

    #[test]
    fn an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership() {
        // THE DIRECTION MEMBERSHIP MUST NOT BE DERIVED FROM OPS. An op is
        // attacker-supplied and its Stoa address is a field the SENDER chose, so a
        // peer that joined a Stoa because an op mentioned it would be a peer any
        // stranger can enrol.
        //
        // WHAT THIS TEST USED TO DO, AND WHY IT WAS CHANGED. It built five ops in a
        // `MemoryOpLog`, asserted `len() == 5` under the comment "the fixture must
        // reach the assertion", and then never read the log again. Changing the
        // loop to `0..0` was predicted to fail it and **observed to pass**
        // (`findings/spec-test.md` entry 4): the ops could not reach any assertion,
        // because a `MemoryOpLog` and a `MembershipStore` share no state. The
        // surviving content was `contains(unjoined) == false` on a store nothing
        // was joined into — true of any empty store.
        //
        // The fixture now REACHES the assertion: the op store and the membership
        // store are the same directory on a real disk, so an implementation that
        // derived membership from what it found in that directory has the material
        // to do it and is caught doing it. That is the shape
        // `a_membership_is_recordable_into_a_store_that_previously_held_none`
        // already had, and the only shape at this boundary that can fail.
        let dir = WireTempDir::new("ops-enrol-nobody");
        let unjoined = a_joinable_record("A Stoa nobody here joined");
        let unjoined_address = unjoined.address().unwrap();

        // Many ops, not one: a handler that enrolled on the Nth would pass a
        // single-op test. On a real disk, in the same directory the membership
        // store is about to be opened in.
        {
            let mut log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("a fresh op store is creatable");
            for n in 0..5 {
                log.append(
                    an_op_in(unjoined_address, &format!("post {n}")),
                    Arrival::unordered(),
                )
                .unwrap();
            }
            assert_eq!(log.len().unwrap(), 5, "the ops must be on the disk");
        }

        // The peer is in no Stoa for that address, and the listing does not
        // contain it — asked through the wire, against the same directory.
        let listing = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            list_stoas("{}", store)
        });
        let lv: serde_json::Value = serde_json::from_str(&listing).unwrap();
        // The error arm first, spelled out: an implementation that derived
        // membership from those ops can fail EITHER by listing a Stoa nobody
        // joined or by choking on what it derived, and both are this test's
        // business. Without this arm the failure arrives as an `unwrap` on
        // `None`, which names nothing.
        assert!(
            lv.get("error").is_none(),
            "a listing beside an op store must not fail: {listing}"
        );
        assert_eq!(
            lv["items"].as_array().unwrap().len(),
            0,
            "five ops for a Stoa nobody joined must enrol nobody: {listing}"
        );

        // And an op for an unjoined Stoa does not disturb a membership that DOES
        // exist. The retained record is compared byte for byte, which is what
        // catches a store that re-wrote the row rather than leaving it alone.
        let joined = a_joinable_record("The one Stoa");
        let joined_address = joined.address().unwrap();
        assert_ne!(
            joined_address, unjoined_address,
            "the fixture's two Stoas must differ, or the assertions below prove nothing"
        );
        with_membership_store(&membership_path_in(dir.path()), |store| {
            join_stoa(&join_request(&joined, &joined_address), store)
        });
        let before = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            serde_json::to_string(&store.get(&joined_address).unwrap().unwrap().genesis.title)
                .unwrap()
        });

        {
            let mut log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("the op store must reopen");
            log.append(an_op_in(unjoined_address, "another"), Arrival::unordered())
                .unwrap();
            assert_eq!(log.len().unwrap(), 6, "the sixth op must be on the disk");
        }

        let after = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            list_stoas("{}", store)
        });
        let av: serde_json::Value = serde_json::from_str(&after).unwrap();
        let addresses: Vec<&str> = av["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["stoa"].as_str().unwrap())
            .collect();
        assert_eq!(
            addresses,
            vec![joined_address.to_hex().as_str()],
            "the peer is still in exactly the one Stoa it joined: {after}"
        );
        let still = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            serde_json::to_string(&store.get(&joined_address).unwrap().unwrap().genesis.title)
                .unwrap()
        });
        assert_eq!(
            still, before,
            "that Stoa's retained record must be unchanged"
        );
    }

    #[test]
    fn an_empty_op_log_does_not_empty_the_listing() {
        // The other direction of the same boundary, at the wire. A peer in several
        // Stoas whose op log holds NOTHING must still list every one of them — an
        // answer derived from the log would list none.
        //
        // As above, the op store is a REAL one in the SAME directory rather than a
        // `MemoryOpLog` the listing cannot reach. The previous version created a
        // `MemoryOpLog`, asserted `len() == 0`, and never touched it again, which
        // made this a second copy of
        // `every_stoa_is_reachable_by_paging_and_appears_once`
        // (`findings/spec-test.md` entry 4).
        let dir = WireTempDir::new("quiet-stoas");
        for n in 0..3 {
            let title = format!("Quiet {n}");
            with_membership_store(&membership_path_in(dir.path()), |store| {
                create_stoa(
                    &serde_json::json!({ "title": &title }).to_string(),
                    || Ok(creator_key()),
                    store,
                )
            });
        }
        // An op store that exists, opens, and holds nothing — the state an
        // implementation reading it would answer "no Stoas" from.
        {
            let log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("a fresh op store is creatable");
            assert_eq!(log.len().unwrap(), 0, "the op store on disk must be empty");
        }
        let listing = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            list_stoas(&serde_json::json!({ "perPage": 20 }).to_string(), store)
        });
        let lv: serde_json::Value = serde_json::from_str(&listing).unwrap();
        assert_eq!(
            lv["items"].as_array().unwrap().len(),
            3,
            "every Stoa the peer is in must be listed however few ops it holds: {listing}"
        );
    }

    #[test]
    fn a_hostile_request_is_an_error_rather_than_an_abort_and_carries_no_result() {
        // Every shape the spec enumerates — an absent field, a field of the wrong
        // type, an address that is not an address, a record that is not a record,
        // and a record and address that disagree — across all three methods. These
        // are the first methods on this surface that reach persistent state, so a
        // panic here aborts the module process and takes the user's session with
        // it.
        let g = a_joinable_record("Agora");
        let address = g.address().unwrap().to_hex();
        let genesis_hex = hex::encode(g.canonical_bytes().unwrap());
        let elsewhere = a_joinable_record("Elsewhere").address().unwrap().to_hex();

        // create: the title is the only field, and there is no address to malform.
        let creates = [
            "not json".to_string(),
            r#"{}"#.to_string(),
            r#"{"title":7}"#.to_string(),
            r#"{"title":null}"#.to_string(),
            r#"{"title":["a"]}"#.to_string(),
            r#"[]"#.to_string(),
        ];
        // join: address and record, each malformable, plus the two disagreeing.
        let joins = [
            "not json".to_string(),
            r#"{}"#.to_string(),
            format!(r#"{{"genesis":"{genesis_hex}"}}"#),
            format!(r#"{{"stoa":"{address}"}}"#),
            r#"{"stoa":7,"genesis":"00"}"#.to_string(),
            format!(r#"{{"stoa":"nothex","genesis":"{genesis_hex}"}}"#),
            format!(r#"{{"stoa":"00ff","genesis":"{genesis_hex}"}}"#),
            format!(r#"{{"stoa":"{address}","genesis":7}}"#),
            format!(r#"{{"stoa":"{address}","genesis":"nothex!"}}"#),
            // The two disagreeing: a well-formed record for a different Stoa.
            format!(r#"{{"stoa":"{elsewhere}","genesis":"{genesis_hex}"}}"#),
        ];
        // list: the pagination arguments.
        let lists = [
            "not json".to_string(),
            r#"{"page":-1}"#.to_string(),
            r#"{"page":1.5}"#.to_string(),
            r#"{"page":"first"}"#.to_string(),
            r#"{"perPage":"many"}"#.to_string(),
            r#"{"perPage":-3}"#.to_string(),
        ];

        for bad in &creates {
            let out = create_stoa(bad, || Ok(creator_key()), &mut a_membership_store());
            assert_error_only(&out, bad, "stoa");
        }
        for bad in &joins {
            let out = join_stoa(bad, &mut a_membership_store());
            assert_error_only(&out, bad, "stoa");
        }
        for bad in &lists {
            let out = list_stoas(bad, &a_membership_store());
            assert_error_only(&out, bad, "items");
        }

        // The module answers subsequent calls: a good request after every bad one.
        let mut store = a_membership_store();
        let good = create(&mut store, "Agora");
        assert!(good.get("error").is_none(), "got {good}");
    }

    /// Assert a reply is the error shape and carries no result field.
    ///
    /// A helper because the pair of assertions is the same at every call site and a
    /// second copy would eventually check only the first half — which is the
    /// partial-success shape §2.5 forbids, unasserted.
    fn assert_error_only(out: &str, request: &str, result_field: &str) {
        let v: serde_json::Value = serde_json::from_str(out)
            .unwrap_or_else(|e| panic!("for {request:?}, reply was not JSON ({e}): {out}"));
        assert!(v.get("error").is_some(), "for {request:?}, got {out}");
        assert!(
            v.get(result_field).is_none(),
            "a failure must never also carry a result — §2.5; for {request:?}, got {out}"
        );
    }

    #[test]
    fn a_failed_call_records_nothing_and_disturbs_no_retained_record() {
        // "A failed call records nothing" over a store that already HOLDS
        // something, which is the case a test against an empty store cannot see:
        // an empty store's "unchanged" is indistinguishable from "wiped".
        let mut store = a_membership_store();
        let one = create(&mut store, "Agora");
        let address = crate::identity::Address::from_hex(one["stoa"].as_str().unwrap()).unwrap();
        let before = store.get(&address).unwrap().unwrap();

        let g = a_joinable_record("Elsewhere");
        for bad in [
            r#"{"title":7}"#.to_string(),
            format!(r#"{{"title":"{}"}}"#, "x".repeat(1025)),
        ] {
            let _ = create_stoa(&bad, || Ok(creator_key()), &mut store);
        }
        let _ = join_stoa(&join_request(&g, &address), &mut store);
        let _ = join_stoa(r#"{"stoa":"nothex","genesis":"00"}"#, &mut store);

        assert_eq!(
            store.len().unwrap(),
            1,
            "the set of Stoas the peer is in must be unchanged"
        );
        assert_eq!(
            store.get(&address).unwrap().unwrap(),
            before,
            "the retained record of every Stoa must be unchanged"
        );
    }

    #[test]
    fn a_store_that_cannot_be_opened_is_the_error_shape_and_not_an_empty_listing() {
        // The failure one step earlier than the read, at the adapter's entry point.
        // An empty listing is indistinguishable from a peer that is in no Stoa, so
        // flattening this would invite the user to re-paste every address they hold.
        //
        // Reached with a path that cannot be a SQLite database: a DIRECTORY.
        //
        // **The REASON is asserted, not merely that something failed.** Without
        // that, this test passes for a handler that refused for any reason at all
        // — including one unrelated to the store being unopenable — and it would
        // stay green if SQLite's behaviour on a directory changed from "unable to
        // open" to something else entirely. It would also stay green for a
        // `with_membership_store` that returned a fixed error and never tried. The
        // literal is the message SQLite gives and this code passes through; it is
        // the same obligation `a_store_that_cannot_be_opened_is_the_error_shape_
        // and_not_an_empty_feed` carries for the feed, where the reason can be
        // injected because that handler takes a closure and this one takes a path.
        let dir = std::env::temp_dir();
        let out = with_membership_store_read(&dir, |store| list_stoas("{}", store));
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v.get("items").is_none(),
            "a failure must never also carry a result — §2.5"
        );
        let reason = v["error"].as_str().unwrap();
        assert!(
            reason.contains("unable to open database file"),
            "the reason the store could not be opened must reach the view so it \
             can be named, got {out}"
        );
        // And the failure is the STORE's, reported in the membership store's own
        // vocabulary rather than as a bare SQLite string — which is what makes
        // this the error shape a view renders and not a leaked backend message.
        assert!(
            reason.contains("membership store"),
            "the refusal must say which store could not be used, got {out}"
        );
    }

    #[test]
    fn the_membership_path_is_a_file_of_its_own_beside_the_op_logs() {
        // Hardcoded on both sides, because this is an on-disk name every peer's
        // installation carries and changing it orphans their memberships. It must
        // also NOT be the op log's file: sharing one would mean the op log's layout
        // version had to mean two things, which is the defect the separate file
        // exists to prevent.
        let dir = std::path::Path::new("/some/dir");
        assert_eq!(
            membership_path_in(dir),
            std::path::PathBuf::from("/some/dir/stoas.sqlite")
        );
        assert_ne!(
            membership_path_in(dir),
            dir.join("ops.sqlite"),
            "membership must not share the op log's file"
        );
    }

    #[test]
    fn the_policy_a_created_stoa_declares_is_reported_by_name() {
        // NO SPEC: the spec requires the posting policy be answerable from what was
        // retained and does not say which policy a creation declares or what it is
        // called on the wire. Creation accepts no policy parameter and always
        // declares `Open`; the wire name is the literal "open".
        //
        // Hardcoded, because it is a wire string a view branches on. `Policy` has
        // one variant today, so `policy_name` is exhaustively covered by this one
        // case — and it has no wildcard arm, so a second variant fails to compile
        // rather than silently rendering as "open".
        let mut store = a_membership_store();
        let reply = create(&mut store, "Agora");
        assert_eq!(reply["policy"], "open");

        let g = a_joinable_record("Elsewhere");
        let joined = join_stoa(&join_request(&g, &g.address().unwrap()), &mut store);
        let v: serde_json::Value = serde_json::from_str(&joined).unwrap();
        assert_eq!(v["policy"], "open");
    }

    #[test]
    fn an_unknown_field_in_a_request_is_ignored_rather_than_refused() {
        // NO SPEC: the spec does not say what an unrecognised field does. Ignored,
        // matching `list_threads`'s treatment of an offered `order` — an unknown
        // field is not a caller error, and refusing one would break every view
        // written against a later, wider request shape.
        let mut store = a_membership_store();
        let plain = create_stoa(r#"{"title":"Agora"}"#, || Ok(creator_key()), &mut store);
        let mut other = a_membership_store();
        let extra = create_stoa(
            r#"{"title":"Agora","somethingElse":true,"order":"new"}"#,
            || Ok(creator_key()),
            &mut other,
        );
        assert_eq!(plain, extra, "an unknown field must not change the answer");
    }

    // ─── An existing op store stays readable, and membership survives ─────
    //
    // Two requirements that only a REAL op store on a REAL disk can pin, which is
    // why they live here rather than beside the in-memory handler tests above:
    //
    // - "Adding membership does not make an existing store unreadable" is a claim
    //   about two files in one directory. Every test above uses
    //   `MembershipStore::in_memory`, which has no file at all — so none of them
    //   can see a membership store that clobbered, relocated or re-versioned the
    //   op log's file, and the requirement's own scenarios name an op store that
    //   holds ops.
    // - "Membership survives a restart" is a claim about what `MembershipStore`
    //   leaves on disk, reached through the handlers a view calls, which
    //   `with_membership_store` is the only entry point to.

    /// A temporary directory, and the guard that removes it.
    ///
    /// Built with `std::fs` rather than a `tempfile` dependency, following
    /// `log/sqlite.rs`'s and `membership.rs`'s own fixtures. The guard must be held
    /// for the test's lifetime: dropping it removes the directory.
    struct WireTempDir(std::path::PathBuf);

    impl WireTempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "dialectica-wire-stoa-{}-{name}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
            WireTempDir(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for WireTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// One signed op in `stoa`, with a body that identifies it.
    fn an_op_in(stoa: crate::identity::Address, body: &str) -> crate::op::SignedOp {
        let key = feed_key(4);
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    /// The bodies of every `Post` a real on-disk op log holds, sorted.
    ///
    /// Reads the ops back out of the STORE rather than out of the `SignedOp`
    /// values the test still holds, because "its ops are readable" is a claim
    /// about the file and not about the test's own memory.
    fn bodies_on_disk(dir: &std::path::Path) -> Vec<String> {
        let log = crate::log::SqliteOpLog::open(&dir.join("ops.sqlite"))
            .expect("the op store must still open");
        let mut out: Vec<String> = log
            .iter()
            .expect("the op store's ops must still be readable")
            .iter()
            .filter_map(|entry| match &entry.op.op.kind {
                OpKind::Post { body, .. } => Some(body.clone()),
                _ => None,
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_store_holding_ops_and_no_memberships_opens_and_keeps_its_ops() {
        // THE REQUIREMENT THE WHOLE DESIGN PIVOTS ON, and the one with no test
        // before this. `design.md` picks a separate `stoas.sqlite` over three
        // alternatives precisely so that "a store already holding ops stays
        // readable, and a membership is recordable into it" holds by construction
        // — and until this test existed, the construction was unchecked.
        //
        // The op store is written FIRST and by `SqliteOpLog` alone, so it is a
        // store that genuinely predates membership rather than one this change
        // helped create. Then membership is exercised in the same directory, and
        // the ops are read back OUT OF THE FILE.
        //
        // The expected bodies are HARDCODED literals rather than read back from
        // the log before the membership work and compared with itself afterwards:
        // that comparison would pass for a membership store that truncated both
        // reads equally.
        let dir = WireTempDir::new("coexist-opens");
        let genesis = a_joinable_record("A Stoa with ops");
        let stoa = genesis.address().unwrap();

        {
            let mut log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("a fresh op store is creatable");
            log.append(an_op_in(stoa, "first"), Arrival::unordered())
                .unwrap();
            log.append(an_op_in(stoa, "second"), Arrival::unordered())
                .unwrap();
            assert_eq!(
                log.len().unwrap(),
                2,
                "the fixture must reach the assertion"
            );
        }

        // Opening the membership store beside it succeeds and is NOT refused on
        // the grounds that the directory predates membership.
        let listing = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            list_stoas("{}", store)
        });
        let v: serde_json::Value = serde_json::from_str(&listing).unwrap();
        assert!(
            v.get("error").is_none(),
            "opening beside an existing op store must not be refused: {listing}"
        );
        // The peer is reported as being in no Stoa — not as an error, and not as
        // a Stoa invented from the ops that are sitting right there.
        assert_eq!(v["items"].as_array().unwrap().len(), 0);

        // And the ops the store already held are still readable, by body.
        assert_eq!(bodies_on_disk(dir.path()), vec!["first", "second"]);
    }

    #[test]
    fn a_membership_is_recordable_into_a_store_that_previously_held_none() {
        // The half `design.md`'s own table says the rejected alternatives fail:
        // "opening succeeds and ops are readable" holds for a `memberships` table
        // added to `create_schema`, and "a membership is recordable" does not —
        // the first write dies as `no such table`. So the join must be exercised,
        // not only the open.
        let dir = WireTempDir::new("coexist-records");
        let with_ops = a_joinable_record("A Stoa with ops");
        let ops_stoa = with_ops.address().unwrap();

        {
            let mut log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("a fresh op store is creatable");
            log.append(an_op_in(ops_stoa, "already here"), Arrival::unordered())
                .unwrap();
        }

        let joining = a_joinable_record("The Stoa being joined");
        let joined_address = joining.address().unwrap();
        let out = with_membership_store(&membership_path_in(dir.path()), |store| {
            join_stoa(&join_request(&joining, &joined_address), store)
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v.get("error").is_none(),
            "a membership must be recordable into a store that held none: {out}"
        );
        assert_eq!(v["stoa"].as_str().unwrap(), joined_address.to_hex());

        // The ops the store already held are still readable afterwards.
        assert_eq!(bodies_on_disk(dir.path()), vec!["already here"]);

        // And membership did not invent a Stoa out of the op that was there: the
        // listing is exactly the one Stoa that was joined. This is the half that
        // fails for an implementation deriving membership from the log — which is
        // reachable here and nowhere above, because no test above has both an op
        // store and a membership store in one place.
        let listing = with_membership_store_read(&membership_path_in(dir.path()), |store| {
            list_stoas("{}", store)
        });
        let lv: serde_json::Value = serde_json::from_str(&listing).unwrap();
        let addresses: Vec<&str> = lv["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["stoa"].as_str().unwrap())
            .collect();
        assert_eq!(
            addresses,
            vec![joined_address.to_hex().as_str()],
            "the listing must be what membership records and nothing from the ops"
        );
        assert_ne!(
            joined_address, ops_stoa,
            "the fixture's two Stoas must differ, or the assertion above proves nothing"
        );
    }

    #[test]
    fn the_membership_store_does_not_write_into_the_op_logs_file() {
        // The file boundary as a property of the BYTES rather than of the path
        // name. `the_membership_path_is_a_file_of_its_own_beside_the_op_logs`
        // asserts two `PathBuf`s differ, which is a statement about
        // `membership_path_in` and says nothing about what the store then does.
        //
        // A membership store that opened the op log's file anyway — a hardcoded
        // name inside `MembershipStore::open`, say — would pass that test and
        // fail this one, because the op log's file would change under it.
        //
        // The op store's bytes are hashed BEFORE and AFTER, and the ops are read
        // back by body against hardcoded literals, so "unchanged" is checked two
        // independent ways.
        let dir = WireTempDir::new("separate-files");
        let stoa = a_joinable_record("A Stoa with ops").address().unwrap();
        let ops_path = dir.path().join("ops.sqlite");

        {
            let mut log =
                crate::log::SqliteOpLog::open(&ops_path).expect("a fresh op store is creatable");
            log.append(an_op_in(stoa, "untouched"), Arrival::unordered())
                .unwrap();
        }
        let before = std::fs::read(&ops_path).expect("the op store's file is readable");
        assert!(
            !before.is_empty(),
            "the fixture must have written something"
        );

        let joining = a_joinable_record("Somewhere new");
        let address = joining.address().unwrap();
        let out = with_membership_store(&membership_path_in(dir.path()), |store| {
            join_stoa(&join_request(&joining, &address), store)
        });
        assert!(
            serde_json::from_str::<serde_json::Value>(&out)
                .unwrap()
                .get("error")
                .is_none(),
            "the join must succeed for this to be about the file: {out}"
        );

        let after = std::fs::read(&ops_path).expect("the op store's file is still readable");
        assert_eq!(
            before, after,
            "recording a membership must not write a byte into the op log's file"
        );
        assert_eq!(bodies_on_disk(dir.path()), vec!["untouched"]);

        // And the membership store left a file of its OWN, so the assertion above
        // is not passing because nothing was written anywhere at all.
        assert!(
            membership_path_in(dir.path()).exists(),
            "the membership store must have its own file"
        );
    }

    #[test]
    fn a_created_and_a_joined_stoa_both_survive_a_restart_with_their_founding_values() {
        // "Membership survives a restart", through the handlers a view calls. The
        // store-level test in `membership.rs` uses `MembershipStore` directly and
        // so cannot see a `create_stoa` that built its record from something it
        // did not retain, nor a `with_membership_store` that opened a different
        // file on the second call.
        //
        // "Restart" is modelled as every store object being dropped and the path
        // reopened, which is exactly what the adapter does — it opens per call.
        // Each founding title is asserted against a HARDCODED literal rather than
        // against the create reply, so a store that retained an empty title for
        // both would fail.
        let dir = WireTempDir::new("restart");
        let path = membership_path_in(dir.path());

        let created = with_membership_store(&path, |store| {
            create_stoa(r#"{"title":"The one I made"}"#, || Ok(creator_key()), store)
        });
        let cv: serde_json::Value = serde_json::from_str(&created).unwrap();
        assert!(cv.get("error").is_none(), "got {created}");
        let created_address = cv["stoa"].as_str().unwrap().to_string();

        let joining = a_joinable_record("The one I joined");
        let joined_address = joining.address().unwrap();
        let joined = with_membership_store(&path, |store| {
            join_stoa(&join_request(&joining, &joined_address), store)
        });
        assert!(
            serde_json::from_str::<serde_json::Value>(&joined)
                .unwrap()
                .get("error")
                .is_none(),
            "got {joined}"
        );

        // Every store object is gone by now — `with_membership_store` opens and
        // drops one per call. Reopen from the path, as a restarted process does.
        let listing = with_membership_store_read(&path, |store| {
            list_stoas(r#"{"page":0,"perPage":20}"#, store)
        });
        let lv: serde_json::Value = serde_json::from_str(&listing).unwrap();
        assert!(lv.get("error").is_none(), "got {listing}");

        let mut rows: Vec<(String, String)> = lv["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["stoa"].as_str().unwrap().to_string(),
                    row[FOUNDING_TITLE].as_str().unwrap().to_string(),
                )
            })
            .collect();
        rows.sort();

        let mut expected = vec![
            (created_address, "The one I made".to_string()),
            (joined_address.to_hex(), "The one I joined".to_string()),
        ];
        expected.sort();
        assert_eq!(
            rows, expected,
            "both Stoas must survive the restart and still answer their founding titles"
        );

        // And the retained record still verifies against the address it is held
        // under, after the restart — the half a store that kept only titles and
        // addresses would fail.
        let store = crate::membership::MembershipStore::open(&path).unwrap();
        for (hex, _) in &expected {
            let address = crate::identity::Address::from_hex(hex).unwrap();
            let held = store
                .get(&address)
                .unwrap()
                .expect("the Stoa is still retained");
            assert!(
                held.genesis.matches(&address),
                "the retained record must still hash to its address after a restart"
            );
        }
    }

    #[test]
    fn a_join_refused_at_the_wire_leaves_nothing_behind_a_restart() {
        // A refused join must leave no trace that a later process could read as a
        // membership. Reached through the wire handler and checked after a reopen,
        // because "nothing was recorded" and "nothing was COMMITTED" are different
        // claims and only the second survives a restart.
        //
        // BOTH addresses are checked: the one the caller claimed, and the one the
        // supplied record actually names. A handler that "helpfully" filed the
        // record under its own address would leave exactly that behind.
        //
        // **A SECOND, LEGITIMATE Stoa is joined in the same store, and it must
        // survive.** Measured: without it, this test passes for a store that
        // persists NOTHING AT ALL — "the refused join left nothing" and "the store
        // forgets everything" produce the same empty listing, which is this
        // project's recurring defect family. The surviving Stoa is what tells the
        // two explanations apart.
        let dir = WireTempDir::new("refused-restart");
        let path = membership_path_in(dir.path());
        let real = a_joinable_record("Agora");
        let claimed = real.address().unwrap();
        let impostor = crate::stoa::Genesis {
            creator: feed_key(9).public_key(),
            ..real.clone()
        };

        let survivor = a_joinable_record("The one that is really joined");
        let survivor_address = survivor.address().unwrap();
        let ok = with_membership_store(&path, |store| {
            join_stoa(&join_request(&survivor, &survivor_address), store)
        });
        assert!(
            serde_json::from_str::<serde_json::Value>(&ok)
                .unwrap()
                .get("error")
                .is_none(),
            "the legitimate join must succeed, or this test cannot tell a refused \
             join from a store that persists nothing: {ok}"
        );

        let out = with_membership_store(&path, |store| {
            join_stoa(&join_request(&impostor, &claimed), store)
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "the join must be refused: {out}");

        let listing = with_membership_store_read(&path, |store| list_stoas("{}", store));
        let lv: serde_json::Value = serde_json::from_str(&listing).unwrap();
        let addresses: Vec<&str> = lv["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["stoa"].as_str().unwrap())
            .collect();
        assert_eq!(
            addresses,
            vec![survivor_address.to_hex().as_str()],
            "after the restart the listing must hold exactly the Stoa that was \
             really joined — the refused one left nothing, and the real one survived: {listing}"
        );

        let store = crate::membership::MembershipStore::open(&path).unwrap();
        assert!(
            !store.contains(&claimed).unwrap(),
            "not in the Stoa that was claimed"
        );
        assert!(
            !store.contains(&impostor.address().unwrap()).unwrap(),
            "and not in the Stoa the supplied record names either"
        );
    }

    #[test]
    fn a_listing_page_that_is_not_the_last_says_so_on_the_wire() {
        // `hasMore` at the wire, both halves, with a population that fills exactly
        // two pages — the case where "look one row past the end" and "compare
        // against a count" disagree.
        //
        // `every_stoa_is_reachable_by_paging_and_appears_once` pages through with
        // the `listed` helper, which STOPS when `hasMore` is not true: a `hasMore`
        // stuck at `false` makes that helper return the first page and the
        // assertion there fails on the count, but a `hasMore` stuck at `true`
        // makes it loop to its 1000-page guard and panic on the fixture rather
        // than on the requirement. Neither reading pins the boundary itself, and
        // the counts are HARDCODED here rather than derived from `items.len()`.
        let mut store = a_membership_store();
        for n in 0..4 {
            create(&mut store, &format!("Stoa {n}"));
        }

        let first: serde_json::Value =
            serde_json::from_str(&list_stoas(r#"{"page":0,"perPage":2}"#, &store)).unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 2);
        assert_eq!(
            first["hasMore"], true,
            "a further page exists and the reply must say so: {first}"
        );
        assert_eq!(first["page"], 0);

        let second: serde_json::Value =
            serde_json::from_str(&list_stoas(r#"{"page":1,"perPage":2}"#, &store)).unwrap();
        assert_eq!(second["items"].as_array().unwrap().len(), 2);
        assert_eq!(
            second["hasMore"], false,
            "an exactly-full last page must not claim a further one: {second}"
        );
        assert_eq!(second["page"], 1);

        // The two pages are disjoint, which is what makes `hasMore` above about
        // paging rather than about a number the handler made up.
        let a = first["items"][0]["stoa"].as_str().unwrap();
        let b = second["items"][0]["stoa"].as_str().unwrap();
        assert_ne!(a, b, "the second page must not repeat the first");
    }

    #[test]
    fn the_creator_a_creation_names_is_the_identity_the_probe_reports() {
        // THE property the `4313cf6` bug broke, pinned where `cargo test` reaches
        // it. A Stoa's creator is its sole moderator and is fixed inside the
        // address preimage forever, so a creator key this peer would never sign
        // with is a Stoa nobody can moderate — permanently, and with no error
        // anywhere.
        //
        // WHY THIS TEST AND NOT `keystore.rs`'s. That one asserts
        // `identity_address() == identity_public_key().address()`, which is a fact
        // about two `Keystore` methods and is true whatever the module wires up.
        // This one goes through the two WIRE HANDLERS, each reached through the
        // `core::keystore` function the adapter calls — so it fails for the
        // mutation that actually shipped: pointing one of the two at
        // `stoa_address(stoa)` while leaving the other alone.
        //
        // The mutation it catches, verified by running it: change
        // `creator_and_poster_in`'s second element to `ks.stoa_address(&creator.address())`
        // and this test fails, where the whole rest of the suite passes.
        let dir = WireTempDir::new("creator-is-poster");
        crate::keystore::Keystore::generate()
            .expect("the test host has randomness")
            .create(
                &crate::keystore::default_path_in(dir.path()),
                &crate::keystore::Unlock::Unencrypted,
            )
            .expect("a keystore is writable into a fresh directory");

        // `create_stoa` is given the creator key exactly as the adapter gives it.
        let mut store = a_membership_store();
        let out = create_stoa(
            &serde_json::json!({ "title": "Agora" }).to_string(),
            || crate::keystore::creator_key_in(dir.path()),
            &mut store,
        );
        let created: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            created.get("error").is_none(),
            "creation must succeed: {out}"
        );
        let stoa = crate::identity::Address::from_hex(created["stoa"].as_str().unwrap()).unwrap();

        // `get_capabilities` is given the `creator_and_poster_in` half of the
        // pairing, which is what this test is about.
        //
        // **It is no longer what the adapter passes**, and saying so is the point.
        // This comment claimed it was, and `main`'s `identity-onboarding` made that
        // false: the adapter now calls `get_capabilities_from_stores`, whose lookup
        // is `posting_identity` — a PATH-DERIVED per-Stoa address read out of the
        // identity record. So the live probe and `create_stoa`'s root-derived
        // creator are two different keys again, which is the very divergence
        // `creator_and_poster_in` exists to prevent, reintroduced by a merge rather
        // than by an edit. This test cannot see it, because it injects both halves;
        // the gap is recorded in `design.md` under Decisions and reported as a spec
        // question rather than patched here.
        //
        // What the test still proves is the pairing itself: given the pairing, the
        // creator a creation names IS the identity the probe reports.
        let probe = get_capabilities(
            &serde_json::json!({ "stoa": stoa.to_hex() }).to_string(),
            |_stoa| {
                crate::keystore::poster_address_in(dir.path())
                    .map(|a| a.to_hex())
                    .map_err(|e| e.to_string())
            },
        );
        let probed: serde_json::Value = serde_json::from_str(&probe).unwrap();

        // The record the creation retained, read back out of the store rather than
        // rebuilt, so the creator asserted on is the one that went into the
        // address preimage.
        let genesis = store.get(&stoa).unwrap().unwrap().genesis;
        let moderators = crate::moderation::Moderators::of(&genesis).unwrap();

        // Half one: the identity the probe reports IS the creator's own address.
        // This is the half that fails when the two derivations diverge.
        assert_eq!(
            probed["identity"].as_str(),
            Some(genesis.creator.address().to_hex().as_str()),
            "the identity the probe reports must be the address of the key the \
             creation named as creator, or a view shows the user an identity that \
             cannot moderate what they just made: probe {probe}"
        );

        // Half two: and that key is therefore the Stoa's moderator, which is the
        // authority check the whole pairing exists to satisfy.
        let reported =
            crate::identity::Address::from_hex(probed["identity"].as_str().unwrap()).unwrap();
        assert_eq!(moderators.stoa(), &stoa);
        assert!(
            moderators.contains(&genesis.creator),
            "the creator must be the Stoa's moderator"
        );
        assert_eq!(
            reported,
            genesis.creator.address(),
            "and the moderator's address must be the one the probe reported"
        );
    }

    #[test]
    fn a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership() {
        // "Verification consults nothing but the two inputs", at the wire. Shown by
        // making the SAME decision in three states that differ in everything a
        // verifier could have consulted: an empty store, a store already holding
        // that exact Stoa, and a store holding a different Stoa plus a real op log
        // full of ops for the Stoa being verified.
        //
        // A verifier that consulted any of those would have different material
        // available in the three cases. The three replies are compared to each
        // other AND to a hardcoded expectation of what the refusal says, so three
        // identical *wrong* answers would not pass.
        let dir = WireTempDir::new("verification-inputs");
        let real = a_joinable_record("Agora");
        let claimed = real.address().unwrap();
        let impostor = crate::stoa::Genesis {
            title: "Not Agora".to_string(),
            ..real.clone()
        };
        let request = join_request(&impostor, &claimed);

        let mut empty = a_membership_store();
        let from_empty = join_stoa(&request, &mut empty);

        let mut holding_it = a_membership_store();
        join_stoa(&join_request(&real, &claimed), &mut holding_it);
        let from_holding_it = join_stoa(&request, &mut holding_it);

        // The third case is on a REAL DISK, in the same directory as its op log.
        // In-memory, as this was, the ops sat somewhere the store could not have
        // reached even in principle, so "a verifier that looked would have found
        // them" was not true of the fixture. Now it is: the ops are one
        // `dir.join("ops.sqlite")` away from the store being used.
        let elsewhere = a_joinable_record("Elsewhere");
        {
            let mut log = crate::log::SqliteOpLog::open(&dir.path().join("ops.sqlite"))
                .expect("a fresh op store is creatable");
            for n in 0..3 {
                log.append(an_op_in(claimed, &format!("op {n}")), Arrival::unordered())
                    .unwrap();
            }
            // The ops are on the disk, which is the state this third case differs
            // by. NOT "the fixture must reach the assertion", which this said
            // before and which was backwards: reaching the assertion is exactly
            // what must not happen. The assertion is that the three replies agree,
            // and a reply derived from ops would differ.
            assert_eq!(log.len().unwrap(), 3, "the ops must be on the disk");
        }
        with_membership_store(&membership_path_in(dir.path()), |store| {
            join_stoa(
                &join_request(&elsewhere, &elsewhere.address().unwrap()),
                store,
            )
        });
        let from_with_ops = with_membership_store(&membership_path_in(dir.path()), |store| {
            join_stoa(&request, store)
        });

        assert_eq!(
            from_empty, from_holding_it,
            "the verification's answer must not depend on what membership holds"
        );
        assert_eq!(
            from_empty, from_with_ops,
            "the verification's answer must not depend on what ops exist"
        );
        // And the answer is the refusal, spelled out — not three agreeing joins.
        // The expected message is a hardcoded literal, so a handler that agreed
        // three times about something else does not pass.
        let v: serde_json::Value = serde_json::from_str(&from_empty).unwrap();
        assert_eq!(
            v.get("error").and_then(|e| e.as_str()),
            Some("the genesis record does not hash to the Stoa address it was given with"),
            "a mismatched record must be refused, got {from_empty}"
        );
        assert!(v.get("stoa").is_none());
    }
}
