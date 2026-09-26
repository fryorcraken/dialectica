# Spec-test findings — `e2e-created-stoa-flow`

Reviewed the spec and the tests only: `proposal.md`, the spec deltas
(`feed-view`, `stoa-navigation-view`), the live `view-navigation`/`feed-view`/
`stoa-navigation-view` specs, the unarchived `thread-view` and `moderation-view`
contracts, `tasks.md`, and `git diff fe093be...HEAD -- dialectica-ui/tests`. No
file under `dialectica-ui/src/`, `Main.qml`'s workflow logic, or `design.md` was
opened, per the runner's brief.

## 1. Scenario coverage

Walked all four sitometres specs (`create.yaml`, `feed.yaml`, `thread.yaml`,
`moderation.yaml`) against proposal.md's four numbered observations and the
requirements it names as already live. Each maps cleanly:

- `create.yaml` → `stoa-navigation-view`'s no-key/key-held states, the created
  address, the re-read listing, and the new "shareable from the creation
  reply" scenario.
- `feed.yaml` → `view-navigation`'s feed-from-row transition, the new
  `feed-view` empty-vs-failed requirement (the central "the feed was read, and
  holds nothing" step), `composer-view`'s posting gate and stored-not-delivered
  copy, and the feed-to-list return.
- `thread.yaml` → `view-navigation`'s thread-from-row transition and
  `thread-view`'s "root with no replies" and reply-composer requirements (read
  from the unarchived `openspec/changes/ui-thread-view/` folder, as
  proposal.md directs).
- `moderation.yaml` → `moderation-view`'s inertness-is-rendered and
  reachable-with-a-way-out requirements (from the unarchived
  `openspec/changes/moderation-screen/` folder).

No scenario in the four contracts I was pointed at is asserted at a layer that
cannot observe it: these are full sitometres runs against a real Basecamp, so a
cross-process call, a real click on a `MouseArea`, and actual rendered text are
all within what the harness can see. I found no untestable scenario and no
scenario left without a test among what this piece adds.

**The `feed-view` ADDED requirement's five scenarios are all covered, but not
where `tasks.md` says.** `tasks.md`'s tests row states they are "pinned in
`tst_feed_states.qml`." Four are (`test_an_empty_store_is_the_ok_state_with_no_rows`,
`test_a_store_failure_is_the_failed_state_and_never_an_empty_feed`,
`test_the_two_states_are_distinguishable`,
`test_a_success_without_an_items_array_becomes_a_named_failure`), but the fifth
— "A failed read does not leave an earlier read's rows on screen" — has no
sequential-reload test anywhere in `tst_feed_states.qml`; every test there
builds a fresh screen from one fixed reply, so none exercises a screen that
succeeded with rows and was then reloaded into failure. That scenario is
instead pinned in `tst_e2e_handles.qml`'s `test_the_feed_handles_follow_the_feed_screen`,
which does drive exactly that sequence (2 rows → 0 → 2 → failed) and asserts
`main.feedRowCount === 0` while noting as a precondition that the screen's own
`rows` model still holds the stale page. Coverage exists; the citation is
just in the wrong file. Not a checkbox — nothing needs to change in the tests
— but worth fixing in `tasks.md` so a future reader looking for that scenario's
test doesn't come up empty in the file named.

## 2. Can each test fail?

Read every test added or changed in `tst_composer.qml`, `tst_e2e_handles.qml`,
`tst_navigation.qml` and `tst_stoa_screens.qml` (full diff against
`fe093be...HEAD`) against the "asks the implementation what it wrote and
agrees" shape. None of the four fit it:

- `tst_composer.qml`'s `test_the_field_and_the_submit_are_named_by_kind` finds
  children by a name derived from `kind` and would fail on a `findChild`
  returning `null`, on the draft not reflecting the named field, or on the
  other kind's names being present.
