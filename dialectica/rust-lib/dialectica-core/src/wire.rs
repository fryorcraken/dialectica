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

/// Parse a request into the JSON **object** every method's contract specifies.
///
/// # The defect this exists to close, found by a test and not by review
///
/// Every handler used to open with `serde_json::from_str::<Value>` and then reach
/// for its fields with `.get(..)`. That is correct for an object and **silently
/// wrong for every other JSON value**: `serde_json::Value::get` returns `None` on
/// an array, a number, a string or `null`, which is indistinguishable from an
/// object that simply lacks the field.
///
/// For a handler whose fields are all required the consequence was merely a
/// confusing message — `list_stoas("[]")` reported nothing wrong at all and
/// **served a successful page**, because `page` and `perPage` are optional and
/// both defaulted. A caller sending an array got a reply that looked like an
/// answer to a question it never asked.
///
/// **Why no existing test caught it, which is the part worth recording.** Every
/// hostile-input fixture in this file used either `"not json"` — unparseable, so
/// it dies at `from_str` — or `{}`, an object missing its fields. Both are refused
/// whether or not the request is checked for being an object, so the two
/// explanations produce the same answer on every fixture. That is this project's
/// one test-defect family verbatim, and the integration suite's blanket sweep
/// found it by including `[]`, `7` and `null`.
///
/// # `null` is refused rather than treated as an empty object
///
/// A caller sending `null` has not sent an empty request; they have sent a value
/// meaning "nothing", and answering as though they had sent `{}` is guessing. The
/// handlers whose every field is optional are exactly the ones where that guess
/// would be invisible, which is why it is refused here rather than per handler.
/// Returns the whole [`serde_json::Value`] rather than the inner
/// [`serde_json::Map`], so that the existing field parsers — which take a
/// `&Value` and call `.get` — need no signature change. What the caller gains is
/// the guarantee that `.get` now means what it reads as.
fn parse_request(request: &str) -> Result<serde_json::Value, String> {
    match serde_json::from_str::<serde_json::Value>(request) {
        Ok(v @ serde_json::Value::Object(_)) => Ok(v),
        // Named by what arrived, so a caller sending an array is told they sent an
        // array rather than being told a field is missing from it.
        //
        // The object arm is spelled rather than left to an `unreachable!`, even
        // though the arm above has already taken it: an `unreachable!` is a panic,
        // a panic aborts the module process (PHASE0-FINDINGS §3), and a
        // "cannot happen" that happens is precisely when that matters. It costs one
        // line and removes a panic from the boundary every request crosses.
        Ok(other) => Err(error_json(&format!(
            "a request must be a JSON object, got {}",
            match other {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "a boolean",
                serde_json::Value::Number(_) => "a number",
                serde_json::Value::String(_) => "a string",
                serde_json::Value::Array(_) => "an array",
                serde_json::Value::Object(_) => "an object",
            }
        ))),
        Err(e) => Err(error_json(&format!("invalid JSON: {e}"))),
    }
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
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
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
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
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
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
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
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
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
    // `?` rather than a match: `parse_request`'s error arm is already the wire
    // reply, which is the whole reason this function's `Result` is `String` on both
    // sides.
    let parsed = parse_request(request)?;
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

// ─── The write path ───────────────────────────────────────────────────────
//
// PLAN.md §9.1 Stage B and Stage D, plus the identity and Stoa creation the MVP
// needs before either. Every handler follows §2.5 without exception: JSON in,
// JSON out, `{"error":"..."}` as the only failure, never a partial success.
//
// THE SHARED SHAPE OF A PUBLISH REPLY is `{"op":"<hex op id>"}`, which is what
// §9.1 specifies (`createPost(...) -> {op}`). It is an op ID and not a success
// flag, because a caller genuinely needs it: a reply names its parent by op id,
// and a view that had to re-read the feed to discover what it just posted would
// be doing a second call to learn something the first one knew.
//
// WHAT NO HANDLER HERE DOES IS SEND. Every one stores locally and returns. See
// `publish.rs` on why that seam is the right way round, and on what a transport
// layer would add.

/// A required hex field, parsed into an [`crate::identity::Address`].
///
/// Its own function because five handlers need it and `list_threads` already
/// spelled it inline once. A present-but-wrong-typed field, an absent one, and an
/// unparseable one are three different mistakes reported by name — the rule
/// `parse_channel_id` set and every field parser here follows.
fn parse_address(
    parsed: &serde_json::Value,
    field: &str,
) -> Result<crate::identity::Address, String> {
    match parsed.get(field) {
        Some(serde_json::Value::String(s)) => crate::identity::Address::from_hex(s)
            .map_err(|e| error_json(&format!("{field}: {e}"))),
        Some(_) => Err(error_json(&format!("{field} must be a string"))),
        None => Err(error_json(&format!("missing field: {field}"))),
    }
}

/// A required hex field, parsed into an [`crate::op::OpId`].
///
/// Separate from [`parse_address`] rather than generic over the two, because they
/// are different types with different error vocabularies and an op id being
/// mistaken for an address is exactly what the domain separation in `op.rs`
/// exists to prevent. A shared generic would be one function that could return
/// either, which is the confusion, not the fix.
fn parse_op_id(parsed: &serde_json::Value, field: &str) -> Result<crate::op::OpId, String> {
    match parsed.get(field) {
        Some(serde_json::Value::String(s)) => {
            crate::op::OpId::from_hex(s).map_err(|e| error_json(&format!("{field}: {e}")))
        }
        Some(_) => Err(error_json(&format!("{field} must be a string"))),
        None => Err(error_json(&format!("missing field: {field}"))),
    }
}

/// A required string field.
///
/// **Absent and empty are different**, and this distinguishes them: a missing
/// `body` is a caller bug, while an empty one is a post someone may legitimately
/// have written (`op.rs` round-trips an empty body deliberately). So this refuses
/// the first and returns the second.
fn parse_string(parsed: &serde_json::Value, field: &str) -> Result<String, String> {
    match parsed.get(field) {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(error_json(&format!("{field} must be a string"))),
        None => Err(error_json(&format!("missing field: {field}"))),
    }
}

/// An optional list of strings, defaulting to empty.
///
/// Optional because most posts have none, and §4.6 makes attachments structurally
/// optional. **A non-array, or an array holding a non-string, is refused rather
/// than filtered** — silently dropping a malformed element would publish a post
/// missing an attachment the author attached, which they would discover only by
/// looking.
fn parse_string_list(parsed: &serde_json::Value, field: &str) -> Result<Vec<String>, String> {
    match parsed.get(field) {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    serde_json::Value::String(s) => out.push(s.clone()),
                    _ => {
                        return Err(error_json(&format!(
                            "every entry in {field} must be a string"
                        )))
                    }
                }
            }
            Ok(out)
        }
        Some(_) => Err(error_json(&format!("{field} must be an array of strings"))),
    }
}

