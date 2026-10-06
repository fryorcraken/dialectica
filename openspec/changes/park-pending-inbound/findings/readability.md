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

- [x] **`dev-writer`** — `delivery.rs:1602-1609` — **defect (stale comment).**
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
      **Fixed** in `09eca672`. The doc on `Delivering::start` is now a
      numbered list, and step 2 is "queue the startup review (review event 3),
      whose set of known channels is exactly those — after step 1, and before
      anything can settle". Step 4 says the processor's first take is that
      review.

- [x] **`dev-writer`** — `delivery.rs:926-934` — **defect (misleading name).**
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
      **Fixed** in `09eca672`, both halves.
      - **`Opening::settle` is now `Opening::finish(held)`.** Its body says in a
        comment that `self` is dropped at the closing brace and that the drop is
        the settle, so one path settles whether the worker finishes the open or
        the guard is merely dropped.
      - **The test barrier is `Delivering::drain` / `Action::Drain`.** Every call
        site in `delivery/tests.rs` and `delivery/tests/parking.rs` is renamed,
        and its doc says it is named apart from the settles of an open.
      - **`settle` now names only the real settles**, `ChannelBook::settle` and
        `Channels::settle`, which the module doc's seam map points at.
      - Also in `design.md` Decision 9.

- [x] **`dev-writer`** — `delivery.rs:1432-1450` — **shape + naming.**
      `Processor::park` binds `let parked = self.stores.parked()…` to a
      `Result<Parking, ParkError>` and then destructures
      `let Parking { parked, evicted } = match parked {…}`, so `parked` is a
      `Result` for four lines and a `bool` after, in the same scope. `Parking`
      (what one park did), `Parked` (a stored row), `ParkedStore`, `ParkError`,
      `Taken::Park`, `Note::Parked` and `Processor::park` are seven near-names.
      Rename `Parking` to `ParkOutcome` (fields `kept`, `evicted`) and name the
      `Result` `outcome`. The shadow then goes and `Parking { parked: false }`
      stops reading as a tautology.
      **Fixed** in `09eca672`, with an enum rather than renamed fields.
      - **`Parking` is now `ParkOutcome`, with two variants**,
        `Parked { evicted }` and `Discarded`. The spec revision (security finding
        2) means a discarded payload evicts nothing, and the enum makes that the
        only shape there is. There is no `parked: bool` left to read as a
        tautology.
      - **The `Result` is bound as `outcome`**, so the shadow is gone.
      - **`Parked` (a stored row), `ParkedStore`, `ParkError`, `Taken::Park`,
        `Note::Parked` and `Processor::park` keep their names.** Each names a
        different thing, and none was a misreading.

- [x] **`dev-writer`** — `delivery.rs:1440-1449` — **shape.**
      The two discard branches of `Processor::park` repeat the same two lines
      (`let total = self.channels.count_discard(); record(…ParkDiscarded{total})`),
      once in a loop over `evicted` and once for `!parked`. The number of
      discards is `evicted + (!parked as usize)`; one loop does it. As written, a
      reader must check that the two copies stay identical.
      **Fixed** in `09eca672`. `Processor::park` maps the outcome to
      `(discards, parked)`, which is `(evicted, true)` or `(1, false)`. One loop
      counts and logs each discard, and one `if` logs the park.

- [x] **`dev-writer`** — `delivery.rs:1458-1481` — **shape + stale doc.**
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
      **Fixed** in `09eca672`.
      - **`Processor::review` is a three-arm dispatch:** `Held` to
        `decide_parked`, `Unopened` to `refuse_parked`, and `Startup` to
        `refuse_unknown_at_startup`.
      - **Each function's doc names its event**, the review doc says "one
        function per event", and `refuse_parked`'s doc says it serves event 2
        and each channel event 3 refuses.
      - **`refuse_unknown_at_startup(&known)` can be called directly.** I added
        no test that does, since the startup arm's behaviour is pinned through
        `start` by the existing startup tests.

- [x] **`dev-writer`** — `delivery.rs:1204-1214` and `parked.rs:447-455` —
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
      **Fixed** in `09eca672`. This is the same change as architecture entry 2.
      - **The convention lives once, in `shedding::choose`.** Both callers pass
        their held messages in hand-over order and the arrival, and neither
        builds loads any more.
      - **The sentinel is now a type.** `Newest` is `Held(place)` then
        `Arrival`, with derived `Ord`, so "the arrival is the newest of all" is
        the variant order and cannot be changed in one copy.

- [x] **`dev-writer`** — `delivery.rs:1619-1707` — **shape (low).**
      `Delivering::start` is 89 lines and does nine jobs in a fixed order the
      comments defend (guard, build channels, count opens, queue the startup
      review, subscribe, spawn the processor, spawn the worker, send the node
      start, send each open). This change added the review step and its six-line
      comment. The ordering is the point, so it should stay one function; the
      processor spawn (a four-argument `Processor::new` inside a `self.spawn`
      call) is the part that does not need to be inline. Say so or leave it:
      this is the weakest finding here.
      **Fixed** in `09eca672`, as you suggested. The processor spawn is now
      `Delivering::spawn_processor(channels, stores, clock)`, beside
      `spawn_listener`. `start` keeps the ordered sequence as one function,
      because the order is the point.

