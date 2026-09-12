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

// `OpLog` must be IN SCOPE for `log.get(..)` to resolve: `SqliteOpLog` gets that
// method from the trait, not inherently, and a trait method is unreachable
// without its trait imported. Caught by the Nix build rather than by
// `cargo test`, which cannot compile this file at all — see the gate note below.
#[cfg(logos_scaffold)]
use dialectica_core::log::OpLog as _;

// The generated client type, for its associated `decode_*` function. The
// scaffold emits one module per dependency, so the delivery client is at
// `crate::delivery_module::DeliveryModuleClient` — a path that exists only in a
// builder build, which is why this import is gated alongside everything else
// that names it.
#[cfg(logos_scaffold)]
use crate::delivery_module::DeliveryModuleClient;

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

    /// Open (or re-open) this Stoa's reliable channel, and report what was
    /// opened.
    ///
    /// Takes `{"stoa":"<hex>"}` and returns
    /// `{"channelId":"…","contentTopic":"…"}`, or the error shape.
    ///
    /// **Idempotent, and re-opening is supported** (PLAN.md §4.3). The channel id
    /// is a pure function of the Stoa address with no epoch in it, so joining a
    /// Stoa, leaving it and rejoining derives the same id — which is what lets a
    /// rejoin work without a restart. An earlier design forbade this to route
    /// around a reported upstream crash on channel re-creation; §4.3 withdrew
    /// both the workaround and the restriction it bought.
    ///
    /// `senderId` is not in the reply. It is the one of the three values that
    /// must **differ** between peers (§4.1), and placing it beside the two that
    /// must agree is how the confusion §4.3 spends a section on begins.
    fn join_stoa_channel(&mut self, request: String) -> String;

    /// Publish an op this peer already holds onto its Stoa's channel.
    ///
    /// Takes `{"stoa":"<hex>","op":"<hex>"}` and returns
    /// `{"published":true,"opId":"<hex>"}`, or the error shape.
    ///
    /// **The op must already be in the store**, and that ordering is the
    /// contract rather than an implementation detail: §3.3 makes the op log the
    /// authority and every view "a cache that can be rebuilt by replay", so an
    /// op that reached the network without reaching the log would be invisible to
    /// its own author. This method reads the stored bytes and frames them — it
    /// never re-encodes, because a re-encode differing in any way would produce a
    /// payload that fails verification on every peer receiving it.
    ///
    /// A successful reply means the send was **dispatched**, not delivered.
    /// §4.4: an ACK means "some participants received it", never that a
    /// particular peer has it, and delivery reports the outcome later through
    /// `channelMessageSent` / `channelMessageError`.
    fn publish_op(&mut self, request: String) -> String;

    /// How this peer's inbound channel traffic is doing.
    ///
    /// Takes `{}` and returns `{"queued":N,"dropped":N,"cap":N}`.
    ///
    /// **`dropped` is the reason this method exists.** PLAN.md §2.3 requires the
    /// inbound event queue to be bounded — "A consumer slower than the event rate
    /// grows the queue until OOM [...] Bound it ourselves" — and a bound means
    /// payloads are sometimes discarded. A peer shedding load with no way to say
    /// so could not be told apart from a Stoa that is merely quiet, and those
    /// call for opposite responses. Surfacing the counter is what keeps the bound
    /// from being a silent leak.
    fn transport_status(&mut self, request: String) -> String;

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

// ─── The transport's shared state, and why it is static ───────────────────
//
// The inbound listener runs on a thread this module spawns, and that thread
// CANNOT hold `&mut self`: `on_context_ready` takes `&mut self` for the duration
// of the call only, and the generated dispatch owns the instance behind a lock
// for the duration of each method. So anything the listener and a handler both
// touch lives here rather than on `Dialectica`.
//
// This is the pattern the SDK's own integration fixture uses
// (`tests/caller/rust-lib/src/lib.rs`), and it is a consequence of the
// threading model rather than a preference.

/// The inbound queue, shared between the listener thread and `transport_status`.
///
/// PLAN.md §2.3 assigns the bound: the SDK's event subscription is an unbounded
/// `mpsc` whose C trampoline "never blocks and never fails", so a consumer slower
/// than the event rate grows it until OOM. The listener drains the subscription
/// as fast as it can and parks payloads here, where `MAX_QUEUED_PAYLOADS` caps
/// the memory and drops are counted.
#[cfg(logos_scaffold)]
static INBOUND: std::sync::Mutex<Option<core::InboundQueue>> = std::sync::Mutex::new(None);

