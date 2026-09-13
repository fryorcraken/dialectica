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
pub fn ping(request: &str) -> String {
    guarded("ping", || {
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {}", e)),
        };
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
        };
        let stoa = match parsed.get("stoa") {
            Some(serde_json::Value::String(s)) => match crate::identity::Address::from_hex(s) {
                Ok(a) => a,
                Err(e) => return error_json(&format!("stoa: {e}")),
            },
            Some(_) => return error_json("stoa must be a string"),
            None => return error_json("missing field: stoa"),
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

/// The `stoa` field, parsed. One job, because three handlers below need it and a
/// fourth copy would eventually disagree with the first three about whether a
/// missing field and a wrong-typed one are the same mistake.
///
/// The `Err` arm is already the wire reply, following `parse_channel_id`: a caller
/// cannot accidentally invent a second error shape while converting one.
fn parse_stoa(parsed: &serde_json::Value) -> Result<crate::identity::Address, String> {
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
                    // `Keystore::generate` may not fail, and the fallible shape is
                    // what this returns anyway, so there is no `expect` added here.
                    self.keystore = Some(crate::keystore::Keystore::generate());
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
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
/// Pinned by a test against hardcoded key names, for the reason
/// `the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies` gives: a
/// view is written against these exact names and renaming one is a breaking change
/// no type checker would catch.
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
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
    list_threads_inner(request, log, genesis)
}

/// Decode a genesis record from its hex form and check it names this Stoa.
///
/// # Why the caller supplies the record at all
///
/// [`Moderators::of`](crate::moderation::Moderators::of) needs a genesis record
/// and there is nowhere else to get one: §9.1 Stage D is where `joinStoa`
/// records what a peer has joined, and it does not exist. Until it does, the
/// record travels with the request.
///
/// **That is not a weakening, because the record is self-authenticating.** §4.8:
/// an address *is* the hash of the genesis record, so a wrong or tampered record
/// fails to match the address it claims. This function verifies rather than
/// trusts, and a mismatch is an error and never a read of something close
/// enough. A caller cannot use this to install themselves as a Stoa's moderator:
/// changing the creator changes the record, which changes the address, which no
/// longer matches the Stoa whose posts are being read.
pub fn genesis_for(
    parsed: &serde_json::Value,
    stoa: &crate::identity::Address,
) -> Result<crate::stoa::Genesis, String> {
    let hex_str = match parsed.get("genesis") {
        Some(serde_json::Value::String(s)) => s,
        Some(_) => return Err(error_json("genesis must be a string")),
        None => return Err(error_json("missing field: genesis")),
    };
    let bytes = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(_) => return Err(error_json("genesis is not valid hex")),
    };
    let genesis = match crate::stoa::Genesis::decode(&bytes) {
        Ok(g) => g,
        Err(e) => return Err(error_json(&format!("genesis: {e}"))),
    };
    // The self-authenticating check, and the whole reason a caller-supplied
    // record is safe. `matches` re-derives the address from the record and
    // compares; a tampered record cannot survive it.
    if !genesis.matches(stoa) {
        return Err(error_json(
            "the genesis record does not hash to the Stoa address it was given with",
        ));
    }
    Ok(genesis)
}

fn list_threads_inner<L: crate::log::OpLog>(
    request: &str,
    log: &L,
    genesis: &crate::stoa::Genesis,
) -> String {
    guarded("list_threads", || {
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
        };

        let stoa = match parsed.get("stoa") {
            Some(serde_json::Value::String(s)) => match crate::identity::Address::from_hex(s) {
                Ok(a) => a,
                Err(e) => return error_json(&format!("stoa: {e}")),
            },
            Some(_) => return error_json("stoa must be a string"),
            None => return error_json("missing field: stoa"),
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
            return error_json(
                "the genesis record does not describe the Stoa this feed was asked for",
            );
        }

        let moderators = match crate::moderation::Moderators::of(genesis) {
            Ok(m) => m,
            Err(e) => return error_json(&format!("genesis: {e}")),
        };

        // A present-but-wrong-typed field is a different mistake from an absent
        // one, and a negative or fractional page is neither — each is refused by
        // name rather than coerced, because coercing would answer a question the
        // caller did not ask.
        let page = match parse_index(&parsed, "page") {
            Ok(v) => v.unwrap_or(0),
            Err(e) => return e,
        };
        let per_page = match parse_index(&parsed, "perPage") {
            Ok(v) => crate::feed::clamp_per_page(v),
            Err(e) => return e,
        };

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
    })
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
        };
        let stoa = match parsed.get("stoa") {
            Some(serde_json::Value::String(s)) => match crate::identity::Address::from_hex(s) {
                Ok(a) => a,
                Err(e) => return error_json(&format!("stoa: {e}")),
            },
            Some(_) => return error_json("stoa must be a string"),
            None => return error_json("missing field: stoa"),
        };
        let genesis = match genesis_for(&parsed, &stoa) {
            Ok(g) => g,
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
        list_threads(request, &log, &genesis)
    })
}

/// A non-negative integer field, absent, or a refusal already in the wire shape.
///
/// Separated out because `page` and `perPage` are the same parsing job with the
/// same three failure modes, and a second copy would eventually disagree with
/// the first about whether `-1` is an error or a zero.
fn parse_index(parsed: &serde_json::Value, field: &str) -> Result<Option<usize>, String> {
    match parsed.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => match n.as_u64() {
            // `as_u64` refuses a negative and a fractional number, which is
            // exactly the set that should be refused: a page of -1 is not a
            // page, and silently clamping it to 0 would serve the first page to
            // a caller who asked for something impossible.
            Some(v) => Ok(Some(v as usize)),
            None => Err(error_json(&format!(
                "{field} must be a non-negative whole number"
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
    let parsed: serde_json::Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return Err(error_json(&format!("invalid JSON: {e}"))),
    };
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
    /// Same shape as `keystore.rs`'s and `log/sqlite.rs`'s, for the reason they
    /// record: one need, in tests, is not worth a `tempfile` dependency. The guard
    /// must be held for the test's lifetime.
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
        let (v, _) = slate_through_the_wire();
        for field in ["slate", "count", "candidates"] {
            assert!(v.get(field).is_some(), "the reply is missing {field}: {v}");
        }
        for candidate in v["candidates"].as_array().unwrap() {
            for field in ["index", "path", "address", "publicKey"] {
                assert!(
                    candidate.get(field).is_some(),
                    "a candidate is missing {field}: {v}"
                );
            }
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

    #[test]
    fn every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape() {
        // The wire contract is only useful if it holds for EVERY method, so
        // check the property rather than each method's happy path again.
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
        ] {
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("handler emitted invalid JSON ({e}): {out}"));
            assert!(v.is_object(), "every reply is a JSON object, got {out}");
        }
    }
}
