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
pub mod feed;
pub mod identity;
pub mod keystore;
pub mod log;
pub mod membership;
pub mod moderation;
pub mod op;
pub mod revision;
pub mod sanitise;
pub mod stoa;
pub mod wire;

// The wire surface is re-exported at the crate root because it IS the module's
// contract — the adapter calls these by name, and a caller should not have to
// know which submodule a handler happens to live in.
pub use wire::{
    callee_error, channel_exists_reply, create_stoa, error_json, get_capabilities, guarded,
    join_stoa, list_stoas, list_threads, list_threads_from_request, membership_path_in,
    panic_probe, parse_channel_id, ping, version, with_membership_store,
};
