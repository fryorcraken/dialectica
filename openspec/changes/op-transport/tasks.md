# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

The spec row is left for `spec-writer` to tick: the spec was written before this
agent ran, and ticking another agent's row is the one thing the one-row-per-agent
rule exists to prevent.

## 1. Read the contract rather than a summary of it

- [x] Read `dialectica/contracts/delivery_module.lidl` in full. **15 methods and
      10 events**, and the four methods plus four events this capability needs are
      all present: `channelCreate`, `channelSend`, `channelClose`,
      `channelExists`; `channelMessageReceived`, `channelMessageSent`,
      `channelMessageError`, `messagePropagated`. The brief warned that the
      contract had been mis-described three times in one day; it was read here, not
      taken second-hand.
- [x] Read PLAN.md from `origin/main` for §4.1 (the channel/topic/senderId
      split), §4.2 (topics are a local filter, not routing), §4.3 (the channel id
      is the rendezvous), §4.4 (what SDS does not promise, and the 150 KiB cap),
      §9.2 (the MVP scope).
- [x] Read the `op-ordering` change's archived `design.md` rather than
      re-deriving what the transport supplies. It records the **four exhausted
      routes** to the missing ordering metadata and the measurement behind the
      `timestamp` trap (`currentTimestampNs()`, a `CLOCK_REALTIME` read). Recorded
      in `design.md` so the next reader does not spend the afternoon again.
- [x] Confirm the baseline suite before writing anything: **475 tests passing**.
      The worktree was missing its gitignored `logos-rust-sdk-src` symlink, so
      `cargo test` could not resolve the manifest at all; staged it from the same
      `/nix/store` path every other worktree uses.

## 2. `design.md`, written alongside the code and revised by it

- [x] Sketched before `tasks.md`, per the agent contract.
- [x] **Decisions** carries the eight choices with their alternatives: one
      `ChannelIdentity` type rather than two free functions; the topic/channel
      prefixes; `OpenChannels` as a map rather than a set; one `Refusal` enum;
      a second size constant rather than a re-export of `op.rs`'s field cap; the
      store-then-send ordering; `Arrival::unordered()` rather than
      `from_parts(None, None)`; and what stays in the adapter.
- [x] Records the **dead end** the delivery-outcome obligation runs into, so it is
      not rediscovered: both outcome events are asynchronous and keyed by
      `requestId`, so surfacing an undelivered op needs state outliving the publish
      call, a requestId-to-op-id map and a clock. A component, not a branch.
- [x] Names what a green suite here **structurally cannot see** (§6 below).

## 3. Channel identity, held by construction rather than checked

- [x] `ChannelIdentity::of(&Address)` — one pure function, one argument. There is
      no parameter through which a session counter, an epoch, a clock or a peer
      identity could enter, which is what makes the spec's "nothing else
      participates" structural.
- [x] Private fields with accessors, so the only route to a `ChannelIdentity` is
      the derivation. A caller cannot assemble one by hand and hand it to a send.
- [x] Content topic is §4.1's format verbatim —
      `/dialectica/1/s/<hex>/proto` — because §4.2 makes the `/dialectica/1/`
      head the thing that places every dialectica topic on one shard.
- [x] Channel id takes a **different discriminant in the same position** (`c`
      against `s`) rather than sharing the topic's value or being bare hex.
      Reasoning in `design.md`.
- [x] No epoch in either, deterministic or otherwise. §4.3 costs the
      deterministic variant out, and the withdrawn `#4116` workaround is the
      instance of paying for it.

## 4. The receive boundary

- [x] `receive(InboundMessage, &OpenChannels, &mut L)` in `dialectica-core`, so
      every refusal is reachable by a test without a node running.
- [x] `InboundMessage` carries **all four** event fields, including `sender_id`
      and `timestamp`. A struct that dropped them would be hiding the fields
      rather than refusing them; neither reaches anything recorded.
- [x] The five refusals are five `Refusal` variants, plus `Storage` for a write
      failure — which is not a judgement about the payload and is the only one
      where retrying could work.
- [x] `Refusal::Undecodable` **carries** `OpError` rather than flattening it:
      `op-format` distinguishes eleven ways a byte string is not an op, and
      discarding that one layer later is the same mistake at a smaller scale.
- [x] Guards run in the spec's order, and the **size check runs before the
      decode** — a 4 MiB payload costs a length comparison, not a parse.
- [x] The single `log.append` is the last statement, so a refusal has no
      intermediate mutation to roll back. CLAUDE.md's "keep handler bodies free of
      partial mutation" rather than a rollback path.
- [x] `Arrival::unordered()`, the named constructor, so grepping for it finds
      every place the contract's gap is absorbed.

## 5. Publishing

- [x] Append first, return the bytes second. The function **returns** a
      `Publishable` rather than sending, so there is no code path here that could
      undo the append — the spec's "a failure to hand the bytes to the transport
      SHALL NOT remove the op from the log" made structural.
