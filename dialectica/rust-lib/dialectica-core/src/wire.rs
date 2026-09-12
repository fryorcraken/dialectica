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
pub fn get_capabilities(
    request: &str,
    lookup: impl Fn(&crate::identity::Address) -> Result<String, crate::keystore::KeystoreError>,
) -> String {
    guarded("get_capabilities", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
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
    guarded("list_threads", || {
        let parsed = match Request::parse(request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        list_threads_inner(&parsed, log, genesis)
    })
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
    parsed: &Request,
    stoa: &crate::identity::Address,
) -> Result<crate::stoa::Genesis, String> {
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
/// Separated out because `page` and `perPage` are the same parsing job with the
/// same three failure modes, and a second copy would eventually disagree with
/// the first about whether `-1` is an error or a zero.
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
            // `as_u64` refuses a negative and a fractional number — a page of -1
            // is not a page, and silently clamping it to 0 would serve the first
            // page to a caller who asked for something impossible.
            //
            // **It refuses by SPELLING rather than by value, and the message says
            // so.** `1e2` is JSON for exactly 100 and `0.0` for exactly 0 — both
            // non-negative, both whole — and `as_u64` returns `None` for each,
            // because serde parses any number carrying a `.` or an `e` as `f64`.
            // Several JSON serialisers emit `1e2` for 100, so this is a spelling a
            // legitimate caller can send.
            //
            // The message was "must be a non-negative whole number", which told
            // such a caller its 100 was not whole. Corrected to name the spelling,
            // which is what is actually being refused. The ACCEPTANCE is
            // deliberately unchanged: widening it to accept an exactly-integral
            // float is a contract question the spec does not answer, and it does
            // not belong in a commit about the envelope. Filed rather than fixed —
            // a caller told the truth can restring its number today.
            Some(v) => Ok(Some(v as usize)),
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

    /// A method named, and callable with a raw request string.
    type NamedMethod = (&'static str, fn(&str) -> String);

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
        vec![
            ("ping", ping_m),
            ("get_capabilities", caps_m),
            ("list_threads", feed_m),
            ("list_threads_from_request", feed_req_m),
            ("parse_channel_id", channel_m),
        ]
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
        for (name, method) in every_request_taking_method() {
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
        for (name, method) in every_request_taking_method() {
            let message = error_message(&method("{}"));
            assert_ne!(
                message, REQUEST_NOT_AN_OBJECT,
                "{name} refused `{{}}` for its shape rather than its missing field"
            );
            assert!(message.contains("missing field"), "{name}: got {message:?}");
        }
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
        // that earn it, against a hardcoded literal — because the reason this
        // was wrong for two years is that nothing read the message beside the
        // input that produced it.
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
    fn the_feed_path_parses_its_request_once() {
        // WHAT IS AND IS NOT ASSERTABLE HERE, said plainly.
        //
        // The double parse was: `list_threads_from_request` parsed the request to
        // read `stoa` and `genesis`, then handed the raw `&str` to `list_threads`,
        // which parsed it again. Both replies were identical, so NO assertion on
        // output can see the difference — which is why it survived on main.
        //
        // What CAN be asserted is the type, and it is asserted by the compiler
        // rather than by this test: `list_threads_inner` takes `&Request`, so
        // there is no `&str` in scope for a second parse to consume. Reverting the
        // signature to `&str` is what would let the bug back in, and that is a
        // compile-visible change to a private function rather than something this
        // test could catch.
        //
        // So this test's job is the narrower one the fix DID have to preserve:
        // that moving the parse and removing a nested `guarded` frame changed no
        // reply. Two entry points, three request shapes each, compared against
        // each other — because the refactor's whole claim is that these agree.
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
        // this file — a compile-fail assertion would have to be a
        // `compile_fail` doctest, and this crate's doctest run is empty by
        // design. Recording the verified error is therefore the whole proof, and
        // it is deliberately stated as such rather than dressed up as a test
        // that passes for a weaker reason.
        //
        // What IS testable, and is: the positive half lives in
        // `wire::request::tests::request_is_constructible_here_because_this_module_defines_it`,
        // which compiles the same line inside the defining module. Together they
        // say the refusal above is about the boundary and not about a typo.
        //
        // And the runtime half of the guarantee — that the only constructor
        // reachable from here refuses a non-object — is
        // `request_parse_is_the_only_way_to_reach_a_field_read`, below.
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
    fn request_parse_is_the_only_way_to_reach_a_field_read() {
        // The structural half of the fix, asserted as behaviour: a `Request`
        // cannot be built from a non-object, so a handler holding one cannot
        // have skipped the check. This is what makes the guard inherited by a
        // method nobody has written yet rather than something each author must
        // remember.
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
}
