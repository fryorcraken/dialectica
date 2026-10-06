## Context

The thread screen (`dialectica-ui/src/qml/DThreadScreen.qml`) renders a caption
under its reply composer. It read "A reply is a signed record. It can be edited
later." See proposal.md, "Why", for what is wrong with that.

The comment above the caption explained it as the design bundle's
`replyCaveat` with its last clause, "earlier versions stay readable", dropped
because the earlier-versions control on a revised post is inert. The archived
design of the change that built the screen
(`openspec/changes/archive/2026-09-28-ui-thread-view/design.md`, the bullet
"The bundle's `replyCaveat` is not used verbatim") makes the same argument and
ends "what remains is true: a reply is a signed record that can be edited
later."

## Goals / Non-Goals

**Goals:**

- The caption, and every other string rendered with the reply composer or in
  its place behind a shut posting gate, makes no claim that a reply can be
  edited or re-published in a later version.
- A QML component test that fails if such a claim is reintroduced, worded
  differently or not.

**Non-Goals:**

- Building editing. No affordance, no core method, no change to the module
  surface.
- Touching the `edited` marker or the inert "read the earlier versions" row on
  a revised post. Both report what an author did to a post, which the new
  requirement says it does not bear on.
- Editing the archived `ui-thread-view` design. It is a record of what was
  decided then; this document carries the correction (Decision 2).

## Decisions

### 1. Drop the sentence; build no editing

The caption now reads "A reply is a signed record." and nothing else.

The issue offered two ways out: cut the caption to what is true today, or build
the editing it promises. The owner decided on 2026-09-29, in the issue:

> **Decision (owner, 2026-09-29): drop the text.** The reply confirmation
> should not say a reply can be edited later. Editing (#99, `revisePost`) is
> not in 0.0.1, so don't build it here.

No editing exists for the sentence to describe.
`git grep -n -i "revise" -- dialectica/rust-lib/src/lib.rs dialectica-ui/src/qml/Core.qml openspec/specs/content-authoring`
returns nothing, so there is no revise method on the module surface, none on
the view's call path and none in the authoring contract. Widening the pattern
to `revis|edit` over the first two files finds only comments and no method.
The comments that concern posts say that a post's *id* does not move when the
post is edited. That is about how core resolves revisions it receives
(`post-revision`), not about whether this interface can author one.

**Alternatives considered:**

- *Build editing.* Ruled out by the owner and by the milestone: it is tracked
  separately and not in 0.0.1.
- *Remove the caption entirely.* "Drop the text" could be read that way. It is
  not how this change reads it. The owner was choosing between the issue's two
  options, and the first was explicitly "drop the sentence to what is true
  today ('A reply is a signed record.')". The remaining half is true, because a
  reply is a signed op. The spec does not say whether the caption stays, so the
  test that keeps it carries a `NO SPEC:` marker. That makes it something the
  spec-writer can confirm or overturn, not a contracted fact.

### 2. The reasoning this replaces was wrong on its own terms

The old comment, and the archived design bullet it summarised, dropped
"earlier versions stay readable" because it "promises the facility the control
above is inert for". By the same test, "It can be edited later" had to go too.
Reading a prior version and publishing a new one are both missing, and the
second is the more absent of the two: the earlier-versions control is at least
rendered, inert, while nothing on the screen offers to edit anything. The
argument removed the weaker false promise and kept the stronger one.

This document supersedes the archived bullet's last sentence. The archive is
not edited, because an archived design records what was decided when it was
decided. Anyone who greps the archive for the caption will find both documents.
The code comment above the caption is rewritten here so the false claim is no
longer repeated in the tree. It now says what a reply is, and why no editing or
version claim may be added beside it.

### 3. How the test recognises an edit claim

The spec forbids a *claim*, not a particular string. The test therefore walks
the rendered text of the composer's group (`replyComposerOpen`) and of the
shut gate (`replyGateShut`), and fails on any string matching an edit-claim
pattern: a word stemming from edit, revise, amend, rewrite or change, or a
"later/new/newer/another/next version" phrase, case-insensitive.

The walk is scoped to those two subtrees, not the whole screen. The fixture's
root post is revised, so the screen legitimately renders `edited` and "read
the earlier versions" in the thread's rows. A whole-screen walk would fail on
exactly the reports the requirement says it does not bear on.

**Alternatives considered:**

- *Assert the old sentence is absent.* A reworded promise ("Replies may be
  revised") would pass. That is the failure the requirement is written
  against, so it was rejected.
- *Pin the exact set of strings in the group*, as the inert-row test does for
  its two labels. The group renders the composer's byte counter, outcome
  messages and core's verbatim refusal reason. Each of these changes with the
  fixture and with unrelated copy work, so the test would fail for reasons
  other than the claim. Rejected.

**Guards, and what breaks without each:**

- **The pattern is tested in both directions** by
  `test_the_edit_claim_matcher_flags_edit_claims_and_nothing_else`. A matcher
  that always says "no claim" makes the three scenario tests pass on any tree.
  Measured: with `claimsEditing` mutated to `return false`, this test is the
  only one in `tst_thread_reply.qml` that goes red, and all three scenario
  tests stay green.
- **Each scenario test first asserts its walk found text.** For the open gate,
  that is the caption itself, found by `objectName: "replyCaption"`, which was
  added for this. For the shut gate, it is the gate's own heading. Without that
  assertion, a walk that reached the wrong item, or none, would collect nothing
  and report no claim.
- **The old caption turns exactly two tests red**: the open-composer test and
  the after-publish test, each failing on the claim assertion and naming the
  sentence. Measured on the old caption text with only the `objectName`
  added, so neither failed on the non-vacuity check.
- **The shut-gate test cannot fail on the old tree**, because the shut gate
  never carried the claim. Its ability to fail was measured by temporarily
  adding a Text reading "a reply can be edited later" to the shut gate. That
  test, and only that test, went red. The mutation was then reverted.

## Risks / Trade-offs

- [The pattern is a word list, and a paraphrase outside it would pass] →
  The list covers the verbs a promise to edit would use, and the phrases a
  promise to publish a later version would use. A reviewer adding copy to this
  group is the backstop. This is the hand-maintained-list shape, and it is
  accepted here because the alternative, pinning exact strings, fails on every
  unrelated copy change.
- [The bundle's own clause, "earlier versions stay readable", is not matched
  on its own] → It promises reading a prior version, not editing or
  publishing one, so the new requirement does not forbid it and the pattern
  does not flag it. It stays out of the caption because nothing reads a prior
  version, and the rewritten code comment says so. That is a fact about the
  module surface, not a contracted rule, and it is recorded here so that
  nobody reads the test's silence as permission.
- [A future, true sentence in this group that uses one of those words, such as
  "the draft can be edited before publishing", fails the test] → That is the
  right moment to read the requirement, which the comment above the tests
  names. If the sentence is about the draft and not about a published reply,
  the pattern or the scope needs narrowing, and that is a deliberate edit.
- [The test sees what `qmltestrunner` renders, not what Basecamp renders] →
  The caption is a static string in the plugin's own file with no host
  dependency, so the two cannot diverge in what text is set.
