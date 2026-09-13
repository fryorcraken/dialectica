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

    /// Publish a post into a Stoa.
    ///
    /// Takes `{"stoa":"<hex>","body":"…"}` and returns
    /// `{"opId":"<hex>","wasNew":bool}`, or the error shape.
    ///
    /// **The identity is not a parameter and a request naming one is refused.**
    /// It falls out of the Stoa (PLAN.md §5.2 gives a user one identity per
    /// Stoa), so no method here can be asked to sign as someone it is not.
    ///
    /// **`wasNew` is not decoration.** An op id is the hash of bytes carrying no
    /// timestamp and no nonce, so publishing the same body into the same Stoa
    /// twice publishes ONE op and the second call reports the first's id. Both
    /// arrive as a success, and this field is the only thing that tells a
    /// deduplicated publish from a first one.
    ///
    /// A success says the op exists **locally**. It says nothing about whether
    /// any peer received it — delivery's outcome arrives later, and a call that
    /// waited on it would be a call that can hang.
    fn publish_post(&mut self, request: String) -> String;

    /// Publish a reply.
    ///
    /// Takes `{"stoa":"<hex>","parent":"<hex>","body":"…"}`.
    ///
    /// **There is no `thread` argument, and one supplied is refused.** The
    /// thread is derived from the parent, which makes a reply filed under a
    /// thread its parent does not belong to unrepresentable rather than checked
    /// at each call site.
    ///
    /// The cost is contracted rather than hidden: a reply to a parent this peer
    /// does not hold is **refused**, because with no parent to read there is no
    /// thread to derive. The refusal distinguishes "not held" from "held but not
    /// a post", so a caller can tell a propagation gap from a category mistake.
    fn publish_reply(&mut self, request: String) -> String;

    /// Publish a vote.
    ///
    /// Takes `{"stoa":"<hex>","target":"<hex>","direction":"up"|"down"}`. An
    /// unrecognised direction is refused naming what was supplied and is never
    /// mapped onto a recognised one.
    ///
    /// **The reply carries an op id and nothing describing an effect.** No
    /// ordering, count, tally or score in the current contract reads a `Vote`
    /// op, so there is no position for publishing one to have changed. A
    /// published vote accumulates history a later scorer reads; whether a scorer
    /// exists is not a property of publishing one.
    fn publish_vote(&mut self, request: String) -> String;

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
    /// The onboarding state that spans two calls: the master key a slate was
    /// offered under, and which slate is live.
    ///
    /// **A `core` type rather than fields here**, and that is the point rather than
    /// tidiness. It held only the nonce, with the mint-or-open decision in a helper
    /// on this struct — and this file is `#[cfg(logos_scaffold)]`, so that decision
    /// was compiled out of `cargo test` entirely. It minted a fresh key per call, so
    /// the slate showed one identity and the keep wrote another; no test could see
    /// it, because there was nothing compiled to see. See
    /// [`core::wire::OnboardingSession`] for the defect and the fix.
    onboarding: core::wire::OnboardingSession,
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

    /// The record of chosen paths, in the directory the host gave this instance.
    ///
    /// Factored out for the reason [`Dialectica::storage_dir`] was: the same two
    /// lines were in three handlers. Not in `core` for the same reason either —
    /// `core` has no notion of a host-supplied directory.
    fn paths(
        dir: &std::path::Path,
    ) -> Result<core::identity_store::IdentityStore, core::identity_store::IdentityStoreError> {
        core::identity_store::IdentityStore::open(
            &core::identity_store::IdentityStore::default_path_in(dir),
        )
    }

    /// The keystore on disk, or `NotFound`.
    ///
    /// **This does not mint.** Minting is `core`'s decision, in
    /// [`core::wire::OnboardingSession`], because the question "what does no
    /// keystore mean here" is different for a slate (offer candidates of a key that
    /// does not exist yet) and for a report (there is nobody), and a helper here
    /// answering it for both was how a slate and a keep came to see different keys.
    /// What is left in this file is the part that genuinely cannot move: where the
    /// file is, and the environment its protection is read from.
    fn open_keystore(
        dir: &std::path::Path,
    ) -> Result<core::keystore::Keystore, core::keystore::KeystoreError> {
        core::keystore::open_from_env(&core::keystore::default_path_in(dir))
    }
}

