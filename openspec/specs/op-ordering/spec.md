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

The property the prohibition was defending is preserved **by construction and more strongly than before**: the counter is inside the signed preimage and the op id is a function of the op's own bytes, so **every input to this comparison travels with the op**. Two peers holding the same two ops compute the same order from the ops alone, consulting no local state, no arrival record, and nothing either peer received separately. The prior design could not say that — it depended on recorded arrival metadata, which is per-peer by construction and which the "seeing one op twice" problem exists because of. **The current time this system now reads does not change that.** It decides the counter an op is signed with and whether an arriving op is admitted, and it never enters a comparison between two ops.

**Ordering SHALL NOT consult the transport's Lamport timestamp or message id, where either is ever supplied.** A second order would be precisely the disagreement the original reasoning names, arrived at from the other side. The transport's clock and this system's counter both start from epoch milliseconds and take the same step on send, but the transport's clock also advances on traffic that no application sees. The two therefore advance on different sets of messages and cannot be relied on to agree op for op. Recorded arrival metadata MAY still be retained as a record of what a peer received; it SHALL NOT order.

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

A peer MUST have a Lamport clock for each Stoa. The clock's value MUST be the highest counter carried by the ops of that Stoa the peer holds, and MUST depend on those ops and on nothing else. It MUST be zero where the peer holds no ops of that Stoa, or holds only ops that carry no counter.

**The clock is derived on demand, never stored as an independent value.** A peer SHALL NOT persist a counter as a separate piece of state that a later read trusts in preference to the log, and SHALL NOT carry one in memory that a restart would reset differently from a recomputation.

This is what makes the clock monotone across a restart and across a rebuild-by-replay with no migration, recovery step, or high-water-mark record: the ops are the only input, so a peer that reopens its store, or reconstructs it entirely from ops it re-fetches, computes the value it had. A stored counter would be a second source of truth, and the two disagree in exactly the cases that matter — a store restored from a backup, a replay reaching further back than the counter, a crash between appending an op and updating the counter. In each, the stored value is the wrong one and the one a naive implementation would believe.

**What a reader can check, and what only an implementation can.** The scenarios below assert the *consequences* of deriving on demand — the value survives a restart, survives a rebuild in a different append sequence, and is the same on two peers that received the same ops in different sequences. They do not assert that no counter was recorded beside the ops, because no caller can observe that: a stored counter kept perfectly in step with the log is indistinguishable from a derivation through the read interface, and the cases where it would diverge (a restore from backup, a replay reaching further back, a crash between the append and the counter update) are not reachable through this contract's own operations. The "never stored" sentence above is therefore a **constraint on the implementation**, discharged structurally rather than by a scenario, and `design.md` carries how. Writing it as a scenario clause would be a requirement no test could distinguish from its negation.

**The clock is the highest counter held, with no exception for a counter far above the rest.** This clock previously excluded a counter more than a fixed bound above the rest, so that one absurd counter could not pull it to the ceiling. That exclusion is gone. The receive window, "An op whose counter is more than one hour ahead of this peer's time is refused on arrival", keeps such a counter out of the store altogether, so every counter the clock reads is one the peer admitted. **The clock does not read the current time.** The time enters when an op is signed, through "A published op takes the later of the current time and one above the author's clock". It does not enter the clock, so the clock remains a function of the ops held and nothing else.

The clock SHALL be scoped per Stoa. Ops of one Stoa never order against ops of another, so a shared clock would leak one Stoa's activity into another's counters, letting a reader in a quiet Stoa infer that the peer is busy elsewhere.

**Per-Stoa scoping bounds that leak but does not remove it within a Stoa, and this contract states the remainder rather than leaving it to be discovered.** A published counter is the later of its author's current time and one above its author's clock, and the clock is the highest counter the author holds. So whenever the author holds an op whose counter is at or above the author's current time, the counter on its next op **states the highest counter it held of that Stoa at the moment of publishing**. Anyone who can read ops of that Stoa can read it. That makes a peer's next published op an **oracle for whether that peer has received a particular op signed ahead of the time**. An observer publishes an op whose counter is ahead of the target's time, relays it to the target or withholds it, and reads the counter on the target's next post. One above the observer's op means it arrived, and the target's own time means it did not.

**This is inherent to a Lamport counter and is accepted, with the cost bounded as follows.** Pegging the counter to the time narrows the leak compared with a pure counter. An op signed at its author's own time says nothing about what its author received, so a probe needs an op signed ahead of the target's time, and the receive window caps how far ahead that can be. The leak never reveals *which* ops arrived beyond the probe the observer planted. It reveals nothing about Stoas the observer cannot read, which is what the scoping above buys. A counter at its author's own time reveals that author's clock reading, which the wall-clock field of an honest op already discloses. And it is a property of a peer that **publishes**: a peer that only reads emits no counter and is not probed by this.