- [x] **`dev-writer`** — `delivery.rs:226`, `delivery.rs:1328`,
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
      **Fixed** in `09eca672`, at your three sites and two more I found with
      `grep -n "Decision\|design"` over both files.
      - **Your three:** `delivery.rs`'s Decision 14 and Decision 10 now say
        "`delivery-wiring`'s design", and `parked.rs`'s Decision 4 says
        "`park-pending-inbound`'s design".
      - **The two more:** `node_config`'s "`design.md` records the choice" is
        now `delivery-wiring`'s, since its design is where the `Edge` mode
        decision is. The `the_park_bounds_are_pinned` comment now names
        `park-pending-inbound`'s Decision 4.
      - **Left as it was:** the module doc's
        "`design.md` (the `park-pending-inbound` change)", which already names
        its change.

- [x] **`dev-writer`** — `delivery.rs:495-497` — **style (low).**
      The `Stores` doc runs two paragraphs together: "…design, Decision 13." is
      followed directly by "A path and not open handles: …" with no `///` blank
      line, so rustdoc renders one paragraph and the second half answers a
      question ("why a path?") the first never asked. The line before it was
      edited by this change.
      **Fixed** in `09eca672`: a blank `///` line now separates the two
      paragraphs.

## `parked.rs`

- [x] **`dev-writer`** — `parked.rs:384-437` — **shape.**
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
      **Fixed** in `09eca672`, with your `shed_to_fit` shape under another name.
      - **`plan_park` does two things in sequence.** The per-channel check comes
        first, then a loop over two `Total { measure, arriving, bound }` values,
        each with named fields. Each `Total` calls `Total::make_room(kept,
        evict, channel) -> bool`.
      - **Every exit that does not park returns `Plan::Discard`**, which carries
        no evictions, so the planned evictions are never kept when the payload
        goes.
      - **The unreachable arm now returns the evictions-free plan**, with a
        comment saying why: it keeps the bound and loses nothing already
        parked. That is the spec's rule since its revision (security finding
        2).
      - **`Plan` is now an enum**, `Park { evict }` or `Discard`.

- [x] **`dev-writer`** — `parked.rs:195-239` — **style (low).**
      `create_schema` "creates the schema if the file has none, returning the
      layout version the file then stamps". It reads the version, creates or not,
      and returns the version: the name says one job and the signature says
      another, and the sibling stores' `create_schema` return `()` (`sender.rs:194`)
      or a `Created` (`log/sqlite.rs`). `found != PARKED_LAYOUT_VERSION` in
      `from_connection` is then the only reason it returns anything. Rename
      (`ensure_schema`) or return the `Created`-style enum the op log's does.
      **Fixed** in `09eca672` by the rename. It is now `ensure_schema`, and its
      doc says it makes sure the file has a schema, creating it if none, and
      returns the version the file stamps for the caller to check. The siblings'
      `create_schema` shapes are left for #204, the shared store skeleton.

- [x] **`dev-writer`** — `parked.rs:285-310` — **style (low).**
      `take_channel` builds `taken` inside a bare `{ … }` block with no comment;
      the block exists to end `select`'s borrow of `tx` before `tx.execute`.
      A reader asks "why the braces?". `query_map(...)?.collect::<Result<Vec<_>,_>>()`
      followed by a `.map` into `Parked` removes both the block and the manual
      `push` loop; or say in one comment why the borrow needs ending.
      **Fixed** in `09eca672`, as you suggested.
      - **The block and the `push` loop are gone.** `take_channel` is now
        `tx.prepare(…).and_then(|mut select| select.query_map(…)?.collect())`
        into a `Vec<Vec<u8>>`, then a `.map` into `Parked`.
      - **No comment is needed:** the statement that prepared the select ends
        inside the closure, before `tx.execute`.

## Tests (`delivery/tests/parking.rs`, `delivery/tests.rs`, `parked.rs` tests)

- [x] **`tester`** — `delivery/tests/parking.rs:391-396`, `586-606`, `962-973`,
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
      **Outcome (tester): fixed.** `one_open_one_unanswered(peer, answer)` now
      starts the peer with a counted event feed and returns an `Unanswered`
      struct with named fields (`events`, `asked`, `gate`, `open`, `stuck`), not a
      tuple; `answer` is delivery's reply to the second creation, `None` for
      created. It is called by `nothing_but_the_three_events_reviews_a_parked_message`,
      the four wiring tests and `handed_over_while_opening_then_settled`, so the
      setup is written once. The junk payload held at its log line is written
      once too: `hold_the_boundary_up` in `tests.rs` now joins and calls
      `hold_the_boundary_up_on(peer, events, &open)`, which
      `handed_over_while_opening_then_settled` calls directly. One behaviour
      difference, in the stronger direction: `nothing_but_…` now waits for the
      second open to reach delivery before it sends, as the others did. The
      suite is green, and the mutation of the worker's declined arm (settling
      it as held) failed `a_message_taken_after_its_open_was_declined_is_refused_not_parked`,
      which runs through both helpers, as predicted.

