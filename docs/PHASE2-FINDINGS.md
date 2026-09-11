# Phase 2 findings — the transport spike

Phase 2 is two sentences in PLAN.md §9: *"wire the real modules. Swap the fakes
for `delivery_module` channels and `storage_module`."* Both sentences rest on
§4.3's channel strategy, and §13 recorded the assumption underneath it as
untested:

> **Does #4116 survive a process restart?** §4.3's approach — never re-create a
> channel inside one node's lifetime — assumes persisted SDS state does not
> corrupt a fresh `createNode`.

This is what a spike against that question found. As in PHASE0-FINDINGS, what
builds today and what version anything is at are questions a command answers;
they are named rather than written down.

**Read §5 first if you are short of time.** It is what this spike could not
prove, and it is larger than what it could.

---

## 1. The ordering gap is real, and it is two layers lower than §13 placed it

**This is the spike's most certain result, and the only one established by
reading the exact revisions that run.** §13 concluded — by reading the LIDL
contract and the Reliable Channel API spec — that
`channelMessageReceived(channelId, senderId, payload, timestamp)` carries no
Lamport value and no SDS message id, and that *"the Reliable Channel API's
`MessageReceivedEvent` ... carries exactly one field — the reassembled
payload — so `channelMessageReceived` never receives them either."*

**The conclusion is right. The located cause is wrong, and the correction
matters for anyone filing the upstream bug.**

### The chain, verbatim, at the revision that runs

The delivery module pin is in `scaffold.toml` (`[modules.delivery_module]`);
the spike read that exact commit with `git show`, not the working tree, which
PLAN.md §12 warns sits on a pre-channels branch.

**Layer 1 — the SDS handler, where the fields are actually dropped.** The
deliverable SDS hands to the channel layer carries two fields:

```nim
  SdsDeliverable* = object
    ## Deliverable segment tagged with the sender id from the SDS envelope.
    content*: seq[byte]
    senderId*: SdsParticipantID
```

In `handleIncoming` (`logos_delivery/channels/scalable_data_sync/scalable_data_sync.nim`),
the fully deserialised SDS message `msg` is in scope and is demonstrably read
for its ordering fields — `msg.messageId` is the pending-content stash key on
line 223 — and then, on the very next line, only two of them are copied forward:

```nim
      self.pendingContent[msg.messageId] =
        SdsDeliverable(content: unwrapped.message, senderId: msg.senderId)
```

**The adjacency is the evidence.** `msg.messageId` is used as a key and
discarded as a value in one statement. The same two-field construction is
repeated for the immediate-delivery path on line 232. This is the single point
where ordering metadata is discarded.

**One link in this chain was not read directly, and it should be before the bug
is filed.** `msg`'s own type is nim-sds's `Message`, a nimble dependency pinned
in `nimble.lock` rather than vendored, and no realised copy was available to
read. That `msg` carries a `messageId` is proven by the line above; that it also
carries `lamportTimestamp` is inferred from the SDS persistency layer sorting
`SdsMessage` on `(lamportTimestamp, messageId)` (§4) and from the SDS spec, not
from the declaration. The conclusion does not turn on it — what reaches
dialectica is fixed by the three types below, whatever `Message` holds — but a
filing that claims "these fields are available and dropped here" should quote
the declaration.

**Layer 2 — the Reliable Channel API event.** By the time the event type is
built there is nothing left to drop:

```nim
  type ChannelMessageReceivedEvent* = object
    channelId*: ChannelId
    senderId*: SdsParticipantID
    payload*: seq[byte]
```

Three fields, not §13's "exactly one field — the reassembled payload". The
`channelId` is re-attached from `self`, not recovered from the wire.

**Layer 3 — the FFI event JSON.** A hand-written object literal with four keys:

```nim
      emitEvent("onChannelMessageReceived"):
        $(
          %*{
            "eventType": "channel_message_received",
            "channelId": string(event.channelId),
            "senderId": $event.senderId,
            "payload": string(base64.encode(event.payload)),
          }
        ),
```

