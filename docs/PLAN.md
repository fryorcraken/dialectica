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

### 3.3 The local store is ours

**An "op" is a signed operation, and it is the unit everything else is built
from.** Creating a Stoa, posting, publishing a revision of your own post (§5.7),
hiding something as a moderator (§6), voting — each is one op, signed by its
author and published to the Stoa's channel. Nothing else crosses the wire, and
the forum's whole state is a function of the ops a peer has seen.

Each peer keeps a **local SQLite store** holding every op it has seen, plus a
materialised view of the forum derived from it. Ops are the authority; the view
is a cache that can be rebuilt by replay.

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
self-asserted. Verification therefore happens on **read**, filtering unsigned or
badly-signed ops out. The store may hold junk; the reader never trusts it.

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

`channelCreate(channelId, contentTopic, senderId)` decouples channel from topic.

- `contentTopic` = the Stoa, hashed and bucketed: `/dialectica/1/s/<hex>/proto`
- `channelId` = the Stoa, and **the same value for every peer in it** — it is the
  rendezvous, not a local handle (§4.3)
- `senderId` = **one per user per Stoa, permanent** — every participant's is
  different (the API requires it); what is stable is that a given user keeps
  theirs across sessions. A transport self-filter, not an author identity; see
  below
- `threadId` and `parentPostId` live in the **payload**, never the topic

**`senderId` is not an author identity, and the plan should not treat it as
one.** It exists so SDS can tell a participant's own messages from everyone
else's: the spec notes that "outside of filtering messages originating from the
sender itself, the `sender_id` field is not used for much", and the receive step
is a SHOULD to "ignore the message if it has a `sender_id` matching its own".
Acknowledgement accounting runs off message ids carried in causal history and
bloom filters, not off sender ids — so reliability does not depend on a
`senderId` meaning anything in particular.

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
which §4.3 makes a crash risk.

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

**So the channel id can carry no per-peer state.** Not a session counter, not a
local sequence number, not anything that varies with one peer's history. A value
that differs between peers does not produce an error: it produces two Stoas that
cannot see each other, silently and permanently. §4.5 states the same rule from
the other direction — derive `channelId` as a pure function of the addressed
object.

**Live bug, read it before touching channel lifecycle:
`logos-messaging/logos-delivery#4116`.** Closing a channel that has received a
peer message and then re-creating it with the same id kills the whole node
process — and the v0.2.1 docstring claims the opposite (the issue reproduces on
0.2.0; whether it was re-confirmed at 0.2.1 is not recorded), so following the
documentation is what walks you into it. The issue has the conditions and the
repro; do not restate them here, they will be wrong once it is fixed.

What it costs *us* is the shape below, which stands on its own.

**Dialectica closes a channel in two places: when a user leaves a Stoa, and on
shutdown.** Both, and the second is the one that looks optional and is not.

**The delivery node is not ours to stop, and it outlives us.**
`delivery_module` is a separate, shared process — `createNode` is called once
per context (§11), and issue #4116 notes in passing that when the node dies "any
other module sharing that node loses it too". So dialectica exiting does not
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

So dialectica closes channels, and #4116 makes a close followed by a re-create
dangerous. Three ways out; the first is ruled out by this section's own rule.

**Ruled out — a per-peer epoch in the channel id, bumped on each open.** That is
what a per-peer value in a rendezvous field costs: peer A reopens at
`stoa-abc/e8` while peer B is still on `stoa-abc/e7`, and they stop seeing each
other with no error anywhere, which is worse than the crash because it is
silent.

**Legitimate but unbuilt — a *deterministic* epoch every peer computes
identically**, from a genesis-record field or a coarse clock bucket. The issue
confirms that create → close → create with a *different* id does not reproduce,
so this genuinely avoids both the crash and the partition. It is not designed
here, and the hard part is that a value which changes must change for everyone
at once — a rendezvous problem at each boundary. **This is the option to revisit
if the assumption below fails.**

**Chosen for v1 — leave the id alone and never re-create a channel inside one
node's lifetime.** The bug needs a close and a re-create in the same node;
dialectica controls whether that ever happens. Concretely:

