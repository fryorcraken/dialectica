//! Dialectica core — the Rust-authored Logos module.
//!
//! Phase 0 scope: prove the `codegen.rust` path end to end (PLAN.md §9). The
//! contract below is deliberately trivial; what it exercises is the *path*.
//!
//! **The trait IS the contract.** `codegen.rust.trait` in `metadata.json` makes
//! the builder derive the `.lidl` from this trait at build time
//! (`logos-lidl-gen --from-rust src/lib.rs --trait DialecticaModule`), so there
//! is no committed `.lidl` that can drift from the code. Changing a signature
//! here changes the module's public API — which is exactly the deliberateness
//! PLAN.md §2.2 asks for.
//!
//! The trait must stay in THIS file, at the top level and outside any `cfg`:
//! the generator reads `src/lib.rs` as text with `syn`, so a trait moved into a
//! submodule or hidden behind a feature gate is a trait the contract loses.

// The pure inner crate holds every decision (PLAN.md §9). This crate holds the
// contract trait, the adapter, and nothing else worth testing — which is the
// arrangement §2.3 forces, since nothing in THIS file can be reached by
// `cargo test` at all.
//
// Gated to match its only consumers, the adapter impl below. Outside the
// builder that impl is `cfg`'d out and this import is genuinely unused, so an
// ungated `use` warns on every plain `cargo build` — and a warning that is
// expected is a warning nobody reads.
#[cfg(logos_scaffold)]
use dialectica_core as core;

// `RustModuleContext` is defined by the generated scaffold. The contract trait
// below mentions it and must stay ungated (the generator reads this file as
// text), so without the scaffold the trait would not compile and `cargo test`
// would be back where it started.
//
// This stand-in exists ONLY so the trait type-checks in a plain checkout. It is
// never constructed and never reached by a test — `on_context_ready` is
// defaulted framework plumbing, not contract surface. The fields mirror the
// real type's so a mismatch shows up here as a compile error in the builder's
// own build rather than as a runtime surprise.
//
// It is a struct rather than a type alias to `()` on purpose: an alias would
// let `ctx.instance_id` fail only in the gated build, which is the build that
// runs last and reports worst.
#[cfg(not(logos_scaffold))]
#[allow(dead_code)]
pub struct RustModuleContext {
    pub instance_id: String,
    pub instance_persistence_path: String,
}

/// Dialectica's IPC contract.
///
/// Every method takes a JSON `String` and returns a JSON `String` — the
/// ecosystem wire convention (PLAN.md §2.5). Failure is ALWAYS
/// `{"error":"..."}`, never a partial success shape.
///
/// `version` takes no argument because it answers a question with no input. It
/// still returns JSON, so a view never branches on reply shape.
///
/// The trait must be `Send + 'static`: the generated dispatch stores the
/// implementation as a `Box<dyn Any + Send>`.
pub trait DialecticaModule: Send + 'static {
    /// The module's own version, as `{"version":"X.Y.Z"}`.
    fn version(&mut self) -> String;

    /// Round-trip probe. Takes `{"payload":<any>}`, returns `{"pong":<any>}`.
    /// Carrying a payload is the point: it proves the call moved data, not
    /// merely that it completed.
    fn ping(&mut self, request: String) -> String;

    /// Panics on purpose, then returns the error shape anyway.
    ///
    /// This is Phase 0 apparatus, not forum surface. It is how "what does a
    /// panic in a dispatch handler actually do?" gets answered against a
    /// running host instead of by inference, and it is what stops the panic
    /// guard from being a claim nothing exercises. Remove it once the question
    /// is settled and the answer is written down.
    fn panic_probe(&mut self, request: String) -> String;

    /// Asks `delivery_module` whether a reliable channel is open.
    ///
    /// This method exists to prove the Phase 0 bridge (PLAN.md §3.2, §13 open
    /// question 1) rather than to serve a forum need — `channelExists` is the
    /// cheapest delivery call with no side effect and no node required, so a
    /// failure here is a bridge failure and not a network one.
    ///
    /// What it proves is a COMPILE-time fact more than a runtime one: if
    /// `modules().delivery_module.channel_exists(..)` type-checks, the builder
    /// generated a typed Rust client from delivery's contract, which is the
    /// whole question Phase 0 was created to answer.
    ///
    /// Takes `{"channelId":"..."}`. Returns `{"exists":<verbatim>}` or the
    /// error shape.
    fn delivery_channel_exists(&mut self, request: String) -> String;

    /// Framework plumbing, not a contract method — the generator skips
    /// defaulted methods when deriving the `.lidl`.
    fn on_context_ready(&mut self, _ctx: &RustModuleContext) {}
}