/// Which Stoas this peer has an open channel for, keyed by channel id.
///
/// The listener needs to map an arriving `channelId` back to a Stoa address, and
/// it cannot re-derive one — [`core::channel_id`] runs the other way. Held rather
/// than recomputed because inverting a derivation is exactly the shape that
/// invites a peer to accept an op for a Stoa it never joined.
#[cfg(logos_scaffold)]
static CHANNELS: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> =
    std::sync::Mutex::new(None);

/// Whether `createNode` has been called.
///
/// PLAN.md §11: "**`createNode` exactly once per context.** The delivery node is
/// a singleton per Logos Core instance". `on_context_ready` is itself guarded to
/// fire once by the generated scaffold, so this is belt-and-braces — but the cost
/// of a second `createNode` is a shared node in an undefined state, and the cost
/// of an atomic is nothing.
#[cfg(logos_scaffold)]
static NODE_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

    fn join_stoa_channel(&mut self, request: String) -> String {
        core::guarded("join_stoa_channel", || {
            let Some(dir) = self.persistence_path.clone() else {
                return core::error_json(
                    "the host has not yet told this module where its storage is; \
                     try again once the module is ready",
                );
            };
            let parsed: serde_json::Value = match serde_json::from_str(&request) {
                Ok(v) => v,
                Err(e) => return core::error_json(&format!("invalid JSON: {e}")),
            };
            let stoa = match core::parse_stoa(&parsed) {
                Ok(s) => s,
                Err(e) => return e,
            };

            // All three strings come from `core`, which is where they are tested.
            // `senderId` needs the keystore, because §4.1 makes it one per user
            // per Stoa and §5.2's per-Stoa identity is what discharges that.
            let channel = core::channel_id(&stoa);
            let topic = core::content_topic(&stoa);
            let path = core::keystore::default_path_in(std::path::Path::new(&dir));
            let sender = match core::keystore::open_from_env(&path) {
                Ok(ks) => core::sender_id(&ks.stoa_address(&stoa)),
                // A channel cannot be opened without an identity to open it
                // under, and the keystore's own message names the fix — the same
                // reasoning `get_capabilities` follows.
                Err(e) => return core::error_json(&format!("cannot derive a senderId: {e}")),
            };

            match modules()
                .delivery_module
                .channel_create(&channel, &topic, &sender)
            {
                Ok(v) => {
                    // Delivery answers `Ok` at the Rust level even when it is
                    // declining the call, with an envelope in the body
                    // (PHASE0-FINDINGS §6). Recognising that BEFORE reading the
                    // value is the whole fix, and `callee_error` is the shared
                    // decoder `channel_exists_reply` already uses.
                    if let Some(message) = core::callee_error(&v) {
                        return core::error_json(message);
                    }
                    // Record the mapping so the listener can turn an arriving
                    // channelId back into the Stoa it belongs to.
                    if let Ok(mut guard) = CHANNELS.lock() {
                        guard
                            .get_or_insert_with(std::collections::HashMap::new)
                            .insert(channel.clone(), stoa.to_hex());
                    }
                    core::channel_json(&stoa)
                }
                Err(e) => core::error_json(&format!("delivery_module.channelCreate: {e}")),
            }
        })
    }

    fn publish_op(&mut self, request: String) -> String {
        core::guarded("publish_op", || {
            let Some(dir) = self.persistence_path.clone() else {
                return core::error_json(
                    "the host has not yet told this module where its storage is; \
                     try again once the module is ready",
                );
            };
            let req = match core::parse_publish_request(&request) {
                Ok(r) => r,
                Err(e) => return e,
            };
            let log = match core::log::SqliteOpLog::open(
                &std::path::Path::new(&dir).join("ops.sqlite"),
            ) {
                Ok(l) => l,
                Err(e) => return core::error_json(&e.to_string()),
            };
            // Read the STORED bytes. Never re-encode: the bytes in the store are
            // the bytes that were signed, and a re-encode differing in any way
            // produces a payload that fails verification on every receiving peer.
            let payload = match core::outbound_for(&log, &req.stoa, &req.op) {
                Ok(Some(p)) => p,
                // Absence is a defined answer and not a storage failure, so it
                // gets its own message rather than being reported as a broken
                // disk.
                Ok(None) => {
                    return core::error_json(
                        "this peer does not hold that op, so there is nothing to \
                         publish; store it before publishing it",
                    )
                }
                Err(e) => return core::error_json(&e.to_string()),
            };

            let channel = core::channel_id(&req.stoa);
            match modules().delivery_module.channel_send(&channel, &payload) {
                Ok(v) => {
                    if let Some(message) = core::callee_error(&v) {
                        return core::error_json(message);
                    }
                    // Dispatched, not delivered. §4.4: an ACK means "some
                    // participants received it", and the outcome arrives later as
                    // `channelMessageSent` / `channelMessageError`.
                    match log.get(&req.op) {
                        Ok(Some(entry)) => core::published_json(&entry.op.op),
                        // The op was there a moment ago — this is a storage
                        // failure, not an absence, and it must not be reported as
                        // a successful publish.
                        Ok(None) | Err(_) => core::error_json(
                            "the op was published but could not be read back to \
                             confirm which; the store changed underneath the call",
                        ),
                    }
                }
                Err(e) => core::error_json(&format!("delivery_module.channelSend: {e}")),
            }
        })
    }

    fn transport_status(&mut self, _request: String) -> String {
        core::guarded("transport_status", || {
            let (queued, dropped) = match INBOUND.lock() {
                Ok(guard) => match guard.as_ref() {
                    Some(q) => (q.len(), q.dropped()),
                    // No listener yet means nothing has arrived, which is a
                    // legitimate zero rather than an error.
                    None => (0, 0),
                },
                // A poisoned lock means the listener panicked. Reported rather
                // than unwrapped: an `unwrap` here would abort the module process
                // (PHASE0-FINDINGS §3), turning a listener fault into a dead
                // interface.
                Err(_) => {
                    return core::error_json(
                        "the inbound listener has failed and its queue cannot be \
                         read; inbound ops are not being received",
                    )
                }
            };
            serde_json::json!({
                "queued": queued,
                "dropped": dropped,
                "cap": core::MAX_QUEUED_PAYLOADS,
            })
            .to_string()
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

        // The node, exactly once (§11). `createNode` with `entryLayer`
        // "channels" — its own docstring makes that the default and it is the
        // layer reliable channels live on; a kernel-only node fails every
        // `channel*` call with "no reliable channel manager".
        //
        // dialectica NEVER calls `stop()`. §4.3: the node is shared, and
        // stopping it "would tear delivery down for every other module using
        // it, which is not dialectica's call to make."
        if !NODE_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let cfg = serde_json::json!({
                "entryLayer": "channels",
                "mode": "Edge",
                "preset": "logos.test",
            })
            .to_string();
            match modules().delivery_module.create_node(&cfg) {
                Ok(v) => {
                    if let Some(message) = core::callee_error(&v) {
                        eprintln!("dialectica: delivery createNode declined: {message}");
                    } else if let Err(e) = modules().delivery_module.start() {
                        eprintln!("dialectica: delivery start failed: {e}");
                    }
                }
                Err(e) => eprintln!("dialectica: delivery createNode failed: {e}"),
            }
        }

        self.spawn_inbound_listener(ctx.instance_persistence_path.clone());
    }
}

