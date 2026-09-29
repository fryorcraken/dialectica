## Context

See `proposal.md` for what was tidied and why. The spec work is five archives,
each preceded where needed by a correction to its delta, then this change's own
deltas and three in-place Purpose edits. Outside the specs, four comments
change because this piece's own moves made them stale, and the `tester` stage
adds tests for requirements the archives promoted (Non-Goals lists both). No
production code, existing test assertion or wire shape moves, so the design
questions are about **how a live contract is changed safely**, not about how
anything is built.

Two facts about the tooling shape every decision below. Both are recorded in
`docs/OPENSPEC-ARCHIVE.md`:

- **A `MODIFIED` requirement replaces the whole live block.** A delta written
  against an older live spec silently deletes whatever landed since. Neither
  `validate --strict` nor `archive` reports it.
- **`validate --strict` cannot see a contradiction**, either inside one spec or
  between two.

## Goals / Non-Goals

**Goals:**

- The live contract in `openspec/specs/` describes what main ships. That covers
  the five shipped-but-unarchived changes, and no citation in it points at
  something that no longer exists.
- How the thread screen and the moderation screen are entered and left is
  specified once, in `view-navigation`, and not also in the screen's own
  capability.

**Non-Goals:**

- Any behaviour change. Every requirement this change's own deltas touch keeps
  its obligations. The two behaviour-level additions in the piece belong to
  `moderation-screen`'s corrected delta, and the code already met both
  (`proposal.md`, last paragraph of What Changes).
- The "this change" wording sweep across the live specs is tracked in #186.
  It is out of scope so that this piece's review stays small.
- `relevance-votes`.
- Any production-code change. Outside the specs, the piece edits comments and
  adds tests, and nothing else:
  - **Four comments** that this piece's own moves made stale. See Risks.
  - **Tests the `tester` stage added**, because archiving promotes requirements
    into the live contract, and a promoted requirement no test pins is an
    obligation CI does not check. Every added line is test code:
    - `sqlite.rs`: two `#[test]` functions and a `stored_author` helper, all
      inside `mod tests`. One pins op-log's stored author column (`a98c434`),
      the other a storage failure at read time (`5f95e7e`).
    - `tst_thread_reply.qml`: one test that a successful publish inserts no
      item (`5f95e7e`).
    - `tst_moderation_screen.qml` and `tst_stoa_screens.qml`: two `NO SPEC:`
      comments reworded to cite the requirements that now cover them
      (`a98c434`). Comment-only; no existing assertion changes.
  - **Tests still pending** from the five open `tester` findings in
    `findings/readability.md` and `findings/spec-test.md`. Until those boxes
    are closed, this inventory is incomplete; each box's outcome names what it
    added.

## Decisions

### 1. A change counts as shipped when its merged PR, not its `tasks.md`, says so

Each of the five archived changes was checked against its merged PR and against
the code and tests on main. Its `tasks.md` was not taken as the evidence,
because a stage row can be ticked or left unticked independently of what
merged:

| Change | Shipped in | Delta corrected | Archived |
|---|---|---|---|
| `sqlite-projection` | #20 | `1f5df8f` | `0519416` |
| `first-run-identity` | #128 | `b1d8966` | `b545d3b` |
| `ui-thread-view` | #122 | `584359b` | `d70f662` |
| `ui-remaining-screens` | #124 | not needed | `2fa2b11` |
| `moderation-screen` | #127 | `8e508d8` | `eb8be6b` |

`gh pr view <n>` re-checks each PR's merge state. `proposal.md`'s "Unarchived
changes" lists what each correction changed.

**Each correction is its own commit, before its archive commit, and the
correction goes into the delta and not into the live spec afterwards.** The
alternatives were:

- **Archive as-is and fix the live spec afterwards.** Rejected because of the
  `MODIFIED` fact above: `sqlite-projection`'s op-log delta, promoted as
  written, would have deleted the blank-title encoding refusal. It would also
  have promoted a decay point taken from transport metadata, which `op-clock`
  had already stopped using. A later fix has to notice the deletion first, and
  nothing reports one.
- **Correct and archive in one commit.** Rejected because the archive diff is
  meant to be mechanical and checkable against the delta. Mixing a judgement
  into it hides the judgement inside the mechanical part. Kept apart, the
  correction commit is where a reviewer reads the reasoning, and the archive
  commit is where they confirm the promotion matched the delta.

The archives were applied in merge order, oldest first, as
`docs/OPENSPEC-ARCHIVE.md` requires, so each later `MODIFIED` applies to the text
the earlier ones produced.

### 2. `relevance-votes` stays unarchived

