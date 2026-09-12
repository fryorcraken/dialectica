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

    /// What a caller may do in a Stoa right now, and why not if not.
    ///
    /// Takes `{"stoa":"<hex>"}` and returns
    /// `{"canPost":bool, "identity":"…" | "reason":"…"}` — the `|` is
    /// exclusive. Every posting affordance in the view is gated on this and
    /// never on a build flag (PLAN.md §5.6): a compose box the user typed into
    /// and cannot submit has lost their draft.
    ///
    /// The probe re-determines its answer on every call by design, so a view
    /// asks it whenever it renders rather than caching it.
    fn get_capabilities(&mut self, request: String) -> String;

    /// One page of a Stoa's thread heads.
    ///
    /// Takes `{"stoa":"<hex>", "page":N, "perPage":N, "includeHidden":bool}`
    /// and returns `{"items":[…],"page":N,"hasMore":bool}` — the ecosystem's
    /// pagination shape, of which this is the project's first instance.
    ///
    /// **There is no `order` argument**, because core computes exactly one
    /// ordering and an accepted-but-degraded parameter would be a method
    /// telling its caller a falsehood. See `dialectica_core::feed` for what
    /// that ordering claims and the much larger thing it does not.
    ///
    /// A storage failure comes back as the error shape and NEVER as an empty
    /// page: the two mean opposite things and render identically if they are
    /// ever collapsed.
    fn list_threads(&mut self, request: String) -> String;

    /// Create a Stoa this peer is in, and return its address.
    ///
    /// Takes `{"title":"…"}` and returns
    /// `{"stoa":"<hex>","foundingTitle":"…","policy":"open"}`.
    ///
    /// **There is no creator argument, and there cannot be.** The creator key is
    /// what makes the creator the Stoa's sole moderator and it is fixed inside
    /// the address preimage forever, so a call accepting one would be a call
    /// that can be asked to create a Stoa the caller cannot moderate and whose
    /// address cannot be un-minted. The key comes from this peer's keystore.
    ///
    /// **The address is returned rather than only success**, because a creation
    /// reporting `{"ok":true}` leaves the view unable to name, share or read
    /// what it just made.
    ///
    /// Creation fails when the peer has no usable signing key and never mints
    /// one for the occasion; the reason is the keystore's own, the same one
    /// `getCapabilities` reports.
    ///
    /// **The same title twice is the same Stoa.** A genesis record carries no
    /// nonce and no timestamp, so the same creator and title *is* the same
    /// record and the same address. A user who wants two Stoas gives them two
    /// titles.
    fn create_stoa(&mut self, request: String) -> String;

    /// Join a Stoa somebody else created.
    ///
    /// Takes `{"stoa":"<hex>","genesis":"<hex>"}` and returns the same reply
    /// shape `createStoa` does.
    ///
    /// **It takes the genesis record as well as the address, and that is a
    /// property of the address rather than a limitation of this call.** An
    /// address is a one-way hash of the record: enough to *verify* a record
    /// somebody hands over, and not enough to *reconstruct* one. Since
    /// moderation cannot be resolved for a Stoa whose record this peer does not
    /// hold, the record has to arrive with the address — there is nowhere else
    /// for it to come from. A bare address is not joinable.
    ///
    /// The record is verified against the address before anything is recorded,
    /// and a mismatch is an error rather than a join of something close enough.
    /// Joining a Stoa this peer is already in succeeds and changes nothing.
    fn join_stoa(&mut self, request: String) -> String;

    /// One page of the Stoas this peer is in — those it created and those it
    /// joined.
    ///
    /// Takes `{"page":N,"perPage":N}` and returns
    /// `{"items":[{"stoa":"<hex>","foundingTitle":"…"}],"page":N,"hasMore":bool}`.
    ///
    /// **The contents are what membership records and nothing derived from the
    /// ops this peer holds.** A Stoa joined and still quiet has no ops at all,
    /// so an op-derived answer would omit precisely the Stoas a user has just
    /// acted on; and an op addressed to a Stoa nobody joined must never enrol
    /// this peer in it.
    ///
    /// **`foundingTitle`, never `title`.** What a Stoa is called *today* comes
    /// from a moderator-signed metadata op, and nothing resolves those yet —
    /// so presenting this as a current title would assert something no peer has
    /// checked.
    fn list_stoas(&mut self, request: String) -> String;

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
/// constructor injection (PLAN.md §2.3), and `interface: "universal"` scans this
/// type's `public:` surface, so a constructor taking parameters (even
/// all-defaulted ones) miscompiles. The one genuinely parameterless constructor
/// is what `Default` provides.
///
/// # The one piece of state, and why it cannot be anywhere else
///
/// The persistence path arrives through `on_context_ready` and through nothing
/// else. It is the host telling this instance where its storage lives, it is
/// not knowable before that callback, and every read this module serves needs
/// it. So it is held here, as the only field, and every handler that needs a
/// store opens one from it.
///
/// `Option<String>` rather than `String`, because "the host has not told us
/// yet" is a real state a call can arrive in — the dispatch table is live before
/// the callback in principle — and it must produce an honest error rather than a
/// read against an empty path.
#[cfg(logos_scaffold)]
#[derive(Default)]
struct Dialectica {
    persistence_path: Option<String>,
}

