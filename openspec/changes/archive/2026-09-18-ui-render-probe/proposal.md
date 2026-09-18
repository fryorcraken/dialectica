# Prove the screens paint something

## Why

**Dialectica's QML suite has no rendering verification of any kind.** No spec
grabs an image, samples a pixel, or asserts that anything reached the screen.
Every existing spec reads properties, counts rows and inspects text — all of
which a component satisfies perfectly while painting nothing at all.

That gap has a named shape in this codebase. `run-qml-tests.sh`'s
`check_bindings` closes the case where a binding evaluates to `undefined`, and
`check_qml_names.py` closes the case where a type name collides with basecamp's.
**Neither can see a component whose children never enter the scene graph.**
Nothing warns, no binding is undefined, no name collides, every gate stays green,
and the symptom is a blank view — which CLAUDE.md records as indistinguishable
from a plugin that failed to load.

The sibling repo has the defect on record rather than as a hypothesis.
`radicle-ui`'s `CommitView.qml` declared a property named `data`, shadowing
`Item`'s default property, so **every** declared child silently failed to become
a scene-graph child: `children.length` reported 0 and the whole view — header,
diff, everything — rendered as a blank rectangle. Its probe,
`tests/tst_commitview.qml:132-156`, catches it by grabbing the item and asserting
the result is not a single flat colour, and it passes in that repo today.

PLAN.md §10's standing rule is that **a green gate which cannot see the thing it
claims to check is worse than no gate**, and it already calls the `qml` job "the
weakest gate that exists". This is one specific blindness in that job, with a
known reproduction and a cheap check.

## What Changes

A **rendering probe**: one or more QML specs that instantiate a screen, grab it,
and assert the grabbed image is not a single flat colour.

- **No golden image, no reference fixture, no pixel-diffing.** The assertion is
  "more than one colour is present", which is invariant to fonts, antialiasing,
  theme tokens and platform. This is what keeps it out of the
  rendering-determinism problem that makes screenshot tests brittle.
- **Sampled on a stride**, not exhaustively — a full scan of one screen is
  530,000 pixel reads and buys nothing, since a component that paints anything at
  all differs from its top-left within a few pixels.
- **The probe must be demonstrated to fail** on a component that paints nothing.
  A probe that cannot fail is this repo's most-documented defect class, and the
  spec requires the demonstration rather than assuming it.
- **Scope is the four existing screens** — `FeedScreen`, `DStoaListScreen`,
  `DJoinScreen`, `DOnboardingScreen`. All four exist in
  `dialectica-ui/src/qml/` and all four are already instantiated under
  `qmltestrunner` by the current suite, so none needs new machinery to reach.

### Two findings from running this, which the spec is written around

Both were measured on Qt 6.10.3 under this repo's runner, because the brief asked
for verification rather than assumption. **They are the difference between a
working probe and a green one that measures nothing**, and neither is guessable
from the sibling's source.

**1. The subject must not be the `TestCase` itself or a child of it.** Written
the way every existing spec in this suite builds its subject — `TestCase` as the
visual root, subject created into it — `grabImage` returns a **uniformly white
buffer regardless of what was painted**. Measured: a populated `FeedScreen`
grabbed 1000x530 with *zero* of 530,000 pixels differing from white, and a plain
red `Rectangle` grabbed as white rather than as red. A probe written in the
suite's ordinary shape would therefore fail on a perfectly healthy screen, and
the natural repair — loosening the assertion until it passes — produces a probe
that can never fail.

The sibling repo's shape works: an `Item` root, with the subject and the
`TestCase` as **siblings** inside it. Measured in dialectica with that shape, the
same `FeedScreen` grabbed 148,425 differing pixels.

**2. Subjects must not overlap.** With the `Item`-root shape, `grabImage(item)`
captures the window region at that item's geometry. Two subjects stacked at the
origin each grab the other's paint: measured, a **uniform white** rectangle
reported 4,824 differing pixels purely because another item sat beneath it. Laid
out non-overlapping, the grabs are faithful — a red-and-blue item reported
`topLeft=#0000ff` and 4,375 differing, a solid green one `#008000` and 0, and an
`Item` painting nothing `#ffffff` and 0.

**The probe discriminates the real defect.** Staging the sibling's exact bug — a
root declaring `property var data`, with two declared children — against a
healthy twin, non-overlapping, measured on Qt 6.10.3:

| subject | `children.length` | flat? |
|---|---|---|
| healthy | 2 | **false** |
| `data` shadowed | 0 | **true**, `#ffffff` |

That is the demonstration the spec requires, reproduced in this repo rather than
inherited from the sibling's.

## Capabilities

### New Capabilities

- `view-render-probe`: what a rendering probe asserts, which screens carry one,
  what it must be demonstrated to catch, and — stated as requirements because
  the boundary is what stops it being trusted for more — what it does not cover.

**Why a new capability rather than a requirement added to an existing one.** The
inventory has no capability about the view's test apparatus. The closest are
`feed-view`, `stoa-navigation-view` and `view-identity-onboarding`, and each
contracts what its screen may **claim** — the honesty of a label, a count, a
rendered time. "Something was painted" is a different predicate over all four
screens at once, so splitting it across them would put one rule in four places,
which `docs/OPENSPEC-ARCHIVE.md` names as the failure that reorganisation exists
to prevent.

### Modified Capabilities

None. No existing requirement changes: this adds a check over screens whose
contracts stay exactly as they are.

## Impact

- **Added:** QML spec(s) under `dialectica-ui/tests/`, picked up automatically by
  `run-qml-tests.sh`'s `tst_*.qml` loop and therefore by CI's `qml` job with no
  workflow edit.
- **No source changes.** Nothing under `dialectica-ui/src/qml/` or `dialectica/`
  is modified — this is a gate over existing behaviour, not a change to it.
- **No new dependency.** `grabImage` is QtTest's own, verified working under this
  repo's `QT_QPA_PLATFORM=offscreen` runner.
- **Runtime cost is a consideration rather than an afterthought.** The probes
  need `when: windowShown` and a rendering wait; the feasibility runs took about
  10 seconds per spec file, against 77ms for a spec that renders nothing. Keeping
  the probes in **one** spec file rather than one per screen is the difference
  between paying that once and paying it four times, and `design.md` owns that
  choice.