- [x] **`tester`** — `delivery/tests/parking.rs:853-856` — **style (naming of
      sections).** The last section is headed "Added by the tester", an author
      and not a topic, and its tests belong under headings the file already has:
      `a_full_queue_records_the_discard…` and `over_a_total_bound_…` under
      Bounds, `a_parked_op_the_op_log_cannot_take_…` and
      `a_message_parked_just_before_its_open_settles_…` under Reviews. A reader
      looking for "where is the bound tested" reads two sections. The provenance
      belongs in the commit, not the file.
      **Outcome (tester): fixed.** The heading is gone. The tests you name are
      under Bounds (`a_full_queue_records_…`, `over_a_total_bound_…`) and Reviews
      (`a_parked_op_the_op_log_cannot_take_…`), and
      `a_message_parked_just_before_its_open_settles_…` is with the seams, which
      is the other half of the same requirement as the two `Channels::take`
      tests beside it. So are the park-logging, taken-after-settle and
      not-readable tests, under Parking. The file's sections are listed in its
      header.

- [x] **`tester`** — `delivery/tests.rs:3396` — **findability.**
      `mod parking;` is the last line of a 3,396-line file, and the file's
      header says nothing of it. A reader of `tests.rs` meets `Peer::processor`,
      `Processor::decide` and `Processor::run_reviews` (the helpers parking
      tests lean on, lines 427-474) with no hint that 1,206 lines of their use
      are in a sibling. Declare it beside the other helpers and say in the
      header that parking's tests are there.
      **Outcome (tester): fixed.** `mod parking;` is declared straight after the
      helpers, before the first test, and `tests.rs`'s header says parking's
      tests are there and which helpers they lean on.

- [x] **`tester`** — `delivery/tests.rs:1628`, `2062`, `2131`, `2156`, `2188`,
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
      **Outcome (tester): fixed, by your second option.** Not moved: the move is
      churn in a 3,400-line file and each test reads as well where it is.
      `parking.rs`'s header now names them by requirement and by test: the three
      that end in `refused_and_not_parked`, the repeated-open pair, the
      asked-twice test, the open-waits-behind-another test, the restarted-peer
      test, and `taking_a_message_from_delivery_does_not_wait_on_the_boundary`,
      which the finding did not list and which is the same contract. The line
      numbers in the finding were stale after the review's own edits; I named
      them from the tree.

- [x] **`tester`** — `delivery/tests.rs:593`, `2721-2722`, `2916-2919` —
      **defect (stale comments, low).** Three comments narrate a version of the
      code or the test that no longer exists, in grammar that does not parse:
      line 593 "The wait this replaced had a limit set wrongly in `start` pass
      every test"; line 2721 "Nothing waits on an open any more, so an
      unanswered open cannot hold the boundary up"; lines 2916-2919 "the spec's
      answer to the question this test used to leave open, with the opposite
      expectation to the one it first pinned." State what the fixture does now
      and keep history in the commit; the first is a "why" worth keeping if it
      is made a sentence.
      **Outcome (tester): fixed for two of the three; the third no longer
      applies as read.** `hold_the_boundary_up`'s doc (was 2721) now says an
      unanswered open does not hold the boundary up because its message is
      parked, so the boundary is held at the processor's own log line instead.
      The comment in `a_message_on_a_channel_whose_open_waits_behind_another_…`
      (was 2916) states what the fixture does and what it is red against, with
      the history left to the commit. I also took the same fix to a comment you
      did not list, in `traffic_on_a_channel_this_peer_is_not_opening_…` ("with
      the opposite expectation"). Line 593: the sentence "The wait this replaced
      had a limit set wrongly in `start` pass every test" is not in `tests.rs`
      now. It is in the doc on `Processor::bounds` in `delivery.rs`, where the
      `dev-writer` has corrected it ("passed every test") — implementation code,
      not mine, and it parses.

- [x] **`tester`** — `parked.rs:533-540` — **style (low).**
      Test helper `payloads(store, channel)` calls `take_channel`, so it
      **removes** what it returns. Its name reads as a read. In
      `a_review_takes_a_channels_messages_in_hand_over_order_and_leaves_none` it
      is used twice in a row on purpose; elsewhere a second `payloads` on the
      same channel would silently return empty. Name it `take_payloads`.
      **Outcome (tester): fixed.** Renamed `take_payloads` at every call, with a
      doc line saying it removes what it returns and that a second call on a
      channel returns nothing.

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