Worth noting that this one is hand-rolled `%*{...}` rather than the generic
`$newJsonEvent(...)` flattener its sibling sent/error events use. The key set is
a literal, so no extra field can leak through even if a lower layer started
supplying one.

**Layer 4 — the delivery module.** `DeliveryModuleImpl::event_callback` parses
that JSON and reads exactly `channelId`, `senderId` and `payload`, base64-decoded.

### So the correction to §13 is this

§13 says the gap is *"a layer below the LIDL contract"* and that closing it
*"needs a change at both layers"* — the Reliable Channel API and the delivery
module. **That is one layer short.** The event type genuinely has no Lamport
field to forward, so a change to the delivery module and the RC event type would
forward a value that `SdsDeliverable` never carried. **Three types must change,
and the lowest is the one that matters**: `SdsDeliverable` is where the data
still exists and stops.

This does not change dialectica's design — §13's practical conclusion (ops are
recorded as unordered, with a defined degraded order) stands exactly as merged,
and `dialectica-core` already pins it (`op::tests::an_op_carries_no_ordering_fields`).
It changes **what to file, and against which repository.**

### One nuance that makes the gap sharper than "no metadata reaches us"

Ordering is not unenforced upstream — it is enforced and then made invisible.
`handleIncoming` parks segments whose causal dependencies are missing and
releases them in causal order, and the receive path emits in that order. So a
consumer gets correctly-ordered events **and no way to verify the order, detect
a gap, or re-sort after the fact.**

The cost of that is concrete: the pending-content stash is capped at
`MaxPendingContent = 32`, and on overflow the oldest pending entry is evicted
with only a log line —

```nim
        self.pendingContent.del(oldest)
        warn "SDS pending-content stash full, dropping oldest entry",
          channelId = self.channelId, dropped = oldest
```

**To a consumer that is a message which never arrives, with no ordering field
that could reveal the hole.** Note what the eviction picks, too: the first key
the table iteration yields, which in a hash table is not the oldest by any
causal or temporal measure despite the message saying "oldest". Any future dialectica-side
completeness check has nothing at this layer to key on — which is an argument
for the §13 filing, not merely a curiosity.

---

## 2. `timestamp` is a local clock read — confirmed, and the cited line has moved

§13 records that `channelMessageReceived`'s `timestamp` *"is the receiving
peer's own `CLOCK_REALTIME` read taken when its callback fires"*, citing
`plugin.cpp:158`.

**The claim is exactly right. The citation is stale** — at the pinned rev the
call is at `src/delivery_module_plugin.cpp:212`, and the helper is:

```cpp
int64_t currentTimestampNs() {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return static_cast<int64_t>(ts.tv_sec) * 1000000000LL + static_cast<int64_t>(ts.tv_nsec);
}
```

`event_callback` takes `int64_t timestamp = currentTimestampNs();` **once, at
the top, before dispatching on event type**, and hands that same value to every
event it forwards.

Two consequences worth carrying, neither visible from the LIDL contract:

- **The channel event JSON carries no timestamp at all** (§1, layer 3). So this
  is not the module choosing a local read over message metadata — there is no
  message metadata to prefer. The module synthesises the only timestamp that
  exists.
