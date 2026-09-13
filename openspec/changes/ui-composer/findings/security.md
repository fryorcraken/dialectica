# Security — `ui-composer`

Reviewed on `piece/ui-composer` at `bb3219c`, in a worktree of its own. Suite
green at 107 tests before and after every mutation; all mutations reverted and
`git status --porcelain` is empty.

The attack surfaces this piece adds are: core's refusal message rendered
verbatim, the probe's reason rendered verbatim, peer-supplied feed rows reaching
a new vote control, and the author's own draft reaching `callModule`.

## What is clean

**Nothing attacker-controlled renders as markup, and this is checked by format
rather than by content.** Every text-bearing element in the changed files
declares `textFormat: Text.PlainText` (or `TextEdit.PlainText`), including the
two that carry strings the view did not write: `PublishOutcome.qml:99` for
core's refusal detail and `FeedScreen.qml:609` for the probe's reason.

The sibling `ui-stoa-list` defect — a `StyledText` mutation surviving because a
test asserted `Text.text`, the source string, which the format does not change —
is closed here by `nonPlainTextElements()` walking the tree and asserting the
*property*, not the string. `test_a_refusal_message_containing_markup_is_not_rendered_as_markup`
feeds `"<b>no such op</b> is held by this <i>peer</i>"` and asserts both that
the source arrives verbatim and that the element holding it has
`textFormat === 0`. That second assertion is the one that fails under the
mutation; the file says so and it is right.

I checked the CI gate that backs this (`ci.yml:262-274`, counting `\bText \{`
openings against `textFormat:` assignments) against `Composer.qml`: 3 openings,
4 assignments, so it passes. The pattern deliberately does not match
`SanitisedText {`, and I confirmed it also does not match `TextEdit {` — the
`TextEdit` at `Composer.qml:257` is covered by its own explicit
`textFormat: TextEdit.PlainText` at line 270 rather than by the gate.

**Peer text still goes through the sanitiser.** The new vote control sits beside
`SanitisedText`, which is unchanged and renders `value.text` as PlainText with
no property to override it. The row's `author` goes to `PostHeader` and the
hidden-row marker is a literal. Nothing this piece adds binds a raw peer string
to anything but a PlainText element.

**The draft reaches core unaltered.** `Composer.draft` is an alias to
`field.text` rather than a two-way binding — the comment at lines 22-35 gets the
reason right, and a two-way binding would have broken the first keystroke — and
nothing on the path to `Core.publishPost` trims, normalises or strips. The
`TextEdit` is `PlainText` for the second reason the comment names: a rich-text
editor normalises what is typed into it, which for a body signed over its bytes
is a correctness bug as well as a surprise. `test_the_body_sent_is_byte_for_byte_the_draft`
asserts against the raw argument string handed to `callModule`, which is the
only way to prove it.

**The gate fails closed, structurally.** `FeedScreen.qml:140`'s
`probe.ok && probe.value.canPost === true` is a strict comparison, so `"true"`,
`1`, `null`, absent and a non-object probe reply all reach the same closed state
without a branch per shape. `tst_gate_affordance.qml:186` drives eight such
shapes and, for each, asserts three separate things: nothing writable, no
submit affordance, and **no text input of any kind** — the third found by
`readOnly !== undefined` rather than by editability, which is what catches a
read-only preview box that the editable probe cannot see by construction. That
is a genuinely strong gate test and I could not get a box past it.

**The vote path applies the same success test as the composer.**
`FeedScreen.qml:114` requires `reply.ok` and a non-empty string `opId` before
`recordVote`, so a malformed or error reply leaves the control showing what it
showed. `ownVotes` is session-only with nothing persisting it.

**No secret material is compared or handled here.** The piece adds no
comparison of key material, no timing-sensitive path, and no allocation sized
from peer input — the QML engine has no indexing or arithmetic reachable from
peer bytes in the way core does. There is no panic-equivalent: `Core.call()`
wraps both `callModule` and `JSON.parse` in `try`/`catch`, and `utf8Length` was
deliberately written not to throw on the unpaired surrogate that
`encodeURIComponent` raises `URIError` on.

## Findings