- [x] The channel lookup happens **after** the append, so publishing on a Stoa
      with no open channel still stores the op. Checking first would lose it, and
      that ordering is what mutation 3 below verifies.
- [x] `PublishError::NoChannel` is distinguishable from `NotStored`, per the
      spec's "distinguishably from a transport failure on an open channel".
- [x] `publish` takes `&OpenChannels`, not `&mut`, so opening a channel is not
      reachable from it at all — "SHALL NOT open a channel as a side effect" by
      type rather than by discipline.
- [x] The payload is `SignedOp::to_bytes()` with nothing around it. An envelope
      would be attacker-controlled bytes outside the signature, which is the one
      shape this system has no defence for.

## 6. Tests, and mutation-verifying them

**475 before, 531 after this change's 56 tests, 562 once `origin/main` was
merged** — `main` had moved twice and brought 31 tests of its own. None ignored,
and no doc-test registered: the count gate's `ran == declared` holds at 562,
checked by hand against the per-file `#[test]` counts rather than asserted.

- [x] Expectations hardcoded, never read back from the implementation.
- [x] **The known-answer test was derived independently of this crate**, which is
      the only thing that makes it worth having. `stoa_address` is SHA-256 over a
      32-byte domain prefix, so the expected address came from `sha256sum` over a
      48-byte preimage built with `printf` — a separate implementation entirely —
      and the command is in the test's comment so the next reader can repeat it.
      A value read back from the code it pins agrees with any bug that code has.
- [x] The prefix-collision fixture is **constructed, not hunted**: two addresses
      agreeing in 31 of 32 bytes, built with `Address::from_bytes`. `op-log`'s
      design records a 2-byte prefix match in `iter_stoa` and an 8-byte one in
      `iter_target` each surviving the whole suite, because those fixtures pinned a
      hash coincidence and documented it as the property.
- [x] Unspecified behaviour marked: one `// NO SPEC:` on
      `a_send_that_the_transport_accepted_is_not_a_delivery`. **Since removed** —
      the `spec-writer` adopted the behaviour as the requirement "A successful
      publish is a statement about the local log and nothing more", so the comment
      now points at that requirement by name. **No `// NO SPEC:` marker remains in
      this change.**
- [x] Mutation-verified. Five mutations, each caught, each reverted and the file
      confirmed byte-identical to its pre-mutation state afterwards:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | the size check runs **after** the decode | `the_size_check_runs_before_the_decode`, `a_payload_over_the_limit_is_refused_distinguishably` |
| 2 | the Stoa comparison matches only **byte 0** of the address | `two_stoas_sharing_an_address_prefix_are_not_confused` — **and nothing else** |
| 3 | the channel check runs **before** the append, so a publish with no channel loses the op | `publishing_without_an_open_channel_fails_and_opens_nothing`, `publishing_on_another_stoas_open_channel_still_fails` |
| 4 | the inbound `timestamp` is recorded as a **Lamport value** | `the_arrival_timestamp_is_not_recorded_as_ordering_metadata`, `the_timestamp_handed_in_does_not_change_what_is_recorded`, `what_is_recorded_does_not_vary_with_receive_sequence`, `a_valid_op_is_stored_and_recorded_as_unordered`, `every_arrival_over_this_transport_is_recorded_as_unordered`, `a_peers_own_op_carries_no_ordering_metadata_of_its_own` |
| 5 | the signature check never fires | `an_op_whose_signature_does_not_verify_is_refused_distinguishably`, `an_op_whose_key_does_not_bind_to_its_claimed_author_is_refused`, `an_unauthentic_op_is_not_admitted_on_the_promise_of_a_later_check`, `a_forged_sender_identifier_grants_nothing` |

**Mutation 2 is the one worth reading.** It was caught by exactly one test, and
`an_op_naming_another_stoa_is_refused` — the test whose *name* is about the Stoa
comparison — **passed under it**. That fixture uses two hash-derived addresses,
which differ in byte 0, so a boundary comparing one byte behaves exactly like one
comparing all 32. That is this project's recurring defect family: a fixture on
which the rule and its most plausible wrong neighbour give the same answer. The
constructed 31-byte-prefix fixture is what separates them, and without it the
cross-Stoa leak would have shipped green.

**Mutation 4's most informative failure** was
`what_is_recorded_does_not_vary_with_receive_sequence`, which printed the two
runs' maps genuinely diverging — the same op recorded with `lamport: Some(0)` in
one receive order and `Some(1)` in the other. That is the per-peer divergence the
property exists to prevent, shown rather than argued.

## 7. What the green gate structurally cannot see

Recorded because a passing suite here proves less than it appears to.

- [x] **That two peers actually meet.** Every test derives both sides of a
      channel identity in one process. That the derivation is a pure function is
      checkable and checked; that two machines consequently exchange ops is a
      property of the node, and the spec lists it as out of scope for exactly this
      reason.