- **`messageReceived` is the exception, and it is the one §11 already flags.**
  That branch alone reads `msgObj.value("timestamp", 0.0)` off the wire rather
  than using the synthesised value — which is why its units differ (§11,
  delivery bug #26). The units trap and the local-clock trap are therefore the
  *same* divergence seen from two ends: exactly one event carries a real
  message timestamp, and it is not the channel one.

**A citation-shaped lesson.** §13's line number was right when written and is
wrong now, while the sentence around it stayed true. Cite the function and the
pin, not the line.

---

## 3. What a real `channelMessageReceived` carries

Stated as the wire shape a Phase 2 ingest path must decode, so nothing has to
re-derive it:

| Field | Type at LIDL | What it actually is |
|---|---|---|
| `channelId` | `tstr` | the channel's own id, re-attached from local state, not from the wire |
| `senderId` | `tstr` | the SDS participant id from the envelope — the one field that does cross the network |
| `payload` | `bstr` | the reassembled application bytes; base64 over the FFI, decoded by the module |
| `timestamp` | `int` | the **receiver's** `CLOCK_REALTIME` in nanoseconds at callback time (§2) |

And, on the SDK side, an event arrives as
`EventData { event: String, data: serde_json::Value }`, where `data` is the
positional argument array — so a consumer indexes rather than reads by name.

**Nothing else reaches us.** No Lamport timestamp, no SDS message id, no causal
history, no bloom filter, no retrieval hint. `setRetrievalHint` is explicitly a
no-op upstream — persisted hints are never read back — so it is not merely
unforwarded, it is unstored.

---

## 4. SDS state is genuinely persisted, and where

Relevant because it is the precondition that makes the #4116 restart question
meaningful at all: if nothing survived a restart there would be nothing to
corrupt a fresh `createNode`.

**It survives, and the module chooses where.** `applyConfigDefaults` in the
delivery module defaults the node's storage directory to the host-supplied
per-instance persistence path:

```cpp
        if (target && !findKey(*target, {"localstoragepath", "local-storage-path"})) {
            (*target)["localStoragePath"] = persistencePath + "/data";
        }
```

with a comment naming the reason — *"so side-by-side instances don't share
upstream's cwd-relative `./data`"*. That is what makes two profiles on one
machine independent, and it is the hook a two-profile test depends on.

What is stored under it, per the SDS persistency layer:

| Category | Key | Value |
|---|---|---|
| `sds.meta` | `key(channelId)` | `ChannelMeta` |
| `sds.log` | `key(channelId, msgId)` | `SdsMessage` |

and `loadChannel` reconstructs history in memory by sorting on
`(lamportTimestamp, messageId)` — **§5.7's rule exactly, applied upstream, on
data we cannot see.** The Lamport value is not merely computed and dropped in
flight; it is durably stored, keyed, and re-sorted on load. It stops at the
`SdsDeliverable` boundary (§1) and nowhere earlier.

Two things follow for Phase 2:

- A restart does **not** start from a blank channel. `channelCreate` with a
  known id restores meta and history — which is what the delivery contract's
  own `channelCreate` docstring means by *"Persisted channel state survives
  `channelClose`, so re-creating a channel with the same id restores it."*
- **That docstring is the one §4.3 warns is the trap.** It describes
  close-then-recreate as a supported operation, and #4116 is the report that
  doing so inside one node's lifetime kills the node process. The documentation
  and the bug report contradict each other, and §4.3's whole shape exists
  because of it. Nothing in this spike changes that; see §5.

---

## 5. What this spike did NOT prove — including its primary question

**The primary question is unanswered.** Stated plainly, because a findings
document that omits its own gaps is worse than none (PHASE0-FINDINGS §7 is the
model).

### The #4116 restart question: not settled

**No delivery node was created, no channel was opened, and no message was sent
or received.** Everything in §1–§4 above is established by reading the exact
pinned revisions of the sources that run — which is strong evidence about
*what the code does*, and no evidence at all about what two live nodes do across
a restart.

So **§4.3 is neither confirmed nor refuted by this spike.** Its "never re-create
a channel inside one node's lifetime" rule stands as the plan's choice, still
resting on the untested assumption §13 named. Treat §13's entry as open.

**Where it stopped, precisely.** The spike got as far as:

- `lgs basecamp build --variant all` — green, both `lgx` and `lgx-portable`.
- `lgs basecamp modules` — captured `delivery_module` at the pinned rev.
- `lgs basecamp install` — reported `installing 3 module(s) (deps first)`,
  which is §11's trap correctly avoided. All three built (delivery took 8m42s
  once its ~380 dependencies were cached). **Delivery and the core both
  installed into the `alice` profile. The UI install failed** on
  `Forbidden root entry: assets` — §5b, which is a live defect in the pin
  configuration rather than anything about this spike.

So the experiment stopped **one step short of a launch**, with two of three
modules installed and the third blocked by a packaging bug. Nothing about the
transport was reached.

### The blocker a re-run must plan around, and it is not the build

