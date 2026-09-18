# Code review — `ui-thread-view`

Covers all four dimensions in one pass (correctness, security, readability,
architecture), per `RUNNER.md`'s allowance for a self-contained change. Each
finding below names its dimension.

## What was verified clean (not a checkbox — no action needed)

- **The security property (D1) holds and is proven by mutation.** Changed
  `resolveDepth`'s "parent not on this page" branch from `return -1` to
  `return depth + 1` and ran `tst_thread_nesting.qml`:
  `test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root` and
  `test_items_carrying_no_id_do_not_share_a_parent_slot` both went red (12
  passed, 2 failed); every other test in the file, including the three cycle
  tests (`test_a_cycle_is_not_rendered_at_the_roots_depth`,
  `test_a_parent_cycle_of_two_terminates`,
  `test_an_item_naming_itself_as_its_parent_terminates`), stayed green,
  confirming the cycle route to `-1` is a genuinely separate code path and not
  a second route to the same bug. Reverted; `git status` clean afterward.
- **D10's `thread`-vs-`currentVersion`/`id` fix landed correctly.**
  `FeedScreen.qml:436-442` (`threadTarget`) reads `rowData.thread`, distinct
  from `voteTarget` at `FeedScreen.qml:259-265`, which correctly reads
  `rowData.currentVersion`. Cross-checked against
  `dialectica/rust-lib/dialectica-core/src/wire.rs:6438-6444` (feed row key
  set: `attachments, author, body, currentVersion, isHidden, isRevised,
  thread` — no `id`) and `feed.rs:130-140`. Mutated `threadTarget` back to
  `currentVersion` and ran `tst_navigation.qml`:
  `test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op` failed with
  `Actual: dd2222...` (the edited version) vs `Expected: cc1111...` (the
  stable thread id) — the strengthened fixture (different values for `thread`
  and `currentVersion`) genuinely discriminates the two candidate
  implementations rather than passing either way. Reverted; clean.