- **Open each Stoa's channel once per node lifetime.** Do not close and reopen
  as a way of recovering from an error, refreshing state, or reacting to
  connectivity changes. Reopen only after the node itself has gone away.
- **On leaving a Stoa, close and do not reopen in that session.** Rejoining
  before restart is the one user-visible path into the bug; make it re-create
  the node, or defer the rejoin, or accept it as a known limitation until #4116
  is fixed.

That leaves the ordinary restart safe, since the node is new.

**Untested, and the approach above rests on it:** #4116 reproduces within one
running node. Whether persisted SDS state surviving a full process restart
corrupts a fresh `createNode` the same way is unknown. Two runs against a real
node, with peer traffic received in the first, settles it — worth doing before
relying on any of this, because the failure surfaces at startup while the code
responsible ran in the previous session.

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
- **No ordering metadata reaching the application.** The Lamport total order and
  the message-id tie-break above are real and are what SDS orders its own log
  by — but they stop below us. The Reliable Channel API's received-message event
  carries the payload alone, so neither value reaches a consumer, and
  `channelMessageReceived` cannot forward what it never got. **Read the promises
  above as internal to SDS, not as an interface.** §13 has the finding and what
  each upstream layer would have to add.
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

To keep it cheap: derive `channelId` as a pure function of the addressed object
— Stoa now, `(stoa, thread)` later — and never let channel identity leak into
payloads or storage keys. Ops carry `threadId` from day one anyway (the topic
cannot carry it), so the split becomes a routing change rather than a migration.

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

**Phase 1 — addresses as links.** A Stoa address is a copyable string. Importing
one is how you join a Stoa nobody told the app about; a Stoa address appearing
in a post renders as a link that enters that Stoa on click. This is the whole
mechanism, and it is enough for a network that grows by word of mouth.

Two things to get right, both security-relevant: an address must be
**self-authenticating** — pasting it is enough to verify what you joined,
because the address is a hash of the genesis record (§5.1), so a wrong or
tampered record fails to match. And in-post addresses are **attacker-supplied
content**: render them as an explicit affordance the reader chooses to act on,
never auto-join, and show what is being joined before joining it.

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

Three tiers, only two of which exist in v1:

| Tier | Covers | v1 |
|---|---|---|
| SDS window | recent ops, retransmission, causal-history and SDS-Repair backfill | ✅ |
| Local SQLite | everything this peer has ever seen | ✅ |
| Logos Storage snapshots | deep history, beyond what live peers hold | ❌ later |

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

### 5.6 The keystore

Copy radicle's proven model: encrypted key at a fixed path, public key yields
the handle, three unlock paths (plaintext keystore / passphrase env / agent),
and **the module never prompts** — it fails with a message naming the fix.

Ship a capability probe and gate every posting affordance on it:

```
getCapabilities() -> {"canPost":bool, "identity":"…" | "reason":"…"}
```

Gate on the probe, never on a build flag. A compose box that cannot be submitted
loses whatever the user typed.

> LEZ's own keystore is plaintext JSON at 0644 containing every secret, with
> `// TODO: Use password for storage encryption`. Match the crypto, not the
> key handling.

---

### 5.7 Mutability: revision, not shared state

**A post is never edited in place. An edit is a new version of that post,
published and signed by the same author.**

This is the whole conflict rule for post content, and it is deliberately not a
merge strategy. Two versions of a post do not *conflict* — they are ordered:

- **Authorship decides validity.** A version signed by anyone other than the
  post's original author is invalid and dropped on read (§3.3). There is no
  case where two authors contend for one post.
- **Lamport order decides currency.** Among an author's own versions, the
  highest Lamport timestamp is current, ties broken by ascending message id —
  the same rule SDS already applies (§4.4), so nothing new is invented.
- **History is kept.** Superseded versions stay in the op log. The UI can show
  that a post was edited, and a moderator acting on a post is acting on a
  version they can name.

The reason this matters beyond edits: it means dialectica has **almost no
shared mutable state**. Posts and replies are append-only; an edit appends too.
SDS gives an order, and an order plus "only the author may revise" is a complete
answer — no CRDT, no merge function, no last-writer-wins ambiguity.

