# op-ordering Specification

## Purpose

Defines what orders two ops in a Stoa, what a receiving peer records alongside an op in order to answer that, and what order it produces when the transport supplies no ordering metadata.

The complementary half of this contract lives in the `op-format` capability, whose requirement "An op carries no ordering field and no per-peer state" states that an op SHALL NOT carry a Lamport timestamp or a transport message id, and that the omission is enforced by the encoding's length. That requirement is not restated here: this capability governs what a peer records *alongside* an op, and `op-format` governs what an op may contain. Two specs asserting one rule is how two copies drift and the wrong one gets read.

## Requirements

### Requirement: Ops are ordered by the counter they carry, not by anything the transport says

Ops SHALL be ordered by **descending Lamport counter as carried in the op's own signed bytes**, with ties broken by **ascending op id**.

**This requirement previously forbade exactly what it now requires, and the reversal is deliberate.** Its prior text read: *"A peer SHALL NOT compute a Lamport timestamp of its own, and SHALL NOT maintain a second logical clock alongside the transport's."* That prohibition is **withdrawn**.

The reasoning that produced it is preserved, because it is still the thing to defend against: *two orders over the same messages can disagree, and the disagreement produces no error — each peer stays internally consistent while rendering a thread differently from its neighbour.* That failure is exactly as bad as it was. What has changed is the answer to which single order is authoritative.

The prohibition assumed the transport's order was available to defer to. It is not, and the transport does not supply one: `op-transport` contracts that every arrival over the live transport carries no Lamport timestamp and no message id. So the prohibition's practical effect was never "use the transport's order instead of ours" — it was **no order at all**, with every consumer falling to the degraded path and resolving on a hash. A rule that forbids the only available order in favour of one that never arrives protects nothing.

The property the prohibition was defending is preserved **by construction and more strongly than before**: the counter is inside the signed preimage and the op id is a function of the op's own bytes, so **every input to this comparison travels with the op**. Two peers holding the same two ops compute the same order from the ops alone, consulting no local state, no arrival record, and nothing either peer received separately. The prior design could not say that — it depended on recorded arrival metadata, which is per-peer by construction and which the "seeing one op twice" problem exists because of.

**Ordering SHALL NOT consult the transport's Lamport timestamp or message id, where either is ever supplied.** A second order would be precisely the disagreement the original reasoning names, arrived at from the other side: the transport's clock advances on traffic no application sees and is initialised from epoch-milliseconds, so it can never agree with a counter advanced on ops. Recorded arrival metadata MAY still be retained as a record of what a peer received; it SHALL NOT order.

**The tiebreak is the op id and not the transport's message id.** The op id is a function of the op's own bytes, so every peer holding the op computes the same one without consulting anything it received; a message id is assigned by the transport, does not reach this system at all, and would be a tiebreak absent on every op — which is not a tiebreak.

#### Scenario: A higher Lamport timestamp is more recent

- **WHEN** two ops carry different Lamport counters in their own signed bytes
- **THEN** the op with the higher counter orders first
- **AND** the op ids do not affect the result
- **AND** the counter read is the op's own, not one recorded against its arrival

#### Scenario: Equal Lamport timestamps are broken by message id

- **WHEN** two ops carry equal Lamport counters and different op ids
- **THEN** the op with the lower op id orders first
- **AND** no transport message id is consulted, whether or not one was recorded

**This scenario keeps its original name and its tiebreak has changed.** It previously broke a tie by the transport's message id. That value does not reach this system on any op, so a tiebreak on it was a tiebreak that never fired; the op id replaces it and is strictly better for the purpose, being a function of the op's own bytes that every peer computes identically.

#### Scenario: The order is total

- **WHEN** any two ops with distinct op ids are compared
- **THEN** exactly one of the two orders first
- **AND** the comparison is consistent however the two are presented

#### Scenario: The order is transitive

- **WHEN** one op orders before a second, and that second orders before a third
- **THEN** the first orders before the third

#### Scenario: Every peer computes the same order

- **WHEN** two peers hold the same ops
- **THEN** both produce the same order
- **AND** neither consults its own clock, its arrival sequence, its recorded arrival metadata, or any other local state

#### Scenario: Transport metadata does not change the order