// ─── Why the module surface is behind a cfg ───────────────────────────────
//
// `cargo test` cannot compile the module surface, and no arrangement makes it
// able to. The scaffold include pulls in `generated/provider_gen.rs`, which the
// BUILDER writes into a staged copy of this crate; in a plain checkout the file
// does not exist and the crate fails to compile before a single test runs:
//
//     error: couldn't read .../generated/provider_gen.rs: No such file
//
// That is not a step someone forgot. The scaffold is derived from the trait
// above at build time, so committing a copy would recreate exactly the
// contract/code drift `codegen.rust.trait` exists to prevent.
//
// So the surface is gated on the scaffold being present: `build.rs` sets the
// `logos_scaffold` cfg when it finds the file, which is every builder
// invocation and no plain `cargo test`. The test build then compiles `core`
// and stops — which is the whole reason `core` holds every decision and this
// file holds none.
//
// The trap this avoids: gating on `#[cfg(not(test))]` instead. It looks
// equivalent and is not. A plain `cargo build` (no `--test`) would still try
// the include and still fail, so the gate would not even achieve the thing it
// was reached for.
//
// The include stays at CRATE ROOT rather than inside a `mod`: the scaffold
// declares the author's hook as `extern "Rust" { fn logos_module_install(); }`
// and defines `install::<T>()`, `context()` and `RustModuleContext` as plain
// items. Keeping them at the root is what the generator's own fixture does,
// and it is the arrangement its comments describe.
#[cfg(logos_scaffold)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/generated/provider_gen.rs"
));

/// The module instance.
///
/// `Default`-constructed and a process-global singleton — the SDK gives no
/// constructor injection (PLAN.md §2.3), so this deliberately holds no state.
/// When Phase 1 needs state, it goes in `core` behind an `Arc`, not here.
#[cfg(logos_scaffold)]
#[derive(Default)]
struct Dialectica;

// A thin adapter and nothing more. Every method forwards straight into `core`,
// which is where the guard and the decisions live. If a body here ever grows
// past one line, that logic belongs in `core` — otherwise it is logic no test
// can reach.
#[cfg(logos_scaffold)]
impl DialecticaModule for Dialectica {
    fn version(&mut self) -> String {
        core::version(env!("CARGO_PKG_VERSION"))
    }

    fn ping(&mut self, request: String) -> String {
        core::ping(&request)
    }

    fn panic_probe(&mut self, request: String) -> String {
        core::panic_probe(&request)
    }

    fn delivery_channel_exists(&mut self, request: String) -> String {
        // The one place in this file with more than a forwarding line, and the
        // only reason is that `modules()` cannot exist in `core` — it calls
        // `lp_*` symbols undefined in a test binary (PLAN.md §2.3). So the
        // parts that CAN be tested live in `core` on either side of the call,
        // and only the call itself is here.
        //
        // The guard still wraps everything, including the cross-module call: a
        // panic raised while decoding a reply is a panic in a dispatch handler
        // like any other.
        core::guarded("delivery_channel_exists", || {
            let channel_id = match core::parse_channel_id(&request) {
                Ok(id) => id,
                Err(e) => return e,
            };
            // The generated client's signature, verbatim from the scaffold:
            //   pub fn channel_exists(&self, channel_id: &str)
            //       -> Result<serde_json::Value, LogosError>
            //
            // `-> result` in the contract becomes `serde_json::Value` on this
            // side, so delivery's verbatim "true"/"false" arrives as a JSON
            // value rather than a Rust bool. `channel_exists_reply` is handed
            // the Value and decides what to do with it — that decision is in
            // `core` because it is the part worth testing.
            match modules().delivery_module.channel_exists(&channel_id) {
                Ok(v) => core::channel_exists_reply(&v),
                Err(e) => core::error_json(&format!("delivery_module.channelExists: {e}")),
            }
        })
    }

    fn on_context_ready(&mut self, ctx: &RustModuleContext) {
        eprintln!(
            "dialectica ready: instance {} (persistence: {})",
            ctx.instance_id, ctx.instance_persistence_path
        );
    }
}

#[cfg(logos_scaffold)]
#[no_mangle]
pub extern "Rust" fn logos_module_install() {
    install::<Dialectica>();
}
