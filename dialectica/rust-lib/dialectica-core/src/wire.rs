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
fn reject_forbidden_fields(parsed: &serde_json::Value) -> Result<(), String> {
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
fn required_string<'a>(parsed: &'a serde_json::Value, field: &str) -> Result<&'a str, String> {
    match parsed.get(field) {
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(_) => Err(error_json(&format!("{field} must be a string"))),
        None => Err(error_json(&format!("missing field: {field}"))),
    }
}

/// The Stoa address every publish names.
fn required_stoa(parsed: &serde_json::Value) -> Result<crate::identity::Address, String> {
    let hex_str = required_string(parsed, "stoa")?;
    crate::identity::Address::from_hex(hex_str).map_err(|e| error_json(&format!("stoa: {e}")))
}

/// An op id field — a parent, or a vote's target.
fn required_op_id(parsed: &serde_json::Value, field: &str) -> Result<crate::op::OpId, String> {
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
fn required_direction(parsed: &serde_json::Value) -> Result<crate::op::VoteDirection, String> {
    let name = required_string(parsed, "direction")?;
    match name {
        "up" => Ok(crate::op::VoteDirection::Up),
        "down" => Ok(crate::op::VoteDirection::Down),
        other => Err(error_json(&format!(
            "direction must be \"up\" or \"down\", got \"{other}\""
        ))),
    }
}

/// Parse the whole request first, then act. The ordering is the requirement.
///
/// Each handler reads every field it needs before [`crate::authoring`] is
/// reached, so "a refused publish appends nothing and delivery was not invoked"
/// is structural: there is nothing to append until the last field has parsed.
/// Under validate-as-you-go that property would be an artefact of the order the
/// statements happen to be in.
fn parsed_object(request: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(request).map_err(|e| error_json(&format!("invalid JSON: {e}")))
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
/// honest bound on what a sink is for. It does not survive the adapter.
///
/// The module adapter assembles the keystore, the key, the store and the sink
/// once and dispatches over the three handlers through one function-pointer
/// type. A generic `impl FnOnce` monomorphises per call site, so the three are
/// three types and cannot share one pointer — and coercing them fails on a
/// higher-ranked lifetime, because a `&mut dyn FnMut(&OpId)` argument is not the
/// `for<'d> fn(…, &'d mut dyn …)` pointer the dispatch needs.
///
/// **That failure is invisible to every gate that can be run here.** The adapter
/// is behind `cfg(logos_scaffold)`, which no `cargo test` sets, so the error
/// surfaces in the builder's build — the one that runs last and reports worst.
/// `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over` is what
/// catches it in this crate instead, and it found this.
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
    guarded("publish_post", || {
        let parsed = match parsed_object(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Err(e) = reject_forbidden_fields(&parsed) {
            return e;
        }
        let stoa = match required_stoa(&parsed) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let body = match required_string(&parsed, "body") {
            Ok(v) => v.to_string(),
            Err(e) => return e,
        };

        match crate::authoring::post(log, key, stoa, body) {
            Ok(published) => {
                deliver(&published.id);
                published_json(&published)
            }
            Err(refusal) => error_json(&refusal.to_string()),
        }
    })
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
    guarded("publish_reply", || {
        let parsed = match parsed_object(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Err(e) = reject_forbidden_fields(&parsed) {
            return e;
        }
        let stoa = match required_stoa(&parsed) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let parent = match required_op_id(&parsed, "parent") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let body = match required_string(&parsed, "body") {
            Ok(v) => v.to_string(),
            Err(e) => return e,
        };

        match crate::authoring::reply(log, key, stoa, parent, body) {
            Ok(published) => {
                deliver(&published.id);
                published_json(&published)
            }
            Err(refusal) => error_json(&refusal.to_string()),
        }
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
    guarded("publish_vote", || {
        let parsed = match parsed_object(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Err(e) = reject_forbidden_fields(&parsed) {
            return e;
        }
        let stoa = match required_stoa(&parsed) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let target = match required_op_id(&parsed, "target") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let direction = match required_direction(&parsed) {
            Ok(v) => v,
            Err(e) => return e,
        };

        match crate::authoring::vote(log, key, stoa, target, direction) {
            Ok(published) => {
                deliver(&published.id);
                published_json(&published)
            }
            Err(refusal) => error_json(&refusal.to_string()),
        }
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
            verify_authored_op(&author, &key.public_key().to_bytes(), b"a post", &sig.to_bytes()),
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
        let out = get_capabilities(
            &format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()),
            |_| panic!("the keystore layer exploded"),
        );
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
        let with_order = list_threads(
            &feed_request(r#""order":"top""#),
            &log,
            &feed_genesis(),
        );
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
        let out = list_threads(
            &feed_request(r#""perPage":1000000"#),
            &log,
            &feed_genesis(),
        );
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
        assert_ne!(out, empty, "empty and unreadable must never be the same reply");
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
        let out = list_threads_from_request(&full_request(), || {
            Ok::<_, crate::log::OpLogError>(log)
        });
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
            (format!(r#"{{"stoa":"{stoa}","genesis":7}}"#), "must be a string"),
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
        // NO SPEC: the spec requires `author`/`identity`/`key` to be refused on
        // any publish, and `thread` on a REPLY. Refusing every name on all three
        // operations is chosen — a caller who sent one has the same wrong model
        // whichever operation it reached — and is what this test pins.
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
        // What this test **cannot** see, and saying so is the point: that the
        // append happened BEFORE the sink was called. The sink cannot read the
        // log to check, because the handler holds it mutably for the duration —
        // so no test through this API can observe the ordering directly. What
        // pins it instead is that `crate::authoring::publish` returns a
        // `Published` only after its `append` has returned `Ok`, and the sink
        // sits after that call. The structural argument is the evidence; this is
        // the observable half.
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

    #[test]
    fn a_publish_whose_delivery_panics_still_reports_the_op_as_published() {
        // A declined handoff leaves the op published. A panicking sink is the
        // most violent decline available, and the guard turns it into the error
        // shape — but the requirement is about the LOG, so what this pins is
        // that the op stays: the append completed before delivery was reached
        // and nothing rolls it back.
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
        // The guard caught it, so this is an error shape rather than an aborted
        // process — but the op is published either way.
        as_json(&out);
        assert!(
            log.get(&expected).unwrap().is_some(),
            "a declined handoff must leave the op in the log, got {out}"
        );
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn a_delivery_that_reports_nothing_and_one_that_reports_promptly_give_one_reply() {
        // "A publish returns while delivery is still outstanding" — the reply is
        // the same either way, because the sink returns nothing and there is no
        // outcome to wait for.
        let key = publish_key();
        let request = publish_request(r#""body":"whatever delivery does""#);

        let mut silent_log = MemoryOpLog::new();
        let silent = publish_post(
            &request,
            &mut silent_log,
            &key,
            &mut |_: &crate::op::OpId| {},
        );

        let mut prompt_log = MemoryOpLog::new();
        let mut reported = false;
        let prompt = publish_post(
            &request,
            &mut prompt_log,
            &key,
            &mut |_: &crate::op::OpId| reported = true,
        );

        assert!(reported, "the prompt sink must actually have been called");
        assert_eq!(
            silent, prompt,
            "the reply must not depend on what delivery did"
        );
        assert!(as_json(&silent).get("error").is_none(), "got {silent}");
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
            }
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
        // The adapter assembles a keystore, a key, a store and a delivery sink
        // once and dispatches over the three handlers — so all three must be
        // usable through ONE function pointer type, with the sink as a
        // `&mut dyn FnMut`.
        //
        // This is the only gate that can check that. The adapter lives in the
        // module crate behind `cfg(logos_scaffold)`, which no `cargo test` ever
        // sets (`lib.rs` explains why at length), so a coercion that failed
        // there would fail in the BUILDER's build — the one that runs last and
        // reports worst.
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
    fn every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape() {
        // The wire contract is only useful if it holds for EVERY method, so
        // check the property rather than each method's happy path again.
        let mut publish_log = MemoryOpLog::new();
        let key = publish_key();
        for out in [
            version("1.0.0"),
            ping(r#"{"payload":1}"#),
            ping("garbage"),
            panic_probe("{}"),
            get_capabilities(&format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()), |_| Ok(
                "abcd".to_string()
            )),
            get_capabilities("garbage", |_| Ok("abcd".to_string())),
            list_threads(&feed_request(""), &log_with_body("hello"), &feed_genesis()),
            list_threads("garbage", &log_with_body("hello"), &feed_genesis()),
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
    }
}
