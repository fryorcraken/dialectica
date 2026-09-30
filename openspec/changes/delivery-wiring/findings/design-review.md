# Design review: delivery-wiring (piece #176, PR #190)

Read at `7a2a3335`: `design.md`, `proposal.md`, `tasks.md`, both spec deltas,
`core/src/delivery.rs`, `core/src/sender.rs`, the adapter diff in
`dialectica/rust-lib/src/lib.rs`, issue #176 (body and comment), and `gh pr view 190`.

The code takes the decisions design.md records: the four-method seam has no
`stop`, `CALL_TIMEOUT` is 35 s, `INBOUND_BOUND` is 256, the node config is
`channels` / `logos.test` / `Edge`, the sender id is `/dialectica/1/p/` plus 32
random bytes, and `SETTLE_LIMIT` is 40 s. I found no code that contradicts a
recorded decision. The preset and mode are stated as an Open Question, and the code
uses `logos.test` and `Edge`, matching Decision 5. Each CLAUDE.md trap has a line in
the design table. Spec deltas, proposal and design agree with each other. Every gap
below is in the prose, the verification claims or the PR body.

Serious means the prose or the change contradicts the issue or itself. A gap is a
decision not recorded. A suggestion is a thin entry.

- [x] **`dev-writer`** (serious, contradicts issue #176 without argument) —
      `proposal.md:136-137` says "This PR is verified by hand with two `lgs basecamp
      launch` profiles", and `design.md:332` says two peers exchanging an op "is
      verified by hand". `tasks.md:56` (7.3) is unticked, and the PR body says "Not
      done here: two live peers exchanging a post". So two documents state a
      verification that has not happened. Issue #176 says "Verification has to be two
      live peers under `lgs basecamp launch`". The owner's comment asks for an
      automated test after the issue lands, and does not waive the manual check. The
      departure (the PR ships without the check the issue calls mandatory) is neither
      argued nor put to the owner. Either run 7.3 and record the result, or reword
      proposal and design to "not yet verified live" and put the missing live check
      to the owner as a stated departure with a reason.
      **Deferred** to the owner, the second option: `proposal.md` and `design.md`
      (Risks) now say the live check has not been run, and design.md's Open
      Questions states the departure from #176 and its reason (a live run needs a
      person to click each profile's tile; this pass could not run it) and asks the
      owner to accept it or hold the merge until 7.3 is run. The PR body's "Open
      for the owner" carries the same. 7.3 stays unticked.
- [ ] **`closer`** — the PR body's Receive bullet reads "the reverse of the closed
      #30". `closed #30` is a GitHub closing keyword followed by an issue number. It
      is harmless on the state of an already-closed #30, but it puts a second
      keyword-plus-number beside `Closes #176`, which the piece forbids (see the
      "PR-body prose closes issues" incident). Reword it to "the reverse of #30's
      choice" and re-scan the body for keyword+`#N`.
- [x] **`dev-writer`** (gap) — Decision 11 records the pending-open wait and its
      bound but not its cost, and no alternative other than "refuse as unknown".
      `Channels::await_settled` blocks the single processor for up to `CALL_TIMEOUT`
      (35 s, `SETTLE_LIMIT` 40 s), so while one `channelCreate` is slow, messages on
      every other, open channel wait behind it. Meanwhile the 256-deep queue of
      Decision 10 fills and discards arrivals, and a discard is final (Decision 10's
      own reasoning). Decisions 10 and 11 interact and neither entry says so. Also
      unrecorded: why the wait sits in the processor and is not a per-channel hold
      or a requeue, and what `SETTLE_LIMIT` costs when reached. Record that cost and
      the alternatives.
      **Fixed** in the docs commit: Decision 11 now has "What the wait costs" (the
      single processor stalls every open channel for up to one call; the queue then
      fills and discards; reaching `SETTLE_LIMIT` judges against what is open then,
      a lost op for a still-unanswered open) and "Alternatives, and why not"
      (refuse without waiting; a per-channel hold; requeue). Decision 10 ends by
      naming the interaction, and Risks carries the stall.
- [x] **`dev-writer`** (gap, mutation evidence) — design.md carries "removing this
      turns X red" for Decisions 3, 11 and 13 only. The other guards have theirs only
      in the PR body's mutation list, which goes with the PR:
      - Decision 10: `>=` to `>` (three bound tests) and discarding the oldest
        (`a_full_queue_keeps_what_it_holds_and_discards_the_arrival`).
      - Decision 12: refusal log carrying the text
        (`a_refusal_is_logged_by_kind_without_text_the_sender_chose`).
      - Decision 7: an unretained sender id still opening a channel
        (`a_sender_identifier_that_cannot_be_retained_opens_no_channel`).
      - Decision 9: sink called before verification (`a_refused_join_requests_no_channel`).
      - Decision 6: dropping the start-once guard
        (`node_creation_is_requested_once_however_often_startup_runs`), and the
        `declined` envelope mutation (three tests).
      - The clock-source mutation (seven tests) in Decision 10.
      Copy each into its Decision entry. Decision 2's source-reading test
      (`the_adapter_never_stops_a_node_and_creates_one_at_one_site`) and Decision 5's
      pin also state no "removing X turns Y red" line; add one for each.
      **Fixed** in the docs commit: each is in its Decision entry (2, 5, 6, 7, 8,
      9, 10, 12). Where the first pass's list gives a count and no test names (the
      `declined` envelope's "three tests", the clock's "seven"), the entry says so
      and cites that list rather than naming tests I did not re-run. This round's
      new guards (Decisions 11, 14, 15, 16, and the hand-over refusal in 10) carry
      the test that was seen red.
