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
pub fn get_capabilities(
    request: &str,
    lookup: impl Fn(&crate::identity::Address) -> Result<String, crate::keystore::KeystoreError>,
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
    lookup: impl Fn(&crate::identity::Address) -> Result<String, crate::keystore::KeystoreError>,
) -> Capability {
    match lookup(stoa) {
        Ok(identity) => Capability::CanPost { identity },
        // The reason IS the error's message, not a rewording of it.
        // `KeystoreError::Display` already names the fix for each case — that
        // is a documented obligation on it, with a test — so paraphrasing here
        // would mean maintaining the same guidance in two places, and the two
        // would drift.
        Err(e) => Capability::CannotPost {
            reason: e.to_string(),
        },
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
/// `Genesis::canonical_bytes` is fallible for a title over the genesis cap, and
/// `MembershipStore::join` encodes before it writes — so an over-long title
/// returns before any statement runs. That ordering is what makes "a failed
/// creation leaves nothing behind" structural rather than a rule to remember.
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
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
        match store.join(&stoa, &genesis) {
            Ok(_) => stoa_reply(&stoa, &genesis),
            Err(e) => error_json(&e.to_string()),
        }
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
/// leaving a caller to discover it after joining. `MembershipStore::join` does the
/// verification, and it consults nothing but its two arguments.
///
/// # A repeated join is not a failure
///
/// A pasted address is exactly the input a user supplies twice. The reply is the
/// same either way and carries no "was this new" flag: the spec asks that the
/// second attempt succeed and change nothing, and a view that rendered "already
/// joined" differently would be rendering a distinction the user did not make.
pub fn join_stoa(request: &str, store: &mut crate::membership::MembershipStore) -> String {
    guarded("join_stoa", || {
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
        };
        let stoa = match parse_stoa(&parsed) {
            Ok(a) => a,
            Err(e) => return e,
        };
        // `genesis_for` is the feed path's decoder and already verifies the record
        // against the address. Reused rather than reimplemented: a second decoder
        // would be a second place for the verification to be forgotten, and this
        // one carries the argument for why a caller-supplied record is safe.
        let genesis = match genesis_for(&parsed, &stoa) {
            Ok(g) => g,
            Err(e) => return e,
        };

        // `join` verifies again. That is not redundant belt-and-braces: the store's
        // check is what makes the store's own invariant hold for every caller,
        // including ones that do not come through this handler, and it is the only
        // place a test of the store alone can exercise.
        match store.join(&stoa, &genesis) {
            Ok(_) => stoa_reply(&stoa, &genesis),
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
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid JSON: {e}")),
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

/// Open a membership store and hand it to one of the three handlers above.
///
/// # Why the handlers take a store and this takes a path
///
/// The three handlers take `&mut MembershipStore` because that is the shape worth
/// testing: creating a Stoa and then listing it is one store used twice, and an
/// opener closure would make an in-memory store — the one that needs no temporary
/// directory and no teardown — unusable for exactly the tests that matter most.
///
/// This function is the adapter's entry point, and it is thin on purpose: open,
/// delegate. It is generic over the handler rather than written three times,
/// because "turn a failed open into the error shape" is one job however it is
/// followed up.
///
/// **A failed open is the error shape and never an empty answer.** `SqliteOpLog`'s
/// own documentation makes the argument: an empty listing is indistinguishable
/// from a peer that is in no Stoa, so flattening this would render a peer whose
/// store is broken as a peer that has joined nothing — and the user would be
/// invited to re-paste every address they hold.
///
/// The guard wraps this too, rather than only the handler inside it: a panic while
/// opening a store is a panic in a dispatch handler like any other, and it aborts
/// the module process the same way.
pub fn with_membership_store(
    method: &str,
    path: &std::path::Path,
    handler: impl FnOnce(&mut crate::membership::MembershipStore) -> String,
) -> String {
    guarded(method, || {
        match crate::membership::MembershipStore::open(path) {
            Ok(mut store) => handler(&mut store),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// The membership store's file name inside a host-supplied directory.
///
/// A function rather than a literal at the adapter's call site, following
/// `keystore::default_path_in`: the naming convention belongs with the thing
/// named, so a rename is one edit rather than a search.
///
/// **A file of its own, beside the op log's and never inside it.** `design.md` has
/// the argument; the load-bearing half is that `SqliteOpLog` refuses any layout
/// version that is not exactly its own, and its schema is created only for a
/// never-stamped file — so a membership table added there would reach fresh stores
/// and never an existing one, and the ways round that are a version bump that
/// makes an existing store permanently unopenable or a silent repair of a file the
/// op log's layout check exists to refuse.
pub fn membership_path_in(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("stoas.sqlite")
}

/// Pull the Stoa address out of a request, or the error shape to send back.
///
/// Factored out because create does not take one and both of the other two do, and
/// because the three-way distinction — absent, wrong-typed, not an address — is
/// the one `module-wire-contract` requires be reported by name. A second copy
/// would eventually disagree with the first about which of the three it was.
fn parse_stoa(parsed: &serde_json::Value) -> Result<crate::identity::Address, String> {
    match parsed.get("stoa") {
        Some(serde_json::Value::String(s)) => {
            crate::identity::Address::from_hex(s).map_err(|e| error_json(&format!("stoa: {e}")))
        }
        Some(_) => Err(error_json("stoa must be a string")),
        None => Err(error_json("missing field: stoa")),
    }
}

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
    fn probe_err_with(request: &str, make: impl Fn() -> KeystoreError) -> serde_json::Value {
        serde_json::from_str(&get_capabilities(request, |_| Err(make()))).unwrap()
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
        ] {
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("handler emitted invalid JSON ({e}): {out}"));
            assert!(v.is_object(), "every reply is a JSON object, got {out}");
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
        let listing =
            with_membership_store("list_stoas", &membership_path_in(dir.path()), |store| {
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
        with_membership_store("join_stoa", &membership_path_in(dir.path()), |store| {
            join_stoa(&join_request(&joined, &joined_address), store)
        });
        let before = with_membership_store("_probe", &membership_path_in(dir.path()), |store| {
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

        let after = with_membership_store("list_stoas", &membership_path_in(dir.path()), |store| {
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
        let still = with_membership_store("_probe", &membership_path_in(dir.path()), |store| {
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
            with_membership_store("create_stoa", &membership_path_in(dir.path()), |store| {
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
        let listing =
            with_membership_store("list_stoas", &membership_path_in(dir.path()), |store| {
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
        let out = with_membership_store("list_stoas", &dir, |store| list_stoas("{}", store));
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
        let listing =
            with_membership_store("list_stoas", &membership_path_in(dir.path()), |store| {
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
        let out = with_membership_store("join_stoa", &membership_path_in(dir.path()), |store| {
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
        let listing =
            with_membership_store("list_stoas", &membership_path_in(dir.path()), |store| {
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
        let out = with_membership_store("join_stoa", &membership_path_in(dir.path()), |store| {
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

        let created = with_membership_store("create_stoa", &path, |store| {
            create_stoa(r#"{"title":"The one I made"}"#, || Ok(creator_key()), store)
        });
        let cv: serde_json::Value = serde_json::from_str(&created).unwrap();
        assert!(cv.get("error").is_none(), "got {created}");
        let created_address = cv["stoa"].as_str().unwrap().to_string();

        let joining = a_joinable_record("The one I joined");
        let joined_address = joining.address().unwrap();
        let joined = with_membership_store("join_stoa", &path, |store| {
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
        let listing = with_membership_store("list_stoas", &path, |store| {
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
        let ok = with_membership_store("join_stoa", &path, |store| {
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

        let out = with_membership_store("join_stoa", &path, |store| {
            join_stoa(&join_request(&impostor, &claimed), store)
        });
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "the join must be refused: {out}");

        let listing = with_membership_store("list_stoas", &path, |store| list_stoas("{}", store));
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

        // `get_capabilities` is given the identity lookup exactly as the adapter
        // gives it — including the `_stoa` argument it must NOT use to derive.
        let probe = get_capabilities(
            &serde_json::json!({ "stoa": stoa.to_hex() }).to_string(),
            |_stoa| crate::keystore::poster_address_in(dir.path()).map(|a| a.to_hex()),
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
        with_membership_store("join_stoa", &membership_path_in(dir.path()), |store| {
            join_stoa(
                &join_request(&elsewhere, &elsewhere.address().unwrap()),
                store,
            )
        });
        let from_with_ops =
            with_membership_store("join_stoa", &membership_path_in(dir.path()), |store| {
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
