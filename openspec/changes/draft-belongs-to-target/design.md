# Design

## Context

See `proposal.md`, "Why", for the defect.

The shape that produced it: `Main.qml` mounts `FeedScreen` and `DThreadScreen`
once each and re-points them at whichever Stoa or thread the navigator chose.
Each screen mounts one `DComposer`, so one composer serves every target of its
kind, and its `TextEdit` outlives the target the text was typed for.
`DComposer.submit()` sends the field's text to whatever `stoaAddress` and
`replyParent` hold when it runs. Nothing connected the two.

Constraints:

- View only. The QML engine has no storage of its own under Basecamp, and no
  core method stores a draft.
- The call into core is synchronous today (`Core.call` through
  `bridge.callModule`). `DComposer` already declines to rely on that for its
  `publishing` guard, and this change follows it.
- A transition in `Main.qml` passes through intermediate values: `enterOnly`
  writes the navigator's states one at a time, and the feed's address and the
  thread's id are each withheld until the Stoa's record has landed. A composer
  therefore sees targets nobody is ever shown.

## Goals / Non-Goals

**Goals:**

- The field never holds text entered for a target other than the one a submit
  would name, at any moment a user or a test could act on it.
- One mechanism for both composers, in the component they share.

**Non-Goals:**

- A new QML type, a change to the navigator, or anything in `Core.qml`.
- An end-to-end spec under `dialectica-ui/tests/ui/`. See Risks.

## Decisions

### 1. Drafts are held per target inside `DComposer`, and the field shows the current target's

`DComposer` keeps `heldDrafts`, a map from a target key to the text entered for
that target. `targetKey` is a binding over what the composer would publish to.
When it changes, the field is set to what is held under the new key, which is
empty unless text was entered for it. Every edit to the field is written to the
entry for the current key as it is made.

Why this matters enough to build, from the issue: an op is signed and permanent,
and there is no delete. A draft that reaches the wrong Stoa is an authored op
under the user's key, visible to every peer in a community it was not written
for and moderated by people it was not addressed to. A misdirected reply also
misrepresents what its author was answering. In a forum built on separate
communities with separate moderators that is a disclosure and integrity problem,
not only a UX gap.

Alternatives, and what ruled each out:

- **Empty the field when a visit begins**, by extending `beginVisit()`'s
  outcome reset to the draft. The issue names this and why it is a separate
  choice: it discards the text on reopening the *same* Stoa. The owner settled
  that a draft is kept per target for the session, so this is ruled out by the
  spec's retention requirement, not by taste.
- **Save on leaving, restore in `beginVisit()`.** It needs every way out of a
  screen to save first, and the navigator's intermediate states make "leaving"
  hard to name: `openThread` empties `chosen` before it sets `reading`. Holding
  each edit as it is made has no moment of leaving to miss.
- **One composer instance per target**, by recreating the screen or putting the
  composer in a `Loader` keyed by target. A draft behind a shut gate or a failed
  read must outlive its composer not being rendered, so the instances would have
  to be kept alive or their text stored anyway, which is this decision with a
  restructured navigator added to it.
- **A store on `Main.qml`, or a `DDraftStore` singleton.** A singleton lives as
  long as the QML engine, so a view opened anew in the same engine would find
  the old drafts, against "A view opened anew MUST hold no draft". A store on
  `Main.qml` has the right lifetime but must be threaded through two screens to
  the one component that reads and writes it. A composer lives exactly as long
  as its screen, which lives as long as the view, so the map inside the composer
  has the required lifetime with nothing passed down.

The post composer and the reply composer are separate instances, so each has
its own map. Nothing depends on that: the key carries the kind (Decision 2).

The owner's remaining answers need no mechanism of their own. In view memory
and gone when Basecamp closes, because persisting would widen the core API: the
map is a QML property. No cap on how many are held: a plain object. A held
draft behind a shut gate stays held and unseen: the composer is still mounted
and still holds the field, and its screen's gate is what stops rendering it,
as before this change. A restored draft is not announced: nothing was added
that could announce one.

