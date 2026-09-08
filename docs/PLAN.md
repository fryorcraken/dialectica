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

### 2.3 Two facts that shape the core API

**Every `modules().x` call is IPC.** Modules run in separate processes over Qt
Remote Objects unix sockets. Never put a cross-module call in a hot loop.

**Async callbacks run on the module's event loop, after the current method
returns.** You cannot await a cross-module result inside a method. The shape is:
fire, stash, read back through a later call. Design the API around this rather
than discovering it.

### 2.4 The wire contract

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
does not use SDS, and does not need to: every op is idempotent by `opId`, safe
to apply out of order or repeatedly, so convergence comes from merge semantics
rather than from transport ordering.

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
from Rust therefore needs a `dependency_overrides` entry pointing the generator
at its impl header:

```json
"dependency_overrides": {
  "delivery_module": {
    "file": "…/delivery_module_plugin.h",
    "input": "delivery_module",
    "impl_class": "DeliveryModuleImpl"
  }
}
```

The builder documents and supports this. **Nobody has demonstrated it for this
case.** It is the single unproven link in the architecture, and Phase 0 exists
to retire it before anything depends on it.

### 3.3 The local store is ours

Each peer keeps a **local SQLite store** holding every op it has seen, plus a
materialised view of the forum derived from it. Ops are the authority; the view
is a cache that can be rebuilt by replay.

This is the piece cloud_data would have provided, and owning it is what lets us
index for the queries a forum actually makes — newest threads, paginated
replies, a Stoa's index — rather than scanning every row.

**Op authenticity is dialectica's job, not the transport's** (§6). A forged op
cannot be prevented from *arriving*: SDS has no membership and `senderId` is
self-asserted. Verification therefore happens on **read**, filtering unsigned or
badly-signed ops out. The store may hold junk; the reader never trusts it.

---

## 4. Addressing and transport

### 4.1 One reliability channel per Stoa

`channelCreate(channelId, contentTopic, senderId)` decouples channel from topic.

- `contentTopic` = the Stoa, hashed and bucketed: `/dialectica/1/s/<hex>/proto`
- `channelId` = the Stoa (plus an epoch — see below)
- `threadId` and `parentPostId` live in the **payload**, never the topic

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

### 4.3 The channel-id trap

