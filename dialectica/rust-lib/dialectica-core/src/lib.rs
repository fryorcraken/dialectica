//! Everything dialectica actually decides, with ZERO SDK types.
//!
//! This split is forced, not stylistic (PLAN.md §2.3). The module crate's
//! `lib.rs` `include!`s a scaffold the *builder* generates, and that scaffold
//! calls `lp_*` symbols which are undefined outside a real module image. So
//! that file cannot be compiled by `cargo test`, and anything reachable only
//! from it is untestable.
//!
//! This crate is the answer, and PLAN.md §9's Phase 1 "pure inner crate". It
//! holds the guard, the handler bodies and the forum's semantics; it depends on
//! no SDK crate at all, so the constraint is enforced by the compiler rather
//! than by review. The module crate is a thin adapter that forwards to it.

// Private: a shared decoding primitive, not part of the module's contract.
// §2.5 says widening the surface is a deliberate act, and a read head is an
// implementation detail every decoder happens to share.
mod cursor;

pub mod arrival;
pub mod authoring;
pub mod feed;
pub mod identity;
pub mod identity_store;
pub mod keystore;
pub mod log;
pub mod membership;
pub mod moderation;
pub mod onboarding;
pub mod op;
pub mod revision;
pub mod sanitise;
pub mod stoa;
pub mod thread;
pub mod transport;
pub mod wire;

// The wire surface is re-exported at the crate root because it IS the module's
// contract — the adapter calls these by name, and a caller should not have to
// know which submodule a handler happens to live in.
// `Request`, `REQUEST_NOT_AN_OBJECT` and `MAX_REQUEST_BYTES` are here for the
// same reason the handlers are: the envelope IS the contract's request half. A
// handler added in this crate reaches `Request::parse` as the only way to get a
// parsed request, which is how both the object check and the size cap are
// inherited rather than remembered. `Request` itself lives in `wire::request`
// rather than in `wire` — a file with no handler in it, so its private field is
// private to somewhere a handler cannot reach.
pub use wire::{
    callee_error, channel_exists_reply, create_stoa, error_json, generate_identity_slate,
    get_capabilities, get_capabilities_from_stores, guarded, join_stoa, keep_identity, list_stoas,
    list_threads, list_threads_from_request, membership_path_in, no_identity, panic_probe,
    parse_channel_id, ping, posting_identity, publish_post, publish_reply, publish_vote,
    read_thread, read_thread_from_request, stoa_of, version, who_am_i, with_membership_store,
    with_membership_store_read, KeepTargets, OnboardingSession, Request, MAX_REQUEST_BYTES,
    REQUEST_NOT_AN_OBJECT,
};
