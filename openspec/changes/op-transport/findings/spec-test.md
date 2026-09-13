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

- [x] **`spec-writer`** — the requirement "An oversized payload is refused, and
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

      **FIXED — `spec-writer`, accepted in full.** The finding is right that a
      requirement asking for a comparison against a value that does not exist is
      a spec defect, and the rewrite is the one proposed.

      - The scenario now reads *"the configured maximum is compared against a
        hardcoded 150 KiB written independently of it / THEN they are equal / AND
        a local edit to the configured maximum fails this rather than passing
        quietly"*. That is what the existing test does, so the test now covers a
        requirement it can fail for the reason the requirement gives.
      - The requirement carries a new paragraph saying plainly that the pin is
        what is checkable and agreement with the network is not, and that a
        scenario claiming the two are compared would be comparing the constant
        against itself — the finding's own sentence, kept because it is the thing
        a future reader needs in order not to reinstate the old wording.
      - A new scope exclusion names the half that is out of reach: *"That the
        message-size limit contracted below equals what the network validates
        against"*, with the reason (no transport interface supplies one) and the
        consequence (a peer over-sending because of a drifted network limit is a
        failure this capability cannot detect).
      - **The requirement's title changed** — "and the limit is the transport's"
        → "against a limit pinned at 150 KiB" — because the old title asserted
        exactly what the body now denies. This is an `ADDED` delta not yet
        archived, so the rename costs nothing. **One stale reference remains, and
        it is not mine to edit:** `design.md:38` still quotes the old title.
        Left for `dev-writer`.

      What is **not** claimed: that 150 KiB is right. It is the figure the system
      was designed against, and it stays named in the requirement for the reason
      the old text gave (a drifted limit still satisfies a scenario probing only
      absurd sizes). Whether the network still validates at that figure is now an
      acknowledged gap instead of a false assertion.