- **WHEN** two ops are ordered, then ordered again with differing transport Lamport timestamps and message ids recorded against them
- **THEN** the order is the same in both cases

#### Scenario: The order is derived from the ops alone

- **WHEN** the inputs the comparison reads are examined
- **THEN** every one of them is carried inside the ops being compared

### Requirement: Ops are deduplicated by op id before they are ordered

A caller SHALL deduplicate ops by op id before ordering them. The order is total over distinct op ids; two records of the same op id MAY compare as equal.

This is a contract on callers rather than an implementation detail, which is why it is stated here. Ops are idempotent by op id, so a store holding one record per op id satisfies it by construction — but a caller ordering a list assembled before deduplication would not, and the consequence is the failure this capability exists to prevent: a tie leaves the relative order of two records to the sort's stability and the order they were assembled in, which is arrival order, which differs from peer to peer.

**The same op received twice is the same op, and the clock fields do not change that.** Both fields are inside the preimage, so two arrivals of one op carry identical values for them — a peer cannot receive one op with two different counters, because that would be two ops with two ids. This is a stronger position than the prior design, where the recorded arrival could genuinely differ between two receipts of one op and a store had to choose between them.

#### Scenario: Two records of the same op may tie

- **WHEN** two records carrying the same op id are compared
- **THEN** the comparison may report neither as ordering first

#### Scenario: Distinct op ids never tie

- **WHEN** two ops with distinct op ids are compared, with any combination of counters and wall-clocks
- **THEN** exactly one of the two orders first

#### Scenario: One op received twice carries one counter

- **WHEN** the same op is received twice
- **THEN** both yield the same op id
- **AND** both carry the same counter

### Requirement: Absent transport metadata is represented, never fabricated

A peer SHALL be able to record that the transport supplied no Lamport timestamp or no message id for an op. It SHALL NOT substitute a local clock reading, an arrival counter, or a default value for a metadata value it did not receive.

A substituted value is indistinguishable from a received one once recorded, so a peer that substitutes cannot later tell which of its ops are genuinely ordered. Two peers substituting different local values also order the same pair of ops differently, with nothing to detect it.

Recorded metadata SHALL NOT affect the op it is recorded against: the same op received twice with differing metadata SHALL yield the same op id both times. This is what makes metadata safe to accept from an untrusted transport, and it is the boundary between this capability and `op-format`, which owns the op's own contents.

#### Scenario: Two arrivals of the same op are the same op

- **WHEN** the same op is received twice with different ordering metadata
- **THEN** both yield the same op id

#### Scenario: A missing Lamport timestamp is recorded as missing

- **WHEN** an op arrives with no Lamport timestamp
- **THEN** the recorded metadata reports the timestamp as absent
- **AND** no local clock reading is recorded in its place

#### Scenario: A missing message id is recorded as missing

- **WHEN** an op arrives with no message id
- **THEN** the recorded metadata reports the message id as absent

#### Scenario: Absence is distinguishable from a real value

- **WHEN** metadata with an absent Lamport timestamp is compared against metadata carrying one
- **THEN** the two are not equal
- **AND** the absent one is reported as unordered by the transport

### Requirement: An op carrying no counter sorts below every op that carries one

An op **carrying no Lamport counter** SHALL order after every op that carries one, whatever the values involved. Two such ops SHALL be ordered relative to each other by ascending op id.

**This requirement is retained and re-aimed, and that re-aiming is the whole of this change's migration answer.** Its prior condition was whether *the transport* had ordered an op; its condition is now whether *the op itself* carries a counter. An op carries no counter exactly when it was encoded under the version predating the clock fields, so the population this rule governs is the ops that already exist.

The consequence is that **already-stored content is neither reordered among itself nor dropped**. Ops predating this change continue to order against one another by ascending op id, exactly as they do today and with the same result; ops carrying a counter order among themselves by it; and the two populations are separated with the older one below. Nothing needs rewriting, no stored op changes, and no op is discarded for lacking a field its author's build could not have written.

**Why the older ops sort below rather than above.** An op carrying a counter is better evidence than one about which nothing is known, and the alternative would place every pre-existing op ahead of everything published afterwards — so a Stoa's feed would be permanently headed by its oldest content, and a post's current version could never advance past a revision predating the change.