#[cfg(logos_scaffold)]
impl Dialectica {
    /// The host's storage directory, or the error shape saying it has not arrived.
    ///
    /// **One place, because it is a guard.** CLAUDE.md: "A guard is a job. Keep it
    /// separate, so 'is it called everywhere?' stays a question with an answer."
    /// It was two inline copies with identical wording, and every handler that
    /// reaches storage needs it — so each new one was another copy to keep in
    /// step, and two of them disagreeing about one state is a user being told
    /// different things about the same fact.
    ///
    /// `Result<PathBuf, String>` with the error arm already being the wire reply,
    /// following `parse_channel_id`: a caller cannot accidentally invent a second
    /// error shape while converting one.
    ///
    /// A `PathBuf` rather than the `String` the callers used to clone, so that
    /// each one stops spelling `std::path::Path::new(&dir)` for itself.
    fn storage_dir(&self) -> Result<std::path::PathBuf, String> {
        match &self.persistence_path {
            Some(dir) => Ok(std::path::PathBuf::from(dir)),
            None => Err(core::error_json(
                "the host has not yet told this module where its storage is; \
                 try again once the module is ready",
            )),
        }
    }
}

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

    fn get_capabilities(&mut self, request: String) -> String {
        // The keystore path is derived from the persistence path the host gave
        // us, which is why this is not a bare forward: `core` cannot read the
        // environment or know the host's layout (PLAN.md §2.3), so the adapter
        // supplies the lookup and `core` decides what its result means.
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // `poster_address_in` and not an accessor chosen here: WHICH key this is
        // is a decision, and this file is not compiled by `cargo test`, read by
        // any CI gate, or mutable by `cargo mutants`. It lived here as two call
        // sites that had to agree, and re-diverging them restored a shipped bug
        // with every gate green. `core::keystore::creator_and_poster_in` derives
        // both halves in one expression and
        // `the_creator_a_creation_names_is_the_identity_the_probe_reports` fails
        // when they disagree. The `stoa` argument is still taken — and still
        // validated by `core` — because the probe is scoped to a Stoa and stays
        // so when per-Stoa identity is switched back on.
        core::get_capabilities(&request, |_stoa| {
            core::keystore::poster_address_in(&dir).map(|a| a.to_hex())
        })
    }

    fn list_threads(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // The store is opened per call rather than held open, which is the
        // simple thing and the correct one today: the host may hand the same
        // path to another instance, and a handle held across calls would have to
        // answer what happens when it goes stale. `SqliteOpLog::open` is cheap
        // and its failure is exactly the "unreadable store" the view renders as
        // screen 07's failed state.
        core::list_threads_from_request(&request, || {
            core::log::SqliteOpLog::open(&dir.join("ops.sqlite"))
        })
    }

    fn create_stoa(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // Two host-derived paths, and they are DIFFERENT FILES on purpose: the
        // keystore holds the key that becomes the creator, and the membership
        // store holds what was created. `core` cannot know either layout, so the
        // adapter supplies both and `core` decides what their failures mean.
        core::with_membership_store("create_stoa", &core::membership_path_in(&dir), |store| {
            // The SAME derivation `get_capabilities` above reports — one
            // expression in `core`, not two call sites here agreeing. See
            // `core::keystore::creator_and_poster_in` for why that distinction is
            // the whole point.
            core::create_stoa(&request, || core::keystore::creator_key_in(&dir), store)
        })
    }

    fn join_stoa(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        core::with_membership_store("join_stoa", &core::membership_path_in(&dir), |store| {
            core::join_stoa(&request, store)
        })
    }

    fn list_stoas(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        core::with_membership_store("list_stoas", &core::membership_path_in(&dir), |store| {
            core::list_stoas(&request, store)
        })
    }

    fn on_context_ready(&mut self, ctx: &RustModuleContext) {
        eprintln!(
            "dialectica ready: instance {} (persistence: {})",
            ctx.instance_id, ctx.instance_persistence_path
        );
        // The ONLY place this is set. Every read this module serves needs it and
        // nothing else supplies it.
        self.persistence_path = Some(ctx.instance_persistence_path.clone());
    }
}

#[cfg(logos_scaffold)]
#[no_mangle]
pub extern "Rust" fn logos_module_install() {
    install::<Dialectica>();
}
