# Readability review: park-pending-inbound

Dimension reviewed: **readability only** (naming, function size and single job,
comments that go stale, whether the three trigger seams are easy to find and
follow). Correctness, security and architecture are other instances'. I did not
run `cargo mutants`: it measures what tests catch, which is not this dimension.
Every claim below was read against the tree at `30b9d14f`.

Severity key: **defect** = a reader is told something false or is likely to be
misled; **shape** = a function or fixture doing more than one job; **style** =
a preference, safe to reject.

## `delivery.rs`

- [ ] **`dev-writer`** — `delivery.rs:1602-1609` — **defect (stale comment).**
      The doc on `Delivering::start` says "In order: count the channel of every
      Stoa … as being opened, subscribe to `channelMessageReceived` …, start the
      processor and the worker, ask for the node, then ask for each of those
      channels." This change put a step between the first two: `start` now calls
      `channels.begin_startup_review()` (line 1658), review trigger 3, which
      must come after the opens are counted and before anything can settle.
      The doc is the first place a reader goes to learn the order, and it omits
      the one step whose position the spec's third event depends on; only an
      inline comment 50 lines down says it.
      **Scenario:** a reader who places the review by the doc has no place to
      put it, and may queue it after `subscribe()` or after the opens are sent,
      where the inline comment's "set of known channels is exactly startup's"
      no longer holds.

- [ ] **`dev-writer`** — `delivery.rs:926-934` — **defect (misleading name).**
      `Opening::settle(mut self, held)` does not settle anything: it sets
      `self.held` and returns, and the settle happens in `Drop` when `self` goes
      out of scope at the closing brace. The doc says "Settle the open now",
      which is true of the effect and false of the body, and the body carries no
      comment saying the drop is the settle. `settle` also now names three other
      things in the same file with different meaning: `ChannelBook::settle`
      (change the book, return a review), `Channels::settle` (that, under the
      lock, plus queueing), and the test barrier `Delivering::settle` /
      `Action::Settle` (wait for the worker). The seam map in the module doc
      points at `ChannelBook::settle` as "the one place a settle becomes a
      review"; a reader searching `settle` meets five candidates.
      Suggest `Opening::settle` become `Opening::finish(held)` or
      `give_answer(held)` with one line saying the drop does the work, and the
      barrier pair keep their name or become `drain`/`Drain`.

- [ ] **`dev-writer`** — `delivery.rs:1432-1450` — **shape + naming.**
      `Processor::park` binds `let parked = self.stores.parked()…` to a
      `Result<Parking, ParkError>` and then destructures
      `let Parking { parked, evicted } = match parked {…}`, so `parked` is a
      `Result` for four lines and a `bool` after, in the same scope. `Parking`
      (what one park did), `Parked` (a stored row), `ParkedStore`, `ParkError`,
      `Taken::Park`, `Note::Parked` and `Processor::park` are seven near-names.
      Rename `Parking` to `ParkOutcome` (fields `kept`, `evicted`) and name the
      `Result` `outcome`. The shadow then goes and `Parking { parked: false }`
      stops reading as a tautology.

- [ ] **`dev-writer`** — `delivery.rs:1440-1449` — **shape.**
      The two discard branches of `Processor::park` repeat the same two lines
      (`let total = self.channels.count_discard(); record(…ParkDiscarded{total})`),
      once in a loop over `evicted` and once for `!parked`. The number of
      discards is `evicted + (!parked as usize)`; one loop does it. As written, a
      reader must check that the two copies stay identical.

- [ ] **`dev-writer`** — `delivery.rs:1458-1481` — **shape + stale doc.**
      `Processor::review` is one `match` whose three arms are three different
      jobs: `Held` decides each message under its own `contained`, `Unopened`
      refuses, and `Startup` lists the store's channels, filters by `known`, and
      refuses each, with its own error handling and an early `return record(…)`
      of a `()`. The doc says "Decide every message the review's event left
      parked", true of one arm in three. These are the three review events the
      spec names, and the module doc promises each seam is "one named function"
      a test can call; the third, startup's filtering, is not. Extract
      `decide_parked(channel)` and `refuse_unknown_at_startup(known)` so
      `review` is a three-line dispatch, and the startup arm can be tested
      without a `Review` value.

- [ ] **`dev-writer`** — `delivery.rs:1204-1214` and `parked.rs:447-455` —
      **shape.** The convention "the arriving payload is counted with its own
      channel as that channel's newest of all" is written twice, once for the
      queue (`own.newest = u64::MAX`) and once for the parked messages
      (`channel_loads`), with `u64::MAX` as an uncommented sentinel at both. The
      doc on `shed` argues at length that this is one rule in one function, and
      the arrival's tie-break is the half that is not: change the sentinel in one
      copy and the queue and the parked bounds stop sharing a tie-break while
      `shed` and its test stay green. Give `parked.rs` a small constructor
      (for example `Load::arrival(measure)`) or a `tally` helper both callers
      use, and name the sentinel.