**The alternative was rejected rather than overlooked.** Suppressing or fuzzing the counter would break the one property the counter exists for — that every peer computes the same order from the ops alone — since a counter a peer may distort is a counter two peers can disagree about. A forum that orders causally has to put the causal value on the wire. What follows for callers: a counter SHALL NOT be treated as private, and a peer SHALL NOT be told that publishing conceals what it has received.

#### Scenario: A peer holding no ops has a zero clock

- **WHEN** a peer's clock for a Stoa it holds no ops of is read
- **THEN** it is zero

#### Scenario: A peer holding only ops without a counter has a zero clock

- **WHEN** a peer holds ops of a Stoa, all encoded under the version predating the clock fields, and its clock for that Stoa is read
- **THEN** it is zero

#### Scenario: The clock reflects the highest counter it accepted

- **WHEN** a peer holds ops of one Stoa carrying several different counters, however far apart they are
- **THEN** its clock for that Stoa equals the highest of them

This scenario keeps its name, and "accepted" now means admitted to the store. Its condition used to require every counter to be within the advance bound. It now holds however far apart the counters are, because the clock excludes no counter the peer holds.

#### Scenario: A counter above the clock advances it

- **WHEN** an op carrying a counter above this peer's clock for its Stoa is stored
- **THEN** the peer's clock advances to that counter

#### Scenario: A lower counter never moves the clock backwards

- **WHEN** an op arrives carrying a counter below this peer's clock
- **THEN** the peer's clock is unchanged

#### Scenario: The clock survives a restart

- **WHEN** a peer's store is closed and reopened
- **THEN** its clock is the value it had before

#### Scenario: The clock survives a rebuild by replay

- **WHEN** a peer's store is discarded and rebuilt by appending the same ops in a different sequence
- **THEN** its clock is the same value

#### Scenario: A rebuild reaches the same answer as the original ingest

- **WHEN** a peer holding ops of a Stoa whose counters are far apart rebuilds its store from the same ops
- **THEN** its clock after the rebuild equals its clock before it
- **AND** both equal the highest counter among those ops

This scenario keeps its name. It used to fix the clock at the highest counter within the advance bound, below an over-bound op's counter. There is no advance bound now, so the clock is the highest counter held.

#### Scenario: Two peers receiving the same ops reach the same clock

- **WHEN** two peers receive the same set of ops in different sequences
- **THEN** both hold the same clock for that Stoa

#### Scenario: One Stoa's ops do not advance another Stoa's clock

- **WHEN** a peer holds ops of two Stoas carrying different counters
- **THEN** each Stoa's clock reflects only that Stoa's ops

### Requirement: The wall-clock decides nothing, and is handed out in a form that resists being sorted

The author-asserted wall-clock **field** SHALL NOT participate in any ordering, comparison, tiebreak, resolution or gating decision. No comparison of two ops SHALL read it, and no requirement in any capability SHALL be satisfied by consulting it.

**This requirement used to stand for a wider principle — that the author's claimed time decides nothing — and that principle is withdrawn.** The Lamport counter is now pegged to its author's clock: an honest author signs the later of its current time and one above its clock. So the counter carries the author's reading of the time, and that reading decides both where the op orders and, through the receive window, whether a peer admits it at all. What bounds the author's choice is no longer that time decides nothing. It is that the window refuses a counter more than one hour ahead of the receiving peer's own time, so an author can lead honest ops by at most one hour.

What remains of this requirement is the part that still holds and still matters: the **separate** wall-clock field orders nothing, gates nothing, and is handed out in a form that resists being sorted. Nothing checks it. The window reads the counter and never this field, so the field is bounded by nothing, and a value bounded by nothing must not decide anything.

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

**The receive window is not that promotion, and what separates them is the value each reads.** The window reads the counter, which orders, and refuses to admit an op whose counter is too far ahead. The clamp reads the wall-clock field, which orders nothing, and changes only how that field is shown. A peer with a badly skewed clock does pay a price under the window: it refuses honest ops, or has its own refused. That price is stated in the window's requirement rather than hidden here. The clamp adds nothing to it and MUST NOT be made to.

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

### Requirement: A published op takes the later of the current time and one above the author's clock

An op a peer publishes MUST carry a counter equal to the greater of two values: the peer's current Unix time in milliseconds, and one above the peer's clock for the op's Stoa at the moment of publishing.

This is the send rule of the SDS protocol (LIP-109), `max(timeNowInMs, current_lamport_timestamp + 1)`, applied to each Stoa separately. The clock it reads is the one the requirement "A peer's Lamport clock is a function of the ops it holds" defines, so the current time enters the counter here, when an op is signed, and nowhere in the clock itself.

**The counter is still a causality mechanism.** A peer holding an op at N publishes above N, whatever its own time says, so an op written by a peer that held another always carries the greater counter of the two. Every peer that receives both computes the same relation from the ops alone.

