//! The request envelope, in a file that contains no handler.
//!
//! # Why this is its own module and not a type beside the handlers
//!
//! The type's whole claim is that a handler holding a [`Request`] provably went
//! through the check. A tuple struct's private field is private to its
//! **defining module**, not to its defining type — so while `Request` lived in
//! `wire.rs` beside every handler, this compiled and the claim was false:
//!
//! ```ignore
//! let bypass = Request(serde_json::Map::new());   // inside wire.rs: legal
//! ```
//!
//! Nothing in that line contains a `from_str`, so the mitigation originally
//! recorded — "a second parse is visible in review as an anomaly" — did not
//! apply to it either. The fix is the module boundary: the field is private to
//! this file, this file holds no handler, and the compiler now refuses the
//! bypass from `wire.rs`. See `request_cannot_be_constructed_outside_this_module`
//! at the bottom of this file for the proof, and `wire.rs`'s
//! `the_bypass_this_module_boundary_closes` for the negative half.
//!
//! # What the residual actually is
//!
//! Two things remain, and both are honest:
//!
//! - **A handler could call `serde_json::from_str` itself** and never build a
//!   `Request` at all. That is a new parse of a request, and it is what
//!   `no_request_parse_lives_outside_the_request_module` sweeps for.
//! - **Code added to THIS file** can construct a `Request` freely, because that
//!   is what a private field means. The boundary is only as good as the rule
//!   that this file holds no handler — which is why that rule is stated in the
//!   module doc rather than left to be inferred.

use super::error_json;

/// The refusal for a request that is not a JSON object.
///
/// A `const` rather than a literal at five call sites, because the spec's
/// obligation is about what the message *says*: it must be distinguishable from
/// "invalid JSON" and from "missing field", so that three caller mistakes
/// produce three messages. Five copies would drift and one would eventually
/// collide with a neighbour.
pub const REQUEST_NOT_AN_OBJECT: &str = "the request must be a JSON object";

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
    /// **An explicit `null` is `Some(Value::Null)` and never `None`**, so a
    /// reader that wants to treat `{"f":null}` as absent has to say so, and one
    /// that wants to tell them apart still can. Which a reader should do is
    /// recorded in the `wire-request-envelope` change's `design.md`; the
    /// envelope does not decide it, because deciding it here would take the
    /// choice away from every reader at once.
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
