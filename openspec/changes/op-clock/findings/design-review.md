# Design review — `op-clock`

Dimension: **design record**. Whether the code took the decisions `design.md`
records, whether decisions worth recording were recorded, and whether anything
contradicts `docs/PLAN.md` as it stands on `origin/main` (`9357417`).

## The decisions are in good shape

Stated plainly rather than padded into boxes below: I verified a sample of the
nine recorded decisions against the code and **found no case where the code
contradicts a recorded decision**. The ones I checked structurally:

- **Decision 1** (`Option<OpClock>` rather than two loose `Option`s) — holds.
  `op.rs:593` is `pub clock: Option<OpClock>`; `canonical_bytes` writes
  `VERSION_2` and sixteen bytes from one `match` arm (`op.rs:719-739`), so
  version and field presence really are one fact and a version-1 op's bytes are
  unchanged by construction, not by a recorded-constant test.
- **Decision 2** (`cmp_ops` cannot name an `Arrival`) — holds, and is the
  strongest thing in the change. `OpEntry` is `{ counter: Option<u64>, id }`
  (`arrival.rs:314`); `cmp_tiebreak` is gone. The spec scenario "every input is
  carried inside the ops being compared" is now a statement about the type.
- **Decision 3** (ascending fold) — holds. `clock_from_counters`
  (`arrival.rs:252-276`) sorts before folding, and `arrival.rs:227` carries the
  correctness argument in the code itself. The `counter - clock` direction is
  deliberate against debug-build overflow and says so.
- **Decision 8** (`score_epoch` takes the counter) — holds, and is pinned by a
  test that discriminates the right thing:
  `the_score_epoch_is_the_ops_counter_and_not_the_recorded_lamport_value`
  (`log/sqlite.rs:1433`) uses a fixture where the counter and the recorded
  arrival **disagree**, so it fails if the old value is restored.