**Pegging the counter to the time lets a peer that holds nothing order correctly.** A peer that has received nothing of a Stoa publishes at its current time. That places its op among the Stoa's recent ops, where a pure counter would have signed one and placed it below every op the peer had not yet received.

**What a counter states and what it does not.** Short of saturation, a counter C states that its author held no op of that Stoa carrying C or more when it published, and that its author's current time was not after C. It does not state *which* op the author held, nor that the author held any op at C − 1, and a reader MUST NOT infer either. A scalar counter cannot carry that, and the contract does not pretend otherwise. **Detecting that an op refers to something the peer does not hold is a property of the ops themselves.** A reply names its parent op id inside the signed preimage, so "I hold a reply to X and no X" can be answered with no counter involved. That is the causal edge this forum acts on. The counter supplies an order, not a gap.

**Saturation, not overflow.** Where one above the clock would exceed the maximum representable counter, the value MUST saturate at the maximum rather than wrap. A wrapping counter would place the highest op below the lowest, inverting the order for every op in the Stoa at once.

**One reading of the time signs both clock fields.** A publish MUST take its current time once. The op's wall-clock field MUST carry the same value as the current time its counter was computed from, so an op whose author's clock was behind that time carries a counter equal to its wall-clock. The counter is computed from the time and not read from the field, so this gives the wall-clock field no part in any decision. It is what makes exact the clock requirement's statement that a counter at its author's own time discloses nothing the wall-clock field of an honest op does not.

#### Scenario: A first op in a Stoa carries the current time

- **WHEN** a peer holding no ops of a Stoa publishes into it
- **THEN** the op's counter equals the peer's current Unix time in milliseconds

#### Scenario: A clock behind the current time yields the current time

- **WHEN** a peer whose clock for a Stoa is more than one below its current Unix time in milliseconds publishes into that Stoa
- **THEN** the op's counter equals the peer's current time
- **AND** it is not one above the peer's clock

#### Scenario: A clock at or ahead of the current time yields one above it

- **WHEN** a peer holding an op of a Stoa whose counter is at or above the peer's current Unix time in milliseconds publishes into that Stoa
- **THEN** the op's counter is one above the highest counter the peer holds for that Stoa

#### Scenario: One reading of the time signs the counter and the wall-clock alike

- **WHEN** a peer whose clock for a Stoa is behind its current Unix time in milliseconds publishes into that Stoa
- **THEN** the op's counter equals that current time
- **AND** the op's wall-clock equals that same current time

#### Scenario: A counter taken from the clock leaves the wall-clock at the current time

- **WHEN** a peer whose clock for a Stoa is above its current Unix time in milliseconds publishes into that Stoa
- **THEN** the op's counter is one above that clock
- **AND** the op's wall-clock equals the peer's current time, not the counter

#### Scenario: Publishing after receiving advances past what was received

- **WHEN** a peer receives an op carrying a counter and then publishes its own
- **THEN** the published op's counter is greater than the received one

#### Scenario: A reply carries a greater counter than the post it replies to, and the rule places it first

- **WHEN** one peer publishes a post, and a second receives it and publishes a reply to it
- **THEN** the reply's counter is greater than the post's
- **AND** the ordering rule places the reply before the post, on every peer holding both

This scenario replaces "A reply orders after the post it replies to". Its last clause placed the post first, which contradicts the ordering rule, because the rule places the higher counter first. `thread-read` shows a thread's replies in the reverse of this rule, which is where a reply appears after the post it answers.

#### Scenario: A reply to an op signed ahead of the time still carries the greater counter

- **WHEN** a peer holds a post whose counter is ahead of the peer's current time, but within the receive window, and the peer publishes a reply to it
- **THEN** the reply's counter is greater than the post's

#### Scenario: Two successive publishes take successive counters

- **WHEN** a peer publishes two ops into one Stoa with nothing received between them
- **THEN** the second carries a counter greater than the first
- **AND** this holds when the peer's current time is the same at both publishes

#### Scenario: An op signed ahead of the time leads only until the time passes it

- **WHEN** one peer holds an op whose counter is ahead of the current time, and a second peer that does not hold it publishes once its own current time has passed that counter
- **THEN** the second peer's op carries the greater counter
- **AND** the ordering rule places the second peer's op first

#### Scenario: A publish at the maximum representable clock saturates

- **WHEN** a peer whose clock for a Stoa is the maximum representable counter publishes into that Stoa
- **THEN** publishing succeeds
- **AND** the op carries the maximum representable counter

### Requirement: An op whose counter is more than one hour ahead of this peer's time is refused on arrival