**Dialectica's module contract has no way to reach any of the calls the
question needs.** The trait in `dialectica/rust-lib/src/lib.rs` exposes
`version`, `ping`, `panic_probe` and `delivery_channel_exists` — and
`channelExists` is deliberately *"the cheapest delivery call with no side effect
and no node required"*. There is no `createNode`, no `start`, no `channelCreate`,
no `channelSend`, no `channelClose`, and no subscription to
`channelMessageReceived`.

So answering #4116 is **not** a matter of clicking buttons in a build that
already exists. It requires adding scaffolding methods to the contract trait —
which is a change to the deliverable API (CLAUDE.md: *"the core API is the
deliverable"*) even when the methods are labelled temporary, because
`codegen.rust.trait` derives the published `.lidl` from that trait. Phase 0 set
the precedent for how to do this (`panic_probe` is contract surface carrying a
"remove it once the question is settled" comment), so the path is known; it is
just larger than a spike's "write as little code as possible" and should be
budgeted as such.

**A second reason the harness is not trivial**, and it is the more interesting
one: the question is about a **process restart**, so the apparatus must survive
one. A probe that opens a channel, receives peer traffic, and then reports is a
single-run probe. What #4116 needs is a probe whose *second* run, against
persisted state written by the first, is the measurement — with a real peer
having sent something in between. That is two profiles, two launches each, and
a way to tell "the node came up clean" from "the node came up and died", which
per PHASE0-FINDINGS §9 means reading `launch.state` and the profile log rather
than trusting a tile.

### Also not proven, from PHASE0-FINDINGS §7's list

- **Two peers exchanging anything.** Both profiles exist and both are
  configured with a real `runtime_dir`; neither was launched in this spike.
- **That the ordering gap holds at runtime.** §1 is a source reading. The
  empirical confirmation the brief asked for — what a *real* received message
  carries — was not taken, and it remains the cheapest high-value measurement
  available the moment a node runs. If a Lamport value did somehow reach a
  consumer it would contradict §1, and §1 is what would be wrong.

---

## 5b. `[repos.lgpm]` is not the lgpm that runs — and it is what blocked this spike

**This is where the spike actually stopped, and it is a live defect rather than
a budget problem.** Everything built. Delivery and the core both *installed*.
The UI did not:

```
error: lgpm install .../logos-dialectica_ui-module.lgx into alice failed:
  Error: Package validation failed: Forbidden root entry: assets
```

That is verbatim the failure `scaffold.toml`'s `[repos.lgpm]` comment says the
pin exists to avoid, and which PHASE0-FINDINGS §8 records as a deliberate,
working split. **It is not working.**

### The pin is ignored

The lgpm the install invokes reports:

```
lgpm version pre-release-202af6f (dev build)
commit: 202af6fa0f0f4493bc59c8a609dff9326f78a18d
```

`scaffold.toml` pins `[repos.lgpm]` to `d3af2972f51d9c542537d80d60ad8d20282ddcd1`.
**Different commits.** The binary's path says why:

```
~/.cache/logos-scaffold/basecamp/<BASECAMP-COMMIT>/lgpm-result/bin/lgpm
```

The cache is keyed by the **basecamp** commit, and `lgpm-result` sits *inside*
that directory beside `app-result`. There is no lgpm-pin-keyed directory
anywhere in the cache. So the lgpm that runs is the one **basecamp's own flake
supplies**, and `[repos.lgpm]` selects nothing.

### Why this matters more than one failed install

PHASE0-FINDINGS §8 frames the basecamp/lgpm split as a deliberate trade — *"a
split pair that installs and runs, or a matched pair that cannot install the UI
at all"* — and says it is tolerable because *"the split is on the read side and
we have exercised it"*. The evidence for "exercised" was a Phase 0 run that
installed all three modules.

**That reasoning no longer holds, because the mechanism it assumed does not
exist.** The pin was never what made Phase 0's install work. Whatever did (a
cache populated when basecamp's embedded lgpm happened to be older or newer, or
a hand-run install) is not reproducible from the checked-in configuration — a
fresh cache gets basecamp's lgpm and fails. The `doctor` WARN about the split
pins is therefore pointing at a real problem while describing the wrong
mechanism: there is no split *to choose*, because only one of the two pins is
consulted.

