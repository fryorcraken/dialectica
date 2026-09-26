# Design review — position-and-index (#166)

Read the owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024,
2026-09-25) before this review. It settles item 2: a malformed `index` is
refused with an error naming `index`; leaving the message unspecified is
ruled out.

Checked:

- `design.md` D1–D5, Goals/Non-Goals, Risks against the production code
  (`parse_index`, `keep_identity`, `thread_page_json`, `read_thread` in
  `thread.rs`/`wire.rs`) and against the tests added in
  `git diff origin/main...HEAD`.
- `design.md`, `proposal.md` and both spec deltas (`identity-onboarding`,
  `thread-read`) against issue #166's body and the owner's decision comment.
- The reconciliation commit (`fe9ea3d`, "Bring position-and-index's design in
  line with the place rule") against the current `thread-read` delta, for
  anything still describing the pre-reconciliation state.

## Result: no blocking findings

Every Decision holds up against the code:

- **D1** (`index` keeps sharing `parse_index`): confirmed — `keep_identity`
  calls `parse_index(&parsed, "index")` and every arm formats `{field}` into
  its message, so the shared guard is real, not asserted.
- **D2/D3** (wire-level property tests, shared-author fixture): confirmed —
  `a_thread_log_with_shared_authors` has two author-sharing pairs and asserts
  `authors.len() < len`; `no_two_items_of_a_thread_share_a_position_…` and
  `a_position_is_the_same_whatever_page_size_…` exist as described, with the
  "what these two cannot see" gap (an op id) correctly called out and pointed
  at D5.
- **D4** (raw-JSON-text keep helper): confirmed — `keep_with_raw_index`
  exists, `MALFORMED_INDEXES` is the shared table both by-name and
  stores-nothing tests iterate, matching the spec's 8-entry scenario list
  exactly (`-1`, `1.5`, `0.0`, `1e2`, `"two"`, `[]`, `true`,
  `18446744073709551616`).
- **D5** (place rule, two-reads test): confirmed against both the code and
  the spec text. `thread.rs`'s `resolve_item`/`read_thread` genuinely drop a
  hidden reply before enumerating (`placed.at(index)`), so "position is
  determined by place alone" is what the code does, not just what the test
  asserts. `the_item_at_a_place_carries_that_places_position_in_every_read`
  matches the scenario's two clauses precisely.

The spec moved once mid-piece (`2ac71ef` adding the `thread-read` place-rule
scenario, `fe9ea3d` reconciling `design.md`). Nothing in the current
`design.md` still describes the pre-reconciliation state: Context, Goals,
D2, and Risks were all rewritten in `fe9ea3d`, and the current text
(uniqueness narrowed to "one read", the place-rule paragraph, D5) matches
`specs/thread-read/spec.md` as it now stands line for line.

`identity-onboarding`'s new requirement matches `feed-read`'s existing list
of malformed kinds for `page`/`perPage` (`spec.md:555`) exactly, which is the
right basis for "no spec requires it yet" → "add the requirement to the
capability that owns `keep_identity`" that issue #166 and the owner's
decision comment call for. The owner's comment names three example kinds
(negative, fractional, not a number); the requirement's fuller list (also
covering an integer written with a decimal point/exponent, and one too
large to represent) is justified in `proposal.md` as following `feed-read`'s
existing list rather than narrowing or contradicting the owner's decision.