/// The one publish reply shape, built in one place.
fn published_json(op: &crate::op::SignedOp) -> String {
    serde_json::json!({ "op": op.op.id().to_hex() }).to_string()
}

// ─── Where the signing identity comes from, and why it is not here ────────
//
// EVERY HANDLER BELOW TAKES A KEY AND NEVER GOES LOOKING FOR ONE. `create_stoa`
// takes a `&PublicKey` (it signs nothing — see `publish::create_stoa`); each
// publishing handler takes a `&SecretKey`.
//
// That is a seam rather than an omission. **There is deliberately no
// `create_identity` here**: minting a secret, writing the keystore, and deciding
// what key material may leave core are separately owned, and they are the root of
// the dependency tree rather than a leaf of this one. This half of the write path
// depends only on the narrowest thing it actually needs — "a key that can sign" —
// which `identity.rs` already supplies as a merged, stable type.
//
// What that costs a caller is one argument. What it buys is that this file has no
// opinion about where a key lives, so a keystore-backed provider, an agent-backed
// one, or a test's `SecretKey::generate()` are all the same to it, and none of them
// is a change to any function here.
//
// **One identity per user for this MVP**: the key passed in is the root key, and
// `identity::derive_stoa_key` is deliberately NOT called anywhere in the write
// path. The cost is §5.2's cross-Stoa unlinkability — one author address appears in
// every Stoa the user posts in, so a reader holding ops from two Stoas can link the
// same person across both. That is a privacy property deferred on purpose. Restoring
// it is a change to what the CALLER passes, not to this file.

