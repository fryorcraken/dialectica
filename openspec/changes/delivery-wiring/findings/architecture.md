# Architecture review: delivery-wiring (#176)

Reviewed at `7a2a3335` (`origin/piece/176-delivery-wiring`), diff `origin/main...HEAD`.
Dimension covered: **architecture** only. `cargo test -p dialectica -p dialectica-core`
passes (1256 core tests, then the integration suites) and `nix build ./dialectica#lgx`
succeeds, so the `cfg(logos_scaffold)` adapter compiles.

- [x] **`dev-writer`** — `dialectica-core/src/delivery.rs:330` — a third copy of the
      panic-payload renderer, after this piece extracted the second **to stop
      exactly this**.
      **Scenario:** `git grep -n -F "non-string panic payload"` finds three
      non-test copies of the same `downcast_ref::<&str>` / `<String>` chain:
      `delivery.rs:335`, `wire.rs:75` (inside `guarded`) and `wire.rs:2636`
      (`panic_detail`, added by commit `6a69ce4c`). The doc on `wire::panic_detail`
      says it is "shared by the two sinks … so the two cannot come to render the
      same payload differently", which was true for two and is now false for
      three; `guarded`, the guard every handler runs under, still carries its own
      inline copy. A fourth payload type (or a change to what an unknown payload
      renders as) will be made in one and not the others.
      **Fix:** make `wire::panic_detail` `pub(crate)`, call it from `guarded` and
      from `delivery.rs`, delete the two others.
      **Severity:** low. A defect against the change's own stated rationale, not a
      taste point. **Measured:** the grep above.
      **Fixed** in `cd5d7c82`: `wire::panic_detail` is `pub(crate)` and is the
      only rendering; `guarded`, the two sinks and `delivery.rs` call it, and the
      grep above now finds one non-test site. A no-behaviour refactor, so no test
      goes red without it; `wire.rs`'s existing `guarded` panic tests still pin the
      rendering.

- [x] **`dev-writer`** — `dialectica-core/src/delivery.rs:485` — `Channels::receive`
      holds the channel-book mutex across signature verification **and the SQLite
      append**, and `design.md:80` says the opposite.
      **Scenario:** `transport::receive(message, &lock(&self.book).open, log, now_ms)`
      keeps the `MutexGuard` temporary alive for the whole call, because
      `receive` borrows `&OpenChannels`. `receive` verifies an ed25519 signature
      and runs `log.append` (`transport.rs:524`, `:552`). While the processor is
      inside a slow append (it waits on a busy `ops.sqlite` when the dispatch
      thread is appending), the worker's `Opening::drop` -> `Channels::settle`
      (`:514` -> `:451`) and `Channels::handoff` (`:493`) block on the same mutex.
      So a channel open is not recorded, and a send is not made, until an inbound
      op has finished storing. No deadlock (the processor never waits on the
      worker while holding it), but it couples the two threads the design says
      "share only a mutex held for map lookups" (`design.md:80`), and that
      sentence is what argued the event loop and worker cannot be held up.
      **Fix:** copy what the boundary needs out under the lock and release it
      before verify/append (`receive` taking the resolved Stoa, or a snapshot of
      the open set), or, if the coupling is accepted, correct `design.md:80` and
      `Channels::receive`'s doc. The reshape is the better one: the invariant
      then holds by construction rather than by a sentence.
      **Severity:** low to medium. Not reproduced with a timing test; it follows
      from the guard's lifetime.
      **Fixed** in `cd5d7c82` (the room: `transport::receive` split into lookup,
      `judge` and `admit`, with `Judged` only `judge` can make) and `3b72e546`
      (the change): `Channels::receive` is gone; `Channels::stoa_of` copies the
      Stoa out under the lock and the processor judges and appends with it
      released. The reshape, as suggested, so it holds by construction.
      `the_channel_book_is_not_held_while_an_op_is_appended` holds `ops.sqlite`'s
      write lock from a second connection so the append waits, and asserts the
      book is free meanwhile; it was red before `3b72e546`. design.md Decision 15.

