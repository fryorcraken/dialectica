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
