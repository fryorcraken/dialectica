## Why

Issue #162 found two faults in the ordering contract. First, `op-ordering`
contradicts itself: the scenario "A reply orders after the post it replies to"
says the post comes first, but the ordering rule it sits under puts the higher
counter first, and the reply carries the higher counter. Second, `ADVANCE_BOUND`
lets an author pick a counter that no honest clock will follow. Such an op is
stored and sorted by its counter. Every honest answer to it then carries a
**lower** counter, so since #159 the author of an over-bound reply is placed
after every answer to it in the thread.

The owner decided on 2026-09-25, in
<https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926>,
to peg the counter to wall-clock time as SDS does. The definition followed is
LIP-109 (`logos-lips/docs/anoncomms/raw/sds.md`). Its *Participant state*
section (lines 148–155) starts the Lamport timestamp at the current epoch time
in milliseconds, so that a new joiner orders correctly against recent messages
without first syncing past ones. Its *Send Message* step (lines 184–192) sets
the timestamp to `max(timeNowInMs, current_lamport_timestamp + 1)`. A receive
window of one hour replaces `ADVANCE_BOUND` as the defence against a counter
chosen to jump the order.

## What Changes

- **Publish.** A peer signs each op with the later of its current Unix time in
  milliseconds and one above its clock for that Stoa, saturating at the maximum.
  The clock stays a function of the ops held, and zero when none are held. The
  current time enters only at publish and at receive.
- **Clock.** The clock is the highest counter the peer holds for that Stoa. The
  ascending fold that stopped at the advance bound is gone.
- **Receive.** An inbound op whose counter is **more than one hour ahead of the
  receiving peer's own current time** is refused at the transport boundary and
  is not stored. The refusal is reported distinguishably from the five that
  exist today. There is **no lower bound**: an op in the past is accepted,
  however old. A refused op that arrives again once it is within the window is
  accepted normally, because a refusal leaves nothing behind. An op carrying no
  counter (the version before the clock fields) is not subject to the window.
  The reference time is the peer's own clock, not the timestamp the transport
  hands in with a message.
- **BREAKING (behaviour):** `ADVANCE_BOUND` and the requirement "A received
  counter advances this peer's clock only within a bounded distance" are
  removed. An op carrying the maximum representable counter used to be stored
  and placed at the head of the order. It is now refused.
- **Issue item 1.** The `op-ordering` scenario becomes "A reply carries a
  greater counter than the post it replies to, and the rule places it first".
  It agrees with the rule and is pinned by a test.
- **Issue item 3.** `thread-read`'s position sentence now says the position is
  the item's place in the sequence the read returns, not "the position the
  ordering rule gives it". Its advance-bound clause goes too: with the window in
  place, a reply that follows the publish rule always carries a greater counter
  than the reply it answers.
- **Worth-checking point.** An op signed ahead of the clock (at most one hour
  ahead) leads only the ops published by peers that do not hold it, and only
  until their clocks pass its counter. This is specified once in `op-ordering`,
  because every reader that takes the rule's first entry inherits it. The
  analysis for each reader is below, for `design.md`.
- **Correcting text elsewhere.** Several capabilities give "the ordering carries
  no time" or "a counter says nothing about wall-clock time" as the *reason* for
  a rule. Each is rewritten so that the old reason does not remain as a false
  statement. None of those rules changes except the ones listed above.

### The recorded rules this change reverses

The owner's decision reverses two rules on purpose and replaces one mechanism.
Each is listed with where it is recorded now and with the reason.

**1. No op is refused for a field value.** Recorded in:

- `op-ordering`, "A received counter advances this peer's clock only within a
  bounded distance": *"deliberately not a refusal … refusing it would be refusing
  content for a field, which is the censorship vector the wall-clock requirement
  refuses for the same reason"*. **Removed.**
- `op-format`, "An implausible wall-clock is accepted at the boundary, never
  refused": *"A forum that is censorship-resistant by design cannot have a rule
  that drops an author for a clock."* **Modified.** The wall-clock *field* is
  still never refused. The rationale now states that the counter is refused,
  and explains why.
