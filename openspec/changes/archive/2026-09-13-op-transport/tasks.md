# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — runner
- [x] `openspec validate --strict`, then `archive` — runner

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
- [x] The five refusals are five `InboundRefusal` variants, plus `Storage` for a
      write failure — which is not a judgement about the payload and is the only one
      where retrying could work.
- [x] `InboundRefusal::Undecodable` **carries** `OpError` rather than flattening it:
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
      claims otherwise. The record is the requirement "A successful publish is a
      statement about the local log and nothing more", which names the three owed
      things, plus `design.md`'s section on the seam they attach to. This bullet
      used to point at a `// NO SPEC:` marker, contradicting §6 above within the
      same file: the marker was removed when the `spec-writer` adopted the
      behaviour as that requirement, and §6 is the current claim.
- [x] **That the size bound matches what the network actually validates.** The
      constant is pinned to 150 KiB, and the spec is explicit that agreement with
      the network's limit is **not** checkable here — no limit reaches this
      capability from the transport, so there is no second value to compare
      against. The test pins our value against local drift and nothing more. (This
      bullet previously said the spec "requires it equal the transport's stated
      limit"; the spec was rewritten in `b1af4e3` precisely because a scenario
      claiming the two are compared would be comparing the constant against
      itself.)
- [x] **That a publishable op is receivable.** The suite now measures the gap
      rather than leaving it unseen — `a_body_at_the_authoring_cap_encodes_past_the_message_limit`
      shows a body at `authoring::MAX_BODY_LEN` encoding to 153,740 bytes and being
      refused by `receive` — but **nothing refuses it at publish**, and closing that
      contradicts a merged `content-authoring` scenario. See `design.md`, "The
      publish cap and the message limit leave a band of unreceivable ops".

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
- [x] `docs/UI-BRIEF.md` gained the rendering obligation, titled **"A successful
      publish means 'saved here', not 'posted'"**. Cited by title rather than by
      number because merging `origin/main` at `b85111d` added two identity
      obligations that took numbers 7 and 8, moving this one to 9 — a number in a
      cross-file citation is a claim that goes stale on somebody else's merge.
      Written as the half
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

## 11. The `tester` stage: the two findings routed here

Two boxes, both fixed, both with predicted-versus-observed recorded in the finding
itself rather than summarised here.

- [x] `findings/correctness.md`'s `tester` entry — the **one surviving mutant** of
      the 32 on this file. `emptiness_tracks_what_is_open_in_both_directions` added,
      asserting the `false` direction of `OpenChannels::is_empty`, which no test had
      ever observed. Two channels rather than one, because with a single channel
      "not empty" and "nothing has been closed yet" are the same state.
- [x] `findings/readability.md`'s `tester` entry — the name
      `identity_does_not_vary_with_local_state`. Fixed as a **rename plus a new
      test**, the rename alone being the weaker half: the renamed
      `identity_does_not_vary_with_the_peers_history` witnesses the history scenario
      honestly, and the new `the_derivation_is_a_pure_function_of_the_address`
      witnesses the construction scenario, which had no test and was discharged by
      reading `of`'s signature.
- [x] The pid mutation the spec-test reviewer reported was **reproduced before
      deciding the fix**, rather than the box being read as a naming task. It
      behaved exactly as reported: the old test passed, only the known-answer pin
      failed, and it failed as "the derivation changed" rather than as local state
      participating.
- [x] **`cargo mutants` re-run against the core crate's own manifest** (the
      workspace manifest finds 0 and exits 0, so a clean result that way means
      nothing): **32 mutants, 27 caught, 5 unviable, 0 missed**, from 26/5/1. The
      survivor is gone and no new one appeared.
- [x] Every mutation reverted, and the implementation proved untouched **by diff
      rather than from memory**: `git diff` on `transport.rs` shows test-module
      content only, no line outside `mod tests` altered.