- [x] **`spec-writer`** — the requirement "Channel identity is a pure function of
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

      **FIXED — `spec-writer`, and the mutation is the reason this was taken as a
      spec defect rather than a coverage gap.** Mutation 4b is the measurement
      that settles it: a value stable within a peer and different between peers is
      exactly the silent partition the requirement's justification names, and a
      single-process test cannot vary it, so a scenario asking a peer to vary "a
      different peer identity" asks for something no test at this surface can do.
      Restated as the finding proposes, in three parts:

      - **The scenario is split.** *"Identity does not vary with local state"* is
        gone. In its place: *"Identity does not vary with the peer's history"* —
        channels opened and closed, ops stored, time passed between two
        derivations, which is what the existing test actually witnesses and can
        fail on; and *"The derivation takes the Stoa address and nothing else"* —
        the construction half, checked by reading what the derivation accepts.
      - **The requirement says how it is held**, in a new paragraph that names why
        the between-peer half is not observable from one peer ("a peer has one of
        each, and a derivation consulting one would agree with itself every time
        it was asked") and then discharges it in two stated obligations: by
        construction (the address is the only input, so there is no parameter such
        a value could enter through) and by the pinned known answer.
      - **The pin is named as the guard it turned out to be.** The pinned-answer
        scenario gains *"AND a derivation that had come to consult a value
        differing between peers fails it too, since such a value cannot equal the
        independently derived one"* — which is precisely what mutation 4b
        demonstrated, now written as a requirement instead of left for a reader to
        infer from a test failure message that says "the derivation changed".

      Two consistency edits the finding did not ask for but the change forced,
      because `validate --strict` would not have caught either:

      - The requirement's justification previously ended *"so the property has to
        hold by construction rather than be checked"*, which contradicted the new
        paragraph's reliance on a pin that does check it. It now reads *"held
        before a peer runs rather than reported after — by what the derivation
        accepts, and by a pin against a value nothing in this implementation
        produced"*.
      - The scope exclusion *"That two peers deriving one Stoa's channel identity
        actually meet"* claimed the pure-function property "is checkable" without
        qualification. It now says how it is held, and adds that comparing one
        peer's derivation against another's is itself out of reach — "which is why
        the requirement below says how it is held instead of asking for a
        comparison no peer can make".

      **No test change requested here**, and none made:
      `identity_does_not_vary_with_local_state` already tests the history half
      honestly, including the comment saying so. What was wrong was the
      requirement it was answering. Renaming that test to match the new scenario
      name would be an improvement and is `tester`'s call, not a defect.

- [x] **`spec-writer`** — the requirement "The delivery node is shared and is
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

      **FIXED — `spec-writer`, taking the second of the two options offered.** The
      finding is right on both halves: the requirement is a real constraint worth
      stating, and leaving it out of the scope list while the list excludes five
      other node-dependent properties reads as a claim that it is covered. It is
      not excluded — it is restated as what it actually is.

      - **Three scenarios became one, and the one names what is read rather than
        what is run.** *"Shutdown does not stop the node"*, *"Leaving a Stoa does
        not stop the node"* and *"The node is created once"* are replaced by *"No
        stop call exists in this application's lifecycle handlers"*: WHEN the
        handlers run on shutdown and on leaving a Stoa are examined for what they
        call, THEN neither stops the node, neither creates one, and the shutdown
        handler closes the channels this peer opened. That is checkable by reading
        the handlers, which is the only place the call could appear.
      - **The requirement says why it is stated here and discharged elsewhere**:
        no part of this capability's surface starts, stops or counts nodes, so the
        obligation binds the adapter's lifecycle handlers. Stated where the
        reasoning lives; met where the call would be made.
      - **It says outright that a peer cannot observe compliance**, and that the
        obligation is therefore a prohibition on the code rather than on an
        outcome — including the finding's own scenario as the thing it is
        admitting: *"A change that adds one satisfies every other requirement in
        this capability and breaks this one, and nothing in this capability's
        surface will say so."*
      - **A scope exclusion was added** — *"Observing that the shared delivery
        node was left running"* — pointing at the requirement and saying what
        observing the failure would take (a second module in the same context
        losing its delivery). That closes the asymmetry the finding identified.

      **Rejected: excluding the requirement outright.** The finding offered that
      as the first option and it is the wrong one here. A stop call is the kind of
      thing a later change adds in one line while every gate stays green; removing
      the requirement removes the only written record that it must not, and the
      constraint's cost falls on another module's users rather than on ours. A
      requirement that is checkable by reading is weaker than one checkable by
      running, and it is not nothing.

      **The finding's scenario stands undefended and is worth repeating**: a
      `nodeStop()` added to `lib.rs`'s shutdown handler passes all 562 tests. The
      new scenario is what a reviewer reads the handler against; no test closes it.

- [x] **`tester`** — the store-failure path is unreached: `Refusal::Storage` and
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

      **Fixed** in `89070d6`, taking the suggested fixture. `AppendFailsLog` is a
      `MemoryOpLog` whose `append` refuses and whose **reads delegate to the real
      log** — a fake failing on read too could not witness "the op is NOT in the
      log", which is the half of the contract that separates `NotStored` from
      `NoChannel`, where the op IS stored.

      Three tests, each run against the mutation it names before being trusted.
      Predicted and observed agreed in all three:

      - `a_store_failure_on_receive_is_refused_as_a_store_failure_and_not_as_a_forgery`.
        The payload is a valid, verifying op naming the right Stoa, so every
        earlier guard passes and the append is the only thing that can refuse.
        Under this finding's own mutation — `map_err(Refusal::Storage)` →
        `map_err(|_| Refusal::FailsVerification)` — it fails with *"a store
        failure was reported as FailsVerification, which sends the reader looking
        in the wrong place"*.
      - `a_publish_that_could_not_store_is_not_reported_as_a_missing_channel`. The
        channel is open, so `NoChannel` is not a reachable answer and the append
        is the only failure left. Under `map_err(PublishError::NotStored)` →
        `map_err(|_| NoChannel { stoa, id })` it fails naming the wrong variant.
      - `the_two_ways_a_publish_fails_disagree_about_whether_the_op_exists`. Pins
        the distinction rather than each variant, since what a caller acts on is
        which of the two it got — and asserts the asymmetry the variants exist
        for: after `NoChannel` the op is readable, after `NotStored` it is not.
        Under the same mutation the two failures compare equal.

      **One thing the mutation run showed that is worth more than the fix.**
      Under the publish mutation,
      `publishing_without_an_open_channel_fails_and_opens_nothing` — the test this
      finding calls vacuous — **still passes**. A store failure reported as a
      missing channel breaks neither assertion that existed before. That is this
      finding's diagnosis confirmed by measurement rather than accepted on
      argument, and it is the reason the new tests assert against a *reachable*
      wrong answer rather than adding a fourth guard to the old one.

      Its `!matches!(err, NotStored(_))` is left in place. It is now non-vacuous —
      `AppendFailsLog` makes `NotStored` constructible within the module — though
      it is still not the assertion that would catch the mutation, which is why
      the two new publish tests exist beside it rather than instead of it.

- [x] **`spec-writer`** — one `// NO SPEC:` marker, on
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

      **FIXED — `spec-writer`, adopting the finding's proposal exactly, and its
      diagnosis of the design.md argument is upheld.** A new requirement,
      **"A successful publish is a statement about the local log and nothing
      more"**, placed after the publish requirement it qualifies:

      - **The checkable positive.** A successful publish means the op is in the
        local log and its bytes were handed to the transport, and **no field of
        what a publish reports may carry, imply or be documented as carrying a
        delivery outcome**. Two scenarios: *"A send the transport accepted is not
        a delivery"* (what is reported identifies the op, names the channel,
        carries the bytes — and no field of it reports whether a peer received it)
        and *"A publish reports success on the strength of the log alone"*. Both
        are what `a_send_that_the_transport_accepted_is_not_a_delivery` already
        asserts, so **the `// NO SPEC:` marker can come down** — that is
        `dev-writer`'s or `tester`'s line to delete, not mine, and it should go
        with a pointer to the requirement name.
      - **The negative the publish path needs**: *"A publish SHALL NOT be reported
        as having failed on the strength of a delivery outcome."*
      - **The obligation is named rather than left silent**: an op the transport
        errored on, or that never propagated within some bound, has to become
        visible, "or this contract converts a loud failure into a silent one" —
        stated as an obligation this capability does not discharge, with what
        meeting it needs (state outliving the call, an association from the send
        handle to the op, a clock to bound the wait) and why that is a component.
      - **The scope exclusion was corrected, not removed.** *"That a published op
        reaches another peer"* now carries: *"Excluding the observation is not
        excluding the obligation"*, pointing at the new requirement. The finding's
        distinction — observing delivery is a different question from what a
        publish call claims — is the one that exclusion was blurring.

      **The false impossibility is upheld as false.** `design.md`'s *"cannot be
      discharged at this boundary at all"* is a scoping decision in an
      impossibility's clothes, and the tell is the finding's: the same paragraph
      specifies the solution. What is true is the narrower claim — a synchronous
      reply cannot carry an asynchronous outcome — and the new requirement states
      *that*, as the reason the reply must not be shaped as though it could.
      **`design.md` is not mine to edit**; its Decisions section should say "not
      built here, and here is the seam" rather than "cannot be done". Flagged for
      `dev-writer`.

      **A correction to the brief that dispatched me, and it matters for the
      pointer.** The brief said `docs/PLAN.md` §9.2's publish-path follow-ups
      "already record that obligation landing on this capability" and to read them
      on `origin/main`. **They are not on `origin/main`.** `git show
      origin/main:docs/PLAN.md` has no `channelMessageError` or `messagePropagated`
      in §9.2 at all; the follow-ups are on the unmerged local branch
      `docs/publish-followups` (`962b746`, *"Record two publish-path follow-ups…"*).
      I read that commit's diff. Its text is the owner's decision and it does say
      *"The obligation lands on `op-transport`"*, so the two now agree in
      substance — the spec requirement was written against it deliberately,
      including its two facts (the outcome arrives asynchronously; a reply could
      not carry it) and its conclusion (the bound and what a view shows are this
      capability's to specify).

      **Two consequences the runner should know.** The spec cannot cite a PLAN
      section number and does not, so nothing here breaks when that branch lands;
      but until it does, **PLAN.md carries no record of this obligation**, and the
      one place it is written down other than that unmerged branch is now this
      spec requirement. And `docs/UI-BRIEF.md` will need the rendering obligation
      — a successful publish is not "posted" — which `docs/publish-followups`
      itself anticipates. Not done here: this piece does not touch the brief, and
      a brief edit against an unmerged PLAN change is the stale-brief failure
      CLAUDE.md warns about, from the other direction.

      **SUPERSEDED, same session — §9.2's follow-ups reached `origin/main` while
      this was being written**, via PR #51 (authoring), which carried the
      cherry-picked `962b746`. `git grep -c messagePropagated origin/main --
      docs/PLAN.md` returns 4; the paragraph above was true when written and is
      not now. Merged again and **checked the two texts agree rather than assuming
      it, which found one real disagreement and two overreaches of mine.** All
      three are mine to fix, and are fixed:

      - **The landed `content-authoring` spec (`openspec/specs/content-authoring/
        spec.md:37-40`) requires that a *failed handoff* still report the op as
        published.** My requirement said a refused handoff "is a failure of the
        publish call itself … and reportable as such" — a clause added for
        internal consistency with the no-open-channel refusal, and flatly
        contradicting a spec that names `op-transport` as the obligation's owner.
        Corrected: **neither** a delivery outcome **nor** a failed handoff may be
        reported as a failed publish, with the reason the two differ only in when
        they are knowable, and with the no-open-channel case explained as a
        distinction between refusals of a publish that never stored anything —
        which is what it actually is, re-read at spec line 207 rather than
        recalled.
      - **My opening sentence overreached.** It said a successful publish means
        the op is in the log *and that its bytes were handed to the transport* —
        the second half being exactly what `content-authoring` forbids reading into
        it. Now: the log, and that is the whole of what it means, with the transport
        having accepted the bytes explicitly excluded alongside a peer having
        received it.
      - **I nearly committed the failure this brief was about.** Reconciling the
        above, I wrote a scenario *"A handoff that fails does not make the publish
        a failure"* — then checked it against the surface and found `publish`
        **returns** a `Publishable` and never sends (`transport.rs:573-600`), so no
        handoff failure is reachable from it and the scenario could not be tested
        here. Replaced with what is checkable — that there is no route by which a
        handoff failure could unpublish the op, which is the structural property
        the function's shape already holds — and the requirement now says plainly
        that reporting the handoff correctly binds whichever caller performs the
        send, naming `content-authoring` as contracting that reply.

      **And the agreement is now closer than "does not contradict".** PLAN.md's
      landed text says the bound, and what a view shows for an op in flight versus
      one that never propagated, are **this capability's to specify** — which my
      original wording did not carry; it said only that the obligation is not
      discharged here, which disowns the specification along with the
      implementation. The requirement now names **three owed things** (the bound,
      what a peer records for an op in flight, what it records for one that never
      propagated), says they are owed here, and says none is met by this change —
      a scope statement rather than an impossibility, which is the distinction this
      whole finding turned on. PLAN.md's §9.2 paragraph is struck to match and
      points at the requirement, so the two now say the same thing from both ends
      rather than one anticipating the other.

      **Test count moved and not because of this**: 562 → 623, which is #51's
      authoring suite arriving with the merge. Spec edits moved nothing.

- [x] **`spec-writer`** — `the_content_topic_keeps_the_prefix_autosharding_reads`
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

      **FIXED — `spec-writer`.** The finding's framing is the one adopted: the test
      carried the reasoning and the contract should. Added to the requirement "A
      Stoa's ops travel on one reliable channel per Stoa", beside the
      no-human-readable-name rule it sits naturally with rather than in the
      derivation requirement, because it is a property of what the names *are*
      rather than of what they are derived *from*.

      - The requirement now states that both the content topic and the channel
        identifier **SHALL each begin with the literal `/dialectica/1/`**, that the
        network's autosharding hashes only the application and version segments
        and ignores the rest, and that this prefix is therefore what puts every
        dialectica Stoa on one shard.
      - It names the failure mode the finding measured: changing, shortening or
        version-bumping the prefix moves every adopting Stoa to a different shard
        from every non-adopting one — **"which is the same silent partition the
        next requirement exists to prevent, arrived at from the other direction"**
        — and so **"A change to this prefix is a network migration and SHALL be
        treated as one"**, which is the sentence aimed at the reader the finding's
        scenario describes.
      - Scenario *"Both names keep the prefix autosharding reads"*, including
        *"AND the check is against that literal rather than against whatever the
        implementation currently produces"* — the existing test already asserts the
        literal and says why in its comment, so it now covers a requirement.

      No PLAN section number reaches the spec; the autosharding fact is restated in
      one clause, which is the substance the §4.2 citation in the test comment was
      standing in for.

- [x] **`spec-writer`** — `docs/PLAN.md` on **`origin/main`** line 343
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

      **FIXED — `spec-writer`, in `docs/PLAN.md` on this branch, and this was the
      one finding worth taking first.** The contradiction is real: I re-read §3.3
      in context on `origin/main` and the clause reads as licence exactly as the
      finding says. The two sentences the finding quotes are now struck through and
      replaced, in §3.3 (worktree line ~343 after the merge):

      - *"Verification therefore happens on **read** … The store may hold junk;
        the reader never trusts it"* is struck, and followed by **"Specified, and
        this is not where verification happens — see the `op-transport` spec"**,
        naming the requirement ("Every inbound payload is validated before it
        reaches storage") and stating what it does instead: refuses an unauthentic
        op at the transport boundary rather than storing it for a reader to filter.
      - **The half that is still true is kept, because striking it whole would
        create a second wrong reading.** A reader that does not trust the store
        remains the right posture and remains why `op-log` verifies nothing on
        append. What is withdrawn is "the store may hold junk" as a description of
        ops arriving from a peer, and the replacement says so in those terms:
        reading it as licence to append unverified ops "is the forgery-storage
        failure that boundary exists to prevent".

      The two halves the finding identified — PLAN.md carried the `op-log` half and
      not the transport half — are now both carried, and in the same paragraph, so
      a reader arriving at §3.3 for the data model cannot pick up one without the
      other. No section number reaches the spec in either direction.

- [x] **`spec-writer`** — `docs/PLAN.md` on `origin/main` §4.1 lines 376-383
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

      **FIXED — `spec-writer`, and the finding's diagnosis of *what* is stale is
      the part worth recording, because it is not the quotes.** Both SDS quotes are
      accurate descriptions of SDS and are therefore kept verbatim; what was
      missing was that they describe a layer below the event dialectica receives,
      so §4.1 pointed a reader at a filter with nothing to filter.

      A new paragraph follows them in §4.1 (worktree line ~402 after the merge),
      opening **"Do not implement that SHOULD"**:

      - It says why: `channelMessageReceived` does not fire for a participant's own
        messages, so a self-filter keyed on the sender identifier "is a branch that
        never executes".
      - It carries the finding's own second-order consequence, which is the one a
        dev would not work out unaided — "a dev who writes it will believe their own
        ops are being deduplicated by it rather than by op id".
      - It points at the requirement by name ("A peer's own published op is not
        received back as an arrival") and states what follows from it: storing on
        publication is the only route by which a peer holds its own op.
      - It points at §11's trap list for the asymmetry itself **rather than
        restating it**, which is where the ~3,500-line separation the finding
        measured is closed. The reciprocal pointer was already added in §11 by the
        earlier shedding pass on this branch, so the two sections now reference each
        other instead of neither referencing the other.

      **Not moved into §4.1:** the `messageReceived`-fires-for-own-messages fact
      itself. It is a delivery-module trap that presents as a storage bug, which is
      §11's job, and duplicating it into §4.1 would create the two-copies-drift
      failure the next finding is about.

- [x] **`spec-writer`** — `docs/PLAN.md` on `origin/main` has **no reference to
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

      **FIXED IN PART, with two citations corrected and one item deferred with a
      reason.** The finding's premise held against `origin/main` and no longer
      does: the branch had **already** shed most of §4.1, §4.3, §4.4, §4.5, §9.2
      and §11 before this review ran — `git diff origin/main -- docs/PLAN.md` shows
      the pass, and the spec is now named eleven times. So the finding's line
      numbers are `origin/main`'s, and several of the copies it lists were struck
      before it was written. Item by item, against the worktree after the merge:

      **The pure-function rule — three copies, not four.** 478-483 and 601 were
      already struck and pointed at the spec. Newly struck here:

      - **565-567** (*"Re-creating a channel is allowed … no restriction on
        reopening"*) → struck, with **"Specified as a requirement rather than a
        permission"**. This is the finding's separate observation about
        reopen-being-stated-as-a-permission, and it is the same edit, so it is done
        here rather than twice.
      - **573-574** (*"The channel id stays a pure function of the addressed
        object. No epoch in it"*) → struck, "Specified above", with the reasoning
        below it kept under an explicit heading: **"Both exclusions' reasoning stays
        here, because the spec states the rule and not the judgement behind it."**
        That reasoning is the per-peer partition worked example and the
        whose-problem-is-this judgement about `logos-delivery` — which a spec cannot
        hold and which `design.md` for this change did not need, since the spec
        states the prohibition outright.

      **CORRECTION — 2245-2249 is not a fourth copy.** Worktree ~2286: *"The record
      carries **no per-peer value** — no epoch, no session counter."* I read the
      surrounding paragraph. It is about the **Stoa genesis record**, whose hash is
      the Stoa address — a different object, owned by `stoa-genesis`, applying the
      same principle and cross-referencing §4.3 for the channel id derived *from*
      that address. Striking it as an `op-transport` duplicate would delete a
      `stoa-genesis` requirement's reasoning. **Left as is**, deliberately.

      **The arrival-timestamp finding — one copy struck, two left with reasons.**

      - **4131-4138** (worktree ~4179) → struck. This is the genuine near-verbatim
        one the finding flagged, including the spec's own *"there is no wire
        timestamp on this event"*. Replaced with a pointer to both requirements by
        name. **Kept: the measurement and the pairing** — the value is a
        `CLOCK_REALTIME` read taken when the callback fires, and exactly one event
        reads a real wire timestamp, "which is why only that one shows §11's units
        divergence. The two traps are one divergence seen from both ends." That
        observation is not in the spec and should not be.
      - **572-584** (worktree ~608) was already annotated by the earlier pass,
        pointing at both requirements by name.
      - **CORRECTION — 2578 is not a restatement.** Worktree ~2623, in §7.2: *"The
        epoch stored against a decay-free score is therefore the op's Lamport
        timestamp, never a receive-clock reading."* That is relevance decay
        reasoning about what a projection may store, reaching a different conclusion
        from the same fact. Not a copy. **Left as is.**

      **node-shared-never-stopped (501-507 and 3898-3899)** — both already handled
      by the earlier pass: §4.3's behaviour line is struck and points at the spec
      while the shared-node argument stays as reasoning, and §11's entry carries
      "Contracted in the `op-transport` spec; kept here because it presents as a
      runtime failure in someone else's module." No further edit; re-read both to
      confirm rather than trusting the diff stat.

      **The sender-id rule (376-383, 569-570, 2338-2339).** 376-383 is closed by the
      finding above. 569-570 (worktree ~605) is *"No authenticity. `sender_id` is an
      application-chosen string"* inside §4.4's list of what **SDS** does not
      promise — a description of the transport, correctly placed, and the premise the
      spec's requirement is built on rather than a copy of it. **Left as is.** No
      third instance is present in the worktree; `grep` for the two phrases returns
      only §3.3's `self-asserted` (closed above) and §4.4's line.

      **DEFERRED — the §13 degraded-order text (4246-4249, worktree ~4310).** Not
      struck, and the reason is that it is not this capability's to shed: it restates
      **`op-ordering`'s** requirement, which the `op-transport` spec explicitly
      declines to restate. Shedding it means pointing at `op-ordering`'s spec — and
      §13 on this branch already records that `op-ordering`'s leading requirement
      **contradicts** the withdrawal above it and that *"resolving it needs its own
      change"*. Pointing a struck-through line at a requirement the same section
      calls an open contradiction would make PLAN.md worse. **Where it goes:** the
      change that resolves the `op-ordering` prohibition, which §13 already names as
      owing this. Flagged for the runner as a cross-piece item rather than left
      silent.

      **Also left: the `#4116` set-aside (485-491) and the Edge/Core cost analysis.**
      The finding's own "Areas that are clean" says these are reasoning a spec cannot
      hold and should stay. Agreed, and recorded here so a later sweep does not read
      the surrounding strikethroughs as a mandate to continue.

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