- [x] **`dev-writer`** — `dialectica-core/src/delivery.rs:883-887` — `Delivering`
      encodes three states in two fields (`started: bool`, `outbox: Option<Outbox>`),
      and the fourth combination gets a wrong log line.
      **Scenario:** `start` sets `started = true`, then the worker thread fails to
      spawn (`Builder::spawn` refuses) and it returns `true` with `outbox = None`
      (`:974-978`). Every later `joined` / `published` goes to `request`'s `None`
      arm (`:1015`) and logs `NotStarted`: "delivery wiring has not started, so
      … was not requested", though startup ran, the listener and processor are
      up, and only the worker is missing. The comment at `:930` ("Its own flag,
      not `outbox.is_some()`") is the tell: a branch holding up an invariant the
      shape should hold. An enum (`NotStarted` / `Started { outbox: Option<..> }`,
      or `Wired(Outbox)` beside a `NoWorker`) makes the impossible pair
      unrepresentable and gives the worker-gone case its own arm.
      **Severity:** low. Shape and one misleading line; the path is reachable
      only when the OS refuses a thread. CLAUDE.md, "Put the complexity in the
      data structure, not the logic".
      **Fixed** in `89b3b552`: `Wiring { NotStarted, NoWorker, Running(Outbox) }`
      replaces the flag and the option, and `NoWorker` logs "the delivery worker
      could not be started". `a_worker_that_could_not_start_is_named_as_such_and_startup_does_not_run_again`
      pins the line and that the state counts as started; it builds `Wiring`
      directly, so it was never run against the old shape and has not been seen
      red.

- [x] **`tester`** — `dialectica/rust-lib/src/lib.rs:609-625` (`DeliveryModule::channel_create`)
      — the outbound forwarding of three adjacent `&str` arguments is unpinned,
      while the inbound mapping of two adjacent `String`s is pinned.
      **Scenario:** swap `channel_id` and `content_topic` in the call to
      `channel_create_with_timeout`. It compiles (all `&str`). `cargo test` does
      not compile this file (`design.md:7-9`), and the tests drive `Delivery`
      through a fake that stands on the trait's side of the seam, so nothing
      ever observes what the adapter forwards to the generated client. Every
      test stays green while delivery creates every channel under the topic's
      name, so every send and receive misses; only the manual check in task 7.3
      could notice.
      `the_adapter_maps_each_event_field_to_the_field_of_the_same_name`
      (`delivery/tests.rs:683`) exists for precisely this failure class on the
      inbound side and reads the source text; the outbound side has no
      equivalent. The same reading (whitespace stripped, comments stripped)
      pins `channel_create_with_timeout(channel_id,content_topic,sender_id,`.
      **Severity:** low to medium. **Measured:** not by mutation, because the
      edit was refused by this session's permission classifier; it follows from
      the gated region being outside every `cargo` gate, which `design.md`
      states.
      **Outcome (`tester`): fixed.**
      `delivery::tests::the_adapter_forwards_each_delivery_argument_in_the_order_the_seam_names_them`
      reads `lib.rs` (comments stripped, whitespace removed) for
      `create_node_with_timeout(config,`,
      `channel_create_with_timeout(channel_id,content_topic,sender_id,` and
      `channel_send_with_timeout(channel_id,payload,`, each exactly once. Mutation:
      `channel_id` and `content_topic` swapped in `lib.rs`'s `channel_create` (the
      swap this finding describes; it compiles, all `&str`). Predicted red, observed
      red, with the message "the adapter does not forward" and the
      `channel_create_with_timeout(...)` text above. No other test moved.
      Restored. It is a text pin, which is the only layer that can see
      this file without `nix build`; it pins the order, not that the client is
      called.

## Judged and left without a box