- [ ] **`dev-writer`** — `FeedScreen.qml:427` — two feed rows missing
      `currentVersion` collide on one vote slot, so a vote on one marks the other
      **Scenario:** the vote control binds
      `screen.ownVotes[row.modelData.currentVersion] || 0` and `voteOn` is called
      with the same expression. A row whose `currentVersion` is absent keys the
      map on the JavaScript value `undefined`, which stringifies to the single
      key `"undefined"`. Feed rows are peer-derived content reaching the view
      through `list_threads`; `FeedScreen.reload()` validates that `items` is an
      array (line 176) but validates **nothing about the shape of each element**.
      So a reply carrying two such rows renders two controls that share one slot.
      The spec's requirement "The vote control shows the viewer their own vote
      back" states "a vote on one post does not mark another", and
      `design.md:178-181` claims this is held "by construction ... because the
      key *is* the post" — that argument holds only while every row has a
      distinct `currentVersion`, which is a property of peer-supplied data rather
      than of the code.
      **Measured**, with a scratch spec file driving the real `FeedScreen`
      against a `list_threads` reply whose two rows omit `currentVersion`
      (probe written, run, and deleted; it is not part of this branch):
      after `voteOn(rows[0].currentVersion, 1)` with core replying
      `{"opId":"votedop","wasNew":true}`, `ownVotes` is `{"undefined":1}` and
      `ownVotes[rows[1].currentVersion]` reads back **`1`** — a vote on row 0
      marks row 1. Separately, the request that reached `callModule` was
      `{"stoa":"abab…","direction":"up"}` with **no `target` field at all**,
      because `JSON.stringify` omits a key whose value is `undefined` — so the
      view sends core a vote naming no target and then marks two controls on
      core's answer to it.
      The existing test `test_a_published_vote_is_reflected_on_that_posts_control_only`
      (`tst_vote_and_gate.qml:135`) cannot see either half: its `twoRows()`
      fixture gives both rows explicit distinct `currentVersion` values `"v1"`
      and `"v2"`, and no fixture anywhere in the suite omits the field. All 107
      tests stay green. **Severity: medium.** It needs peer rows that omit a
      field core currently always sends, so it is a "does not rest on a guarantee
      made one module away" gap of exactly the kind `FeedScreen.qml:169-175`
      already argues for on `items`. The same argument applied one level down is
      the fix: a row whose `currentVersion` is not a non-empty string should not
      key the map or reach `publishVote`.

- [ ] **`tester`** — `tst_vote_and_gate.qml:51-60` — the two-row fixture cannot
      expose a row-shape defect, because both rows are well-formed
      **Scenario:** `twoRows()` is the only feed fixture used by the vote tests
      and every row in it carries every field. The suite therefore has no case
      where a peer sends a row with a missing or non-string `currentVersion`, a
      missing `body`, or a `body` that is a bare string rather than the
      `{text, removed, marked}` object `SanitisedText` expects. The standing rule
      is that everything arriving over the network is attacker-controlled;
      `list_threads`' rows are that, and the view's only validation of them is
      `Array.isArray(items)`. **Measured:** I grepped the four new test files for
      any fixture row omitting a field — there is none; every `items` array in
      `tst_vote_and_gate.qml`, `tst_gate_affordance.qml` and `tst_feed_states.qml`
      uses the complete shape. A row-shape case belongs beside the existing
      malformed-*reply* cases, which are good. **Severity: medium.**

## What I could not check

- **Whether core can actually emit a row without `currentVersion`.**
  `wire.rs:1476` maps `"currentVersion": row.current_version` unconditionally,
  so today's core always sends it. The finding stands on the boundary rule
  rather than on a reachable core path: the view is the boundary for anything
  `list_threads` returns, and it does not validate element shape. If the
  reviewers agree the guarantee one module away is sufficient here, that is a
  decision to record rather than a check to add — but it should be recorded,
  because the identical argument was explicitly rejected one level up at
  `FeedScreen.qml:169-175`.
- **Rendering and layout.** Nothing in this repo can see that a refusal is
  legible or that core's verbatim message cannot overflow its container. A very
  long single-token refusal renders with `wrapMode: Text.WrapAnywhere`
  (`PublishOutcome.qml:98`), which is the right choice, but whether it stays on
  screen is unverified.
- **`qmllint` exit status.** My invocation used `--bare`, which broke module
  resolution; CI's (`ci.yml:562`, `-I dialectica-ui/src/qml`, no `--bare`) is
  the authority and I did not reproduce it.
- **No information leak found, but stated as a negative:** core's refusal and
  probe reason are rendered verbatim by design and by spec, so anything core
  chooses to put in an error reaches the screen. That is core's disclosure
  decision, not this piece's, and the view is explicitly forbidden from
  rewording or branching on it.