This remains a defined degraded order rather than the ordering rule. Its purpose is that the comparison is total and identical on every peer even for ops that carry nothing to order by, so that two peers holding the same ops never disagree. Its purpose is not to approximate any temporal order, which it cannot do: an op id is a hash and carries no recency whatever.

#### Scenario: An unordered op sorts below an ordered one

- **WHEN** an op carrying no counter is compared against an op that carries one
- **THEN** the op carrying a counter orders first
- **AND** this holds regardless of how low that counter is

#### Scenario: Two unordered ops are ordered by op id

- **WHEN** two ops both carrying no counter are compared
- **THEN** the op with the lower op id orders first

#### Scenario: A caller can tell a degraded order from an ordered one

- **WHEN** an op is inspected
- **THEN** whether it carries a counter is reported
- **AND** a caller can act on that without inferring it from the ordering result
- **AND** the answer is a property of the op rather than of any arrival recorded against it

#### Scenario: Existing ops keep the relative order they already had

- **WHEN** a set of ops predating the clock fields is ordered before and after ops carrying counters are added alongside them
- **THEN** the relative order within the predating set is unchanged

#### Scenario: The degraded order is still total and peer-independent

- **WHEN** two peers hold the same ops, none of which carries a counter
- **THEN** both produce the same order

### Requirement: A peer's Lamport clock is a function of the ops it holds

A peer SHALL have a Lamport clock for each Stoa, and that clock's value SHALL be a function of the ops of that Stoa the peer holds and of nothing else. It SHALL be zero where the peer holds no ops of that Stoa.

**The clock is derived on demand, never stored as an independent value.** A peer SHALL NOT persist a counter as a separate piece of state that a later read trusts in preference to the log, and SHALL NOT carry one in memory that a restart would reset differently from a recomputation.

This is what makes the clock monotone across a restart and across a rebuild-by-replay with no migration, recovery step, or high-water-mark record: the ops are the only input, so a peer that reopens its store, or reconstructs it entirely from ops it re-fetches, computes the value it had. A stored counter would be a second source of truth, and the two disagree in exactly the cases that matter — a store restored from a backup, a replay reaching further back than the counter, a crash between appending an op and updating the counter. In each, the stored value is the wrong one and the one a naive implementation would believe.

**What a reader can check, and what only an implementation can.** The scenarios below assert the *consequences* of deriving on demand — the value survives a restart, survives a rebuild in a different append sequence, and lands in the same place after a rebuild as after the original ingest. They do not assert that no counter was recorded beside the ops, because no caller can observe that: a stored counter kept perfectly in step with the log is indistinguishable from a derivation through the read interface, and the cases where it would diverge (a restore from backup, a replay reaching further back, a crash between the append and the counter update) are not reachable through this contract's own operations. The "never stored" sentence above is therefore a **constraint on the implementation**, discharged structurally rather than by a scenario, and `design.md` carries how. Writing it as a scenario clause would be a requirement no test could distinguish from its negation.

**The function is not simply "the highest counter held", and the difference is the subject of the advance requirement below.** An op whose counter is implausibly far above the rest does not raise this peer's clock, so the clock is the highest counter the peer has **accepted as an advance** rather than the highest it has stored. Stating it as a function of the ops held is what keeps the derivation property intact: the accept-or-not decision is itself computed from the ops, so two peers holding the same ops still reach the same clock, and a replay reaches the same answer as the original ingest.

The clock SHALL be scoped per Stoa. Ops of one Stoa never order against ops of another, so a shared clock would leak one Stoa's activity into another's counters, letting a reader in a quiet Stoa infer that the peer is busy elsewhere.

**Per-Stoa scoping bounds that leak; it does not remove the leak within a Stoa, and this contract states the remainder rather than leaving it to be discovered.** A published counter is one above the author's clock, and the clock is a function of the ops held — so the counter on an op **states how many of that Stoa's ops its author had accepted as advances at the moment of publishing**. Anyone who can read ops of that Stoa can read it. That makes a peer's next published op an **oracle for that peer's reception state within the Stoa**, and an observer who controls what a peer receives can query it: publish *n* ops, relay only some to a target, and the counter on the target's next post distinguishes how many arrived. Two peers publishing identical content after receiving different subsets are distinguishable by this value alone.