- **Decision 9** (`check_layout`'s column list) — the near-miss is recorded at
  `design.md:200-206` **and** pinned by
  `a_store_this_build_wrote_passes_its_own_layout_check` (`log/sqlite.rs:1026`),
  whose comment states why it is not a formality. This is handled correctly.

The two withdrawn prohibitions are each retired **explicitly, in the delta that
owns them**, with the original reasoning preserved and re-aimed rather than
deleted — `op-ordering/spec.md:230-236` and `op-format/spec.md:124`. Neither was
added alongside text that still forbids it. The PLAN.md reasoning migration is
likewise thorough: §13's Lamport and `createdAt` entries, §5.7, §6 and §9.1 §8
are all struck through with pointers to the spec, and I found no duplicated
reasoning left in both documents.

The `Placed`/`Placed::at` type (`thread.rs:227-246`) makes "every item has a real
position" a compiler property rather than a loop's discipline. That is a good
decision; see the last finding about it not being recorded.

## Findings

- [ ] **`dev-writer`** — `authoring.rs:51-58` states as live a premise this
      change falsifies. The `Published` doc comment reads: *"An op id is a
      function of the op's bytes and those bytes carry no timestamp and no nonce,
      so one identity publishing the same content twice produces one op."* Both
      halves are now false — the bytes carry a counter **and** a timestamp
      (`op.rs:593`), and the whole point of `content-authoring`'s modification is
      that the same content authored twice publishes **two** ops. **Verified:**
      `git diff origin/main...880bb2b -- .../authoring.rs` shows lines 51-58
      untouched; the change edited the `use` statements two lines below it and
      added `Authorship` thirty lines down, so this was read past rather than
      missed for lack of proximity. This is the exact class `tasks.md:76-83`
      enumerated ("four files currently carry written arguments against exactly
      what this change builds") — `op.rs`, `arrival.rs`, `transport.rs` and
      `sqlite.rs` were all corrected; this fifth site was not on the list and was
      not found. It matters more than an ordinary stale comment because it sits
      on the return type of the publish path and explains a field (`appended`)
      whose meaning genuinely did change.

- [ ] **`dev-writer`** — `docs/PLAN.md:2025-2033` (§7.2 rule 5) still argues from
      a prohibition this change withdrew, and is the **only** surviving instance.
      It reads: *"No value in the system expresses a post's age: `op-format`
      forbids an op from carrying a wall-clock timestamp ('a wall clock is a
      field the adversary sets') [...] So decay is blocked on the transport
      supplying an authorship time, which nothing currently plans to."* All three
      clauses are now false: the op carries a wall-clock, `op-format`'s
      prohibition is withdrawn as to exactly that field
      (`specs/op-format/spec.md:124`), and decay is no longer blocked on the
      transport. **Verified:** `grep -n "op-format forbids"` over the branch's
      PLAN.md returns this line and nothing else; `grep -n "no Lamport value
      reaches"` returns only struck-through text. So the sweep was nearly
      complete and stopped one section short. This is the section that governs
      `score_epoch`, which **decision 8 is about** — so the change's own recorded
      decision and PLAN's live text now disagree about whether an age exists.
      Note the correct resolution is not simply "delete the paragraph": rule 5's
      *conclusion* (the stored epoch is the counter, never a receive-clock
      reading) survives and is what decision 8 implements. It is the premise that
      has to be re-aimed — and the wall-clock must be explicitly named as **not**
      the new age input, or the next reader takes the withdrawal as permission.

- [ ] **`dev-writer`** — the tiebreak choice is **not a recorded decision**,
      though it is a real one with a rejected alternative. `design.md:53` mentions
      only that `cmp_tiebreak` "is deleted with the transport message-id tiebreak
      it implemented" — as a consequence of decision 2, not as a choice. The
      reasoning that makes it a decision (the op id is a pure function of the
      op's own bytes; the transport message id was the alternative and fails
      because it is absent on every op, so it is "a tiebreak that never fires")
      lives in `proposal.md:53-59` and in the spec delta at
      `op-ordering/spec.md:240,255`. Under the repo's own division of labour that
      is the wrong place: `proposal.md` argues *whether*, `design.md` records
      *why this one*. A reader reconstructing why the order tiebreaks on a hash —
      which carries no recency, and looks arbitrary — will not find it in
      Decisions. Missing part: **the alternative and what ruled it out**, which
      the guidance names as the part that rots first.

- [ ] **`dev-writer`** — accept-and-clamp over reject-at-the-boundary is not
      recorded as a decision, and the censorship argument for it appears in
      `design.md` **nowhere**. **Verified:** `grep -n "censorship\|reject"` over
      `design.md` returns three hits, none of them this — line 88 is about the
      fold, 109 about the advance bound, 212 about the composer. The reasoning
      exists and is good, but only in `proposal.md:74-78` and in module prose
      (`asserted_time.rs:25-39`, `op-ordering/spec.md:89`). This is the decision
      with the largest blast radius in the piece: it is why a malformed clock
      cannot be used to make a peer invisible, and it is the reason the *whole*
      clamp lives at read time rather than at the boundary — decision 6 records
      the `now_ms` injection mechanics but takes "clamping is read-time" as
      given rather than as the thing chosen. An entry here should carry the
      constraint (a peer with a skewed clock would be silently dropped by
      everyone, indistinguishable from moderation nobody performed), the
      alternative (refuse at the boundary, which is the intuitive one and what
      "validate at the boundary" in CLAUDE.md's security posture would suggest),
      and the cost (a reader may be shown a time that is not when the post was
      written, so the clamp must be reported).

- [ ] **`dev-writer`** — that **content-dedup ends** is not recorded in
      Decisions, and it is the change's one irreversible user-visible
      consequence. `design.md:214` mentions it only inside the composer bullet
      ("There is no delete in this system, so an accidental duplicate is
      permanent"), framed as motivation for a QML guard rather than as a decision
      taken about the core contract. It belongs in Decisions in its own right: an
      op id is a function of the preimage, the preimage gained two fields, so the
      dedup that `content-authoring` previously **guaranteed** is gone — and the
      alternative that was available and rejected (a nonce, which separates two
      identical posts and does nothing for ordering) is recorded only in
      `PLAN.md:4144` as struck-through history. The cost this forecloses is
      stated nowhere in `design.md`: with no delete and revision replacing
      content rather than withdrawing it, an accidental duplicate is permanent
      for every peer, and the author's only remedy is editing one into an
      apology. That sentence exists — in `DComposer.qml:200-203`, a QML file,
      which is not where a core-contract consequence is discoverable.

- [ ] **`dev-writer`** — `tasks.md:20` says `design.md` "carries the six
      decisions". It carries nine. Small, but it is a count in a document the
      repo's own rule says should not carry counts a command can answer, and a
      reader who stops at six stops before decisions 7-9 — which include the
      `score_epoch` reasoning and the `check_layout` near-miss, the two most
      consequential entries in the piece.

## Two things I checked that are NOT findings

Recorded so the next reviewer does not re-litigate them.

- **The counter-advance bound measured against held ops rather than the clock at
  arrival** *is* recorded, and well — `design.md:71-101`, decision 3, including
  the rejected alternative ("checking the bound on the receive path against the
  clock as it then stood") and the transitivity note. The brief flagged this as
  possibly-missing; it is not.
- **The composer guard being unreachable-by-construction today** *is* recorded,
  at `design.md:208-236`, and is the best-argued entry in the document: it states
  that the duplicate is already unreachable via synchronous `callModule` and
  single-threaded JS, that this is a reason to write the guard rather than skip
  it, and — unusually — that the obvious "tap twice, assert one publish" test
  would pass with the guard deleted, so it was deliberately not written. It names
  which three mutations were run instead. No finding.

## Provenance

The author reports finding no claim that traces to neither an owner decision nor
a spec. I tested that rather than accepting it, and **it holds** for the claims
this piece relies on. The owner ruling (Lamport authoritative, wall clock
display-only, "newest first" by Lamport) is visible in the spec deltas and in the
code's shape. The one thing that looked like an invented rule — `ADVANCE_BOUND`'s
value — is explicitly flagged in `design.md:104` as *not* fixed by the spec, with
the reasoning given as the `dev-writer`'s own choice rather than dressed as
inherited. That is the honest form.

`docs/UI-BRIEF.md` is modified by this change (182 lines). Per the review brief
it is being deleted by PR #83 under an owner ruling that it was output wrongly
read back as input, so I did not review those edits and no finding above depends
on them. Flagging only that this change's `proposal.md:206-210` lists the four
UI-BRIEF fixes as part of its impact — when #83 lands, that paragraph becomes a
claim about a file that does not exist, and whoever resolves the collision should
strike it rather than preserve it.
