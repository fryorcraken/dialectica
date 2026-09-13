## Why

Every capability built so far assumes ops arrive from somewhere and says nothing about how. `op-log` stores what arrived, `op-ordering` orders what was recorded alongside it, `moderation-resolution` reads the result — and no contract says a Stoa's ops reach another peer at all. The MVP's "receive ops from other peers, over delivery's reliable channel" is the missing edge, and it is also the one boundary where hostile bytes enter the system.

Two properties make this worth a capability of its own rather than a paragraph in one that exists. The first is that the channel's identity must be identical on every peer in a Stoa: a value that differs per peer produces no error, it produces two sets of peers that never see each other, silently and permanently. The second is that this is the only place attacker-controlled bytes cross into the peer, so what is refused and how distinguishably each refusal is reported is a contract, not an implementation choice.

## What Changes

- A new `op-transport` capability: one reliable channel per Stoa, carrying nothing but signed ops.
- **Channel identity is a pure function of the Stoa address.** No per-peer state, no epoch, no session counter — the requirement is written as a rendezvous obligation because its failure mode is a silent partition rather than an error.
- **Publish**: an op the local peer authored is stored locally first, then its bytes go out on the Stoa's channel. Ordering between those two acts is contracted, because the reverse order loses an op the peer told its user it had published. Publishing to a Stoa with no open channel fails distinguishably and opens no channel — a publish is not how a peer comes to be in a Stoa.
- **Receive**: a validation boundary with an enumerated, individually distinguishable set of refusals — an unaddressed channel, a payload that does not decode, an op whose signature or author binding fails, an op naming a Stoa other than the channel's, an oversized payload.
- **What arrives alongside an op, and what must never be read from it.** The receive event supplies a sender identifier and a timestamp. Neither is a wire fact about the op: the sender identifier is an application-chosen transport string that must never reach an authorisation decision, and the timestamp is the receiving peer's own clock read, differing per peer for one message. The spec forbids deriving authorship from the first and any ordering from the second.
- **Arrivals are recorded as unordered**, because no Lamport value, message id or causal history reaches this layer. The degraded order that follows from that is `op-ordering`'s and is not restated.
- **Lifecycle**: the delivery node is shared and outlives dialectica, so dialectica never stops it; a channel is closed when a user leaves a Stoa and on shutdown, and closing is best-effort.

Not in this change: any core API method shape, Logos Storage and attachments, moderation publishing, Stoa discovery beyond what a channel needs, and the node's own configuration.

## Capabilities

### New Capabilities

- `op-transport`: how a Stoa's signed ops move between peers over one reliable channel — how the channel is identified, what publishing an op means, what an inbound payload must survive to be stored, and what the transport tells a receiving peer that it must not believe.

### Modified Capabilities

None, and two of those absences are deliberate decisions rather than nothing to report.

**`op-ordering` is not modified, and there is a live contradiction between it and PLAN.md that this change does not resolve.** `op-ordering`'s leading requirement states that "A peer SHALL NOT compute a Lamport timestamp of its own, and SHALL NOT maintain a second logical clock alongside the transport's." PLAN.md has since withdrawn that prohibition, on the reasoning that a dialectica-level clock never needed to agree with the transport's — only with other peers' dialectica clocks, and every peer sees the same ops. The spec and the plan therefore disagree today.

Resolving it is a change of its own, not a clause in this one, for a reason that is about scope rather than convenience: withdrawing the prohibition without supplying the replacement would leave `op-ordering` with a requirement that forbids nothing and requires nothing in its place, and the replacement is a design — an author-asserted clock value inside the signed op preimage needs a bound before it is an ordering at all, since the author sets it freely. PLAN.md names both adversarial cases and explicitly does not design the defence. A transport spec written on top of a half-withdrawn prohibition would make the contradiction permanent by burying it, which is why it is stated here instead.

**This change is neutral to how that resolves.** It contracts that this transport supplies no ordering metadata and that an arrival is therefore recorded as unordered — which is true whether or not a dialectica-level clock is later added, because such a clock would live inside the signed op rather than alongside it, and would reach a peer through `op-format`'s decoder rather than through the receive event's fields. Nothing in `op-transport` is made wrong by either resolution.

**`op-log` is not modified**, and the reason is worth stating because the two specs read as though they were in tension. `op-log` contracts that appending verifies nothing and stores an op whose signature does not verify; this change contracts that a payload whose signature does not verify is refused and never appended. Both hold at once, because they govern different acts: `op-log` says what the log does with an op it is handed, and `op-transport` says which inbound payloads become an op that is handed to it. A log deciding validity on append would be deciding it from whatever the peer knew at that instant; a transport boundary appending whatever arrived would be storing forgeries for any peer that sent them. The spec's Purpose names that boundary so the next reader does not have to derive it — the failure this project has already had once is two capabilities that were each internally consistent and had never been read beside one another.

## Impact

- A new capability with no existing spec to reconcile against, and no requirement removed from any capability.
- **Depends on a running delivery node**, which is why several properties in the spec are stated as scope exclusions rather than scenarios: channel rendezvous between two live peers, retransmission, and whether an unsubscribe reached the network cannot be observed without one. The spec says so in place rather than writing a scenario nobody can run.
- **Constrains lifecycle at the module boundary**: `createNode` is called once per context and dialectica never calls the node's `stop()`. Both are properties of a shared node, and getting either wrong takes delivery away from every other module in the same Logos Core instance.
- Names the boundary with the Stoa-lifecycle work in two places rather than crossing it: what a channel is opened *for* is a membership question, and the genesis record a receiving peer needs in order to know a Stoa is real is membership's to store.
- No change to the op format, to the op log, or to any resolver.