**The tester's proposed extraction ("a pure event fields to `Arriving` function
in core") does not belong in this piece, and would not cover what it is meant
to.** The risk is a swap or omission in the field-by-field copy from the
*generated* decoder's struct (`lib.rs:651-656`). That struct's type exists only
in a builder build, so core cannot name it: a core function would have to take a
core-owned mirror of the four fields, and the copy into that mirror would still
sit in the adapter, untested, with the same swap available. It moves the
function without moving the risk. The proportionate gate is what the tester
already wrote (the text pin at `delivery/tests.rs:683`). If the owner wants the
swap ruled out by construction rather than by a text read, the shape is
newtypes on `Arriving`'s `channel_id` / `sender_id` so a swap does not compile
— a change to `Arriving`'s type and the adapter's one mapping, and a follow-up,
not part of a wiring piece.

## Clean, in prose

- **Where the logic sits.** The gated region added is small and is what the
  design says: four one-line forwards (`DeliveryModule`), one subscription plus
  field copy (`channel_messages`), one `start` call in `on_context_ready`, and
  three sink hookups (`publishing`, `create_stoa`, `join_stoa`). Everything with
  a decision in it (decline reading, ordering, sender identifier, queue, refusal
  logging, the listener loop) is in `core::delivery` behind the four-method
  `Delivery` seam, driven by a fake that answers from its input. That follows
  the repository's own precedent (`OnboardingSession` moved out of the adapter
  for the same reason). The hookups (`joined` x2, `published`, `start`) are not
  pinned by any test: dropping one keeps every cargo gate green. That is the
  piece's stated position and task 7.3 (two peers under `lgs basecamp launch`,
  unticked) is the only gate on it, so 7.3 has to be run before merge; it is not
  a finding against the code.
- **The refactor commit (`6a69ce4c`) made room and changed no behaviour.**
  `op_log_path_in` replaces the four adapter spellings (no non-test spelling of
  `ops.sqlite` remains outside that function), and the `joined` sink is called
  only on the success arm of `store.join`, with the adapter passing a no-op in
  that commit. Two rustfmt-only hunks in unrelated `wire.rs` tests ride along
  (`record_failure`, the `get_stoa` `assert!`); harmless, and CI's fmt gate
  cannot see this crate anyway.
- **Core API.** No new wire method: the `DialecticaModule` trait is untouched in
  the diff, `metadata.json` is unchanged, and no reply shape changes. The
  Rust-level signatures of `core::create_stoa` and `core::join_stoa` gained a
  sink parameter; every caller (adapter, tests, `tests/seeded_reference.rs`) is
  updated. `dialectica-core` widens its public surface with items only its own
  tests need (`declined`, `node_config`, `INBOUND_BOUND`, `Journal`, `Stderr`);
  it has one consumer, so this is not worth a box.
- **Complexity in data.** `ChannelBook { open, pending }` with a `Drop` guard
  (`Opening`) settling on every path out of the worker, `SenderId` as a newtype,
  `Note` as the one place every log line is worded, and an exhaustive
  no-wildcard `refusal_kind` are the right shape; each makes an invariant hold
  by construction.
- **Not raised:** `Delivering::start` (about 75 lines, four commented phases)
  reads as one job whose order matters (subscribe, then processor, then worker,
  then node, then memberships). Splitting inbound from outbound would read
  better; a preference, not a defect. The sender store is a third copy of the
  layout-versioned SQLite scaffolding (`membership.rs`, `identity_store.rs`
  before it), a pre-existing pattern with a documented reason for its own file.
  `cargo mutants` was not run: this dimension reviews shape, not test strength.

## Re-review round 1 `7a2a3335..369561d1`