- **`publish_reply`'s wire shape matches**: `Core.qml:245-248` sends
  `{stoa, parent, body}`, matching `wire.rs:2944-2955`'s `required_op_id(...,
  "parent")`. No `thread` field is ever sent, matching the "core derives
  thread from parent and refuses one supplied" design (D5).
- **Every `Text` in `DThreadScreen.qml` sets `textFormat: Text.PlainText`
  explicitly** (18 of 18 `Text {` blocks), and `SanitisedText.qml` does the
  same for the three `Text` elements it wraps. No reliance on QML's
  `AutoText` default. Peer-supplied strings (body, author key, Stoa title)
  are rendered as core sanitised them; the view applies no second
  transformation, consistent with D8.
- **The satisfied-by-construction task box holds.** `DThreadScreen.qml`
  instantiates exactly one `DComposer` (line 684), with `kind: "reply"` and
  `parentOp: screen.threadId` (the root). There is no second `DComposer`
  instantiation and no per-post reply affordance anywhere in the file, so a
  reply naming any post other than the root is not constructible through this
  screen's interface — confirmed by reading the whole file, not just the
  composer block.
- **Full suite green**: `sh dialectica-ui/tests/run-qml-tests.sh` — 24 spec
  files, all passing (0 failed across every file, including
  `tst_thread_navigation.qml`, `tst_thread_nesting.qml`,
  `tst_thread_reply.qml`, `tst_thread_states.qml`). Three QWARNs in
  `tst_vote_and_gate.qml` (`Unable to assign QString to int` at
  `SanitisedText.qml:30`) are a pre-existing, deliberately-malformed fixture
  each paired with a PASS — unrelated to this piece.
- **Gates green**: `check_qml_names.py dialectica-ui`, `check_qml_members.sh`,
  `check_qml_reachable.py dialectica-ui` all reported clean.
- **`edited` is wired from `isRevised` only** (`DThreadScreen.qml:460`), never
  from comparing `id`/`currentVersion`, matching the task list's claim.
- **`voteTarget`/`threadTarget` in `FeedScreen.qml` are correctly two separate
  guards reading two different fields for two different purposes** (a vote
  targets the version it was cast on; a thread is opened by its stable root
  id) — this is not the "fourth slightly-different guard" CLAUDE.md warns
  against; the design doc states the distinction explicitly and it is real.

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DThreadScreen.qml:59-101` vs
      `dialectica-ui/src/qml/FeedScreen.qml:83-155` — **[architecture]**
      `capabilityFrom(probe)` and `identityFrom(probe)` are byte-for-byte
      identical between the two files (verified by direct comparison, not
      just similarity) — same branching, same `=== true` strictness, same
      fallback-to-`probe.error` logic. This is the "fourth slightly-different
      guard" pattern CLAUDE.md asks to reshape rather than copy — except here
      there is zero variance, which is a stronger signal than the four-copies
      threshold the rule names. **Scenario:** a future fix to the
      `hasIdentity === true` strictness rule (e.g. to handle a new probe
      shape) applied to `FeedScreen.qml` and not remembered for
      `DThreadScreen.qml` silently reintroduces the exact defect
      `identityFrom`'s own comment describes averting ("a chip handed a
      looser test renders the IDENTITY PRESENT arm... with every gate
      green") — but only on the thread screen, where nothing forces the two
      copies to agree. The natural fix is a shared helper (e.g. on `Core.qml`
      or a small imported JS module), which neither `check_qml_names.py` nor
      `check_qml_members.sh` would object to. Not introduced by this piece —
      `FeedScreen.qml` had it first — but this piece is the second
      occurrence, which is the point at which duplication stops being
      coincidence.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/Main.qml:144-146` —
      **[architecture]** This piece's commit (`4e13bf5`) added
      `rootOp === ""` to `openThread()`'s guard, a function `Main.qml` (owned
      by `piece/ui-navigation`, per design.md's own "What the merge took from
      each side" table) rather than by this piece's `DThreadScreen.qml`. The
      change is spec-backed (`thread-view`'s "No thread is rendered before
      one has been chosen") and the reasoning in the added comment is sound,
      but it edits a file this piece does not own per the recorded split, and
      design.md's D9 section does not mention this edit alongside the other
      recorded cross-piece touches (the transition mechanism, the `REPLYING
      AS` line). **Scenario:** a maintainer reading D9's "what the merge took
      from each side" table to find every place this piece touched
      `Main.qml` will miss this one line — it is real, present in the diff
      against `cc37f2e`, and unrecorded. Confirmed the guard is currently
      unreachable through the live UI (`FeedScreen.qml:864`'s
      `visible: target !== ""` already withholds the affordance for an
      op-less row), so this is redundant defense-in-depth rather than a
      functional bug — but the ownership boundary crossing and the gap in
      D9's record are both real. Low severity; document-only fix (add the
      line to D9's table) would close it.

- [ ] **`dev-writer`** — `openspec/changes/ui-thread-view/design.md:64-70`
      (D9, first paragraph) — **[readability]** States "`reading` is set only
      through `openThread()`, which does not clear `chosen`" as the current
      design, but the "What the merge took from each side" section further
      down the same document explains that navigation's mechanism — which
      DOES clear `chosen` (confirmed at `Main.qml:150`,
      `root.chosen = null`) — was kept instead. Read in isolation, the first
      paragraph describes behaviour the shipped code does not have.
      **Scenario:** a reader who stops at D9's opening paragraph (a plausible
      read order — it is the first mention of the mechanism) comes away
      believing `chosen` stays set while a thread is open, the opposite of
      what `Main.qml:70-101`'s own comment says ("`chosen` is CLEARED while a
      thread is open... One state, one property"). The later section does
      correct it, but nothing in the first paragraph flags that it is
      describing a superseded design rather than the shipped one. A one-line
      forward-reference ("superseded by the merge — see below") would remove
      the ambiguity. Cosmetic; not a functional defect since the code itself
      is internally consistent and correct.

## What this review did not re-litigate

Findings already recorded by other reviewers on this piece are not repeated
here — this file covers only what a reading of the code itself (not the
spec-test correspondence, not the design/decision correspondence) turned up.