`proposal.md`'s "Deliberately left unspecified" section and `design.md`'s
Risks entry agree on the `u64::MAX + 1` wrong-reason issue, and both record
that a fix belongs in a separate issue against the parser — this is the
"reasoning migrates" case done correctly: the issue's mid-piece discovery
(the op-id gap in the position requirement) was moved into the spec itself
(`thread-read`'s new place-rule paragraph and scenario), not left sitting
only in `design.md` or the issue thread.

## One non-blocking suggestion

- [x] **`dev-writer`** — `design.md` D5's "What breaks without it" still
      reads in the future tense: *"is for the `tester` to measure, under
      task 3.4"*. Task 3.4 is now done (`tasks.md` ticked, commit `1889eaf`),
      and its commit message already carries the actual mutation evidence
      ("Proven red with the position set to the item's op id … Proven green
      against the untouched implementation"). Not a defect — CLAUDE.md
      accepts a commit message as the mutation evidence's home — but a
      one-line update to D5 pointing at `1889eaf` instead of describing the
      proof as pending would stop a future reader from wondering whether the
      guard was ever actually measured. Purely cosmetic; does not block
      merge.

      **Outcome (`dev-writer`): fixed** in the commit that ticks this box. D5's
      "What breaks without it" now records the measured result and points at
      `1889eaf`. The op-id mutation turns only the place test red, and D2's two
      tests stay green. The place test also goes red under a constant and under
      `item.author`, and stays green under a per-page index. The proof was re-run
      in `70bef05`, after the fixture refactor. The same pending-proof tense was
      in two more places, so those were fixed in the same commit. D2's "What these
      two cannot see" still said "while that box is unticked". D1 still said "the
      `tester` owns the full proofs", and it now names both reachable
      `parse_index` arms and the tests each one turns red. D5 also gains a
      paragraph recording the fixture's explicit ascending clocks as a guard. The
      reasoning was only in `tasks.md` 3.4 and a test doc comment, and without
      the clocks the test's adjacency depends on hash order. No test covers this
      outcome, because it changes prose only.

## Re-review of the findings-round commits

Read the owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024,
2026-09-25) again before this round. It settles item 2 and is unchanged by
anything reviewed here.

Scope: `git diff 19667a04...HEAD` — `b55d164`, `70bef05`, `9e550eb2`,
`4e2b8c0f`, per `tasks.md`'s "Re-review of the findings-round commits" list.

Checked and confirmed against the code and tests as they now stand:

- **D1** (`9e550eb2`): both `parse_index` message arms still format `{field}`
  (`wire.rs:1780-1831`). Dropping it from the wrong-type arm turns
  `each_malformed_kind_of_index_is_refused_by_name` red on `"two"`/`[]`/`true`
  and `malformed_pagination_fields_are_refused_by_name` red, matching the
  text. `MALFORMED_INDEXES`'s first entry is `("negative", "-1")`, matching
  the claim that dropping `{field}` from the non-negative-integer arm turns
  the by-name test red "first on `-1`". `1889eaf` is a real commit and its
  message matches D5's citation of it.
- **D2/D5**: `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`,
  `a_position_is_the_same_whatever_page_size_the_read_used` and
  `the_item_at_a_place_carries_that_places_position_in_every_read` all exist
  with the behaviour D2/D5 describe. D5's new paragraph on explicit ascending
  clocks matches `arrival::cmp_ops` exactly: `(None, None) => a.id.cmp(b.id)`
  ties two clockless ops on op id, an unpredictable hash — so the claim that a
  clockless version of the fixture "failed against the correct implementation"
  is architecturally sound, not asserted.
- **`70bef05`'s fixture refactor**: `a_thread_post_with_clock` is the sole
  place a fixture post's `Op` literal is built; `a_thread_post` delegates to
  it with `clock: None`; the hidden-reply fixture's `under_root_at` calls it
  with an explicit `OpClock`. Test-only, as claimed — no production file
  changed in this commit.
- **`proposal.md`'s new note and the live `identity-onboarding` Purpose
  edit** (`b55d164`): agree with each other and with `design.md`. The Purpose
  paragraph names `feed-read`, states the parallel (same kinds of malformed
  value, neither restates the other), and doesn't touch `feed-read`'s own
  Purpose — confirmed unchanged by `git diff` against
  `openspec/specs/feed-read/spec.md`. The precedent cited (`d8a56272`, #160,
  "align two Purposes with 0.0.1") is a real commit doing the same kind of
  direct Purpose edit. The `op-ordering`/`op-format` boundary-note pattern
  cited as the shape for a one-directional pointer is real
  (`openspec/specs/op-ordering/spec.md:7,99`).
- `thread-read` delta's MUST→SHALL: both instances changed, matches
  `readability.md`'s outcome, and `git grep -F` finds no other place quoting
  the old wording.

## One gap: the asymmetric-Purpose-edit reasoning is recorded where it will be deleted

- [x] **`dev-writer`** — `design.md` has no Decisions entry for "edit
      `identity-onboarding`'s Purpose directly rather than through a delta,
      and only in one direction." This is exactly the shape of the other five
      entries — a real alternative existed (a fourth boundary name in the
      existing list; a reverse pointer added to `feed-read`'s own Purpose),
      each was ruled out with a reason, and it costs something (the boundary
      is now asymmetric: `index`'s list of kinds points at `feed-read`, and
      `feed-read` gets no pointer back). The reasoning for all of this exists,
      but only in `findings/architecture.md`'s "Outcome (`spec-writer`):
      fixed" text — and `.claude/agents/README.md` ("Two files carry the
      state of a change") says plainly: **"The `closer` deletes the directory
      before merge, once no box is empty."** `findings/` does not survive to
      `openspec/changes/archive/`; `proposal.md`, `design.md` and `tasks.md`
      do (`docs/OPENSPEC-ARCHIVE.md`). `proposal.md`'s note captures what was
      chosen and the constraint (a delta can't touch a Purpose) but not why
      the fix is one-directional — that reasoning ("Listing every other
      capability that refuses a field by name would be an open-ended list
      that goes stale when a sixth one appears") lives only in the file that
      is about to be deleted. `b55d164`'s own commit message says as much:
      "feed-read's Purpose gets no reverse pointer, and the finding's outcome
      gives the reason" — naming the findings file as the reason's only home.
      This is the "reasoning migrates" trap from the opposite direction: not
      an issue closed without writing up its reasoning, but a review finding
      answered with real reasoning that is scoped to die with the findings
      file. Move the "why one-directional" sentence into `design.md` (a new
      Decision, or a line added to D1 given both concern `parse_index`'s
      shared-refusal boundary) or into `proposal.md`'s existing note, before
      the `closer` deletes `findings/`.

      **Outcome (`dev-writer`): fixed** in the commit that ticks this box.
      `design.md` gains D6, a Decisions entry of its own rather than a line in
      D1: D1 is about the parser, and this is about where the spec records a
      boundary. D6 carries everything `findings/architecture.md` held and
      `proposal.md` did not. It says why the paragraph is separate from the
      boundary list, and why the Purpose leaves `parse_index` unnamed. It says
      why `feed-read` gets no reverse pointer: its boundaries name capabilities
      whose rules it uses, a list of every by-name refuser would be open-ended
      and go stale, and the note belongs on the side whose list of kinds
      followed the other's, as `op-ordering`'s does for `op-format`. It also
      keeps the architecture re-review's measured evidence that the direct
      Purpose edit survives a real `openspec archive`. D6 states the cost as
      well. The boundary is asymmetric, so a reader who starts in `feed-read`
      gets no pointer. The shared parser narrows that gap, and D6 says which
      change the by-name test catches and which it cannot see. No test covers
      this outcome, because it changes prose only.

No other findings from this round. The rest of the re-reviewed material —
D1/D2/D5's rewritten prose, the fixture refactor, the spec MUST→SHALL edit,
and `proposal.md`'s note — matches the code, the tests, and each other.

## Re-review of the D6 commit

Read the owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024,
2026-09-25) again before this round. It settles item 2 and is unchanged by
anything in `536a03e`.

Scope: `git show 536a03e`, which adds `design.md` D6 in answer to the "One
gap" finding above and ticks that finding's box.

- **Does D6 answer the finding?** Yes. The finding asked for the
  "why one-directional" reasoning to move out of `findings/architecture.md`
  (deleted before merge) into `design.md` or `proposal.md`, in the shape of
  the other five Decisions entries: what was chosen, the constraint, the
  alternatives and what ruled each out, and what it costs. D6 has all four:
  chosen (the paragraph, direct edit, one direction), constraint (a delta
  cannot touch a Purpose), three alternatives each ruled out with a distinct
  reason, and a "What this costs" paragraph naming the asymmetry and stating
  plainly that the choice is untested because a Purpose states no behaviour
  (matching CLAUDE.md's guidance to say so rather than pad with a box).

- **Agreement with `proposal.md`, the live `identity-onboarding` Purpose,
  `feed-read`'s Purpose, and D1:** confirmed. The live
  `openspec/specs/identity-onboarding/spec.md` Purpose (lines 25-31) contains
  exactly the paragraph D6 describes — names `feed-read`, states the parallel,
  says neither restates the other, and notes a `feed-read` change must be
  checked against `index`'s requirement. `openspec/specs/feed-read/spec.md`'s
  Purpose is unedited and has no mention of `identity-onboarding` or `index`,
  matching D6's "not edited" claim. `proposal.md` (lines 15-96) states the
  same constraint, the same direct-edit choice and the same one-directional
  reasoning D6 elaborates; nothing contradicts. D1 (lines 59-84) is about the
  parser-sharing decision and the `{field}`-formatting guard; D6 correctly
  treats it as a separate concern ("D1 is about the parser, and this is about
  where the spec records a boundary") rather than duplicating it, and D6's
  "What this costs" paragraph correctly restates D1's mutation evidence
  (dropping the field name from either `parse_index` arm turns
  `each_malformed_kind_of_index_is_refused_by_name` red) rather than
  re-deriving it.

- **Citations, checked against source:**
  - `docs/OPENSPEC-ARCHIVE.md` ("Two capabilities asserting one rule"),
    lines 136-145: real, and it does name the `op-ordering`/`op-format`
    precedent and say `spec-backfill`'s silence produced a duplicate — matches
    D6's characterisation.
  - Commit `d8a56272`: real (`#160`, "Make the identity spec's no-rotation
    scenario testable, and align two Purposes with 0.0.1"). Its commit
    message states "The identity and identity-onboarding Purpose paragraphs
    are edited directly, since a delta cannot change a Purpose" — the exact
    precedent D6 cites it for.
  - `op-ordering` and `op-format`'s Purposes: `op-ordering/spec.md:7` names
    `op-format`'s requirement and declines to restate it ("That requirement is
    not restated here… Two specs asserting one rule is how two copies drift");
    `op-format/spec.md`'s Purpose (lines 3-4) carries no reference back to
    `op-ordering`. Matches D6's claim precisely, including the one-directional
    shape it draws the parallel from.
  - Test name `each_malformed_kind_of_index_is_refused_by_name`
    (`dialectica/rust-lib/dialectica-core/src/wire.rs:5090`): real.

- **Is the by-name test's catch/miss claim true?** Yes, verified by reading
  the test and its fixture table (`wire.rs:5075-5121`).
  `MALFORMED_INDEXES` is a fixed 8-entry table of today's known malformed
  kinds; the test loops over it and asserts `message.contains("index")` for
  each. D6's claim that "a code change that stops refusing one of
  `MALFORMED_INDEXES`' kinds, or drops the name, reaches `index` too, and
  `each_malformed_kind_of_index_is_refused_by_name` goes red" holds — this is
  the same mechanism D1 already describes and pins with mutation evidence.
  D6's claim that "a change that refuses a new kind also reaches `index`, but
  no test of `index` sees it, because the table lists only today's kinds"
  also holds: the table is a hand-maintained literal, not derived from
  `parse_index`'s own arms, so a new malformed shape `parse_index` starts
  refusing (or a shape it currently accepts and starts rejecting) has no
  corresponding table entry and the loop never exercises it.

No findings. D6 answers the finding it was written for, agrees with every
document and spec text checked against it, and every citation checked
(`docs/OPENSPEC-ARCHIVE.md`, `d8a56272`, `op-ordering`/`op-format`, the test
name) is real and accurately characterised.