- `tst_e2e_handles.qml`'s three new tests are built as deliberately
  discriminating fixtures rather than passive readbacks — e.g.
  `test_the_creation_handles_follow_the_creation_reply` seeds the listing with
  `stoaA` *before* creating `stoaB`, specifically so a `createdStoa` wired to
  "the first listed row" instead of the creation reply reads `stoaA` and fails.
  `test_the_feed_handles_follow_the_feed_screen` and
  `test_the_thread_handles_follow_the_thread_screen` drive a success-then-
  failure sequence and assert the count drops to 0 on failure while the raw
  model doesn't.
- `tst_navigation.qml`'s two new tests use a bridge that refuses any read not
  carrying the chosen Stoa's exact record, checking *every* call rather than
  the last — which is what makes them able to catch a first-read-wrong,
  later-reads-correct race (the `tasks.md` 3.2 write-up records exactly that
  shape of red run).
- `tst_stoa_screens.qml`'s added assertion pair checks both that two rows are
  still rendered and that no failure string is present, closing the half of
  the scenario the pre-existing test left unchecked.

**Mutation sampling was not performed against source.** The brief for this
dispatch instructs not to open `Main.qml`, `FeedScreen.qml`,
`DThreadScreen.qml`, `DStoaListScreen.qml`, or any other file under
`dialectica-ui/src/` — which is where every canonical mutation for these tests
(flip the guard, remove the binding-order fix, etc.) would have to be made. I
did not work around that with an edit elsewhere. In its place I spot-checked
two of `tasks.md`'s cited CI runs directly with `gh run view`:

