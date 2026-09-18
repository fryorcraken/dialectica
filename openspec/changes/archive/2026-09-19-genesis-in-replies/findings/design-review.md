# Design review: genesis-in-replies

**No `design.md` exists for this change**, and the evidence below says it should
have. This review therefore checks the recorded decisions against `proposal.md`,
`tasks.md`, the spec delta, the code, and `docs/PLAN.md` (read from `origin/main`),
and reports the missing artefact as its own finding rather than silently
substituting `proposal.md` for it.

- [x] **`dev-writer`** — no `design.md` in `openspec/changes/genesis-in-replies/`
      This change met the trigger `dev-writer.md` sets for writing one. CLAUDE.md
      names the core module's wire API as "the part of this project to be most
      deliberate about... widening it is a decision to make on purpose rather than
      a side effect of needing one more field," and this change widens three reply
      shapes (`create_stoa`, `join_stoa` via `stoa_reply`, and every `list_stoas`
      item). It also carries at least three decisions with real alternatives and
      real costs — the `map`-to-loop rewrite so an encode failure returns the error
      shape (`wire.rs`, `membership_page_json`), the encode-failure-is-never-silent
      rule (`GENESIS`'s docstring), and the QML map reassignment forced by `var`
      not notifying on an in-place key write (`DStoaListScreen.qml`,
      `rememberGenesis`). A precedent exists for a change of this shape: the
      archived `2026-09-14-expose-name` (a comparably scoped "widen a wire call to
      expose an existing derivation") wrote a `design.md`. **Scenario:** a future
      reader wants to know why `membership_page_json` loops instead of maps, or why
      the QML map is reassigned rather than mutated, and has nowhere durable to
      look — the reasoning lives only in `wire.rs` doc comments, a QML comment, and
      the PR body (ephemeral, not part of the repo). All three decisions below are
      in fact well-recorded **in code comments**; the gap is that none of them is
      in the one place `design-reviewer` and future archive-readers are told to
      check, and `tasks.md` carries no design-writing task at all — the omission
      looks like it was never considered rather than decided against.

      **Fixed**: `openspec/changes/genesis-in-replies/design.md` now exists. I
      agree with the judgement, and the `2026-09-14-expose-name` precedent is
      the argument that settles it — same shape (widen a wire call to expose
      what core already held), same capability, and it wrote one. Combined with
      CLAUDE.md naming the core wire API as the thing to be most deliberate
      about, "widening three reply shapes" is not a change that should reach
      the archive with its reasoning only in code comments.

      It carries all three decisions you named, each with the alternative that
      was rejected and — the part the code comments could not carry — **what
      breaks without it**:

      - **`map` → loop.** Replacing the error arm with `unwrap_or_default()`
        turns the new
        `a_record_that_cannot_be_encoded_is_a_failure_rather_than_an_empty_field`
        red, emitting `{"…","genesis":"","…"}`. That is the mutation evidence
        you noted was measured nowhere; it is measured now, and the test that
        carries it was added on this pass in answer to the spec-test review.
      - **The QML map reassignment.** Recorded with your observation that no
        test pins the re-evaluation itself, stated as a **known accepted gap**
        rather than left implicit: the existing specs assert map contents and
        `canShare` in JS, so a "simplification" back to in-place mutation would
        likely stay green. Catching it needs a spec asserting the button's
        `visible` after a reload, which is the rendering layer no component
        test here reaches. Named so it is not mistaken for tested.
      - **The relation, not the shape** — kept, with your point that the spec
        text already does design.md's job here.

      Also recorded, though you filed them as clean: the reachability argument
      for the encode-failure arm (defensive, not live), so it is not deleted as
      dead code by someone who measures only reachability; and the Goals /
      Non-Goals boundaries, including that `policy` is deliberately not on list
      items.

      `tasks.md` now carries the design work as §6.4 rather than leaving it
      absent, and the stage block's `design + code` row notes that `design.md`
      came on the findings pass rather than the first one — ticking it without
      that note would misdate the artefact.

- [x] **`dev-writer`** — `proposal.md` "Why", and no `design.md`/PLAN.md entry
      recording the refused diagnosis
      The refusal that mattered most here — that a decode failure reading
      "genesis record ended mid-field" was *not* a codec bug, against a brief that
      apparently assumed it was — is recorded only in the **PR body** ("It was
      never a codec bug", with the byte-level evidence: 48 bytes, version `01`,
      etc.) and is *not* in `proposal.md`, which states the correct diagnosis
      flatly as established fact with no trace that a different hypothesis was
      chased first or why it was wrong. The PR body is not part of the repository
      history that survives past the PR; `openspec archive` moves `design.md` (not
      the PR description) into `changes/archive/`. **Scenario:** the next
      truncation error in this codebase — and `docs/PLAN.md` records this general
      shape of trap already, at length, for other subsystems (see "Silent failure
      is this codebase's house style") — sends someone to re-derive "check whether
      the input was ever supplied before assuming the codec is broken" from
      scratch, because the one artefact that would have told them (a `design.md`
      Decisions entry, or even a `## Decisions` note under a Context header) does
      not exist in the tree. This belongs with the missing-`design.md` finding
      above: had one existed, this is exactly the kind of dead-end investigation
      README.md's "write the dead end down" rule asks to be recorded beside the
      decision it rules out.

      **Fixed.** `design.md`'s Decisions section opens with "The reported
      diagnosis was refused, and that refusal is the most reusable thing here",
      and it is placed first deliberately — it is the entry with the longest
      reach, since the other three are about this change and this one is about
      how to read an error message.

      It carries the byte-level evidence out of the PR body so it survives the
      archive: 48 bytes, version `01`, 32-byte creator, policy `00`, title
      length `0000000A`, title `test0.0.01`, plus
      `stoa.rs::decode_of_encode_is_the_identity` already holding in both
      directions. Then the actual cause — `wire.rs` decodes a `genesis` hex
      string supplied by the *caller*, `list_stoas` never returned one, the
      view sent `""`, and zero bytes fail the version-byte take. **Core was
      correctly refusing an empty record the view sent.**

      The generalisation is stated as the reusable part, since that is what a
      future reader needs and not the particular record: *a decoder reporting
      truncation is a claim about the bytes it was given, not about where they
      were stored* — so the first question is whether the input was ever
      supplied, not whether the codec is broken. Reading it as a codec or
      storage defect sends the investigation to the furthest possible point
      from the cause, which was a wire shape omitting a field.

      Your reasoning for *why* it had to move is recorded with it: `openspec
      archive` moves `design.md` into `changes/archive/`, and a PR description
      is not part of repository history. I have noted there that the dead end
      was the **brief's own premise** rather than a hypothesis the author chose
      to chase, which if anything raises its value — an agent was dispatched to
      hunt a codec bug and correctly refused the premise instead of finding
      something to blame.

      `proposal.md` is deliberately left as it stands, stating the correct
      diagnosis flatly. A proposal's job is why the change is needed; the
      history of what was disproved on the way belongs in Decisions, which is
      where it now is. Flagging that as a choice rather than an oversight.

## What is in good shape

The three implementation decisions the brief asked about **are** recorded, just
in the wrong document:

- **`membership_page_json`: `map` closure → loop.** `wire.rs`'s comment directly
  above the function states the reason precisely: "Reported as a failure rather
  than as an item missing its record... a closure could only have swallowed it or
  panicked." The `GENESIS` constant's docstring above it states the general
  principle ("Encoding cannot fail here, and is not silently defaulted") with the
  causal link back to the owner's bug. This is a complete decision — what was
  chosen, the constraint, the cost of the alternative (silent empty field
  reintroducing the defect) — missing only the **mutation evidence** a `design.md`
  entry would carry (e.g., "reverting to `.map()` and swallowing the error with
  `.unwrap_or_default()` turns `a_listed_stoa_carries_the_record...` red"), which
  was not measured anywhere in `tasks.md` or the PR body.
- **QML map reassignment.** `DStoaListScreen.qml`'s comment on `rememberGenesis`
  states the platform fact plainly: "a QML `var` property does not notify on an
  in-place key write, so bindings on `canShare` would not re-evaluate." This is
  exactly the kind of non-obvious fact CLAUDE.md's design.md convention exists to
  pin down before a later editor "simplifies" it back into `next[stoa] = genesis;
  screen.genesisByStoaChanged()` without the reassignment — which would silently
  stop working. No test appears to pin the *re-evaluation* itself (the QML tests
  check the final map contents and `canShare`, not that mutating in place fails to
  update bindings), so if this is ever "simplified," the regression may not be
  caught by re-running the existing suite. Worth a design.md mutation note even
  though the fact itself is correctly stated in code.
- **The relation, not the shape.** Both the spec delta ("The record MUST be the
  one the accompanying address is the hash of... A reply carrying a well-formed
  record of some other Stoa satisfies 'the field decodes' and is useless") and the
  test helper `genesis_of` (its own doc comment: "The relation rather than a
  pinned literal") state the reasoning, not just the requirement. This is properly
  recorded — the spec text itself carries the "why," which is unusual and good:
  most specs would state only the MUST and leave the reasoning to `design.md`, but
  here it does the job design.md would have. No finding.
- **The capability placement** (`stoa-membership` rather than a new capability) is
  a reasonable call and needs no separate justification: every requirement this
  change touches (`create_stoa`'s reply, `join_stoa`'s reply, `list_stoas`' item
  shape) is a requirement `stoa-membership` already owns, and the new requirement
  is additive to replies that capability already defines. No finding.
- **The unverified gap (task 5.4)** is recorded honestly, not implied to be
  verified. `tasks.md` leaves the row explicitly unticked with a paragraph
  distinguishing exactly what was established (1052 Rust tests including a real
  reopened-store test, 411 QML tests, three of which were proved to fail first,
  and a read confirmation that neither forwarding layer narrows the shape) from
  what was not (the actual click-through, because loading the plugin needs a human
  click basecamp cannot receive from an agent). This is the shape CLAUDE.md's
  "never report a result you did not obtain" and the README's "a wrong-tree
  success is indistinguishable from a right-tree one" both ask for. No finding.
- **The share affordance un-hiding** is recorded as a deliberate, named
  consequence, not a side effect noticed in passing: `proposal.md`'s "What
  Changes" states "The view fills its record map from both replies, which
  restores **Open** and un-hides the **share** affordance — both were gated on the
  same lookup." No finding.
- **The stale QML comment** predicting this exact fix ("Closing this properly is a
  CORE change — widen the listing item to carry the retained record") is replaced
  with an accurate comment describing what the code now does, and task 3.5 names
  this explicitly as done. No finding.
- **PLAN.md** needed no edit from this change: `create_stoa`/`join_stoa`/
  `list_stoas` were already marked "Built" before this change: `genesis-in-replies`
  is a bugfix/widening of already-built behaviour, not new behaviour PLAN.md was
  tracking as unbuilt, so the "PLAN.md sheds in two directions" rule does not
  apply here. No finding.