/// `{"title":"…"}` -> `{"stoa":"<hex>","title":"…","genesis":"<hex>"}`.
///
/// # The genesis record is returned, and it has to be
///
/// A Stoa's address is a one-way hash of its record, so a peer given only the
/// address cannot recover the record — and every read needs the record, because
/// [`crate::moderation::Moderators::of`] takes one. So a creator sharing their
/// Stoa must share both, and a reply that returned only the address would have
/// handed them half of what a joiner needs.
///
/// This is the same pairing `list_threads` already requires in its request, and
/// it is why [`crate::publish::join_stoa`] takes a record: the address
/// authenticates the record, and the record is the thing with the content.
pub fn create_stoa<S: crate::log::StoaRegistry>(
    request: &str,
    store: &mut S,
    creator: &crate::identity::PublicKey,
) -> String {
    guarded("create_stoa", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let title = match parse_string(&parsed, "title") {
            Ok(t) => t,
            Err(e) => return e,
        };
        match crate::publish::create_stoa(store, creator, &title) {
            Ok(joined) => joined_stoa_json(&joined),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// `{"stoa":"<hex>","genesis":"<hex>"}` -> the joined Stoa.
///
/// # Verifies rather than trusts, and the check is not local to this function
///
/// §4.8's self-authenticating property: the address is the hash of the record, so
/// a tampered record cannot pass for the Stoa an address names.
/// [`crate::publish::join_stoa`] makes the check, and it is there rather than here
/// so that a second caller of the write path cannot reach a join that skipped it.
///
/// **In-post addresses are attacker-supplied content** (§4.8), which is what makes
/// this a security surface rather than a form. What core can enforce is that the
/// record matches; what it cannot enforce is that the user meant to join — nothing
/// auto-joins, and the UI brief's obligation 2b owns the rest, because a
/// confirmation showing only a title has shown the forgeable half.
pub fn join_stoa<S: crate::log::StoaRegistry>(request: &str, store: &mut S) -> String {
    guarded("join_stoa", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        // `genesis_for` decodes AND checks the record against the address, which
        // is the same function `list_threads` uses. One decoder, one check.
        let genesis = match genesis_for(&parsed, &stoa) {
            Ok(g) => g,
            Err(e) => return e,
        };
        match crate::publish::join_stoa(store, &stoa, &genesis) {
            Ok(joined) => joined_stoa_json(&joined),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// One Stoa as the view receives it.
///
/// # `title` is the GENESIS title, and `isGenesisFallback` says so
///
/// §5.7 makes a reader prefer the latest valid `StoaMetadata` op and fall back to
/// the genesis title. **Nothing resolves that op** — PLAN.md §9.1 names it as a
/// core gap and it is still open — so every title here is the founding one, and
/// `isGenesisFallback` is unconditionally `true`.
///
/// Reported rather than omitted, because §9.1 is explicit that the two are
/// "different epistemic states": a founding title may be years stale, and a view
/// that cannot tell presents it as current. A field that is always `true` today
/// becomes meaningful the moment metadata resolution lands, and a view written
/// against it needs no change then.
///
/// `description` is absent rather than empty, and that is not an oversight: a
/// description lives ONLY in a metadata op (the genesis record deliberately has no
/// such field, so that re-wording does not mint a new Stoa). With nothing
/// resolving those ops there is no description to report, and an empty string
/// would be a claim that one exists and is blank.
fn joined_stoa_json(joined: &crate::log::JoinedStoa) -> String {
    let address = match joined.address() {
        Ok(a) => a,
        // A record in the store that cannot be encoded is a corrupt store, not a
        // Stoa with no address. Reported rather than rendered.
        Err(e) => return error_json(&format!("genesis record: {e}")),
    };
    serde_json::json!({
        "stoa": address.to_hex(),
        "title": joined.genesis.title,
        // The record itself, hex, so a creator can share what a joiner needs.
        "genesis": hex::encode(match joined.genesis.canonical_bytes() {
            Ok(b) => b,
            Err(e) => return error_json(&format!("genesis record: {e}")),
        }),
        "relation": joined.relation.as_str(),
        "isGenesisFallback": true,
    })
    .to_string()
}

/// `{"page":N,"perPage":N}` -> the Stoas this peer created or joined.
///
/// The ecosystem's pagination shape, over a set that is small by nature — a user
/// joins a handful of forums. Paginated anyway, because §2.5 makes it the shape
/// for a list and a method that returned a bare array would be the one exception a
/// view had to special-case.
///
/// **Not derived from the op log.** A Stoa this peer joined and nobody has posted
/// in must still appear, and an op gossiped for a Stoa this peer never joined must
/// not — see the `stoas` table's own comment. `iter_stoa` answers a different
/// question.
pub fn list_stoas<S: crate::log::StoaRegistry>(request: &str, store: &S) -> String {
    guarded("list_stoas", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
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

        let all = match store.list_stoas() {
            Ok(v) => v,
            // §11.1 obligation 5, at a second boundary: a storage failure is the
            // error shape and NEVER an empty list. "You are in no Stoas" and "I
            // could not read the store" render identically and mean opposite
            // things — and this is the FIRST screen a user sees, so collapsing
            // them here presents a broken peer as a new one.
            Err(e) => return error_json(&e.to_string()),
        };

        let start = page.saturating_mul(per_page).min(all.len());
        let end = start.saturating_add(per_page).min(all.len());
        let has_more = end < all.len();

        // Each row is built by the same function the single-Stoa replies use, so
        // a list row and a join reply cannot describe one Stoa differently.
        // Re-parsed from its JSON string because that function returns the wire
        // form; the alternative is a second builder returning a `Value`, which is
        // the duplication this avoids.
        let mut items = Vec::with_capacity(end.saturating_sub(start));
        for joined in &all[start..end] {
            let row: serde_json::Value = match serde_json::from_str(&joined_stoa_json(joined)) {
                Ok(v) => v,
                Err(e) => return error_json(&format!("a stored Stoa could not be rendered: {e}")),
            };
            // A row that came back as an error is a corrupt record, and it must
            // not be paged over silently — §2.5 forbids a reply that is partly a
            // success, and a list with one error object inside it is exactly that.
            if row.get("error").is_some() {
                return row.to_string();
            }
            items.push(row);
        }

        serde_json::json!({ "items": items, "page": page, "hasMore": has_more }).to_string()
    })
}

/// `{"stoa":"…","body":"…","attachments":[…]}` -> `{"op":"<hex>"}`.
pub fn create_post<S: crate::log::OpLog + crate::log::StoaRegistry>(
    request: &str,
    store: &mut S,
    key: &crate::identity::SecretKey,
) -> String {
    guarded("create_post", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        let body = match parse_string(&parsed, "body") {
            Ok(b) => b,
            Err(e) => return e,
        };
        let attachments = match parse_string_list(&parsed, "attachments") {
            Ok(a) => a,
            Err(e) => return e,
        };
        match crate::publish::create_post(store, key, &stoa, &body, &attachments) {
            Ok(op) => published_json(&op),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// `{"stoa":"…","parent":"…","body":"…","attachments":[…]}` -> `{"op":"<hex>"}`.
///
/// **There is no `thread` argument**, and §9.1's sketch has one. It is omitted
/// deliberately: the thread is a function of the parent, so accepting it would let
/// a caller file a reply under a thread it does not belong to — see
/// [`crate::publish::create_reply`], which derives it. An argument that must always
/// equal something core can compute is an argument core should compute.
pub fn create_reply<S: crate::log::OpLog + crate::log::StoaRegistry>(
    request: &str,
    store: &mut S,
    key: &crate::identity::SecretKey,
) -> String {
    guarded("create_reply", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        let parent = match parse_op_id(&parsed, "parent") {
            Ok(p) => p,
            Err(e) => return e,
        };
        let body = match parse_string(&parsed, "body") {
            Ok(b) => b,
            Err(e) => return e,
        };
        let attachments = match parse_string_list(&parsed, "attachments") {
            Ok(a) => a,
            Err(e) => return e,
        };
        match crate::publish::create_reply(store, key, &stoa, &parent, &body, &attachments) {
            Ok(op) => published_json(&op),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// `{"stoa":"…","target":"…","direction":"up"|"down"}` -> `{"op":"<hex>"}`.
///
/// # The direction is a NAME, not a number
///
/// `"up"` and `"down"`, refused if anything else. The wire format's discriminants
/// are 0 and 1 (`op.rs`), and exposing those at the JSON boundary would couple a
/// view to a byte value whose whole point is that it is internal — and would make
/// a caller's off-by-one a silent downvote rather than an error.
///
/// **Never defaulted.** An unrecognised direction is refused, for the reason
/// `stoa.rs` refuses an unknown policy: guessing which the caller meant is how a
/// downvote is recorded as an upvote, and a vote is attributed to a person.
///
/// # A vote today changes nothing a reader can see
///
/// §7.2 rule 2 ships no score and nothing reads `Vote` ops. This method stores
/// history for when scoring lands — see [`crate::publish::create_vote`] for the
/// argument and for PLAN.md's recorded objection to the method existing at all.
/// A view offering a vote control must not imply an effect that is not there.
pub fn create_vote<S: crate::log::OpLog + crate::log::StoaRegistry>(
    request: &str,
    store: &mut S,
    key: &crate::identity::SecretKey,
) -> String {
    guarded("create_vote", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        let target = match parse_op_id(&parsed, "target") {
            Ok(t) => t,
            Err(e) => return e,
        };
        let direction = match parsed.get("direction") {
            Some(serde_json::Value::String(s)) if s == "up" => crate::op::VoteDirection::Up,
            Some(serde_json::Value::String(s)) if s == "down" => crate::op::VoteDirection::Down,
            Some(serde_json::Value::String(_)) => {
                return error_json("direction must be \"up\" or \"down\"")
            }
            Some(_) => return error_json("direction must be a string"),
            None => return error_json("missing field: direction"),
        };
        match crate::publish::create_vote(store, key, &stoa, &target, direction) {
            Ok(op) => published_json(&op),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// `{"stoa":"…","genesis":"…","thread":"…","page":N,"perPage":N,"includeHidden":b}`
/// -> one page of a thread.
///
/// The pagination shape, like the feed. The genesis record is a parameter for the
/// same reason it is there — [`crate::moderation::Moderators::of`] needs one and
/// there is nowhere else to get it, and a caller without one must not get a thread
/// with moderation silently not applied.
///
/// A storage failure is the error shape and never an empty thread, which is
/// §11.1 obligation 5 at a third boundary. An empty thread is a real answer — a
/// thread whose root has not reached this peer — and it must not be reachable by
/// a read that failed.
pub fn get_thread<L: crate::log::OpLog>(
    request: &str,
    log: &L,
    genesis: &crate::stoa::Genesis,
) -> String {
    guarded("get_thread", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        // The genesis record must describe the Stoa asked for, or the moderator
        // set being applied governs a different forum than the posts being
        // filtered. `list_threads` makes the same check for the same reason.
        let genesis_address = match genesis.address() {
            Ok(a) => a,
            Err(e) => return error_json(&format!("genesis: {e}")),
        };
        if genesis_address != stoa {
            return error_json(
                "the genesis record does not describe the Stoa this thread was asked for",
            );
        }
        let thread = match parse_op_id(&parsed, "thread") {
            Ok(t) => t,
            Err(e) => return e,
        };
        let moderators = match crate::moderation::Moderators::of(genesis) {
            Ok(m) => m,
            Err(e) => return error_json(&format!("genesis: {e}")),
        };
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

        match crate::publish::read_thread(
            log,
            &moderators,
            &stoa,
            &thread,
            page,
            per_page,
            include_hidden,
        ) {
            Ok(page) => thread_page_json(&page),
            Err(e) => error_json(&e.to_string()),
        }
    })
}

/// The thread handler as the module actually calls it: genesis record as hex.
///
/// [`get_thread`] takes a decoded [`Genesis`](crate::stoa::Genesis) because that is
/// the shape worth testing directly — threading a hex string through every fixture
/// would be asserting on decoding at the same time. This is the form the adapter
/// forwards to, and it is thin on purpose: parse, verify, delegate.
///
/// **It exists because the adapter must stay one line per method.** The module
/// crate's own rule is that "if a body here ever grows past one line, that logic
/// belongs in `core` — otherwise it is logic no test can reach", and the adapter
/// cannot be compiled by `cargo test` at all. Doing this parse there would have put
/// eighteen untestable lines in the one file no test reaches;
/// [`list_threads_from_request`] exists for exactly the same reason and this is its
/// counterpart.
pub fn get_thread_from_request<L: crate::log::OpLog>(request: &str, log: &L) -> String {
    guarded("get_thread", || {
        let parsed = match parse_request(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let stoa = match parse_address(&parsed, "stoa") {
            Ok(a) => a,
            Err(e) => return e,
        };
        // Decodes the record AND checks it against the address — the same
        // self-authenticating check `list_threads_from_request` makes, through the
        // same function, so the two cannot disagree about what a valid pairing is.
        let genesis = match genesis_for(&parsed, &stoa) {
            Ok(g) => g,
            Err(e) => return e,
        };
        get_thread(request, log, &genesis)
    })
}

/// The thread pagination shape, built in one place.
fn thread_page_json(page: &crate::publish::ThreadPage) -> String {
    let items: Vec<serde_json::Value> = page
        .items
        .iter()
        .map(|row| {
            let mut obj = serde_json::json!({
                "post": row.post,
                "currentVersion": row.current_version,
                "author": row.author,
                "body": sanitised_json(&row.body),
                "attachments": row.attachments.iter().map(sanitised_json).collect::<Vec<_>>(),
                "isRevised": row.is_revised,
                "isHidden": row.is_hidden,
            });
            // ABSENT, not null-and-present, for the thread root. §9.1 is explicit
            // about the analogous `decidedBy`: "a field that is sometimes
            // meaningless is [a partly-successful shape] in miniature." A root has
            // no parent, so the key is not there.
            if let Some(parent) = &row.parent {
                obj["parent"] = serde_json::json!(parent);
            }
            obj
        })
        .collect();
    serde_json::json!({
        "items": items,
        "page": page.page,
        "hasMore": page.has_more,
    })
    .to_string()
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

    #[test]
    fn a_request_that_is_valid_json_but_not_an_object_is_refused() {
        // THE REGRESSION TEST FOR A DEFECT THE INTEGRATION SUITE FOUND, and it is
        // worth stating exactly why nothing here caught it before.
        //
        // Every hostile-input fixture in this file used `"not json"` — which dies
        // at the parse — or `{}`, an object missing its fields. Both are refused
        // whether or not a handler checks that the request IS an object, so the two
        // explanations produced the same answer on every fixture. That is this
        // project's one test-defect family, and `[]` is the input that separates
        // them.
        //
        // The consequence was not cosmetic. `serde_json::Value::get` returns `None`
        // on an array, which is indistinguishable from a missing field — so for a
        // handler whose every field is OPTIONAL, an array parsed as an empty
        // request and was SERVED. `list_stoas("[]")` returned a successful page.
        //
        // `null` is included deliberately: it is the value most likely to be
        // "helpfully" treated as an empty object by a later edit.
        for bad in ["[]", "null", "7", r#""a string""#, "true", "[1,2,3]"] {
            let out = parse_request(bad)
                .expect_err(&format!("{bad} must be refused as not an object"));
            let v: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("the refusal must be valid JSON ({e}): {out}"));
            assert!(v.get("error").is_some(), "for {bad}, got {out}");
            // The message must name what arrived rather than claiming a field is
            // missing — which is what sent a caller looking in the wrong place.
            assert!(
                v["error"].as_str().unwrap().contains("must be a JSON object"),
                "for {bad}, got {out}"
            );
        }

        // An object is accepted, so the check is not refuse-everything. Both the
        // empty object and a populated one, because the empty one is the case a
        // over-eager check would catch.
        for good in [r#"{}"#, r#"{"stoa":"ab"}"#] {
            assert!(parse_request(good).is_ok(), "{good} must be accepted");
        }

        // And through a real handler, which is where it mattered: the one whose
        // fields are all optional and which therefore served an answer.
        let out = list_stoas("[]", &MemoryOpLog::new());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some(), "got {out}");
        assert!(
            v.get("items").is_none(),
            "a failure must never also carry a result — §2.5: {out}"
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
            get_capabilities(&format!(r#"{{"stoa":"{}"}}"#, a_stoa().to_hex()), |_| Ok(
                "abcd".to_string()
            )),
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
