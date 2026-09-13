//! The request envelope, in a file that contains no handler.
//!
//! # Why this is its own module and not a type beside the handlers
//!
//! The type's whole claim is that a handler holding a [`Request`] provably went
//! through the check. A tuple struct's private field is private to its
//! **defining module**, not to its defining type — so while `Request` lived in
//! `wire.rs` beside every handler, this compiled and the claim was false:
//!
//! `text`, not `ignore`: this block is illustrative prose showing code that no
//! longer compiles from `wire.rs` — the defect this module closed — so it is not
//! a doc-test anyone should compile or count. `ignore` still registers a
//! doc-test (reported as ignored), which made cargo run 507 tests where the
//! source declared 506, and CI's count-the-tests gate caught exactly that. The
//! same slip was fixed once already in `keystore.rs`.
//!
//! ```text
//! let bypass = Request(serde_json::Map::new());   // inside wire.rs: legal
//! ```
//!
//! Nothing in that line contains a `from_str`, so the mitigation originally
//! recorded — "a second parse is visible in review as an anomaly" — did not
//! apply to it either. The fix is the module boundary: the field is private to
//! this file, this file holds no handler, and the compiler now refuses the
//! bypass from `wire.rs`. See
//! `tests::request_is_constructible_here_because_this_module_defines_it` at the
//! bottom of this file for the positive half of the proof, and `wire.rs`'s
//! `the_bypass_this_module_boundary_closes` for the negative half.
//!
//! # What the residual actually is
//!
//! Two things remain, both honest, and the first is larger than it looks:
//!
//! - **A handler need not hold a `Request` at all.** It can call
//!   `serde_json::from_str` itself and read fields off a bare `Value`, and
//!   nothing in the type system objects — the boundary makes it impossible to
//!   hold a `Request` without the check, not impossible to skip the type.
//!   `wire.rs`'s `the_sixth_method_the_boundary_does_not_stop` builds exactly
//!   that method and demonstrates it serving an array. What stands against it is
//!   the sweep in `every_request_taking_method`, whose doc states the obligation,
//!   and review.
//! - **Code added to THIS file** can construct a `Request` freely, because that
//!   is what a private field means. The boundary is only as good as the rule
//!   that this file holds no handler — which is why that rule is stated in the
//!   module doc rather than left to be inferred.

use super::error_json;

/// The refusal for a request that is not a JSON object.
///
/// # Why a `const` for a string used in exactly one place
///
/// There is **one** production use of this, in [`Request::parse`] below — which
/// is the whole point of the type, so a "many call sites would drift" argument
/// does not apply and never did. (An earlier version of this comment made that
/// argument, citing five call sites. Five was the count under the per-handler
/// `if !parsed.is_object()` design this change **rejected**; see `design.md` §1.
/// An argument for a structure that does not exist is worse than no comment,
/// because it reads as a justification and a reader who checks it finds the code
/// disagreeing.)
///
/// The reason is that the string is **contract surface a view may render**. The
/// spec's obligation is about what the message *says*: it must be told apart from
/// "invalid JSON" and from "missing field", so that three caller mistakes produce
/// three messages. Naming it makes rewording it a deliberate act rather than an
/// edit to a literal, and
/// `super::tests::the_non_object_message_is_pinned_to_a_known_answer` pins it to
/// a hardcoded answer so the reword cannot pass unnoticed.
///
/// The test assertions referring to this constant are not call sites in the
/// drift sense — a test asserting the literal is the mechanism that *catches*
/// drift, not a copy that suffers it.
pub const REQUEST_NOT_AN_OBJECT: &str = "the request must be a JSON object";