- [x] **That anything was ever sent or received.** No op in this suite has
      crossed a network. `publish` returns bytes a test then inspects, and
      `receive` is handed bytes a test just built — so the suite covers what this
      code decides and nothing about whether delivery carries it.
- [x] **Anything in the adapter.** `dialectica/rust-lib/src/lib.rs` is behind
      `cfg(logos_scaffold)` and is not compiled by `cargo test`, so the
      `channelCreate`/`channelSend`/`channelClose` calls themselves are untested
      by definition. This is why those three lines carry no logic: everything that
      could be wrong was moved to where a test can reach it.
- [x] **That a failed delivery is surfaced.** It is not, by anyone, and no test
      claims otherwise. See the `// NO SPEC:` marker and `design.md`.
- [x] **That the size bound matches what the network actually validates.** The
      constant is pinned to 150 KiB and the spec requires it equal the transport's
      stated limit — but nothing here reads that limit from the transport, so the
      test pins our value rather than their agreement.

## 8. Not done, and deliberately

- [x] **No core API method and no JSON handler.** The proposal excludes "any core
      API method shape", so nothing was added to the `DialecticaModule` trait. What
      this change adds is the typed boundary those methods will call; the adapter
      wiring lands with the method that needs it.
- [x] **No `senderId` derivation.** §5.2's per-Stoa identity supplies it and is
      out of the MVP by owner decision (§9.2). The boundary takes it as a
      parameter and stores it nowhere.
- [x] **No change to the op format, the op log, `arrival.rs`, or any resolver.**
      `lib.rs` gains one `pub mod` line and nothing else in the crate was touched.
- [x] **Not routed through `wire::request::Request`**, which landed on `main`
      while this change was in flight. That type is the envelope for the module's
      JSON `String`→`String` methods; this boundary is handed a `&[u8]` payload and
      a `&str` channel id from a delivery **event**, so there is no JSON on the
      path and therefore no second parser. The core API methods that will use
      `Request` are excluded from this change.

## 9. Merged `origin/main` before finishing

- [x] `git merge origin/main` (not rebase), because the branch had gone stale in
      the dangerous direction: its diff against `main` **deleted content it never
      touched** — `CLAUDE.md` −21 (the "Worktrees are not scratch" section),
      `.claude/agents/README.md` −5 (the checkbox-gate blindspot rule), `wire.rs`
      −1480, `wire/request.rs` −378, the archived wire-request change, and
      `openspec/specs/module-wire-contract/spec.md` −228. `mergeStateStatus`
      reported `UNKNOWN` rather than a conflict, so nothing would have warned.
- [x] Verified with `git diff origin/main --stat` that the only files differing
      are the ones this change touches: `transport.rs`, one line of `lib.rs`, the
      three `op-transport` documents, and `docs/PLAN.md` — the last being the
      `spec-writer`'s shedding commit that was already on the branch.
- [x] Gates re-run on the merged tree: 562 tests passing, clippy
      `-D warnings` clean, `rustfmt --check` clean on the new file,
      `openspec validate op-transport --strict` valid.

      **That 562 is this merge's number and was superseded by the next one.** #51
      merged while this piece was in review and its authoring suite arrived with
      the second `origin/main` merge, taking the baseline to **623** with nothing
      here changing. Recorded because a stated baseline that has moved sends the
      next agent hunting a regression that is someone else's feature.

## 10. Acted on review findings

- [x] `findings/spec-test.md`'s one open entry, `tester`'s store-failure path:
      `AppendFailsLog` added and three tests written, each proved able to fail by
      the mutation it names. The finding carries predicted-versus-observed for all
      three; they agreed. **626 tests passing** after, from 623.
- [x] The `// NO SPEC:` marker replaced with a pointer to its requirement **by
      name, not by line number**, the requirement having been written for exactly
      this behaviour.
- [x] `design.md`'s delivery-outcome section rewritten from *"cannot be discharged
      at this boundary at all"* to what is owed and why none of it is met here,
      naming the seam. The old wording was a scoping decision wearing an
      impossibility's clothes — it claimed the obligation could not be met and
      then described how it is met.
- [x] `design.md:38`'s quotation of a renamed requirement title corrected to
      "An oversized payload is refused, against a limit pinned at 150 KiB".
- [x] `docs/UI-BRIEF.md` gained the rendering obligation as its **obligation 7**:
      a successful publish means "saved here", not "posted". Written as the half
      that is true today — the prohibition, plus *do not design an in-flight state*
      since no call produces the signal one would wait on — with the positive half
      left for whoever answers the three owed things. PLAN §9.2's pointer updated
      to match, since it had said the brief would need this only once the three
      were answered.
- [x] Checked that the new requirement and PLAN §9.2 agree rather than assuming
      it. They do, and closely: §9.2 is struck through and points at the
      requirement, the requirement names the three owed things §9.2 assigns here,
      and both call it unbuilt. The `spec-writer` had already done this check in
      `b81ce46` and found one contradiction against `content-authoring`, which it
      fixed there.
