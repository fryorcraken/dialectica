# Dialectica — plan

A decentralized forum on the Logos stack. Δ.

This document records decisions and the reasoning behind them. It is not a
status tracker: it says why things are the way they are, not how far along they
are. Facts a command can answer do not belong here.

---

## 1. What dialectica is

A forum whose organising concept is the **Stoa** — a sub-forum anyone may create
and moderate. Two properties held in deliberate tension:

Dialectica is a peer-to-peer application: it ships no servers and operates no
service. Each Stoa is created and moderated by its own participants, who are
responsible for the content they publish and for how they moderate it.

- **Permissionless Stoa creation.** Creating a Stoa requires no approval step
  and no registration with a central service, because there is no global
  registry to register with — a Stoa is a genesis record its creator publishes.
- **Good moderation and curation within a Stoa.** A Stoa's moderators shape it.
  A forum where nothing can be removed is not a forum, it is a firehose.

When a design decision below looks arbitrary, check it against that pair. Most
of them are the pair, applied.

---

## 2. Architecture

### 2.1 Two modules, one repo

```
dialectica/          core module — Rust, codegen.rust, interface: cdylib
dialectica-ui/       ui_qml module — QML, thin
```

The split is **forced, not stylistic**. Basecamp sandboxes the QML engine with
a deny-all network access manager and no filesystem access outside the plugin
directory. A view cannot fetch or read anything itself. All network, storage,
crypto and state lives in core.

Licence: dual MIT / Apache-2.0, matching the surrounding ecosystem.

### 2.1.1 Dual remotes: Radicle and GitHub

Both, and the split is not symmetric.

- **Radicle** is where the project lives. A peer-to-peer forum hosted solely on
  a single centralised platform would sit oddly with its own design.
- **GitHub is load-bearing and cannot be dropped**: Actions runs CI, and the
  Logos module catalogue is hosted on **GitHub Releases** — `lgpd` and `lgpm`
  fetch packages from there, so a module that is not released on GitHub cannot
  be installed by the standard tooling.

Treat GitHub as the release and CI substrate, Radicle as the canonical home.
Both remotes get every push; releases are cut on GitHub tags (§10).

Published to the catalogue at
**https://github.com/fryorcraken/logos-modules**.

Two consequences of being a monorepo — two modules under one git repo, where
the catalogue expects one module per submodule. Both were paid for once already
in radicle and transfer unchanged:

- **The catalogue's release action must support a `module_path` pointing
  *inside* a submodule.** Older versions fail checkout with "pathspec did not
  match any file(s) known to git" for this layout. Pin
  `_release-module.yml` to a fixed tag rather than a moving one, so the release
  pipeline cannot change under the catalogue without a commit here saying so.
- **`release-all.yml` auto-discovery cannot see two modules in one repo** — it
  reads module paths straight from `.gitmodules` submodule paths. `dialectica`
  and `dialectica_ui` each need their own manually-triggered workflow, not the
  umbrella.

### 2.2 The core is authored in Rust, natively

`logos-module-builder` has a first-class Rust module path — `codegen.rust` plus
a LIDL contract plus `logos-rust-sdk`. This is shipped, tested and has runnable
examples (including a cross-language one wiring a Rust provider, a C++ provider
and a Rust consumer through `logoscore`).

Consequences, all verified against the SDK source and its doctests:

- **No hand-written C++ shim.** The author writes a Rust trait impl and one
  `logos_module_install()` hook.
- **No `build.rs`, no `buildRustPackage`.** The builder supplies
  `logos-rust-sdk` at the exact rev the generator came from, compiles the crate
  to a staticlib, and links it. `CMakeLists.txt` is one line:
  `logos_module(NAME dialectica)`.
- **Rust-first authoring.** Setting `codegen.rust.trait` makes the builder
  derive the `.lidl` contract *from the Rust trait*. The `.rs` is the single
  source of truth for the API. Given how much this project cares about the core
  API being deliberate, this is the right direction — the contract cannot drift
  from the code because it is generated from it.

The outbound call surface is `lp_*` — plain C ABI, JSON over strings, **no Qt
types**. The builder generates a typed Rust client per declared dependency:

```rust
modules().delivery_module.send(topic, payload)
```

Qt exists only as a host transport detail, invisible at this boundary.

> `StdLogosResult` is std-C++ (it holds `nlohmann::json` and `std::string`), not
> C-ABI. It belongs to the C++ "universal" module style. Do not try to bind it
> from Rust — the Rust path does not go through it.

### 2.3 The SDK: use it, do not fork it

`logos-rust-sdk` (MIT/Apache-2.0) is the runtime. Forking was considered and
rejected — not on quality grounds, but because **forking it alone accomplishes
nothing**: `logos-module-builder` supplies both the generator and the SDK source
from its own single pinned input, staged beside your crate. Your `Cargo.toml`
does not control that dependency. A fork means also forking or overriding the
builder, then maintaining a generator against an `lp_*` ABI that moved through
protocol 0.5→0.9 in a single month — with upstream issue #4 openly proposing to
rework the SDK's foundation. That is permanent rebasing.

**Contribute upstream instead**, and design around what is missing.

#### What is missing, and what dialectica does about it

- **There is no panic guard.** No `catch_unwind` anywhere; the default profile
  unwinds. Every generated `extern "C"` dispatch calls straight into author
  code, so a panic unwinds through an `extern "C"` frame — undefined behaviour.

  > **Measured, and worse than this predicted** (PHASE0-FINDINGS §3). The
  > mutex-poisoning below is real code but is never reached: the process
  > **aborts** — `failed to initiate panic, error 5`, SIGABRT — before any
  > later dispatch can meet a poisoned lock. The caller waits out its 20s
  > timeout, and every call after that gets `MODULE_NOT_LOADED`. So the guard
  > is load-bearing rather than hardening, and poison tolerance is the
  > secondary fix, not the primary one.

  The mechanism that would bite if the process survived: the panic poisons the
  `INSTANCE` mutex and dispatch locks with a bare `.unwrap()`.

  This is the same defect radicle hit and fixed with its `guarded()` wrapper,
  and it matters here specifically: parsing untrusted forum content inside a
  dispatch handler is exactly where a panic happens. Until it is fixed
  upstream, **guard at our own boundary** — no handler may unwind. Upstreaming
  it is a ~30-line change to the dispatch emitter plus poison-tolerant locks,
  and is the highest-value contribution available.

- **Event queues are unbounded.** `std::sync::mpsc` with no backpressure, no
  bound and no drop policy; the C trampoline never blocks and never fails. A
  consumer slower than the event rate grows the queue until OOM — and ingesting
  `channelMessageReceived` during a backfill is precisely that shape. **Bound
  it ourselves.**

- **There is no mock host**, and this is not a testing inconvenience — it is an
  architectural constraint. Anything touching `modules().<dep>` or `context()`
  calls `lp_*` symbols that are undefined in an rlib and will not link into a
  test binary.

  So: **all domain logic lives in a pure inner crate with zero SDK types** —
  ops, signatures, SQLite, ordering, moderation rules — and the trait impl is a
  thin adapter over it. This is forced by the SDK rather than supported by it,
  and it happens to be the right structure regardless (§9 Phase 1 assumes it).

- **The builder's pinned SDK rev lags HEAD.** Check what the pin actually
  delivers before designing against upstream documentation — the README
  describes HEAD, the code you link may not be it. The gap has included
  subscription status and restart policy (without which `recv()` blocks forever
  and a dead provider hangs a listener thread permanently), per-call timeouts,
  and argument type checking (without which a wrong-typed argument silently
  becomes `0` or `""`).

#### Constraints the SDK imposes on our shape

- The module instance is **`Default`-constructed and a process-global
  singleton**. No constructor injection: you cannot hand it a config or an open
  database handle. State reachable from both dispatch and a background thread
  lives in `static`s or `Arc`s rather than an owned struct.
- **`concurrency` in metadata decides threading.** `single` (the default) takes
  the instance mutex for the whole handler, so calls serialize; `multi` gives
  `&self` and overlapping handlers, and you own interior mutability.
- **No async runtime, and none needed.** `EventSubscription` is `Send` by
  design: subscribe in `on_context_ready`, move the subscription into a thread
  we own, and block on it there. Background work — the sync loop, SQLite,
  signature verification — lives on our threads, not the module's event loop.

### 2.4 Two facts that shape the core API

**Every `modules().x` call is IPC.** Modules run in separate processes over Qt
Remote Objects unix sockets. Never put a cross-module call in a hot loop.

**Async callbacks run on the module's event loop, after the current method
returns.** You cannot await a cross-module result inside a method. The shape is:
fire, stash, read back through a later call. Design the API around this rather
than discovering it.

### 2.5 The wire contract

Adopted wholesale from the ecosystem convention:

- Every method takes and returns JSON.
- Failure is **always** `{"error":"..."}`. Never a partial success shape.
- Paginated calls take `(page, perPage)` and return
  `{"items":[...],"page":N,"hasMore":bool}`.
- Shapes are source-independent, so a view renders without branching on origin.

Widening this interface is a deliberate decision, not a side effect of needing
one more field.

**The contract is the `module-wire-contract` spec, which is where to read it.**
It states both halves of the envelope — a request is an object and so is a
reply — the one failure shape, the panic guard, and how a callee's own failure
is decoded. This section is the summary; the spec is the obligation.

---

## 3. Dependencies

| Module | Role | Consumed as |
|---|---|---|
| `delivery_module` | SDS reliability channels — the sync layer | module dependency |
| `storage_module` | image and attachment bytes (§4.6) | module dependency |

Consumed **as module dependencies**, not vendored — which keeps dialectica
inside the Logos module system.

### 3.1 Why not cloud_data_module

Considered and rejected, and the reasoning is architectural rather than a
judgement on the code.

`cloud_data_module` solves convergence with **CRDTs over plain pub/sub**. It
does not use SDS, and does not need to: every op (a signed operation — §3.3
defines the term) is idempotent by `opId`, safe to apply out of order or
repeatedly, so convergence comes from merge semantics rather than from transport
ordering.

Dialectica takes **SDS as the primary reliability layer** (§4). Those are
*alternatives, not layers*. Running both would pay SDS's per-message bloom
filter and causal history cost for guarantees a CRDT already provides, and
would leave two convergence mechanisms to reconcile.

Two of its three jobs were unusable for a forum regardless: `query()` is a full
table scan with no index, sort or pagination — which is precisely what a forum
feed is — and it has no op authenticity, so moderation flags are forgeable.
Having replaced both, owning the third (the oplog and local store) is a small
marginal cost for full control of the data path, no dependency on a v0.1.0
single-contributor prototype, and no licensing question (the repo currently
carries no licence at all).

**The later move is to extract our own SDS + storage layer as a reusable
module** — what cloud_data was reaching for, built on SDS rather than beside it.

### 3.2 The one unproven link

`delivery_module` v0.2.1 does not publish a `lidl` flake output. Consuming it
from Rust therefore needs a `dependency_overrides` entry.

> **Retired by Phase 0 — and the header form below does not work.** Pointing
> the override at an impl header fails from a Rust module: the `--dep` path
> feeds the *LIDL parser*, and only the C++ generator has a header frontend.
> The working form commits a converted `.lidl` and points at that. See
> `docs/PHASE0-FINDINGS.md` §1 for the error and the conversion, and §6 for the
> bridge carrying a live cross-module call. The architecture stands; this
> section's JSON does not.

The form that does **not** work, kept because `impl_class` is documented as
required for exactly this case and the failure names nothing useful:

```json
"dependency_overrides": {
  "delivery_module": {
    "file": "…/delivery_module_plugin.h",
    "input": "delivery_module",
    "impl_class": "DeliveryModuleImpl"
  }
}
```

What is *not* in doubt: the channel API this plan is designed against exists
upstream at v0.2.1 — `channelCreate`, `channelSend`, `channelClose`,
`channelExists`, and the `channelMessageReceived` / `Sent` / `Error` events —
and the generator has a regression test for decoding exactly the binary event
payload shape we need, so a typed Rust client with `Vec<u8>` payloads is the
expected output. The open question is the bridge, not the API.

**The contract is in this repo, so read it rather than reasoning about it:**
`dialectica/contracts/delivery_module.lidl` is the converted `.lidl` the override
points at (above), and it is the whole surface. Everything §9.2's MVP needs is
there: `createNode`, `start`, `stop`, `send`, `subscribe`, `unsubscribe`,
`storeQuery`, the four `channel*` methods, `getNodeInfo` /
`getAvailableNodeInfoIDs` / `getAvailableConfigs`, `collectOpenMetricsText`; and
the events `messageSent` / `messageError` / `messagePropagated` /
`messageReceived`, `connectionStateChanged`, `channelMessageReceived` /
`channelMessageSent` / `channelMessageError`, `nodeStarted` / `nodeStopped`.

> **A correction, recorded in place because of how it happened.** It was stated
> in session that the contract **lacked** `createNode`, `channelCreate` and
> `channelSend`. **That was wrong** — all three are in the file above, with
> docstrings. The claim came from a transport spike's report rather than from
> opening the contract, which is the same failure §13's `SdsDeliverable` note
> records and the same one §12 warns about for the stale working trees: **a
> summary of a file is not the file.** The file is committed here and costs one
> `Read`. There is no reason to take a second-hand account of it.

### 3.3 The local store is ours

**An "op" is a signed operation, and it is the unit everything else is built
from.** Creating a Stoa, posting, publishing a revision of your own post (§5.7),
hiding something as a moderator (§6), voting — each is one op, signed by its
author and published to the Stoa's channel. Nothing else crosses the wire, and
the forum's whole state is a function of the ops a peer has seen.

Each peer is to keep a **local SQLite store** holding every op it has seen, plus
a materialised view of the forum derived from it. Ops are the authority; the view
is a cache that can be rebuilt by replay.

**The op log exists behind a `Store` trait** (see the `op-log` spec), with two
implementations: in memory, and on disk in SQLite. **The materialised view and
its query indexes are still to build** — the log is the authority and it
persists; the cache derived from it does not exist.

The log stores **inputs and never conclusions**: one row per op, carrying who
signed it, what it acts on, and where the transport placed it. No score, no vote
tally, no hidden flag. That line is where §7.2's cheap-retuning promise comes
from, and §6's read-time authority check is what forces it — a weight is not
knowable when an op is stored. The `sqlite-projection` change's `design.md` has
the argument and the shape the view will need.

Two peers routinely hold **different sets of ops** — one was offline, one joined
late, a message has not propagated yet — so they can legitimately disagree about
what the forum currently looks like. §4.4's eventual consistency is what closes
that gap over time, and §7.2 rule 1 is what stops the divergence being mistaken
for a bug.

This is the piece cloud_data would have provided, and owning it is what lets us
index for the queries a forum actually makes — newest threads, paginated
replies, a Stoa's index — rather than scanning every row.

**Op authenticity is dialectica's job, not the transport's** (§6). A forged op
cannot be prevented from *arriving*: SDS has no membership and `senderId` is
self-asserted.

~~Verification therefore happens on **read**, filtering unsigned or
badly-signed ops out. The store may hold junk; the reader never trusts it.~~
**Specified, and this is not where verification happens — see the
`op-transport` spec**, whose requirement "Every inbound payload is validated
before it reaches storage" refuses an unauthentic op at the transport boundary
rather than storing it for a reader to filter. A reader that does not trust the
store is still the right posture, and remains why `op-log` verifies nothing on
append; but "the store may hold junk" was never the design for ops arriving from
a peer, and reading it as licence to append unverified ops is the forgery-storage
failure that boundary exists to prevent.

**A generic op decoder may be wanted eventually, and is not built.** Every op
decodes attacker-controlled bytes with the same failure modes — truncation,
trailing bytes, a length prefix lying in either direction, an unknown version —
and the genesis record's decoder (see the `stoa-genesis` spec) already has all
of them, with a bounds-checked cursor as the plausibly reusable half.

There is one decoder, so there is nothing yet to generalise from: an abstraction
derived from a single instance is a guess. Noted here only so the second decoder
is written knowing the first exists — that is when the seam becomes visible, and
whether there is one at all. One reason to expect the fit to be awkward: a
genesis record is **self-identifying by hash**, its encoding being its address
preimage, where a post or a moderation op is addressed by its own id and carries
a signature.

---

## 4. Addressing and transport

### 4.1 One reliability channel per Stoa

**Specified — see the `op-transport` spec**, which carries one channel per Stoa,
channel identity as a pure function of the Stoa address, the sender identifier
never reaching an authorisation decision, and the receive-side validation
boundary. The behaviour below is struck through; the reasoning under it is not,
because it is what the spec deliberately does not carry.

`channelCreate(channelId, contentTopic, senderId)` decouples channel from topic.

- ~~`contentTopic` = the Stoa, hashed and bucketed: `/dialectica/1/s/<hex>/proto`~~
- ~~`channelId` = the Stoa, and **the same value for every peer in it** — it is the
  rendezvous, not a local handle~~
- `senderId` = **one per user per Stoa, permanent** — every participant's is
  different (the API requires it); what is stable is that a given user keeps
  theirs across sessions. A transport self-filter, not an author identity; see
  below. **Not yet built**: §9.2's MVP ships one identity per user, so the
  per-Stoa scope this bullet assumes is the destination rather than the present
  state.
- ~~`threadId` and `parentPostId` live in the **payload**, never the topic~~

**`senderId` is not an author identity, and the plan should not treat it as
one.** It exists so SDS can tell a participant's own messages from everyone
else's: the spec notes that "outside of filtering messages originating from the
sender itself, the `sender_id` field is not used for much", and the receive step
is a SHOULD to "ignore the message if it has a `sender_id` matching its own".
Acknowledgement accounting runs off message ids carried in causal history and
bloom filters, not off sender ids — so reliability does not depend on a
`senderId` meaning anything in particular.

**Do not implement that SHOULD.** It describes SDS, one layer below the event
dialectica receives, and on the reliable-channel path the filter it asks for has
nothing to filter: `channelMessageReceived` does not fire for a participant's own
messages, so a self-filter keyed on the sender identifier is a branch that never
executes — and a dev who writes it will believe their own ops are being
deduplicated by it rather than by op id. Specified: the `op-transport` spec's "A
peer's own published op is not received back as an arrival", which is what makes
storing on publication the only route by which a peer holds its own op. The
asymmetry itself is in §11's trap list, because it presents as a storage bug.

**The application owns it.** `channelCreate(channelId, contentTopic, senderId)`
takes it as a parameter, and nothing in the delivery module persists it: the SDS
persistence backend covers causal history and outgoing buffers, the segmentation
backend covers partial reassembly, and identity is in neither. The Reliable
Channel API says it SHOULD be "unique and persisted between sessions" — that
duty is dialectica's, and §5.2's per-Stoa identity is what discharges it.

**It binds at channel creation**, so a peer's own `senderId` is fixed for as
long as it holds that channel open. With one channel per Stoa and one permanent
identity per user per Stoa (§5.2), these agree by construction — a user's
`senderId` is stable exactly where SDS wants it stable, and no rotation
machinery is needed. Changing it would mean closing and re-opening the channel,
which §5.2 rules out for a different reason: the identity is permanent, so
there is nothing to rotate to.

**What a reliable channel discloses.** Every receiving peer is handed the sender
id with every message — `event channelMessageReceived(channelId, senderId,
payload, timestamp)` in `dialectica/contracts/delivery_module.lidl`. So a
`senderId` links everything sent on its channel, which under per-Stoa identity
is the intended scope: it links a Stoa's posts to one pseudonym and no further.
Plain messaging carries no such field — `method send(contentTopic, payload)` and
`event messageReceived(messageHash, contentTopic, payload, timestamp)` in the
same contract have no sender at all. That asymmetry is the lever for any future
scope question: reliability is what introduces the identifier, so the identifier
can be avoided exactly where reliability is not needed.

**The trap this leaves for anyone revisiting §4.5.** A per-thread channel split
would keep a Stoa-level channel carrying the thread index, and a user announces
every thread they start on that one channel under a single `senderId` — relinking
every thread they created, and defeating the point of splitting. The rule to apply
if that day comes: reliable channels where the participant set is already the
anonymity set, plain pub/sub where it is not, since `senderId` exists only to
serve SDS reliability and a thread index needs availability rather than
ordering. Note that this removes a protocol-level identifier, not a
network-level one — publishing still emits a signal, and Filter/Store/LightPush
disclose content topics to peers (below).

**The general trap, worth stating once:** a pseudonym is only as unlinkable as
the most broadly-scoped identifier travelling with it. Any field attached at a
scope wider than the pseudonym — a session id, a presence marker, a per-Stoa
ack token — reintroduces exactly this leak.

**SDS Repair (SDS-R) is wanted where available, and it constrains this.** Its
repair backoff computes a distance from the original `sender_id`, and its
response-group membership is derived from it so that a sender is always in the
group for its own messages; it also adds `sender_id` to `HistoryEntry`, exposing
sender ids for other people's messages. A `senderId` that a user keeps for as
long as they are in the Stoa satisfies all of that; one changing more often than
the channel would not. This is an independent reason the scope in §5.2 is the Stoa rather than
anything narrower — and the SDS spec expects `sender_id`'s "importance ... to
increase once a p2p retrieval mechanism is added", so the tension grows rather
than fades.

**Never put a human-readable Stoa name in a topic.** Filter, Store and
LightPush disclose content topics to peers, linking IP to interest. Hashed
buckets also buy k-anonymity.

### 4.2 Content topics give no routing relief

Autosharding hashes only `application` + `version` — the topic name is ignored.
Every `/dialectica/1/*` topic lands on one shard, and a Core-mode node
subscribes to all shards in the cluster regardless, filtering locally.

**Topics are a local filter, not routing.** Minting more of them buys nothing
and costs real money: LIP-23 measures Store query response time doubling from
10 to 100 content topics. One topic per thread would be actively wrong.

### 4.3 The channel id is the rendezvous

**`channelId` names the conversation, not our handle on it.** Every peer in a
Stoa must compute the same value or they cannot see each other. The clearest
statement is in the JS SDK tutorial (`docs.waku.org`, "Reliable channels"),
which is informal prose rather than a normative spec but describes the intent
exactly: the channel name "acts as an identifier to the conversation,
participants will try to ensure they all have the same messages within a given
channel."

The contrast with `senderId` is the tell — two adjacent parameters in one call
with opposite requirements, one to be agreed on and one to differ. Where that
requirement is stated matters, because the layers disagree in strength:

- **SDS is the strong one.** A Participant ID is "globally unique, immutable"
  (`sds.md`, design assumptions), and the sender "MUST include its own globally
  unique identifier in the `sender_id` field".
- **The Reliable Channel API is weaker**, asking only that `senderId` "SHOULD be
  unique and persisted between sessions".

So uniqueness is a MUST underneath and a SHOULD at the surface, and the
immutability SDS assumes is what §5.2's permanent per-user identity supplies.
(The tutorial's bolded "every participant **must** have a different id" is that
same requirement in informal prose — quote the spec, not the tutorial, when the
strength of the obligation is the point.)

~~**So the channel id can carry no per-peer state.**~~ **Specified — the
`op-transport` spec's "Channel identity is a pure function of the Stoa address"
carries this, including that a deterministic epoch is not a permitted variant of
it.** Kept in one line because it is the rule the rest of this section reasons
about: a value that differs between peers produces no error, it produces two
Stoas that cannot see each other, silently and permanently.

**Set aside, 2026-09-12: `logos-messaging/logos-delivery#4116`.** The issue
reports that closing a channel which has received a peer message and then
re-creating it with the same id kills the node process. **We are no longer
designing around it.** It was never deterministically reproduced, the spike
that was meant to settle it never created a node, and an undetermined bug was
shaping a real user-facing restriction. Treat the documented behaviour as
correct until something here actually fails.

**What this does not change:** the reason dialectica closes channels at all.
That argument is below and rests on the shared node, not on any bug.

**What it does change** is recorded at the end of this section.

~~**Dialectica closes a channel in two places: when a user leaves a Stoa, and on
shutdown.**~~ **Specified — the `op-transport` spec carries both closes, that the
shared node is never stopped, that closing is best-effort, and that a closed
channel is reopenable under the same identity.** The argument for why the second
close is not optional stays below, since the spec states the obligation and not
the reasoning.

**The delivery node is not ours to stop, and it outlives us.**
`delivery_module` is a separate, shared process — `createNode` is called once
per context (§11), and when the node dies, any other module sharing that node
loses it too. So dialectica exiting does not
stop the node, and the node's own `stop()` is not the alternative: calling it
would tear delivery down for every other module using it, which is not
dialectica's call to make.

**So the unsubscribe that a close performs is the point.** A channel left open
keeps the shared node doing work for a Stoa belonging to an app nobody has open,
and what that costs depends on the node's mode:

- **Edge mode**: filter subscriptions and the remote peer slots serving them
  stay alive. That reaches past our process and consumes someone else's
  resources.
- **Core mode**: the topic keeps running its handler chain and SDS loops. Local
  CPU rather than network membership — §4.2 already establishes that a Core node
  subscribes to every shard in the cluster regardless and filters locally, so
  **dropping a content topic does not leave a gossipsub mesh.** Any argument
  that it does contradicts §4.2 and is wrong.

Either way it is work done on behalf of a user who has closed the application.

Two caveats, because the close is less decisive than it looks: the unsubscribe
is **refcounted by content topic**, so it only takes effect when the last
channel on that topic goes; and it is **best-effort**, with failures logged at
debug and not surfaced. The module contract's docstring for `channelClose` says
only "stops its SDS loops" and does not mention the unsubscribe at all.

That is why both cases close: leaving a Stoa and shutting down are the same
situation — a channel that must not outlive the app that opened it.

~~**Re-creating a channel is allowed.** Close and reopen the same id within one
node's lifetime — to recover from an error, or because a user left a Stoa and
rejoined. There is no epoch in the channel id and no restriction on reopening.~~
**Specified as a requirement rather than a permission** — the `op-transport`
spec's channel-lifecycle requirement contracts that a closed channel is
reopenable under the same identifier and without a restart.

An earlier version of this section forbade all of that to avoid #4116, at the
cost of one real limitation: **rejoining a Stoa required a restart.** That
limitation is withdrawn along with the premise.

~~**The channel id stays a pure function of the addressed object.** No epoch in
it — not a per-peer one, and not a deterministic one either.~~ Specified above.
**Both exclusions' reasoning stays here, because the spec states the rule and not
the judgement behind it:**

The per-peer form is ruled out by this section's own rule: peer A reopens at
`stoa-abc/e8` while peer B is still on `stoa-abc/e7`, and they stop seeing each
other with **no error anywhere** — a silent permanent partition, which §4.5
rules out independently of any bug.

**A deterministic epoch is ruled out for a different reason, and it is a
judgement about whose problem this is.** Making every peer recompute a matching
epoch is a rendezvous problem at each boundary — a value that changes must
change for everyone at once — and dialectica would carry that complexity
forever in order to route around a defect in `logos-delivery`. **If re-creating
a channel kills the node, that is an upstream bug and it gets fixed upstream.**
Do not reintroduce an epoch here as the remedy.

### 4.4 What SDS does and does not promise

Promises: eventual consistency among *active participants* — Lamport total
order (initialised to epoch-ms so new joiners order correctly), causal-history
buffering, bloom-filter-inferred ACKs, retransmission of unacknowledged
messages, deterministic tie-break by ascending message id.

Does not promise:

- **No confidentiality.** Payloads travel unencrypted unless the application
  encrypts them.
- **No authenticity.** `sender_id` is an application-chosen string. Sign your
  own posts.
- **No membership.** Anyone can join a channel.
- **No delivery to absent peers.** ACK means "some participants received it".
- **No ordering metadata reaching the application** — ~~and what a receiving peer
  therefore records~~ **is specified: the `op-transport` spec's "An arrival over
  this transport carries no ordering metadata" and "The arrival timestamp is a
  local clock reading and orders nothing".** The Lamport total order and
  the message-id tie-break above are real and are what SDS orders its own log
  by — but they stop below us. What dialectica receives from the delivery
  module is `channelMessageReceived(channelId, senderId, payload, timestamp)`,
  and that `timestamp` is the *receiving peer's own clock read*, not a wire
  value — so nothing in it orders anything. (Do not read those four fields as
  contradicting §13's "one field": that is the Reliable Channel event one layer
  further down, and the delivery module adds the three it can supply locally.)
  **Read the promises above as internal to SDS, not as an interface** — which
  is the right posture regardless of what upstream forwards. SDS is transport;
  ordering at forum scope is dialectica's, carried in the signed op. §13 has
  the layers the values are dropped at and the layering rule that makes this a
  division of labour rather than a blocker.
- **150 KiB max message size**, hard cap — a network-wide gossipsub validation
  limit, not unilaterally raisable. See §4.6.

Status: SDS is LIP-109 at **raw**, the weakest maturity tier, and the channels
API is marked Developer Preview with "expect the API surface to change".

### 4.5 The scaling path, deliberately deferred

