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

    /// A slate of candidate identities for a Stoa.
    ///
    /// Takes `{"stoa":"<hex>"}` and returns
    /// `{"slate":"<hex>","count":N,"candidates":[…]}` or the error shape.
    ///
    /// **There is no count parameter**, and the omission is the `identity-onboarding`
    /// spec's requirement rather than a simplification: a caller-supplied count is
    /// a number that decides how much key derivation this module performs. The
    /// count is reported so a view need not hardcode it.
    ///
    /// Each candidate carries an address and a public key and **no secret** — the
    /// view cannot sign, and a secret that has crossed this boundary cannot be
    /// recalled.
    ///
    /// Nothing is written. A slate that persisted would record a choice the user
    /// has not made.
    fn generate_identity_slate(&mut self, request: String) -> String;

    /// Keep one candidate from the slate, making it this user's identity.
    ///
    /// Takes `{"stoa":"<hex>","slate":"<hex>","index":N}` and returns
    /// `{"kept":true,"address":"…","publicKey":"…","path":N,"encrypted":bool}` or
    /// `{"kept":false,"reason":"…"}` — the two are exclusive.
    ///
    /// The `slate` field is the identifier the slate was returned with, and a
    /// selection against a slate that is no longer the current one is **refused**
    /// rather than satisfied by the current one's candidate at that index.
    ///
    /// `encrypted` reports whether the master key was encrypted at rest, so that an
    /// unencrypted keystore is a state a view can name rather than a silent
    /// default.
    ///
    /// Keeping is refused where an identity already exists. Replacing one discards
    /// every identity derived from it while the ops they signed remain published,
    /// so it is a separate operation this contract does not provide.
    fn keep_identity(&mut self, request: String) -> String;

    /// Who the user is in a Stoa, or why there is nobody.
    ///
    /// Takes `{"stoa":"<hex>"}` and returns
    /// `{"hasIdentity":true,"address":"…","publicKey":"…","path":N,"recoveryNeedsTheRecord":bool}`
    /// or `{"hasIdentity":false,"reason":"…"}` — the two are exclusive.
    ///
    /// **A different question from `getCapabilities`**, and the two can honestly
    /// disagree: a stored identity whose keystore permissions are too open is a
    /// real identity that cannot currently be used. A view with only the posting
    /// probe would have to render "you are nobody" to a user who has an identity
    /// and a fixable problem.
    ///
    /// `recoveryNeedsTheRecord` is how a view learns that an exported master key is
    /// not by itself a complete backup — the chosen derivation paths live only in
    /// local storage, and the view has no filesystem access to discover that for
    /// itself.
    fn who_am_i(&mut self, request: String) -> String;

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
///
/// # The second field, and why it is here rather than in `core`
///
/// `live_slate` is the identifier of the slate `generateIdentitySlate` last
/// returned, and it is what makes a selection against a **superseded** slate
/// refusable: a keep quotes the slate it was made against, and only something
/// spanning two calls can say which one is current. `core` is a pure crate with
/// no ambient state by design, so the one value that has to outlive a call lives
/// here.
///
/// **It holds 32 bytes of public randomness, not key material.** A slate
/// identifier names which derivation paths were offered, and a path is not secret
/// — `identity-onboarding` says so outright. That is what makes this field cheap:
/// it is not a secret held across calls, and there is no clearing obligation on
/// it. The master key stays in the keystore, which is opened per call.
///
/// `Default` still supplies the one genuinely parameterless constructor
/// `interface: "universal"` requires — `Option` defaults to `None`, so adding
/// this field adds no constructor parameter.
#[cfg(logos_scaffold)]
#[derive(Default)]
struct Dialectica {
    persistence_path: Option<String>,
    live_slate: Option<core::onboarding::SlateNonce>,
}

#[cfg(logos_scaffold)]
impl Dialectica {
    /// The directory the host gave this instance, or the error to return.
    ///
    /// Factored out at the point CLAUDE.md names: this was the same four lines in
    /// two handlers and would have been in five. A `Result` whose `Err` arm is
    /// already the wire reply, following `core::parse_channel_id`, so a caller
    /// cannot invent a second error shape while converting one.
    ///
    /// It is not in `core` because `core` has no notion of a host handing it a
    /// path — that is the whole reason this adapter exists.
    fn storage_dir(&self) -> Result<std::path::PathBuf, String> {
        match &self.persistence_path {
            Some(dir) => Ok(std::path::PathBuf::from(dir)),
            None => Err(core::error_json(
                "the host has not yet told this module where its storage is; \
                 try again once the module is ready",
            )),
        }
    }