#[cfg(logos_scaffold)]
impl Dialectica {
    /// Drain `channelMessageReceived` forever, on a thread of our own.
    ///
    /// # Why a thread, and why it cannot hold `self`
    ///
    /// The SDK delivers events by pushing into an `mpsc` from the protocol's own
    /// callback thread; an author drains it with `recv()`. There is no ambient
    /// callback registration, so somebody has to own a loop — and it cannot be a
    /// dispatch handler, because a handler that blocked would hold the instance
    /// lock forever.
    ///
    /// So the thread captures no `&mut self`. Everything it shares with a handler
    /// is in the statics above, which is the arrangement the SDK's own
    /// integration fixture uses.
    ///
    /// # The loop terminates, and that took a bug fix upstream to be true
    ///
    /// `EventSubscription` owns the `Sender` feeding its own `Receiver`, so
    /// `recv()` could once never return `Err` and a `for ev in sub` loop never
    /// ended — parking the listener forever after the provider was gone, and
    /// making any cleanup written after the loop unreachable. PLAN.md §11 records
    /// the hazard. The pinned SDK rev fixes it by polling the abandoned flag, so
    /// the loop below ends when delivery goes away instead of leaking a thread.
    fn spawn_inbound_listener(&self, dir: String) {
        // Initialise the shared state before anything can read it, so a status
        // call arriving before the first message sees a queue rather than a
        // `None` it has to interpret.
        if let Ok(mut guard) = INBOUND.lock() {
            guard.get_or_insert_with(core::InboundQueue::new);
        }

        let mut delivery = modules().delivery_module;
        let sub = match delivery.on_channel_message_received() {
            Ok(s) => s,
            Err(e) => {
                // Loud, because a peer with no listener silently receives
                // nothing: every read serves only its own posts, and that is
                // indistinguishable from a quiet Stoa.
                eprintln!(
                    "dialectica: could not subscribe to channelMessageReceived \
                     ({e}); this peer will NOT receive ops from other peers"
                );
                return;
            }
        };

        std::thread::spawn(move || {
            for ev in sub {
                // Decoding is the generated client's job; what it hands back is
                // the four fields the event declares.
                let Some(msg) = DeliveryModuleClient::decode_channel_message_received(&ev) else {
                    eprintln!("dialectica: an inbound channel event did not decode");
                    continue;
                };

                // NOTE what is NOT read from `msg`: `timestamp` and `senderId`.
                //
                // `timestamp` is the receiving peer's own CLOCK_REALTIME read
                // (§4.4, §13), so it differs per peer for one message and orders
                // nothing. `senderId` is an application-chosen string an attacker
                // sets freely (§4.1), so it reaches no decision. The Stoa comes
                // from the channel this peer opened, and authorship from the op's
                // signature.

                let Some(stoa_hex) = CHANNELS
                    .lock()
                    .ok()
                    .and_then(|g| g.as_ref().and_then(|m| m.get(&msg.channel_id).cloned()))
                else {
                    // A message on a channel this peer never opened. Dropped
                    // rather than guessed at: inverting the channel-id derivation
                    // to recover a Stoa would be how a peer comes to accept an op
                    // for a Stoa it never joined.
                    continue;
                };
                let Ok(stoa) = core::identity::Address::from_hex(&stoa_hex) else {
                    continue;
                };

                // Park it. The bound is here rather than in the ingest, because
                // the unbounded thing is the subscription's queue and draining it
                // promptly is what keeps that from growing.
                let queued = match INBOUND.lock() {
                    Ok(mut guard) => {
                        let q = guard.get_or_insert_with(core::InboundQueue::new);
                        let outcome = q.push(msg.payload);
                        if outcome != core::Pushed::Queued {
                            eprintln!(
                                "dialectica: shedding inbound load ({outcome:?}); \
                                 {} dropped in total",
                                q.dropped()
                            );
                        }
                        true
                    }
                    Err(_) => false,
                };
                if !queued {
                    continue;
                }

                // Ingest what is waiting. Opened per drain rather than held, for
                // the reason `list_threads` gives: a handle held across calls
                // would have to answer what happens when it goes stale.
                let mut log = match core::log::SqliteOpLog::open(
                    &std::path::Path::new(&dir).join("ops.sqlite"),
                ) {
                    Ok(l) => l,
                    Err(e) => {
                        // The payload stays queued, so a transient failure is
                        // retried on the next arrival rather than losing the op.
                        eprintln!("dialectica: cannot open the op log to ingest: {e}");
                        continue;
                    }
                };
                loop {
                    let Some(payload) = INBOUND
                        .lock()
                        .ok()
                        .and_then(|mut g| g.as_mut().and_then(|q| q.pop()))
                    else {
                        break;
                    };
                    // THE hostile-input boundary. Every refusal is named, and a
                    // refusal is a normal outcome on an open network — logged at
                    // a level that does not drown the log, since an attacker
                    // controls how often it happens.
                    match core::ingest(&mut log, &stoa, &payload) {
                        Ok(_) => {}
                        Err(e) => eprintln!("dialectica: refused an inbound op: {e}"),
                    }
                }
            }
            eprintln!(
                "dialectica: the inbound listener has ended; delivery is gone and \
                 this peer will no longer receive ops"
            );
        });
    }
}

#[cfg(logos_scaffold)]
#[no_mangle]
pub extern "Rust" fn logos_module_install() {
    install::<Dialectica>();
}