### What this rule does not cover

Three pieces of state are not authored revisions of a post, and each needs its
own answer:

- **Moderation flags.** Not the author's to revise, by design. A moderator's
  hide and the author's edit are about different things and do not contend: an
  edit does not clear a hide, and a hide does not invalidate an edit. Among
  *moderation* ops on the same target, last-write-wins by Lamport order, valid
  only if the signer was a moderator at that time (§6).
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

**Signed ops with a Stoa moderator set.** Every op is signed by its author.
Moderation ops are valid only when signed by a current moderator, and every peer
verifies independently — so a hide binds for everyone running honest code.

```
genesis: {version, creator_pk, policy, title}   ← built; see the stoa-genesis spec
post:    {..., sig(author_sk)}
hide:    {..., sig(mod_sk)}    ← rejected if signer ∉ moderators
```

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
converges, and it names an `opId`.

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

**It does not unblock §7.2's scoring**, and the plan should not pretend
otherwise. Rule 2 ships no score precisely because sybil resistance is absent,
and that is RLN's job rather than a holding proof's: §5.3's ordering argument
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

#### The five rules

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

**2. v1 ships `new` and `active`, and no score at all.** With no sybil
resistance (§7), a vote-weighted score is not a relevance signal — it is a dial
the cheapest attacker turns. Two orderings that cannot be gamed by minting
identities:

- **`new`** — Lamport order descending. §4.4 already defines a total order with
  a tie-break; reuse it exactly rather than inventing a second ordering.
- **`active`** — threads by the Lamport timestamp of their most recent
  non-hidden reply. Gameable only by *posting*, which moderation and rate
  limiting already govern.

This is a smaller claim than "we have a relevance model" and it is the true one,
in the same spirit as §7's refusal to claim sybil resistance.

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
the holding proof §7 brings forward for policy gating. Until then, rule 2
stands — no score at all. Gating on a re-presentable credential would raise the
unit cost of a sybil vote without stopping one, and would claim more than it
delivers.

**4. Moderation filters, it does not penalise.** A post hidden by a valid
moderator op is **excluded** from the projection, not demoted. §6 says a hide
binds; a percentage haircut does not bind, it merely means a sufficiently
upvoted hidden post outranks a visible one. Keep hidden posts in the op log
(§5.7 keeps history) and let the UI offer a "show hidden" view — but the default
feed omits them.

**5. Decay must be indexable.** §2.5's paginated API has to `ORDER BY … LIMIT`
in SQLite, and a score recomputed from the current clock on every read cannot be
indexed. Store a decay-free score plus a timestamp and apply decay in the
`ORDER BY` expression, or bucket age coarsely and recompute on a timer. **Decide
this when the projection schema is designed, in Phase 1** — retrofitting an
index onto a time-varying score is the expensive version.

#### The shape to reserve now

```
score = f(engagement) · decay(age) · weight(author_claims)
```

with `weight(∅) = 0` for votes (rule 3) and moderation handled by exclusion
rather than a term (rule 4). In v1 no claim exists, so nothing is ranked by this
and `new`/`active` are what ship. Writing the shape down now is what lets the
claims layer land without a schema migration.

#### Where the rules come from

Every rule above is stated as an inversion because `logos-messaging/OpChan` —
the nearest kin, a Logos-ecosystem forum with a real relevance implementation —
shipped the un-inverted version. **Appendix A** has the arithmetic and the
citations; the one number worth carrying inline is that in OpChan's scorer,
**three free sybil upvotes outrank holding an ENS name**, and a credentialed
voter's premium is one tenth of the raw vote it rides on. That is rule 3 as a
measurement rather than an opinion.

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

**Zero SDK types in this crate.** That is not a preference: anything touching
`modules()` or `context()` calls `lp_*` symbols undefined in an rlib and will
not link into a test binary, so SDK types anywhere in the domain logic make it
untestable (§2.3).

**Phase 2 — wire the real modules.** Swap the fakes for `delivery_module`
channels and `storage_module`.