- `op-transport`, "This capability decides nothing about an op beyond admitting
  it", where validation answers only *"whether the bytes are a well-formed,
  authentic op addressed to this channel's Stoa, and nothing further"*.
  **Modified**, and the window joins the list of refusals in "Every inbound
  payload is validated before it reaches storage".
- `openspec/changes/archive/2026-09-16-op-clock/design.md`, decision 11 and the
  Context paragraph *"A field a malicious peer sets freely is not an ordering
  until it has a bound"*. Archived, so not edited. `design.md` supersedes it.

*Why it is reversed.* Refusing nothing left an author free to sign any counter.
The advance bound kept that counter from dragging honest clocks along, but it
did so by keeping the clock below ops that were already ordered above it, which
is what put answers below the ops they answered. The window makes such a counter
impossible to store. **The cost is the one `op-format` warned about, and it is
accepted rather than denied:** if a peer's clock runs more than an hour fast,
every honest peer refuses its ops until their own time catches up, and if a
peer's clock runs more than an hour slow, it refuses honest ops. One hour of
tolerance is what that trade buys.

**2. The author's claimed time decides nothing.** Recorded in:

- `op-ordering`, "The wall-clock decides nothing, and is handed out in a form
  that resists being sorted". **Modified.** The wall-clock field still decides
  nothing and is still handed out as text. The requirement now says that the
  *counter* carries the author's reading of the time and decides both order and
  admission.
- `op-format`, "An op carries no ordering field and no per-peer state", where
  the paragraph *"a Lamport counter is meaningful only relative to ops a peer
  has seen … A wall-clock is an absolute claim about the world, which a peer has
  nothing to check against. That is why one may order and the other may not"*
  no longer holds for the counter. **Modified.**
- `op-ordering`, "An implausible wall-clock is clamped for display and reported
  as clamped": *"acceptable only because nothing depends on the outcome"*. The
  reading peer's own clock now decides admission. **Modified** to say why that
  does not promote the display clamp into a rule.
- `feed-view`, "The ordering label claims a position …": *"the ordering carries
  no instants"*. `post-revision`, "The current version is the one the ordering
  rule places first": *"It says nothing about wall-clock time"*. `thread-read`:
  *"A counter says its author had seen something at the counter below it"*, in
  two requirements. `op-ordering`: the publish requirement's *"carries N+1"*,
  the clock requirement's reception-state oracle, and the first requirement's
  *"initialised from epoch-milliseconds, so it can never agree"*. Each is
  **modified** so that it states what a pegged counter says.
- `openspec/changes/archive/2026-09-16-op-clock/design.md`, Context (*"an
  author-asserted wall-clock that decides nothing"*) and decision 8 (*"not
  chooseable for rank — the advance bound is what makes the second true"*).
  Archived, so not edited.

*Why it is reversed.* A counter pegged to the author's clock is the author's
claim about the time, so the principle cannot survive in its general form. What
remains true, and what the modified text keeps, is that the **separate
wall-clock field** decides nothing and cannot be sorted on.

**3. `ADVANCE_BOUND` is replaced.** Recorded in `op-ordering`'s removed
requirement, in the clock requirement's scenarios that named "the advance bound",
in `thread-read`'s sequence requirement, and in archived `op-clock/design.md`
decisions 3 and 4. The one-hour window takes over its job: the most an author
can lead honest ops by is one hour.

### Notes for the dev-writer, to carry into `design.md`

These are reasoning, not behaviour. Each belongs in a Decisions entry.

- **What the one-hour window allows against each reader of the rule's first
  entry.** Each reader that takes the first entry is exposed only to an op
  signed ahead of wall time. That op leads the ops published by peers that do
  not hold it, until their time passes its counter. Ops published by peers that
  do hold it carry a greater counter and lead it.
  - `revision::current_version` considers only the post author's own versions,
    so the lead is over the author's other versions, from a device that had not
    yet received the ahead-signed one, for up to an hour. No third party can
    use it. Before this change, an author who signed `u64::MAX` fixed that
    version as current permanently, and the author's own later revisions (one
    above an unadvanced clock) could never displace it.
  - `moderation::resolve` filters for authority first, so only a moderator can
    use this. A moderator can sign a `hide` or `unhide` up to an hour ahead and
    beat an opposite action that another moderator published within that hour
    without having received it. A correction published after receiving it wins.
    Before this change, `u64::MAX` won a dispute between moderators permanently.
  - The feed's `latestReply`: any author can make their reply the thread's
    latest for up to an hour against replies from peers that do not hold it.
    Before this change, `u64::MAX` held that position permanently.
