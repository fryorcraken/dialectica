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

    // ─── The write path ───────────────────────────────────────────────────
    //
    // PLAN.md §9.1's Stage B and Stage D, plus the two acts that precede both.
    // Every one of these is a WIDENING of the deliverable, which CLAUDE.md asks
    // be done on purpose rather than as a side effect — so each carries the
    // reason it is a method rather than a field on another one.
    //
    // NONE OF THEM SENDS ANYTHING. Each stores locally and returns. The delivery
    // transport is separately owned, and `dialectica_core::publish` records why
    // storing-then-sending is the right order and what a transport adds.

    // NOTE: `createIdentity` is NOT in this trait, and its absence is a seam
    // rather than a gap. Minting the root secret and writing the keystore are
    // separately owned — they are the root of the dependency tree, not a leaf of
    // the write path — so every method below is written against "a key that can
    // sign" and nothing more. When the identity method lands it is a new method on
    // this trait and no change to any of these.
    //
    // Until it does, the publishing methods below report the keystore's own error
    // when there is no identity yet, which is `KeystoreError::NotFound` — "no
    // keystore found; create one before posting". That is the honest answer.

    /// Create a Stoa. `{"title":"…"}` -> `{"stoa":"…","genesis":"…", …}`.
    ///
    /// **Returns the genesis record as well as the address, and must.** The
    /// address is a one-way hash of the record, so a peer given only the address
    /// cannot recover it — and every read needs it, because a moderator set can
    /// only be built from one. A creator sharing their Stoa shares both.
    ///
    /// Publishes no op: `op.rs` records that hashing the record is what creates
    /// a Stoa, so there is nothing for a signed op to add.
    fn create_stoa(&mut self, request: String) -> String;

    /// Join a Stoa. `{"stoa":"…","genesis":"…"}` -> the Stoa.
    ///
    /// **Verifies rather than trusts** (§4.8): the record must hash to the
    /// address it arrived with, and a mismatch is an error rather than a join of
    /// something close enough. A caller cannot use this to install themselves as
    /// a Stoa's moderator — changing the creator changes the address.
    fn join_stoa(&mut self, request: String) -> String;

    /// The Stoas this peer created or joined, one page.
    ///
    /// **Not derived from the op log**, and the distinction is load-bearing: a
    /// joined Stoa nobody has posted in must still be listed, and an op gossiped
    /// for a Stoa this peer never joined must not create one. `list_threads`
    /// answers "what have I seen addressed here", which is a different question.
    ///
    /// A storage failure is the error shape and never an empty list — §11.1
    /// obligation 5, on the first screen a user sees, where "you are in no
    /// Stoas" and "I could not read the store" would otherwise render alike.
    fn list_stoas(&mut self, request: String) -> String;

    /// Publish a top-level post. `{"stoa":"…","body":"…"}` -> `{"op":"…"}`.
    ///
    /// Returns the op id rather than a success flag, because a reply names its
    /// parent by op id: a view that had to re-read the feed to learn what it just
    /// posted would be making a second call for something the first one knew.
    fn create_post(&mut self, request: String) -> String;

    /// Publish a reply. `{"stoa":"…","parent":"…","body":"…"}` -> `{"op":"…"}`.
    ///
    /// **There is no `thread` argument**, though §9.1's sketch has one. The
    /// thread is a function of the parent, so accepting it would let a caller
    /// file a reply under a thread it does not belong to — validly signed, and
    /// rendered in the wrong thread on every peer. Core derives it.
    fn create_reply(&mut self, request: String) -> String;

    /// Publish a vote. `{"stoa":"…","target":"…","direction":"up"|"down"}`.
    ///
    /// **Nothing reads votes yet** (§7.2 rule 2 ships no score), so a vote
    /// published today changes nothing a reader can see. The op kind exists and
    /// the history is worth collecting before scoring lands; PLAN.md §9.1 records
    /// the opposing argument, and `dialectica_core::publish::create_vote` carries
    /// both. A view offering this must not imply an effect it does not have.
    fn create_vote(&mut self, request: String) -> String;

    /// One page of a thread: a root post and its replies, resolved.
    ///
    /// Its own method rather than a parameter on `list_threads`, because they are
    /// different reads: a feed is a list of thread *heads* and this is one
    /// thread's posts. §9.1 names the absence of a thread read as a core gap;
    /// this closes it.
    ///
    /// Takes the genesis record for the reason `list_threads` does — a moderator
    /// set can only be built from one, and a caller without it must not get a
    /// thread with moderation silently not applied.
    fn get_thread(&mut self, request: String) -> String;

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
        // Through `store_path` rather than joining inline, so the read path and
        // every write handler cannot end up opening two different files — a peer
        // whose own posts were invisible to its own feed, with no error anywhere.
        core::list_threads_from_request(&request, || {
            core::log::SqliteOpLog::open(&store_path(&dir))
        })
    }

    fn create_stoa(&mut self, request: String) -> String {
        self.with_store_and_key(&request, |request, store, key| {
            core::create_stoa(request, store, &key.public_key())
        })
    }

    fn join_stoa(&mut self, request: String) -> String {
        // No key needed: joining signs nothing. Taking one would be asking the
        // user to unlock a keystore in order to read.
        self.with_store(&request, core::join_stoa)
    }

    fn list_stoas(&mut self, request: String) -> String {
        self.with_store(&request, |request, store| core::list_stoas(request, store))
    }

    fn create_post(&mut self, request: String) -> String {
        self.with_store_and_key(&request, core::create_post)
    }

    fn create_reply(&mut self, request: String) -> String {
        self.with_store_and_key(&request, core::create_reply)
    }

    fn create_vote(&mut self, request: String) -> String {
        self.with_store_and_key(&request, core::create_vote)
    }

    fn get_thread(&mut self, request: String) -> String {
        // The genesis record travels in the request and is checked against the Stoa
        // address there, exactly as `list_threads` does it — in `core`, through
        // `get_thread_from_request`, because a parse written here would be logic no
        // test can reach.
        self.with_store(&request, |request, store| {
            core::wire::get_thread_from_request(request, store)
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

/// The one "host has not told us where storage is" reply.
///
/// Written once because six handlers need it and a second spelling would be a
/// second message for one state. It is the error shape rather than an empty
/// answer: a call arriving before `on_context_ready` has not found an empty store,
/// it has found no store at all.
#[cfg(logos_scaffold)]
fn not_ready() -> String {
    core::error_json(
        "the host has not yet told this module where its storage is; \
         try again once the module is ready",
    )
}

/// The signing key for this peer's single identity.
///
/// # THIS FUNCTION IS THE SEAM, AND IT IS THE ONLY THING THE IDENTITY OWNER HAS
/// TO REPLACE
///
/// The write path needs a `SecretKey` to sign ops with. Where that key comes from
/// is the identity half of the MVP and is separately owned, so the dependency is
/// named here, in one function, rather than spread across six handlers.
///
/// **What the write path requires of it**, stated so the other side can be designed
/// against it rather than guessed at:
///
/// - it yields a `dialectica_core::identity::SecretKey`, which is what
///   `Op::sign` takes and what `sign_op_bytes` signs with;
/// - it is the ROOT key, because the MVP is one identity per user —
///   **not** `keystore.stoa_key(&stoa)`, which is §5.2's per-Stoa derivation and is
///   deliberately uncalled;
/// - it is needed only for the duration of one publish, so nothing here holds it
///   across calls or wants it to outlive the handler;
/// - its failure is a `KeystoreError`, so "no identity yet" arrives as
///   `NotFound` — whose message already names the fix.
///
/// Nothing beyond that. The write path never needs the root bytes, never needs to
/// know whether the keystore was encrypted, and never needs a second key.
///
/// # Why it is unimplemented rather than implemented here
///
/// `Keystore` exposes `stoa_key` and no root-key accessor, and its documentation
/// says why: "the root is the one value that, if leaked, yields every Stoa identity
/// a user has, and nothing outside this type needs it." The MVP's one-identity rule
/// makes that sentence false — something outside now does need it — and **changing
/// it is a decision about key material leaving core**, which belongs with the agent
/// owning the keystore write path rather than with this one.
///
/// So this reports rather than reaching past that boundary. The publishing methods
/// are complete, tested against `SecretKey::generate()` in the integration suite,
/// and become live the moment this returns a key.
#[cfg(logos_scaffold)]
fn root_signing_key(
    _keystore: &core::keystore::Keystore,
) -> Result<core::identity::SecretKey, core::keystore::KeystoreError> {
    // `Locked` is the closest existing variant: the keystore is present and this
    // build cannot obtain a usable signing key from it. Its message points at the
    // passphrase, which is not the reason here — so this is a deliberately
    // temporary stand-in rather than a correct error, and it is the one thing in
    // this file waiting on another change.
    //
    // Chosen over adding a variant because `KeystoreError` is the identity owner's
    // type, and growing it from this side is the collision the split exists to
    // avoid.
    Err(core::keystore::KeystoreError::Locked)
}

/// Where the op store lives inside the host-stamped persistence directory.
///
/// One function so the read path and the write path cannot open two different
/// files — which would be a peer whose posts are invisible to its own feed, with
/// no error anywhere.
#[cfg(logos_scaffold)]
fn store_path(dir: &str) -> std::path::PathBuf {
    std::path::Path::new(dir).join("ops.sqlite")
}

#[cfg(logos_scaffold)]
impl Dialectica {
    /// Open the store and hand it to a handler.
    ///
    /// The store is opened per call rather than held, for the reason
    /// `list_threads` already gives: the host may hand the same path to another
    /// instance, and a handle held across calls would have to answer what happens
    /// when it goes stale.
    ///
    /// A failure to open is §2.5's error shape and never an empty answer — the
    /// rule `list_threads` follows, applied to every write handler at once so
    /// there is one place it can be got wrong.
    fn with_store(
        &mut self,
        request: &str,
        handler: impl FnOnce(&str, &mut core::log::SqliteOpLog) -> String,
    ) -> String {
        let Some(dir) = self.persistence_path.clone() else {
            return not_ready();
        };
        let mut store = match core::log::SqliteOpLog::open(&store_path(&dir)) {
            Ok(s) => s,
            Err(e) => return core::error_json(&e.to_string()),
        };
        handler(request, &mut store)
    }

    /// Open the store, obtain the signing key, and hand both to a handler.
    ///
    /// # THE SEAM WITH THE IDENTITY OWNER IS THE ONE LINE MARKED BELOW
    ///
    /// Everything in this function except that line is settled. What is not settled
    /// is how a `SecretKey` is obtained from the keystore, because that is the
    /// identity half — the root secret's lifetime, what may leave core, and whether
    /// an unlocked key is handed out at all are decisions belonging with the code
    /// that writes the keystore, not with the code that publishes ops.
    ///
    /// **What this side needs is exactly one thing:** a `&SecretKey` to sign with,
    /// for the whole duration of one publish. Nothing here needs the root bytes,
    /// needs to know whether the keystore was encrypted, or needs the key to
    /// outlive the call. Any provider satisfying that shape works unchanged.
    ///
    /// # The keystore is opened per call, and that is not an oversight
    ///
    /// Holding an unlocked root secret across calls would mean this module keeps
    /// decrypted key material in memory for its whole lifetime, which is a
    /// different security posture from the one `keystore.rs` was built for — and it
    /// would answer "when does a secret stop being needed" with "never". Opening per
    /// publish keeps the answer "for one op".
    ///
    /// The cost is one Argon2 derivation per published op on an encrypted keystore,
    /// ~1s at this build's parameters. That is the right trade for a user-initiated
    /// action and would not be for a read, which is why no read path does this.
    fn with_store_and_key(
        &mut self,
        request: &str,
        handler: impl FnOnce(&str, &mut core::log::SqliteOpLog, &core::identity::SecretKey) -> String,
    ) -> String {
        let Some(dir) = self.persistence_path.clone() else {
            return not_ready();
        };
        let keystore_path = core::keystore::default_path_in(std::path::Path::new(&dir));
        // The keystore FIRST. A peer with no identity cannot publish, and saying so
        // costs nothing — where opening the store first would report a storage
        // problem to a user whose actual state is "you have no identity yet". That
        // is also the honest error until the identity method exists:
        // `KeystoreError::NotFound` says "create one before posting".
        let keystore = match core::keystore::open_from_env(&keystore_path) {
            Ok(k) => k,
            Err(e) => return core::error_json(&e.to_string()),
        };
        let mut store = match core::log::SqliteOpLog::open(&store_path(&dir)) {
            Ok(s) => s,
            Err(e) => return core::error_json(&e.to_string()),
        };
        // ─── THE SEAM ─────────────────────────────────────────────────────
        //
        // One identity per user for this MVP, so this must be the ROOT key —
        // NOT `keystore.stoa_key(&stoa)`, which is §5.2's per-Stoa derivation
        // and is deliberately uncalled.
        //
        // `Keystore` exposes no root-key accessor today, by design: its own
        // comment says "there is no accessor for the root itself [...] nothing
        // outside this type needs it". This is the caller that now does, and
        // adding it is the identity owner's decision rather than this file's —
        // which is why this line does not reach past that boundary and invent one.
        let key = match root_signing_key(&keystore) {
            Ok(k) => k,
            Err(e) => return core::error_json(&e.to_string()),
        };
        handler(request, &mut store, &key)
    }
}

#[cfg(logos_scaffold)]
#[no_mangle]
pub extern "Rust" fn logos_module_install() {
    install::<Dialectica>();
}