- [x] **`dev-writer`** (gap, reasoning that migrates) — the traps this change
      learned are needed by whoever next touches `lib.rs` or `delivery.rs`, and they
      sit only in this change's `design.md`, which will be archived. Put a short
      entry in CLAUDE.md's "Module contract traps" (or a `docs/` file linked from its
      table) for each:
      - delivery answers a decline as `Ok` with an error envelope (`declined`); a
        second `createNode` says "Context already initialized".
      - delivery v0.2.1 can emit `channelMessageReceived` before the `channelCreate`
        answer reaches the caller (the pending-open state).
      - delivery's 30 s callback timeout outlasts the IPC 20 s default, hence 35 s.
      - `cargo test` does not compile the adapter; only `nix build ./dialectica#lgx`
        does.
      CLAUDE.md's `RET_STALE_WARN` line still says "Handle" and "Ignoring it
      double-counts". Decision-table row 3 says it is unreachable on this surface,
      confirmed at v0.2.1. Amend that line so the two do not read as contradicting.
      **Fixed** in the docs commit: CLAUDE.md's "Module contract traps" gains four
      entries (decline as `Ok` with an envelope, empty `error` and "Context already
      initialized"; the 30 s callback against the 20 s IPC default, and the
      "already exists" answer after it; `channelMessageReceived` before the
      `channelCreate` answer; `cargo test` not compiling the adapter), and the
      `RET_STALE_WARN` entry now says it reaches outcome-event counting and not the
      module's own calls at v0.2.1.
- [x] **`dev-writer`** (gap, proposal is stale against the code) —
      `proposal.md:156-157` says the spec takes a position on open questions 1-4 and
      "each can be reversed before the code lands". The code has landed. `design.md`
      Open Questions and the PR body carry only the preset and mode. Questions 1-4
      (a failed open leaves the join unchanged, sender-id privacy, discard the
      arrival, re-publish sends again) are neither recorded as resolved nor still put
      to the owner. Decisions 7, 9 and 10 cover 1-3 by implication, and 4 is only in
      tasks 4.4. State in design.md which of the four are settled by the spec and
      which remain open, and align the PR body's "Open for the owner".
      **Fixed** in the docs commit: design.md's Open Questions says 1–4 are settled
      by the spec and implemented, with the Decision for each, 5 is the preset and
      mode, and 6 (added by the spec-writer since) is the owner's. The proposal's
      sentence was already corrected by the spec-writer in `9c96a41a`. The PR body's
      "Open for the owner" is aligned.