Not built now. Named so the data model does not foreclose it:

> One SDS channel **per thread**, with the Stoa's channel carrying only the
> thread index.

This turns one hot channel into many cold ones and drops the participant set per
channel to people actually in that conversation.

To keep it cheap: ~~derive `channelId` as a pure function of the addressed object~~
— **specified for the Stoa case in the `op-transport` spec; `(stoa, thread)` is
the part still ahead** — and never let channel identity leak into payloads or
storage keys. Ops carry `threadId` from day one anyway (the topic cannot carry
it), so the split becomes a routing change rather than a migration.

### 4.6 Images and attachments go to Logos Storage

Posts carry a **CID**; `storage_module` holds the bytes. This sidesteps the
150 KiB message cap cleanly — no chunking protocol of our own, and no large
blobs in the op log, where they would bloat both SDS traffic and later
snapshots.

Logos Storage has **no identity concept at all**, and its persistence is
interest-driven rather than guaranteed — *"if no one is interested in your
files, chances are that losing your node means your data is lost."*

**So replication is dialectica's job, and the model is: readers become
seeders.** Anyone who downloads an image serves it afterwards, which makes a
popular attachment progressively better replicated — the property Storage's
organic-replication design is built to reward. The **author seeds their own
attachments for a long period**, so a post is not dependent on someone else
having read it yet.

Consequences to design for, none of which need solving in v1 but all of which
should be decided before attachments ship:

- **How long the author seeds**, and whether that is bounded at all. Indefinite
  is simplest and probably right for a desktop app.
- **Whether a reader can opt out** of re-seeding. Some will want to.
- **A dead CID is not a broken post.** A post whose attachment cannot be
  resolved is still a valid, verifiable post — the CID is a field in a signed
  op, and unresolvable is a *fetch* outcome, not a validation failure. Render
  the attachment as missing; never let it invalidate the post or the thread.
- **No re-pinning of someone else's content by default.** Automatically
  re-hosting whatever arrives is how a client becomes a distributor of things
  its user never chose to store.

### 4.8 Stoa discovery

Permissionless creation (§1) means there is no registry to look a Stoa up in.
Discovery is therefore a separate problem from addressing, and it is staged —
each phase is independently useful and none depends on a later one landing.

**Phase 0 — one embedded Stoa.** A single well-known Stoa ships with the app.
No discovery mechanism at all, and none needed to prove the rest of the system
works.

**Phase 1 — addresses as links.** A Stoa address is a copyable string, and a
Stoa address appearing in a post renders as a link that offers to enter that
Stoa. This is the whole mechanism, and it is enough for a network that grows by
word of mouth.

~~Importing an address is how you join a Stoa nobody told the app about.~~
~~An address must be **self-authenticating** — pasting it is enough to verify
what you joined.~~ **Retracted; the joining half is built.** Creating, joining
and listing Stoas are contracted by the `stoa-membership` capability, which
states that **an address alone is not joinable**: the address is a one-way hash
of the genesis record, so it is sufficient to *verify* a record somebody hands
over and insufficient to *reconstruct* one. A join therefore takes the address
**and** the record it names. This is the same error §5.5 records having made
twice in its own signature; the sentence above stated the false half more
confidently than the retraction, which is why it is struck here rather than
merely cross-referenced.

What survives of the self-authentication property, and it is the load-bearing
half: the address is a hash of the genesis record (§5.1), so a wrong or tampered
record **fails to match the address it is offered with**. Verification needs
nothing but those two inputs — no registry, no peer, no network call.

Still to get right, and still not built: in-post addresses are
**attacker-supplied content**. Render them as an explicit affordance the reader
chooses to act on, never auto-join, and show what is being joined before joining
it. That is a UI obligation — see §5.5, which holds it.

**Phase 2 — opt-in broadcast.** A dedicated content topic, **outside SDS**,
carries Stoa announcements. A creator decides at creation whether their Stoa is
broadcast or unlisted; any user can rebroadcast the ones they know, so
availability does not depend on the creator staying online.

Deliberately not SDS: this is a gossip of independent facts with no ordering or
causal relationship, so SDS's causal history and bloom-filter machinery would be
cost without benefit. Plain publish/subscribe is the right shape.

Consequences to design for: the topic is **unauthenticated and spammable** —
anyone may announce anything, including Stoas that do not exist or that
impersonate another by name. Announcements are therefore *hints*, verified
against the genesis record before being shown as real, and names in this
listing are never treated as unique.

**Phase 3 — on-chain curation.** A curated Stoa index, following the model
Logos is building for community-curated app catalogues (documented internally as
"Catalog Bootstrap / On-boarding", λ-Prize `logos-co/ecosystem#211`).

The structural idea transfers cleanly, and is worth stating in dialectica's own
terms because it is what makes curation compatible with §1:

- A LEZ program holds a **list of Stoa addresses**, bounded by a maximum entry
  count, with a defined add/remove API.
- The program supports **namespaces**. Each namespace pairs a list instance with
  a pluggable **governance program**. Nothing about *who* curates is fixed in
  the list program itself.
- Once a namespace is created, its list and governance pairing is fixed —
  but **anyone can deploy a new namespace** with their own list and their own
  governance, and any client can point at it.

That last property is the important one. A curated index is not a registry a
Stoa must be in; it is one opinion about which Stoas are worth showing first,
and a competing opinion costs a new namespace. Dialectica points at one by
default; a fork or a different build points elsewhere.

**Two properties dialectica must preserve for this to stay curation rather than
gatekeeping:**

- **Import-by-address keeps working regardless.** Phase 1 must remain fully
  functional for Stoas no index lists. In the catalogue design this is served by
  an explicit app-triggered install path, precisely so that a curation app which
  is *not* the default one stays usable. Dialectica's equivalent is that
  pasting a Stoa address never consults an index.
- **A bounded list is a feature, not a limitation.** An unbounded index is
  trivially spammable regardless of governance quality, and unscannable by a
  user anyway. But a cap makes inclusion **competitive once full** — adding
  implies displacing — and the governance layer needs a defined answer for that
  (ranking, explicit swap proposal, or inclusion simply failing until a removal
  passes). Whether the cap is fixed at deployment or adjustable by governance is
  an open question in the catalogue design too.

**Inherited open questions.** The catalogue model is not finished, and
dialectica would meet the same three:

- **Placeholder governance.** An index needs *some* governance program set from
  the start, even a no-op one, and whether it can be swapped later is the
  question that matters.
- **Migration.** When the default index repoints, users already on the old one
  need defined behaviour — silent switch, prompt, or no change until reinstall.
- **How the pointer itself is updated** — build-time constant, remote config, or
  user-visible setting. This determines whether repointing needs an app release,
  which is a real coupling to someone else's schedule.

That coupling is the argument for keeping **Phases 1 and 2 fully sufficient on
their own**. Curation is an enhancement to the first-run experience; it can be
late, or never arrive, without blocking anything.

### 4.7 Durability — and what v1 deliberately does not have

Three tiers, only two of which are in v1's scope:

| Tier | Covers | in v1's scope |
|---|---|---|
| SDS window | recent ops, retransmission, causal-history and SDS-Repair backfill | ✅ |
| Local SQLite | everything this peer has ever seen | ✅ — built; ops survive a restart |
| Logos Storage snapshots | deep history, beyond what live peers hold | ❌ later |

The middle tier holds the **ops**. The materialised view derived from them is
not built, so a restart keeps everything a peer has seen and rebuilds what it
renders by replay — which is §3.3's arrangement working as intended rather than
a gap.

**SDS gives real but bounded backfill.** A peer receiving a message whose
`causal_history` names ops it lacks buffers and fetches them, and **SDS-Repair**
has peers rebroadcast missing messages with pseudorandom backoff (`T_min` ≈30s,
`T_max` 120–600s) sharded into response groups of ~128. A joining peer does pull
history from other participants.

What bounds it:

- **Causal history is 2 entries deep by default** (`sdsCausalHistorySize`), so
  walking backwards is a chain, not a jump.
- **SDS-Repair is an optional extension**, and its spec says plainly it is *"not
  meant to replace mechanisms for long-term consistency."*
- Dependencies unmet after a timeout may be marked **irretrievably lost**.
- It reaches only what **live participants still hold**. A Stoa that has been
  quiet longer than peers' retention has nothing to serve.

So: recent history arrives on its own; deep history is what snapshots are for.
That is the honest v1, and it is not a regression against the alternative —
cloud_data's backfill was off by default and required a store-peer multiaddr
that `delivery_module` cannot enumerate.

**Local retention is what makes v1 usable.** A peer that has been in a Stoa for
months holds months of history locally, regardless of the network's two-day
window. The forum is readable for the people already there; it is not yet
*shareable backwards* to newcomers.

Delivery's Store is **not** a substitute: two-day default retention, no
availability guarantee, and `storeQuery` is flagged "USE AT YOUR OWN RISK"
(kernel API, may change without deprecation) and needs a manually supplied
store-peer multiaddr.

**Snapshots are the next thing built after v1.** The seam to get right:
a snapshot cadence, publishing the CID on a control topic, letting a joining
peer find the latest, and stitching "snapshot at Lamport T" to "ops since T"
without gaps or double-application. That last part is the fiddly one.

Prior art worth reading when that work starts, not before:
**LP-0017 (Whistleblower)** in `logos-co/lambda-prize` — a worked example of the
Logos Storage CID + Delivery broadcast pattern, extracted into a reusable
`logos-chronicle` module alongside a QML view plugin. Not SDS-based, so it is a
reference for the storage pipeline shape rather than something to depend on.

---

## 5. Identity

### 5.1 The construction

**Address = hash of a small genesis record**, in the style of LEZ private
accounts — which use `AccountId = SHA256(prefix || npk || vpk || identifier)`
with a `u128` diversifier giving 2^128 addresses per keypair.

Hash a **record**, not the bare public key:

```
address = H(prefix || genesis_record)      # record holds one key today
```

Costs the same today. If rotation ever lands, the record can hold a key *log*
and the address survives instead of forcing a migration. Do not foreclose it by
hashing the raw key.

### 5.2 Scope: one identity per Stoa, permanent

A user has **one identity within a Stoa**, stable across every thread in it and
across sessions. Identities are unlinkable **across** Stoas — derive the
diversifier from the Stoa id so per-Stoa pseudonyms fall out of a single root
key for free.

A stable pseudonym is what lets anything at all accumulate against an identity —
a moderator's judgement, and more importantly the relevance signals §7.2 is
built on. An identity that does not persist is one nothing can be said about.

**Thread-scoped identity was explored and rejected**, and the reasoning is worth
keeping because the motivation was real. Rotating keys between threads would buy
per-thread unlinkability *within* a Stoa — an observer could not say "whoever
argued X in thread 1 is whoever argued Y in thread 47" — and LP-0016 makes that
case well: a persistent handle accrues social history, readers pre-judge by it,
and a minority view expressed early suppresses that account's later
participation.

Three things decided against it, in ascending order of how conclusive they are:

- **Nothing can accumulate against an identity that lasts one thread.** That is
  a loss for moderation, but the sharper cost is to §7.2: relevance weighted by
  a credential needs the credential to attach to something that persists.
- **The privacy gain is much smaller than it looks.** Writing style, posting
  time, the reply graph and the network layer all still link a user's thread
  identities, and every peer holds the op log needed to do it. It defeats a
  casual reader profiling a handle; it does not defeat a motivated observer. In
  a Stoa with four active participants it is theatre outright — the anonymity
  set is the participant set.
- **It is incompatible with SDS-R, which this design wants.** SDS Repair's
  backoff and response-group arithmetic both assume a sender id that is stable
  and still answered to, and `senderId` binds at `channelCreate` for a channel's
  lifetime (§4.1). One channel per Stoa, plus an identity each user keeps for
  that Stoa, makes the transport identifier stable exactly where SDS wants it,
  with no rotation machinery to build.

The last is the decisive one, and it is independent of the privacy argument: it
would rule out narrower scopes even if the unlinkability gain were larger than
it is.

**What is protected, stated exactly:** cross-Stoa unlinkability. A user's
identity in Stoa A cannot be tied to their identity in Stoa B by the protocol.
Within a Stoa, a pseudonym is stable by design, and §4.1's `senderId` links a
Stoa's posts to that one pseudonym and no further.

#### The MVP ships ONE identity per user, and this section is the destination

**Owner decision, recorded not argued: for the MVP a user has one identity
across every Stoa.** Everything above stays the design — it is where this goes,
and the MVP is a waypoint on the way there, not a change of mind. §9.2 lists the
scope this belongs to.

**The cost is exactly what this section protects against, and it is accepted
knowingly.** Anyone observing two Stoas can link the same participant across
them: one key signs in both, so the public key is the join. Cross-Stoa
unlinkability is **suspended, not withdrawn** — the property is still wanted,
still argued for above, and does not hold in the MVP. Do not describe the MVP as
having it, and do not describe the design as having dropped it.