A peer MUST refuse an op arriving from the transport if the op's counter exceeds the peer's own current Unix time in milliseconds by more than the receive window, and MUST NOT store it. The window MUST be one hour, which is 3,600,000 milliseconds, and MUST be the same on every peer. This requirement MUST NOT refuse an op whose counter exceeds the peer's current time by the window exactly, or by less. The refusal is reported at the transport boundary, as `op-transport` contracts.

**There is no lower bound.** A peer MUST NOT refuse an op because its counter is below the peer's current time, however far below. History that reaches a peer late, through repair, sync or a snapshot of another peer's store, is in the past by definition, and a lower bound would refuse exactly that history.

**An op carrying no counter is not subject to the window.** Such an op was encoded under the version that predates the clock fields and carries nothing to compare. It MUST NOT be refused for lacking a counter, and the ordering rule places it below every op that carries one.

**A refusal leaves nothing behind that affects a later arrival.** A peer MUST NOT record a refusal in any form that changes how a later arrival of the same bytes is judged. Once the peer's current time has come within the window of an op's counter, the same op arriving again MUST be admitted exactly as an op arriving for the first time would be.

**The reference is this peer's own current time.** The window MUST be measured against the current time as this peer reads it. It MUST NOT be measured against the timestamp the transport hands in with a message, nor against any value the op carries other than its counter. The op's wall-clock field MUST NOT affect the decision.

**This replaces the advance bound, and it is a refusal on purpose.** The advance bound stored every counter and declined only to advance the clock past an implausible one. So the clock stayed below ops that were already ordered above it. An honest peer answering an op signed far ahead therefore signed a *lower* counter than the op it answered, and in a thread the answered reply came after every answer to it. The window removes such an op instead. Nothing more than one hour ahead is stored, so the clock can follow every counter held, and an answer always carries the greater counter. The most an author can lead honest ops by is one hour. A counter at the maximum representable value is refused; under the advance bound, the same counter bought its author the head of the order permanently.

**The cost is stated, not hidden.** This refuses an authentic op for the value of a field, which this system previously ruled out. A peer whose clock runs more than an hour fast has every op it publishes refused by every peer whose clock is right, until those peers' time catches up with its counters. A peer whose clock runs more than an hour slow refuses honest ops. Neither refusal reaches the author. One hour of tolerance is what that trade buys, and it is far larger than the skew of any clock that is synchronised at all.

**The comparison MUST NOT overflow.** A counter at the maximum representable value, or a current time at the maximum representable value, MUST NOT make the check wrap or abort. The check MUST reach the answer that the rule above gives.

#### Scenario: An op at the edge of the window is admitted

- **WHEN** an op arrives whose counter is exactly one hour ahead of this peer's current time
- **THEN** it is stored

#### Scenario: An op one millisecond beyond the window is refused

- **WHEN** an op arrives whose counter is one hour and one millisecond ahead of this peer's current time
- **THEN** it is refused
- **AND** it is not stored
- **AND** the peer's clock for that Stoa is unchanged

#### Scenario: A maximal counter is refused, and the peer can still publish

- **WHEN** an op carrying the maximum representable counter arrives at a peer whose current time is far below it, and the peer then publishes its own op into that Stoa
- **THEN** the arriving op is refused
- **AND** publishing succeeds
- **AND** the published op carries the later of the peer's current time and one above its unchanged clock

#### Scenario: An op far in the past is admitted

- **WHEN** ops carrying counters of zero and of one arrive at a peer whose current time is far above both
- **THEN** each is stored

#### Scenario: An op carrying no counter is admitted whatever the time

- **WHEN** an op encoded under the version predating the clock fields arrives
- **THEN** it is not refused on account of the window

#### Scenario: A refused op is admitted when it arrives again within the window

- **WHEN** an op is refused for being ahead of the window, and the same bytes arrive again once this peer's current time is within one hour of its counter
- **THEN** the op is stored
- **AND** the outcome is the one a first arrival of that op would have had

#### Scenario: The decision does not read the timestamp handed in with a message

- **WHEN** one op is received at one current time three times, with a far-past, a far-future and a zero timestamp handed in beside it
- **THEN** the admission decision is the same in all three cases

#### Scenario: The decision does not read the wall-clock field

- **WHEN** an op whose counter is within the window carries a wall-clock centuries ahead, and an op whose counter is beyond the window carries a wall-clock equal to this peer's current time
- **THEN** the first is stored
- **AND** the second is refused

#### Scenario: The window's value is pinned against silent drift

- **WHEN** the configured window is compared against a hardcoded 3,600,000 milliseconds written independently of it
- **THEN** they are equal
- **AND** a local edit to the configured window fails this rather than passing quietly

#### Scenario: Extreme values do not abort the check

- **WHEN** an op carrying the maximum representable counter arrives while this peer's current time is zero, and an op carrying a counter of zero arrives while this peer's current time is the maximum representable value
- **THEN** the first is refused
- **AND** the second is stored
- **AND** neither aborts the process