- **Passages from the issue worth keeping.** Issue item 2's diagnosis (why an
  over-bound counter places an author after every answer to them). The archived
  `2026-09-24-thread-reply-order/design.md` Risks case *"An over-bound parent"*,
  which this change closes. The owner comment's superseded options: (1) write
  down the current behaviour, (2) demote over-bound ops, (3) refuse over-bound
  ops by arrival order. These are the rejected alternatives.
- **Where "now" comes from.** `dialectica-core` reads no clock (archived
  `op-clock/design.md` decision 6), so the current time must be passed into both
  the publish path and the receive path. The spec forbids using the timestamp
  the transport hands in with a message as the reference. That timestamp is also
  nanoseconds where every other delivery event is ISO-8601 (delivery bug #26).
  The spec requires publish to compute the counter from one reading of the time
  and to sign that same reading as the wall-clock field. How that reading
  reaches `core` is the design's.
- **Overflow.** Neither the window comparison nor `max(now, clock + 1)` may
  overflow or panic, because a panic on the receive path aborts the module
  process. The SQLite log stores counters in `score_epoch` through an `as i64`
  cast, so `u64::MAX` is stored as `-1`. If "highest counter held" is computed
  in SQL (for example with `MAX(score_epoch)`), counters of 2^63 or more sort
  below small ones. The current code avoids this by reading the rows back into
  `u64` before folding them.
- **Stored ops with small counters** (signed before this change) order below
  every op published after it. The owner ruled that no migration is needed,
  because nothing has been released.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `op-ordering`: the publish rule becomes `max(now, clock + 1)`, with the
  wall-clock field signed from the same reading of the time. The clock
  becomes the highest counter held. The advance-bound requirement is removed,
  and a one-hour receive window is added. The reply scenario is corrected. The
  "wall-clock decides nothing" and display-clamp requirements are narrowed to
  the wall-clock field, and they state the reversal. The first requirement's
  reason for rejecting the transport clock is corrected.
- `op-transport`: the window refusal joins the inbound validation list, and the
  "decides nothing beyond admitting it" requirement names it. An op already held
  that arrives again beyond the window is refused like any other arrival.
- `op-format`: the rationale for admitting the counter, and for never refusing
  an implausible wall-clock, is rewritten. It no longer rests on "a counter is
  not a claim about time" or "no op is dropped for a clock".
- `thread-read`: the position phrase is reworded to the read's sequence. The
  advance-bound clause and the "seen something at the counter below it" wording
  are replaced.
- `feed-view`: the reason "the ordering carries no instants" is corrected. The
  prohibition on "most recent first" is unchanged.
- `post-revision`: "It says nothing about wall-clock time" is corrected. The
  prohibition on calling the current version the most recent by any clock is
  unchanged.

## Impact

- **Code:** `dialectica-core/src/arrival.rs` (`ADVANCE_BOUND`,
  `clock_from_counters`, `next_counter`), `authoring.rs` (the publish path needs
  the current time), `transport.rs` (`receive` needs the current time and a new
  refusal), `log/sqlite.rs` (the clock derivation), and `wire.rs` (where the
  host's time is sampled). Doc comments that cite `ADVANCE_BOUND` in `op.rs`,
  `asserted_time.rs` and `thread.rs` also change.
- **Wire format:** unchanged. The counter keeps its width, and the separate
  wall-clock field stays.
- **Out of scope, per the owner's decision:** changing the op format, merging
  the counter with the wall-clock field, and migrating stored ops. Also out of
  scope: anything that *causes* a refused op to arrive again. SDS marks a
  message as received once it is delivered, so the live channel may never
  re-deliver an op this peer refused. The spec requires only that a second
  arrival is accepted. Demand-driven repair is not built.