- [x] **`dev-writer`** (suggestion, thin entries) —
      - Decision 5 gives the `logos.test` cluster number, the 150 KiB maximum
        message size and "Edge publishes and receives through the service nodes" with
        no source, and it says "matches #30". Nothing shows an `Edge` node receives
        `channelMessageReceived` on a reliable channel, which is the whole feature.
        Decision 7 says "Not measured" for its SDS claim, and Decision 5 should say
        the same about these claims, or cite the file (`docs/SOURCES.md` names the
        checkouts).
      - Decision 7 states the format but not why: the `/dialectica/1/p/` structure
        against a bare hex string, the `p`, and the choice of 32 bytes. The reason is
        a paragraph in the `sender.rs` doc comment, and a paragraph means it was a
        decision.
      - Decision 6 says unanswered means decline, and the cost is unrecorded. A
        `createNode` that delivery accepted but answered after 35 s is treated as
        declined, so `start` is never requested and no channel on that node can work.
      **Fixed** in the docs commit. Decision 5 cites `networks_config.nim` and
      `default_values.nim` at `logos-delivery` `bfdb5afd`, the rev delivery v0.2.1's
      `flake.lock` pins — and notes that a later checkout (`4a85db1b`) names cluster
      2 for both presets, so the claim is about the pin — and says in bold that an
      `Edge` node receiving `channelMessageReceived` is not measured. Decision 7
      gains "The format" (32 bytes, the structured head, `p`, the newtype).
      Decision 6 gains the cost of an unanswered `createNode`, and why a node has
      no "already exists" recovery the way a channel does (Decision 14).

## Re-review round 1 `7a2a3335..369561d1`

Read: `design.md` in full at `369561d1`, `proposal.md`, `tasks.md`, the
`op-transport` delta's inbound requirement and scenarios, `delivery.rs` (seam,
`node_config`, `declined`, `channel_answer`, `Channels`, `Opening`, `Worker`,
`InboundQueue`, `listen`, `hand_over`, `Processor`, `Delivering`), issue #176 (body
and comment, fresh), and `gh pr view 190`. I re-read delivery's source at
`bfdb5afd` for Decision 14's two quotations (`channel_lifecycle.nim:43`,
`channel_api.nim:24`): both are as cited.

**Earlier findings.** All confirmed fixed in the text. The PR body's Receive
bullet no longer contains "closed #30" (it now says "the reverse of #30's choice"),
and the only keyword-plus-number is `Closes #176`; that box above is the closer's
to tick. The live-verification departure is now argued in `design.md` Open
Questions and put to the owner, with 7.3 open, and `proposal.md`, `design.md`, the
PR body and `tasks.md` all say "not run".

**The code takes the decisions.** `CALL_TIMEOUT` 35 s, `SETTLE_LIMIT` 40 s,
`INBOUND_BOUND` 256, the `channels` / `logos.test` / `Edge` node config,
`ALREADY_EXISTS` as a substring of the decline reason, `>=` in the queue bound,
the unknown-channel check before the size check in `refused_on_hand_over`, startup
marking its opens before `subscribe()`, the `Opening` guard travelling in
`Action::Open`, the per-event `catch_unwind` in `listen`, and the wait as its own
loop in `await_settled` are each in the code as recorded. No code contradicts a
recorded decision.

**The owner's three open questions** are each stated as open, in `design.md` Open
Questions and the PR body, and the code does what each says today: `logos.test` and
`Edge` (`node_config`); one 256-deep queue shared by every open Stoa
(`InboundQueue::offer`); the sender identifier held at whatever length arrives
(`Arriving.sender_id: String`, no bound anywhere on the inbound path). One
disagreement between documents is the first box below.

