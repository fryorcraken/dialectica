## Why

A reader can reach a Stoa's feed and see its thread heads, and can publish a
top-level post — but there is nowhere to read a thread or answer one. `Main.qml`
is a three-screen navigator (list, join, feed) and records the gap in its own
header: `DComposer.qml` supports replying and is tested in that mode, and "the
instantiation arrives with the thread screen". `Core.qml` binds `publishReply`
and does **not** bind `readThread`, so the one call this screen is built on has
no route out of the view at all.

That leaves the two MVP items `docs/PLAN.md` §9.2 lists as "Reply to a post" and
"view a thread" unreachable, while the core half of both is built, contracted and
merged (`thread-read`, `content-authoring`). This is the last unbuilt screen the
MVP needs, and the missing half is entirely the view's.

## What Changes

- **A new `thread-view` capability**, contracting the screen that renders one
  thread: how a flat page of items becomes a nested rendering, what is rendered
  for an item whose parent the view does not hold, how the revised marker is
  presented, and which affordance is inert because no call backs it.
- **`Core.qml` gains a `readThread` binding**, through the view's single call
  path, so the screen has a call to make. The absent binding is why the screen
  cannot exist today.
- **`Main.qml` gains a route to the thread screen and a route back out**, making
  it a four-screen navigator. `stoa-navigation-view`'s "Every state a user can
  enter has a specified way out" is the standing rule this must satisfy, and it
  is also the requirement that names the feed-to-list return as a gap left open
  "without the feed gaining a way to say it is finished" — the thread screen is
  reached from the feed, so this change adds a transition the feed must be able
  to leave and return from.
- **The reply composer is wired to `publishReply`**, not inert: core serves it,
  and `docs/PLAN.md` §9.2's ruling 4 case 1 makes an unwired control a defect
  rather than a phase. `composer-view` already contracts everything about what
  the composer does once a reply is submitted or refused; this change supplies
  the instantiation those requirements were written for and restates none of them.
- **The "read the earlier versions" affordance is rendered inert**, under
  `composer-view`'s existing inertness convention rather than a second one. No
  contract method reads prior versions — `docs/PLAN.md` §9.2 case 2 entry 4
  already documents this, so the placeholder is documented before it is built.
- **No score, tally or vote control on this screen.** Voting is out of the MVP by
  ruling 1, and `composer-view`'s "The vote control displays no score" governs
  any control that did ship.

## Capabilities

### New Capabilities

- `thread-view`: What the thread screen renders given a flat page of thread
  items — the nesting it computes, what an item whose parent is off the page or
  hidden is rendered as, how a revised post and a withheld body are presented,
  which affordances are inert and why, and how the screen is entered and left.

### Modified Capabilities

None. `thread-read` contracts the core call this screen consumes and is not
changed by rendering it; `composer-view` already contracts the reply composer's
behaviour and required none of its requirements to be reachable from a screen
("a reply box under a thread head would be a thread-view affordance on a screen
that is not one"). This change makes them reachable without altering them.

## Impact

- **`dialectica-ui/src/qml/Core.qml`** — adds the `readThread` binding. No other
  core call changes.
- **`dialectica-ui/src/qml/Main.qml`** — a fourth screen and its two transitions.
  Its existing invariant that the navigator's whole state is which property is
  non-null, with no `StackView`, is the constraint a fourth screen has to hold.
- **A new thread screen component** under `dialectica-ui/src/qml/`, plus its QML
  tests. Any new QML type name takes the `D` prefix; CLAUDE.md's
  `check_qml_names.py` gate enforces it and the exemption list is not a place to
  add a twelfth name.
- **`DComposer.qml`** — instantiated in its reply mode for the first time. The
  component already supports it and is tested in that mode.
- **No core change.** `read_thread` and `publish_reply` are both on the trait and
  serve this screen as they are.
- **`docs/PLAN.md`** — §9.2 case 2 entry 4 already documents the inert
  earlier-versions control, so no new placeholder needs documenting; what needs
  pruning is §9.1's thread-view prose that this change's spec supersedes.