**What makes the shortcut cheap to reverse is that the mechanism already
exists.** `derive_stoa_key(root, stoa_address)` is built and tested
(`dialectica-core`'s `identity.rs`; the HKDF expansion and its reasoning are in
that function's own doc comment). One identity per user means **not calling it**
and signing with the root key directly. Restoring per-Stoa identity is switching
that call back on, not a redesign — no wire-format change, no address change, no
new primitive.

**The deferred work is the flows, not the crypto.** Creating or joining a Stoa
has to ask *which* identity, which means a create-or-select step at both of
those moments, and a keystore that holds more than one identity for a user to
select from. That is UI and state, and it is the part the MVP is not paying for.

Two things this does not suspend. **§4.1's `senderId` requirements still hold** —
one value per user per Stoa, permanent, and different from every other
participant's — so an MVP signing with one key still needs a per-Stoa
`senderId`, which is a transport identifier and not an author identity. And
§13's open question about SDS-R disclosure gets *sharper* rather than softer:
its answer above ("under per-Stoa identity this leaks nothing the channel does
not already leak") is exactly the premise the MVP removes.

### 5.2.1 What an identity is called

**A display name is generated from the identity's public key, never typed.**
Words drawn from Greek philosophy and letters — *measured attic stoic*, *sober
ionic thales* — not `user_8f3a` and not a handle someone registered. The shape
is settled below and is **four words**, for reasons that are arithmetic rather
than aesthetic.

**But a user is not handed one.** ~~At onboarding they are shown a slate of five
generated identities and pick one, and they may refresh the slate as many times
as they like.~~ **Built — see the `identity-onboarding` spec**, which carries both
the fixed count reported with the set and the unlimited regeneration as
requirements. What matters here and is not a requirement anywhere: a name is
*chosen* in the ordinary sense — it carries intent, and a user who refreshed forty
times meant the one they kept.

**What is being chosen is the key, and the name is the key's shadow.** This
distinction is not pedantry and it is the single most important sentence in this
section for anyone writing copy: *"pick your identity"* is true, *"pick your
username"* is false. A user who believes they picked a name will later ask to
change it, and they cannot — §5.3 has no rotation, so an identity is permanent
within its Stoa and the name is a pure function of it. The only way to get a
different name is a different identity, which is a different person as far as
this Stoa is concerned. An interface that obscures this generates a support
question it cannot answer.

Placed here, immediately after the section that defines what an identity *is*,
because "what does an identity look like on screen?" is the next question a
reader of §5.2 asks, and the answer is a consequence of §5.2 rather than an
independent feature. Every constraint below falls out of per-Stoa permanent
pseudonymity: there is no registry to hold chosen names, no cross-Stoa profile
to carry one, and no rotation to let a user abandon one.

#### Why drawn rather than typed

The slate makes this a choice, so the question is not "chosen or not" but
**"chosen from a generated set, or typed into a box"**. Three reasons for the
former, in ascending order of how conclusive they are.

- **A 32-byte address is unreadable, and unreadable identity is not
  pseudonymity in any useful sense.** A reader who cannot tell two participants
  apart at a glance cannot follow an argument between them, which is the one
  thing this forum is named for. A hex prefix technically distinguishes them and
  is read by nobody.
- **There is no registry, so chosen names cannot be unique.** §1's
  permissionless property is not a preference here — there is no service to
  hold a namespace and no authority to arbitrate a claim. A chosen-name system
  with no uniqueness is strictly worse than a generated one: it *invites* the
  impersonation it cannot prevent, because choosing implies a claim was granted.
- **A typed name is a cross-Stoa correlation channel, and it defeats §5.2.**
  This is the decisive one. The same human typing `fryorcraken` into six Stoas
  has linked six identities the protocol went to real trouble to keep apart —
  key derivation per Stoa, a hashed topic bucket, a `senderId` scoped to one
  channel — and has done it with a text field. A drawn name cannot carry
  arbitrary information across Stoas, because the user selects from what the
  keys happen to produce rather than supplying the string.

  **The slate weakens this rather than preserving it whole, and that is worth
  being exact about.** A determined user can refresh until each of their Stoa
  identities lands on the same noun, and has then built a weak cross-Stoa
  signal by hand. It is far worse than a text field — one shared word among
  three, deniable, and costing many refreshes per Stoa — but it is not zero, and
  the honest claim is that drawing raises the cost of self-linkage rather than
  removing the channel. Nothing here can prevent a user who wants to be
  correlated from correlating themselves, and it is not obvious that anything
  should.

That third point generalises past names, and is worth stating as a rule: **any
user-supplied string that persists across Stoas is a linkage channel.** The same
argument will apply to avatars, signatures, bios and anything else a future
version is tempted to let people carry between Stoas.

#### The derivation

```
name = words(H(NAME_PREFIX || public_key))
```

with `NAME_PREFIX` a fixed 32-byte domain separator in the style §5.1 already
uses for addresses — versioned, so a future wordlist or scheme mints different
names from identical keys rather than silently colliding with this one.

Four properties this has to have, each of which decides something:

- **Deterministic and total.** The same key yields the same name on every peer,
  forever, with no lookup and no state. This is why it is a hash of the key and
  not an op: **a name is not published and cannot be**, because a published name
  is one two peers could disagree about, and two peers rendering one identity
  differently is a bug users report as impersonation.
- **Derived from the public key, not from the address.** The address is already
  `H(prefix || genesis_record)` and §5.1 keeps that record extensible against a
  future key log. Deriving the name from the *key* means a name tracks the key
  that signs, which is what a reader is actually being shown. If rotation ever
  lands (§5.3), this is the seam where the question "does the name change?"
  arrives, and it should arrive loudly rather than being pre-answered here by
  an accident of which input was hashed.
- **Distinct domain separation from every address prefix.** `identity.rs`
  already keeps author and Stoa addresses in separate domains so no byte string
  is both. The name domain joins that set for the same reason.
- **Index extraction is from distinct hash bytes per slot**, so that the two
  adjectives and the two nouns are independent draws rather than four views of
  the same bits.

##### The byte budget, and where the name's independence from the mark comes from

`H` is SHA-256, so the name's digest is **32 bytes**. The name consumes a fixed
slice of them:

| Bytes | Use |
|---|---|
| `0` | first adjective index (256 entries, 8 bits, one byte exactly) |
| `1` | second adjective index |
| `2..4` | first noun index (512 entries needs 9 bits; take 2 bytes and reduce) |
| `4..6` | second noun index |
| `6..12` | **re-derivation reserve** for the denylist, below |
| `12..32` | unused by the name scheme |

**The name stops at byte 12.** Bytes `12..32` of *this* digest are simply unread.

**Correcting the reason an earlier draft of this subsection gave, because the
conclusion was right and the mechanism was invented.** That draft described
`12..32` as "reserved for the identicon", so that disjoint byte ranges would make
name and mark independent. **That mechanism does not exist, and cannot.** The two
derive from *different digests*:

- the **name** from `H(NAME_PREFIX || public_key)` (this section);
- the **mark** from the **address**, which `identity.rs` computes as
  `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key)` — and the mark
  (`docs/IDENTICON.md`) reads bytes `12..19` **of the address**, not of the
  name's digest.

So the name's bytes `12..32` and the mark's bytes `12..19` are slices of two
unrelated hashes. They were never in danger of overlapping, a byte reservation
across them does no work, and the hazard the draft described — "a near-miss on
one correlating with a near-miss on the other" — **cannot arise, because there is
no shared digest to overlap in.**

**The independence is real; it comes from domain separation.** `NAME_PREFIX` and
`AUTHOR_ADDRESS_PREFIX` are distinct 32-byte separators, so the two digests are
independent functions of the same key. Grinding keys for a target's name yields
an unrelated mark each time, and grinding for the mark yields an unrelated name;
they must be landed together, so **the costs multiply rather than add.** That
property holds because the hashes differ, and it would hold no matter which bytes
each side read.

**So why keep the byte budget at all?** Two reasons, neither of them the
independence claim:

- **The re-derivation reserve is a real requirement** (below), and it needs a
  stated bound whether or not anything else reads the digest.
- **Belt and braces, cheaply.** Writing down that the name stops at byte 12 costs
  a table row and means that if the two schemes are ever unified onto one digest
  — which is a plausible simplification — the boundary is already recorded rather
  than being rediscovered. It is *not* load-bearing today and must not be
  described as though it were.

Two notes on the arithmetic:

- **Reducing a 16-bit draw into 512 entries by `% 512` is exactly uniform**,
  because 512 = 2⁹ divides 2¹⁶ evenly — 128 times. There is no modulo bias to
  trade off. **This makes the power-of-two list sizes load-bearing rather than
  incidental**: 512 and 256 were chosen as the honest ceilings of the source
  material (below), and it is a genuine piece of luck that the honest numbers are
  also the ones that divide cleanly. A list of, say, 500 would introduce a real
  if tiny bias and would need this paragraph to say so.

  *(An earlier draft of this subsection claimed a bias "of about one part in 2¹⁶
  per entry". That was wrong in the safe direction — the instinct that "modulo is
  fine here" is a claim with a magnitude was right, and the magnitude is zero.)*
- **The re-derivation reserve is why the name's slice is 12 bytes and not 6.**
  The denylist re-derives a refused combination *from the next hash bytes*, so
  the number of bytes a name consumes is **data-dependent, not fixed**.

  **On a refusal, all four slots are redrawn, not only the offending one.** This
  is the choice worth stating because it decides the reserve's size: redrawing
  the whole name costs 6 bytes per attempt, so `6..12` buys **one** full
  re-draw, and a second refusal exhausts the reserve. Redrawing only the
  offending slot would be cheaper per attempt but makes the denylist harder to
  reason about — a refused *pair* is refused because of the combination, so
  changing one half can land on a second refused pair, and the loop's
  termination becomes a property of the denylist's shape rather than of the
  budget. Whole-name redraw keeps termination arithmetic.

  **If the reserve is exhausted the derivation must fail loudly rather than read
  on.** The reason is *not* that reading on would corrupt the mark — as
  established above, it could not, since the mark is in a different digest.
  It is that reading past byte 12 makes the name's consumption unbounded, so two
  implementations that disagree about how far to read produce **different names
  for the same key**, which is the two-peers-disagree failure this whole scheme
  exists to prevent. A bounded slice is what makes the derivation checkable
  against a test vector at all.

**What the pair is worth, stated without overstating it.** The name space is 2³⁴
(below) and `docs/IDENTICON.md` counts the mark at roughly 12,400 perceptually
distinct results, about 2¹³·⁶. Independent, the bundle is about **2⁴⁷·⁶** — the
mark multiplying the name rather than adding to it.

**Two cross-document mismatches to fix wherever the two are next restated
together**, recorded here because each document is right about its own half and
wrong about the other's:

- `docs/IDENTICON.md` multiplies the mark against **2²⁵**, which was this
  section's *three-word* space from a superseded draft. Against the four-word
  2³⁴ the bundle is correspondingly larger, so its 2³⁸·⁶ understates the pair.
- It also states that "bytes 0..11 are reserved for the generated-name scheme",
  meaning bytes of the **address**. The name scheme reads no address bytes at
  all, so nothing is reserved there and nothing needs to be. **Both documents
  independently invented the same shared-digest story**, which is worth noting as
  a failure mode rather than a typo: two authors each assumed the other's scheme
  read the digest they were looking at, and neither checked.

**It does not change the threat model.** 2⁴⁷·⁶ is still reachable by a machine
with unlimited regeneration, and the grinding subsection below applies unchanged:
an attacker hunting *any* lookalike rather than one exact target searches a much
smaller set. **This raises the cost of casual impersonation and does not defeat a
motivated attacker. The address remains the identity**, and none of this is a
reason to show one less often.

#### One source: Greek philosophy and letters

**An earlier draft of this section drew on ten science-fiction book universes.**
That is withdrawn entirely — no SF vocabulary survives, and neither do the
mythological creatures a middle draft added. The register was the reason: the
owner asked for something more serious, and *vermilion patient sandworm* is a
fantasy handle where *measured attic stoic* is a name an adult will accept
being called in an argument. **If a generated name sounds like a gamertag, it
is wrong.**

Two constraints survive the change intact, and one is retired by it:

- **The words must survive being torn out of context.** A wordlist entry is read
  by people who have not read Diogenes Laertius. `attic`, `stoic`, `praxis` and
  `thales` work alone; a minor scholiast's name is noise. This constraint did
  most of the work in the SF selection and does more here, because the Greek
  corpus has a far longer tail of obscurity available to be wrong about.
- **A name may describe a texture, never a verdict.** Derived in the earlier
  draft as the rule behind the tone exclusions, and now the load-bearing test
  for the whole list. `attic` is a texture; `plato` is a verdict.
- **~~No single source may dominate.~~ Retired.** It existed because a name
  drawn from one universe reads as an allegiance the user did not declare. With
  one source there is nothing to balance, and the flavour is *Greek thought*
  on purpose rather than by accident.

**Public domain by two and a half thousand years**, which retires the trademark
paragraph the SF list needed: there is no estate, no mark, and no proprietor.
The material is also, unlike a novel's coinages, already half-naturalised into
English — `stoic`, `attic`, `praxis`, `ethos` are English words with Greek
parents, which is exactly why they survive decontextualisation.

**The nouns pool two kinds of word**, which is the owner's decision taken for
the size of the resulting list:

| Kind | What it supplies | Examples |
|---|---|---|
| the vocabulary of Greek thought | abstractions that have naturalised into English | `logos`, `ethos`, `kairos`, `praxis`, `techne`, `aporia`, `kanon`, `stasis`, `arete`, `episteme` |
| thinkers, writers and makers | proper names with register | `thales`, `hypatia`, `solon`, `sappho`, `theophrastos`, `eratosthenes`, `kleanthes`, `pyrrhon` |

**A small number of names are excluded because they read as an argument rather
than a name** — Plato, Aristotle and Socrates above all, whose mere invocation
is a move in a debate, so a user rendered *sober ionic plato* is handed standing
they did not earn. The exclusion is deliberately short: a handful of the most
invoked figures, and everything arguable is kept, because the pooled list was
chosen for its size and a cautious sweep would undo that.

**The adjectives carry the register, and this is where the effort went.** A
proper name and an abstract noun read very differently attached to a person —
*sober ionic thales* against *sober ionic praxis* — and the list that reconciles
them is the adjective list. It is therefore uniformly in **one voice, at once
geographic and temperamental**:

| Kind | Examples |
|---|---|
| geographic | `attic`, `ionic`, `doric`, `aeolic`, `delian`, `samian`, `arcadian`, `laconic`, `rhodian`, `theban` |
| temperamental | `measured`, `sober`, `plain`, `patient`, `quiet`, `steady`, `temperate`, `candid`, `spare`, `lucid` |
| dispositional (the schools) | `stoic`, `skeptic`, `eclectic`, `peripatetic`, `aporetic`, `zetetic`, `gnomic`, `ascetic` |

A geographic adjective attached to a person reads as origin, which is a texture
and never a claim; a temperamental one reads as manner, likewise. Both land the
same way in front of either kind of noun, which is what makes the pooled noun
list work at all.

#### Transliteration: one convention, applied throughout

A list assembled from three sources reads as assembled from three sources, so
the convention is fixed and uniform:

- **kappa → `k`, never `c`** — `kanon`, `kosmos`, `techne`, `kleanthes`, not
  `canon`, `cosmos`, `Cleanthes`.
- **`-os` retained, never Latinised to `-us`** — `theophrastos`, `pyrrhon`,
  `chrysippos`, not `Theophrastus`, `Pyrrho`, `Chrysippus`.
- **upsilon → `y`** — `physis`, `mythos`, `hyle`.
- **chi → `ch`** — `techne`, `psyche`, `arche`. (`kh` is more faithful and reads
  as alien; the constraint that a word survive decontextualisation wins.)
- **No diacritics, no Greek script** — see the ASCII rule below, which is a bidi
  decision rather than a typographic preference.

**The one exception, stated so it does not look like drift:** where English has
fully settled a form, the settled form wins — `thales`, `solon`, `sappho`,
`hypatia`, not a stricter transliteration nobody would recognise. The convention
serves recognisability; it does not outrank it.

#### What is excluded from the lists, and why

These are the rules the curation work is bound by. They are stated here rather
than left to the person writing the lists, because **each one was reached by an
argument that is not recoverable from the word it excludes.**

**1. The project's own vocabulary.** A Greek wordlist inside a project whose
vocabulary is Greek will collide with it, and the collision is worst in a feed,
where every row attributes a post to one of these names.

| Excluded | Why |
|---|---|
| `stoa` | the core concept (§1). *measured attic stoa kairos* reads as a Stoa rather than a person. **`stoic` survives as an adjective** — in the adjective slot it cannot be misread as naming a place, and it is one of the best words available. It must never be a noun. |
| `dialectic`, `dialectical` | the project's name and its method. A user called *dialectic* sounds like the application speaking. |
| `delta` | the logo (§8.1). Same failure. |
| `genesis` | names the founding record (§5.1), the most load-bearing term in the address construction. |
| `agora` | **kept, and flagged first-to-drop.** It is common enough English to survive and is not a term of art in this design — but `docs/UI-BRIEF.md` uses "Join *Agora*?" as its worked example of a forgeable Stoa title. If the picker ever reads ambiguously, this is the first noun to remove. |

**2. Words that assert authority.** `moderator`, `archon`, `ephor`,
`magistrate`, `strategos`. A non-moderator generated as *calm bronze archon
telos* has been handed apparent standing **by the wordlist**, which is precisely
what §5.2.1's rendering obligations exist to prevent — a name is never a
credential. This is the exclusion most likely to be re-proposed by someone who
likes the word, and it is also where the authority-name exclusion below comes
from: the two are the same failure reached by different doors.

**3. A few names that are an argument rather than a name.** **Plato, Aristotle,
Socrates**, and anything else whose invocation is itself a move in a debate: a
user rendered *sober ionic plato* is signed by Plato on every post, and someone
disagreeing with them is visually disagreeing with Plato. They neither earned it
nor chose it, but they benefit from it.

**Deliberately a short list.** Everything arguable is **kept** — the pooled noun
list was chosen for its size, and a cautious sweep through the canon would undo
exactly what it was chosen for. The long tail is the point: *sober ionic thales*
is still Greek and still serious, and nobody treats "Thales said so" as an
argument.

**4. Connotation.** The name is assigned-then-chosen, so a user cannot be blamed
for the word they were handed — but **the system can be blamed for generating
it**, and "the hash chose it" is not a defence anyone accepts. Rejected by
category, because the categories outlast the examples:

- **Boasts** — `titan`, `colossus`, `olympian`, `paragon`, `sovereign`. A name
  that congratulates its bearer is embarrassing to everyone who did not pick it.
- **Tyranny and violence** — `tyrant`, `despot`, `nemesis`, `scourge`,
  `hecatomb`, and the `furies`. `tyrant` is the clearest case in the whole list:
  a live political insult in English, generated by the system and attached to a
  participant in a **political argument forum**, which is a system defaming a
  user.
- **Disorder as an accusation** — `chaos`, `discord`, `eris`, `strife`. In a
  forum whose subject is disagreement these read as a verdict on the person.
- **Pathology and death** — `plague`, `miasma`, `lethe`, `thanatos`, `charon`,
  `hades`. Grim attached to a human being who is about to post.
- **Anything mapping onto a real group** — `barbarian` (Greek for the people who
  did not speak Greek: an ethnic slur with a classical wrapper), `helot`,
  `pariah`, `metic`. The etymology is interesting and irrelevant; the English
  word lands as the English word.
- **Sexual and bodily** — `satyr`, `priapic`, `bacchant`. Named so the next
  person adding words does not rediscover it.

**Two kept after argument, recorded because they are the near-misses.**
`chimera` and `hydra` are monsters but not insults in English — fully absorbed
as "a thing of mixed parts" and "a problem that regrows", neither a claim about
the person. **`siren` was dropped** despite the same absorption, because
attached to a person it is gendered in a way the others are not.

**The rule underneath all of it, which is the thing to keep if the lists are ever
rebuilt from scratch: a generated name may describe a texture, never a verdict.**
`attic`, `measured`, `tidal` and `spare` describe nothing about their bearer.
`heroic` and `craven` both do, in opposite directions, and both are wrong for the
same reason. `attic` is a texture; `plato` is a verdict.

#### The arithmetic, and why the name is four words

**The list sizes were derived from what the sources honestly yield, and the word
count then followed from the arithmetic.** That order matters: the alternative —
picking a target space and padding the lists to reach it — produces names that
read as filler, which costs the register this whole revision exists to buy.

**What the sources honestly yield — and these are estimates, not counts.** No
list has been written; the numbers below come from inventorying candidates by
category and judging where each category runs out. **The whole four-word decision
rests on them**, so the honest status matters: the claim "512 adjectives in one
voice is padding" is a judgement, and the way to falsify it is to write the list
and count. **The conclusion survives a generous error, which is why it is safe to
decide on now.** Suppose the estimate is wrong by a fifth and 300 honest
adjectives exist: `300 × 300 × 512 = 46,080,000`, and
`12,497,500 / 46,080,000 = 0.27121`, so `1 − e^-0.27121 = 0.2375` — **5,000
identities still collide at 24% at three words.** Rescuing three words would take
an adjective list of about **700** — `700 × 700 × 512 = 250,880,000` puts 5,000
at 4.9% — and 700 sober Greek adjectives in one voice is roughly three times the
estimate, which is not a plausible error but an entirely different claim about
the source material.

- **Adjectives: 256.** The single-voice constraint is a real limit. Geographic
  adjectives of the Greek world give roughly 70 usable; the schools and
  dispositions give roughly 55 after the tone exclusions; plain English
  temperament words in the sober register give roughly 90. That is about 215,
  and reaching 256 is honest work. **512 is not reachable in one voice** — the
  last two hundred would be either obscure demes or four near-synonyms for
  *calm*, and a list padded with `placid`/`serene`/`tranquil`/`unruffled` is
  exactly the filler that loses the register.
- **Nouns: 512.** The pooled list is deep: roughly 250 terms from the vocabulary
  of Greek thought, and roughly 250 thinkers, writers and makers once the
  handful of argument-move names come out. **1,024 is not reachable** without
  scraping every minor figure in Diogenes Laertius and every technical entry in
  Liddell–Scott, which reintroduces the "an obscure surname is noise" failure
  the second constraint forbids.

**So the honest sizes are 256 and 512, and at three words that is not enough.**
With two adjectives and one noun the space is `256 × 256 × 512` = 2²⁵ =
**33,554,432**, and birthday collision probability for k identities in one Stoa,
`1 − exp(−k(k−1)/2S)`, gives:

| Identities in one Stoa | three words, S = 2²⁵ |
|---|---|
| 100 | 0.015% |
| 1,000 | 1.5% |
| 5,000 | **31%** |
| 10,000 | **77%** |

By hand at k = 5,000: `k(k−1)/2 = 12,497,500`, and `12,497,500 / 33,554,432 =
0.37245`, so `1 − e^-0.37245 = 1 − 0.68902 = 0.311`.

**31% fails the bar**, and it is worth being blunt that this is *worse* than the
space it replaces on every count except register. The superseded SF scheme was
**256 adjectives × 256 adjectives × 256 nouns = 2²⁴ ≈ 16.8 million** at three
words, with 5,000 identities colliding at 53%; Greek-only sources are smaller
than ten SF universes pooled, so the honest noun list grows from 256 to 512
while the adjective list cannot grow at all, and 2²⁵ at three words leaves
5,000 at 31%. Better than 53% and still a failure.

**Therefore the name is four words: two adjectives and two nouns.**

```
S = 256 × 256 × 512 × 512 = 2³⁴ = 17,179,869,184
```

| Identities in one Stoa | **four words, S = 2³⁴** |
|---|---|
| 100 | 0.00003% |
| 1,000 | 0.003% |
| 5,000 | **0.073%** |
| 10,000 | **0.29%** |

By hand at k = 5,000: `12,497,500 / 17,179,869,184 = 0.00072745`, and for x this
small `1 − e^-x ≈ x`, so **0.073%**. At k = 10,000: `49,995,000 /
17,179,869,184 = 0.0029101`, giving `1 − e^-0.0029101 = 0.002906`, so **0.29%**.

**This reverses the recommendation an earlier draft of this section made**, and
the reversal should be visible rather than quietly corrected. That draft argued
for three words against a fourth, on the grounds that a name renders on every
feed row and recognition degrades with length before information content does.
**That argument is still true and it now loses**, because its premise changed:
it was made against lists of 512 and 1,024 that a blended SF-and-Greek corpus
could sustain, and the Greek-only sources do not sustain them. Given a choice
between a fourth word and a padded list, **the fourth word is the lesser cost**
— length is a fixed, honest price, where padding degrades every name drawn from
the padded region and cannot be undone without a scheme version bump.

Two things make the fourth word cheaper here than it would have been:

- **The second noun is a noun, not a third adjective.** *measured attic thales
  praxis* has two content words a reader can latch onto rather than a longer
  run of modifiers, and the pooled noun list is the one with the range to
  support two draws.
- **The register absorbs length better than the old one did.** Four Greek words
  read as a name in the classical manner; four SF words read as a string of
  adjectives. This is a genuine difference and not a rationalisation, but it is
  a taste claim and is marked as one.

**So: collisions are now rare rather than expected — and the interface rule does
not change.** A Stoa of five thousand has under a one-in-a-thousand chance of
containing a pair. That is a different world from 53%, and it changes nothing
about what the interface must do, because the rule was never a response to the
rate:

> **A name is never presented as unique, and never used as an identifier.** The
> address is the identity. This is the same rule §4.8 and §5.7 already state for
> Stoa titles, arriving a second time by a different route — which is the
> strongest evidence it is the right rule rather than a local patch.

What the interface does on collision is therefore nothing special: it
disambiguates the same way it always should, by showing the address alongside
the name where it matters. **What it must not do is renumber.** Appending `#2`
to the second `measured attic thales praxis` requires agreeing which one was
second, which is arrival order — a per-peer fact (§3.3), so two peers would
number them oppositely and each would be sure the other was the impostor.

**A second collision question the slate introduces: two identical names in one
picker.** Five draws from 2³⁴ collide with probability about `5·4/2S` = `10/S` —
roughly one slate in 1.7 billion, against one in 1.7 million at 2²⁴. A user will
never see it, and at this size arguably no deployment ever will.

~~**The picker still discards and redraws a duplicate**, because the handling is
three lines and the alternative is a display that reads as broken in the one case
it appears. This one *is* worth handling, and it is cheap precisely because it is
local: the slate is generated on one peer, at one moment, with nothing published.
So **the picker discards and redraws a duplicate before displaying**, which is a
presentation rule with no protocol consequence whatsoever.~~

**Built, and not by the picker** — see the `identity-onboarding` spec. Duplicate
handling is in **core**, and it is an index walk rather than a redraw: the
derivation walks forward until it has five distinct paths. A redraw would need a
fresh nonce, which would destroy the reproducibility everything else rests on; the
archived `design.md` carries that argument. The distinction this section draws
against the cross-identity case still holds and is why it is kept: one is a choice
about what to draw, the other a fact about what exists.

~~**Regeneration discards keys, and the user cannot see it.** Each refresh mints
five keypairs and keeps at most one; the rest are gone, unrecoverable, and were
never anywhere.~~ ~~Whether the keystore writes on every refresh or only on
selection is an implementation question with no user-visible consequence, and is
deliberately not decided here.~~

**Both halves are superseded.** A refresh mints **no keypairs**: the five
candidates are derivation paths over **one** master key, differing by path alone —
so there are no discarded keys to be invisible about, and a backup is one secret
rather than five. And the write question **was decided**: nothing writes on
refresh, structurally, because the slate handler has no store parameter to write
to. See the `identity-onboarding` spec for both, and its archived `design.md` for
why five independent roots was rejected.

What survives from this paragraph, because it is the reason the shape is safe:
**an identity becomes real when it signs, and nothing signs during onboarding.**
That is now spec prose rather than a plan note.

#### Grinding — and the slate makes this the central finding

The question was what it costs an attacker to mint keys until one derives a name
resembling a moderator's. **Refreshable onboarding is that attack, shipped as a
feature.**

This is worth stating as plainly as possible because it inverts the usual
framing. An earlier draft of this section argued that grinding was cheap — about
S derivations to hit one specific name, seconds on one core — and concluded that
enlarging the wordlist could not fix it. **The move to 2³⁴ is the interesting
test of that claim, and it does not overturn it**: 17 billion derivations is
minutes rather than seconds on one core, and trivially parallel, so the cost
went from negligible to slightly less negligible. An attacker hunting one
specific name now waits; an attacker hunting *any* name close enough to mislead
a reader — which is the actual attack — searches a far smaller target set and
waits no longer than before. Both halves stand, and both are *beside the point*:
**with unlimited
regeneration the attacker does not need a script.** They press refresh. The
difference between a user refreshing for a name they like and an attacker
refreshing for a name that impersonates a moderator is **intent, not mechanism**,
and no interface can distinguish them, because there is nothing to distinguish.

**This is not a reason to remove regeneration.** The slate is good onboarding
and the attack exists without it — a key is a key, and anyone willing to run a
loop had this already. What it removes is the *option of leaning on cost*. Any
argument of the form "an attacker would have to grind for that" is unavailable
here, and this section must not contain one.

So: what actually defends against impersonation?

**Nothing does, and the address is the identity.** That is the honest answer and
it is already this project's answer for Stoa titles (§4.8, §5.7) — a title is
moderator-chosen, unverified and freely duplicable, so a join confirmation
showing only a title has shown the reader precisely the forgeable half. A
generated name is the same shape in a second location: **freely reachable by
anyone willing to press a button, attached to an address nobody can forge.**

What the attacker gets is a *lookalike name on a different address*. They cannot
forge the 256-bit address and they cannot forge a signature, so nothing they
publish is attributable to the impersonated identity. The whole attack is
social: a reader who recognises people by name is fooled; a reader who has the
address in front of them is not.

**The interface consequence is therefore identical in shape to the join
confirmation, and belongs on §11.1's list:**

> **A name alone is the forgeable half.** Wherever recognition carries weight —
> a moderator's name above all, because that is what converts a button press
> into apparent authority — the address must be present and not one click away.
> A name is never unique and never an identifier.

**Enlarging the space is the wrong instinct *for this problem*, and the move to
2³⁴ is not a counter-example.** The space grew because the honest lists were too
small for three words to hold an acceptable *accidental* collision rate, and for
no other reason. Against grinding it bought minutes. A space big enough to
resist search — say 2⁶⁴ — is a space nobody curated, which forfeits every
property the source constraints above exist to protect: it would be scraped, and
it would ship slurs and verdicts. **Curation and search-resistance are in direct
opposition, and curation wins**, because search-resistance was never achievable
by this route and is not the relevant axis. The two motives are easy to conflate
and must not be: **the space grew to help honest users tell each other apart,
not to make anyone harder to imitate.**

#### Four layers, and only one of them settles anything

Three further recognition aids exist or are intended, and listing them flatly
would invite exactly the wrong conclusion — that between them the problem is
handled. It is not. **Three of the four are recognition aids and the fourth is
the only guarantee**, and they are set out in that order so the asymmetry is
visible rather than averaged away.

**1. The name space, at 2³⁴.** Reduces *accidental* collisions, and nothing
else. Covered in full above.

**2. An identicon — designed in `docs/IDENTICON.md`, not here.**
A visual glyph shown alongside the name, so that two identities sharing a name
still look different at a glance. The property it must have is the one the name
already has: **derived from the key, hence
deterministic, unregistrable and identical on every peer.** It earns its place
on *accidental* collisions — a reader comparing two rows sees two different
pictures without reading a word.

**It varies independently of the name**, because the two derive from
differently-domain-separated hashes of the same key — the name from
`H(NAME_PREFIX || key)`, the mark from the address (see the byte budget above).
So an attacker must land both at once and the costs multiply rather than add.

**It is nonetheless forgeable in exactly the way the name is, and this must not
be softened into a security property.** An attacker grinds for a key whose name
*and* glyph both read close; independence makes that a two-channel search rather
than a one-channel search, which raises the cost by a factor and changes the kind
of protection not at all. **A second forgeable channel is still forgeable**, and a
multiplied cost is still a cost a machine pays once. The visual design is
deliberately not attempted here — what is settled is that it is wanted and what
it must be derived from.

**3. Vouching (§7.3) — the only layer an attacker cannot mint, and the most
limited.** A vouch, declared or earned, points at **a key**, not at a name and
not at a picture. So a lookalike gets the name, gets a near-matching glyph, and
does **not** get the vouch — the reader's own prior judgement does not transfer
to the impostor, because it was never attached to anything the impostor could
copy. That is the only distinction on this list that does not reduce to "look
more carefully".

Two limits, both structural in §7.3 as it stands, and both must be stated or
this reads as a solution:

- **It is private and never published**, which is §7.3's core privacy property.
  So it protects a reader from being fooled by an impersonation of *someone that
  reader has already vouched for*. It does nothing for a reader meeting either
  party for the first time — which is most impersonation, and in a
  permissionless forum the common case.
- **Earned weight accrues from upvotes the reader has already cast** (§7.3), so
  a reader's vouched set is **empty at onboarding** and stays thin until they
  have read for a while. New users have no vouches and are simultaneously the
  least equipped to spot an impostor. The layer is weakest exactly where the
  exposure is highest.

**4. The address — the only thing that settles identity.** Unforgeable,
unmintable, and what every signature actually binds to. Its one weakness is not
cryptographic: **it settles the question only if someone looks.** That is
precisely why §11.1 requires it to be *present* rather than one click away
wherever recognition carries weight, and above all wherever a moderator is
named.

The shape to carry away: **layers 1–3 make an honest mistake less likely; only
layer 4 makes a dishonest claim false.** An interface that shows a name and a
glyph and no address has shipped three recognition aids and zero guarantees.

Two mitigations available and deliberately not taken, recorded so nobody
re-proposes them as fixes. **Deriving the name from the key and the Stoa address
together** helps not at all: the attacker grinds within one Stoa and would
simply grind against that Stoa's derivation. **Rate-limiting the refresh** helps
not at all either: it inconveniences the honest user at onboarding, which is the
worst possible moment to add friction, while an attacker who is willing to run
the derivation outside the app — which is trivial, since it is a hash — never
touches the control being limited.

#### Word-level failure modes

- **Combinations, not just words.** Two individually innocuous adjectives can
  compose into a slur or an insult aimed at a real group. Vetting single words
  is insufficient; the generated *combination* is what ships. **The fourth word
  makes this materially harder**, and that is the one real cost of the shape
  chosen above: 256² ordered adjective pairs was already past hand review, and
  the noun pair adds 512² more, with the adjective–noun and noun–noun junctions
  on top. A pooled noun list of thinkers and abstractions is also more exposed
  than a list of one kind, because a proper name beside an abstract noun can
  compose into a reading neither word carries alone. So the practical
  requirement is a denylist applied at generation — a derived name landing on a
  refused combination **redraws all four slots** from the next six hash bytes,
  deterministically, so every peer skips identically. The byte budget above bounds
  this at one re-draw and requires a loud failure beyond it; the denylist matters
  more at four words than it did at three.
- **The lists are versioned and effectively frozen, and a word removal is a
  scheme version bump.** This is the most operationally important line in the
  section, so it is worth spelling out the mechanism rather than asserting the
  rule. `name = words(H(NAME_PREFIX || public_key))` maps hash bytes to list
  indices, so removing one word **reindexes the list** and every identity whose
  name drew on an index at or after the removed one now renders differently.
  That happens **on peers that have updated and not on peers that have not** —
  so the same key renders as two different people depending on who is looking,
  which is the two-peers-disagree failure this whole scheme exists to avoid, and
  which users report as impersonation.

  So `NAME_PREFIX` is versioned, and **any change to the lists — a removal, an
  addition, a reordering, a size change, a change to the number of words, or a
  change to the byte budget above — mints a new version rather than editing the
  current one.** The byte budget belongs in that list for the same reason as the
  rest: moving the name's slice re-reads different bytes and so renames everyone.
  It does **not** disturb the mark, which reads a different digest entirely. The
  version bump is
  what stops the disagreement: it makes the old and new schemes distinct
  derivations rather than two peers' answers to one question. It is also why the
  *first* version must be conservative: shipping a word that has to come out
  later is not a patch, it is a migration in which everybody's name changes at
  once.
- **ASCII-only, and this is a bidi decision rather than a parochial one.** These
  are display strings composed by us from a fixed list, so unlike post bodies
  they are the one piece of rendered text the project fully controls. Keeping
  them ASCII means a generated name can never itself carry a bidi override or a
  homoglyph — it removes the attack from this surface entirely rather than
  mitigating it. **The bidi obligation still applies to everything a name is
  rendered *next to*** (§11.1), which is the usual case, and a name sitting
  beside an attacker-controlled body can still be visually captured by it.

#### Rendering obligations this creates

Collected here for §11.1 ("Rendering obligations, collected"), which arrives
with the `vouching-state` change and is not in this file until that lands. Each
is a place where core's honest answer is incomplete without something the view
says, which is that section's general shape:

- **A name is never unique and never an identifier.** The address is the
  identity. This is the obligation Stoa titles already carry, now applying to
  the thing *every post is attributed to* — a much larger surface, since a feed
  renders a name per row.
- **A name alone is the forgeable half.** Wherever recognition carries weight,
  and above all wherever a **moderator** is named, the address must be present
  rather than one click away. Anyone can reach any name by pressing refresh.
- **Never imply a user's names are linked across Stoas**, and never build a
  screen that puts them side by side without the owner deciding to (below). The
  names are unlinkable by construction and an interface that groups them has
  undone §5.2 in the presentation layer.
- **Never present a name as changeable.** It is a function of a permanent key
  (§5.3). Copy that says "pick your username" promises a settings screen that
  cannot exist.
- **Never present the recognition layers as adding up to a defence.** An
  identicon and a vouch are recognition aids; the address is the only thing that
  settles who published something. A screen that shows name and glyph and calls
  the pair "verified" has said something false.
- **A four-word name needs room, and must not be truncated to fit.** It is
  longer than the three-word form earlier drafts assumed, and a layout that
  elides the tail has removed one of the two nouns — which is most of the
  distinguishing content, since the adjectives are drawn from the smaller list.
  If a row cannot hold the name, the row is wrong.

#### What is not decided here

- **Whether the same human sees their own names across Stoas in one place.**
  They necessarily know their own identities, so a local, never-published "your
  identities" list leaks nothing to anyone else — but it is a screen whose whole
  content is the correlation §5.2 protects, and building it makes that
  correlation one screenshot away. Not decided; it is a real convenience against
  a real hazard, and it wants the owner's judgement rather than a default.
- **The list contents.** The *sizes* are decided — 256 adjectives and 512 nouns,
  and the four-word shape follows from them — and so are the source, the
  transliteration convention and the exclusion rules. What is not written is the
  768 words themselves, which is curation work rather than design work. **The
  sizes are load-bearing in a way the earlier draft's were not**: they were
  derived from what the sources honestly yield, and the word count was chosen to
  fit them, so a later decision to "just add more adjectives" changes the
  arithmetic that justified four words and needs a version bump either way.
- ~~**The identicon's visual design.**~~ **Settled elsewhere:
  `docs/IDENTICON.md`.** It reads bytes `12..19` of the **address** and counts
  about 12,400 perceptually distinct marks. Nothing in this section constrains
  it, and the byte budget above does not hand it anything — the two schemes read
  different digests. Left in this list as a pointer, because a reader arriving
  from "Four layers" above will otherwise look for the design here.
- **How many refreshes is too many to be honest about.** If a user refreshes
  two hundred times, the interface has watched someone hunt for a specific name
  and has no idea whether they are picking a favourite or building an
  impersonation. Saying nothing is the current assumption and is probably right;
  the alternative — some nudge after N refreshes — would annoy every honest user
  to inconvenience no attacker, per the rate-limiting argument above. Recorded
  because it will be proposed.

### 5.3 No rotation in v1

Deliberate, and the reasoning is ordering: **a key that can be discarded at will
is a key nothing can be attached to.** Rotation lets a user shed whatever has
accumulated against their identity — a moderator's judgement, a poor standing,
and in §7.2's terms any credential-weighted relevance they would rather not
carry — and it does so indistinguishably from a legitimate "my key leaked".

Until the thing that persists is something *other than the key*, rotation
subtracts from the design without adding anything. §5.2's rejection of
thread-scoped identity is the same argument applied to a scope rather than to an
event.

What unblocks rotation is therefore the claims layer (§5.5), where standing
attaches to a revocable credential rather than to a keypair — and §5.1's
record-hashed address is what lets a key log land without every author
migrating, which is why that construction was chosen before anything needed it.

### 5.4 Why not the obvious alternatives

- **λAccount / VLAD** is the right long-term target and its requirements read
  like a forum identity spec (stable address under key rotation, append-only
  key log, no irreversible single-key takeover). But the library spec, registry
  interface and backend decision are all v0.3 deliverables, and the
  blockchain-backed registry is mainnet. Cannot be consumed yet. §5.1's record
  hashing is what keeps that door open.
- **LEZ private accounts taken literally** would inherit the wrong properties —
  they are a financial confidentiality primitive. No rotation, no recovery, no
  sybil resistance, and an address that changes when keys do. Match their
  cryptographic seriousness, not their construction.

  **And not their signature scheme either.** It is tempting to reason that
  because LEZ proof-of-holding is coming (§7.2), dialectica should sign with
  LEZ's scheme so the two interoperate. That reasoning is wrong, and it is worth
  refuting explicitly because it is the plausible mistake:

  - **A claim binds to a key named in its own journal, not to a matching
    curve.** LP-0005's journal exposes a `presenter_pubkey` carried as a
    length-checked byte blob, and binding works by the presenter signing a
    verifier nonce over the journal hash. **The LEZ account key never appears in
    the journal at all**, since keeping `npk` private is the point — which is
    the part that matters here. (Do not read the presenter key as freely
    chosen: an early gate allowed that, and error `3010` was added precisely to
    require the signer *be* the attested account. The conclusion survives, the
    freedom does not.) §5.5's "verified independently of how it signs" is the
    general statement; this is the concrete mechanism.
  - **There is no single "LEZ scheme" to match.** LEZ's own account material is
    already mixed — `npk` is a SHA-256 chain rather than a curve point, and
    `vpk` is an ML-KEM-768 key — and a shipped LEZ program (`sequencer_stake`)
    verifies **ed25519** in-guest. A LEZ guest links whatever signature crate it
    wants.
  - **Matching a scheme LEZ may leave.** LEZ's own source notes its signature
    keys are a hedge, to be reduced "once LEE is upgraded to use PQ signatures".
    Matching today could mean matching something abandoned tomorrow.

  So the forum's signing scheme is chosen on the forum's own criteria — key
  derivation, parse safety, verify cost — and proof-of-holding binds to it as a
  claim regardless.

  **The scheme is Ed25519**, decided on those criteria, over BIP-340 Schnorr on
  secp256k1 (`k256`) — which was the LEZ-matching candidate the bullets above
  dispose of.

  Parse safety was the clearest criterion and is not a matter of taste. In
  `k256` 0.13, `VerifyingKey::from_bytes` and `TryFrom<&[u8]> for Signature`
  each take a byte slice, return a `Result`, and **panic** anyway on a wrong
  length — one via `generic-array`'s `from_slice`, the other via `split_at`. A
  public key and a signature arrive inside every inbound op, so both were
  remotely reachable, and PHASE0-FINDINGS §3 measured what a panic in a dispatch
  handler does: the module process aborts, and the guard cannot help because the
  abort happens below it. Ed25519's constructors take fixed-size arrays, so the
  mistake cannot be expressed rather than having to be guarded against. (`k256`
  0.14 fixes both structurally; the choice was not close enough for that to
  reopen it, since key derivation decided it — see below.)

  **Verification must use `verify_strict`, and every peer must use the same
  predicate.** The requirement is *pinning*, not a reading of RFC 8032: the two
  functions differ in rejecting small-order `A` and `R` and non-canonical `R` —
  torsion malleability — rather than in the cofactored/cofactorless choice, and
  both dalek verifiers are cofactorless. What matters is that peers verify
  independently (§3.3, §6), so any disagreement about which predicate applies is
  a partition in the one place this design cannot tolerate one. The implication
  runs one way: a peer accidentally calling plain `verify` accepts a strict
  superset, so it admits ops that strict peers reject.

  Worth recording because no command will tell you: dalek's own `verify_strict`
  doc comment quotes the RFC's cofactor sentence and describes behaviour the
  code does not implement (upstream `curve25519-dalek#663`). So this is a
  semantics choice pinned to a library whose documentation is wrong about it —
  which is why the call site names the required checks explicitly, so that a
  dependency bump is visible rather than silent.

  ZIP-215 is worth reading here and reaches the *opposite* answer from the same
  premise: it removes the torsion checks and multiplies by the cofactor,
  because for consensus it wanted the more deterministic option. That is a
  legitimate alternative pin; what is not legitimate is peers disagreeing.
- **Chat module identity** is ephemeral (restarting mints a fresh identity) and
  `getIdentity()` is marked `// TODO: Deprecate`. Do not build on it.

There is **no identity or keystore module** in the ecosystem, first-party or
otherwise. `wallet_module` has no key custody. Dialectica defines this
interface rather than consuming one.

### 5.5 Identity must be modular, and carry claims

The interface is not "verify this signature" — it is **"resolve this address to
a set of verified claims"**. Both later features are attestations about a
pseudonym, verified independently of how it signs:

- RLN membership → a rate-limit credential
- LEZ proof-of-holding → "owns this NFT" → higher relevance (§7.2)

A signature-only interface cannot carry either. Costs nothing to get right now;
surgery later.

**"Verified independently of how it signs" is the load-bearing phrase**, and
early proof-of-ownership (§7.2) is what makes it matter rather than being
architectural good manners. A claim is an attestation *about* a pseudonym; it is
not required to share a curve, a key format, or a signature scheme with the
pseudonym it describes. Keeping that boundary clean is what stops an external
credential system from dictating dialectica's own signing scheme — see §5.4,
where that exact reasoning decided the scheme.

Three requirements on the interface, each of which is cheap now and structural
later:

- **A claim must be presentable under distinct pseudonyms without linking
  them.** Identities are unlinkable across Stoas (§5.2), so a user proving "I
  hold RLN membership #4271" under their pseudonym in each of two Stoas would
  link those pseudonyms by the membership id — collapsing the one privacy
  property v1 actually claims.

  **Do not assume the primitives supply this.** RLN's per-epoch nullifiers are
  built around the shape and plausibly do. **LP-0005 does not claim it**: its
  stated property is *threshold privacy* — the verifier learns neither the
  balance, the account identity, nor the `npk` — which is a different guarantee
  from two presentations being unlinkable *to each other*. Its later revision
  points the other way, adding error `3010` ("signer is not the attested
  account") so that every presentation authenticates the same underlying LEZ
  account. Unverified, load-bearing, and carried in §13.
- **Claims must be revocable, and revocation must be locally checkable.**
  §5.3 defers rotation until a credential exists that rotating cannot shed, so a
  claim that cannot be withdrawn is a grant that cannot be undone. §6 already
  needs equivalent machinery for the moderator set.
- **A claim must carry its own expiry, checked on read.** An attestation about a
  token balance is a statement about a moment. OpChan is the cautionary case: it
  ships delegation proofs whose expiry check is commented out, so an expired or
  leaked signing key stays valid to every verifier forever — the honest client
  stops signing, and every peer keeps accepting. Check expiry where the claim is
  *used*, not where it is issued.

#### Externally-anchored credentials: keys stay, proofs supplement

**The answer, and a reader can stop here: the identity keys remain the identity,
and proofs only supplement them.** A proof adds weight to a relevance score and
may display extra information. It never replaces an identity, renames one,
authenticates one, or becomes one. So the identity model is **layered** — a
signing key at the base, optional credentials above it, and **the base unchanged
by anything above**. Everything below is downstream of that sentence.

That is what makes this subsection a record of a *seam* rather than a design. The
mechanics are deliberately a later plan; what matters now is that the layering is
right and that nothing forecloses the upper layer.

A user may one day want to attach an **external** credential to a per-Stoa
pseudonym — a LEZ token-holding proof (§7.1), an ENS name, an NFT, a DID. The
question is not whether to support one; it is whether doing so later costs a
redesign. **It does not, and the evidence is specific:**
a new op kind takes the next free discriminant, and `op-format`'s spec requires
that discriminants "be appended, never inserted", so a kind added later "costs
one unused discriminant and no encoding version, and an older client meets it as
an unrecognised kind rather than misparsing it". That is a spec requirement with
a scenario behind it, not an accident of the current encoding.

**So build nothing now.** No credential op kind, no verifier trait, no plugin
interface. This is recorded because the likely mistake is the opposite one —
constructing the mechanism early to "make room" — and the room already exists.
Widening the core API is a decision to make on purpose, not a side effect of
anticipating a feature.

**Three decisions would close the door**, and this is the half worth knowing —
each is cheap to avoid now and structural to undo later:

- **A credential arriving as anything other than an op.** §3.3 and `op-format`
  both state the rule — an op is "the only thing that crosses the wire", and
  "anything not expressible as an op is not expressible at all". A credential
  fetched over a side channel, read from a sidecar file, or supplied by the view
  is state peers cannot verify independently, which breaks §6's "every peer
  verifies independently" rather than extending it. The pressure to do this will
  come from whichever credential is most awkward to carry, and that is exactly
  when to refuse it.
- **Folding a credential check into op verification.** `identity`'s "Authenticity
  is not authority" is the seam, and it is already in the right place: a
  successful verification means "this op is authentically from the author it
  claims" and must not be read as "this op is permitted". A credential is an
  authority question, settled on read like moderation (§6), not an authenticity
  one. Putting it inside signature verification would give that function a second
  job — and one that depends on another module's availability.
- **Storing the verdict instead of the evidence.** §3.3 already states the rule
  for the op log — it "stores **inputs and never conclusions**" — and a
  credential is where that rule is easiest to break, because caching
  `verified: true` next to an address is the obvious optimisation. It is also how
  a conclusion outlives the fact it was drawn from: a stored verdict has no
  expiry, so it never dies. Spasm's `verified` flag is the shape to avoid
  inheriting — its own documentation scopes it to "whether this address matches
  with at least one attached signature", which is a per-event observation, and it
  is stored on the event and in the database alike. Keep the proof; re-decide the
  verdict.

**The verifier ships in the same change as the field**, and there are now two
measured instances of the alternative. OpChan's commented-out expiry check is one
(above). Spasm — a signer-agnostic social protocol, the nearest thing to prior
art — is the other, and the evidence is an **absence**: its `SpasmEventProofV2` is
declared, copied between representations, and hashed into the event id, and
**there is no validator anywhere**. Not a dead helper, not a commented-out check,
not a TODO. `utils.ts` is six thousand lines, holds the real
`verifyEthereumSignature` and `getVerifiedSigners`, and contains **zero
occurrences of "proof" in any casing** — as do `spasm.ts` and `index.ts`.
Signatures get `ethers.verifyMessage` and a thorough test suite; proofs get
carried. Its fixtures accordingly ship placeholder proofs (`value:
"proof-value1"`, `protocol.name: "proof-protocol-name1"`) that convert through and
are asserted equal to the expected output, so a meaningless proof sails through by
construction. **A credential field nobody checks is worse than no field at all**,
because it reads as a guarantee to everything downstream.

**Two properties that sound like one, and the design consequence.** It is
tempting to divide credentials into self-contained proofs and external lookups,
and to prefer the former. That division is real but it is not where the risk
lives:

- **Verification convergence** — do two peers agree the credential is
  *well-formed*? A self-contained proof: yes, permanently. A state query resolved
  through a sibling module: it depends when, and at what height, it was asked.
  **The rule to keep is that a claim must be verifiable to the same verdict by
  every peer holding it, and a verifier that answers differently to two peers is
  not a verifier.** "Holding it" carries the load: a peer with no verifier module
  is not *disagreeing* about the claim, it simply does not have one to evaluate,
  which is a different axis and is settled under composability below. Appendix A
  records the measured failure — OpChan's
  proof-of-holding was an HTTP call to a third-party indexer, and "a peer without
  an API key computed different scores".
- **Assertion freshness** — is what the credential claims *still true*? **Here a
  proof and a lookup are the same, and neither is fresh.** "I held ≥ N at block H"
  is true forever and says nothing about now; transfer the tokens and the proof
  stays valid while the fact goes stale. ENS names expire and transfer, which makes
  the point from the other side. A proof's only advantage is **honesty**: it names
  the moment it speaks for, where a lookup answers "now" and hides that "now" has
  passed.

So the guidance is *not* "prefer proofs". It is that **any ownership credential
is a statement about a moment**, and that the design must decide what a
moment-old fact entitles someone to. **That question is identical for LEZ and for
ENS**, which is the argument for keeping the architecture adaptable rather than
betting on either. ENS is not ruled out; pinning a block height is the obvious
mitigation for the convergence half and is **unexplored**.

**The intended answer to the freshness half is expiry, which this section already
required**: proofs expire and the holder re-proves on a cadence — seven days,
thirty, whatever the claim warrants — so the reader never chases current state and
a proof past its window stops counting. That **moves the burden to the claimant**,
which is what keeps a credential from becoming state a reader has to reach for.

Three things a future design must handle, recorded because each is easy to get
wrong and none is obvious:

- **The window needs a clock every peer agrees on.** A wall-clock timestamp is
  author-asserted, so a lying `createdAt` extends a proof's life and the claimant
  is exactly the party with the motive. This is §13's author-asserted-clock
  problem — the one it says "must be clamped" — arriving at credentials; §13 owns
  the answer, and it is not to be re-derived here.
- **The window belongs to the credential type, not to the forum.** A LEZ balance
  can move in one block; an ENS registration lasts a year. One global constant
  would be wrong for both.
- **Expiry costs liveness.** A user offline for longer than their window silently
  loses standing. Whether that wants a grace period is **open** (§13) — but it is
  a decision, not a bug, and must not be discovered by the first person it happens
  to.

#### Composability: two levels, and why the interesting one works

Credentials are to be **composable**, at two levels: a Stoa adopting an identity
system its participants install, and a fork adding a credential type of its own.
**The first is preferred; the second is the fallback.**

**Level 2 — a fork adds its own credential type — is nearly free**, and needs
nothing from this design: a fork controls its own op format, so it allocates a
kind and ships. The one thing that would prevent it is door-closer 2 — welding
credential checks into op verification would force a fork to fork the verification
path too — which is a second reason to hold that seam.

**Level 1 — a Stoa adopts an identity system its participants install — is the
preferred direction, and it works.** "This Stoa supports Base NFT series 123;
click here to install `dialectica-nft-eth`" means **peers within one Stoa run
different module sets**, and a peer without the module cannot evaluate the proof
at all.

**That is fine, and the reason is that a proof does strictly two things, both
local:**

1. **It changes the relevance score of messages** — a local fold, exactly like a
   vouch.
2. **It may display extra information** — a local rendering decision.

So a reader who has not installed the module scores those authors as holding
nothing. **This is not a degraded mode, and that is the whole point: "module not
installed" and "holds nothing" are the same local answer, and neither reader is
wrong.** There is no convergence problem here because there is nothing to
converge on.

**Recorded because it is the mistake this section made once:** it is tempting to
reach for `moderation-resolution`'s requirement that two readers resolving from
the same ops reach the same outcome, and conclude that divergent module sets break
it. That applies the wrong section's rule. It governs *moderation*, which must
converge because a hide binds what everyone sees. **Scoring never had that
requirement and could not have**, and §7.3 is the proof: a vouch is per-reader and
**never published**, so two readers holding *identical* ops already compute
different scores by design.

**This is a new cause of that divergence, not the one §7.2 rule 1 names.** Rule 1's
divergence comes from peers holding different op sets; §7.3's comes from private
per-reader state; a missing credential module is a third — same ops, different
module sets. What matters is that §7.3 established the *class*: divergence in
ranking is correct rather than broken, and the mechanism producing it is free to
vary.

**The boundary, which is the rule a future reader must not cross:**

> **A supplementary, externally-anchored proof may influence what a reader sees
> and how they rank it. It may never determine what binds.**

Moderation authority stays convergent, decided from the ops and the moderator set
alone; relevance and display stay local. **This is the axis §7.3 already drew**
between a vouch and a moderation — a reader may privately weight whose judgement
they trust, and that never touches what a moderator's hide does — and §7.2 rule 4
draws the same line from the other end, where "a credential may amplify promotion
but never suppression". Credentials of this kind land on the vouching side of it.
The axis is not new; it is newly applied.

**The qualifier is load-bearing, because §7.1 is the deliberate exception.** A
token-gated Stoa declares "holds ≥ N of token T" as a **posting policy**, and that
is a credential which does determine what binds — §7.1 calls it "a claim in §5.5's
terms" and §7 puts policy gating *first* in the sequence, before credential-gated
scoring, with "both arrive through §5.5's claims interface." The two are not in
conflict because they are different things:

- A **genesis-declared posting policy** is immutable, inside the address preimage,
  and fail-closed by construction — an unknown policy discriminant is refused
  rather than defaulted (§13). Every peer holds it because every peer hashed it to
  get the Stoa's address. That is precisely why it *can* gate, and it is the one
  case where the convergence machinery genuinely is required rather than avoidable.
- A **supplementary credential** is optional, per-reader, and evaluated by a module
  a given peer may not have. That is why it may only add weight.

So the rule above governs the second kind. §7.1 is not an exception to be
reconciled later; it is the reason the qualifier exists.

**A moderator decides what the Stoa honours, and that half does converge.** Which
NFT series on which chain, which token on which LEZ — that is a judgement about the
Stoa, so it belongs to whoever moderates it. The separation is the organising idea
and it is clean:

| | Converges? | Why |
|---|---|---|
| **What the Stoa honours** — which series, chain, token | **Yes** | A moderator decision, so a moderation fact like any other: decided from the ops and the moderator set |
| **Whether a given reader can evaluate it** | **No, and need not** | Local module set, local score, local display — the two strictly-local effects above |

So the declaration travels the **existing moderator-authority path** and only the
evaluation is local.

**Where it lives is unexamined, and the obvious answer is the one a spec already
argues against.** A `StoaMetadata` op (§5.7) looks right — mutable,
moderator-authored, authority checked on read, and a Stoa's accepted proofs will
change over its life. But the `stoa-metadata` spec carries a requirement titled
"Current metadata does not include the posting policy", and its reasoning is
**authorisation-shaped, which a credential declaration also is**: "a rename is
cosmetic; a policy change is authorisation", and "the fallback rule inverts safely
for a title and unsafely for a policy" — a peer that missed a *tightening* falls
back to the looser founding value. A peer missing a declaration that **narrows**
what a Stoa honours would fall back to honouring **more**, which is the same
silent widening by the same route.

That spec also names what adding such a field would require: a second accepted
value so a change is expressible, a settled ordering so "the current declaration"
is determinable, and a fallback rule specified separately and **fail-closed**. So
the honest statement is that **a credential declaration needs the `stoa-metadata`
spec's fail-closed test applied to it, and that has not been done.** Genesis is
not simply "wrong" either — it is immutable, which is a real cost if accepted
proofs change, and a real safety property if they should not silently loosen. **No
field is designed here and none should be**; this records the tension, not the
encoding.

**A forged declaration must not be able to make a reader install anything or
weight anyone**, and the answer needs no new mechanism:

- The declaration is an op, so it is forgeable only by a current moderator — §6's
  read-time authority check already covers it. A non-moderator's declaration is
  authentic and is not a declaration, which is the distinction §6 exists to draw.
- **Acting on it is always the reader's choice.** A Stoa saying "install
  `dialectica-nft-eth`" is a recommendation rendered to a human, never an
  instruction a client follows. A module name arriving over the network is
  attacker-influenced content in the same class as a Stoa address embedded in a
  post — and `docs/UI-BRIEF.md` already settles that class: "render it as an
  affordance the reader chooses to act on. **Never auto-join.**" Same rule, new
  surface.

**Two interface notes**, recorded here because no surface exists yet and
`docs/UI-BRIEF.md` describes only what does:

1. A Stoa recommending a module must not present its absence as **brokenness**. A
   reader without it sees a *correct* view of the Stoa, not a partial one. There is
   no "unverified versus not a holder" distinction to render, because there is no
   abstention — a reader either has the module and sees ownership, or does not and
   sees nothing.
2. A module recommendation is **attacker-influenced content**. Never auto-install
   and never auto-fetch; show what is being suggested and let the reader decide,
   exactly as the join-a-Stoa flow does with an address.

#### The substrate goal, and why it is not now

A stated goal, **deliberately deferred**: dialectica's *data* should be
unopinionated enough that someone else can build a different UI, a different
relevance score, or a different moderation system on the same op log. That is a
later review and an explicit non-goal for now — recorded because it changes how to
read some decisions already made, not because anything should be built toward it.

**Three choices already serve it, and each was made for another reason**, which is
the through-line worth seeing. §3.3's op log stores "inputs and never
conclusions" — no score, no vote tally, no hidden flag — so the inputs survive for
someone else to fold differently, which is exactly what an alternative relevance
score needs. §6 and §5.7 decide authority and currency **on read** rather than
baking them in at write, so an alternative moderation system reads the same ops
and reaches its own conclusions. And the op format being the whole surface —
"anything not expressible as an op is not expressible at all" — means there is no
side-channel state a re-implementer would have to reverse-engineer.

**The honest counterweight**: a substrate and an application pull in opposite
directions, and this project has consistently chosen the application. The
moderation resolver's `Hide`-preferring tie-break is a policy decision baked into
a resolver; §7.2's weights are policy; the rendering obligations in
`docs/UI-BRIEF.md` are policy. None of that is wrong — an application that refuses
to conclude anything is not a forum — but a future split would have to separate
"what the ops say" from "what dialectica concludes from them", and **that boundary
does not currently exist as a boundary.** It runs through the resolvers rather
than between them and anything else. Whoever conducts that review is deciding
where to *draw* a line, not looking for one already there.

### 5.6 The keystore

**Built** — see the `keystore` and `posting-capability` specs. An encrypted root
secret at a caller-supplied path, two non-interactive unlock paths, and
`getCapabilities()` for a view to gate posting on. The module never prompts.

What is **not** built: the third unlock path, an **agent**. Deferred on
proportion rather than difficulty — it is a long-lived process holding decrypted
material and answering a socket, and it buys nothing until a human is repeatedly
typing a passphrase. Until it exists, the passphrase-by-environment path is
readable by other processes running as the same user, which is a real limitation
of that path rather than a bug in it.

---

### 5.7 Mutability: revision, not shared state

**A post is never edited in place. An edit is a new version of that post,
published and signed by the same author.**

This is the whole conflict rule for post content, and it is deliberately not a
merge strategy. Two versions of a post do not *conflict* — they are ordered:

- **Authorship decides validity.** A version signed by anyone other than the
  post's original author is invalid and dropped on read (§3.3). There is no
  case where two authors contend for one post.
- **The ordering rule decides currency.** Among an author's own versions, the
  highest Lamport timestamp is current, ties broken by ascending message id —
  the same rule SDS already applies (§4.4), so nothing new is invented.

  **But no Lamport value reaches us today** (§13), so in the running system
  every version falls to the degraded order: ascending op id, which is a hash
  and carries no recency. What currency means right now is therefore
  *convergent* rather than *temporal* — two peers holding the same versions
  agree on which is current, though neither can say which was written last.
  The rule above is what the same code yields unchanged once the upstream gap
  closes.
- **History is kept.** Superseded versions stay in the op log. The UI can show
  that a post was edited, and a moderator acting on a post is acting on a
  version they can name.

**The resolver that applies this rule is built** (see the `post-revision` spec):
`dialectica-core`'s `revision::current_version` answers "which version of this
post is current?" over the op log. Every version names the original post —
revisions do not chain — and the reasoning for that, for the order in which a
version is verified and its authorship checked, and for what the current
ordering regime does and does not guarantee, is in the change's `design.md`.

The reason this matters beyond edits: it means dialectica has **almost no
shared mutable state**. Posts and replies are append-only; an edit appends too.
SDS gives an order, and an order plus "only the author may revise" is a complete
answer — no CRDT, no merge function, no last-writer-wins ambiguity.

### What this rule does not cover

Three pieces of state are not authored revisions of a post, and each needs its
own answer:

- ~~**Moderation flags.**~~ **Built; see the `moderation-resolution` spec.** Not
  the author's to revise, by design — a moderator's hide and the author's edit
  are about different things and do not contend. Among *moderation* ops on the
  same target, last-write-wins by Lamport order, valid only if the signer was a
  moderator at that time (§6).
- **Stoa metadata** — title, description, policy (§7.1). Owned by the Stoa's
  moderators rather than by any author, so the same moderator-scoped
  last-write-wins rule applies.

  **The genesis record now carries a founding title and policy** (see the
  `stoa-genesis` spec), which makes the relationship concrete: genesis values
  are what the *address commits to* and can never change; the metadata op
  carries what the Stoa is called *today*. A reader prefers the latest valid
  op and falls back to the genesis values.

  **The op is built** — `op.rs`'s `StoaMetadata` kind, carrying a title and a
  description (see the `stoa-metadata` spec). It carries **no `policy`**; the
  reasoning, and what would have to be true to add one, is in the
  `stoa-metadata-op` change's `design.md`.

  **Resolution is not built**, but is no longer blocked: `arrival::cmp_ops`
  applies §5.7's rule, with a defined degraded order while the transport
  supplies no ordering metadata (§13). The op accumulates; nothing reads it
  yet.
- **The moderator set itself.** Deferred with mutable moderation (§6), and the
  one place where a real ordering decision is still open — a set edited
  concurrently by two moderators is the first genuine merge question this design
  has. It does not arise while the creator is the sole moderator.

## 6. Moderation

> **Out of the MVP, by owner decision — scope, not a design change (§9.2).** The
> MVP ships **no moderation UI and no moderation op-publishing path**. Nothing
> here is withdrawn and nothing is deleted: `moderation::resolve` and its spec
> are built, tested and merged, and they stay. The read-time authority check
> below **remains the design** whenever the publishing half lands, and the
> limitation two paragraphs down — that an `Unhide` cannot currently win — is
> the reason the sequencing is comfortable rather than merely convenient: a
> moderation UI shipped today would have to carry an irreversibility warning at
> the point of action. Read the rest of this section as the design that is
> waiting, not as behaviour the MVP has.

**Signed ops with a Stoa moderator set.** Every op is signed by its author.
Moderation ops are valid only when signed by a current moderator, and every peer
verifies independently — so a hide binds for everyone running honest code.

```
genesis: {version, creator_pk, policy, title}   ← built; see the stoa-genesis spec
post:    {..., sig(author_sk)}
hide:    {..., sig(mod_sk)}    ← rejected if signer ∉ moderators
```

**The read-path check is built** — see the `moderation-resolution` spec.
`dialectica-core`'s `moderation::resolve` answers whether a target is hidden,
checking authenticity, authority and the op's own Stoa on every read, and names
the op that decided. `Moderate` carries `Hide` or `Unhide`, ordered by §5.7's
rule. Not built: applying it in a materialised view, and everything below that
depends on a mutable moderator set.

**"A hide binds" is conditional on an ordering we have not built yet, and that
is the sharpest limitation in this section.** §5.7 orders competing moderations
by Lamport timestamp; no Lamport value reaches us and — per §13 — **none is
coming, because ordering at forum scope was never the transport's to supply.**
So every op is currently unordered and the fallback is ascending op id. A
`Moderate` op carries no nonce and no timestamp, so for one Stoa, one moderator
and one target there are **exactly two possible ops** — and an unordered
comparison between them resolves the same way forever. Last-write-wins has no
"last" to consult.

The resolver closes the dangerous half by preferring `Hide` when neither
candidate was transport-ordered, so a pre-emptive `Unhide` cannot veto future
moderation. What that does **not** restore is the ability to *reverse* a hide:
until Lamport values arrive, an `Unhide` competing with a `Hide` of the same
target loses regardless of when it was published. A moderator who hides
something by mistake cannot currently un-hide it by publishing an `Unhide`
alone. That is deliberate — the alternative was the veto.

**What closes it is dialectica's own Lamport counter (§13), not an upstream
fix.** This is a change of owner rather than of mechanism: the resolver code
needs no change either way, but the work is ours and schedulable rather than
somebody else's and indefinite. **Nobody should be waiting for it.**

**This becomes a UI requirement the moment a hide button exists**, and there is
no user-facing surface yet to carry it, so it is recorded here for whoever
builds one. A moderator pressing "hide" is currently taking an action that
cannot be undone on the peers that matter, and nothing in the core will warn
them — `moderation::resolve` answers what is hidden, not what a future reversal
would do. The honest interface says so at the point of action rather than
offering an "unhide" that silently fails to bind. **When dialectica's Lamport
counter lands, the warning is removed along with the tie-break, and the two
should be removed together.**

The record carries **no per-peer value** — no epoch, no session counter. Every
peer hashes it to obtain the Stoa's address, so a value varying with one peer's
history gives that peer a different address for the same Stoa: two Stoas that
cannot see each other, with no error anyone observes. §4.3 states the same rule
for the channel id derived from that address.

**The creator is the sole moderator initially.** A mutable moderator set is
later work — which also defers the founder-as-permanent-root question rather
than answering it prematurely. A Stoa whose moderation people dislike can be
forked — a Stoa's participants are never locked into its moderation.

### 6.1 What moderation can and cannot reach

**Nobody can be prevented from publishing, and there is no membership to
revoke.** SDS has no member list (§4.4) and nothing stops a peer putting a
message on a channel. So moderation cannot remove a person from a Stoa; it can
only change what conforming peers *render*. That is the honest ceiling, and it
is worth stating because every forum design assumes a stronger one.

Within that ceiling, a moderation op binds properly. **A moderator publishes a
`hide`; every peer verifies the signature against the moderator set at that
Lamport time and independently reaches the same answer.** It is protocol, it
converges, and it names an `opId`. Built, against the constant set a sole
creator-moderator gives; "at that Lamport time" becomes a real distinction only
when the set can change.

**An author-scoped suppression op is equally available, and is not ruled out.**
Nothing stops a moderator publishing a signed op naming an *identity* — suppress
this pseudonym in this Stoa — which converges exactly as `hide` does: §5.2 makes
the identity stable and Stoa-scoped, and §5.7's moderator-scoped
last-write-wins already orders it. It differs from `hide` only in what it names,
and it is the closest thing to a ban this design can have. **Deferred with the
mutable moderator set (§6), not rejected** — a single creator-moderator has
little use for it, and it wants the same care about reversibility that `hide`
does.

What is *not* available at any point is a ban that stops publication. A
suppression op tells honest peers not to render someone; a peer running modified
code renders them anyway, and the messages still occupy the channel.

The reason to be pedantic: a design that assumes it can expel someone starts
expecting a stronger guarantee than the transport provides, and reaches for
consensus machinery to get it. There is nothing to converge that a signed op
does not already converge.

**Relevance carries most of the load regardless** (§7.2), and its local,
never-published nature is a deliberate difference in kind rather than a weaker
version of the same thing: a moderation op is one authority's binding judgement
about one target, while a ranking is every peer's own reading of what it has
seen. It is why §5.2 keeps an identity stable enough for a signal to attach to.

### 6.2 Threshold moderation, later

The intended direction, once a Stoa has several moderators: a moderation action
takes effect when **N of M** moderators have signed it, rather than on any
single moderator's signature. It raises the cost of one compromised or rogue
moderator key, and makes a contested removal a decision the moderator group
makes rather than a race.

This is strictly downstream of the mutable moderator set — with a single
creator-moderator there is no threshold to take. The sequence is: creator-only
→ mutable moderator set → threshold rules.

Design shape when it comes: a moderation certificate is an op carrying N
independent signatures over the same `(target, action, epoch)` tuple, valid iff
each signer is a moderator at that Lamport time and the count meets the Stoa's
declared threshold. Verification stays a local check — no coordination protocol,
no aggregation service.

**LP-0016** (`logos-co/lambda-prize`) is worth reading for this specifically.
Its identity model is the opposite of ours — anonymous posting with identity
recoverable only as punishment — so most of it does not transfer. But its N-of-M
certificate aggregation is orthogonal to whether posters are anonymous or
pseudonymous, and that is the part that maps onto this design.

Read one more part than that, though, against the day §5.3's rotation arrives:
its **K-strike revocation**, where each post embeds a Shamir share of the
author's nullifier secret, so that accumulating K moderation certificates
reconstructs the secret, slashes the membership, and retroactively links that
author's prior posts. It is the worked example of standing attached to a
revocable credential rather than to a keypair — which is what §5.3 says has to
exist before rotation can. **Read its status carefully before copying it**: by
its authors' own account the Basecamp packaging is in progress, one deployed
instance exercises parameterisation on chain but not posting or slashing, and
the N-of-M aggregation this section calls the transferable part is demonstrated
with the backend holding all N moderator secrets and signing N votes itself. The
logic exists; the multi-party property does not yet. Costs are
real: on-chain registration and slashing, a stake, and seconds-scale proof
generation per post.
Far beyond v1, and the right thing to read before designing v2's revocation
rather than now.

This layer is not optional and not a nice-to-have: **no layer below provides
authenticity.** SDS has no membership and its `senderId` is an
application-chosen string the spec itself notes "is not used for much".
Moderation that anyone can forge — or forge the removal of — is not
moderation. This is the part of dialectica that nothing else provides, and it is
where the core's real design work lives.

**This is not hypothetical.** The nearest kin project checks moderator authority
only on the send path and never on the read path, so any peer can forge a
moderation — or forge the removal of one — in shipped code. Appendix A has the
detail. Read it as the empirical argument for this section rather than as a
criticism of a sibling project: it is the failure this design exists to avoid.

---

## 7. Spam and sybil resistance

**v1: moderation plus client-side rate limiting. No sybil resistance claimed.**

That claim would be false if made. RLN today is free to mint
(`MembershipFee = 0`, faucet-funded), slashing is a `# TODO`, and it is switched
off on the target network (`rlnRelay: false` on `logos.dev`). Registering 100
memberships costs 100 faucet claims and buys 100× the posting rate.

Later, in this order — **and the order has changed**:

1. **LEZ proof-of-holding**, brought forward, as a Stoa access policy (§7.1).
2. **RLN** as a rate-limit credential — and the precondition for §7.2's scoring
   and for §5.3's rotation.

This section previously ran RLN first, on the reasoning that rate limiting is
the more fundamental protection and that §5.3's rotation waited on it.

**What the reorder actually buys is a policy mechanism, not a relevance
signal.** §7.1's token-gated Stoas work the moment a holding proof exists, and
they need no sybil resistance to be meaningful — a Stoa either admits you or
does not. That is deliverable today, because LP-0005 was live on LEZ testnet
when this was written while RLN's `MembershipFee` was 0, slashing was a `# TODO`
and `rlnRelay` was `false` on `logos.dev`. **Re-check both before acting on this
order** — the ordering is a claim about readiness at a moment, and readiness is
exactly the sort of thing that changes.

**It does not unblock §7.2's credential-gated scoring**, and the plan should not
pretend otherwise. §7.2 rule 2 does ship an interim engagement ordering, but
explicitly *without* a sybil-resistance claim and with a named expiry (rule 6);
the gate rule 3 describes is what a holding proof would have to deliver and does
not. That is RLN's job rather than a holding proof's: §5.3's ordering argument
wanted a credential that rotating cannot shed, and a holding proof is weaker —
tokens move between accounts, and one holding can back several presentations
unless it is nullifier-bound (rule 3). So the sequence is: policy gating first
because it is ready, scoring when RLN lands. Both arrive through §5.5's claims
interface.

Sybil resistance ultimately lives in the membership-allocation service's
pluggable auth hook (LIP 158), which is unbuilt — design against it, do not
claim it.

### 7.1 Token-gated Stoas

A Stoa that wants to restrict who may post can declare a **token-holding
threshold** as its posting policy: "holds ≥ N of token T". This is one variant
of the declared-policy question in §12, alongside open / invite /
first-post-approval — not a separate mechanism.

It composes with what is already decided rather than disturbing it. The policy
lives in the Stoa's **genesis record**; the proof is a **claim** in §5.5's
terms, verified by the identity layer. The op format, the signing model and the
moderation rules are untouched.

**LP-0005** (`logos-co/lambda-prize`, dual MIT/Apache-2.0, live on LEZ testnet)
is the primitive: a presenter proves a shielded LEZ token balance meets a
threshold **without revealing the balance, the account identity, or the
`npk`** — verifiable on-chain via a SPEL program or off-chain.

The privacy property is why this is worth using rather than rolling our own. A
naive balance gate on a transparent chain reveals who holds what; here the gate
learns only that the threshold was met, which is the right default for a forum
where membership itself may be sensitive.

**That property is threshold privacy and nothing more.** It is not presentation
unlinkability — two proofs by the same holder are not established to be
unlinkable to each other, and the gate's `3010` binding requires the signer to
*be* the attested account. §5.5's first requirement therefore does not follow
from this primitive; §13 carries it as open.

Caveats for whoever picks this up: single contributor, and its Basecamp module
is a QML/C++ plugin rather than a `codegen.rust` cdylib — so its packaging is a
useful reference but its module shape is not ours. Take the circuit and the
crates, not the integration. Check its revision history before designing
against it: the gate was substantively rebound partway through, and an earlier
deployed shallow gate verifies no proof at all.

### 7.2 Relevance

**This is where dialectica is investing.** Nobody can be prevented from
publishing (§6.1), so what a Stoa *surfaces* is its main lever over what reading
it is like. Moderation decides what a Stoa refuses; relevance decides the
ordering of everything it does not, which is the larger part.

#### The six rules

Five are standing design rules. **Rule 6 is different in kind**: it is an expiry
condition on rule 2, and its whole purpose is that somebody trips over it later
— so it is listed here rather than left to be discovered inside the section it
governs.

**1. Relevance is a local projection, never an op.** No score is ever published.
A score is a column in the SQLite view (§3.3), derived from ops and rebuilt by
replay like everything else. This is forced anyway — ops are the authority and
a published score is a claim no peer could verify — but it also means ranking
can be retuned without a protocol version bump, which is the property you want
for the one part of the system that will be tuned repeatedly.

The consequence to state rather than discover: **two peers that have seen
different ops rank differently, and that is correct** (§3.3 — divergent op sets
are the normal case, not a fault). It follows from §4.4's eventual consistency
among active participants. Do not reach for a consensus mechanism to make scores
agree; there is nothing to agree on.

**2. v1 ships an engagement ordering, and calls it that.** Two orderings that
cannot be gamed by minting identities, plus one that can:

- **`new`** — Lamport order descending. §4.4 already defines a total order with
  a tie-break; reuse it exactly rather than inventing a second ordering.
- **`active`** — threads by the Lamport timestamp of their most recent
  non-hidden reply. Gameable only by *posting*, which moderation and rate
  limiting already govern.
- **`top`** — vote ops counted per distinct identity, weighted by **how much
  this reader weighs that voter's opinion**. A moderator's upvote weighs
  `K_mod`; a **vouched** voter's weighs `K_vouch` (§7.3); every other weighs
  1 — *some* score rather than zero, which is the interim's whole premise and
  what rule 3 removes later. A post's score is **floored at zero**, so
  downvoting can order a post last but never remove it.

**One vote axis, not two.** §7.4 records why: a two-axis split separating
quality from agreement has no published evaluation anywhere, and the measured
constraint on distributed moderation is latency rather than expressiveness.
Spam and abuse leave the ranking system entirely and go to the moderator as a
**report** (§7.4), which is the separation that has a track record — and which
binds, where a second vote axis would only have ranked.

**`top` is an engagement ordering, not a relevance signal, and the distinction
is the whole of the claim.** It reports how many distinct identities voted,
weighted by the one credential that cannot be minted. It does *not* report how
good or how relevant a post is, because identities are free to create and
nothing establishes that two votes came from two people.

**It is safe for exactly one reason, and that reason is not a property of the
design: nobody is attacking a forum with no transport, no discovery and no
users.** With `K_mod = 3`, four minted identities outvote a moderator, and no
finite weight changes that — minting is free, so every weight is defeated at the
same price.
A defence resting on the environment expires when the environment changes, which
is what **rule 6** exists to catch.

Never claimed for it: that it resists sybils; that two peers agree on it (rule 1
says they will not, and that is correct); or that it measures quality. §7
refuses to claim sybil resistance it does not have, and this ordering sits
inside that refusal rather than being an exception to it.

**Why ship it at all**, given rule 3's argument below is correct: §1 says a
forum where nothing can be removed is a firehose, and the same is true of
attention — a forum that cannot rank is a firehose with a timestamp. Rule 3's
end state is blocked on RLN rather than on anyone's effort, so waiting for it
means shipping nothing for an unbounded interval, and rule 1 makes being wrong
cheap. That trade is defensible while rule 6 holds and indefensible after.

**3. Weight by the credential, not by the vote count.** When proof-of-holding
lands (§7's reordered step 1), count **only votes carrying a valid claim**.
Votes from claimless identities contribute **zero**, not a discounted amount.

This is the single most important design decision in this section, and it is a
deliberate inversion of the obvious approach. The obvious approach — count all
votes, add a bonus for credentialed ones — leaves minting identities the
cheapest available lever, so the credential decorates a signal the attacker
already controls. Gating instead means the credential *is* the signal.

**The gate holds only if the credential cannot be presented more than once per
voter, and nothing available today establishes that.** §7 is explicit that a
holding proof is weaker scarcity than a rate-limit membership: tokens move
between accounts, and one holding can back several presentations unless the
proof prevents it. The requirement is a vote-bearing proof **nullifier-bound per
(Stoa, epoch)** — but LP-0005 does not claim that property (§5.5), so it cannot
currently be assumed from the primitive.

Which is why the rule is stated as a *shape* and not shipped: **this gate waits
on RLN**, whose per-epoch nullifiers are built for exactly this, rather than on
the holding proof §7 brings forward for policy gating. Gating on a
re-presentable credential would raise the unit cost of a sybil vote without
stopping one, and would claim more than it delivers.

**What this rule governs is the end state, and rule 2's interim ordering does
not contradict it.** The distinction the interim needs, which this rule did not
draw because it was written before the moderation resolver existed:

- A credential that **cannot be minted** may weight a vote today. A Stoa's
  moderator set is derived from its genesis record, and the creator's key is
  inside the address preimage — minting an identity does not mint a moderator of
  any existing Stoa. That is a real credential, verified on every read.
- A credential that can be **re-presented** may not, and that is still exactly
  what RLN is for.

Weighting by an unmintable credential is nonetheless **not a defence**, and rule
2 says so: it is better placed than the un-inverted approach this rule warns
about — OpChan's credentialed premium was a *tenth* of the raw vote it rode on,
where a moderator's is a multiple of it — and it is still defeated by minting
one more identity. The interim is justified by the absence of an attacker, never
by the weight. When rule 6 fires, this rule is what replaces it.

**4. Moderation filters, it does not penalise — and a credential may amplify
promotion but never suppression.** A post hidden by a valid moderator op is
**excluded** from the projection, not demoted. §6 says a hide binds; a
percentage haircut does not bind, it merely means a sufficiently upvoted hidden
post outranks a visible one. Keep hidden posts in the op log (§5.7 keeps
history) and let the UI offer a "show hidden" view — but the default feed omits
them.

**A moderator's *downvote* is the same mistake arriving from the other
direction**, and rule 2's weights therefore apply to an **upvote only** — for a
moderator's vote and equally for a vouched or earned one (§7.3). This rule
originally stopped a `hide` from being *weakened* into a
ranking nudge; nothing stopped a ranking nudge being *strengthened* by moderator
authority into a soft hide. An amplified downvote has none of a moderation op's
properties — it does not bind (a well-upvoted post survives it), it names no
deciding op, and it has no specified inverse the way `hide` has `unhide`.
Offering a moderator a non-binding way to suppress something is worse than
offering none, because it is the nearer tool and does not do what it appears to.
**Suppression is a binding judgement or it is nothing.**

**5. Decay must be indexable — and there is currently no age to decay.** §2.5's
paginated API has to `ORDER BY … LIMIT` in SQLite, and a score recomputed from
the current clock on every read cannot be indexed. Store a decay-free score plus
a timestamp and apply decay in the `ORDER BY` expression.

**The reason decay ships disabled is not that it was not got to.** No value in
the system expresses a post's age: `op-format` forbids an op from carrying a
wall-clock timestamp ("a wall clock is a field the adversary sets" — exactly the
failure Appendix A measured, where a post claiming a future time got an unbounded
multiplier), `op-ordering` forbids substituting a local clock for missing
metadata, and a Lamport timestamp is **a counter, not a duration** — it orders
two ops without saying whether an hour or a year separated them. The only clock
available is the receiving peer's own, recorded per peer. So decay is blocked on
the transport supplying an authorship time, which nothing currently plans to.

The epoch stored against a decay-free score is therefore the op's **Lamport
timestamp**, never a receive-clock reading: a per-peer `CLOCK_REALTIME` value
would make two peers rank the same ops differently for a reason unrelated to
which ops they hold, and that is not the divergence rule 1 blesses.

What the projection must nonetheless reserve, because retrofitting it is the
expensive version: the decay-free score and its epoch; an **op-id tiebreak** in
the index (equal scores are the ordinary case — every unvoted post ties — and
without a total order `LIMIT` pagination silently repeats and skips rows); and
**vote counts partitioned by voter rather than pre-weighted or pre-summed**.
That last is what makes rule 1's promise true: a schema storing
`weighted_total = plain + K_mod·moderator` has baked the weight into stored
data, so changing it needs a full replay rather than an `ORDER BY` edit — and it
is wrong anyway, since a voter's authority resolves on read and is not a property
of the vote. §7.3 makes this sharper still: a vouched voter's weight is not even
a property of the *Stoa*, since it differs per reader, so weights must be joined
at query time rather than stored against a vote under any scheme.

**Rule 4's exclusion has to be indexable too, and it is not this same problem.**
A filter over a property resolved on read looks like rule 5 in another hat, but
the two differ in boundedness: a decayed score changes *every row's* sort key
*continuously*, whereas the hidden set is small, enumerable, and changes only
when a moderation op arrives. So it is a derived set with explicit invalidation
points — an ordinary materialised column. Store `is_hidden` on the view row,
written by calling the moderation resolver rather than by a second copy of the
authority rule in SQL, recomputed on moderation-op arrival (the arriving op may
not be the deciding one) and swept when a late genesis record lands. Index it
**ahead of** the score, so the query seeks and walks instead of ranking,
filtering and ranking again. It must never be encoded as a large negative score
offset: that would be indexable too and would silently restore the haircut rule
4 rejects, since a sufficiently upvoted hidden post would climb back.

**This is why rule 4's restriction of the weights to upvotes is load-bearing for
the schema and not only for the semantics.** Had a moderator's downvote been
given exclusion-like force — a threshold at which a post disappears — hidden-ness
would depend on a continuously accumulating vote count over an unbounded set of
rows with no enumerable invalidation points, and it would genuinely become rule
5's problem.

**6. The interim ordering expires, and here is the condition.** "The forum will
not get spammed just yet" is true and has an expiry date. A staged decision with
no named trigger is a permanent decision nobody admitted making, because no
moment ever arrives that obliges anyone to revisit it. Rule 2's `top` is
withdrawn or re-gated before the next release on whichever of these fires first:

- **A Stoa becomes discoverable without a human passing an address.** §4.8 Phase
  2's broadcast topic is the sharp line — it is "unauthenticated and spammable"
  by its own description, and it is the moment an attacker can *find* a Stoa to
  attack. **This is a precondition, not a warning: `top` must not ship enabled
  in the same release as broadcast discovery.**
- **A moderator can no longer read every post in their own Stoa within a
  session.** That is the moment the score stops being decorative and starts
  deciding what gets seen, because nobody is checking the whole Stoa any more.
  **The moderator is the observer and the owner of this check** — it is a
  judgement they make from their own view, in their own client.

  **An earlier draft said "a Stoa exceeds a few hundred participating
  identities", and that was unanswerable rather than merely vague.** Rule 1
  establishes that two peers hold different ops by design, so there is **no
  vantage point from which a Stoa's identity count is a well-defined
  quantity** — and a count of freely mintable identities is attacker-controlled
  in both directions, so an attacker could trip the trigger or stay under it at
  will. A trigger needing an auditor this architecture cannot have is a trigger
  that never fires. The replacement is per-peer observable and has a named
  person, which is what the other three already had.
- **The first sybil attempt is observed** — a burst of votes, in either
  direction, from identities with no posting history. One is enough; the
  question was never whether an attacker *could*, only whether one had bothered.
  Rule 5's separate per-class vote counts make this a query rather than an
  investigation — **but a query nobody runs is not an observation.** The
  moderator owns it, the client surfaces it **unprompted** rather than waiting
  to be asked, and the cadence is *whenever the projection is rebuilt*, since
  that is when the counts change and it costs nothing extra. An alert a person
  must remember to go looking for is the same failure as no alert.
- **A nullifier-bound vote credential lands** (RLN, §7). Rule 3's end state is
  then available and the interim has no remaining justification.

The first three retire `top`; the fourth replaces it. **What expires is the
*plain* vote's non-zero weight, not the ordering itself** — §7.3's vouched class
survives every trigger, because a vouch was never a sybil defence and an
attacker's arrival does not invalidate it. **Whoever proposes §4.8 Phase 2
broadcast discovery owns this check** — it is recorded here rather than
only in the change that introduced `top`, because the first condition fires
inside someone else's change and they will not read that one.

#### The shape to reserve now

```
engagement = Σ_voters  w(voter) · dir(vote)
score      = engagement · decay(age)
```

with `dir ∈ {+1, −1}`, the sum floored at zero (rule 2), moderation handled by
exclusion rather than a term (rule 4), and `decay(age) = 1.0` until an age
exists (rule 5). `w` is the interim weighting today and becomes rule 3's
`w(∅) = 0` gate when RLN lands — a scorer change, not a schema one.

**This corrects an earlier shape that read
`f(engagement) · decay(age) · weight(author_claims)`, and the correction
matters.** That form weighted a post by its *author's* credential, where what is
wanted is weighting a *vote* by its *voter's*. Appendix A measured why the
difference is not cosmetic: OpChan's author multiplier scales a quantity the
attacker already controls, which is how a flat 25% author premium loses to three
free votes. A voter weight changes what the quantity is made of. **There is no
author term at all**, deliberately — nothing about who wrote a post should
multiply how much other people liked it, and if author standing ever matters it
belongs as its own additive term.

#### Where the rules come from

Rules 1, 3, 4 and 5 are each stated as an inversion because
`logos-messaging/OpChan` — the nearest kin, a Logos-ecosystem forum with a real
relevance implementation — shipped the un-inverted version. **Appendix A** has
the arithmetic and the citations; the one number worth carrying inline is that
in OpChan's scorer, **three free sybil upvotes outrank holding an ENS name**,
and a credentialed voter's premium is one tenth of the raw vote it rides on.
That is rule 3 as a measurement rather than an opinion.

**Rule 2 is the exception, and reading it as an inversion is the mistake to
avoid.** It ships something the measurement says is gameable, on the explicit
ground that nobody is there to game it. Appendix A is what the *end state* must
avoid and what rule 6's trigger protects against — not a refutation of the
interim, which fails to the same arithmetic and is justified by the environment
instead. The failure mode to watch for is someone later reading rule 2 as
evidence that the measurement did not matter.

Two design choices for proof-of-holding that OpChan never faced, because its
signal was binary and free:

- **Binary or graded?** LP-0005 proves "balance ≥ N" without revealing the
  balance, so the natural port is a **threshold**, not a count. Grading would
  need either a revealed balance or one proof per tier.
- **Unlinkability across presentations — unverified, and it may not hold.**
  Presenting the same holding proof under a user's pseudonym in two different
  Stoas links those pseudonyms unless each presentation is nullifier-separated,
  collapsing the cross-Stoa unlinkability §5.2 claims. LP-0005 claims threshold
  privacy, **not** presentation unlinkability, and its `3010` account-binding
  may preclude it (§5.5, §13). Settle this before a holding proof carries any
  weight in ranking.

### 7.3 Vouching: a reader's own trust, and why it is not published

Moderator weight (§7.2 rule 2) is one Stoa-wide opinion, and a Stoa has exactly
one moderator today. That makes the interim ordering a fair description of what
*the creator* likes, which is not the same as relevance and does not scale past
a Stoa small enough for one person to read.

**A reader may therefore vouch for an identity**: "this pseudonym produces good
judgement, weigh their votes more heavily **in my ranking**." It is the answer
for someone who has earned standing with actual readers but holds no system
credential — no moderator key, no token, no RLN membership — which is the
majority of anyone worth reading.

#### The vocabulary, and what each rejected word would have implied

**"Vouch"**, because the alternatives each smuggle in a different mechanism:

- **"follow"** already means *show me their posts* in every forum anyone has
  used. Reusing it welds feed subscription to vote weighting, and users want
  those separately — plenty of people are worth reading and unreliable at
  judging others, and the reverse is commoner still.
- **"friend"** implies reciprocity and a social graph. This is one-directional
  and needs no agreement from the other party, who is never told.
- **"trust"** collides with the word §7 uses for cryptographic guarantees. A
  "trusted user" reads as a system property; a *vouched* one is plainly
  somebody's opinion, which is exactly what it is.

So: a reader **vouches for** an identity, holds a **vouched set**, and the
scorer's weight classes are **moderator / vouched / plain**.

#### It is a local projection, not an op — and that is not a detail

**A vouch is never published.** It is local state, like the score it feeds
(§7.2 rule 1), and three independent arguments all land on that:

- **§5.2 makes a published vouch a privacy leak.** Identities are unlinkable
  across Stoas by construction. A vouch list that travelled — or that named
  identities in more than one Stoa — would re-link the pseudonyms §5.2 keeps
  apart, and would do it using the reader's own social graph, which is worse
  than the linkage §5.2 prevents. **A vouch therefore names a Stoa-scoped
  identity and is scoped to that Stoa**, with no cross-Stoa list anywhere.
- **A published vouch graph is a sybil amplifier.** If vouches were ops,
  minting identities that vouch for each other would manufacture standing, and
  the graph would be exactly the unmetered signal §7.2 rule 3 warns about with
  more steps. Keeping it local means an attacker can only affect *their own*
  ranking, which is not an attack.
- **It converges without any protocol.** Nothing has to agree. §7.2 rule 1
  already says two peers rank differently and that is correct; a vouch makes
  that divergence *intentional* rather than merely tolerated.

The consequence worth stating plainly: **vouching does not make anyone more
visible to anyone else.** It changes one reader's feed. Anybody expecting it to
confer status on the person vouched for has misread it, and the UI must not
imply otherwise — no vouch counts, no "N people vouch for this author" badge.
Such a display would be a published vouch graph reconstructed by eye, with all
three problems above.

#### Vouching accrues from what a reader already does

**An explicit vouch list nobody fills in is a feature that ships dead.** Asking
a reader to maintain a curation list is asking for work they did not come to do,
so the vouched set stays empty and §7.3 has no effect. The mechanism has to
start from behaviour the reader is already exhibiting.

**So a reader's own upvotes accrue weight toward the identities they upvote.**
Repeatedly upvoting someone raises that identity's weight in this reader's
ranking, without anyone declaring anything. Call it **earned** weight, against a
**declared** vouch, and keep the two distinguishable in the UI: one is something
the reader chose and can revoke in a click, the other is something they should be
told has happened and be able to undo.

**Only upvotes accrue. A downvote moves nothing.** The asymmetry matters for the
same reason it does in rule 4: accruing *negative* weight from downvotes would
let a reader's disagreements quietly build a filter that hides a viewpoint from
them, which is the failure mode this design is arranged against and is worse for
being invisible. Earned weight only ever raises.

**This is where §7.4's abandoned second axis leaves a real gap, stated rather
than papered over.** With one axis, an upvote means "worth reading" and "I
agree" at once, so weight accrues from a blend of the two — and the literature
§7.4 cites measures exactly that conflation. A reader who upvotes only what they
agree with will build a vouched set that agrees with them. **Nothing in v1
prevents that**, and the honest claim is that vouching makes a reader's
weighting *explicit and revocable* rather than making it viewpoint-neutral.
Bridging (§7.4) is the mechanism that would address it, and it is future work.

Two bounds, both structural:

- **Earned weight is capped below `K_vouch`.** Accrual is evidence, not a
  declaration, and it should never silently exceed what the reader explicitly
  chose. Without a cap, heavy engagement with one identity converges on
  delegating a reader's feed to them by accident.
- **It decays with disuse** — a reader's judgement of a year ago is weaker
  evidence about their present preferences than last week's. This wants the
  same age input §7.2 rule 5 says does not exist yet, so it is **specified as a
  property and deferred in mechanism**, alongside decay itself. Until then,
  accrual is bounded by the cap alone, which is the conservative failure.

#### Weights, and the one ordering constraint that matters

`K_vouch < K_mod`, and both are chosen rather than derived (§7.2 rule 2 says
the same of `K_mod`, and the honesty applies to both). The inequality is not
arbitrary: a moderator's standing is checkable by every peer from the genesis
record, where a vouch is one reader's private judgement, so the more accountable
credential should not weigh less.

`K_vouch` around 2 against `K_mod` of 3 sits inside §7.2's bracket — **but that
bracket was derived for an unmintable, peer-checkable credential, and a vouch is
neither.** It is per-reader, unverifiable by anyone else, and the only weight
class surviving rule 6. Whether the derivation transfers is **assumed here, not
argued.** Two reasons it might — the bracket's argument was about not drowning
organic engagement, which is indifferent to who issued the credential, and
earned weight is capped below `K_vouch` regardless — but neither is a
derivation. Treat the value as a starting point that inherits an argument made
about something else, and settle it when vouching is specified.

**Vouching amplifies upvotes only**, exactly as rule 4 requires of a moderator's
vote, and for the same reason: an amplified downvote is suppression without
moderation's properties. That it is *private* suppression makes it no better — a
reader who silently buries what their vouched set dislikes has built a filter
bubble with a ranking engine, which is at least a product failure and arguably
the thing a dialectic forum exists not to be.

#### What it does not become

**A vouch is not transitive, and this is a decision rather than an omission.**
Weighing the opinions of people my vouched set vouches for is a web of trust,
and a web of trust needs loop detection, depth limits, decay per hop, and a
defined answer when two paths disagree — and it recreates the sybil amplifier
locally, since one bad vouch imports a stranger's entire graph. If transitivity
is ever wanted, it is its own design with its own evidence, not a parameter.

**A vouch does not moderate.** It cannot hide, and it is not an input to
moderation. §6.1 keeps moderation a binding judgement, and a reader's private
weighting is the opposite of binding by construction.

#### Where it sits in the staging

Vouching is **an extension, scheduled after** §7.2's interim ordering ships, and
it is the one piece of §7 that **does not expire under rule 6**: it is not a
sybil defence and never claimed to be, so an attacker's arrival does not
invalidate it. When rule 3's credential gate lands and plain votes drop to zero,
vouched votes survive — **a vouch is a credential too, just one whose issuer is
the reader rather than the system.** That is the property that makes it worth
building rather than a stopgap: it is the only weight class here that stays
meaningful in the end state, and it is what stops that end state from counting
nobody but token holders.

### 7.4 One vote axis, a vouch, and a report

**An earlier draft of this section specified two vote axes — quality separated
from agreement — and a literature review contradicted it.** The design is now
Reddit's single axis plus §7.3's vouch plus a report to the moderator. What
follows is why, because the reasoning is the part worth keeping: the *problem*
the two-axis design was solving is real and measured, and the two-axis design is
simply not the intervention the evidence supports.

#### What the evidence says, including against the obvious design

**Against a second axis:**

- The only large quasi-causal study of vote mechanisms finds **no fault with
  up+down**. Difference-in-differences across 55 political subreddits that
  *changed* their reaction mechanism, 155M comments: up-only and up+down both
  associate with more deliberative, more civic discourse, and the **most
  demagogic** case is subreddits with **no reaction mechanism at all**.
- The one **randomised** removal of downvotes (a field experiment on a 3M-member
  subreddit) improved the scoreboard and **not the behaviour**: negative scores
  fell sharply, moderator removals did not move, and newcomers became *less*
  likely to comment again. Removing the downvote is not a proven intervention.
- **Two-axis voting has no published evaluation anywhere.** Two large forums
  have run karma-plus-agreement for years with no measurement of any kind, and
  their own design discussion reported the axes moving together the large
  majority of the time. Specifying it here would have meant shipping a mechanism
  whose only real deployments have never been assessed.
- The one *evaluated* rich-moderation precedent (labelled categories plus
  metamoderation) found the binding constraint was **latency, not
  expressiveness** — much of a conversation passes before the best and worst
  comments are identified, and only about half of unfair moderations were ever
  corrected. **A peer-to-peer forum inherits a worse version of that**, so
  spending the interaction budget on more expressive voting spends it in the
  wrong place.

**And yet the problem is real, which is why this section still exists:**

- Users **do** downvote disagreement, at scale and in defiance of every
  platform's stated norm. Half a million comments of users discussing their own
  voting show downvoting used as a suppression tool against views that challenge
  a community's norms.
- The cost to dissenters is **quantified**: users who engage with opposing views
  receive measurably fewer upvotes in their home communities. **Single-axis
  voting taxes good-faith disagreement.**
- Negative feedback **percolates**: downvoted authors post more, post worse, and
  go on to downvote others.

So the conflation is real and a second vote axis is not the answer to it. The
answer the evidence points at is a different ranking *function* over the same
single axis — see the bridging note below — and that is future work rather than
v1.

#### The three controls

- **Vote — `up` / `down`.** Exactly as §7.2 rule 2 describes, weighted by how
  much this reader weighs that voter (moderator, vouched, earned, plain). One
  axis, familiar, and no new op.
- **Vouch — §7.3.** The reader's own answer to "whose judgement do I weigh",
  accruing from votes they already cast. It is where "I disagree with you but
  you argue well" *can* live — a reader who finds someone consistently worth
  reading is free to weight them up whether or not they agree with them.
  **Nothing makes them do so, and §7.3 is explicit that nothing prevents the
  opposite**: with one axis an upvote blends "worth reading" with "I agree", so
  a reader who upvotes only what they agree with builds a vouched set that
  agrees with them. Vouching makes a reader's weighting explicit and revocable,
  **not viewpoint-neutral** — bridging is what would deliver that.
- **Report — to the Stoa's moderator.** Spam and abuse leave the ranking system
  entirely and go to someone with binding authority (§6). This is the axis split
  that actually has a track record: separating "rank this" from "this breaks the
  rules" is what the forums that work already do.

**The report is what makes the single axis defensible.** The two-axis design
existed to stop spam and disagreement being the same signal — and a report
separates them *better*, because a moderator's hide **binds** (§6) where an
assessment would only have ranked.

#### A report petitions, and that is its limitation as well as its point

An earlier draft argued a report is the wrong frame here because it petitions an
authority rather than expressing a judgement. That objection was right and is
kept, because it names the real cost rather than being dissolved by the
decision:

- **A report does nothing for the reader who filed it** until someone acts. A
  vote changes their feed immediately; a report disappears into a queue with
  exactly one reader.
- **That queue does not scale.** One moderator per Stoa, no trust-and-safety
  team, and the Slashdot finding above says latency is already the constraint.
- **Reports are weaponisable and there is no backstop.** Mass reporting as a
  harassment technique is well documented on centralised platforms, where
  targets at least have an appeals path. A Stoa has none.
- **Nobody has measured whether reports are precise enough to act on in a
  decentralised system.** Studies of federated moderation find operators fall
  back on instance-level blocklists rather than reports — which over-block and
  fragment the network. If this design rests on "reports reach the right
  moderator and are mostly valid", **that assumption is unmeasured anywhere**.

The mitigation that keeps the reader's objection satisfied: **a report also acts
locally and immediately** — it drops the post out of the reporter's own feed at
once, rather than only entering a queue. The reader is then evaluating *and*
petitioning, the control visibly does something, and a report that changes
nothing observable does not get repurposed as a super-downvote.

**A report is not a moderation.** It does not hide, and it never accumulates
toward hiding however many arrive — §7.2 rule 4's principle holds unchanged:
suppression is a binding judgement or it is nothing. A report is a *signal a
moderator may read*, plus a local filter for the reader who sent it.

#### Encoding

**No new op and no version bump for the vote**, which keeps its existing shape.
A report is a local action in v1 — it changes the reporter's own view and
surfaces to the moderator — so the question of whether it ever becomes a
published op is deferred rather than answered here. If it does, the vote op's
one-byte discriminant has 254 unused values and refuses unknown ones rather than
defaulting them (§11), so the extension stays fail-closed and cheap.

#### Future: bridging-based ranking, and why it is not v1

**This is the mechanism the evidence actually supports for surfacing good
arguments a reader disagrees with**, and it is worth recording precisely so that
nobody re-derives the two-axis design later.

Bridging ranks by whether people who *usually disagree with each other* both
rate something positively. Crucially **it is not a second vote axis** — it infers
the disagreement dimension from the existing single-axis rating matrix by
factorising it, so the interface stays exactly as above. The deployed instance
(a large platform's crowd-sourced fact-check system) has real measured effects:
substantial reductions in agreement with misleading claims and in resharing,
independently replicated.

**Why it is not v1, and what has to be true first:**

- **It is fragile under permissionless identity.** Published analyses show fewer
  than ten strategically placed ratings can push a meaningful fraction of
  low-quality items over the display threshold. The deployed system resists this
  only because the platform supplies sybil resistance out of band — phone
  verification, rater enrolment, per-rater impact scores. **§7 is explicit that
  dialectica has none of that.**
- **§7.3's vouch is the candidate replacement for that missing layer**, which is
  a stronger argument for vouching than §7.3 makes on its own. But it only works
  once vouch data exists, so bridging is **downstream of vouch being populated**,
  not parallel to it.
- It needs a rating matrix with enough density to factorise, which a new Stoa
  does not have.

So the sequence is: single axis and vouch now; bridging when there is both a
matrix to factorise and a sybil-resistance story to protect it. **Revisit it
alongside rule 6's triggers**, since the condition that retires the interim
score is close to the condition that makes bridging both possible and necessary.

#### What was rejected, so it is not re-proposed

- **A second vote axis** (quality separated from agreement). Unevaluated
  anywhere, spends the interaction budget on expressiveness where the measured
  constraint is latency, and its own precedents report the axes moving together.
- **A `contested` ordering built on a split-response signal.** The single-axis
  version of this — ranking by an even split — is **mechanically a "most
  polarising content" sort**: the structural analysis behind the percolation
  finding above shows a 50/50 split is exactly where a voter network is most
  polarised, and separate work finds much controversial content is merely
  off-topic. It has also **never been studied** for whether it surfaces anything
  worth reading. Bridging is the thing this was reaching for, and it is a
  different mechanism.
- **Auto-hiding on accumulated negative signal**, in any form. Refused
  structurally by §7.2 rule 2's floor.

#### Open: a disagreeing reply is a quality signal the vote axis loses

**Parked, not designed**, and it survives the move to one axis — arguably it
matters *more* now, because the single axis is exactly what cannot express it.

**Replying to say "I disagree" is implicitly "this is worth my time."** Nobody
writes a rebuttal to spam; they scroll past. So a disagreeing reply carries the
judgement the vote axis conflates away — and the reader who writes one may well
*downvote* the same post, which is the measured behaviour §7.4 cites.

Three reasons it is worth taking seriously rather than filing as a nicety:

- **It is an organic signal, needing no new control.** A reader who would never
  use an extra button has already expressed the judgement by writing.
- **It is costly, which makes it hard to fake.** Unlike a click, a substantive
  reply takes effort, so it resists the minting attack §7.2 rule 2 admits it
  cannot stop. That makes it a *better* signal than any button, not a weaker
  proxy for one.
- **It needs no new op.** A reply is already a `Post` with a parent (§11), so
  the data is present in the log today.

**What must be settled before it is built**, and why it is not obvious:

- **A reply is not necessarily a disagreement**, and inferring which from text
  is sentiment analysis — an expensive, locale-specific, wrong-by-default
  classifier that this project should not own. The alternative is counting *any*
  substantive reply as a weak positive signal regardless of stance, which needs
  no classifier but rewards pile-ons and flame wars — the failure mode of every
  engagement metric ever shipped.
- **It creates an incentive to reply rather than to vote**, and a forum that
  rewards replying is a forum that rewards argument volume. That may be
  acceptable here — argument is the point — but it is the kind of thing that
  looks fine in design and is corrosive in practice.
- **Self-replies and reply chains** would need excluding or bounding, or an
  author raises their own post by arguing with their critics.

**Weigh it against bridging** rather than in isolation: both address the same
gap, bridging has measured results behind it and this does not, but this needs
no rating-matrix density and no sybil-resistance layer. If bridging proves out
of reach for a permissionless forum, this is the fallback that was always going
to be the real signal.

---

## 8. Privacy posture

**All Stoas are public in v1.** Stated as a deliberate property, not an
oversight: delivery gives no confidentiality, so every Stoa is world-readable by
anyone on the network.

Encryption is a future direction, and the intent is to **use the chat module**
rather than reimplement its cryptography. It already does X3DH-style intro
bundles and end-to-end encryption; that is not work worth owning a second time.

It is blocked on upstream, not on us. Chat state is currently **ephemeral** —
"identity, conversations, and message history live in memory only. Restarting
an instance mints a fresh identity (with a new address)" — and `getIdentity()`
carries a deprecation note. Encrypted Stoas need persistent keys, so this waits
on that work landing upstream.

That makes it a **coordination item**, not merely a scheduling one: worth
raising with the chat module's maintainers early, since a second consumer
needing persistent identity is an argument for prioritising it.

What is protected today: cross-Stoa unlinkability (§5.2), and hashed topic
buckets so peers cannot map interest from topic names (§4.1).

**Within a Stoa, a pseudonym is stable and deliberately so** — §5.2 explains why
narrower scopes were rejected. So a Stoa's posts are linkable to one pseudonym
by anyone, and the SDS `senderId` links them at the transport layer as well
(§4.1). The property v1 claims is that the pseudonym does not reach across
Stoas, and nothing more.

Worth stating because it bounds even that claim: writing style, posting time,
the reply graph and the network layer are all available to any peer holding the
op log, and none of them is addressed here. Cross-Stoa unlinkability is a
protocol property, not an anonymity guarantee.

---

### 8.1 The mark

**Delta (Δ)** — dialectic, difference, change.

Not decoration: `dialectica-ui/metadata.json` must point `icon` at a **256×256
PNG** that exists in the built variant, or Basecamp silently skips the plugin.
A missing icon presents identically to a plugin nobody clicked. Needed for
Phase 0, not later.

## 9. Build order

**Phase 0 — prove the Rust module path.** A `codegen.rust` core plus a QML view
with a trivial API, the `dependency_overrides` LIDL bridge to
`delivery_module`, building and loading in Basecamp, with CI green.

This exists to retire §3.2 before anything depends on it. If that bridge does
not work the architecture changes, and that is a week-one discovery, not a
month-three one.

Two cheap things to settle in the same phase, both from §2.3: **which SDK
revision the builder's pin actually delivers** — which decides whether
subscription restart and per-call timeouts exist at all — and **what a panic in
a handler actually does**, worth knowing experimentally rather than by
inference, since it sets how defensive the guard must be.

> **Done — see `docs/PHASE0-FINDINGS.md`.** Both modules load in Basecamp, the
> view renders, the guard converts a panic to the error shape and the module
> keeps serving, the bridge carries a live call into delivery's own
> implementation, and CI exists covering §10's four jobs — `gh run list` for
> whether it is currently passing. The three questions are answered
> there, along with what is still *not* proven — no delivery node has been
> created, so no channel has been opened and the §4.3 channel reopen trap is
> untested; only one profile has been launched.
>
> The architecture stands. What changed is §3.2's JSON, §10's pins, §11's trap
> list, and the discovery that an unguarded panic **aborts the module process**
> rather than poisoning a mutex as §2.3 predicted — so the guard is
> load-bearing, not hardening.

**Phase 1 — core semantics in a pure inner crate.** Signing, the
Stoa/thread/post model, moderator-set verification, the op log and its SQLite
projection, query indexing — pure Rust behind `Transport` and `Store` traits,
tested against fakes with no node running.

> **The store half is done.** The op log has two implementations behind one
> trait — in memory and in SQLite — and the same behavioural suite runs against
> both, which is what a second implementor was for. What remains under "query
> indexing" is the **materialised view** §3.3 distinguishes from the log: the
> log carries the indexes its own reads need, and the view that a feed is
> rendered from does not exist.

**Zero SDK types in this crate.** That is not a preference: anything touching
`modules()` or `context()` calls `lp_*` symbols undefined in an rlib and will
not link into a test binary, so SDK types anywhere in the domain logic make it
untestable (§2.3).

**Phase 2 — wire the real modules.** Swap the fakes for `delivery_module`
channels and `storage_module`.

**Phase 3 — the forum.** Feeds, threads, composition, moderation UI. Expanded in
§9.1, which is where the API this phase needs is argued.

### 9.1 Phase 3 — the forum

**Why this is a section and not a line.** Every other part of this plan is
argued at length while the view had one sentence, and the asymmetry was not a
judgement that the view is simple — it is that nobody had written it down. The
consequence is concrete rather than aesthetic: **the view is the only consumer
of the core API, and the core API is the deliverable** (§2.5). `wire.rs` today
exposes a version string, a ping, a panic probe, a capability probe and a
delivery bridge — nothing that reaches an op, a log or a resolver. What is
written below is therefore not a UI plan that happens to name some methods; the
methods are the point, and the view is the argument for each one's existence.

**What this section is not.** It does not design appearance — layout, colour and
typography are not decisions this document should own. It designs what the view
must **know**, what it must **ask for**, and what it must **never do**. The
third category is §11.1's, and this section adds to that list rather than
restating it.

> **§11.1 "Rendering obligations, collected" arrives with the `vouching-state`
> change and is not in this file until that lands.** The references to it below
> are deliberate forward references rather than mistakes: this section
> *surfaces* three new obligations — an `Unhide` affordance that must not be
> offered as symmetric, a join confirmation that must show the address and not
> only the title, and a bidi obligation wider than the one §11.1 records — and
> their home is that list, not here. If
> §11.1 is absent when you read this, that change has not merged yet — which is
> a fact `git log` answers and this sentence should not.

#### The constraint everything below follows from

§2.1: Basecamp sandboxes the QML engine with a deny-all network access manager
and no filesystem access outside the plugin directory. **Every byte the view
renders arrives through a core method.** There is no fallback, no direct fetch,
and no "the view can just read the file". So a gap in the API is not an
inconvenience to route around — it is content that cannot be rendered at all.

Two consequences that shape every method named here:

- **A shape the API does not return is a shape no view can compute.** If the
  core answers "hidden: true" without naming the op that decided it, no view
  can offer a moderator the reversal affordance, because it has nothing to name.
- **Every cross-module call is IPC (§2.4), and a feed is a loop.** A method that
  answers one post per call turns a page of thirty into thirty IPC round trips.
  The paginated shape is not politeness; it is the only shape that works.

#### Staged, like §4.8, and for the same reason

§4.8 stages Stoa discovery so that each phase is independently useful and none
depends on a later one landing. Phase 3 wants the same treatment, and the test
is the same: **a partial forum must still be a forum.**

**Stage A — read a Stoa.** A feed and a thread view over one embedded Stoa
(§4.8 Phase 0), rendering current versions, omitting hidden posts. No composing,
no moderating, no joining. This is a usable forum for a reader, and it is the
stage that proves the projection and both resolvers reach a screen.

**Stage B — compose.** A compose affordance gated on `getCapabilities()`, a
reply affordance in a thread, and an edit affordance on the reader's own posts.
Requires a key; the probe already exists to say whether there is one.

**Stage C — moderate.** A hide affordance for a moderator, and the
irreversibility warning §11.1 requires. Only meaningful in a Stoa the reader
moderates, which today means one they created.

The "show hidden" view belongs to **Stage A**, not here, and the placement is a
decision rather than an oversight: it is a reader's affordance over a filter the
projection already applies, and it needs no key and no authority. Putting it in
Stage C would make "see what was moderated" a moderator privilege, which §6.1's
ceiling does not support — the ops are in every peer's log regardless.

**Stage D — reach another Stoa.** §4.8 Phase 1's address, which verifies the
genesis record it is offered with, and in-post addresses rendered as an
affordance rather than acted on. **The joining half is built** — see below and
the `stoa-membership` capability; the in-post affordance is a UI obligation and
is not. §4.8 Phase 1 records why "pasting an address is enough to join" was
wrong: a join takes the address **and** the record.

The ordering is not arbitrary and the dependencies run one way only. B needs A
because a compose box needs somewhere to put the result; C needs A and B because
a hide is an op and moderating needs the same publish path composing does; D
needs A because joining a Stoa you cannot then read accomplishes nothing. **No
stage needs a later one**, which is the §4.8 property worth preserving: if D
never ships, dialectica is a single-Stoa forum, which is a smaller thing than
intended and not a broken one.

~~**What is deliberately not staged here:** votes.~~ **Overridden by the owner's
MVP scope (§9.2), and publishing a vote is now contracted — see the
`content-authoring` spec.** The objection this paragraph raised is not refuted and
the spec does not paper over it: the requirement "A published vote is stored and
readable, and no ordering consumes it" states the bound rather than an effect, and
forbids the reply from describing one. What the vote *control* may honestly claim
in the interface is the live half of the question, and it is §9.2's to carry.

#### 1. What a feed is

**A feed is a paginated list of thread heads, not of posts.** The alternative —
a flat list of every post — was considered and is wrong for a reason that is
about the orderings rather than about taste: §7.2's `active` ranks *threads* by
their most recent non-hidden reply, which is not a property any single post has.
A feed of posts cannot express `active` at all without the view regrouping,
which is work the view has no data to do.

So `listThreads` is the read, and each item is one thread, identified by the op
id of the post that started it (§4.1: a thread is named by the id of its root).

Each item carries what a feed row must render without a second call:

- the thread's id, which is the root post's op id
- the root post's **current version**: its body and its own op id, which are
  different things the moment the post has been edited
- whether the root post has been revised (§5.7's "the UI can show that a post
  was edited")
- the author, as the per-Stoa address (§5.2) — never a name, because there are
  no names
- a reply count, and the id of the thread's most recent non-hidden reply

**Ordering is a parameter, and the accepted values are `new` and `active`**
(§7.2 rule 2). No `top`: with no sybil resistance there is no score, and an
ordering the core cannot compute is not one the API should accept and then
quietly serve as something else. An unrecognised ordering is an error, never
defaulted — the same discipline `stoa.rs` applies to an unknown policy
discriminant, and for the same reason: a view asking for `top` and silently
getting `new` has been told a falsehood no test will catch.

**Both of those orderings are currently degraded, and that is unresolved** —
§7.2 defines each in terms of a Lamport timestamp no op carries yet, so both
fall back to ascending op id today. Per §13 the fix is **ours** (a `createdAt`
and a Lamport counter in the signed preimage), not an upstream one, so this is
work that can be scheduled rather than waited on. Section 8 below carries the
question of what they may honestly be called until then; it is named here so
that nobody reads this paragraph as saying the orderings work.

**Hidden threads are omitted by default**, and the parameter that includes them
is explicit (§7.2 rule 4, §11.1). Worth stating precisely because "hidden
thread" is ambiguous: a thread whose *root post* is hidden is omitted from the
feed; a thread with some hidden replies is not, and its reply count is of
non-hidden replies. A hidden reply is a thread-view concern, not a feed one.

**What is honestly uncertain here.** Whether `hasMore` can be answered without
counting the whole result set is a projection question, not an API one — but it
is the kind of thing that decides a schema, so §7.2 rule 5's instruction to
settle indexing when the projection is designed covers it and this section
should not pre-empt it.

#### 2. What a thread view is

~~A thread view renders **current versions** (§5.7), and the design tension is
that "current" is not the whole truth a reader or a moderator needs.~~

**The thread read is contracted — see the `thread-read` spec.** It says what
identifies a thread and that the identifier does not move when the root is
revised; that membership is derived from the parent chain and a post's own
`thread` field is never trusted; that the items are flat, each naming its
parent, in the system's order with the root first; that pages tile with no gap
and no repeat; that a hidden root is returned marked rather than dropped while a
hidden reply is omitted by default; that an absent thread is refused where an
empty one is served; that the moderation state is three-valued and names its
deciding op; and that an author is reported as an address **and** a public key,
because the generated name derives from the key and the mark from the address.
The reasoning for each is in the `thread-read` change's `proposal.md` and
`design.md`.

~~`getThread` returns the root post plus its replies, paginated, each item
carrying:~~

- ~~the post's id **as a thread position** — the original's op id, which is what a
  reply names as its parent and what never changes across edits~~
- ~~the **current version's** op id, which is what a moderator acts on and what
  changes every time the post is edited. These are two fields because they are
  two facts; `revision::CurrentVersion` carries both for exactly this reason,
  and collapsing them would force every caller to re-derive one.~~
- ~~body and attachments, from the current version~~
- ~~`isRevised`~~
- ~~the author address~~ — **superseded**: an address alone cannot produce the
  generated name, which derives from the public key. The read returns both.
- ~~the parent post's id, so the view can render the reply structure~~
- ~~moderation state~~

~~**The moderation state must name its deciding op, not be a boolean.**~~
~~**What a reader sees of a hidden post, stated exactly.**~~ **Both contracted —
see the `thread-read` spec**, which states the three-valued state and its
deciding op, and that a hidden reply is omitted by default while a hidden root
is returned marked. Restating either here would give the rule two copies that
drift, and a reader finding the stale one cannot tell.

**What remains live is a gap the spec cannot close**, because it is a
divergence between two reads rather than a property of one: **the feed reports
moderation as a boolean and the thread read reports the three-valued object.**
Both are built from the same resolver, so a view must currently branch on which
call produced an item — which §2.5's "JSON shapes are source-independent"
forbids. The thread read's shape is the correct one; the feed's is the older.
Until the feed is brought to it, `restored` is a state the feed cannot express
at all. Recorded in `docs/UI-BRIEF.md` too, since a designer meets it on their
second screen.

**And one obligation the core does not meet**: a view must not render a hidden
post indistinguishably from a visible one in the show-hidden view. A reader who
asked to see what was hidden is owed the knowledge of which ones those were.

**The bidi obligation is wider than §11.1 currently states it, and that is a
third thing for that list.** §11.1 frames Unicode and bidi rendering around
*metadata titles*, because that is where it was found: the metadata op
deliberately does not sanitise, since normalising would break op-id agreement
between peers. The same reasoning applies unchanged to **every** attacker-
supplied string this section renders — post bodies above all, which are the
largest and least constrained of them, and also author addresses if a view ever
abbreviates one. `op.rs` preserves display text exactly and never normalises it,
by design and with a test pinning that; so the obligation follows the text
everywhere it goes, not only to the field where it was first noticed.

**Edit history is deliberately not in this call.** §5.7 keeps superseded
versions in the op log, and a thread view that returned every version of every
post would return most of a thread twice for a facility most readers never open.
`getPostHistory` is the separate call, taking a post id and returning its
versions in the order the resolver defines. It is a Stage B or later concern;
Stage A ships `isRevised` and nothing more, which is the honest amount — "this
was edited" is a fact worth showing even when "here is what it said before" is
not yet available.

#### 3. What composition needs

**Every posting affordance is gated on `getCapabilities()`, never on a build
flag** (§5.6, and the `posting-capability` spec says it as a requirement). The
failure this prevents is specific: a compose box the user typed into and cannot
submit has lost their draft, and losing typed text is the single worst thing a
forum client can do to someone.

The probe already exists and answers `{"canPost":bool, "identity":"…" |
"reason":"…"}`. What Stage B adds is the publish path:

~~- `createPost` — a new thread in a Stoa~~
~~- `createReply` — a post naming a parent~~

**Posting, replying and voting are contracted; see the `content-authoring`
spec.** It says what a caller supplies, what must hold of the op produced, what
the peer holds after, and every refusal — including that the reply names the op
id, that publishing is append-then-hand-off rather than send, that the identity
is derived from the Stoa and never a parameter, and that a reply names only its
parent with the thread derived from it. The reasoning for each, and for the
`createdAt` field that was considered and declined, is in the `authoring-content`
change's `proposal.md` and `design.md`.

Still to build: **`revisePost`** — a new version of one of the caller's own posts.
`post-revision` contracts which version is current; nothing publishes one. Its
refusals turn on authorship of a target op, which is why it was left out of the
authoring change rather than folded into it.

**What the view must never do:** show a compose affordance without having asked
the probe in the current render. The probe is cheap and re-determines its
answer on every call by design; caching it across a keystore change is how a
button outlives the key that justified it.

#### 4. What the moderation UI needs

The ceiling first, because it bounds the whole design: §6.1 — **moderation
changes what conforming peers render and nothing else.** It cannot unpublish,
cannot remove a person, and cannot stop a peer running modified code from
displaying anything it likes. A moderation UI that implies otherwise is
promising something the transport does not deliver.

Within that, Stage C needs three things — two methods and one thing core cannot
provide at all:

- **`getModerationCapability`**, answering whether the caller is a moderator of
  this Stoa — the moderation analogue of `getCapabilities()`, and needed for the
  same reason: a hide button that fails on submission is a hide button that
  should not have been rendered. Today the answer is "are you the creator",
  because the creator is the sole moderator (§6); the method is worth having
  under its own name so that a mutable moderator set is a change to its
  implementation rather than to every call site.
- **`moderatePost`** — publishing a `Moderate` op with a `Hide` or `Unhide`
  action, naming the target.
- **The irreversibility warning**, which is §11.1's first entry and is not
  optional. A moderator pressing hide today takes an action that cannot be
  un-taken on the peers that matter, because the tie-break prefers `Hide` when
  neither candidate was transport-ordered. Nothing in core will warn them:
  `moderation::resolve` answers what is hidden, not what a reversal would do.
  **The warning and the tie-break are removed together** when Lamport values
  arrive.

  **This produces a new obligation, recorded in §11.1**: an `Unhide` affordance
  must not be offered as though it works. A UI that shows hide and unhide as a
  symmetric pair is asserting a symmetry the resolver does not currently have.

The "show hidden" view is **not** in this list — it is Stage A's, and the
argument for that placement is above. Mechanically it is a parameter on the read
calls rather than a separate mode, so that the default and the exception go
through one code path.

#### 5. How a user reaches a Stoa

§4.8 stages this and Stage D implements its Phase 1: **a Stoa address is a
copyable string.**

~~`joinStoa` — take an address, verify the genesis record hashes to it, and
record it as one this peer reads.~~ **Built.** Creating, joining and listing
Stoas exist on the module surface, contracted by the `stoa-membership` capability.
The reasoning — including why the call takes the genesis **record** as well as the
address, which is where this section's signature was wrong — is in that change's
`design.md`. A hash verifies a record somebody hands over and cannot reconstruct
one, so "take an address, verify the genesis record" named no record for the call
to verify.

`getStoa` remains **not built**: it is the metadata-resolution call, resolving the
latest valid `StoaMetadata` op with a fallback to the genesis title (§5.7). The
metadata op exists and accumulates, and nothing resolves it. See "What the
resolvers do not provide" below.

**In-post addresses are attacker-supplied content.** §4.8 is explicit and this
section adds nothing to it except the mechanics: a Stoa address appearing in a
post body renders as an affordance the reader chooses to act on; acting on it
shows what is being joined — the Stoa's title and address — **before** joining;
and nothing auto-joins, ever. The relevant threat is not a malicious Stoa, which
a reader can leave; it is a reader who does not know they joined one. **This is a
UI obligation and is not built**, which is why it stays here rather than moving.

**The obligation this surfaces, also new to §11.1**: a Stoa's *displayed* title
comes from a metadata op signed by its moderators and is not unique, not
verified against anything, and freely chosen. Two Stoas may present the same
title. The address is the identity and the title is decoration, so a join
confirmation that shows only a title has shown the reader the forgeable half.

#### 6. The core API this requires

**Every method below is a claim on `wire.rs`'s future shape and a widening of
the deliverable, which CLAUDE.md asks be done on purpose.** They follow §2.5
without exception: JSON in, JSON out, `{"error":"..."}` as the only failure
shape, never a partial success. Pagination is `(page, perPage)` in and
`{"items":[...],"page":N,"hasMore":bool}` out. The feed and the Stoa listing both
implement it, so the precedent is set rather than pending — read the built shape
before proposing a variation.

Field names are illustrative; the shapes and the arguments for them are not.
**Methods marked BUILT are contracted by a capability in `openspec/specs/`, which
is the authority for their actual shape; the line here is a pointer, not a
signature.**

**Stage A — read**

```
listStoas({page, perPage})  -> BUILT: see the `stoa-membership` capability
getStoa({stoa})             -> {stoa, title, description, policy, isGenesisFallback}
listThreads({stoa, order, page, perPage, includeHidden})
                            -> {"items":[{thread, currentVersion, body, attachments,
                                          author, isRevised, replyCount, lastReply}],
                                page, hasMore}
getThread  -- superseded; see the `thread-read` spec for the contracted shape.
           -- Two departures from the sketch that was here: the author is an
           -- address AND a public key, since the generated name derives from
           -- the key; and there is no `order`, as for the feed.
```

`isGenesisFallback` is the field worth defending: §5.7 says a reader prefers
the latest valid metadata op and falls back to the genesis values, and those are
different epistemic states. A peer that has not yet received a Stoa's metadata
op is showing a founding title that may be years stale, and it should be able to
say so rather than presenting it as current. This is the §11.1 general shape
again — core's honest answer is incomplete without something the view says.

`moderation.state` is one of `unmoderated`, `hidden`, `unhidden`, mirroring the
resolver's enum; `decidedBy` is the op id, absent for `unmoderated`. Absent, not
null-and-present: §2.5 forbids a shape that is partly a success, and a field
that is sometimes meaningless is that shape in miniature.

**Stage B — compose**

```
getCapabilities({stoa})     -> exists today
revisePost({stoa, target, body, attachments})  -> {op}
getPostHistory({stoa, post, page, perPage})
                            -> {"items":[{version, body, attachments, isCurrent}],
                                page, hasMore}
```

**`createPost`, `createReply` and the vote method are contracted — see the
`content-authoring` spec, which supersedes the shapes sketched here.** One
departure is worth flagging because this file sketched it the other way:
`createReply` takes **no `thread`**. The thread is derived from the parent, so a
reply filed under the wrong thread is unrepresentable rather than checked, and the
cost — a reply to a parent this peer does not hold is refused — is contracted
rather than hidden.

**Stage C — moderate**

```
getModerationCapability({stoa}) -> {canModerate:bool, identity | reason}
moderatePost({stoa, target, action})           -> {op}
```

`getModerationCapability` deliberately mirrors `getCapabilities`'s exclusive
either/or shape rather than inventing a second convention for the same job.

**Stage D — reach**

```
createStoa({title})         -> BUILT: see the `stoa-membership` capability
joinStoa({stoa, genesis})   -> BUILT: see the `stoa-membership` capability
```

~~`joinStoa({address})`~~ — **this section specified the wrong input, twice, and
the implementation is the correct one.** An address is a one-way hash: it verifies
a record somebody hands over and cannot reconstruct one, so a join given only an
address has nothing to verify and would leave the peer holding a Stoa whose record
it does not have — which `moderation-resolution` requires before a reader may
decide whether any moderation of that Stoa's content binds. The departure is
argued in the `stoa-lifecycle` change's `design.md`; do not reinstate the
single-argument form here.

The reply carries the founding title, which is what lets the view show what is
being joined before it is joined (§4.8) and which a bare `{"ok":true}` could not
support. Note **founding**, not resolved: `getStoa` above is the resolution call
and is not built, so a join reply names the founding value and says so.

**Methods deliberately NOT proposed**, each with its reason, because a list of
what was declined is the part that stops the API growing by accident:

- ~~**`vote`**~~ — **now proposed and contracted** (§9.2's scope decision; the
  `content-authoring` spec). The objection stands as written and the spec bounds
  what the method may claim rather than claiming an effect it does not have.
- **`getPost`**, a single-post read — every screen that shows a post shows it
  inside a thread or a feed, and §2.4 makes per-item calls the expensive shape.
  Add it when a screen exists that genuinely wants one post.
- **`search`** — no index, no design, and a search that scans the op log per
  keystroke is the hot-loop IPC §2.4 warns about. Not rejected, just not
  designed.
- **`getOp`**, a raw-op read — it would let a view render something no resolver
  approved, which is exactly the discipline the resolvers exist to impose.
- **a delete method** — there is none, structurally. §5.7 keeps history and §6.1
  bounds moderation to rendering; an API method called `delete` would promise a
  removal the protocol cannot perform.

#### 7. What the resolvers do not provide, and this is a finding about core

Writing the calls above found four gaps. Each is core work, not view work, and
each is named here so that whoever builds the projection knows the shape it is
being asked for rather than discovering it from a stalled view.

- **Nothing resolves Stoa metadata.** §5.7 records this — the op accumulates and
  nothing reads it — and `getStoa` is the call that needs it. `log.rs` already
  states the shape: it is "the moderation resolver with a different subject",
  reading by `Address` rather than by target, and reusing the authority check
  unchanged. `iter_target` cannot serve it, because a metadata op's
  `Entry::target()` is `None` by design.
- ~~**There is no thread read.**~~ **Contracted by the `thread-read` spec**, and
  the shape it asks the projection for is not the one this bullet assumed. The
  query is *not* "the posts whose `thread` field is T": that field is the
  author's own claim, and an inbound op may name a thread its parent does not
  belong to. Membership is derived by following parents to a root, so what the
  projection must serve efficiently is a lookup by **parent**, not by thread.
  Whoever builds it should read that spec's membership requirement before
  choosing an index.
- **There is no reply count and no most-recent-reply.** §7.2's `active` ordering
  needs the second, and both are folds over ops the resolvers do not perform.
  Both must also be **of non-hidden replies**, which makes them folds over
  moderation-resolved state rather than over raw ops — a detail easy to get
  wrong once and hard to notice, because a count that includes hidden replies
  looks entirely plausible.
- **`current_version` is per-post and a feed is a list.** Calling it once per
  row is correct and is what the in-memory log makes cheap; over a SQLite
  projection it is a query per row. Nothing is broken today and nothing should
  be optimised speculatively — recorded because it is the shape that decides
  whether the projection stores resolved current versions or resolves on read,
  and that is a schema decision, not a later tuning one.

#### 8. What could not be decided here, and what would decide it

- **Whether `listThreads` needs an `unread` concept.** Every forum has one and
  it needs per-user local state that is not an op and never crosses the wire.
  It is not hard; it is that nothing in this design has yet needed peer-local,
  never-published state that is not a projection of ops, and inventing the
  first instance of that as a feed field is how it gets designed badly. What
  would decide it: a first user reading a Stoa with more than a screenful of
  threads. Until then the question is theoretical.
- ~~**Whether a thread view paginates by reply order or by reply tree.**~~
  **Settled by the `thread-read` spec, in the direction that keeps the question
  open where it matters.** The read returns a flat page in which each item names
  its parent, so a view computes the nesting it wants and core reports no depth.
  The reasoning — including that the design bundle's screen 05 nests by an
  indent over a linear sequence, which this shape serves — is in that change's
  `proposal.md`.
- **What a feed ordering is allowed to be called while no Lamport value
  arrives.** This is the sharpest unresolved thing in the section, and the first
  draft of it was wrong in a way worth recording. §7.2 defines `new` as "Lamport
  order descending" and `active` by the Lamport timestamp of the most recent
  non-hidden reply — so **both** of v1's two orderings are defined in terms of a
  value §13 says does not reach us. Today each would fall back to ascending op
  id, which is a hash and carries no recency whatever.

  The first draft of this bullet proposed "ship `new` only until Lamport values
  arrive", on the assumption that only `active` was affected. It is not: `new`
  is affected identically, and a feed labelled "new" ordered by hash is the same
  lie in a shorter word. Recorded rather than silently fixed, because the
  mistake is the one this whole degraded-ordering situation invites — reading
  "convergent" as "roughly chronological".

  The honest options are therefore: ship one ordering and name it for what it
  actually is rather than for what §7.2 intends it to become; or ship §7.2's
  two names and have the interface state that ordering is currently degraded.
  **This plan does not choose**, and it is a genuine open question rather than a
  deferred detail — a first-run forum whose ordering is arbitrary is a different
  product from one whose ordering is chronological.

  **What would decide it has changed, and the earlier answer was wrong.** This
  bullet used to say the question was removed by §13's upstream gap closing,
  and that nobody should build machinery around it meanwhile. §13 now
  establishes that recency is **ours** — an author-asserted `createdAt` in the
  signed preimage, cheap and needing nothing from upstream. So the question is
  not waiting on anyone: **it is decided by whether we add that field**, and
  the advice to sit still was advice to wait for something that was never
  coming. The scope of "elaborate machinery" is narrower than it looked — a
  timestamp field is not elaborate.

- **Whether `listThreads`'s `replyCount` is worth its cost before then.** It is
  a fold over moderation-resolved replies per row, and under the degraded order
  the "most recent reply" it sits beside is not meaningfully recent. The count
  itself is honest — a thread has a number of visible replies whatever the order
  — so this is a question about the pair, not about the field. What would decide
  it: the same gap closing, or a measurement showing the fold is cheap enough
  that the question does not arise.

#### 9. Why this section shipped without a spec delta

> **Partly superseded.** Stage B's publish half now has one — the
> `content-authoring` spec — written the way this section says a delta should be:
> alongside the thing it contracts. The argument below is why *this section* was
> not itself a delta, and it still holds for everything here that remains
> unbuilt: `revisePost`, `getPostHistory`, the moderation calls, and the two
> orderings, which are still an open question rather than a requirement. **The
> thread read now has a delta too** — the `thread-read` spec. **The feed read
> still has none**, and that is the named debt below rather than an oversight.


`.claude/agents/README.md` puts PLAN.md and the specs in different jobs: PLAN.md
holds **what is not built yet** and the reasoning for it; a spec is a
**behaviour contract** for something that exists, and reasoning never goes in
one. This section is entirely the first kind. Nothing above is implemented,
every method named is a proposal, and the two orderings are an open question
rather than a requirement.

A delta written now would have to invent scenarios for behaviour nobody has
built, which is exactly the failure that README warns against — *"behaviour that
does not exist yet cannot be covered"* — and it would freeze method shapes whose
whole purpose here is to be argued with before anyone commits to them. The
`posting-capability` spec is the model of when a delta *is* right: it was
written alongside a probe that shipped.

**So the specs come per stage, with the change that builds it.** Stage A is
plausibly two capabilities rather than one — a feed contract and a thread
contract — and which it is should be decided by whoever writes it, against the
projection that actually exists, not here.

One thing that will need saying in whichever spec lands first, recorded so it is
not lost: **§2.5's paginated shape has no instance yet.** The first paginated
method sets the precedent for every later one, so its spec is the one that
should pin the envelope — `items`, `page`, `hasMore` — rather than each
subsequent spec restating it and slowly disagreeing.

##### The feed read is built and still has no contract — a named debt, not an oversight

**This is the one place the "specs come per stage" rule has already been
overtaken by the code.** `feed::list_threads` and
`wire::list_threads_from_request` are on `main` and are the largest behaviour
block the integration suite pins, and `openspec/specs/` carries **no feed
requirement at all** — `grep -rli "feed\|list_threads"` over that directory
returns nothing. The read shipped in the feed-screen change, which merged before
this repo adopted OpenSpec, so no delta was skipped; there was no flow to skip
one in.

What is consequently unspecified is not a detail. None of these is a promoted
requirement, and each is behaviour a reader can rely on today:

- a feed lists thread heads and never replies as their own rows
- pages tile with no gap and no repeat, and a page past the end is empty rather
  than a panic or an error
- a thread whose root post is hidden is omitted by default and is included,
  flagged, under the explicit parameter — the distinction this section's
  "Hidden threads are omitted by default" paragraph draws between a hidden root
  and a hidden reply
- a post whose signature does not verify is refused by the reader rather than
  rendered
- a body and its attachment CIDs are sanitised on the way out (`feed.rs:254-255`).
  The nearest promoted text points the other way: `stoa-metadata`'s "Display text
  is preserved rather than sanitised in the data layer" governs what is *stored*,
  and says nothing about what a read returns
- the JSON envelope reports the page that was asked for and whether more follows

**Why this was not closed by the change that found it.** It surfaced in a
spec-test review of the `core-e2e` integration target — a piece declaring
`skip_specs: true` that adds no behaviour and only tests. Writing the feed
contract there would have put a behaviour contract for code that merged in other
pieces into a change whose reviewers never read those pieces, and promoted it
past the review each of those pieces actually had. The contract is owed by
whoever next touches the feed read, which is also the only agent positioned to
write it against the projection that exists rather than against the tests that
happen to pin it.

**The cost of leaving it, stated so it is not rediscovered.** Until this lands,
an integration test in `dialectica-core/tests/end_to_end.rs` is the only written
statement of what the feed does — so a change that makes the feed list replies as
rows contradicts no requirement, passes `openspec validate --strict`, and is
objected to only by a test whose own header says it is organised by boundary
rather than by capability. A reviewer then has no contract to weigh the change
against, which is the failure a spec exists to prevent.

**Store lifecycle is the same shape and is already in hand.** `op-log` as
promoted specifies nothing file-backed, so the suite also pins store creation,
layout versioning, mislabelling and restart with no promoted requirement behind
them. Unlike the feed, that gap has an owner in flight: the `sqlite-projection`
change's `op-log` delta adds "A persistent log survives the process that wrote
it", "A persistent log declares the layout it was written with" and "A persistent
log verifies the layout its declared version promises", and extends the
every-read requirement to separate a storage failure from emptiness. Read that
delta before writing anything here. **One piece of it is genuinely absent even
there**: that opening a path holding no store *creates* it rather than refusing
it — the suite's `a_missing_store_file_is_created_rather_than_refused` — which no
requirement in that delta states. It belongs to `sqlite-projection`, not to a
later change.

**A third, much smaller debt of the same class**, found while answering whether
an integration test may construct a hostile keystore. `posting-capability`'s
requirement "The reason names the fix" enumerates six reasons the probe must
distinguish, and carries a scenario for five of them. **"The keystore's directory
is writable by others" has no scenario** — the phrase appears only in the
requirement text. The *behaviour* is contracted: `keystore`'s "A keystore in a
directory others can write to is refused" specifies the refusal and its
distinguishability, with both a refusing and an accepting scenario. What is
unpinned is that the **probe** surfaces that state as its own reason, which is
`posting-capability`'s claim rather than `keystore`'s. One scenario on an existing
promoted requirement closes it, and it belongs to whoever next touches the probe.

On the question that surfaced it: all three of the enumeration's file-level
states — permissions too open, a directory writable by others, an unreadable or
malformed keystore — are legitimately constructible by a test, because `keystore`
specifies each as a scenario whose WHEN clause *is* that construction. A
requirement that specifies refusing a hostile file cannot also forbid making one
to check the refusal; that scenario would be untestable, which is the defect
`.claude/agents/README.md` names as this repo's most common — "Never write a
scenario that cannot be tested".

### 9.2 The MVP, as scoped by the owner

**The owner has directed a rush to a working MVP.** This section exists so the
staging is visible in one place rather than inferred from ten sections that each
mention a part of it. It is a **record of a scope decision**, not an argument:
the costs below were named and accepted.

**In the MVP:**

1. Create an identity
2. Create a Stoa
3. Post
4. Reply to a post
5. Upvote / downvote
6. Share a Stoa — copy its address. **Built** (the address is what creation
   returns); sharing it *from the UI* is not
7. ~~Join a Stoa by address~~ **Join a Stoa, given its address and its genesis
   record. Built** — see the `stoa-membership` capability. An address alone is
   not joinable; §4.8 Phase 1 records why the original wording was wrong
8. ~~**Receive ops from other peers**, over delivery's reliable channel~~ —
   **specified: the `op-transport` spec.** Publishing and receiving both, with the
   receive-side validation boundary.
9. **View a feed; view a thread**
10. **Persistence on disk** of Stoas, identities and messages

All of it over **delivery's reliable channel** (§4.1). **No Logos Storage** —
§4.6's attachments-by-CID are out, so a post in the MVP is text.

**Out of the MVP:** moderation (§6), per-Stoa identity (§5.2), Logos Storage
(§4.6).

Nothing on either list is deleted or withdrawn. `moderation.rs` and its specs are
built, tested and merged and they stay; §4.6 stands as the attachment design for
when attachments ship.

~~`derive_stoa_key` is built and simply is not called~~ — **this was false when
written.** The adapter called it twice, and a Stoa's `creator` was derived under a
synthetic domain rather than being the key its creator signs with, so a peer
creating a Stoa was its sole moderator under a key it would never sign with. The
stoa-lifecycle change made `identity_key` the root secret directly, which is what
this paragraph had claimed all along. The derivation stays built and tested,
because it is still the destination for per-Stoa identity.

#### Follow-ups the publish path named, not yet built

**"A publish requires a usable identity and says so when there is none" moves to
`posting-capability`.** Decided 2026-09-13, to be done as its own change.

The requirement's subject is the *absence* of an identity, and discovering that
absence means reaching a keystore — which `dialectica-core` deliberately cannot
do. So the requirement is undischarged in core, and the two alternatives to
moving it both cost something real:

- **Narrowing it to the wire shape** would contract the wording of a refusal only
  the adapter can raise, and require nothing of the trigger — a module built with
  no guard would satisfy it. A weaker contract bought with a tickable box.
- **Taking a fallible key-supplier in core** would discharge it, but trades a
  structural property for a tested one: `authoring` currently *cannot* create key
  material, because it never holds anything that could. A closure replaces
  "cannot" with "does not, and here is a test". That may still be right on its own
  merits, and should be judged there rather than as the price of testability.

`posting-capability` already owns eight requirements on this same question, so the
generality is demonstrated. The move is `ADDED` there and `REMOVED` here, verbatim,
with Reason and Migration in one change — which is why it does not ride inside the
change that found it.

Both came out of review and are recorded here rather than in a findings file,
which is deleted at merge.

**The publish prologue/tail wants reshaping into one place.** `publish_post`,
`publish_reply` and `publish_vote` each carry the same prologue — parse, validate,
open the keystore, open the store — and the same tail. Three copies of one guard
is the point at which a guard should become a data structure, and two things make
it more than tidiness:

- It is a **precondition of `publish_moderation`**, where a missed guard is an
  authorisation defect rather than a wrong reply.
- The adapter runs a **64 MiB Argon2id unlock before any validation**, so a
  request that will be refused for a malformed Stoa pays for a full key
  derivation first. Validate, then unlock — one reshape fixes both.

Deliberately **not** done inside the change that revealed it: make the change
easy, then make the easy change. A diff that reshapes three handlers and adds a
publish path cannot be reviewed for either.

**A panicking delivery sink must not report a published op as failed.** `deliver`
runs inside `guarded`, so a sink that panics yields `{"error":…}` with no `opId`
for an op that **is already in the log** — against "a publish SHALL NOT be
reported as having failed on the strength of a delivery outcome".

The owner's decision: **catch it and report the publish as successful.** The op is
published and the requirement says so; delivery is the transport's concern.

**The synchronous reply was never the right place to learn about delivery, and the
delivery contract already says so.** ~~Now contracted~~ — `content-authoring`'s
"Publishing signs, appends, and hands off — in that order" requires that the reply
carry no delivery outcome at all, and that the capability not require the interface
to delivery to be able to express one. What remains open is the obligation below,
not the decision.

`delivery_module.lidl` carries three channel events — `channelMessageSent`,
`channelMessageError` and `messagePropagated` — so the outcome arrives
**asynchronously, after the publish call has returned**. A return value could not
carry it even if we wanted it to.

That also makes the return value a *worse* signal than the events, not merely a
missing one: a sink that accepts an op tells you the transport took it, which is
`channelMessageSent` and says nothing about whether any peer received it.
`messagePropagated` is the fact a user cares about. Publishing and delivering are
two events at two times, and this decision stops the API pretending they are one.

**The obligation lands on `op-transport`**: an op that reaches
`channelMessageError`, or that never reaches `messagePropagated` within some
bound, has to become visible somewhere. Without that this decision converts a loud
failure into a silent one.

~~The bound, and what a view shows for an op in flight versus one that never
propagated, are that capability's to specify~~ — **the obligation is now stated
there rather than only here.** The `op-transport` spec's "A successful publish is
a statement about the local log and nothing more" contracts what a publish may
claim, and names the three things still owed: the bound, what a peer records for
an op in flight, and what it records for one that never propagated. **Still not
built** — meeting it needs state outliving the publish call and a clock, which is
a component rather than a branch.

**`docs/UI-BRIEF.md` carries the half of the rendering obligation that is true
today**, as the obligation titled *"A successful publish means 'saved here', not
'posted'"*: a successful publish must not
be rendered as sent, delivered or seen, and no in-flight state is to be designed
because no call produces the signal one would wait on. What the brief still needs
when the three are answered is the *positive* half — what a view shows for an op
in flight versus one that never propagated — which is additive to the prohibition
rather than a replacement for it. The prohibition did not wait on the three,
because a brief silent about it is one designed against by someone free to render
success as "posted".

Moving `deliver` outside `guarded` was rejected — PHASE0-FINDINGS §3 measured what
an unguarded panic costs (the module aborts, the caller waits out a 20-second
timeout, every later call reports `MODULE_NOT_LOADED`), which is a worse answer
than an unreported delivery failure.

#### How this sits against §9.1's stages

**§9.1's Stage A/B/C/D ordering is not renumbered or restructured by this
section** — it is the dependency analysis, and it still holds. Cross-reference
only:

- **Stage A (read a Stoa)** and **Stage B (compose)** are both in the MVP, and
  the MVP therefore spans two of §9.1's stages rather than sitting inside one.
  §9.1's argument for why B needs A is what makes that ordering safe to collapse;
  its point was never that the stages must ship separately, only that no stage
  needs a later one.
- **Stage C (moderate) is out**, which is the §6 exclusion seen from the staging
  side.
- **Stage D (reach another Stoa)** is **in**, via items 6 and 7 — sharing an
  address, and joining with an address **and** the genesis record it names (§4.8
  Phase 1; "joining by address" was the wrong shape). This is the one place the
  MVP scope departs
  from §9.1's ordering, and it is a deliberate reordering rather than an
  oversight: D before C. §9.1 permits it, since D depends on A and on nothing
  later.

**Votes are the one item that contradicts a §9.1 decision, and it is worth
naming rather than reconciling quietly.** §9.1 deliberately does not stage votes,
on the reasoning that §7.2 rule 2 ships no score, so a vote button publishes an
op that changes nothing a reader sees — *"a control with no visible effect
teaches users the app is broken."* Item 5 puts the control in the MVP anyway.
**That argument is not refuted by this scope decision and should be read
alongside it**: whoever builds the vote control inherits the problem §9.1
identified, and the honest options are a visible per-post tally that is not a
ranking, or a control whose effect the copy does not overstate. §7.4 settles the
control's shape; it does not settle this.

**The core half is now settled and the UI half is not.** The
`content-authoring` spec contracts publishing a vote at exactly the honest width —
the op is signed, appended and readable by its op id and by its target, and the
reply carries an op id and nothing describing an effect. So core makes no claim a
reader could be misled by. **What remains is entirely an interface obligation**:
a vote control must not imply a ranking, and this is the open item, not a
contracted one. It belongs on §11.1's rendering-obligations list when that lands.

---

## 10. CI

Radicle's `ci.yml` is the template: `lint` / `qml` / `rust` / `build` /
`release`, plus a matrixed sitometres e2e workflow.

**Built — `.github/workflows/ci.yml` is the artefact, and it carries its own
reasoning per job.** What follows is what building it changed about this
section, since three of the claims below turned out to be wrong.

**Pins:**

- **`lgs`: crates.io `0.3.1`, not the unreleased rev.** This section previously
  accepted a force-push risk that dialectica does not have to take. Radicle
  pins an unreleased rev for two reasons, and neither applies here.

  `setup --inspector` is the first, and dialectica's CI never calls `setup`.

  The second is `build --print-output`, absent from v0.3.1 — and the reason it
  does not matter is worth stating precisely, because the obvious version is
  wrong. It is *not* that `LOGOS_SCAFFOLD_PRINT_OUTPUT` substitutes for the
  flag: `print_output_enabled()` is only ever consulted by `run_logged`, and
  **`lgs basecamp build` never calls it.** `run_build_portable_nix` runs
  `cmd.output()`, capturing both streams into memory, and on failure surfaces
  stderr through its own `bail!`. So on the build path the flag and the env var
  are equally inert — which is exactly why the unreleased rev buys nothing, and
  why the flag exists only for `install` at that tag. Failure output still
  reaches the log; it arrives by the bail, not by streaming.

  Pin the release, and the risk is retired rather than documented. (The env var
  is set anyway, correctly, for an `install` job that does not exist yet.)
- **A Nix version floor exists, and it is invisible until it bites.** Nix ≤2.24
  refuses to evaluate a lock file containing a relative path input **before**
  applying `--override-input`, so `dialectica-ui`'s `path:../dialectica` fails
  to build while the core — which has no such input — builds fine. The error
  names the input, which makes it read like a missing override when the
  override is present and correct. Bisected: 2.22.4 and 2.24.14 fail, 2.26.4
  and later pass. CI pins the Nix version explicitly rather than inheriting
  whatever the installer action happens to bundle. Radicle never hit this only
  because its build job uses a different installer.

**Anti-false-green, throughout.** Radicle's CI is built around the observation
that a green gate which cannot see the thing it claims to check is worse than no
gate. Copy the habits, not just the jobs: assert test *counts* against spec
counts, assert packaged manifests kept their entry points, assert dev and
portable variants were not transposed, and verify a new test fails before it
passes.

That principle earned its place immediately, and against the template itself:

- **Radicle's `qmllint` gate cannot fire, and ours inherited it.** It greps for
  `^.*:[0-9]+:[0-9]+: (error|Error)`, which expects `file:line:col: error:` —
  but Qt6 prints severity *first*: `Warning: file:line:col: ...`. Two separate
  facts, tested against the real binary at Qt 6.10.3 and worth keeping apart:

  - **Severity-first formatting kills the regex.** An unknown type and an
    unqualified access both produce zero matches against it.
  - **The exit code is the signal that works.** A hard syntax error exits 255,
    where the same regex still matches nothing.

  Since the step ran the linter under `|| true`, the discarded exit code was
  the only thing that could have caught the second case, and the regex was the
  only gate. **Worth telling the radicle maintainers**, since their copy is
  identical. Gate on the exit code; silence a genuine non-defect with qmllint's
  own `--<category> disable`, which names what it ignores, rather than a regex
  that ignores everything. Version named because output formats are a moving
  target — re-test rather than assume.
- **A ported assertion can be wrong for the port.** Radicle asserts a truthy
  `main` on the ui_qml packaged manifest. Dialectica's is `{}` — a QML-only
  module has no binary entry point — so the assertion would have failed a
  correct build. Check by shape, and re-derive every inherited assertion
  against what this repo actually produces.
- **Assert against a derived number, never a literal.** The test-count check
  compares cargo's result against a count of `#[test]` attributes derived from
  the Rust tree, so it cannot rot. A hardcoded floor that nothing keeps in sync
  is itself a false green. Read the workflow for which paths it walks — naming
  them here would be a second copy that drifts.

**Deliberately not built, each with its re-entry condition** (recorded at the
foot of the workflow too):

- **No e2e job.** The view is four buttons with no specs, and a matrixed job
  over zero specs cannot fail — the exact thing this section forbids. It
  arrives with the specs, and **sitometres is pinned to its latest release**
  when it does: radicle pins a git commit only to work around a probe bug in
  published 0.1.0, which is a workaround rather than a pattern to copy.
- **No `doctor` job**, despite §11 saying to run `doctor`. Two pins WARN on
  every run by design (PHASE0-FINDINGS §8), so the job would be permanently
  red, and a permanently red gate trains people to ignore it. Run it by hand;
  add the job when the pins are back on a matched set.
- **No `install`/`launch` job**, and no drift check on the checked-in delivery
  contract.

**The `qml` job is the weakest gate that exists**, and it is labelled as such in
the workflow rather than left to be discovered: it proves `Main.qml` parses and
nothing about `call()`'s JSON parsing, its error branch, or the missing-bridge
path. That logic is genuinely testable and currently untested.

---

## 11. Traps, collected

Each of these cost someone a debugging session. All are structural — they bite
at build or run time, not review time.

- **A public constructor with parameters** — even all-defaulted — is scanned by
  the universal-interface generator as a zero-arg RPC method named after the
  class, and miscompiles. One parameterless constructor; DI through a private
  setter.
- **Sibling flake refs must be one line.** Scaffold's override parser is
  line-based and will not see a multi-line `inputs.x = { url = …; }`.
- **`follows` wiring is mandatory.** A sub-flake pulling a module that itself
  depends on `logos-module-builder` must add
  `inputs.<dep>.inputs.logos-module-builder.follows = "logos-module-builder"`,
  or `flake.lock` gets two builder nodes, the stale one silently wins under
  `--override-input`, and the build fails with `no 'main' field in
  metadata.json`.
- **Set `runtime_dir` to the session's real one** (e.g. `/run/user/1000`). The
  in-profile default overflows the 108-byte `sun_path` cap and **every module
  segfaults** at "Failed to register module for remote access".
- **`lgs basecamp setup`, `modules` *and* `install` each strip every comment
  from `scaffold.toml`** — not `setup` alone. Rediscovered the expensive way:
  the comments were restored, then vanished again on the next unrelated verb.
  Run `git diff scaffold.toml` after **any** `lgs basecamp` verb.
- **`lgs basecamp install` does not install a module's declared
  `dependencies`.** It builds the `[modules.*]` project sources and never reads
  the `dependencies` array in `metadata.json`. `lgs basecamp modules` is the
  verb that captures runtime dependencies; run it first, then `install`. Skip it
  and the module fails to load with `Cannot resolve dependencies for: <name>`,
  which presents as a launcher tile that does nothing when clicked — while the
  build stays green and `modules --show` lists the dependency it never
  installed.
- **`pgrep basecamp` finds nothing while Basecamp is running.** The launcher is
  `.LogosBasecamp.elf` and each module is a separate `.logos_host.elf`, both
  under the dynamic loader. Read the PID from `<profile>/launch.state` instead
  (`lgs basecamp paths <profile>` locates it). Requested as a first-class verb
  in logos-co/scaffold#268.
- **Run `lgs basecamp doctor` before believing a green build.** It catches pin
  drift and split basecamp/lgpm pin sets that no build failure surfaces. Expect
  two WARNs: the basecamp/lgpm split and the delivery pin are both deliberate,
  and PHASE0-FINDINGS §8 says why. That they are decided rather than drift is
  also why there is no `doctor` CI job (§10) — an always-red gate trains people
  to ignore it.
- **The UI icon must be a 256×256 PNG**, and the UI module must declare core in
  `dependencies` at a **matching version**.
- **Pin `logos-module-builder` ≥ 0.2.5** — earlier builders deliver empty binary
  event payloads. Assert non-empty payloads in a test.
- **A panic in a dispatch handler aborts the module process.** Measured, not
  inferred: `failed to initiate panic, error 5`, SIGABRT, and every later call
  gets `MODULE_NOT_LOADED` — the caller having first waited out a 20s timeout
  that names nothing (PHASE0-FINDINGS §3). Not the mutex poisoning §2.3
  predicted; the process dies before a poisoned lock can be met. The SDK has no
  guard, so **no handler may unwind** — ours is what stands between those two
  outcomes.
- **`recv()` on an event subscription may block forever** on an older SDK rev
  that lacks subscription status — a dead provider hangs the listener thread
  permanently. Check what the builder's pin delivers before relying on a
  timeout.
- **Handle `RET_STALE_WARN` (3)** from the delivery C ABI: a non-terminal
  "still running" tick every ~5s, always followed by a terminal OK/ERR. Ignoring
  it double-counts completions.
- **`createNode` exactly once per context.** The delivery node is a singleton
  per Logos Core instance; `stop()` kills traffic for every module using it.
  Contracted in the `op-transport` spec; kept here because it presents as a
  runtime failure in someone else's module.
- **`messageReceived`'s timestamp is nanoseconds**; every other event is
  ISO-8601 (delivery bug #26).
- **`messageReceived` fires for your own messages; `channelMessageReceived` does
  not** — own sends come back as `channelMessageSent`. The consequence is
  contracted in the `op-transport` spec ("A peer's own published op is not
  received back as an arrival"); the asymmetry itself stays here, because it is
  what makes a missing-own-post bug look like a storage bug.

---

## 12. Where the source material is

This plan was written from five investigations of local checkouts. When a claim
here needs re-checking, these are where it came from — all absolute paths on the
development machine, none of them vendored into this repo.

| What | Where |
|---|---|
| **The structural template** — monorepo, core+UI, Rust FFI, CI, `CLAUDE.md` conventions | `/home/fryorcraken/src/rad/radicle-logos-module` |
| Docs source for docs.logos.co (module building, delivery API, LEZ, LGX format) | `/home/fryorcraken/src/logos-co/logos-docs/docs/` |
| Delivery module — **read at tag `v0.2.1`**, not the working tree | `/home/fryorcraken/src/logos-co/logos-delivery-module` |
| Waku/Nim internals — sharding, retention, message size, RLN config | `/home/fryorcraken/src/logos-messaging/logos-delivery` |
| SDS spec (LIP-109) and LIP-23 content topics | `/home/fryorcraken/src/logos-co/logos-lips/docs/` |
| LEZ private accounts — nullifiers, ML-KEM, commitment set | `/home/fryorcraken/src/logos-blockchain/logos-execution-zone` |
| λAccount / VLAD identity roadmap and FURPS | `/home/fryorcraken/src/logos-co/roadmap/content/anoncomms/` |
| A real delivery consumer (older API, still instructive) | `/home/fryorcraken/src/logos-co/logos-delivery-demo` |
| `logos-scaffold` source, docs and bundled skills | `/home/fryorcraken/src/logos-co/logos-scaffold` |
| OpChan — the nearest kin forum, read for Appendix A. **No local checkout**; clone from GitHub | `logos-messaging/OpChan` |
| λ-Prize LP-0005 / LP-0016 / LP-0017 — **submission write-ups only, not code.** The solution repos are not cloned here, so every claim about them is the builders' self-assessment | `/home/fryorcraken/src/logos-co/lambda-prize/` |
| The JS SDK reliable-channels tutorial — informal prose, and §4.3's clearest statement of what a channel id is | `/home/fryorcraken/src/logos-messaging/docs.waku.org/` |
| Spasm — a signer-agnostic social protocol, read for §5.5's second never-verified-credential instance. **The code is the spec**: versioning lives in format strings (`spasmid01`, `SpasmEventV2`) and the normative reference for its hashing rule is its README, so read `src.ts/`, not the docs site | `/home/fryorcraken/src/spasm-network/spasm.js` |

Two of these are **stale working trees** and will mislead if read directly:
`logos-delivery-module` sits on a pre-channels branch, and
`logos-messaging/logos-delivery` predates reliable channels entirely. Use
`git show v0.2.1:<path>` for the delivery API.

**The one exception, and reach for it first: the delivery module's interface is
vendored here.** `dialectica/contracts/delivery_module.lidl` is the converted
v0.2.1 contract the build actually compiles against — every method, every event,
every docstring. For "does delivery expose X?" that file is both the nearest and
the most authoritative answer, and §3.2 records the correction that followed from
answering it from a report instead.

`logos-module-builder` and `logos-rust-sdk` are not local checkouts — they are
flake inputs. The Rust module examples and doctests live inside the
`logos-rust-sdk` source, under `tests/` and `doctests/`.

**Clone the SDK from GitHub rather than reading it out of `/nix/store`.** The
store copy is a bare source snapshot with no git history, and it is whatever
revision the builder pinned — which has lagged HEAD substantially. Reading the
store tells you what you will *link against*; reading GitHub tells you what the
documentation describes. Both are worth knowing, and they are not the same
thing (§2.3).

## 13. Open questions

- ~~Does the `dependency_overrides` LIDL bridge to `delivery_module` work?~~
  **Answered: yes, with a conversion step §3.2 did not anticipate.** The header
  form does not work from Rust; a committed `.lidl` does, and the bridge has
  carried a live call into delivery's own implementation
  (PHASE0-FINDINGS §1, §6).
- **Should every root identity be derived?** Raised by the owner and **recorded
  unanswered, for the owner's own review** of identity and derivation: *"I would
  expect us to have all root identities using derivation."* Today §5.1's root
  secret is generated (`SecretKey::generate`) and only the per-Stoa key is derived
  from it — under **two** schemes now, not one: `derive_stoa_key` takes the root and
  the Stoa, and `derive_stoa_key_at_path` takes a chosen derivation path as a third
  input, under a bumped salt so the two cannot silently reproduce each other. The
  path-taking one is what a kept identity uses (see the `identity-onboarding`
  spec); the pathless one has no production caller left. The
  question is whether a root should itself be a derived child of something
  higher, and what that something is. **Do not answer it here**; it touches §5.1,
  §5.6's keystore and the LEZ key-tree path in the next entry, and the owner has
  reserved it.
- **The LEZ forum-key branch should say `dialectica`, not `Forum`** — an upstream
  ask on LEZ rather than a dialectica change, recorded so it is not re-derived.
  The LEZ key tree separates branches by root HMAC domain —
  `/LEE-Keys/v1/Master/Public` and `/LEE-Keys/v1/Master/Private`. A prior
  investigation proposed `/LEE-Keys/v1/Master/Forum` as a third branch for forum
  keys. **The owner does not want that path**: they want `dialectica` in it. The
  preference is recorded; the shape of the ask, and whether LEZ wants an
  app-named branch at all, is upstream's to answer. Nothing in dialectica depends
  on it today — §5.1's construction is our own HKDF, not a LEZ key-tree path.
- What is the actual participant ceiling for one SDS channel? Unmeasured, and
  the answer sets when §4.5 stops being optional. With SDS now the *only* sync
  layer, there is no CRDT fallback if a channel degrades.
- How far back does SDS-Repair realistically reach in a live Stoa? That number
  decides how urgent snapshots are — **and it is less load-bearing than it
  looks**, because §13 establishes that thread completeness is computable from
  the parent pointers we already hold. A thread-scoped, demand-driven repair
  is ours to build whatever SDS-R reaches.
- **How is the moderator set ordered when two moderators edit it
  concurrently?** The one genuine merge question in the design (§5.7), and it
  does not arise while the creator is the sole moderator — so it is answered
  alongside mutable moderation, not before.
- **Should a disagreeing reply count as a quality signal?** Written up in §7.4 —
  replying to disagree is implicitly "worth my time", it is costly enough to
  resist minting, and it needs no new op. Blocked on avoiding sentiment
  analysis, and on whether rewarding replies rewards argument volume. **Weigh it
  against bridging**, which addresses the same gap with measured results but
  needs a dense rating matrix and a sybil-resistance layer this forum lacks.
- **Does bridging-based ranking work without platform-supplied sybil
  resistance?** §7.4 records it as the evidenced answer to surfacing good
  arguments a reader disagrees with, and records why it is not v1: published
  analyses show fewer than ten placed ratings can push low-quality items over
  threshold, and the deployed instance relies on out-of-band identity checks §7
  says dialectica has none of. **§7.3's vouch is the candidate replacement** —
  whether it is sufficient is the open question, and it cannot be answered until
  vouch data exists.
- ~~**When the `policy` field lands.**~~ **Answered: it is in the genesis
  record now**, with `open` as its only accepted value — `dialectica-core`'s
  `stoa::Policy`. The reasoning stands as written and is why it landed early: a
  genesis record is immutable and address-determining, so adding the field later
  would have changed the address of every Stoa already created.

  Two decisions made while implementing it, both worth knowing before adding the
  second variant. An unknown policy discriminant is **refused, never defaulted
  to `open`** — defaulting is how a token-gated Stoa silently becomes
  world-postable on an older client, and refusing to display a Stoa is the
  recoverable direction. And the encoding carries a **version discriminant**, so
  a record from a newer client fails as "unknown version" rather than as a
  misparse.

- ~~**May a Stoa metadata op change `policy` as well as the display fields?**~~
  **Answered: no, not in that op.** §5.7 raised it; `op.rs`'s `StoaMetadata`
  kind carries a title and a description and no policy. The argument and the
  alternatives are in the `stoa-metadata-op` change's `design.md` — do not
  restate them here.

  What reopens it, in one line each, because these are the conditions and not
  the reasoning: `Policy` gains a second accepted variant; metadata resolution
  is actually built on `arrival::cmp_ops`, so "the current policy" is a
  determinable thing rather than a phrase; and a policy fallback rule is
  specified separately from the display one, as fail-closed. Adding it then
  costs an unused op-kind discriminant, not a wire-format version and not any
  Stoa's address.

- **Is a threshold the right shape for proof-of-holding as a relevance signal,
  and what threshold?** §7.2 argues a threshold rather than a graded count,
  because LP-0005 proves "balance ≥ N" without revealing the balance. But the
  choice of N is a policy decision per Stoa, and a badly-chosen N makes the
  signal either universal or empty. Likely a `policy` field question (above).
- **Does adopting SDS-R change anything about what a peer discloses?** §5.2
  takes SDS-R as wanted, and §4.1 records that it exposes `sender_id` in
  `HistoryEntry` — so a repair-enabled peer sees sender ids attached to other
  people's message history, and answering repairs is itself observable
  behaviour. Under per-Stoa identity this leaks nothing the channel does not
  already leak, which is why it is a question rather than a blocker; it becomes
  one again for any future narrower scope.
- **Can a claim be presented under two pseudonyms without linking them, with
  the primitives that actually exist?** §5.5 requires it and §7.2 rule 3 needs
  it, but **LP-0005 does not claim it** — its property is threshold privacy, and
  its `3010` binding requires the signer to be the attested account, which
  points the other way. RLN's per-epoch nullifiers are the better candidate and
  are also unverified here. This is the load-bearing unknown in the claims
  design: settle it before any credential carries weight in ranking or gating.
- ~~**Are votes an op in v1 at all?**~~ **Answered: yes, and they are read —
  §7.2 rule 2's `top` counts them.** The kind is in the op format (`op.rs`),
  carrying a target and a direction, so the history accumulated from v1 and the
  scorer arrived over it without a wire-format version bump, which is what
  collecting them early bought. Both directions are recorded even though
  Appendix A found the signal is upvote-only — what to *count* is the scorer's
  decision, where §7.2 can change it, rather than the format's, where changing
  it costs a version.

  **This entry previously ended "rule 2 still ships no score, so nothing reads
  them yet", and that outlived rule 2 by one revision.** Noted because of where
  it sat rather than because one sentence went stale: a struck-through
  *Answered* block is the shape a reader trusts most, since it presents itself
  as settled, and this is the first place outside §7 that someone arrives at
  asking what votes do today. **An answered question is not a finished
  question** — when the section it summarises changes, the summary is part of
  that change.
- ~~**Is a hide reversible?**~~ **Answered: yes — the inverse is named. But it
  cannot currently win, and that half is not settled.**
  `op.rs` carries one `Moderate` kind with an `action` of `Hide` or `Unhide`,
  rather than two kinds, because §6.2's threshold certificate signs "the same
  `(target, action, epoch)` tuple" and a tuple needs `action` to be a field.
  Last-write-wins over a set of one was not an ordering, and a moderation
  system with no correction path makes every mistake permanent.

  **In the degraded order, an `Unhide` loses to a `Hide` of the same target
  regardless of when it was published** — the resolver prefers `Hide` where the
  transport ordered neither, because otherwise a pre-emptive `Unhide` would veto
  every future `Hide` permanently. So reversibility exists in the format and is
  suspended in practice until the Lamport gap above closes. §6 has the full
  statement; do not read this entry as "reversal works today".
- ~~**§5.7's ordering rule has no input at the contract we have.**~~
  **Answered: the rule stands unchanged; its input is missing upstream, and the
  gap is a layer below the LIDL contract.** `dialectica-core`'s `arrival::Arrival`
  records what the transport supplied alongside an op, and `arrival::cmp_ops`
  applies §5.7's rule to it. No application-level ordering was designed, because
  SDS's rule — insert by Lamport timestamp, ties by ascending message id — is
  already §5.7's.

  The gap is **two layers**, both documented with quoted source in
  `openspec/changes/archive/2026-09-11-op-ordering/design.md`:

  1. **The Reliable Channel API's `MessageReceivedEvent` carries one field** —
     the reassembled payload (LIP text quoted at design.md:40-54). The spec
     knows which value orders: it says elsewhere that the timestamp it wraps
     "acts only as a uniqueness salt; ordering is provided by the SDS Lamport
     timestamp", and then does not pass that timestamp on.
  2. **The delivery module drops even the timestamp it forwards**
     (`delivery_module_plugin.cpp` at tag `v0.2.1`, quoted at design.md:56-69).

  **The values exist at the bottom and are real.** `nim-sds` declares them as
  first-class fields on every message — `sds/types/sds_message.nim`:

  ```nim
  type SdsMessage* {.requiresInit.} = object
    messageId*: SdsMessageID
    lamportTimestamp*: int64
    ...
  ```

  So this is a **forwarding gap, not a protocol limitation**: the data SDS
  needs for §5.7's rule is present on the wire and is discarded on the way up.
  That is the sentence an upstream filing should lead with.

  > **A correction, recorded because of how it happened.** This entry briefly
  > claimed the event carries *three* fields and that the values were dropped
  > lower still, inside SDS, in a type called `SdsDeliverable` — presented in
  > bold as a correction of the sourced two-layer finding above. **That was
  > wrong and the type does not exist**: a sweep of every upstream checkout on
  > this machine found `SdsDeliverable` nowhere outside dialectica's own prose.
  > It came from an agent report that was written into this document without
  > being checked against source, and it cited as its evidence the very design
  > document that says "One field."
  >
  > **The lesson is about direction of travel.** A claim that *removes*
  > sourcing — replacing a quoted line number with a summary — should be held
  > to a higher standard than the claim it replaces, not a lower one. The
  > caveat it carried ("no realised copy to read") read as a narrow sourcing
  > gap about one field; it should have been read as a signal that nobody had
  > opened the file.

  Two findings worth carrying forward. ~~**The `timestamp` we do receive is
  unusable for ordering.**~~ **Specified — the `op-transport` spec's "The
  arrival timestamp is a local clock reading and orders nothing" contracts
  what a receiving peer may do with it, and its companion requirement
  contracts that this transport supplies no ordering metadata to record.**
  What stays here is the measurement and the pairing: the value is the
  receiving peer's own `CLOCK_REALTIME` read taken when its callback fires,
  and exactly one event (`messageReceived`) reads a real wire timestamp —
  which is why only that one shows §11's units divergence. The two traps are
  one divergence seen from both ends.

  ~~And **a dialectica-side Lamport clock is the one thing not to build**~~
  — **withdrawn, 2026-09-12.** The argument was that SDS's clock advances on
  traffic no application sees and is initialised from epoch-ms, so a
  dialectica clock could never be made to agree with it.

  **That is true and it is not a reason.** It assumes our clock must agree
  with SDS's, and it does not: **a dialectica-level clock needs to agree with
  other peers' dialectica clocks**, and every peer sees the same ops. Stop
  trying to reconcile with a clock we cannot read and the objection
  disappears. The mistake was letting "we cannot match SDS" stand in for "we
  cannot order".

  **The withdrawal has not reached the spec, and that is an open contradiction
  rather than a loose end.** `op-ordering`'s leading requirement still states
  that "A peer SHALL NOT compute a Lamport timestamp of its own, and SHALL NOT
  maintain a second logical clock alongside the transport's". The plan withdrew
  that above; the spec has not been changed, so the two disagree today.
  **Resolving it needs its own change**, because withdrawing the prohibition
  without the replacement leaves a requirement that forbids nothing and requires
  nothing in its place — and the replacement is the design this entry says is
  not done here: an author-set counter is not an ordering until it has a bound,
  and both adversarial cases below are unaddressed. The `op-transport` spec was
  written deliberately neutral to how this resolves: it contracts what the
  transport supplies, which is nothing, and says so without depending on the
  prohibition being either live or withdrawn.

  ### The layering rule, which everything above is a consequence of

  **Use SDS's API as it is, and build the ordering and causality the
  application needs on top of it.** SDS is HTTP or TCP in this stack. TCP
  retransmits, orders within a connection, and tells you a transfer failed —
  and it still cannot tell you your file is half-written, because it does not
  know what a complete file is. Only the application does.

  So SDS repairing what it can see is not a substitute for dialectica knowing
  what a **complete thread** is, and no upstream change would make it one.

  **This reframes the whole entry.** The missing fields are not a deficiency
  waiting on a fix we should design around — they were never ours to depend
  on. An application that can only order its own content while the transport
  hands it ordering metadata breaks the moment that transport changes, and
  §4.4 already warns that SDS is LIP-109 at *raw*, the weakest maturity tier,
  with an API marked Developer Preview that expects to change.

  Read the rest of §13 as *what dialectica owns*, not as *what upstream owes
  us*. The upstream filing is still worth making; nothing here waits on it.

  **What we may build at our own layer**, carried inside the signed op
  preimage so a relay cannot forge or strip it:

  - **An author-asserted wall-clock `createdAt`.** Cheap, needs nothing from
    upstream, and gives a real recency ordering today. It is *asserted*, so it
    **must be clamped** and must never feed a security decision — Appendix A
    records the nearest kin project reading an unclamped author timestamp for
    decay, which lets a post pin itself to the top permanently.
  - **A dialectica Lamport counter**, advanced on the ops we receive.
    Self-consistent across peers without reference to SDS.

  **Causality is ours too, and an earlier draft of this entry got that
  wrong.** It claimed happened-before was the one thing unrecoverable at our
  layer. It is not: **a Lamport counter is the causality mechanism.** Alice
  receives an op at N, sets her clock to `max(local, N)`, and replies at N+1;
  the reply carries "I had seen something at N". That is precisely the
  written-knowing-about relation, and a dialectica counter supplies it.

  The error was conflating the counter with the wall clock, and attributing
  the wall clock's weakness to both.

  **What SDS's `causalHistory` adds is not causality but *gap detection*** —
  an explicit list of message ids the sender held, so a receiver can notice
  "this references X and I do not have X". A scalar counter cannot: N+1 says
  she had seen *something* at N, never *which*.

  **But we already have that edge, and at the granularity we actually want.**
  A reply names its parent op id inside the signed preimage — it must, that is
  what makes it a reply — so "I hold a reply to X and no X" is detectable with
  no `causalHistory` at all. **The parent pointer is the causal edge the forum
  cares about.**

  And the scopes differ in a way that matters more than the mechanism:

  - **SDS repairs per Stoa.** One channel per Stoa (§4.1), so its machinery
    chases gaps across every thread at once — most of which a given reader
    will never open.
  - **Dialectica needs repair per thread.** Someone opening a thread wants
    *that* thread complete. Ops missing from threads nobody is reading are not
    urgent and may never be worth fetching.

  A thread is a tree walkable from its root, so the set of ops needed to
  render it completely is computable from the ops we hold. That is a better
  repair trigger than a filter over the whole Stoa, because it is
  **demand-driven**: repair what someone is looking at.

  **So SDS's reliability machinery is both more than we need and differently
  shaped** — global where we want local, a message graph where we have a post
  tree. Losing `causalHistory` costs less than it appears to.

  **Nothing about ordering requires the upstream gap to close**, and nothing
  about thread completeness does either. Recency, total order and
  happened-before are all buildable here; the gap is a reason to build them
  rather than a reason to wait.

  Not designed here; §5.7 keeps its rule and `Arrival` its shape until one is.

  **Two adversarial cases the design must answer, named now so they are not
  discovered later.** Both fields sit in a signed op, so a *relay* cannot
  forge them — but the **author** controls both completely, and an author is
  not trusted:

  - **A far-future `createdAt`** pins a post to the top of a recency ordering
    permanently. Appendix A measures exactly this failure in the nearest kin
    project. The clamp is the whole defence and it is not specified here.
  - **An arbitrarily high Lamport counter** does the same to the total order,
    and is the attack the `createdAt` clamp does not cover. A counter is only
    meaningful relative to ops a peer has seen, so the bound is different in
    kind from a wall-clock clamp.

  Neither is hard, and neither is optional. **A field a malicious peer sets
  freely is not an ordering until it has a bound.**

  Until the fields arrive, ops are recorded as unordered and fall back to a
  defined degraded order (ascending op id, always below any op the transport did
  order) that is identical on every peer and reports itself as degraded. **The op
  log and the resolvers are unblocked**: they have a defined thing to key on.

- **Should an op carry an author-asserted `createdAt`, and what clamps it?**
  **Raised by the authoring change, which declined to add it and contracted the
  consequence instead.** One gap, two symptoms, and the second was not previously
  written down:

  1. Both accepted feed orderings degrade to ascending op id, because no op
     carries a value that orders anything (§9.1 §8 has this half).
  2. **Two identical posts are one op.** An op id hashes bytes carrying no
     timestamp and no nonce, so one author posting the same body into one Stoa
     twice publishes once. That is right for a double-clicked submit and wrong for
     someone deliberately posting "agreed" twice.

  The `content-authoring` spec makes symptom 2 **visible rather than surprising** —
  its requirement "Publishing the same content twice publishes one op" states the
  behaviour, requires the newly-stored-or-already-present answer to reach the
  caller, and names this field as the declined fix. So the next reader meets a
  decision rather than a user complaint.

  **Why the authoring change declined it** is recorded where that reasoning
  belongs, in that change's `design.md` under Decisions, rather than copied here —
  two copies of a rationale drift and the wrong one gets read. In short: it is a
  `MODIFIED` to `op-format`'s "An op carries no ordering field and no per-peer
  state", and the clamp that would make it safe is unspecified.

  **A nonce is the narrower alternative** — it separates two identical posts and
  does nothing for ordering. Worth naming so the two are not conflated: if only
  symptom 2 needs fixing, a nonce is cheaper and needs no clamp; if ordering is
  wanted, `createdAt` covers both and the clamp is the work.

  **What would decide it:** whoever takes the ordering question in §9.1 §8, since
  it is the same field. Whichever change adds it must modify the
  `content-authoring` requirement above, which is the correct place for the
  pressure to land.

- **Does an expiring credential want a grace period?** §5.5 settles that proofs
  expire and the holder re-proves on a cadence, and records the cost: a user
  offline for longer than their window silently loses standing, and a moderator
  weighted by token holding quietly stops being weighted. Whether that is
  acceptable or wants a grace period is not settled. It is a policy question
  rather than a mechanism one — the same shape as §7.2 rule 5's decay, which is
  "specified as a property and deferred in mechanism" (§7.3 uses that phrasing for
  vouch decay) — and it cannot be answered before a credential exists to expire.
  **Recorded so it is a decision rather than a surprise**, per §5.5.

- **What does a peer do when a verifier module is present but cannot answer?**
  §5.5 settles the *absent* case and it needs no machinery: a reader without the
  module scores the author as holding nothing, which is the same local answer, and
  no peer is wrong. The residue is narrower — a module that is installed but fails
  a particular check, because the RPC it fronts is down or the chain is
  unreachable. Scoring the author as holding nothing is the obvious answer and is
  probably right, since it is what a reader lacking the module already does. What
  is unexamined is whether a *transient* failure should be distinguishable from a
  settled one to whoever is looking at the screen, and whether retry belongs in
  the module or above it. **Not answerable before a credential module exists**,
  and low stakes either way, because the effects stay local (§5.5).

---

## Appendix A. OpChan's relevance implementation, measured

Supporting evidence for §7.2. Here rather than inline because it is a reading of
someone else's code at a moment in time: it will drift, and §7.2's rules must
not depend on it staying accurate. Read it once to see why each rule is an
inversion, then trust the rules.

`logos-messaging/OpChan` is a Logos-ecosystem forum — TypeScript over plain
Waku, no SDS, no module packaging — so **no code transfers**. The design does,
mostly as a negative example.

**Read at `main` as of 2026-09-09**, commit `d48f975`. Every claim below is from
that revision; re-read before treating any of it as current, and note that a fix
upstream does not invalidate the corresponding rule in §7.2 — the rules are
positions this project holds, not observations about someone else's repo.

**Source the code, not the docs.** The scorer is
`packages/core/src/lib/forum/RelevanceCalculator.ts`. Its architecture doc
(`packages/core/docs/architecture.md`) documents a completely different set of
constants, in an additive rather than multiplicative form; it does not describe
what runs.

### The score

```
score = ( (10 + 1.0·upvotes + 0.5·comments) · verification_multiplier
          + 0.1·verified_upvoters + 0.05·verified_commenters )
        · e^(−0.1·days) · (moderated ? 0.5 : 1)
```

`verification_multiplier` is 1.25 for an ENS holder, 1.10 for a connected
wallet, 1.0 otherwise. Comments score the same way from a base of 5, with
upvotes only. Cells use a different formula again.

For a fresh post:

| | score |
|---|---|
| anonymous author, 0 upvotes | 10.00 |
| **ENS-verified** author, 0 upvotes | 12.50 |
| anonymous author, **3 free sybil upvotes** | 13.00 |

**Three throwaway identities outweigh holding an ENS name** — 13.00 against
12.50 — though the fairer statement of the same fact is the ratio: a
credentialed voter's premium (+0.10) is a *tenth* of the raw vote it rides on
(+1.00), and the author multiplier is a flat 25% however many votes are in play.
The credential garnishes an unmetered signal instead of gating it, which is
§7.2 rule 3 as a measurement. (The table compares an ENS holder at zero votes;
one with the same three votes scores 16.25. The ratio is the durable point.)

Downvotes are collected, attached to posts, and then filtered out of every
scorer. The signal is upvote-only.

### Four more findings, each a rule

- **The moderation penalty is ×0.5**, so a well-upvoted hidden post outranks a
  fresh visible one. A haircut does not bind (rule 4).
- **Decay reads an author-asserted timestamp** with nothing clamping it, so a
  post claiming a future time gets an unbounded multiplier above 1. The
  validator does notice future timestamps and produce a warning — which is never
  called on the ingest path. Clamp on read; an op's claimed time is
  attacker-controlled.
- **Cell decay is dead code.** The reduce finding a cell's most recent post is
  seeded with `Date.now()`, so the seed beats every honest post and the
  multiplier is ≈1.0 always — an empty cell scores its full undecayed base and
  outranks an active one. Invisible without a test asserting *ordering* rather
  than that a score was produced.
- **Vote dedup is last-write-wins by inequality, not `>`**, so an older vote
  can overwrite a newer one and peers resolve a vote-flip differently by arrival
  order. The moderation path two cases away uses `>` correctly. Use Lamport
  order (§4.4); do not invent a second rule.

### Moderation authority is never checked on read

Six call sites check that the actor is a cell admin — all on the *send* path.
The read path applies any moderation message it finds without comparing the
signer to the cell's owner. **Any peer can forge a moderation, or forge the
removal of one**, in shipped code. Anonymous authorship is likewise unbound: the
check is that the author string is shaped like a UUID, while the signature is
verified against a public key the message itself carries.

This is §6's central claim demonstrated rather than argued.

### Proof-of-holding, and why the datapoint is not what it looks like

OpChan's was an HTTP call to a third-party indexer returning a boolean, cached
and trusted — every peer had to trust that service, and a peer without an API
key computed different scores. LP-0005 (§7.1) is a cryptographic proof verified
locally and offline, so §7's reordering is not repeating this.

The signal was **binary** (one holding and fifty were identical) and was removed
in a commit titled *"remove bitcoin + appkit, use eth + viem/wagmi"* — **it went
out because the Bitcoin wallet stack went out, not because proof-of-holding was
judged a bad signal.** No ADR, issue or commit anywhere argues against the
approach. An earlier reading of this repo concluded the opposite; that reading
was wrong, and the correction is why §7 could reorder with a clear conscience.

### No per-Stoa standing to copy

One global identity, one global claim, one global multiplier, with a per-cell
hide bolted on. Relevance scoped to a Stoa is a design to originate, not port.