The owner ruled it out of scope for this piece ("(2) ignore"). Its own
`tasks.md` also leaves open the implementation section ("§4 Implementation,
when this is accepted") and the `openspec validate` step before archiving.
Neither is this piece's to decide. Archiving it would promote a
`relevance-ordering` contract that no shipped code implements, which is the
opposite of what this tidy is for.

### 3. Live-spec fixes are carried as deltas, not as `skip_specs` with direct edits

Every requirement-level correction to a live spec is a `MODIFIED`, `REMOVED` or
`ADDED` block under this change's `specs/`, promoted by the closer's `archive`.
None is a direct edit to `openspec/specs/`.

- **Rejected: `skip_specs: true` plus direct edits.** The owner confirmed that
  `skip_specs` is for test-only pieces, which have no requirement to promote.
  This piece changes requirements, so the marker would misdescribe it. Direct
  edits are also the shape `docs/OPENSPEC-ARCHIVE.md` warns about under
  `stoa-genesis`: a live spec that differs from every delta, with no diff
  anywhere showing why.
- **What the delta route buys.** `openspec validate spec-tidy --strict` runs on
  the change, each `MODIFIED` heading is checked against a live heading, the
  review reads each correction as a before/after block, and the archived folder
  records what changed and why.

**The one exception is a Purpose, which a delta cannot carry.** The three
Purpose edits are direct and each is its own commit: `view-navigation`'s in
`06bfd48`, and `thread-view`'s and `moderation-view`'s in `ff55cf7`. See Risks
for the window this opens.

### 4. `view-navigation` is the single home for how a screen is entered and left

This is the owner's decision ("3. ok yes sound sgood").

**Why `view-navigation`, and not each screen's own capability.**

- Its Purpose already claimed the ground: it "owns transitions and nothing a
  screen renders".
- The defect it exists for is a cross-screen property. A component test cannot
  tell a working screen from an unreachable one, so the check has to read the
  registrations and the view's sources together.
- Its general rule, "Every state a user can enter has a specified way out", is
  what every per-screen return otherwise had to cite from somewhere else.
  `thread-view`'s route requirement did exactly that, and it also duplicated
  `view-navigation`'s "A thread is opened from a feed row and can be left".
  That put two live copies of one transition in two capabilities. That is the
  drift `docs/OPENSPEC-ARCHIVE.md`'s "Two capabilities asserting one rule"
  describes.

**Rejected alternatives:**

- **Push routes out to the screens, and shrink `view-navigation` to the
  reachability gate.** This would scatter the general way-out rule across every
  screen capability, and each copy would have to be kept in step.
- **Leave the duplication.** This keeps the drift and fixes nothing.

**The two moves take different shapes, on purpose.**
`docs/OPENSPEC-ARCHIVE.md`'s pattern for moving requirements is `REMOVED` from
the old capability and `ADDED` verbatim to the new.

- **`moderation-view` follows that pattern.** `view-navigation` had no
  requirement covering that screen, so "The screen is reachable, and leaving it
  returns where the user was" is `ADDED` there nearly verbatim. The only
  changes are the renamings `proposal.md` lists, plus one sentence leaving what
  the screen renders to `moderation-view`.
- **`thread-view` does not, and cannot.** Adding its requirement verbatim would
  recreate the duplicate this decision removes. So its obligations are folded
  into the existing `MODIFIED` "A thread is opened from a feed row and can be
  left", and every scenario keeps its substance:
  - Two scenarios move across unchanged.
  - One scenario moves across with its title changed from "The screen carries…"
    to "The thread screen carries…".
  - One scenario becomes an extra `AND` on the existing "Acting on a feed row
    opens its thread".
  - "The feed is reachable again from the thread" is dropped. The live "The
    feed is reached again from the thread" already requires the same outcome.


  The `REMOVED` block's Migration names where each part went, so the fold can
  be checked.

`moderation-screen` had already been archived when the decision came. That is
why the moderation move is a `spec-tidy` delta and not a correction to
`moderation-screen`'s delta.

### 5. The "this change" sweep is tracked separately

About twenty passages across twelve live specs say "this change", which means
nothing once a change is archived. Rewording them fits a tidy, but they are
spread across capabilities this piece otherwise does not touch. Folding them in
would put a second, unrelated review on top of the archives and the navigation
move. The owner had it tracked separately, as #186, which gives the grep that
finds them and asks for a re-count rather than trusting the survey's.

This change's own deltas remove the phrase wherever they already touch a
requirement. Examples are `generated-names`' issue-#80 paragraph and
`feed-view`'s "not widened by this change". The deltas do not go looking for
more.

## Risks / Trade-offs

- **[Purpose and requirements disagree until spec-tidy is archived]**
  - On this branch, `thread-view`'s and `moderation-view`'s Purposes already
    say `view-navigation` owns their entry and exit.
  - Each spec still holds its own route requirement until the `REMOVED` blocks
    are promoted.
  - `view-navigation` does not yet hold the moderation route.
  - **Mitigation:** the closer archives before the merge
    (`docs/OPENSPEC-ARCHIVE.md`), so main never sees the window. The stage row
    "`openspec validate --strict`, then `archive`" is that step. A merge that
    skipped it would leave main's Purposes contradicting their requirements.

- **[Code comments cite requirements this piece moved]** The archives and the
  navigation move made four comments stale. They are fixed in this piece, as
  comment-only edits that change no behaviour:
  - `Main.qml`'s comment on `openThread` cited `thread-view` for "No thread is
    rendered before one has been chosen". It now cites the `view-navigation`
    requirement that holds that scenario.
  - `FeedScreen.qml`'s comment on the moderation route credited
    `moderation-view` with reachability. It now names `view-navigation`'s
    moderation route requirement.
  - The headers of `dialectica-ui/tests/ui/thread.yaml` and `moderation.yaml`
    said their capability "lives only in the unarchived" change folder. They
    now name the live requirements. `moderation.yaml` splits its route claims
    (`view-navigation`'s) from its inertness claim (`moderation-view`'s).

  **The residual risk is that nothing enforces these citations.** A later move
  of a requirement leaves its comments stale again with every gate green.
  `git grep -n -E "thread-view|moderation-view|view-navigation" -- dialectica-ui`
  is the sweep that found these.

- **[Short SHAs do not survive the squash]** The commits cited above and in
  `proposal.md` exist on `piece/spec-tidy` and in the PR's commit list, but
  not on main after the squash-merge.
  - Every commit subject names its change, so each is findable in the PR.
  - The archive folders under `openspec/changes/archive/` are the durable
    record of what each promotion did.