- [ ] **`dev-writer`** — `delivery.rs:1619-1707` — **shape (low).**
      `Delivering::start` is 89 lines and does nine jobs in a fixed order the
      comments defend (guard, build channels, count opens, queue the startup
      review, subscribe, spawn the processor, spawn the worker, send the node
      start, send each open). This change added the review step and its six-line
      comment. The ordering is the point, so it should stay one function; the
      processor spawn (a four-argument `Processor::new` inside a `self.spawn`
      call) is the part that does not need to be inline. Say so or leave it:
      this is the weakest finding here.

- [ ] **`dev-writer`** — `delivery.rs:226`, `delivery.rs:1328`,
      `parked.rs:69` — **defect (ambiguous reference).**
      Comments cite "design.md Decision 14", "design Decision 10" and "`design.md`
      Decision 4" with no change name. Two designs now carry numbered Decisions
      the code cites: `delivery-wiring`'s (14 is the already-exists wording, 10
      is the queue, 13 is the stores) and this change's (4 is the bounds, 10 is
      the processor's lifetime, 5 is the shedding rule). `delivery.rs:1328`'s
      "Decision 10" is `delivery-wiring`'s; this change's Decision 10 is
      something else, and both sit in the same file. After archive neither is at
      `changes/<name>/design.md`. `delivery.rs:495` already qualifies
      ("`delivery-wiring`'s design, Decision 13"); do the same at the other
      three.

- [ ] **`dev-writer`** — `delivery.rs:495-497` — **style (low).**
      The `Stores` doc runs two paragraphs together: "…design, Decision 13." is
      followed directly by "A path and not open handles: …" with no `///` blank
      line, so rustdoc renders one paragraph and the second half answers a
      question ("why a path?") the first never asked. The line before it was
      edited by this change.

## `parked.rs`

- [ ] **`dev-writer`** — `parked.rs:384-437` — **shape.**
      `plan_park` is 54 lines doing two jobs: the per-channel check (lines
      385-394) and the total-shedding loop (396-436), the latter three levels
      deep (`for` → `loop` → `match` → `match`) with three separate
      `return Plan { park: false, evict }` exits. Its `measures` is an array of
      unlabelled `(fn(&Row) -> u64, u64, usize)` tuples whose members the loop
      names `measure, arriving, bound`; a two-field struct, or two calls to a
      `shed_to_fit(kept, measure, arriving, bound)` helper, would carry the same
      thing with names. The comment `// Unreachable: shed names only a channel
      with a load.` is followed by code that silently discards the payload while
      keeping the evictions already planned; say why that is the safe default,
      or return the evictions-free plan.

- [ ] **`dev-writer`** — `parked.rs:195-239` — **style (low).**
      `create_schema` "creates the schema if the file has none, returning the
      layout version the file then stamps". It reads the version, creates or not,
      and returns the version: the name says one job and the signature says
      another, and the sibling stores' `create_schema` return `()` (`sender.rs:194`)
      or a `Created` (`log/sqlite.rs`). `found != PARKED_LAYOUT_VERSION` in
      `from_connection` is then the only reason it returns anything. Rename
      (`ensure_schema`) or return the `Created`-style enum the op log's does.

- [ ] **`dev-writer`** — `parked.rs:285-310` — **style (low).**
      `take_channel` builds `taken` inside a bare `{ … }` block with no comment;
      the block exists to end `select`'s borrow of `tx` before `tx.execute`.
      A reader asks "why the braces?". `query_map(...)?.collect::<Result<Vec<_>,_>>()`
      followed by a `.map` into `Parked` removes both the block and the manual
      `push` loop; or say in one comment why the borrow needs ending.

## Tests (`delivery/tests/parking.rs`, `delivery/tests.rs`, `parked.rs` tests)

- [ ] **`tester`** — `delivery/tests/parking.rs:391-396`, `586-606`, `962-973`,
      and `delivery/tests.rs:2723-2743` — **shape.**
      "Agora open, Lyceum's open asked of delivery and held at a gate" is set up
      three times (`nothing_but_the_three_events_reviews_a_parked_message`,
      `one_open_one_unanswered`, `handed_over_while_opening_then_settled`),
      differing only in `start_counted` against `start_listening`. The
      "junk payload held at its log line" fixture is set up twice
      (`hold_the_boundary_up` in `tests.rs` and lines 978-988 of
      `handed_over_while_opening_then_settled`, same bytes, same `hold_on`, same
      `eventually`). A change to what "unanswered open" means is a four-place
      edit; `one_open_one_unanswered` should take the events constructor (or
      return `asked` too) and the other three call it.