Read: the full range diff for `delivery.rs`, `transport.rs`, `wire.rs`, `sender.rs`
and `CLAUDE.md`; `delivery.rs` whole at HEAD; the adapter's delivery hookups in
`lib.rs` (unchanged in this range) and the text pins that read them; `design.md`
Decisions 11 and 15. Confirmed by running, not reading: `cargo test -p dialectica
-p dialectica-core` is green at `cd5d7c82` (1256 core tests), at `6111011b` (1269)
and at HEAD (1286), each checked out on its own; `nix build ./dialectica#lgx`
compiles the adapter at HEAD. Both refactor commits change no behaviour:
`cd5d7c82` moves the body of `receive` into `judge`/`admit` verbatim and swaps two
inline payload renderings for `panic_detail`; `6111011b` changes `Opening`'s borrow
to an `Arc` and moves one `>` comparison into `refuse_oversized`.

The four earlier findings are confirmed fixed: one non-test renderer of a panic
payload remains (`wire::panic_detail`); `Channels::receive` is gone and `stoa_of`
copies the Stoa out, so no lock spans a verify or an append; `Wiring` replaces the
flag and the option, and a refused worker has its own arm; the outbound-argument
text pin exists and pins the order.

- [x] **`dev-writer`** — `transport.rs:507-512`, `delivery.rs:1077-1092` —
      `Processor::decide` spells the boundary's pipeline a second time, and
      `transport::receive`, the spelling the ~55 boundary tests drive, has no
      production caller.
      **Scenario:** `git grep -n -E "\breceive\("` finds calls only in
      `transport.rs`'s tests and `authoring.rs`'s test helper `deliver`. The
      running module reaches the boundary through `stoa_of` -> `judge` -> `admit`
      composed by hand in `decide`. `receive`'s doc says it "is what every other
      caller uses" and `authoring.rs:1939` calls its helper "the real receive
      boundary"; neither is what runs. A change that adds a step between the
      lookup and `judge` (a per-channel rate check, a second lookup) goes in
      `receive`, is covered by every transport test, and is absent from
      production with every gate green. The `decide` comment ("`transport::receive`'s
      order") is the reviewer's only link between the two, and it is a sentence.
      **Fix:** make one spelling the only one. Either give `transport` a single
      composition both call (for example `receive` taking the resolved
      `Option<Address>` and an `impl FnOnce() -> Result<L, OpLogError>` that opens
      the log only for a judged op, with the map lookup left to the caller), or
      have `decide`'s three calls be a named method (`fn admit_inbound(&self, ..)
      -> Result<Admitted, InboundRefusal>`) that a test compares with `receive` on
      the same inputs. If the duplication is accepted, correct `receive`'s "every
      other caller" sentence, since it has none. `decide` also carries the "and"
      tell in its own doc ("Put one message through the boundary, and log what it
      decided"): the extraction separates the boundary from the logging.
      **Severity:** low. A shape and drift-risk finding, not a live defect; the two
      spellings agree today.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`, no behaviour change: the first of the two shapes
      offered. `transport::receive_via(message, stoa_of, now_ms, write)` is the
      one place the order (lookup, `judge`, write) is written; `receive` is
      `receive_via` with `OpenChannels::stoa_of` and `admit` into the log it was
      handed, and `Processor::pass` is `receive_via` with the book's copying
      lookup and a write that opens the op log and calls `admit`. So a step added
      between the lookup and `judge` goes in `receive_via` and reaches the
      running module and the ~55 boundary tests together. `decide` is now three
      calls (`await_settled`, `pass`, `record_decision`), which separates the
      boundary from the logging. Whole suite green before and after.

- [x] **`dev-writer`** — `transport.rs:548-552` — `judge` takes the channel's Stoa
      as a bare `Address` and never reads `message.channel_id`, so `Judged` no
      longer certifies the channel check its doc says it certifies.
      **Scenario:** `judge(message_on_channel_a, stoa_b, now)` accepts an op whose
      Stoa is `stoa_b`, though the message arrived on `stoa_a`'s channel: nothing
      in `judge` relates the id to the address. `receive` derived the Stoa from the
      id inside one function; the split moved that tie to the caller. `Judged`'s
      doc ("passed every check `receive` makes before it writes"; "`admit` cannot
      be handed an op that skipped a check") is then true of checks 2-6 only. The
      one caller today, `decide`, passes `stoa_of(&message.channel_id)`, so nothing
      is wrong now; the property the type was introduced for holds by a caller's
      convention, which is what CLAUDE.md's "invariant holds by construction"
      rule is against. `judge` is `pub`.
      **Fix:** either have `judge` refuse when `ChannelIdentity::of(&channel_stoa)
      .channel_id() != message.channel_id` (cheap; it is a check, so it is
      `UnknownChannel`), or make `OpenChannels::stoa_of` return a token type that
      `judge` takes in place of an `Address`; or narrow the `Judged` and `judge`
      docs to what they certify.
      **Severity:** low. Public-surface shape; no reachable wrong result today.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`, by construction rather than by a check: `judge`
      is now private, and its one caller is `receive_via`, which looks the Stoa
      up itself under `message.channel_id`. No caller can pass a message and a
      Stoa separately any more, so `Judged` certifies the channel check too, and
      its doc now says so. `transport::tests::the_boundary_looks_a_channel_up_under_the_messages_own_identifier`
      pins the key: two channels open, an op naming the one it did not arrive
      on, the lookup asked exactly the arrival channel's id, and the refusal a
      Stoa mismatch against it. Red with the lookup keyed by `message.sender_id`
      instead (the lookup was asked `a-participant`); restored.

- [x] **`dev-writer`** — `arrival.rs:221-223` — the doc on
      `exceeds_receive_window` says "`crate::transport::receive` is its one
      caller"; after `cd5d7c82` the caller is `judge`.
      **Scenario:** `git grep -n -F "exceeds_receive_window" -- '*.rs'` outside
      tests finds one call, `transport.rs:579`, inside `judge`. The section is the
      one that says which paths may reach the window ("called from the receive
      boundary and from nowhere else"), so a reader checking it against the code
      lands in `judge`, not `receive`. **Severity:** low, a doc made false by this
      round's split.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`: the doc now names `transport::judge` as the one
      caller, says it is private and reached only through `receive_via`, and
      that `receive` and the delivery processor both go through that.

- [x] **`tester`** — `delivery/tests.rs:765` — the negative half of
      `the_adapter_hands_delivery_every_recorded_membership_and_every_published_op`
      is a hand-written list of five closure spellings, and a sixth passes.
      **Scenario:** added to `lib.rs`, under `#[cfg(logos_scaffold)]`,
      `let mut sink = |_x: &core::op::OpId| {};` — a closure that ignores its
      argument, spelled `|_x|`, which the list (`|_id|`, `|_stoa|`, `|_|`, `|_op|`,
      `|_op_id|`) does not name. **Measured:** `cargo test -p dialectica-core
      the_adapter` runs 6 tests and all 6 pass with that line in the adapter. The
      list is an enumeration of the spellings someone thought of; a prefix rule
      is total over spellings nobody has typed yet, and `lib.rs` has no `|_`
      anywhere today (`git grep -n -F "|_" -- dialectica/rust-lib/src/lib.rs`
      finds nothing), so `code.contains("|_")` would pass now and fail on any
      ignoring closure. The positive counts still pin the real sinks; this is the
      guard for a *new* handler's sink. **Severity:** low.
      **Outcome (`tester`): fixed.** The negative half of
      `the_adapter_hands_delivery_every_recorded_membership_and_every_published_op`
      is now the prefix rule, `|_` (a closure opens with `|` and a discarded
      parameter starts with `_`), plus `,_|` for a discarded second parameter, in
      place of the list of five spellings. Mutation: your own line,
      `let mut sink = |_x: &core::op::OpId| {};` added to `lib.rs` under
      `#[cfg(logos_scaffold)]`. Predicted red, observed red ("the adapter contains a
      closure that ignores its argument: `|_`"); the five-spelling list passes it, as
      measured in this finding. Restored (`git diff --stat` on `lib.rs` empty). Not
      covered, by design: a sink that ignores its argument by using it in a way
      that changes nothing (`|id| drop(id)`), which no text pin can tell from a
      real one.

**Clean, in prose.**

- **The `Opening` guard travelling inside `Action::Open`** is the right shape:
  ownership, not a rule each path follows. Every route by which an open fails to
  reach delivery (no worker, a worker gone, no sender identifier, a panic, a
  refused send) ends in a `Drop`, and `request` builds the action only when an
  outbox exists, so no open is marked for a request that is logged and dropped.
  Holding an `Arc<Channels>` rather than a borrow is what that needs; no cycle
  (`Channels` holds no sender), and the guard's drop takes the book lock through
  `lock()`, poison-tolerant.
- **`Wiring`** is the reshape that was asked for. `start` writes `NoWorker` before
  any work and overwrites it with `Running` only on success, so a panic or an early
  return leaves the state the log line describes.
- **The adapter stays thin and no wire method was added.** `lib.rs` is unchanged
  in this range; the `DialecticaModule` trait and `metadata.json` are untouched.
  Every decision added this round (hand-over refusals, the settle limit, the
  oversize predicate) is in `core::delivery` or `core::transport`, behind the
  four-method seam, and tested by a driver that is not the adapter.
- **`refuse_oversized`** is the right extraction: one predicate for `judge` and
  the hand-over, so the two cannot disagree at exactly the limit; the
  unknown-channel-first order in `refused_on_hand_over` is stated and pinned.
- **`Judged`/`admit`** does hold the property it names for checks 2-6 (private
  field, single constructor), and opening the log only for a judged op is a real
  gain. The finding above is about the channel tie, not the split.
- **The text pins of the adapter are the proportionate gate** for a file no
  `cargo` gate compiles, and the piece says so; the one remaining weakness is the
  list above. `CLAUDE.md`'s additions are pinned to `v0.2.1` and to named code,
  so they invalidate visibly. Not raised: `delivery.rs` is 1366 lines with seven
  commented sections and could be split into files along them; a preference. The
  module doc's line 38 is one unwrapped 130-column line (rustfmt does not reflow
  comments). `cargo mutants` was not run: this dimension reviews shape.

## Re-review round 2 `369561d1..2cb71aaf`

Read: the full source diff of the range (`delivery.rs`, `transport.rs`,
`arrival.rs`), `Channels`/`ChannelBook`/`Pending`/`Processor` whole at HEAD, the
new tests that read the book (`delivery/tests.rs:2256`, `:2299`, `:2364`, `:2417`),
and `design.md` for the two refactors. Confirmed by running, not reading: each
refactor commit checked out on its own and `cargo test -p dialectica-core --lib`
run: green at `7c9d6cd6` (1287 tests) and at `ff3eba85` (1287), and
`cargo clippy -p dialectica-core --all-targets -- -D warnings` is clean at HEAD.
Neither refactor changes behaviour: `7c9d6cd6` moves the two halves' composition
into `receive_via` and splits `decide` into `pass` and `record_decision` with the
same calls in the same order (the one added test pins the lookup key, and passes
on the commit that adds it); `ff3eba85` wraps the count in `Pending { requests }`
and names `!open && pending` as `is_opening`, and adds no field beyond the count.

The round-1 outcomes are confirmed: `receive` has a production caller through
`receive_via` (`Processor::pass` and `receive` both call it, and no other spelling
of lookup-judge-write remains), `judge` is private so `Judged` certifies the channel
tie by construction, and `arrival.rs:223` names `judge` reached through
`receive_via`.

- [x] **`dev-writer`** — `delivery.rs:559-567` — `ChannelBook::wait_ends` spells
      "this channel is being opened" a second time, one commit after
      `ff3eba85` extracted `is_opening` to be its one spelling.
      **Scenario:** `is_opening` is `!self.open.is_open(id) && self.pending.contains_key(id)`
      (`:553-555`). `wait_ends` is `if self.open.is_open(id) { return None }` followed by
      `self.pending.get_mut(id).map(..)`, which returns `Some` exactly when
      `!open && pending`. `await_settled` (`:696-699`) then asks the two in sequence
      about the same channel: `wait_ends` decides whether to wait, `is_opening`
      decides whether to keep waiting, and the two agree only because both
      conditions were typed the same way. A change to what "being opened" means (a
      third state, say "held but not yet subscribed") goes into `is_opening`, is
      covered by every test that drives the loop, and leaves `wait_ends` starting a
      clock for a channel the loop will not wait on, or the reverse.
      **Fix:** `wait_ends` guards with `if !self.is_opening(channel_id) { return None; }`
      and then takes the pending record, so the predicate is written once. The
      `Option` from `get_mut` is then unreachable-`None`, which `.and_then` on the
      lookup already handles without a new branch.
      **Severity:** low. A defect against the refactor's own rationale ("the fourth
      slightly-different copy of a guard"), not a live bug: the two agree today.
      **Measured:** `git grep -n -E "is_open\(channel_id\)" -- dialectica/rust-lib/dialectica-core/src/delivery.rs`
      finds the `!open` half at `:554` (`is_opening`), `:561` (`wait_ends`) and
      `:672` (`is_known`, a different predicate); the first two are the same one.
      **Fixed** (`dev-writer`) in the commit `Ask whether a message waits on its
      open through one predicate`, as the fix shape says: `ChannelBook::wait_ends`
      returns `None` unless `is_opening`, then takes the pending record, and its
      doc names `is_opening` as the one question both it and `await_settled`'s
      loop ask. No behaviour change, so no test can show it red: the 94
      `delivery::` tests pass before and after, and the `git grep` above now
      finds `is_open(channel_id)` only in `is_opening` and `is_known`.

**Judged and left without a box (taste).**

- **`wait_ends` names a field, a `Pending` method and a `ChannelBook` method,
  and both methods mutate.** `Pending::wait_ends(&mut self, limit)` and
  `ChannelBook::wait_ends(&mut self, ..)` read as accessors and start the open's
  clock on first call; the test at `delivery/tests.rs:2275` uses the call as though it
  were a read (`ends_at`). The doc says what it does, and `&mut self` shows it, so
  it is a naming preference (`begin_or_get_deadline`, say), not a defect; noted
  because the rustdoc link `[`Pending::wait_ends`]` in three docs has a field and a
  method for it to mean.
- **The compile-time asserts and `the_call_timeout_outlasts_deliverys_own_...`
  overlap by design** (the test comment says why the test writes `30` itself, not
  reading `DELIVERY_CALLBACK_TIMEOUT`, which exists only to feed the asserts). That
  is the right split for a fact about a file this repo does not hold; not a finding.
- **`limit` is passed on every `wait_ends` call but used on the first only.** It
  is the processor's field and the per-open deadline ignores a later, different
  limit; harmless with one caller, and the doc states "one fixed time".

**Clean, in prose.**

- **The per-open deadline is in the data, not the loop.** `Pending::wait_ends` holds
  the one instant, `opening` clears it, and an expired wait needs no state of its
  own because every later message gets the same past instant back: the invariant
  ("the time passes once per open") holds by construction, with no flag to keep in
  step. The four tests read the book or race a clock they control, and each names the
  mutation that reddens it.
- **`await_settled` reads the deadline once**, before the loop, so a request made
  meanwhile cannot extend a message already waiting; the doc says so and the storm
  test pins it.
- **`decide` is now three calls**, the "and" in its old doc is gone, and `pass` carries
  the clock-read comment beside the one place the clock is read.
- **The adapter is untouched in the range** (`lib.rs` not in the diff); no wire
  method, no `metadata.json` change. `cargo mutants` was not run: this dimension
  reviews shape.