Reusing a `channelId` after `channelClose` on a channel that had received a peer
message **crashes the node** (logos-delivery#4116) — and the v0.2.1 header
docstring claims the opposite ("persisted channel state survives channelClose").
Trust the bug, not the docstring.

Since our channel id derives from the Stoa, **every normal restart is a
close-and-reopen with the same id**: first run creates `stoa-abc` and closes it
on shutdown; the next run recreates `stoa-abc` and crashes the node.

**Carry an epoch in the channel id** — `stoa-abc/e7` — and bump it whenever the
id would otherwise be reused. Same Stoa, a fresh channel id each session, the
bug never triggered. Free now, painful to retrofit, and it doubles as the
version marker that lets peers migrate across the §4.5 split without the two
regimes colliding.

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

### 5.2 Scope: one identity per Stoa

A user has **one identity within a Stoa**, stable across every thread in it.
Identities are unlinkable **across** Stoas — derive the diversifier from the
Stoa id so per-Stoa pseudonyms fall out of a single root key for free.

This is the trade that makes moderation work: banning an identity binds, because
that identity is stable everywhere it can post.

Revisitable later.

### 5.3 No rotation in v1

Deliberate, and the reasoning is ordering: **rotation without spam protection is
a ban-evasion feature.** Until we can distinguish "my key leaked" from "I got
banned", rotation makes moderation unenforceable. It arrives with RLN — see §7.

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
- LEZ proof-of-holding → "owns this NFT" → higher relevance

A signature-only interface cannot carry either. Costs nothing to get right now;
surgery later.

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
- **The moderator set itself.** Deferred with mutable moderation (§6), and the
  one place where a real ordering decision is still open — a set edited
  concurrently by two moderators is the first genuine merge question this design
  has. It does not arise while the creator is the sole moderator.

## 6. Moderation

**Signed ops with a Stoa moderator set.** Every op is signed by its author.
Moderation ops are valid only when signed by a current moderator, and every peer
verifies independently — so a hide or ban binds for everyone running honest
code.

```
genesis: {stoa_id, creator_pk, epoch}
post:    {..., sig(author_sk)}
hide:    {..., sig(mod_sk)}    ← rejected if signer ∉ moderators
```

**The creator is the sole moderator initially.** A mutable moderator set is
later work — which also defers the founder-as-permanent-root question rather
than answering it prematurely. A Stoa whose moderation people dislike can be
forked — a Stoa's participants are never locked into its moderation.

### 6.1 Threshold moderation, later

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
recoverable only as punishment — so almost none of it transfers. But its N-of-M
certificate aggregation is orthogonal to whether posters are anonymous or
pseudonymous, and it is the one part of that solution that maps onto this
design. Read the protocol doc and the ADRs; the code is TypeScript over plain
Waku and does not fit here.

This layer is not optional and not a nice-to-have: **no layer below provides
authenticity.** SDS has no membership and its `senderId` is an
application-chosen string the spec itself notes "is not used for much".
Moderation that anyone can forge — or forge the removal of — is not
moderation. This is the part of dialectica that nothing else provides, and it is
where the core's real design work lives.

---

## 7. Spam and sybil resistance

**v1: moderation plus client-side rate limiting. No sybil resistance claimed.**

That claim would be false if made. RLN today is free to mint
(`MembershipFee = 0`, faucet-funded), slashing is a `# TODO`, and it is switched
off on the target network (`rlnRelay: false` on `logos.dev`). Registering 100
memberships costs 100 faucet claims and buys 100× the posting rate.

Later, in this order:

1. **RLN** as a rate-limit credential — and the precondition for §5.3 rotation.
2. **LEZ proof-of-holding** as a relevance signal, and as a Stoa access policy
   (§7.1).

Both arrive through §5.5's claims interface. Sybil resistance ultimately lives
in the membership-allocation service's pluggable auth hook (LIP 158), which is
unbuilt — design against it, do not claim it.

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

Caveats for whoever picks this up: single contributor, 0.1.0 since May 2026,
and its Basecamp module is a QML/C++ plugin rather than a `codegen.rust`
cdylib — so its packaging is a useful reference but its module shape is not
ours. Take the circuit and the crates, not the integration.

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

This exists to retire §3.1 before anything depends on it. If that bridge does
not work the architecture changes, and that is a week-one discovery, not a
month-three one.

**Phase 1 — core semantics behind traits.** Signing, the Stoa/thread/post model,
moderator-set verification, the op log and its SQLite projection, query
indexing — pure Rust behind `Transport` and `Store` traits, tested against fakes
with no node running.

**Phase 2 — wire the real modules.** Swap the fakes for `delivery_module`
channels and `storage_module`.

**Phase 3 — the forum.** Feeds, threads, composition, moderation UI.

---

## 10. CI

Radicle's `ci.yml` is the template: `lint` / `qml` / `rust` / `build` /
`release`, plus a matrixed sitometres e2e workflow.

**Pins:**

- **`lgs`: the unreleased rev.** `setup --inspector` builds a Basecamp with the
  QML inspector compiled in, which sitometres needs to drive the UI, and it does
  not exist in v0.3.1. Accepted risk, recorded here so it is diagnosable: that
  rev lives on an unmerged PR branch, and a force-push or branch deletion makes
  every CI job fail inside `cargo install`. If every job in both workflows
  suddenly fails there, this is the first thing to check.
- **sitometres: latest release.** Radicle pins a git commit only to work around
  a probe bug in published 0.1.0; that is a workaround, not a pattern to copy.

**Anti-false-green, throughout.** Radicle's CI is built around the observation
that a green gate which cannot see the thing it claims to check is worse than no
gate. Copy the habits, not just the jobs: assert test *counts* against spec
counts, assert packaged manifests kept their entry points, assert dev and
portable variants were not transposed, and verify a new test fails before it
passes.

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
- **`lgs basecamp setup` strips every comment from `scaffold.toml`.** Run
  `git diff scaffold.toml` after any setup.
- **The UI icon must be a 256×256 PNG**, and the UI module must declare core in
  `dependencies` at a **matching version**.
- **Pin `logos-module-builder` ≥ 0.2.5** — earlier builders deliver empty binary
  event payloads. Assert non-empty payloads in a test.
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

Two of these are **stale working trees** and will mislead if read directly:
`logos-delivery-module` sits on a pre-channels branch, and
`logos-messaging/logos-delivery` predates reliable channels entirely. Use
`git show v0.2.1:<path>` for the delivery API.

`logos-module-builder` and `logos-rust-sdk` are not local checkouts — they are
flake inputs. Find them via a consuming project's `flake.lock`, or in
`/nix/store`. The Rust module examples and doctests live inside the
`logos-rust-sdk` source, under `tests/` and `doctests/`.

## 13. Open questions

- Does the `dependency_overrides` LIDL bridge to `delivery_module` work?
  (Phase 0 answers this.)
- What is the actual participant ceiling for one SDS channel? Unmeasured, and
  the answer sets when §4.5 stops being optional. With SDS now the *only* sync
  layer, there is no CRDT fallback if a channel degrades.
- How far back does SDS-Repair realistically reach in a live Stoa? That number
  decides how urgent snapshots are.
- **How is the moderator set ordered when two moderators edit it
  concurrently?** The one genuine merge question in the design (§5.7), and it
  does not arise while the creator is the sole moderator — so it is answered
  alongside mutable moderation, not before.
- **Not whether a Stoa declares a posting policy, but when the field lands.**
  Open / invite / first-post-approval / token-threshold (§7.1) are all variants
  of one mechanism, so the genesis record wants a `policy` field even while
  `open` is the only implemented value. Adding it in Phase 1 costs an enum with
  one variant; adding it later means migrating every Stoa already created.