- [ ] **`tester`** — `delivery/tests/parking.rs:853-856` — **style (naming of
      sections).** The last section is headed "Added by the tester", an author
      and not a topic, and its tests belong under headings the file already has:
      `a_full_queue_records_the_discard…` and `over_a_total_bound_…` under
      Bounds, `a_parked_op_the_op_log_cannot_take_…` and
      `a_message_parked_just_before_its_open_settles_…` under Reviews. A reader
      looking for "where is the bound tested" reads two sections. The provenance
      belongs in the commit, not the file.

- [ ] **`tester`** — `delivery/tests.rs:3396` — **findability.**
      `mod parking;` is the last line of a 3,396-line file, and the file's
      header says nothing of it. A reader of `tests.rs` meets `Peer::processor`,
      `Processor::decide` and `Processor::run_reviews` (the helpers parking
      tests lean on, lines 427-474) with no hint that 1,206 lines of their use
      are in a sibling. Declare it beside the other helpers and say in the
      header that parking's tests are there.

- [ ] **`tester`** — `delivery/tests.rs:1628`, `2062`, `2131`, `2156`, `2188`,
      `2915`, `2963` — **findability (suggestion).**
      The header of `parking.rs` says it holds "what each requirement of
      `op-transport`'s parking contract looks like from a message", yet six
      tests of that contract, and a helper, remain in `tests.rs`: an open given
      up leaves nothing parked (`refused_and_not_parked`,
      `an_open_this_peer_gives_up_…`), a message on an open channel does not
      park on a repeated open, a channel asked for twice is being opened, a
      declined repeat leaves the channel open, a message whose open waits behind
      another, and a restarted peer keeping what delivery hands over. The
      scenario "Messages parked … decided once" is in one file and its mirror
      "refused, not parked" in the other. Moving them is churn in a 3,400-line
      file; if not moved, name them in the `parking.rs` header.

- [ ] **`tester`** — `delivery/tests.rs:593`, `2721-2722`, `2916-2919` —
      **defect (stale comments, low).** Three comments narrate a version of the
      code or the test that no longer exists, in grammar that does not parse:
      line 593 "The wait this replaced had a limit set wrongly in `start` pass
      every test"; line 2721 "Nothing waits on an open any more, so an
      unanswered open cannot hold the boundary up"; lines 2916-2919 "the spec's
      answer to the question this test used to leave open, with the opposite
      expectation to the one it first pinned." State what the fixture does now
      and keep history in the commit; the first is a "why" worth keeping if it
      is made a sentence.

- [ ] **`tester`** — `parked.rs:533-540` — **style (low).**
      Test helper `payloads(store, channel)` calls `take_channel`, so it
      **removes** what it returns. Its name reads as a read. In
      `a_review_takes_a_channels_messages_in_hand_over_order_and_leaves_none` it
      is used twice in a row on purpose; elsewhere a second `payloads` on the
      same channel would silently return empty. Name it `take_payloads`.

## What is clean

- **The three seams are easy to find.** The module doc of `delivery.rs`
  (lines 35-51) names each seam as a question with a linked function
  (`ChannelBook::on_take`, `ChannelBook::settle` and `startup_review`,
  `Channels::take`), each of those functions opens its doc with a bold "the one
  place" sentence and the spec requirement it implements, `Review` says it is
  made in exactly two places, and the top of `parking.rs` lists the same
  functions in the same order, so doc, code and tests agree. `on_take` is
  seven lines and a test calls it directly; `settle` and `startup_review` are
  short and table-driven in the test. I traced each from the doc to the function
  to a test and none needed a search.
- **Comments say why.** `Channels` explains why one lock and why reviews do
  not queue among payloads (the order clause); `Taken`, `Review` and
  `ChannelBook` explain the state they exist for and the race behind it;
  `Processor::new` says it is the one constructor and the lesson that
  motivated it. `Note`'s doc says what no line carries. I found no comment that
  restates its code.
- **`parked.rs` is well-organised.** The module doc says what a row keeps and
  what it must not, why a file of its own, and that nothing panics; the SQL
  carries its own comments; `PARK_BOUNDS` has the arithmetic checked (8 MiB over
  150 KiB is 54 payloads) and compile-time asserts for the orders the spec
  requires.
- **No residue of the removed wait** in names: `SETTLE_LIMIT`, `await_settled`,
  `settle_limit` and the rest of the removed set are not found anywhere under
  `dialectica/` (`grep -rn` returned nothing).
- **Naming of the review path** (`Review::Held`, `Unopened`, `Startup`;
  `refuse_parked`, `take_parked`, `judge`, `park`) says what each does.