- [x] Three further renames, routed by the `spec-writer`'s scope note in
      `findings/security.md` when `9669ddf` moved three scenario titles off the
      events they named:
      `leaving_a_stoa_closes_its_channel_and_no_other` →
      `closing_one_stoas_channel_closes_that_one_and_no_other`,
      `shutdown_closes_every_open_channel` →
      `closing_every_open_channel_yields_each_channels_identifier`,
      `a_stoa_can_be_rejoined_without_a_restart` →
      `a_channel_closed_can_be_reopened_under_the_same_identifier`. No assertion
      changed. Each keeps its old name in a comment saying what the claim was and
      why it had no site, so the rename does not erase the reason for it.
- [x] Gates: `rustfmt --check` with `skip_children=true` on the one changed file
      clean, `clippy --all-targets -D warnings` clean, suite green. **744 tests
      passing, from 742** — exactly the two new tests, none ignored and no doc-test
      registered, so the count gate's `ran == declared` still holds.

**One process note, because it cost a verification step rather than nothing.** A
`spec-writer` committed `9669ddf` to this branch **while this `tester` stage was
running in the same worktree**, which the flow forbids: at most one of the three
writers holds a piece at a time, precisely because a `tester` mutates
implementation code it does not own. Nothing was lost — the collision was caught by
diffing rather than trusting the tree, every mutation had already been reverted, and
the two agents' files did not overlap — but the tip moved from `654a396` to `9669ddf`
mid-stage and the uncommitted spec delta was visible in this tree as though it were
this stage's own work. Recorded so the next dispatch does not read a clean outcome as
evidence the overlap was safe.

## 13. Closed the design-review findings, and merged `origin/main` a third time

- [x] **Merged `origin/main` at `733544d`** (two commits: #50 stoa-lifecycle,
      #59 the closer), for the same staleness reason §9 records. Only `docs/PLAN.md`
      conflicted, in §9.2's MVP list, and it was the one place the design reviewer
      predicted: `main` rewrote items 6 and 7 to say joining takes the address **and**
      the genesis record, while this branch struck through item 8 because
      `op-transport` now specifies receiving. Resolved by keeping `main`'s 6 and 7
      verbatim and this branch's 8 — taking either side whole would have dropped the
      other's correction, and the joining rewrite is the half that regresses
      *silently*, restoring a wrong claim about what a user needs in order to join.
- [x] Verified the rest of PLAN.md auto-merged as additions **onto** `main`'s text
      rather than over it, hunk by hunk against the named commit: §4.8's `#4116`
      withdrawal and shared-node close argument, and §9.1's publish-obligation
      rewrite, all survive with this change's strike-throughs layered on top.
- [x] Verified the diff **naming the merged commit** rather than `origin/main`:
      `git diff 733544d HEAD --stat` is pure insertions, so the phantom deletions
      (`membership.rs` −1842, the `stoa-membership` spec −414, `ci.yml` −89) are gone.
      The `origin/main` form is the one that gives a false alarm once main moves again.
- [x] **`nix build .#lgx` passed** — run because a clean auto-merge can still produce
      a tree that compiles nowhere, and `cargo test` cannot see it: `rust-lib/src/lib.rs`
      is entirely `#[cfg(logos_scaffold)]` and #50 touched that file. The one code
      file that auto-merged was `dialectica-core/src/lib.rs`, and the merge added a
      single `pub mod transport;` line, so there was no room for the identical-body
      duplication that cost the stoa piece 23 compile errors behind four green gates.
- [x] Gates on the merged tree: **821 tests passing** (795 + 26), from 744 pre-merge.
      The +77 is `main`'s membership suite arriving, not this change growing.
      `clippy --all-targets -D warnings` clean, `rustfmt --check` with
      `skip_children=true` clean on the one changed file,
      `openspec validate op-transport --strict` valid.
- [x] `findings/design-review.md`'s three entries closed — all three **fixed**, none
      rejected or deferred. The phantom-type entry turned up **two instances beyond
      the two reported**, because the reviewer's grep covered `design.md` only and the
      bare `Refusal` name was also in `design.md`'s section heading, its rendering of
      the `receive` signature, and two lines of `tasks.md`. Durable reasoning moved
      into `design.md` ahead of `findings/` being deleted: why the type is
      `InboundRefusal`, why `Publishable` is `#[must_use]` with the
      two-unpredicted-sites measurement, and why `close_all` sorts.
