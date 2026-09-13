# Findings — spec-test review, `op-transport`

Read: `openspec/changes/op-transport/specs/op-transport/spec.md` (456 lines) and
`dialectica/rust-lib/dialectica-core/src/transport.rs`'s `mod tests` (56 tests,
counted with `grep -c "    #\[test\]"`). `design.md` read only for the
delivery-outcome dead end, as the brief permitted. Implementation bodies read
only at the lines mutated in part 2 — `receive`, `publish`,
`ChannelIdentity::of`, `OpenChannels::close_all` — and the `Refusal` /
`PublishError` type declarations.

Baseline before any mutation: **562 passing, 0 failed, 0 ignored**, which
confirms `tasks.md` §6's count. Tree confirmed byte-identical after every
mutation (`git diff --stat` empty) and the full suite re-run green at the end.

`openspec validate --changes --strict` passes (3 items).

## Mutations run, and what each measured

Seven mutations. Five reproduce the author's `tasks.md` §6 table; two are new.

| # | Mutation | Result |
|---|---|---|
| 1 | inbound `timestamp` recorded as a Lamport value (`Arrival::from_parts(Some(ts), None)`) | **caught** by 6 tests, the two named for it included |
| 2 | `verify()` bypassed when `sender_id.len() == 64` — the sender id admitting a forgery | **caught** by `a_forged_sender_identifier_grants_nothing`, and by nothing else |
| 3 | Stoa comparison on **byte 0 only** | **caught only** by `two_stoas_sharing_an_address_prefix_are_not_confused`; `an_op_naming_another_stoa_is_refused` **passed** |
| 4a | `ChannelIdentity::of` consults a global call counter (an epoch changing every 4 calls) | **caught** by `identity_does_not_vary_with_local_state` |
| 4b | `ChannelIdentity::of` consults `std::process::id()` — local state **stable within a peer, different between peers** | **SURVIVED** `identity_does_not_vary_with_local_state`. Caught only by `the_derivation_is_pinned_to_a_known_answer`, and there as "the derivation changed" |
| 5 | `close_all` returns every id while removing only the first | **caught** by `shutdown_closes_every_open_channel` |
| 6 | size fencepost `>` → `>=` | **caught** by `a_payload_at_the_limit_is_not_refused_for_its_size` and `an_op_at_the_limit_is_admitted` |
| 7 | channel check moved **before** the append, so a publish with no channel loses the op | **caught** by `publishing_without_an_open_channel_fails_and_opens_nothing` and `publishing_on_another_stoas_open_channel_still_fails` |

Mutation 3 reproduces the author's report exactly and is the defect family this
project keeps hitting. It is **already handled** — the constructed
31-byte-shared-prefix fixture exists and is the only thing that kills it. No
finding; recorded because the measurement is the evidence that the fixture earns
its place.

Mutation 4b is the one new measurement that matters, and it is finding 1 below.

## Findings

- [ ] **`spec-writer`** — the requirement "An oversized payload is refused, and
      the limit is the transport's", scenario *"The limit's value is pinned
      against silent drift"* — **untestable as written**, and this is a spec
      defect rather than a coverage gap.
      The scenario says *"WHEN the configured maximum is compared against the
      transport's stated message limit / THEN they are equal"*. There is no
      transport-stated limit to compare against:
      `grep -rn "153600\|150 \* 1024\|maxMessage\|max_message\|MAX_MESSAGE"` over
      `dialectica/contracts/` returns nothing, so the delivery contract states no
      message limit at all. `the_message_limit_is_pinned_to_the_transports_stated_value`
      therefore compares `MAX_MESSAGE_BYTES` against `153_600` and against
      `150 * 1024` — the same constant written two ways. That is arithmetic, not
      agreement with a second party.
      **Scenario:** the network raises its gossipsub limit to 1 MiB, or lowers it
      to 128 KiB. Our constant is now wrong, every peer silently refuses or
      over-sends, and this test still passes — because both of its expectations
      are our own value. The test's own comment claims it guards "interop and not
      a local preference", which is the half it cannot do.
      Rewrite the scenario to say what is checkable — that the constant is pinned
      to a hardcoded 150 KiB so a local edit fails loudly — and move "equals what
      the network validates" into the out-of-scope list beside the other
      properties needing a live node. `tasks.md` §7 already states this honestly;
      the spec does not.
      **Severity: medium** — a green gate asserting an interop property it
      structurally cannot reach.