**This is inherent to a Lamport counter and is accepted, with the cost bounded as follows.** It reveals a *count* of accepted advances and never *which* ops — the same limit the publish requirement below states for the positive claim — so it does not confirm receipt of any named op except where the observer has narrowed the possibilities by controlling delivery. It reveals nothing about Stoas the observer cannot read, which is what the scoping above buys. It is a property of a peer that **publishes**; a peer that only reads emits no counter and is not probed by this.

**The alternative was rejected rather than overlooked.** Suppressing or fuzzing the counter would break the one property the counter exists for — that every peer computes the same order from the ops alone — since a counter a peer may distort is a counter two peers can disagree about. A forum that orders causally has to put the causal value on the wire. What follows for callers: a counter SHALL NOT be treated as private, and a peer SHALL NOT be told that publishing conceals what it has received.

#### Scenario: A peer holding no ops has a zero clock

- **WHEN** a peer's clock for a Stoa it holds no ops of is read
- **THEN** it is zero

#### Scenario: The clock reflects the highest counter it accepted

- **WHEN** a peer holds ops of one Stoa carrying several different counters, all within the advance bound
- **THEN** its clock for that Stoa equals the highest of them

#### Scenario: The clock survives a restart

- **WHEN** a peer's store is closed and reopened
- **THEN** its clock is the value it had before

#### Scenario: The clock survives a rebuild by replay

- **WHEN** a peer's store is discarded and rebuilt by appending the same ops in a different sequence
- **THEN** its clock is the same value

#### Scenario: A rebuild reaches the same answer as the original ingest

- **WHEN** a peer that received an op exceeding the advance bound rebuilds its store from the same ops
- **THEN** its clock after the rebuild equals its clock before it
- **AND** both equal the highest counter within the bound, and neither equals the over-bound op's counter

#### Scenario: One Stoa's ops do not advance another Stoa's clock

- **WHEN** a peer holds ops of two Stoas carrying different counters
- **THEN** each Stoa's clock reflects only that Stoa's ops

### Requirement: A published op takes one above the highest counter the author has seen

An op a peer publishes SHALL carry a counter one greater than that peer's clock for the op's Stoa at the moment of publishing.

This is what makes the counter a causality mechanism rather than a per-peer sequence. A peer that has seen an op at N and then publishes carries N+1, which states that its op was written knowing of something at N. Every peer that receives both computes the same relation from the ops alone.

**What this states and what it does not.** It states *"the author had seen something at N"*. It does not state *which* op, and a reader SHALL NOT infer that a counter of N+1 means its author had seen any particular op at N. A scalar counter cannot carry that, and the contract does not pretend otherwise. **Detecting that an op refers to something the peer does not hold is a property of the ops themselves** — a reply names its parent op id inside the signed preimage, so "I hold a reply to X and no X" is answerable with no counter involved. That is the causal edge this forum acts on; the counter supplies an order, not a gap.

#### Scenario: A first op in a Stoa carries one

- **WHEN** a peer with no ops of a Stoa publishes into it
- **THEN** the op carries a counter of one

#### Scenario: Publishing after receiving advances past what was received

- **WHEN** a peer receives an op carrying a counter and then publishes its own
- **THEN** the published op's counter is greater than the received one

#### Scenario: A reply orders after the post it replies to

- **WHEN** one peer publishes a post, a second receives it and publishes a reply
- **THEN** the reply's counter is greater than the post's
- **AND** every peer holding both places the post first

#### Scenario: Two successive publishes take successive counters

- **WHEN** a peer publishes two ops into one Stoa with nothing received between them
- **THEN** the second carries a counter greater than the first

### Requirement: A received counter advances this peer's clock only within a bounded distance

A peer receiving an op whose counter exceeds its own clock for that Stoa SHALL advance its clock to that counter **only where the excess is within a fixed bound**. Where the excess is greater than that bound, the peer SHALL store and order the op normally and SHALL NOT advance its own clock.

The bound SHALL be a fixed constant, SHALL be the same on every peer, and SHALL be stated in the implementation as a named value rather than derived from a peer's history.