**Phase 3 — the forum.** Feeds, threads, composition, moderation UI.

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
  compares cargo's result against the count of `#[test]` attributes in `src/`,
  so it cannot rot. A hardcoded floor that nothing keeps in sync is itself a
  false green.

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
- **`messageReceived`'s timestamp is nanoseconds**; every other event is
  ISO-8601 (delivery bug #26).
- **`messageReceived` fires for your own messages; `channelMessageReceived` does
  not** — own sends come back as `channelMessageSent`.

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

Two of these are **stale working trees** and will mislead if read directly:
`logos-delivery-module` sits on a pre-channels branch, and
`logos-messaging/logos-delivery` predates reliable channels entirely. Use
`git show v0.2.1:<path>` for the delivery API.

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
- What is the actual participant ceiling for one SDS channel? Unmeasured, and
  the answer sets when §4.5 stops being optional. With SDS now the *only* sync
  layer, there is no CRDT fallback if a channel degrades.
- How far back does SDS-Repair realistically reach in a live Stoa? That number
  decides how urgent snapshots are.
- **How is the moderator set ordered when two moderators edit it
  concurrently?** The one genuine merge question in the design (§5.7), and it
  does not arise while the creator is the sole moderator — so it is answered
  alongside mutable moderation, not before.
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
- **Does #4116 survive a process restart?** §4.3's approach — never re-create a
  channel inside one node's lifetime — assumes persisted SDS state does not
  corrupt a fresh `createNode`. Two runs against a real node, with peer traffic
  received in the first, settles it.
- ~~**Are votes an op in v1 at all?**~~ **Answered: yes, collected and read by
  nothing.** The kind is in the op format (`op.rs`), carrying a target and a
  direction, so the history accumulates from v1 and scoring arrives later
  without a wire-format version bump. §7.2 rule 2 still ships no score, so
  nothing reads them yet. Both directions are recorded even though Appendix A
  found the signal is upvote-only — what to *count* is the scorer's decision,
  where §7.2 can change it, rather than the format's, where changing it costs a
  version.
- ~~**Is a hide reversible?**~~ **Answered: yes — the inverse is named.**
  `op.rs` carries one `Moderate` kind with an `action` of `Hide` or `Unhide`,
  rather than two kinds, because §6.2's threshold certificate signs "the same
  `(target, action, epoch)` tuple" and a tuple needs `action` to be a field.
  Last-write-wins over a set of one was not an ordering, and a moderation
  system with no correction path makes every mistake permanent.
- ~~**§5.7's ordering rule has no input at the contract we have.**~~
  **Answered: the rule stands unchanged; its input is missing upstream, and the
  gap is a layer below the LIDL contract.** `dialectica-core`'s `arrival::Arrival`
  records what the transport supplied alongside an op, and `arrival::cmp_ops`
  applies §5.7's rule to it. No application-level ordering was designed, because
  SDS's rule — insert by Lamport timestamp, ties by ascending message id — is
  already §5.7's.

  The investigation moved where the gap is. It is **not** that
  `delivery_module.lidl` forgot to forward the fields: the Reliable Channel API's
  `MessageReceivedEvent`, which the delivery module consumes, carries exactly one
  field — the reassembled payload — so `channelMessageReceived` never receives
  them either. Closing this needs a change at both layers, and neither is
  dialectica's; `openspec/changes/op-ordering/design.md` records the field names
  and types for filing.

  Two findings worth carrying forward. **`channelMessageReceived`'s `timestamp`
  is unusable for ordering** — it is the receiving peer's own `CLOCK_REALTIME`
  read taken when its callback fires, so it differs per peer for one message,
  which is worse than §11's units problem and worth filing separately as a bug.
  And **a dialectica-side Lamport clock is the one thing not to build**: SDS's
  clock advances on traffic no application sees and is initialised from epoch-ms,
  so a clock advanced on op arrivals could not be made to agree with it — and two
  orders that disagree produce no error, only two peers rendering a thread
  differently.

  Until the fields arrive, ops are recorded as unordered and fall back to a
  defined degraded order (ascending op id, always below any op the transport did
  order) that is identical on every peer and reports itself as degraded. **The op
  log and the resolvers are unblocked**: they have a defined thing to key on.

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
