//! Sets one cfg: whether the builder's generated scaffold is present.
//!
//! The Logos Rust path deliberately needs no `build.rs` — the builder generates
//! `generated/provider_gen.rs` and stages the SDK itself. This one does not
//! generate anything and must not: it only *observes* whether the generated
//! file is there, so `src/lib.rs` can gate the module surface on it.
//!
//! Why that gate is needed at all: without it, `cargo test` fails to compile
//! the crate ("couldn't read .../generated/provider_gen.rs") and there is no
//! test layer at all outside Nix. With it, `cargo test` compiles `src/core.rs`
//! — the part that holds every decision — and the builder still compiles the
//! full surface, because the file is there when the builder runs.
//!
//! Do not "improve" this into generating a stub scaffold when the file is
//! missing. A stub would let `cargo test` compile the adapter against a
//! contract nothing derived, which is a green gate that cannot see the thing it
//! claims to check.

use std::path::Path;

fn main() {
    // Cargo 1.80+ requires custom cfgs to be declared, or it warns on every
    // build. Declaring it also makes `logos_scaffold` greppable as a real
    // build input rather than a magic string.
    println!("cargo::rustc-check-cfg=cfg(logos_scaffold)");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is always set by cargo");
    let scaffold = Path::new(&manifest_dir).join("generated").join("provider_gen.rs");

    // Re-run when the scaffold appears or vanishes. Without this, a cached
    // build from a plain checkout would keep the cfg off even once the builder
    // staged the file — which would silently produce a staticlib exporting
    // none of the `logos_module_*` symbols, linking clean and failing at
    // dlopen.
    println!("cargo::rerun-if-changed={}", scaffold.display());

    if scaffold.exists() {
        println!("cargo::rustc-cfg=logos_scaffold");
    }
}