**The excess SHALL be measured against a value computed from the ops the peer holds, and SHALL NOT be measured against whatever the peer's clock happened to be at the instant the op arrived.** This is the difference between a rule that is a function of the op set and one that is a function of arrival order, and only the first is admissible: the same ops delivered in two sequences must yield one clock, or two peers disagree and a peer's own rebuild disagrees with itself.

The distinction is easy to lose because the arrival-order version is the one that falls out of writing the check on the receive path. A peer that received a long run of ordinary ops before a `u64::MAX` one would accept the jump as within the bound, while a peer that received the `u64::MAX` op first would refuse it — same ops, two clocks, no error anywhere. Deriving the reference value from the held ops removes the possibility rather than making it unlikely.

**The peer's own published ops are not exempt from the bound and do not need to be.** A peer publishes at one above its own clock, which is within any bound by construction.

**This is the answer to an author who signs `u64::MAX`, and it is deliberately not a refusal.** An op carrying an absurd counter is authentic, verifies, and is a genuine op its author published; refusing it would be refusing content for a field, which is the censorship vector the wall-clock requirement refuses for the same reason. What the attack must not be allowed to do is **drag every honest peer's clock to the ceiling**, because a peer whose clock is at `u64::MAX` can never publish again — its next op would need a counter above the maximum — and a single hostile op would silence an entire Stoa permanently for everyone who received it.

So the cost of the attack is bounded to what it cannot be denied: **the attacker's own op sits at the head of that Stoa's order.** It buys one position, in one Stoa, for one op, and it costs every honest peer nothing. A reader may still see it first; a reader may not be prevented from posting.

**Saturation, not overflow.** Where an advance or a publish would exceed the maximum representable counter, the value SHALL saturate rather than wrap. A wrapping counter would place the highest op below the lowest, inverting the order for every op in the Stoa at once.

#### Scenario: A counter just above this peer's clock advances it

- **WHEN** an op arrives carrying a counter one greater than this peer's clock
- **THEN** the peer's clock advances to that counter

#### Scenario: A counter within the bound advances the clock

- **WHEN** an op arrives carrying a counter above this peer's clock but within the bound
- **THEN** the peer's clock advances to that counter

#### Scenario: A maximal counter does not advance the clock

- **WHEN** an op carrying the maximum representable counter arrives at a peer whose clock is far below it
- **THEN** the op is stored
- **AND** the peer's clock is unchanged

#### Scenario: A peer can still publish after receiving a maximal counter

- **WHEN** a peer receives an op carrying the maximum representable counter and then publishes its own op
- **THEN** publishing succeeds
- **AND** the published op carries a counter derived from the peer's own unchanged clock

#### Scenario: An over-bound op still takes its place in the order

- **WHEN** an op whose counter exceeds the bound is stored alongside ordinary ops
- **THEN** it orders ahead of them by its counter
- **AND** ordering it did not require the peer's clock to have advanced

#### Scenario: A lower counter never moves the clock backwards

- **WHEN** an op arrives carrying a counter below this peer's clock
- **THEN** the peer's clock is unchanged

#### Scenario: Two peers receiving the same ops reach the same clock

- **WHEN** two peers receive the same set of ops in different sequences
- **THEN** both hold the same clock for that Stoa

### Requirement: The wall-clock decides nothing, and is handed out in a form that resists being sorted

The author-asserted wall-clock SHALL NOT participate in any ordering, comparison, tiebreak, resolution or gating decision. No comparison of two ops SHALL read it, and no requirement in any capability SHALL be satisfied by consulting it.

**Being display-only is a discipline the interface has to enforce, not a property the value has.** A number that means a time invites a sort, and a view that sorts on it produces a ranking every author can forge. The contract therefore constrains the shape it is handed out in, not only the uses it is put to:

- A read SHALL surface the wall-clock as **display text already formatted for rendering**, and SHALL NOT surface it as a bare number of milliseconds, seconds, or any other unit a comparison would accept.
- Every item carrying it SHALL carry **an explicit marker that the value is the author's claim**, so a view has the fact available at the point of rendering rather than needing to have read this contract.
- The **ordering position** a view needs SHALL be carried separately, as its own field, so a view placing items in order never needs to reach for the time. What that field promises is settled by the capability that carries it — `thread-read` contracts it as an item's place in the whole thread rather than as a sort key — and what this requirement fixes is only that it is not the time and is not derived from it.