// The one piece of assembly this file does, and the reason it is here rather
// than in `core`.
//
// The three publish handlers need FOUR things `core` structurally cannot reach:
// the keystore (a path derived from what the host supplied), the per-Stoa
// signing key, a store, and delivery. Each is the adapter's to supply, exactly
// as `get_capabilities` supplies a lookup and `list_threads` supplies a store
// opener.
//
// It is one function rather than three copies because the assembly is identical
// for all three and only the handler differs. Three copies is three places to
// forget the key or to open the store in the wrong order.
#[cfg(logos_scaffold)]
impl Dialectica {
    /// Assemble the keystore, the key, the store and the delivery sink, then run
    /// one publish handler.
    ///
    /// # The Stoa is read twice, and that is not a redundancy to remove
    ///
    /// The signing key is per-Stoa (PLAN.md §5.2), so the Stoa has to be known
    /// before the key can be derived — and the handler parses the request
    /// properly, refusing forbidden fields and naming its own failures. So this
    /// reads the Stoa cheaply to derive a key, and the handler re-reads it as
    /// part of the parse it owns. The alternative, threading a parsed Stoa in
    /// from here, would put half the request's validation in the one file no
    /// test can reach.
    ///
    /// # Delivery's outcome is discarded, deliberately
    ///
    /// The sink returns nothing. A publish is "append and publish", not "send":
    /// the append has completed before the sink is called, a refused handoff
    /// leaves the op published, and there is no outcome to wait for — so a call
    /// that hung on delivery cannot be written here.
    ///
    /// **The channel identity is `op-transport`'s to settle, not this file's.**
    /// Until that capability lands there is nothing to hand an op to, so the
    /// sink is a no-op that logs. A no-op is honest; inventing a channel-naming
    /// scheme here would be two peers computing different values and opening
    /// channels nobody else is in — silently, and permanently.
    fn publishing<F>(&mut self, request: &str, method: &str, handler: F) -> String
    where
        F: FnOnce(
            &str,
            &mut core::log::SqliteOpLog,
            &core::identity::SecretKey,
            &mut dyn FnMut(&core::op::OpId),
        ) -> String,
    {
        let Some(dir) = self.persistence_path.clone() else {
            return core::error_json(
                "the host has not yet told this module where its storage is; \
                 try again once the module is ready",
            );
        };
        let dir = std::path::PathBuf::from(dir);

        core::guarded(method, || {
            // The Stoa, read only far enough to derive a key. The handler owns
            // the real parse and reports every other malformation by name.
            let parsed: serde_json::Value = match serde_json::from_str(request) {
                Ok(v) => v,
                Err(e) => return core::error_json(&format!("invalid JSON: {e}")),
            };
            let stoa = match parsed.get("stoa") {
                Some(serde_json::Value::String(s)) => match core::identity::Address::from_hex(s) {
                    Ok(a) => a,
                    Err(e) => return core::error_json(&format!("stoa: {e}")),
                },
                Some(_) => return core::error_json("stoa must be a string"),
                None => return core::error_json("missing field: stoa"),
            };

            // A publish requires a usable identity and NEVER creates one. This
            // opens an existing keystore; nothing here calls `generate` or
            // `create`, so no key material can appear as a side effect of a
            // publish being attempted.
            //
            // `core::no_identity` rather than a `format!` here: this file is
            // behind `cfg(logos_scaffold)` and no `cargo test` compiles it, so a
            // message worded here is a message no gate can see. The wording lives
            // in `Refusal::NoIdentity`'s `Display`, which is compiled and
            // asserted on. Raising the refusal is the adapter's job because only
            // the adapter can open a keystore; wording it is not.
            let keystore_path = core::keystore::default_path_in(&dir);
            let keystore = match core::keystore::open_from_env(&keystore_path) {
                Ok(k) => k,
                Err(e) => return core::no_identity(&e.to_string()),
            };
            let key = keystore.stoa_key(&stoa);

            let mut log = match core::log::SqliteOpLog::open(&dir.join("ops.sqlite")) {
                Ok(l) => l,
                Err(e) => return core::error_json(&e.to_string()),
            };

            handler(request, &mut log, &key, &mut |id| {
                // `op-transport` owns what happens here. Logged rather than
                // silent, so "the op was published and went nowhere" is visible
                // in a daemon log rather than inferred from a peer never seeing
                // it.
                eprintln!(
                    "dialectica published {} — delivery is not wired yet (op-transport)",
                    id.to_hex()
                );
            })
        })
    }
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

// A thin adapter and nothing more. Every method forwards into `core`, which is
// where the guard and the decisions live, because nothing in this file is reached
// by `cargo test`, by clippy, by fmt, or by `cargo mutants` — a wrong line here
// ships with every gate green, and one already did (see
// `core::keystore::creator_and_poster_in`).
//
// THE RULE, stated as what it actually permits. A body here may derive a host path
// and pass it in; it may not make a decision. `storage_dir()` plus one `core` call
// is the shape.
//
// Counted rather than asserted, because the claim this replaces was a miscount.
// Of the ten methods below, three — `version`, `ping`, `panic_probe` — are
// single-line forwards, because they need no path. The other seven are multi-line:
// five are `storage_dir()` plus a `core` call, `delivery_channel_exists` also
// because `modules()` calls `lp_*` symbols undefined in a test binary (PLAN.md
// §2.3), and `on_context_ready` because it is the one setter.
//
// This used to say "if a body here ever grows past one line, that logic belongs in
// `core`", with `delivery_channel_exists` excused as "the one place in this file
// with more than a forwarding line". Both were false of the file by the time they
// were read, and three of the multi-line bodies were added by the change that left
// the claims standing (`findings/readability.md` entry 4). A line count was the
// wrong test; what a body is allowed to CONTAIN is the right one.
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
        // The one body here that is multi-line for a reason other than deriving a
        // host path: `modules()` cannot exist in `core` — it calls `lp_*` symbols
        // undefined in a test binary (PLAN.md §2.3). So the parts that CAN be
        // tested live in `core` on either side of the call, and only the call
        // itself is here.
        //
        // It used to claim to be "the one place in this file with more than a
        // forwarding line", which was true when written and false by the time it
        // was read (`findings/readability.md` entry 4).
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
        // The two stores' paths are derived from the persistence path the host gave
        // us, which is why this is not a bare forward: `core` cannot read the
        // environment or know the host's layout (PLAN.md §2.3), so the adapter
        // supplies the openers and `core` decides what their results mean.
        //
        // The DERIVATION is no longer here. It was — `ks.stoa_address(stoa)`, the
        // pathless scheme — while `whoAmI` used the path-taking one under a bumped
        // salt, so the two methods reported two different addresses for one user in
        // one Stoa. It could not be tested where it was, because this file is not
        // compiled by `cargo test`; see `core::wire::posting_identity`.
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        core::wire::get_capabilities_from_stores(
            &request,
            || Self::open_keystore(&dir),
            || Self::paths(&dir),
        )
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
        core::with_membership_store(&core::membership_path_in(&dir), |store| {
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
        core::with_membership_store(&core::membership_path_in(&dir), |store| {
            core::join_stoa(&request, store)
        })
    }

    fn list_stoas(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // The READ half, so this handler's read-only-ness survives the seam.
        core::with_membership_store_read(&core::membership_path_in(&dir), |store| {
            core::list_stoas(&request, store)
        })
    }

    fn generate_identity_slate(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        // The session holds both the master key and the live nonce, so there is no
        // `Cell` to carry a nonce back out and no way for the two to be set
        // separately. That pairing is the fix for the defect described on
        // `OnboardingSession`: a slate is a (key, nonce) pair, and holding half of
        // it is what let the keep write a different identity from the one shown.
        core::generate_identity_slate(&mut self.onboarding, &request, || Self::open_keystore(&dir))
    }

    fn keep_identity(&mut self, request: String) -> String {
        let dir = match self.storage_dir() {
            Ok(d) => d,
            Err(e) => return e,
        };
        let paths = match Self::paths(&dir) {
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
            &mut self.onboarding,
            &request,
            || Self::open_keystore(&dir),
            core::KeepTargets {
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
        // `open_keystore` rather than the session: this method REPORTS, so a
        // missing keystore is "there is nobody" and must not mint one — and must
        // not report one the session happens to be holding for an unfinished
        // onboarding either. Reporting a minted, unwritten key as the user's
        // identity would name an identity that does not exist yet.
        core::who_am_i(&request, || Self::open_keystore(&dir), || Self::paths(&dir))
    }

    fn publish_post(&mut self, request: String) -> String {
        self.publishing(&request, "publish_post", core::publish_post)
    }

    fn publish_reply(&mut self, request: String) -> String {
        self.publishing(&request, "publish_reply", core::publish_reply)
    }

    fn publish_vote(&mut self, request: String) -> String {
        self.publishing(&request, "publish_vote", core::publish_vote)
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