/// The largest request this module will parse, in bytes.
///
/// # Why there is a cap at all
///
/// Measured, release build, against a valid feed request padded with one ignored
/// string field: **64 MiB in was accepted and served**, at 92.6 ms of CPU and
/// roughly 2N bytes of transient heap, for a 373-byte reply. The amplification is
/// *inverted* — the reply carries no signal that the request cost anything — so
/// nothing downstream can notice, rate-limit or log it. `ping` is sharper still,
/// because it echoes `payload`: 32 MiB in produced a 33,554,443-byte reply.
///
/// And the failure mode is not a slow reply. `docs/PHASE0-FINDINGS.md` §3
/// measured what a panic in a dispatch handler does: the module process
/// **aborts**, the caller waits out its 20-second timeout, and every later call
/// reports `MODULE_NOT_LOADED`. An allocation failure here is that, not an error
/// reply.
///
/// # Why this is the one place the check belongs
///
/// [`Request::parse`] is the only way a handler can reach a field, so one
/// comparison here bounds every request-taking method — including methods nobody
/// has written yet. That is the same argument the type itself is built on, one
/// step further along: put the invariant where the data is shaped, not in a
/// guard each handler has to remember.
///
/// # THE CAP IS WHAT BUYS THE LENIENCY, and the two decisions are one
///
/// Unknown fields are deliberately **ignored** rather than refused, because a
/// strict envelope means a newer view cannot talk to an older core — the worse
/// failure for two modules that update independently. But ignoring unknown
/// fields is exactly what made the 64 MiB padded request *valid* rather than
/// refused: every byte of that payload sat in a field no method reads.
///
/// So these are not two independent decisions. **Dropping this cap also costs
/// the leniency**, and anyone tempted to raise it a long way should price it as
/// a change to both.
///
/// # Why 4 MiB
///
/// Derived from what a legitimate request can carry, not picked for roundness.
/// The largest request this contract will ever hold is a composed op: §4.4's SDS
/// message cap gives [`crate::op`] a 150 KiB per-field bound, and `op.rs` records
/// that a `Post` with attachments decodes to **768,076 bytes** at those bounds —
/// measured there, not estimated. A request carrying that as JSON, with a
/// hex-encoded genesis record beside it (hex doubles), lands near 1.6 MB. 4 MiB
/// clears that with room for a field the future adds, and is **a factor of 16
/// below the 64 MiB that was served** (67,108,864 / 4,194,304 = 16).
///
/// That factor is the honest one and it is smaller than it sounds, which is why
/// it is written as arithmetic. An earlier version of this comment claimed
/// "three orders of magnitude below", which is wrong by ~60x and also
/// self-contradicting: three orders below 64 MiB is about 67 KiB, a cap that
/// would refuse the 1.6 MB legitimate request the paragraph above says must be
/// cleared. **The cap is bounded from both sides**, and the gap between the
/// largest legitimate request and the cap is the only headroom there is.
///
/// **It is deliberately one number rather than a per-method table.** A per-method
/// cap would be a second thing each new handler has to declare, which is the
/// guard-at-every-call-site shape this file exists to avoid; the tighter bounds
/// that actually matter are per *field*, and they live with the field.
///
/// Pinned by `the_request_cap_is_pinned_to_a_known_answer`, because a cap that
/// drifted upward would still refuse an absurd request and still pass every test
/// that probes only absurd values.
pub const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

/// A request that is known to be a JSON object, because it cannot be built from
/// anything else.
///
/// # Why this is a type and not a branch
///
/// `serde_json::Value::get` answers `None` for **every** non-object. So with a
/// bare `Value` in hand, `parsed.get("stoa")` cannot tell an array from an
/// object missing its `stoa` — and a handler whose fields are all optional
/// serves an array as a request that named nothing. That is the defect this
/// exists to close, and it is latent rather than reachable only because every
/// method on today's surface happens to require a field.
///
/// The fix could have been one `if !parsed.is_object()` per handler. It was not,
/// for a reason that outlives style: **a branch has to be got right at every
/// call site, and "is it checked everywhere?" is then a question you answer by
/// reading every handler.** A new method that forgets the branch compiles,
/// passes clippy, and silently reintroduces the defect.
///
/// With this type the question is answered by the compiler — **given the module
/// boundary this file exists to provide.** The inner map is private to this
/// module, [`Request::parse`] is the only constructor reachable from outside it,
/// and no handler lives here. A handler holding a `Request` therefore went
/// through the check, and a handler written next month inherits it without its
/// author knowing this change happened. See this module's own doc for what the
/// boundary does *not* cover.
///
/// # What it does NOT constrain
///
/// The envelope, not the fields. [`Request::get`] hands back a `&Value` and each
/// handler still decides what type it wanted; a wrong-typed field is that
/// handler's error to report, with its own message. An **unknown** field is
/// carried and ignored, which the contract states deliberately — strictness is a
/// forward-compatibility policy and not an envelope rule.
pub struct Request(serde_json::Map<String, serde_json::Value>);