A view that nonetheless wished to sort by the wall-clock would have to parse a display string back into an instant first. That is the intent: it makes the wrong thing visibly a wrong thing in the diff, rather than a plausible-looking field access.

**This is a measured failure mode rather than a hypothesis.** The nearest comparable project reads an author-asserted timestamp for ranking decay with nothing clamping it, so a post claiming a future time receives an unbounded multiplier and pins itself above every honest post. Its validator does notice future timestamps and produces a warning — which is never called on the ingest path. The defect is not that they lacked a check; it is that the value was available as a number to whoever wanted to rank by it.

#### Scenario: Two ops differing only in wall-clock order identically

- **WHEN** two ops sharing a counter differ only in their wall-clock values, one far in the future
- **THEN** the order between them is the same as if both carried the same wall-clock
- **AND** the future value confers no position

#### Scenario: A far-future wall-clock does not reach the head of the order

- **WHEN** an op carrying a wall-clock centuries in the future and a low counter is ordered against ops carrying higher counters
- **THEN** it orders after them

#### Scenario: The wall-clock reaches a reader as text and not as a number

- **WHEN** an item carrying a wall-clock is read
- **THEN** the value is display text
- **AND** no field of the item carries the instant as a bare number

#### Scenario: A reader is told the wall-clock is the author's claim

- **WHEN** an item carrying a wall-clock is read
- **THEN** it carries a marker identifying the value as author-asserted

#### Scenario: The ordering position is a separate field from the time

- **WHEN** an item is read
- **THEN** the field a caller uses to place it in order is distinct from the field carrying the time
- **AND** placing items in order requires no reading of the time

### Requirement: An implausible wall-clock is clamped for display and reported as clamped

Where an op's wall-clock is implausible against the reading peer's own clock — further in the future than a fixed allowance, or before a fixed floor — the value surfaced for display SHALL be clamped, and the reader SHALL be told that it was.

The allowance and the floor SHALL be fixed constants, and clamping SHALL be a **read-time presentation rule**. The stored op SHALL NOT be altered: the wall-clock is inside the signed preimage, so a peer rewriting it would hold an op that no longer verifies and whose id no longer matches.

**Clamping is defence in depth and not the defence.** The defence is that the value orders nothing — a clamp on a field that decided an ordering would still leave every op within the allowance reorderable by its author. What the clamp buys is that a reader is not shown "posted in 2387", which is a display defect rather than a ranking one, and that a peer whose own clock is wrong does not silently present other peers' honest times as absurd.

**The reading peer's own clock is itself untrusted for this**, and the contract does not pretend otherwise: a peer with a badly skewed clock clamps honest ops and passes hostile ones. That is acceptable **only because nothing depends on the outcome** — the clamp changes what is rendered and nothing else. This asymmetry is the reason clamping may not be promoted into an ordering rule later.

Because the clamp is computed against a local clock, two peers MAY present the same op differently, and a caller SHALL NOT treat the displayed time as a value two peers agree on.

#### Scenario: A far-future wall-clock is clamped for display

- **WHEN** an op whose wall-clock is far beyond the allowance is read
- **THEN** the displayed value is clamped
- **AND** the reader is told it was clamped

#### Scenario: A wall-clock before the floor is clamped

- **WHEN** an op whose wall-clock precedes the floor is read
- **THEN** the displayed value is clamped
- **AND** the reader is told it was clamped

#### Scenario: A plausible wall-clock is presented unclamped and reported as such

- **WHEN** an op whose wall-clock is within the allowance and above the floor is read
- **THEN** the displayed value derives from the author's value
- **AND** the reader is told it was not clamped

#### Scenario: Clamping does not alter the stored op

- **WHEN** an op with an implausible wall-clock is read and then read back from the log
- **THEN** the stored op is byte-identical to the one stored
- **AND** it still verifies

#### Scenario: Clamping never changes an order

- **WHEN** ops whose wall-clocks are clamped are ordered against ops whose wall-clocks are not
- **THEN** the order is the one their counters and op ids give
- **AND** it is unchanged by whether any value was clamped
