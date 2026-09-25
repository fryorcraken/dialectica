## Context

See `proposal.md` — Why for the motivation and the owner's decision
([#162, comment of 2026-09-25](https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926)).
This file is the *how*, and the reasoning the proposal handed on.

The state this change starts from:

- `arrival::next_counter(clock)` signs one above the clock, and
  `arrival::clock_from_counters` folds the held counters in ascending order,
  stopping at the first step larger than `ADVANCE_BOUND` (one million).
- `transport::receive` admits every authentic op addressed to its channel's
  Stoa, whatever its counter.
- `dialectica-core` reads no clock (archived `op-clock/design.md`, decision 6).
  The one clock read is `now_ms()` in the adapter (`dialectica/rust-lib/src/lib.rs`),
  which hands it to the publish path as `Authorship::asserted_ms`.
- **`transport::receive` has no caller in the adapter yet.** Delivery is not
  wired: the adapter's publish sink logs "delivery is not wired yet
  (op-transport)", and nothing hands an inbound message to `receive`. So the
  receive half of this change is complete in `core` and tested there, and the
  adapter line that will call it does not exist to be changed.

## Goals / Non-Goals

**Goals:**

- The publish rule `max(now, clock + 1)`, saturating.
- The clock as the highest counter held, derived as before.
- The one-hour window at `transport::receive`, reported as its own refusal, and
  reachable from no other path.

**Non-Goals:**

- Wiring `receive` into the adapter. That is `op-transport`'s outstanding
  adapter half, and it will pass `now_ms()` when it lands (see Decision 2).
- Anything that makes a refused op arrive again. SDS marks a message received
  on delivery, so the live channel may never re-deliver one this peer refused.
  The spec requires only that a second arrival is judged afresh. Repair is not
  built.
- Changing the op format, merging the two clock fields, or migrating stored ops
  (the owner's ruling).

## Decisions

### 1. Why the counter is pegged to the time, and what the rejected options were

Issue #162's second item is the diagnosis this change acts on, kept here because
issues are edited and closed. `clock_from_counters` kept an op whose counter was
more than `ADVANCE_BOUND` above the clock, but never moved the clock to it, while
`cmp_ops` still ordered that op by its counter. An honest peer answering it
therefore signed a **lower** counter than the reply it answered. Since #159 the
thread read is the reverse of `cmp_ops`, so an author who picked a counter over
the bound ended up **last** in the thread, after every answer to their reply.
The archived `2026-09-24-thread-reply-order/design.md` recorded this under Risks
as "A reply can come before the reply it answers" / "An over-bound parent". This
change closes that case: nothing over the window is stored, so the clock can
follow every counter held and an answer always carries the greater counter.

Three alternatives were discussed with the project manager and superseded by the
owner's decision:

1. **Write down the current behaviour** in `op-ordering`. Rejected: it specifies
   the defect — an author choosing where they appear relative to their answers.
2. **Demote over-bound ops** (store them, but order them below the rest).
   Rejected: a second ordering input beside the counter, which is the "two
   orders that disagree produce no error" failure `op-ordering` exists to
   prevent, and it still leaves the question of what "over the bound" is
   measured from.
3. **Refuse over-bound ops by arrival order** (compare against the clock at the
   moment of arrival). Rejected: `clock_from_counters`'s doc analysed this
   before this change removed it. The same ops received in two sequences give
   two answers, and a peer's own rebuild disagrees with its own ingest.

Option 3 and the window both refuse on arrival, so the difference needs saying.
Option 3 compares against the clock, and the clock depends on which *other* ops
arrived first. The window compares against the time, which depends on no other
op. Two peers with correct clocks decide the same way about the same op, whatever
else each of them holds. What the window does depend on is *when* the op
arrives, and that is the cost Decision 11 works through.

Pegging to the time is SDS's own rule (LIP-109, `sds.md` lines 148–155 and
184–192), so the transport and the application now take the same step on send.
It also gives the window something honest to be measured against: an honest
counter is close to the author's time, so "far ahead of my time" is a property
an honest op does not have.

### 2. How the current time reaches `dialectica-core`

`core` still reads no clock. The time enters as an argument, in two places:

- **Publish** reads `Authorship::asserted_ms`, which the adapter already fills
  from `now_ms()`. No adapter change is needed. See Decision 3 for why this is
  one field and not two.
- **Receive** takes a new `now_ms: u64` parameter:
  `receive(message, channels, log, now_ms)`. The adapter that eventually calls
  it passes `now_ms()`, the same function publish uses.

**Why a parameter and not a field on `InboundMessage`.** `InboundMessage` is
"exactly as `channelMessageReceived` delivers it", and it already carries a
`timestamp` that the spec forbids as the reference. Putting this peer's time
beside it would put two times in one struct, one to be read and one never to
be. As a separate argument, the forbidden one is visibly not what the window
reads. The types also differ — the event's `timestamp` is an `i64` of
nanoseconds (delivery bug #26), `now_ms` a `u64` of milliseconds — so passing
one for the other does not compile without a cast someone would have to write.

**Named `now_ms`, following the existing convention.** `Authorship::asserted_ms`'s
doc records that every other millisecond value in `core` is the *reading* peer's
clock, and the receiving peer is a reading peer. `thread.rs` and `wire.rs` already
call that value `now_ms`.

### 3. One field carries the one reading that signs both clock fields

`op-ordering` requires a publish to take its current time once and sign that
value as the wall-clock field too ("One reading of the time signs both clock
fields"). The decision here is how that one reading reaches `publish`: as the
single existing field `Authorship::asserted_ms`. `publish` computes
`next_counter(clock, who.asserted_ms)` and signs the same `asserted_ms` as the
wall-clock field.

*Considered:* a second field, `Authorship::now_ms`, for the counter. *Rejected*:
two fields let a caller pass two values that disagree, which is the state the
spec forbids. With one field the forbidden state cannot be written down, so
nothing has to check for it. A test built on two values would also exercise a
state the adapter never produces.

*What pins it:* `one_reading_of_the_time_signs_both_clock_fields`
(`authoring.rs`) signs at a time other than the fixture constant and asserts
that the counter and the wall-clock are both that time. It covers the spec's
first scenario, where the clock is behind the time. The second scenario, where
the counter comes from the clock and the wall-clock stays at the time, is
satisfied by the same line of `publish`: the field is signed as passed, whatever
`next_counter` returned.

This does not make the wall-clock field decide anything. The counter is computed
from the host's time, not read from the field. The field is written from the
same number, and no comparison reads it.

### 4. The window is checked in `transport::receive` and nowhere the store can reach

The check is a pure predicate, `arrival::exceeds_receive_window(counter, now_ms)`,
with one caller: `transport::receive`, after every other check and before the
append.

**What keeps it off the rebuild and replay paths is that they have no time to
give it.** Rebuilding a store, replaying ops, and restoring a snapshot all go
through `OpLog::append(op, arrival)`, which takes no time, and `OpLog::clock`,
which reads none. A path that holds no `now` cannot call a predicate that needs
one without someone threading a clock into the log's API. That change would be a
visible widening of a public trait, not a flag passed wrongly at one call site.

*Considered:* a flag on `append` (`check_window: bool`). *Rejected*: a flag has
to be right at every call site, and the default is whichever value the author
of the next call site guesses. *Considered:* the check inside
`OpLog::append`'s implementations. *Rejected*: a store rebuilt on a peer whose
clock has since been set back, or from another peer's snapshot, would refuse ops
it had already admitted. The clock would then differ from the one the ops gave
before, which breaks "the clock survives a rebuild".

*What holds it, honestly stated:* the signature of `OpLog::append`, not a test.
No mutation can move the check into `append` without first giving `append` a
time, so there was nothing to run. What a check in `append` would break, against
any real time, is every test that appends `u64::MAX` as a stored op:
`the_clock_override_agrees_with_the_trait_default_it_replaces`,
`the_clock_survives_a_restart_and_a_rebuild_in_another_sequence` (both
`sqlite.rs`) and `a_publish_at_the_maximum_representable_clock_saturates`
(`authoring.rs`). Those tests stand for a store written before this change or
restored from elsewhere, and they are the reason `append` must admit what
`receive` refuses.

### 5. The predicate takes the counter alone

`exceeds_receive_window(counter: u64, now_ms: u64)` takes a `u64` and not an
`OpClock` or an `Op`. The spec requires that the wall-clock field not affect the
decision. With this signature, the function cannot read it.

`receive` extracts the counter with `signed.op.clock.map(|c| c.counter)`, and an
op carrying none (`VERSION_1`) skips the check, as the spec requires.

*What breaks without it:* mutating that extraction to read `asserted_ms` turns
eight `transport.rs` tests red. Among them is
`the_decision_does_not_read_the_wall_clock_field`, whose two ops point their
wall-clocks the opposite way to their counters.

### 6. The comparison is written so that it cannot overflow

`counter.saturating_sub(now_ms) > RECEIVE_WINDOW_MS`. The obvious form,
`counter > now_ms + RECEIVE_WINDOW_MS`, overflows when `now_ms` is within an
hour of `u64::MAX`. In a debug build that panics, and a panic in a handler aborts
the module process (PHASE0-FINDINGS §3). The saturating form answers
`counter ≤ now_ms` as "not ahead", which is the spec's "no lower bound", with no
arithmetic that can wrap.

*What breaks without it:* `extreme_values_do_not_abort_the_window_check`
(`arrival.rs`) panics with "attempt to add with overflow" under the `+` form
(tests run in debug). Measured.

### 7. The clock is the maximum, and the SQLite override still folds in Rust

`clock_from_counters` becomes `max`, or zero over nothing. The ascending fold,
and the argument for sorting before folding, go with `ADVANCE_BOUND`: a maximum
is a function of the set whatever the sequence.

`SqliteOpLog::clock` still reads `score_epoch` rows back into `u64` and folds
them in Rust, rather than asking SQLite for `MAX(score_epoch)`. `append` writes
the counter through an `as i64` cast, so a counter of 2^63 or more is stored
negative, and SQL's `MAX` would rank it below every small one. The window keeps
such counters out of the receive path for the next 292 million years. But
`append` itself admits them — a store written before this change, a snapshot, or
a test fixture — and the clock must agree with the trait's definition over
whatever the store holds.

*What breaks without it:* an override reading only the SQL-highest row (`ORDER BY
score_epoch DESC LIMIT 1`, which is `MAX`'s ranking) turns
`the_clock_override_agrees_with_the_trait_default_it_replaces` red (9 against
`u64::MAX`), and `the_clock_survives_a_restart_and_a_rebuild_in_another_sequence`
red (1,789,729,304,000 against `u64::MAX`). Measured.

### 8. `next_counter(clock, now_ms)` is `max(now_ms, clock.saturating_add(1))`

It saturates rather than wrapping, for the reason the spec gives. It is one
expression rather than a branch on whether the clock is behind the time, because
the two cases are the same `max`.

### 9. The window is judged last, and reported as its own variant

`InboundRefusal::AheadOfTime { counter, now_ms }` is checked after the Stoa
comparison and before the append. The spec fixes the order: a forgery must be
reported as a forgery, not by a field its forger chose. The variant carries both
numbers, so that a log line can say by how much an op was ahead, which is how a
reader tells a clock an hour fast from a counter chosen to jump the order.

*What breaks without it:* moving the window check to just after the decode turns
`another_failure_is_reported_ahead_of_the_window` (`transport.rs`) red. It then
reports a tampered op as `AheadOfTime` rather than `FailsVerification`. Measured.

### 10. What the one-hour window allows against each reader of the rule's first entry

Three readers take the ordering rule's first entry:
`revision::current_version`, `moderation::resolve` and the feed's `latestReply`.
Issue #162 asked whether an over-bound counter was "always first" for them.
Under this change, each is exposed only to an op signed ahead of the time. Such
an op leads the ops published by peers that do not hold it, until those peers'
time passes its counter. Ops published by a peer that does hold it carry a
greater counter and lead it.

- **`revision::current_version`** considers only the post author's own valid
  versions (`is_valid_revision`). So the lead is over the author's other
  versions, published from a device that had not yet received the ahead-signed
  one, for up to an hour. No third party can use it. *Before:* an author who
  signed `u64::MAX` fixed that version as current permanently, and their own
  later revisions (one above an unadvanced clock) could never displace it.
- **`moderation::resolve`** filters for authority before it takes the first
  entry, so only a moderator can use this. A moderator can sign a `hide` or
  `unhide` up to an hour ahead and beat an opposite action that another
  moderator published within that hour without having received it. A correction
  published after receiving it wins. *Before:* `u64::MAX` won a dispute between
  moderators permanently.
- **The feed's `latestReply`**: any author can make their reply the thread's
  latest for up to an hour, against replies from peers that do not hold it.
  *Before:* `u64::MAX` held that position permanently.

This is specified once, in `op-ordering` ("An op signed ahead of the time leads
only until the time passes it"), because every reader inherits it from the
rule. No reader has code of its own for it. The exposure is recorded in each
reader's doc comments too, `revision.rs` on `current_version` and `moderation.rs`
on `resolve`, because this folder is archived and the code stays.

### 11. A receiver whose own clock is wrong

The window is measured against the receiver's own reading of the time, so a
wrong clock changes what that receiver admits. Write *R* for the real time, and
say an honest author signs about *R*.

**A receiver whose clock is slow by S** (it reads *R − S*):

- *Refuses what it should accept:* every op whose counter is above
  *R − S + 1h*. For *S* ≤ 1h, that is only an op ahead of the real time by more
  than *1h − S*, so its tolerance for fast authors shrinks by *S*. For *S* > 1h
  it refuses **every fresh honest op**, and admits a given op only once its own
  reading reaches that op's counter − 1h. That is a delay of *S − 1h*. On the
  live channel, which does not re-deliver, a refused op may never arrive again.
- *Accepts what it should refuse:* nothing. Its upper limit is below a correct
  peer's, so everything it admits a correct peer admits too.
- *Its own publishes:* each carries `max(R − S, clock + 1)`. Where it holds
  recent ops, the clock term wins and its op orders after what it holds, which
  is correct. Where it holds nothing recent — the case for *S* > 1h, since it
  refuses fresh ops — its op signs *R − S*. Every correct peer admits it (there
  is no lower bound), and orders it below the honest ops of the last *S*. So its
  posts sink. A reply to a post it holds still carries the greater counter and
  still follows the post.

**A receiver whose clock is fast by F** (it reads *R + F*):

- *Refuses what it should accept:* nothing. Its upper limit, *R + F + 1h*, is
  above a correct peer's.
- *Accepts what it should refuse:* ops up to *F + 1h* ahead of the real time.
  It stores them, and its clock follows them.
- *Its own publishes:* each carries at least *R + F*. For *F* ≤ 1h, correct
  peers admit them. Such an op leads honest ops for up to *F*, and every peer
  that holds it signs its next op above it, so for a while that Stoa's counters
  run up to *F* ahead of the real time. For *F* > 1h, **every correct peer
  refuses every op it publishes** until the real time reaches its counter − 1h,
  a delay of *F − 1h*. The same is true of any op it received from beyond the
  correct window and then answered, because its answer carries a counter above
  that op's.

**There is no ratchet, and that is why the reference is the time and not the
clock.** Each receiver compares against its own reading of the time, never its
clock. So an op that is admitted and raises every clock does not raise any
receiver's limit. An author who leads by an hour cannot lead by two with the
next op, because each receiver's limit is still its own time plus one hour.

## Risks / Trade-offs

- **[A peer with a clock more than an hour wrong is cut off, and is not told]** →
  The spec states this cost and the owner accepted it. Nothing here mitigates
  it, and neither refusal reaches the author: a fast peer's ops are refused by
  correct peers, and a slow peer refuses theirs. A future status surface could
  report "N ops refused as ahead of this machine's time", which would at least
  show a slow receiver. Not built.
- **[An honest reply to an op at the window's edge can itself be past the edge
  for a third peer]** → If *A* signs an op exactly one hour ahead of *B*'s time,
  *B* admits it and replies with that counter + 1. A third peer *C* whose clock
  is at or behind *B*'s then refuses *B*'s honest reply, until *C*'s time moves
  on by the difference. On a channel that does not re-deliver, *C* may never
  hold it. The exposure is at most the milliseconds by which *C* is behind *B*,
  plus one per reply in the chain. That is small against a clock synchronised at
  all, and it is the same trade the spec states for skew.
- **[Stored ops signed under the old rule carry small counters]** → They order
  below every op published after this change. The owner ruled that no migration
  is needed, because nothing has been released.
- **[A peer that holds an op, and then has its clock set back by more than an
  hour, refuses that op if it arrives again]** → `op-transport` requires this
  ("An op this peer already holds is judged like any other arrival"). The
  refusal is `AheadOfTime`, not `AlreadyPresent`: validation precedes every
  lookup by op id, and in `receive` that lookup is the append, which is what
  reports the duplicate. The held op is left as it was. The cost is a log line
  that reads as a fresh refusal of an op the peer already holds.
  `a_held_op_arriving_again_beyond_the_window_is_refused_rather_than_reported_as_held`
  (`transport.rs`) pins it.
- **[The adapter does not call `receive` yet]** → When it does, it must pass the
  host's `now_ms()`, never the event's `timestamp`. The types differ (Decision
  2), so the mistake takes a cast to write.

## Migration Plan

None. The owner ruled that stored ops need no migration, because nothing has
been released. Rolling back is reverting the change. Ops published under the
pegged rule carry large counters, and the old rule would admit and order them
like any other.