- Run `36213442819` (task 3.1's predicted-red run): matches exactly —
  `sitometres feed spec` red, the other four green, failure on "the feed was
  read, and holds nothing."
- Run `36214314703` (task 3.4's predicted-green run, the D8 fix): matches
  exactly — all five `ui-tests.yml` jobs green.

Both cited runs are real and match their claims. That corroborates the CI-based
mutation evidence `tasks.md` already records for the properties I could not
mutate myself (design.md's D3/D8, and tasks.md sections 3–7's red/green pairs),
but it is not a substitute for an independent mutation, and I record the gap
rather than claim I closed it. **Given the choice of one thing to flag if this
review is re-run without that constraint:** `test_the_feed_handles_follow_the_feed_screen`'s
assertion `main.feedRowCount === 0` after the failed reload (the "does not
leave rows on screen" scenario, §1 above) is the one I'd mutate first, since it
is the only one of the five `feed-view` ADDED scenarios not corroborated by an
independent CI red run recorded in `tasks.md`.

## 3. NO SPEC gaps

`git grep -n "NO SPEC"` over `dialectica-ui/tests` finds fifteen hits, none in
a line this piece's diff added — confirmed against the full diff, not merely
grep placement. The PR body's "None." claim for NO SPEC is accurate for what
this piece adds.

## 4. Requirements moved between capabilities

Not applicable in the cross-capability sense — the delta is a REMOVED+ADDED
pair *within* `stoa-navigation-view` (forced by the tool's refusal to let a
MODIFIED block drop a scenario, per proposal.md), not a move to a different
capability. I checked it anyway as the closest analogue: compared the ADDED
requirement's carried-over paragraphs and scenarios word-for-word against the
REMOVED one in `openspec/specs/stoa-navigation-view/spec.md`. The first four
paragraphs (both-halves, in-full, no-reconstruction) are verbatim. Of the six
scenarios, "A Stoa joined in this session can be shared," "A share carries both
halves," "What is shared is what a join accepts," and "No share is offered for
a Stoa whose record the view does not hold" are verbatim; the other two are the
two the migration note says are replaced, and are in fact replaced with
opposite/adjusted content as described. The MODIFIED "Joining shows..."
requirement's only change is the citation name, exactly as the migration note
claims — checked by full-text comparison against the live spec. No smuggled
behaviour change found.

## 5. Spec soundness

- **Self-consistency:** the `feed-view` ADDED requirement is consistent with
  the capability's existing Purpose and requirements (ordering, time-marking);
  no contradiction found.
- **Testability:** no scenario in the delta or in `thread-view`/`moderation-view`
  asserts something no test could check.
- **Staleness against #134:** read fresh with `gh issue view 134`. The issue's
  "What this needs" explicitly lists widening past join-refusal to the feed,
  thread and moderation screens, and separately calls out that a successful
  join needs a seeder — exactly the split this piece and proposal.md describe.
  No out-of-scope requirement found; the issue remains open and consistent with
  "Part of #134."

## Challenging the `readThreadArea`/`moderateArea` impossibility claim

Per the runner's specific instruction: checked whether any existing spec in
`dialectica-ui/tests/` uses `mouseClick` with a window, as a precedent against
`tasks.md`'s claim that these two controls "genuinely have no component-test
click" because `tst_navigation.qml` has no window for `mouseClick`.

**`git grep -n "mouseClick" dialectica-ui/tests` returns nothing at all** — no
test file in this suite calls `mouseClick`, anywhere, not only in
`tst_navigation.qml`. So the literal claim about `tst_navigation.qml` is true,
and it is true of every other file too.

**But the suite does have a precedent for a real, rendered window inside a
component test**, which the claim's framing doesn't address: `git grep -n
"windowShown" dialectica-ui/tests` finds it in two files,
`tst_render_probe.qml` and `tst_screen_frame_geometry.qml`. Both build a
`TestCase { when: windowShown }`, parent a real screen component to it, and
wait for a render pass (`waitForRendering`) to measure something a
parentless, unshown component test cannot — colour, in one case, and
implicit-height geometry in the other. `tst_screen_frame_geometry.qml`'s own
header comment states the point directly: *"'QtTest cannot measure geometry'
is false, and this file is the counter-example."*

**So "no component test can click these controls" is a stronger claim than "no
component test currently does," and the stronger claim wasn't checked against
the precedent already in the tree.** A `MouseArea` is a rendered, positioned
item exactly like the ones `tst_screen_frame_geometry.qml` already measures
inside a shown window; nothing found in this review rules out parenting
`DStoaListScreen`'s row (or the feed header) to a shown `TestCase` and calling
`mouseClick` against `readThreadArea`/`moderateArea` directly, the same way
`tst_render_probe.qml` already samples pixels from a shown component's
render. This is not a claim that it *would* work — I did not build the probe,
per the brief — only that the "genuinely...no component-test click" framing
in `tasks.md` was not tested against the one precedent in this repo that bears
on it directly.

**This is not a coverage gap**: `readThreadArea` and `moderateArea` are both
clicked for real by `thread.yaml` and `moderation.yaml` at the sitometres
layer, which is the layer that can observe a real click landing on a real
window — arguably closer to what matters than a component test would be. The
finding is about the strength of the claim recorded in `tasks.md`, not about
missing coverage.

- [ ] **`tester`** — `tasks.md`'s claim that `readThreadArea`/`moderateArea`
      "genuinely have no component-test click" is stated as an impossibility
      but was only checked against `tst_navigation.qml`'s absence of a window,
      not against the `when: windowShown` precedent already in
      `tst_render_probe.qml`/`tst_screen_frame_geometry.qml`. **Scenario:** a
      future reader treats "genuinely...no component-test click" as settled
      and doesn't reconsider it when a third windowed-test need arises,
      because the claim reads as checked rather than as "not attempted."
      **Measured:** `git grep -n "mouseClick" dialectica-ui/tests` returns no
      hits at all (not just in `tst_navigation.qml`); `git grep -n
      "windowShown" dialectica-ui/tests` returns two files building exactly
      the rendered-window recipe a click test would need. Either qualify the
      claim in `tasks.md` (e.g. "not attempted against the windowed-test
      recipe" rather than "genuinely...no"), or attempt the windowed
      `mouseClick` and record why it does or doesn't work. Low severity: the
      real click is already proven at the sitometres layer in
      `thread.yaml`/`moderation.yaml`, so nothing is uncovered either way.

## Clean

Requirement-to-test mapping for all four new specs, the REMOVED/ADDED
migration in `stoa-navigation-view`, the MODIFIED citation-only change, the
NO SPEC sweep, and the issue-staleness check are all clean. The two CI runs
spot-checked with `gh run view` match `tasks.md`'s claims exactly.