**What this does not settle:** whether `lgs` ignores `[repos.lgpm]` by design
(basecamp embeds lgpm, so overriding it separately may be meaningless) or
whether the pin is meant to work and does not. That is one question for the
scaffold maintainers, and it should be asked before anyone else spends a build
cycle on the pin.

### The consequence for the #4116 experiment

The UI is not installable from a clean cache, so `lgs basecamp launch` has no
plugin to click. Note what this does *not* block: **the core and delivery
modules both installed** into the `alice` profile.

The obvious way round is to drive the module over IPC rather than through the
view, as PHASE0-FINDINGS §3 did with `logoscore call dialectica panic_probe`.
**Check before relying on it: there is no `logoscore` on this machine** — not on
`PATH`, and not in the basecamp bundle, whose `bin/` holds only
`LogosBasecamp`, `ui-host` and `logos_host`. So §3's apparatus is not simply
sitting there waiting; where that `logoscore` came from is itself an open
question, and answering it is a prerequisite for the IPC route rather than a
detail of it.

That leaves two candidate paths for a re-run, neither free: fix or bypass the
lgpm defect so the UI installs and buttons can be clicked, or find/build the
CLI that talks to a running core. **The second is still the better bet** — it
avoids depending on a QML view for an experiment about transport state, and it
is the one that can be scripted across the two launches a restart test needs.

---

## 6. New traps, in §11's style

- **`lgs basecamp build` and its siblings act on the current directory and take
  no `--directory` flag.** `lgs basecamp build --help` confirms the only options
  are `--quiet`, `--variant` and `--module`. An agent or script working in a git
  worktree therefore cannot point `lgs` at that worktree without a `cd`, which
  this repo's permission setup refuses to analyse (CLAUDE.md). The practical
  consequence: **a worktree's code is not what gets built** — the main checkout
  is, whatever revision it happens to sit on. Check it with
  `git -C <main-checkout> status --branch` before believing a build exercised
  your change. In this spike the main checkout was 8 commits behind
  `origin/main`, which was harmless only because the spike touched no core code.

- **`lgs basecamp modules` strips `scaffold.toml`'s comments, confirmed again.**
  PHASE0-FINDINGS §4 and §11 both say so; this spike watched it happen —
  `git diff --stat scaffold.toml` reported 77 deletions and no insertions
  immediately after the verb. The value of a third sighting is that it is now
  clearly *every* run, not an occasional one: budget the restore as a step, not
  a contingency.

- **`liblogosdelivery` is built from source because nix distrusts the flake's
  binary cache**, and the message says so only as a warning you will scroll
  past:

  ```
  warning: ignoring untrusted flake configuration setting 'extra-substituters'.
  Pass '--accept-flake-config' to trust it
  ```

  The delivery flake ships `extra-substituters` and `extra-trusted-public-keys`
  pointing at a cache that would supply the whole Nim/Waku/zerokit stack
  prebuilt. A non-trusted nix user cannot accept those settings silently, so
  they are dropped and every derivation is rebuilt locally. **This converts a
  download into a multi-hour compile**, and nothing about it presents as an
  error. Anyone budgeting this experiment should decide deliberately whether to
  pass `--accept-flake-config` (a trust decision about that cache's keys, not a
  formality) or to plan for the from-source build.

- **Run `lgs basecamp install` detached, not in a foreground call with a
  timeout.** Its delivery build outlasts any reasonable command timeout, and
  when the harness times the call out it kills the `nix build` child with it —
  the partially-built derivation is discarded and the next run restarts that
  derivation from the beginning. Nix's own caching protects the ~380
  dependencies but not the one in flight, so a timeout near the end of
  `liblogosdelivery` is the most expensive moment to hit. Start it in the
  background and poll the log at the path the command prints.

- **A stale line-number citation outlives the claim it supports.** §13 cited
  `plugin.cpp:158` for a fact that is now at line 212 (§2). The sentence stayed
  true while the pointer rotted, which is the failure mode CLAUDE.md's
  "self-invalidating" rule is about: cite the **function name and the pin**, so
  a reader who cannot find it knows the pin moved rather than concluding the
  claim was wrong.