- [ ] **`spec-writer`** — the requirement "Channel identity is a pure function of
      the Stoa address", scenario *"Identity does not vary with local state"* —
      **not testable as written**, and the requirement's own prose says why while
      the scenario ignores it.
      The scenario names *"a different peer identity, a different count of prior
      opens, a different clock reading"*. A single-process test can vary the
      second and third; it cannot vary the first, and
      `identity_does_not_vary_with_local_state` admits as much — its
      `let _another_peers_key = a_key(200).public_key();` is an unused binding, a
      comment standing in for an assertion.
      **Measured:** I mutated `ChannelIdentity::of` to append
      `/e{std::process::id()}` to the channel id — local state that is *stable
      within one peer and different between peers*, which is precisely the silent
      partition the requirement's three-paragraph justification exists to
      prevent. `identity_does_not_vary_with_local_state` **passed**. The only
      test that failed was `the_derivation_is_pinned_to_a_known_answer`, and it
      reported "the channel id derivation changed" — it catches the mutation as a
      format change, not as local state participating.
      So the guard for this requirement is the hardcoded-pin test, not the test
      named for it. That is worth writing down rather than leaving a reader to
      infer: split the scenario into the part a test checks (stability across a
      peer's history changing around it — opens, closes, stores, time) and the
      part held **by construction** (`of` takes one `&Address` and there is no
      parameter a peer identity could enter through), and say the pinned
      known-answer test is what makes a construction change fail loudly.
      **Severity: medium** — the requirement whose violation the spec says
      "produces no error" is the one whose named test is weakest.

- [ ] **`spec-writer`** — the requirement "The delivery node is shared and is
      never stopped by this peer" — **three scenarios, no test, and none
      reachable from this capability's surface.**
      `grep -rn "node" transport.rs` returns one comment line.
      `grep -rn "node_stop\|stop_node\|NodeStop\|node_start\|create_node"` over
      `dialectica/rust-lib/` returns nothing. The node exists only in the adapter
      behind `cfg(logos_scaffold)`, which `cargo test` does not compile — so
      "the node is not stopped", "no additional node is created" and "shutdown
      does not stop the node" are assertions about calls never made in code no
      test reaches.
      **Scenario:** a later change adds a `nodeStop()` call to the shutdown
      handler in `lib.rs`. Every one of the 562 tests still passes, delivery dies
      for every other module in the context, and nothing in this suite noticed.
      The requirement may well be right to state — it is a real constraint — but
      the spec's "What is outside this capability" list excludes five other
      node-dependent properties and does **not** exclude this one, which reads as
      a claim that it is covered. Either exclude it there with the same
      reasoning, or state it as an adapter obligation the spec names as untested.
      **Severity: medium** — three scenarios currently read as covered and are
      not.

- [ ] **`tester`** — the store-failure path is unreached: `Refusal::Storage` and
      `PublishError::NotStored` are each constructed by the implementation
      (`transport.rs:482` and `:588`) but **no test drives either**, because there
      is no failing-log fixture. `MemoryOpLog` always succeeds.
      This makes two assertions vacuous rather than wrong:
      `publishing_without_an_open_channel_fails_and_opens_nothing`'s
      `assert!(!matches!(err, PublishError::NotStored(_)))` cannot fail, since
      nothing in the test's reach produces `NotStored`; and
      `every_refusal_is_reported_distinguishably` exercises `Refusal::Storage`
      only by constructing it by hand in `every_refusal_variant()`, which checks
      its `Display` and not that `receive` ever returns it.
      **Scenario:** the `map_err(Refusal::Storage)` at `transport.rs:482` is
      changed to `map_err(|_| Refusal::FailsVerification)`. A disk error now
      reports a forgery to the user and to the log — the exact "sends the reader
      looking in the wrong place" failure the spec's five-distinguishable-causes
      paragraph exists to prevent — and the suite stays green. Same for
      `publish`'s `NotStored`, where the spec additionally requires it be
      distinguishable from `NoChannel`.
      A `MemoryOpLog` wrapper whose `append` returns
      `Err(OpLogError::Storage(..))` on demand closes both. **Severity: medium.**

- [ ] **`spec-writer`** — one `// NO SPEC:` marker, on
      `a_send_that_the_transport_accepted_is_not_a_delivery`
      (`transport.rs:2024`): the spec does not say what happens when a published
      op never reaches a peer, and the author chose that `publish` reports the
      handoff and claims nothing about delivery.
      **The spec should cover it, and the chosen behaviour is the right thing to
      write down.** The spec's scope list already excludes *"That a published op
      reaches another peer"* — but that excludes observing *delivery*, which is a
      different question from what a *publish call* claims. The gap is that a
      successful publish currently means "stored locally" and nothing says so, so
      a caller — the UI most of all — is free to render it as "posted".
      `design.md` says the obligation *"cannot be discharged at this boundary at
      all"*, and for a pure function of its arguments that is correct and well
      argued. But the same paragraph then describes exactly how it is done — a
      `requestId`→op-id map, state outliving the call, a timeout with a clock —
      and concludes "a component, not a branch". That is *not built here*, which
      is a different claim from *cannot be done*; this project has had four
      "cannot" claims disproved on one branch. So the spec should state the
      positive requirement that is checkable at this boundary — a publish's
      success is a statement about the local log and carries no delivery claim,
      and no field of its result may be read as one — and name the delivery
      outcome as a separate obligation belonging to a later capability, so that
      it is a tracked gap rather than a silence.
      **Severity: medium** — an unstated default that the UI can misread as a
      delivery guarantee.

- [ ] **`spec-writer`** — `the_content_topic_keeps_the_prefix_autosharding_reads`
      pins an interop property **no requirement states**: that both the content
      topic and the channel id begin with the literal `/dialectica/1/`, because
      autosharding hashes only `application` + `version` and that head is what
      places every dialectica topic on one shard.
      This is an unmarked spec gap of the kind worth more attention than a marked
      one. The spec requires the topic be derived from the address and carry no
      human-readable name, and it pins the whole string in "The derivation is
      pinned against silent change" — but it never says *why the prefix shape
      matters*, so a future change that moved to `/dialectica/2/…` or
      `/dlx/1/s/…` would read as a version bump rather than as moving every Stoa
      to a different shard. The test carries the reasoning; the contract should.
      **Scenario:** a reader shortening the prefix to save topic bytes sees one
      test fail with "got /dlx/1/…", updates the literal, and re-shards the
      network. **Severity: low** — the behaviour is pinned; only the requirement
      is missing.

- [ ] **`spec-writer`** — `docs/PLAN.md` on **`origin/main`** line 343
      **contradicts** this spec's inbound-validation requirement, and it sits in
      §3.3, the section a reader goes to for the data model.
      Quoted verbatim from `git show origin/main:docs/PLAN.md`, lines 340-343 —
      I read the surrounding context rather than only grepping for it:
      > **Op authenticity is dialectica's job, not the transport's** (§6). A
      > forged op cannot be prevented from *arriving*: SDS has no membership and
      > `senderId` is self-asserted. Verification therefore happens on **read**,
      > filtering unsigned or badly-signed ops out. The store may hold junk; the
      > reader never trusts it.
      The spec now requires the opposite on this path: *"A payload arriving on a
      channel SHALL be validated **before** it is appended to the op log"*, and
      the scenario *"An unauthentic op is not admitted on the promise of a later
      check → it is refused at this boundary **AND** it is not stored for a
      reader to judge later."*
      **Scenario:** a dev implementing the next inbound path reads §3.3, learns
      that "the store may hold junk" is the design, and appends unverified ops —
      which is the forgery-storage failure `receive` exists to prevent, arrived
      at by following the document CLAUDE.md says to read *before any design
      decision*. Note the spec's Purpose already anticipates this exact
      confusion for `op-log` and resolves it (spec line 13); PLAN.md carries the
      `op-log` half and not the transport half, so as written it reads as licence.
      The fix is PLAN.md's: strike line 343's clause and point at this spec.
      **Severity: high** — a false statement about built behaviour, in the
      document that governs the next design decision.

- [ ] **`spec-writer`** — `docs/PLAN.md` on `origin/main` §4.1 lines 376-383
      is now stale in a way that would produce dead code.
      It describes SDS's own `sender_id` semantics accurately, including that
      *"the receive step is a SHOULD to 'ignore the message if it has a
      `sender_id` matching its own'"*. But this spec's requirement "A peer's own
      published op is not received back as an arrival" says the event never fires
      for own messages, so *"a peer relying on the sender identifier to filter
      out its own arrivals would be filtering something that never arrives —
      **which is why the sender identifier is not that filter here**"*. PLAN.md
      does carry the correct fact — line 3902, *"`messageReceived` fires for your
      own messages; `channelMessageReceived` does not"* — but it is ~3,500 lines
      away in §11's trap list and §4.1 never reconciles with it.
      **Scenario:** a dev reads §4.1, writes the self-filter it points at, and
      ships a branch that never executes — and worse, believes their own ops are
      being deduplicated by it rather than by op id. **Severity: medium.**

- [ ] **`spec-writer`** — `docs/PLAN.md` on `origin/main` has **no reference to
      `op-transport` at all** (grep returns zero hits), while it names every
      sibling spec — `op-log`, `op-format`, `op-ordering`, `stoa-genesis`,
      `identity` and others. §4.3 is consequently ~60 lines of *built* behaviour
      restated rather than pointed at, and several requirements are duplicated
      2-4 times over:
      - the pure-function/no-epoch rule at lines 478-483, 541-556, 601 and
        2245-2249 — **four copies**;
      - the arrival-timestamp-orders-nothing finding at 572-584, 4131-4138 and
        2578, one of them near-verbatim with the spec's *"There is no wire
        timestamp on this event"*;
      - node-shared-never-stopped at 501-507 and 3898-3899;
      - the sender-id rule at 376-383, 569-570 and 2338-2339.
      Also: lines 533-539 state reopen-without-restart as *"allowed"* — a
      permission — where the spec now contracts it with a scenario; and the §13
      degraded-order text at 4246-4249 restates what the spec deliberately
      declined to restate (*"is `op-ordering`'s requirement … and is not restated
      here"*).
      **Scenario:** two copies drift and the wrong one gets read — the failure
      `.claude/agents/README.md` names as the reason behaviour moves out of
      PLAN.md on landing. The `~~struck~~ + "Answered: see <spec>"` shape is
      already used correctly elsewhere in §13 (lines 4081-4087 for the ordering
      rule, and four more), so the pattern to follow is in the file.
      **Severity: medium** — no wrong behaviour today, but the shedding step
      this flow requires on landing has not happened for this capability.

## Areas that are clean

Stated in prose rather than as boxes, since none needs action.

**The two "must not be believed" fields are genuinely pinned, not
fake-asserted.** This was the brief's sharpest question and the answer is good.
The `inbound()` helper deliberately supplies a non-trivial sender id
(`"a-participant"`) and timestamp (`1_700_000_000_000_000_000`) rather than
`("", 0)`, with a comment saying why — a zero-value fixture could not tell "the
field is ignored" from "the field happened to be zero". The timestamp mutation
was caught by six tests. `the_timestamp_handed_in_does_not_change_what_is_recorded`
compares against the hardcoded `Arrival::unordered()` rather than against
another run's output, so a boundary that *scaled or offset* the timestamp still
fails. `what_is_recorded_does_not_vary_with_receive_sequence` compares two
runs' whole maps, and under mutation 1 it printed the same op recorded
`lamport: Some(0)` in one order and `Some(1)` in the other — the per-peer
divergence shown rather than argued. `the_sender_identifier_is_not_part_of_what_is_stored`
asserts over the op's whole wire form via a `windows()` scan rather than field
by field, which is the stronger shape.

**The refusal-distinguishability tests are the right shape.** Each of the four
decode/verify/mismatch fixtures asserts a positive variant *and* the negatives
it must not collapse into, and each carries a guard proving the fixture reaches
the check it names — `assert!(SignedOp::from_bytes(&payload).is_ok(), "the
fixture must decode, or this is the decode test")`, `assert!(!forged.verify(),
"the fixture must be an actual forgery")`, `assert!(elsewhere.verify(), "the
fixture must be authentic")`. Those guards are what stop the fixture drifting
into testing an earlier gate. `every_refusal_variant()`'s non-exhaustive `match`
makes the variant list fail to compile when one is added, which is the fix for
the drift `stoa.rs` recorded.

**`two_stoas_sharing_an_address_prefix_are_not_confused` and
`two_channels_sharing_an_id_prefix_are_not_confused`** are the defect family
handled properly: addresses **constructed** via `Address::from_bytes` to agree
in 31 of 32 bytes, with an assertion that the fixture really shares the prefix,
plus a positive control that this Stoa's own op is still admitted so the
comparison is not simply refusing everything.

**`the_derivation_is_pinned_to_a_known_answer`** is the strongest test in the
file. The expected Stoa address was derived outside this crate — the comment
carries the `printf` + `sha256sum` commands and the 48-byte preimage — and the
test first asserts the fixture address, so the two topic assertions cannot
silently pass against a changed fixture. I re-read the working: 32-byte prefix
`/dialectica/1/Address/Stoa` (26 chars) plus six NULs, then 16 bytes of record,
48 total. Consistent.

**`the_size_check_runs_before_the_decode`** is a real ordering test, not a
restatement: its payload is both over-long *and* undecodable, so it is the only
one of the three size tests that can tell the two orderings apart, and its
comment says exactly that. `an_op_at_the_limit_is_admitted` checks the
arithmetic (`assert_eq!(payload.len(), MAX_MESSAGE_BYTES)`) rather than assuming
the padding landed.

**`arbitrary_bytes_are_refused_without_a_panic`** is the best available at this
boundary, not a gap. A panic aborts, so "the call returned" is the only signal
there is; what makes the test worth having is the *input set* rather than the
assertion — every prefix of a valid op (not hand-picked lengths), every
single-byte mutation of the first 96 bytes, and lengths unrelated to the format.
That is what finds the field read without a bounds check. Its sibling
`a_hostile_channel_or_sender_identifier_does_not_panic` crosses 9 hostile
channel ids against the same 9 sender ids, and its comment correctly notes that
a `String` of raw invalid UTF-8 is unrepresentable in Rust and so not a
reachable input — an honest scope statement rather than a silent omission.

**`no_per_op_value_reaches_the_channel_identity`** asserts through the publish
path rather than only through the derivation, and I confirmed `publish`
re-derives from `op.op.stoa` rather than returning the open channel's id — so
the assertion is against a real derivation. The property is structural (`of`
takes one `&Address`), and here the structural argument is airtight in a way
mutation 4b showed it is not for local state, because a Stoa address genuinely
is the only input available.

**Spec self-consistency:** I read all 456 lines. No contradiction found. The
`op-log` boundary paragraph (spec lines 13) pre-empts the one reading that looks
like a contradiction — that a log which "decides nothing" is in tension with a
transport that refuses — and draws the line correctly: `op-log` contracts what
the log does with an op handed to it, this capability contracts which payloads
become such an op. The moderation pair at lines 446-456 reads as a contradiction
and is not, for the reason the requirement itself gives, and both halves have a
test (`an_authentic_moderation_op_from_a_non_moderator_is_admitted` and
`an_unauthentic_op_is_not_admitted_on_the_promise_of_a_later_check`).

**No requirements moved between capabilities** in this change — the spec is all
`## ADDED Requirements`, with no `REMOVED` half to check against, so part 4 of
my checklist does not apply.

**PLAN.md agrees on every value it states.** Worth saying so the shedding pass
does not have to re-audit them: the content topic format at line 367
(`/dialectica/1/s/<hex>/proto`), the 150 KiB cap at lines 585-586 and 608-610,
the best-effort/refcounted close caveats at 525-529, and the own-messages event
fact at 3902 all match the spec exactly. PLAN.md nowhere claims the transport
supplies ordering metadata — lines 561-563 list SDS's Lamport order under its
*own* promises and 572-584 immediately scope it as "internal to SDS, not as an
interface", which survives the spec intact. The `#4116` set-aside at 485-491 and
the Edge/Core cost analysis are reasoning a spec cannot hold and should stay.
Two shapes PLAN.md is silent on rather than wrong about: the channel-id format
(`/dialectica/1/c/<hex>` appears nowhere; line 368 says only "`channelId` = the
Stoa"), and the no-envelope rule — where line 374's "`threadId` and
`parentPostId` live in the **payload**" is loose enough to license the envelope
the spec forbids, since the spec's precise version is that they are inside the
signed op and the payload *is* that op.

## Requirement → test coverage

| Requirement / scenario | Test | Can it fail? |
|---|---|---|
| **One reliable channel per Stoa** | | |
| One Stoa yields one channel id and one topic | `one_stoa_yields_one_channel_id_and_one_content_topic` | yes — also asserts the two differ |
| Two Stoas do not share a channel | `two_stoas_do_not_share_a_channel` | yes |
| A Stoa's title does not appear in its topic | `a_stoas_title_does_not_appear_in_its_content_topic` | yes — title is a string that cannot occur in hex by coincidence |
| No per-op value reaches the channel identity | `no_per_op_value_reaches_the_channel_identity` | yes, via the publish path; property is structural |
| **Channel identity is a pure function of the address** | | |
| Deriving twice yields the same identity | `deriving_twice_yields_the_same_identity` | yes |
| Identity does not vary with local state | `identity_does_not_vary_with_local_state` | **partly — survives per-process local state (mutation 4b). Finding 2** |
| Reopening does not change the identity | `reopening_a_channel_does_not_change_its_identity`, `a_stoa_can_be_rejoined_without_a_restart` | yes |
| The derivation is pinned against silent change | `the_derivation_is_pinned_to_a_known_answer` | yes — independently derived expectation |
| *(unstated: `/dialectica/1/` autosharding head)* | `the_content_topic_keeps_the_prefix_autosharding_reads` | yes — **no requirement. Finding 6** |
| **The sender identifier is never an identity** | | |
| Does not establish authorship | `the_sender_identifier_does_not_establish_authorship` | yes |
| A forged sender id grants nothing | `a_forged_sender_identifier_grants_nothing` | yes — **killed mutation 2, alone** |
| One op under two sender ids is one op | `one_op_under_two_sender_identifiers_is_one_op` | yes — checks `Stored` then `AlreadyPresent` |
| Not part of what is stored | `the_sender_identifier_is_not_part_of_what_is_stored` | yes — whole-wire-form scan |
| **The arrival timestamp orders nothing** | | |
| Not recorded as ordering metadata | `the_arrival_timestamp_is_not_recorded_as_ordering_metadata` | yes — killed mutation 1 |
| The timestamp handed in changes nothing | `the_timestamp_handed_in_does_not_change_what_is_recorded` | yes — 7 values incl. `i64::MIN/MAX`, vs hardcoded `unordered()` |
| Does not vary with receive sequence | `what_is_recorded_does_not_vary_with_receive_sequence` | yes — compares two runs' maps |
| **An arrival carries no ordering metadata** | | |
| Every arrival recorded as unordered | `every_arrival_over_this_transport_is_recorded_as_unordered` | yes — all 5 op kinds |
| No ordering value fabricated from what arrived | `the_arrival_timestamp_is_not_recorded_as_ordering_metadata`, `a_valid_op_is_stored_and_recorded_as_unordered` | yes |
| **A locally-authored op is stored before publishing** | | |
| A published op is in the local log | `a_published_op_is_in_the_local_log` | yes |
| A send failure does not lose the op | `a_send_failure_does_not_lose_the_op` | structural (`publish` returns bytes, never sends); consequence pinned |
| Own op carries no ordering metadata of its own | `a_peers_own_op_carries_no_ordering_metadata_of_its_own` | yes — compares own vs received arrival directly |
| The bytes published are the bytes stored | `the_bytes_published_are_the_bytes_stored` | yes |
| Publishing without an open channel fails, opens nothing, op still stored | `publishing_without_an_open_channel_fails_and_opens_nothing`, `publishing_on_another_stoas_open_channel_still_fails`, `a_publishable_carries_the_channel_the_op_belongs_on` | yes — killed mutation 7; the second rules out "is any channel open" |
| *distinguishable from a transport failure on an open channel* | asserted as `!matches!(NotStored)` | **vacuous — nothing produces `NotStored`. Finding 4** |
| **The channel carries an op's wire form and nothing else** | | |
| A payload is one op's wire form | `the_bytes_published_are_the_bytes_stored` | yes |
| The whole payload is what is decoded | `the_whole_payload_is_what_is_decoded` | yes — asserts the prefix op is *not* stored |
| **Every inbound payload is validated before storage** | | |
| Unknown channel refused | `a_payload_on_an_unknown_channel_is_refused`, `two_channels_sharing_an_id_prefix_are_not_confused` | yes — fixture is a valid op, so only the lookup can refuse |
| Does not decode → refused distinguishably | `a_payload_that_does_not_decode_is_refused_distinguishably` | yes |
| Signature does not verify → refused distinguishably | `an_op_whose_signature_does_not_verify_is_refused_distinguishably` | yes — tampers the body, so the decode is provably reached |
| Key does not bind to claimed author → refused | `an_op_whose_key_does_not_bind_to_its_claimed_author_is_refused` | yes — valid signature, wrong key |
| Refusal leaves nothing behind | `refusal_leaves_nothing_behind_and_the_channel_keeps_working` | yes — three refusals then a valid op |
| A valid op is stored | `a_valid_op_is_stored_and_recorded_as_unordered`, `an_op_at_the_limit_is_admitted` | yes |
| Five causes reported distinguishably | `every_refusal_is_reported_distinguishably` | yes for `Display`; **`Storage` never reached through `receive`. Finding 4** |
| **An op is refused unless it names the channel's Stoa** | | |
| An op naming another Stoa is refused | `an_op_naming_another_stoa_is_refused` | yes — but **survives a byte-0 comparison (mutation 3)** |
| *(whole-address comparison)* | `two_stoas_sharing_an_address_prefix_are_not_confused` | yes — constructed 31-byte prefix; **the only killer of mutation 3** |
| A cross-Stoa copy joins the peer to nothing | `a_cross_stoa_copy_does_not_join_the_peer_to_anything` | yes — checks `len()`, `is_open`, `stoa_of` |
| An authentic op is still refused on the wrong channel | `an_op_naming_another_stoa_is_refused` (asserts `elsewhere.verify()`) | yes |
| **An oversized payload is refused** | | |
| Over the limit → refused as over-long | `a_payload_over_the_limit_is_refused_distinguishably`, `the_size_check_runs_before_the_decode` | yes |
| At the limit → not refused for size | `a_payload_at_the_limit_is_not_refused_for_its_size`, `an_op_at_the_limit_is_admitted` | yes — killed mutation 6 |
| The limit's value pinned against drift | `the_message_limit_is_pinned_to_the_transports_stated_value` | **no — compares the constant with itself. Finding 1** |
| **Receiving never aborts the process** | | |
| Arbitrary bytes refused without a panic | `arbitrary_bytes_are_refused_without_a_panic` | by aborting only — best available; input set is what earns it |
| Hostile channel/sender id does not panic | `a_hostile_channel_or_sender_identifier_does_not_panic` | as above, 81 combinations |
| The peer keeps receiving after a refusal | `the_peer_keeps_receiving_after_a_refusal`, `refusal_leaves_nothing_behind_and_the_channel_keeps_working` | yes |
| **The delivery node is shared and never stopped** | | |
| Shutdown does not stop the node | — | **UNCOVERED. Finding 3** |
| Leaving a Stoa does not stop the node | — | **UNCOVERED. Finding 3** |
| The node is created once | — | **UNCOVERED. Finding 3** |
| **A channel is closed on leaving and on shutdown** | | |
| Leaving closes its channel and no other | `leaving_a_stoa_closes_its_channel_and_no_other` | yes |
| Shutdown closes every open channel | `shutdown_closes_every_open_channel` | yes — killed mutation 5 |
| A Stoa can be rejoined without a restart | `a_stoa_can_be_rejoined_without_a_restart` | yes |
| Closing keeps the ops received on it | `closing_a_channel_keeps_the_ops_received_on_it` | yes — checks `len`, `get` and `iter_stoa` |
| *closing is best-effort, release not observable* | `closing_a_channel_that_is_not_open_is_not_an_error` | partial — idempotence pinned; the reference-counted release is correctly out of scope per the spec |
| *(also: the open-set invariant)* | `an_open_channel_records_the_stoa_its_identity_was_derived_for`, `opening_a_channel_twice_is_one_channel` | yes — this is what the Stoa-mismatch refusal compares against |
| **A peer's own published op is not received back** | | |
| A published op does not depend on being received back | `a_published_op_does_not_depend_on_being_received_back` | yes for the requirement's substance; the transport's own non-delivery is unobservable here |
| An op already held arriving again is one op | `an_op_the_peer_already_holds_arriving_again_is_one_op` | yes — asserts `AlreadyPresent` *and* that the first arrival's metadata survives |
| *(no spec: a publish claims nothing about delivery)* | `a_send_that_the_transport_accepted_is_not_a_delivery` | yes — **`// NO SPEC:`. Finding 5** |
| **This capability decides nothing beyond admitting** | | |
| Authentic moderation from a non-moderator is admitted | `an_authentic_moderation_op_from_a_non_moderator_is_admitted` | yes — asserts the author is *not* in `Moderators::of(&genesis)` |
| An unauthentic op is not admitted on a later-check promise | `an_unauthentic_op_is_not_admitted_on_the_promise_of_a_later_check` | yes |
