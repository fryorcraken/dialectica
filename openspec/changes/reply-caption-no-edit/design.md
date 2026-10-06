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

- The caption, and every other string the screen authors with the reply
  composer or in its place behind a shut posting gate, says nothing about
  whether a reply can be edited, re-published in a later version, or read in
  an earlier one: neither that it can nor that it cannot (Decision 4).
- The caption keeps stating what a reply is, that it is signed.
- A QML component test that fails if such a statement is reintroduced, worded
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
  today ('A reply is a signed record.')". The issue's expected behaviour is
  that "the caption describes what a reply is, and promises only what the app
  does", and the remaining half does exactly that: a reply is a signed op.
  `thread-view`'s *The open reply composer states that a reply is signed*
  contracts it, before a publish and after one, on the statement rather than
  the string, so removing the caption, or rewording it to something that no
  longer says a reply is signed, breaks a requirement.

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
longer repeated in the tree. It states the standing rule and its reasons, and
cites the capability, `thread-view`, rather than this change: the rule outlives
the change, and this folder moves under `changes/archive/` when the change
lands, so a pointer to it by name would read as a live path that is not one.
It carries no history of what the caption used to say; `git log` has that.

### 3. How the test recognises an edit or version statement

The spec forbids a *statement*, not a particular string, on three topics:
whether a reply can be edited, whether a later version of it can be
published, whether an earlier version of it can be read. It forbids it in
both directions (Decision 4). The test therefore walks the rendered text
of the composer's group (`replyComposerOpen`) and of the shut gate
(`replyGateShut`), and fails on any string the matcher `claimsEditing` flags.
The matcher is case-insensitive and flags two things:

- a word stemming from edit, revise, amend, rewrite, change, update, modify,
  correct, supersede, overwrite or republish (the last also hyphenated); and
- the word "version" or "versions", in any phrase.

The second is deliberately a bare word, not a list of qualifiers. "Earlier
versions stay readable", "the previous version", "every version is kept" and
"a later version can be published" are one family, and a list of qualifiers
(earlier, previous, prior, older, later, newer, ...) misses whichever one a
future sentence picks. Nothing the *screen authors* in either group has a
reason to say "version", so the bare word costs no sentence the screen may
write.