    /// The master key, opened from disk or minted fresh in memory.
    ///
    /// **The minting is the reason this function exists, and it writes nothing.**
    /// A slate has to be available on a fresh install, before any identity is
    /// kept, and that needs a master key. So where no keystore exists this hands
    /// back one that has never touched a disk; `keepIdentity` is what writes it,
    /// and until then the user is choosing among candidates of a key that will
    /// only exist if they keep one.
    ///
    /// **The fresh key is deterministic across neither calls nor slates**, which
    /// is a real consequence worth naming: two `generateIdentitySlate` calls on a
    /// fresh install offer candidates of two *different* master keys, so a slate's
    /// identifier is only meaningful alongside the key that produced it. The
    /// `live_slate` check does not catch that, because both slates are equally
    /// live. What makes it harmless is that the keep mints its own key too and
    /// writes THAT one, so the identity kept is always one of the candidates of
    /// the key that was stored — never a candidate of a key that was discarded.
    ///
    /// Any error other than "no keystore" is propagated: a keystore that exists
    /// and cannot be opened must not be silently replaced by a fresh key, which is
    /// how every identity a user has gets discarded with no error saying so.
    fn master_key(
        dir: &std::path::Path,
    ) -> Result<core::keystore::Keystore, core::keystore::KeystoreError> {
        let path = core::keystore::default_path_in(dir);
        match core::keystore::open_from_env(&path) {
            Ok(ks) => Ok(ks),
            Err(core::keystore::KeystoreError::NotFound) => {
                Ok(core::keystore::Keystore::generate())
            }
            Err(e) => Err(e),
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
        let Some(dir) = self.persistence_path.clone() else {
            return core::error_json(
                "the host has not yet told this module where its storage is; \
                 try again once the module is ready",
            );
        };
        core::get_capabilities(&request, |stoa| {
            let path = core::keystore::default_path_in(std::path::Path::new(&dir));
            core::keystore::open_from_env(&path).map(|ks| ks.stoa_address(stoa).to_hex())
        })
    }

    fn list_threads(&mut self, request: String) -> String {
        let Some(dir) = self.persistence_path.clone() else {
            return core::error_json(
                "the host has not yet told this module where its storage is; \
                 try again once the module is ready",
            );
        };
        // The store is opened per call rather than held open, which is the
        // simple thing and the correct one today: the host may hand the same
        // path to another instance, and a handle held across calls would have to
        // answer what happens when it goes stale. `SqliteOpLog::open` is cheap
        // and its failure is exactly the "unreadable store" the view renders as
        // screen 07's failed state.
        core::list_threads_from_request(&request, || {
            core::log::SqliteOpLog::open(&std::path::Path::new(&dir).join("ops.sqlite"))
        })
    }

    fn generate_identity_slate(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // `remembered` rather than writing `self.live_slate` from inside the
        // closure: the closure would need `&mut self` while `core` holds it, and
        // taking a `Cell` out afterwards keeps the borrow local. The nonce is
        // recorded only if a slate was actually produced, so a refused request
        // does not supersede a slate the user is still looking at.
        let remembered = std::cell::Cell::new(None);
        let reply = core::generate_identity_slate(
            &request,
            || Self::master_key(&dir),
            |nonce| remembered.set(Some(nonce)),
        );
        if let Some(nonce) = remembered.into_inner() {
            self.live_slate = Some(nonce);
        }
        reply
    }

    fn keep_identity(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // The keystore to WRITE, minted or opened. On the expected path — a fresh
        // install — this is a new key, and `create` is what refuses to replace an
        // existing one.
        let keystore = match Self::master_key(&dir) {
            Ok(k) => k,
            Err(e) => return core::error_json(&e.to_string()),
        };
        let paths = match core::identity_store::IdentityStore::open(
            &core::identity_store::IdentityStore::default_path_in(&dir),
        ) {
            Ok(p) => p,
            Err(e) => return core::error_json(&e.to_string()),
        };
        // The keystore's own decision about protection at create time, not a copy
        // of it here — `protection_from_env` owns the byte handling and the reason
        // there is no fallback constant. Whichever it returns is recorded in the
        // file and reported in the reply, so an unencrypted keystore is a state a
        // view can name rather than a silent default.
        let unlock = core::keystore::protection_from_env();
        core::keep_identity(
            &request,
            self.live_slate,
            core::KeepTargets {
                keystore: &keystore,
                keystore_path: &core::keystore::default_path_in(&dir),
                unlock: &unlock,
                paths: &paths,
            },
        )
    }

    fn who_am_i(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // `open_from_env` rather than `master_key`: this method REPORTS, so a
        // missing keystore is "there is nobody" and must not mint one. Minting
        // here would report an identity for a key that does not exist.
        core::who_am_i(
            &request,
            || core::keystore::open_from_env(&core::keystore::default_path_in(&dir)),
            || {
                core::identity_store::IdentityStore::open(
                    &core::identity_store::IdentityStore::default_path_in(&dir),
                )
            },
        )
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