impl Request {
    /// Parse a request string, refusing anything that is not a JSON object.
    ///
    /// The `Err` arm is already the wire reply, so a caller cannot invent a
    /// second error shape while converting one — the same convention
    /// [`super::parse_channel_id`] follows.
    ///
    /// **Two steps rather than one, on purpose.** Deserialising straight into a
    /// `Map` would let serde refuse an array for free, but its message
    /// (`invalid type: sequence, expected a map`) arrives through the same `Err`
    /// arm as a genuine parse failure and would be reported as `invalid JSON`.
    /// The spec requires those two be told apart, so the parse stays untyped and
    /// the type check is ours.
    ///
    /// **The recursion bound is serde's, and is being relied on.** A request
    /// nested a million deep is refused here with
    /// `invalid JSON: recursion limit exceeded` — not by any check written
    /// below, but by `serde_json`'s own 128-frame limit on `from_str`. That
    /// limit is a property of the dependency and would silently disappear under
    /// `disable_recursion_limit()` or a `from_reader` variant, so neither is to
    /// be introduced here without putting a depth check in its place.
    pub fn parse(request: &str) -> Result<Self, String> {
        // BEFORE the parse, and the ordering is the whole check. `from_str`
        // allocates roughly 2N bytes on the way to a `Value`, so a check that ran
        // afterwards would refuse the request having already paid for it — and
        // per PHASE0-FINDINGS §3 the price of failing to pay is an abort, not an
        // error reply. `an_oversized_request_is_refused_before_it_is_parsed`
        // pins the ordering by feeding in something that is both oversized and
        // unparseable and asserting which refusal comes back.
        if request.len() > MAX_REQUEST_BYTES {
            return Err(error_json(&format!(
                "the request is {} bytes, over the {MAX_REQUEST_BYTES} byte limit",
                request.len()
            )));
        }
        let parsed: serde_json::Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(e) => return Err(error_json(&format!("invalid JSON: {e}"))),
        };
        match parsed {
            serde_json::Value::Object(map) => Ok(Request(map)),
            // Every other variant, named by the `_` rather than enumerated:
            // an array, a number, a string, a boolean, `null`. All of them
            // answer `None` to every field read, which is the whole defect.
            _ => Err(error_json(REQUEST_NOT_AN_OBJECT)),
        }
    }

    /// A field, or `None` because the request genuinely lacks it.
    ///
    /// Unlike `Value::get`, a `None` here means exactly one thing: this object
    /// has no such key. That is the ambiguity the type removes.
    ///
    /// **An explicit `null` is `Some(Value::Null)` and never `None`**, and that
    /// is the contract's requirement rather than a convenience: a field holding
    /// an explicit `null` is present, not absent, and what happens to it follows
    /// from the field's declared type and optionality in one of three readings —
    /// carried through as a value, defaulted restrictively, or refused as a wrong
    /// type.
    ///
    /// **So the envelope preserves the distinction and decides none of it.**
    /// Collapsing a null here would take the choice away from all three readings
    /// at once — `ping`'s `<any>` payload would lose the value it is documented to
    /// carry, and a future field's reading would be fixed before anyone chose it.
    /// The readers say which reading applies; see `super::parse_index`'s doc for
    /// the limit on the defaulting one, which is the half that can become an
    /// authorisation bypass.
    pub fn get(&self, field: &str) -> Option<&serde_json::Value> {
        self.0.get(field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_constructible_here_because_this_module_defines_it() {
        // The POSITIVE half of the boundary claim, and the reason the negative
        // half is a compile-fail note rather than a test: a private field is
        // private to its defining MODULE, so this line is legal here and
        // illegal in `wire.rs`. Asserting it here is what proves the line in
        // `wire.rs` fails for the reason named rather than for a typo.
        //
        // Deliberately not a `#[cfg(test)]`-only constructor: adding one would
        // be a second way in, reachable from `wire.rs`'s tests, which is the
        // hole this module exists to close.
        let built_directly = Request(serde_json::Map::new());
        assert_eq!(built_directly.get("anything"), None);
    }

    /// A valid request of exactly `bytes` total length, padded with a field no
    /// method reads.
    ///
    /// The padding is a field rather than a giant key or a deep structure on
    /// purpose: an ignored field is the shape the measurement used, and it is the
    /// shape the leniency decision makes *valid*. If the cap were somehow
    /// checked after parsing, this would still be accepted.
    fn a_request_of_exactly(bytes: usize) -> String {
        let skeleton = r#"{"junk":""}"#;
        assert!(
            bytes >= skeleton.len(),
            "cannot build a request smaller than its own skeleton"
        );
        let padding = "x".repeat(bytes - skeleton.len());
        let built = format!(r#"{{"junk":"{padding}"}}"#);
        assert_eq!(built.len(), bytes, "the fixture must be the size it claims");
        built
    }

    #[test]
    fn a_request_at_the_cap_is_accepted_and_one_byte_over_is_refused() {
        // The boundary from BOTH sides, which is the only way a `>` that should
        // have been `>=` (or the reverse) is caught. `op.rs`'s field cap is
        // tested exactly this way, for the reason recorded there: a test that
        // probes only an absurd value passes against a cap that has silently
        // drifted, and against a cap off by one.
        let at_the_cap = a_request_of_exactly(MAX_REQUEST_BYTES);
        assert!(
            Request::parse(&at_the_cap).is_ok(),
            "a request of exactly the cap must be accepted"
        );

        let one_over = a_request_of_exactly(MAX_REQUEST_BYTES + 1);
        assert_eq!(
            Request::parse(&one_over).err(),
            Some(error_json(&format!(
                "the request is {} bytes, over the {MAX_REQUEST_BYTES} byte limit",
                MAX_REQUEST_BYTES + 1
            ))),
            "one byte over the cap must be refused, and say by how much"
        );
    }

    #[test]
    fn an_oversized_request_is_refused_before_it_is_parsed() {
        // The cap's whole job is bounding ALLOCATION, so "refused" is not
        // enough — it must be refused without the ~2N transient heap a parse
        // costs. That is not observable from a return value, so the assertion
        // is the one thing that IS: a request far over the cap that is ALSO
        // unparseable comes back with the size refusal and not with
        // `invalid JSON`. Only an implementation that checks length before
        // calling `from_str` can answer that way.
        //
        // Reversing the two checks in `parse` turns this red, which is the
        // mutation it exists to catch and the one a "refused: yes" assertion
        // cannot see.
        let oversized_and_unparseable = "{".repeat(MAX_REQUEST_BYTES + 1);
        let refused = Request::parse(&oversized_and_unparseable)
            .err()
            .expect("must be refused");
        assert!(
            refused.contains("over the"),
            "an oversized request must be refused for its size, got {refused}"
        );
        assert!(
            !refused.contains("invalid JSON"),
            "the size check must run BEFORE the parse, got {refused}"
        );
    }

    #[test]
    fn the_request_cap_is_pinned_to_a_known_answer() {
        // Hardcoded, not `assert_eq!(MAX_REQUEST_BYTES, 4 * 1024 * 1024)` read
        // back from the definition it is checking. A cap that drifted upward
        // would still refuse a 64 MiB request and still pass every test that
        // probes only absurd values — the defect family this project has
        // recorded three times.
        assert_eq!(MAX_REQUEST_BYTES, 4_194_304);
    }

    #[test]
    fn the_size_refusal_is_not_either_refusal_it_must_be_told_from() {
        // A fourth caller mistake joins the three the spec enumerates, so it
        // earns the same obligation: it must not read as one of the others.
        // Checked on the prefix rather than by `assert_ne!` on whole strings,
        // for the reason
        // `the_non_object_message_does_not_read_as_either_refusal_it_must_be_told_from`
        // records — a message spelled "invalid JSON: too big" compares unequal
        // and still tells the caller the wrong thing.
        let over = Request::parse(&a_request_of_exactly(MAX_REQUEST_BYTES + 1))
            .err()
            .expect("must be refused");
        for neighbour in ["invalid JSON", "missing field", REQUEST_NOT_AN_OBJECT] {
            assert!(
                !over.contains(neighbour),
                "the size refusal reads as {neighbour:?}: {over}"
            );
        }
    }

    #[test]
    fn the_only_reachable_constructor_refuses_a_non_object() {
        for not_an_object in ["[]", "7", r#""s""#, "true", "null"] {
            let refused = Request::parse(not_an_object).err();
            assert_eq!(
                refused,
                Some(error_json(REQUEST_NOT_AN_OBJECT)),
                "Request::parse accepted {not_an_object}"
            );
        }
    }
}