- [x] **`spec-writer`** (gap, documents disagree) — `proposal.md`'s "Open questions
      for the owner" lists six, and the sender identifier's length is not among them.
      `design.md` Open Questions and the PR body's "Open for the owner" both carry it
      as an owner question, and no spec text mentions it. The proposal is where the
      owner reads the list, and it says the sixth "was raised in review" as if that
      were the last. Add it as a seventh (the security review raised it; the spec
      does not bound it; the code holds it at delivery's length), or `design.md` and
      the PR body are listing a question the proposal does not know exists.
      **Outcome (`spec-writer`): fixed.** `proposal.md` "Open questions for the
      owner" now has a seventh, "How long a sender identifier this peer holds may
      be", saying what the code does (holds it as it arrives), that the spec is
      silent, and three options with what each makes the system do: leave it
      unbounded, refuse an over-long one on hand-over, or stop holding it while a
      message waits (which amends the inbound requirement). The list's preamble
      now says the sixth and seventh came from the security review and that the
      spec says nothing about the seventh. Not decided; it is the owner's.
- [x] **`dev-writer`** (gap, cost recorded but not sized, and a rejected alternative
      whose reasoning applies to the chosen one) — Decision 11 and Risks say the
      wait costs "up to `SETTLE_LIMIT` per message". The aggregate is what an owner
      needs and is not stated. `await_settled` restarts its clock for each message
      (`started` is local to the call, `delivery.rs:588`), and the channel stays
      pending until the worker reaches it, so *N* messages on one still-pending
      channel stall every Stoa for *N* × 40 s: 256 of them, the whole queue, is
      10,240 s, about 2 h 51 min, with every other Stoa's arrivals discarded for good
      once the queue is full. Decision 11 rejects "a limit sized for a queue of opens"
      with exactly that argument ("every message on a still-queued channel would hold
      every Stoa up for that long, one after another, and 256 of them fill the queue
      into discards"), then accepts the same shape at 40 s instead of 35 s × the Stoas
      ahead. State the multiplied figure, and say why the per-message shape is
      accepted (the spec's "no later than a fixed time after this peer began waiting
      **on it**" says per message) rather than implying the limit is what bounds the
      stall. Also unrecorded, and compatible with that wording ("no later than"): a
      wait that is *remembered per channel*, so a channel whose wait has already
      timed out is judged at once until its open settles, which bounds the stall at
      one limit per open instead of one per message. Record it as an alternative and
      what ruled it out, or put it to the owner with Question 6, since the attacker
      who can flood a channel this peer is opening is the same one.
      **Fixed** (`dev-writer`) in the commit `Bound the wait on a pending open
      once per open, not once per message`, settled in favour of the per-open
      bound you describe, which the spec now requires (`b6d35fac`). Each pending
      open carries `wait_ends`, set when a message first waits on it and cleared
      by a new request; every message on the channel waits to that instant.
      Decision 11 now states the multiplied figure the per-message shape had
      (n × 40 s; 256 × 40 s = 10,240 s ≈ 2 h 51 min, and the security probe's
      2.01 s for ten messages at 200 ms), why it was replaced, what per open gives
      up (later messages on the stuck channel refused at once), and what still
      bounds the stall (one limit per unanswered open, so K × 40 s for K opens
      stuck at startup, bounded by memberships, not by a sender). The rejection of
      a limit sized for a queue of opens is re-argued under per-open, where the
      old argument no longer applied, and two alternatives are added: the
      per-message clock itself, and starting an open's time at the request.
- [x] **`dev-writer`** (gap, mutation evidence and an unpinned relation) — Decision
      11's "Removing the wait turns … red" lines cover the wait, the poisoned book and
      the guard placement. Nothing says what turns red when the **bound** is removed:
      `tasks.md` 8.6 records that
      `a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait`
      "passed before the change too", and 9.4 says "No test can see the value". Decision
      4 says the same of `CALL_TIMEOUT` in the design ("argued here and in its doc, not
      pinned"); Decision 11 does not say it of `SETTLE_LIMIT`. The relation the whole
      of the "one call, always settles within it" argument rests on, `SETTLE_LIMIT >
      CALL_TIMEOUT`, is checkable without a live delivery, and `tests.rs` uses
      `SETTLE_LIMIT` only to build a `Processor`
      (`git grep -n -F SETTLE_LIMIT`), so lowering it to 10 s changes nothing red.
      Either pin the relation (a compile-time assert beside the constant, or a test)
      and record it, or write in Decision 11 that the value and the relation are not
      pinned; and record what removing the bound (an unbounded wait) does to the
      bounded-wait test, having run it.
      **Fixed** (`dev-writer`) in the commit `Bound the wait on a pending open
      once per open, not once per message`. The relation is pinned at compile
      time beside `SETTLE_LIMIT`: `DELIVERY_CALLBACK_TIMEOUT` (30 s, named for the
      purpose) `< CALL_TIMEOUT < SETTLE_LIMIT`, two `const _: () = assert!(…)`.
      Run both ways: `SETTLE_LIMIT` at 10 s fails the build on the second
      assertion, `CALL_TIMEOUT` at 20 s on the first; restored. Removing the
      bound — the wait ending only on a settle, the deadline check disabled and
      the timed wait made an hour — turns six delivery tests red, each timing out
      in `eventually`, the bounded-wait test among them; with only the timed wait
      lengthened, the two tests whose settle storms keep waking the loop stayed
      green, which is why the recorded mutation disables the check as well.
      Decision 11 and Decision 4 record both; `tasks.md` 10.4 too. This also
      covers the correctness re-review's `tester` box on the same relation (its
      "one assertion over the two constants and a hardcoded 30 s"); that box is
      the tester's to tick.
- [x] **`dev-writer`** (suggestion, a checkable claim that is false, and an
      unrecorded wait) — Decision 3 says "the processor, the listener and the worker
      share only the channel book's mutex". They also share `InboundQueue`'s mutex
      (`offer` on the listener, `take` on the processor, `delivery.rs:875-902`), and
      the dispatch thread takes the book's lock too (`Delivering::joined`,
      `startup_opens`). The conclusion holds, since none of it waits on the worker. The
      one real wait between threads is left out: the dispatch thread's publish
      appends to `ops.sqlite` (Decision 13 names it as one of the threads that opens
      it), so a reply can wait behind the processor's append for as long as the
      SQLite busy timeout, which is rusqlite's five-second default
      (`delivery/tests.rs:1705`; no code sets it). Decision 13 says appends "wait
      rather than fail" and not for how long, nor that this is the library default
      rather than a choice. Correct the sentence, and record the figure and that it
      bounds a publish reply during an inbound append.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`. Decision 3's sentence now lists the dispatch
      thread, the worker, the listener and the processor as the book's users and
      the listener and processor as the queue's. A new bullet there records the
      one wait between threads a reply can meet: a publish's append behind the
      processor's, bounded by rusqlite's default busy timeout, which nothing here
      sets. The figure was measured rather than taken from the test comment: a
      throwaway probe held a write lock on the op log from a second connection
      and timed an `SqliteOpLog` append, which failed "database is locked" at
      5.01 s (probe removed). Decision 13 now says "for up to 5 s, rusqlite's
      default … which nothing here sets" and points at Decision 3.

## Re-review round 2 `369561d1..2cb71aaf`

Read: `design.md` in full at `0a8f8639` (Decisions 3, 4, 6, 10, 11, 14, 15, 16, the
trap table, Risks, Open Questions), `proposal.md` in full, `tasks.md`, the round's
diff of both spec deltas, `delivery.rs` (`Pending`, `ChannelBook`, `SETTLE_LIMIT`
and its two compile-time asserts, `Channels::opening` / `settle` / `await_settled`,
`listen`, `hand_over`, `refused_on_hand_over`, `Processor::decide` / `pass`, the
order in `Delivering::start`), `transport.rs` (`receive_via`, `judge` now private,
`refuse_oversized`), issue #176 (body and comment, fresh) and `gh pr view 190`. I
checked that all 33 test names `design.md` cites in its "turns red" lines exist
(`git grep`); I did not re-run the mutations.

**Round 1's boxes.** The seventh open question is now in `proposal.md` (with the
code's behaviour and three options), the per-open bound is recorded with its
multiplied figure and alternatives, the `SETTLE_LIMIT` relation is pinned at compile
time and recorded, and Decision 3's sentence about shared mutexes is corrected with
the measured 5 s. All confirmed in the text and in the code. The closer's box from
round 0 (`closed #30`) is fixed in the body: `Closes #176` is the only
keyword-plus-number in it.

**The code takes the decisions.** The wait is per open (`Pending::wait_ends`
read once per message in `await_settled`, cleared in `opening`, kept across expiry
because `settle` alone removes the entry); the hand-over check asks the channel
before the size; startup marks its opens before `subscribe()`; the boundary's
order is in `receive_via` only and `judge` is private; the listener contains a
panic per event. No code contradicts a recorded decision, and I found no choice
in the round's code that a reader would make differently and that Decisions leaves
unexplained. Nothing in `design.md` contradicts #176: its live-verification
departure is argued and put to the owner, and `handoff` in place of
`transport::publish`'s bytes is covered by Decision 8.

**The owner's three questions.** Each is open in `design.md` and in the PR body,
with what the code does (`logos.test` and `Edge`; one shared 256-deep queue; the
sender identifier held at the length that arrives). In `proposal.md`, 6 and 7 do;
5 does not, which is the one box.

- [x] **`spec-writer`** (gap, documents disagree) — `proposal.md:209-211`, open
      question 5, says "`#30` used `Edge` on `logos.test`" and never says what this
      change's code does. The reader of the proposal, who is the owner deciding the
      question, has to go to `design.md` Decision 5 or the PR body to learn that the
      code uses `logos.test` and `Edge` (`node_config`, pinned by
      `the_node_preset_and_mode_are_pinned`). Questions 6 and 7 in the same list say
      what the code does ("the spec as written", "The code holds it as it arrives").
      Add one sentence to question 5: this change's code uses `logos.test` and
      `Edge`, the same as #30, and either can change as a constant and its pin
      without a spec change.
      **Outcome (`spec-writer`): fixed.** Question 5 in `proposal.md` now ends
      "This change's code uses `logos.test` and `Edge`, the same as #30. Either can
      change as a constant and its pin, without a spec change." Checked against
      `node_config` in `delivery.rs` (`"preset": "logos.test"`, `"mode": "Edge"`).
      The question stays open for the owner.

## Re-review round 3 `2cb71aaf..7462ded8`

Read: the round's diff of `design.md` (Decisions 4, 6, 10, 11, Risks), `proposal.md`,
`tasks.md` (11.1, 11.2) and the `op-transport` delta, the round's `delivery.rs`
diff (`wait_ends` now asks `is_opening`; `CALL_TIMEOUT`, `SETTLE_LIMIT` and assert
comments), the new `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
test, issue #176 (body and comment, fresh) and `gh pr view 190`. I did not re-run
the mutations.

Round 2's box is answered as its outcome says: `proposal.md` Open Question 5 states
that the code uses `logos.test` and `Edge`. Decision 11's re-argument checks by hand:
40m = (m+1) x 35 at m = 7 (both 280 s); 20 x 36 = 720 against 21 x 35 = 735. The two
withdrawn premises are recorded, the alternatives and what 40 s costs are stated, and
the value is called a judgement, not a measurement. The code matches the decisions: the
constants and asserts are unchanged, and `wait_ends` using `is_opening` is the
behaviour-free predicate merge 11.1 says. The test writes the 30 s as a literal, as
Decision 4 says. Proposal, design, spec delta and the PR body agree; the body's
`NO SPEC:` section says None, and `Closes #176` is its only keyword beside an issue
number. Nothing contradicts #176. Taste, not a box: `design.md:375` now reads "the
four tests named below" with no names at the sentence, so the reader must find the
four bullets at `design.md:435-447`.

- [x] **re-review round 3 `2cb71aaf..7462ded8`: no findings** — read the round's design.md, proposal.md, spec delta, tasks.md, delivery.rs diff, the new order test, issue #176 and PR #190's body; clean

## Re-review round 4 `7462ded8..58460b02`

Decision 11 records the ask choice with its reason (the round-3 probe), the once-only
cap with the alternatives it rules out (every ask extending; a log of asks; storing
only the start; moving the limit into the book; narrowing the spec), and the cost (a
wait that ran out before the ask, and a message already extended, are still lost).
Hand checks: a message extended at an ask before its end stalls under 80 s; 40 + 35 =
75 s in practice; m = 1..6 gives (m+1) x 35 above 40m, equal at 7. The claim that
`an_earlier_message_on_a_queued_open...` is red only with both halves gone holds by
hand (extension alone: the junk waits to the settle and the op is then judged on an
open channel; restart alone: the op waits from the ask). The code takes the decisions:
`Opening::asked` is called once, immediately before `channel_create`; `Wait::extend_from`
is the `Option::take` cap; `Processor::new` is the only construction outside a
struct literal, guarded by the test's count. Every test named in Decision 11 and
tasks.md 12.2 exists. Proposal, design, spec deltas and the PR #190 body agree on the
ask, the under-twice bound and the remaining loss; `Closes #176` is the only closing
keyword; `NO SPEC:` is None; open questions 5, 6 and 7 stay open in all three. Nothing
contradicts #176.

- [x] **re-review round 4 `7462ded8..58460b02`: no findings** — read the round's design.md, proposal.md, spec deltas, tasks.md, delivery.rs diff, PR #190's body and the test names Decision 11 cites; clean
