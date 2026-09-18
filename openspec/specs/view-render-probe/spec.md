# view-render-probe Specification

## Purpose
Establishes that each of the view's screens paints something when it is
rendered, closing the gap where a component satisfies every property assertion in
the suite while its children never enter the scene graph and the screen renders
blank. It contracts the weakest useful claim — that more than one colour is
present — and states the boundaries of that claim, because a probe trusted for
more than it measures is worse than no probe.

## Requirements

### Requirement: A rendered screen is not a single flat colour

Each screen named below SHALL, when instantiated and rendered, produce a grabbed
image containing at least two distinct pixel values. A grabbed image in which
every sampled pixel equals the first sampled pixel SHALL fail the probe.

The screens covered are `FeedScreen`, `DStoaListScreen`, `DJoinScreen` and
`DOnboardingScreen`. Every one of them exists in the view and is already
instantiable under the test runner, so each is reachable by a probe today.

**The assertion is a floor, not a description of the screen.** "At least two
colours" is the weakest claim that distinguishes a screen that painted from one
that did not, and that weakness is deliberate: it is invariant to fonts,
antialiasing, theme token values, platform and Qt version, none of which a probe
should be able to fail on.

Each screen SHALL be probed with whatever data it needs to reach a populated
state, supplied through the same faked module boundary the rest of the suite
uses. A screen probed in a state where it is correct for it to render nothing
would make the probe fail for a reason that is not a defect.

#### Scenario: A populated screen paints

- **WHEN** a covered screen is instantiated, given the data it needs, and
  rendered
- **THEN** the grabbed image contains at least two distinct pixel values
- **AND** the probe passes

#### Scenario: A screen whose children never reach the scene graph

- **WHEN** a component's declared children fail to become scene-graph children,
  so that it reports no children and paints none of them
- **THEN** every sampled pixel of the grabbed image is identical
- **AND** the probe fails

### Requirement: The probe is sampled on a stride

The probe SHALL compare sampled pixels against the first sampled pixel and SHALL
stop at the first pixel that differs. It SHALL NOT require that every pixel in
the grabbed image be examined.

A screen that paints anything at all differs from its first pixel within a short
distance, so an exhaustive scan changes no verdict and costs a scan of the whole
image on every run — for one screen of the size measured here, on the order of
half a million reads.

The stride SHALL be fine enough that the smallest element a covered screen paints
cannot fall between sampled points.

#### Scenario: A screen that paints is detected without a full scan

- **WHEN** a covered screen paints and the probe samples on a stride
- **THEN** the probe reaches a differing pixel and passes
- **AND** it does so without having examined every pixel

### Requirement: The probe is demonstrated to fail, not assumed to

A probe SHALL NOT be accepted on the evidence that it passes. For the probe to
be trusted, a subject that paints nothing SHALL be shown to make it fail, and
that demonstration SHALL be reproducible in this repository rather than cited
from another.

**This is the requirement the rest of the capability rests on.** A probe that
cannot fail passes exactly as quietly as a correct one, which is this project's
most frequently recorded defect: a gate reporting a green it is structurally
unable to withhold.

The demonstration SHALL distinguish the two outcomes on subjects that differ
**only** in whether their children reach the scene graph. A demonstration whose
two subjects differ in any other way — different content, different size,
different colours — leaves two explanations for the difference in verdict and
establishes neither.

#### Scenario: The demonstration separates a healthy subject from a blank one

- **WHEN** two otherwise identical subjects are rendered, one whose declared
  children reach the scene graph and one whose do not
- **THEN** the probe passes on the first and fails on the second

#### Scenario: A probe that cannot fail is rejected

- **WHEN** a probe passes on a subject that paints nothing
- **THEN** that probe does not satisfy this capability, whatever it reports on a
  healthy screen

### Requirement: Each probed subject is rendered without another item behind it

Each probed subject SHALL occupy a region that no other item in the probe's scene
occupies. A probe SHALL NOT place two subjects at overlapping positions.

**A grab is of a region, not of an item in isolation**, so a subject with
another item behind it is grabbed together with whatever that item painted. The
consequence defeats the probe in the direction that matters: a subject painting
nothing, or painting one flat colour, reports as non-flat on the strength of
something else's paint, and the probe passes over exactly the defect it exists to
catch.

This SHALL hold for the probe's own container as well as for sibling subjects: a
subject SHALL NOT be a descendant of an item that paints within the subject's
own region.

#### Scenario: A blank subject over another item is not rescued by it

- **WHEN** a subject that paints nothing is positioned over an item that paints
- **THEN** placing them so they do not overlap is required
- **AND** the probe reports the blank subject as flat

### Requirement: The probe's coverage is bounded to the fact that something painted

The probe SHALL be treated as evidence that a screen painted more than one
colour, and as evidence of nothing else. The following SHALL NOT be inferred
from a passing probe, and a passing probe SHALL NOT be offered as a gate over
any of them:

- **That what was painted is correct.** The probe compares the grab against no
  reference. It is not image diffing, and it holds no golden image.
- **That the layout is right.** Position, size, spacing, alignment and overlap
  are unexamined; a screen with every element stacked in one corner passes.
- **That the colours are right.** The probe reads that two pixels differ, never
  which colours they are. A screen rendering in entirely wrong colours passes.
- **That the text is right, present, or legible.** A screen whose every label is
  empty passes on the strength of any other painted element.
- **That the screen is complete.** One painted element satisfies the probe; the
  absence of every other element does not fail it.

**Stating the boundary is part of the contract rather than a caveat beside it.**
A check whose limits are not written down is read as covering whatever the
reader needed covered, and this probe is unusually easy to over-read: "the screen
renders" is a sentence that sounds like it means the screen is right.

#### Scenario: A screen painting the wrong thing still passes

- **WHEN** a covered screen paints content that is wrong in layout, colour or
  text, but paints more than one colour
- **THEN** the probe passes
- **AND** the probe is not evidence that the screen is correct

#### Scenario: The probe is not offered as a correctness gate

- **WHEN** a change alters what a covered screen paints
- **THEN** a passing probe SHALL NOT be cited as showing the screen still renders
  correctly
