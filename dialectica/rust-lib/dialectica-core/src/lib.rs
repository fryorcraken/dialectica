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

pub mod identity;
pub mod wire;

// The wire surface is re-exported at the crate root because it IS the module's
// contract — the adapter calls these by name, and a caller should not have to
// know which submodule a handler happens to live in.
pub use wire::{
    callee_error, channel_exists_reply, error_json, guarded, panic_probe, parse_channel_id, ping,
    version,
};