That holds for authored text only. The walk also collects text the screen
renders but does not author, which the requirement does not bind: core's
probe reason behind a shut gate, core's message on a refused publish, and the
user's draft. Core's reasons can say "version" truthfully. The probe answers
an unreadable or malformed keystore as a reason rather than an error (the
probe handler's doc in `wire.rs`), and core's own error texts include the
keystore's "keystore format version {v} is newer than this build understands"
(`keystore.rs`) and the identity store's "the identity record declares layout
version {found} ..." (`identity_store.rs`). The matcher flags both. The component suite cannot fail on them today, because
every core string the fixtures supply is hand-written ("no keystore") and
avoids the matcher. A fixture that drove a real reason would fail for a reason
the requirement does not hold, so whether the walk excludes core-supplied text
and the draft, or keeps them and documents why the fixtures avoid the matcher,
is the test suite's to settle against the requirement's two exclusions.

Because the spec forbids a denial as well as a promise, a matcher that flags a
word fits the contract: "A reply cannot be edited" is flagged, and the spec
forbids it. No denial-aware matcher is needed, which is as well, since telling
"cannot be edited" from "can be edited" by pattern would be a second
hand-maintained list in which a wrong exemption passes the claim it was meant
to catch. The matcher approximates the spec from the other side only: a
paraphrase outside its word list passes (Risks).

The walk ignores `visible`: a hidden Text is walked as well. A walk of visible
items only was the alternative, and it would let through a statement that is
hidden when the test looks and shown later, for example one bound to the
composer's `outcome`. Walking everything is stricter than "rendered", and no
item in either group is hidden for a reason that makes it safe to say what the
requirement forbids. Nothing in the guards below fails if the walk is narrowed
to visible items; the choice is recorded on the walk and here.

The walk is scoped to those two subtrees, not the whole screen. The fixture's
root post is revised, so the screen legitimately renders `edited` and "read
the earlier versions" in the thread's rows. A whole-screen walk would fail on
exactly the reports the requirement says it does not bear on. The scope is
itself pinned by `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision`:
on the whole screen the matcher flags both strings, and neither is in either
group's walk. Without it, a matcher that stopped flagging those two strings
would look the same as a walk that stopped reaching them.

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

- **The pattern is tested in both directions**, by four tests: one per family
  of claim (`test_the_matcher_flags_a_claim_that_a_reply_can_be_edited`,
  `..._a_later_version_can_be_published`, `..._an_earlier_version_can_be_read`)
  and `test_the_matcher_leaves_alone_what_the_composers_group_renders`. They
  are separate so the first family to fail does not hide the others. Each
  asserts against strings written out by hand, never against what the screen
  renders. The earlier-version table opens with the bundle's own clause,
  "earlier versions stay readable", and includes the thread rows' own label,
  "read the earlier versions". The last test lists the group's real strings,
  the composer's outcome messages after a publish included.

  A matcher that always says "no claim" makes the three scenario tests pass on
  any tree. Measured: with `claimsEditing` mutated to `return false`, the four
  matcher tests and the scope test go red, and all three scenario tests stay
  green. Measured against the matcher as it stood before the earlier-version
  clause (edit stems and `later|new|... version(s)` only), the earlier-version
  test went red on all six strings, including the bundle's clause, and the
  edit-family test on its four paraphrases using update, modify, correction
  and republish.
- **The scope is pinned**, by
  `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision`: on the
  whole screen the matcher flags `edited` and "read the earlier versions", and
  in neither group's walk is either present. Measured: a Text reading "read the
  earlier versions" added to the open group turns this test red, alongside the
  two scenario tests that walk that group; and a matcher without the
  `versions?` term turns it red on the label.
- **Each scenario test first asserts its walk found text**, anchored on the
  group's own strings and not on the caption. For the open gate that is
  "REPLYING AS", the attribution line, and "Publish the reply", the composer's
  own submit label, which together show the walk covers the group and descends
  into the composer in it. For the shut gate it is the gate's own heading.
  Without the anchor, a walk that reached the wrong item, or none, would
  collect nothing and report no claim.

  The anchor is not the caption because the caption's removal breaks a
  different requirement, *The open reply composer states that a reply is
  signed* (Decision 1), which `test_the_caption_beside_the_composer_is_kept`
  pins; the claim tests must not report that as an inability to look.
  Measured: hiding the caption turns only
  `test_the_caption_beside_the_composer_is_kept` red. What the caption's
  absence leaves open is a caption moved out of the group with a claim in it,
  so a caption anywhere on the screen must lie inside the walked group.
  Measured: moving it out, carrying "Earlier versions stay readable.", turns
  the open-composer and after-publish tests red on that assertion.
- **The old caption turns exactly two tests red**: the open-composer test and
  the after-publish test, each failing on the claim assertion and naming the
  sentence. Measured on the old caption text.
- **The after-publish test is a test of its own, not a copy of the open one.**
  It fails alone when only the post-publish caption carries the claim.
  Measured: a caption whose text changes to "You can update it later." once
  the composer's `outcome` is `"stored"` turns that test red and no other. It
  asserts the composer's `outcome` is `"stored"` and the thread holds two
  items before it walks, so a fixture whose `publish_reply` and `read_thread`
  stop agreeing fails on those assertions and not by walking the
  pre-publish tree and passing.
- **The shut-gate test cannot fail on the old tree**, because the shut gate
  never carried the claim. Its ability to fail was measured by temporarily
  adding an earlier-version sentence to the shut gate's text. That test, and
  only that test, went red. The mutation was then reverted.
- **A placeholder is in the walk without a clause for it.** A `TextField` with
  `placeholderText: "Write a reply. You can edit it later."` added to the open
  group turns both claim tests red through the plain `text` walk, because the
  default style renders the prompt through a child Text. The walk therefore
  does not read `placeholderText`. Text a popup or an attached tooltip carries
  is outside a walk of `children`, and this suite does not see it.

### 4. Say nothing on editing, rather than deny it

`thread-view` forbids the screen's text to say whether a reply can be edited,
re-published or read in an earlier version, in either direction. "A reply
cannot be edited" is as forbidden as "It can be edited later".

A denial is true today, and that is not enough. Editing is planned work:
the owner's decision names it (#99, `revisePost`) as out of 0.0.1, not out of
the product. A denial written now goes false on the day editing lands, and
nothing on that day prompts anyone to remove it. That is the failure this
issue fixed, run in the other direction: copy describing a facility's state,
left behind when the facility moved. The issue asks for a caption that
"describes what a reply is, and promises only what the app does". A
statement that a reply is signed describes what a reply is. A statement
about editing describes what the app does, and is exactly as perishable
whichever way it points.

The owner's decision was also to drop the text: "The reply confirmation
should not say a reply can be edited later." It was not a request to replace
the text with its negation.

**Alternatives considered:**

- *Permit an honest denial*, and have the test tell a denial from a promise.
  Rejected for the staleness above, and because the test would need a
  denial-aware matcher, a second hand-maintained list in which a wrong
  exemption passes the claim it was meant to catch (Decision 3).

**What breaks without it:** nothing in the component suite, which flags
both directions because it reads words. The rule is what makes that matcher
fit the contract rather than exceed it: without it, a test flagging "A reply
cannot be edited" would be failing on text the spec allows.

## Risks / Trade-offs

- [The pattern is a word list, and a paraphrase outside it would pass] →
  The verb stems cover what a promise to edit would use, and the bare word
  "version" covers every phrasing of a later or an earlier one. A reviewer
  adding copy to this group is the backstop. This is the hand-maintained-list
  shape, and it is accepted here because the alternative, pinning exact
  strings, fails on every unrelated copy change. A paraphrase that avoids both
  ("You can redo it", "reread what it said before") passes, and the table in
  the matcher test is where a missed one is added.
- [The bundle's own clause, "earlier versions stay readable", promises reading
  a prior version, not editing or publishing one] → The requirement forbids
  it as well, because nothing reads a prior version, so it would promise an
  absent facility exactly as the edit claim did. The matcher flags it, and the
  matcher test pins that string.
- [A future sentence the spec permits in this group, using one of those words,
  fails the test; "the draft can be edited before publishing" is about the
  draft, not a published reply] → That is the right moment to read the
  requirement, which the comment above the tests names, and narrow the pattern
  or the scope deliberately. A denial ("A reply cannot be edited") is not such
  a sentence: the spec forbids it too (Decision 4), so the matcher flagging it
  is the contract, not over-reach.
- [The test sees what `qmltestrunner` renders, not what Basecamp renders] →
  The caption is a static string in the plugin's own file with no host
  dependency, so the two cannot diverge in what text is set.