### 2. The key is `JSON.stringify([kind, stoaAddress, replyParent])`

The owner's answer is that the key is exactly what the publish sends: the Stoa
address for a post, the Stoa address plus the parent op for a reply. The three
values are the three things `submit()` passes on: which method, which Stoa,
which parent.

- **The parent, not the thread.** Today the parent is always the thread's root.
  Keying on the parent means a later reply-to-this-post control cannot reopen
  the leak: two posts in one thread are two targets from the start.
- **The Stoa as well as the parent.** A feed row's `thread` is peer-supplied,
  so nothing in the view stops two Stoas listing the same op id. They must not
  share a draft, and the spec has a scenario for it.
- **`replyParent`, not `parentOp`.** `replyParent` is the value that reaches
  core, and is "" for a post whatever `parentOp` holds.
- **JSON, not a joined string.** `stoa + ":" + parent` maps `("a:b", "c")` and
  `("a", "b:c")` onto one key, and the parent is peer-supplied text.
  `JSON.stringify` of an array is injective. Every key begins with `[`, so no
  key can be a name `Object.prototype` defines, and `heldDraft` returns only a
  `string` regardless.

A composer with no target (an empty address) gets a key like any other. It is
what the transitions in Context pass through, and it needs no branch: showing
its held draft is a lookup that finds nothing.

### 3. The safety rule rests on one handler: the field is re-filled on the change of target itself

`submit()` still sends `root.draft`, the field's text. What makes that the text
entered for the target it names is `onTargetKeyChanged`, which runs
synchronously inside the property change, before any event can reach a control.

**What breaks without it:** with the handler's body emptied, sixteen tests in
`tst_draft_targets.qml` go red, the cross-target ones of every requirement.
Measured by making that edit and running the file.

Considered: have `submit()` take the body from `heldDrafts` under the key built
from the very address and parent it sends, so the rule would hold from
`submit()`'s own lines. Rejected because it gives the body two sources:
`submittable`, the byte count and the warnings would read the field while the
publish read the map. If the two ever disagreed the user would publish text
they were not shown, which is the defect in another form, and while they agree
no test can tell the two designs apart.

Assigning `text` resets the `TextEdit`'s undo history. That is relied on: undo
in one target's field must not bring back another target's text.
`test_undo_in_another_stoas_field_does_not_bring_the_first_stoas_text` pins it,
entering the text with `insert()` because a write to `text` leaves nothing to
undo and would pass whatever the composer did.

### 4. A store clears the draft under the key the publish named

`submit()` reads `targetKey` before the call and hands it to `applyReply`, which
passes it to `clearDraftOf(key)`. That drops the held entry and empties the
field only when the composer still points at that target.

This is a guard for the day the call stops being synchronous, the same
reasoning `publishing` already carries in that file. Today nothing can re-point
a composer between a submit and its answer.

**What breaks without it:** replacing `clearDraftOf`'s body with
`field.text = ""` turns exactly one test red,
`test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named`.
Measured the same way. That test re-points the composer from inside the fake's
`callModule`, which is the only place it can be done.

### 5. An emptied draft is dropped from the map, not held as ""

Both read back as an empty field. Dropping keeps `heldDrafts` to the targets
that have unsubmitted text, so "no cap" does not also mean an entry for every
target ever visited.

## Risks / Trade-offs

- [No bound on what is held] → The owner's decision. Each entry is text a user
  typed in this session, an over-length draft included, since the spec requires
  one to come back in full.
- [`heldDrafts` is mutated in place, which QML does not notice] → Deliberate:
  nothing binds to it. A future binding over it would silently never update.
  The property's comment says so.
- [`heldDrafts` and `targetKey` are readable from outside] → QML has no private
  members. No screen reads either, and no test asserts through them.
- [The component layer stands in for the host] → Every test drives `Main.qml`
  under `qmltestrunner`, entering text by writing the field's `text` (or
  `insert()` for the undo test). No end-to-end spec was added, so the change
  has not been exercised inside Basecamp, with real key events or an input
  method's pre-edit text.
