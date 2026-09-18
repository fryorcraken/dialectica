# Design: prove the screens paint something

## Approach

One spec file, `dialectica-ui/tests/tst_render_probe.qml`, with an `Item` root.
Inside it: a grid of non-overlapping cells, one per subject, and a `TestCase`
parked in a cell of its own. Each of the four screens is created into its own
cell, the scene is rendered once, and each cell is grabbed and sampled on a
stride until a pixel differs from the first sampled pixel.

Two extra subjects sit in the grid beside the screens: a healthy component and
one carrying the sibling repo's real defect. They are the demonstration that the
probe can fail, and they ship as part of the spec rather than as a one-off run.

## Decisions

### One spec file, not four

**Chosen:** all four screens, plus the demonstration pair, in a single
`tst_render_probe.qml`.

**Alternative considered:** one spec file per screen —
`tst_render_probe_feed.qml` and three siblings.

**What ruled it out.** The proposal flagged this as a real cost on the strength
of feasibility runs at about 10 seconds per spec file, and asked the
implementation to decide it. Measured on this tree, the finished probe runs in
**about 5.1 seconds** for all eleven tests — so the per-file cost is real, and
four files would plausibly have cost four window setups and four rendering
waits rather than one. The single file is also the cheaper option; it simply is
not cheaper by the margin the proposal estimated.

For context, measured now rather than quoted: the other 18 spec files in this
suite total well under two seconds, with one at 1553ms and nothing else above
175ms. This probe is by a wide margin the most expensive spec in the suite, which
is the cost of `when: windowShown` and `waitForRendering` and is unavoidable for
anything that measures pixels.

What decides it instead is that **the demonstration pair and the screens must be
read together.** The pair is not a separate fixture that happens to live nearby;
it is the evidence that the assertion applied to the four screens is an
assertion that can fail. Split across four files, either the pair is duplicated
into each (four copies of one claim, which drift) or it lives in one file and
the other three assert a predicate whose falsifiability is demonstrated
somewhere the reader is not looking. One file keeps the claim and its
demonstration in one diff.

The grid layout is a consequence of that choice rather than an independent one:
one file means one scene, and one scene means the non-overlap requirement is a
layout property of the grid rather than a rule each file has to re-observe.

**What breaks without the single file:** nothing mechanical. This is a
reviewability decision, and the guard against it silently decaying is that the
demonstration pair's two tests sit in the same `TestCase` as the four screen
tests — moving a screen out of this file leaves it visibly without a
falsifiability demonstration.

### The `TestCase` must not be the file's root item

**Chosen:** an `Item` root, with the `TestCase` and every subject as
non-overlapping siblings inside it.

**Alternative considered:** the suite's ordinary shape — `TestCase` as the file
root, subjects declared inside it or built with `createTemporaryObject(comp,
spec)`. Every other spec in this suite is written that way.

**What ruled it out — measured on this tree, Qt 6.10.3, not inherited.** With
`TestCase` as the file root, `grabImage` returns a uniformly white buffer
whatever was painted:

| shape | subject | result |
|---|---|---|
| `TestCase` as file root | red+blue `Rectangle`, declared | `#ffffff`, 0 differing |
| `TestCase` as file root | same, via `createTemporaryObject` | `#ffffff`, 0 differing |
| `TestCase` as file root | the `TestCase` itself | `#ffffff`, 0 differing |
| `Item` root, subject a sibling | same red+blue `Rectangle` | `#ff0000`, 400 differing |

A probe written in the suite's natural shape therefore **fails on a perfectly
healthy screen**, and the natural repair — loosening the assertion until it goes
green — produces a probe that can never fail. That is the trap this decision
exists to stop, and it is why the awkward-looking root is not a candidate for
"simplification" back to the house style.

One refinement worth recording, because the first probe run here got it wrong
and the wrong version looked like a refutation: the failing ingredient is the
`TestCase` being the **file root**, not merely being the subject's parent. An
`Item`-rooted file whose `TestCase` is a sized child grabs its own child
correctly (`#ff0000`, 4000 differing). So "the subject must not be a child of
the TestCase" is the wrong generalisation; "the TestCase must not be the root"
is the measured one. Both shapes above are safe against it, and the sibling
layout is chosen because it also satisfies the non-overlap requirement.

**What breaks without it:** every one of the four screen assertions goes red on
a healthy tree.

### Every subject gets its own cell, and the cells do not overlap

**Chosen:** a grid of explicitly positioned `Item` cells. Each screen is created
*into* its cell, and the cell — not the screen — is what gets grabbed.

**What ruled out the obvious alternative** (stacking subjects at the origin, or
letting the `TestCase` sit over them): `grabImage(item)` captures the window
region at that item's geometry, so a subject with anything behind it is grabbed
together with whatever that thing painted. Measured here: a blank `Item` over a
painted `Rectangle` reported `#008000` and 900 differing pixels — it passed the
probe on another item's paint, while painting nothing itself. The same blank
`Item` with nothing behind it reported `#ffffff` and 0.

That failure runs in the direction that matters: it rescues exactly the defect
the probe exists to catch. Hence the `TestCase` is parked in its own corner of
the grid too, clear of every subject cell.

**What breaks without it:** the `SHADOWED-TWIN` test — the one that proves the
probe can fail — goes green, because the shadowed twin would be grabbed over
whatever sits beneath it.

### The probe measures the CONTENT AREA, not the card

**This is the decision the probe's value rests on, and the first working version
of this change got it wrong.** It is recorded at length because the wrong version
was green, plausible, and would have shipped a gate that could not fail.

**Chosen:** grab the screen's cell, then sample only a sub-rectangle inset past
the card's padding (`DTheme.cardPaddingX` 34, `cardPaddingY` 28, `hairline` 1;
the probe insets 40/34 to clear the border's antialiased edge).

**What ruled out grabbing the whole card.** `ScreenFrame`'s root is a `Rectangle`
painting `DTheme.paper` with a `DTheme.ink` border, so **the card paints two
colours whether or not any content survives.** Measured: a `ScreenFrame`
containing *nothing at all* — 56px tall, no children — grabbed over a full cell
reports **38,487 differing pixels** and sails through the probe.

So the whole-card probe passed on chrome. Confirmed by mutation, which is the
only thing that settles it: with `ScreenFrame`'s body set `visible: false`, so
every screen keeps its card and loses all its content,

- the **whole-card** version reported 9 passed, 0 failed;
- the **content-area** version fails all four screen tests, each reporting flat
  `#efe9dc` — the paper colour, with nothing on it.

**What breaks without it:** the four screen assertions become unfalsifiable. That
is the exact defect class this change exists to close, reintroduced inside the
probe meant to close it.

### The content area is reached by SAMPLING, never by a nested Item

**Chosen:** `grabImage(cell)` followed by a loop bounded to the content
rectangle.

**Alternative considered, and it is the obvious one:** declare a child `Item`
positioned at the inset inside each cell, and `grabImage` that.

**What ruled it out.** `grabImage` on a childless `Item` returns **that item's own
empty subtree**, not what is painted beneath it. Measured: a content `Item` at
x=40 y=34, 920x289, wholly inside a populated 357px-tall card, grabbed **flat
`#ffffff`** — while the same rectangle sampled out of the cell's grab reported
3,402 differing pixels.

This one is worth the space because of the direction it fails in. It makes the
four screen tests go **red on a healthy tree**, and the natural repair is to
conclude the inset is wrong and drop it — which lands straight back on the
whole-card false green above. Two wrong turns that lead to each other.

**What breaks without it:** every screen test fails on a healthy tree, and the
repair most likely to be reached for is the one that silently disables the probe.

### Each sampled rectangle is sized from its own screen

A rectangle taller than the screen reads past it into whatever else the cell
holds. Measured in an earlier shape: a populated feed and an empty frame both
reported **exactly 11,926** differing pixels — the identical number being the
tell that both were reading the same out-of-card paint.

A floor (`minSide`) rejects a degenerate rectangle rather than reporting a
verdict about it, so "flat because there was nothing to look at" can never be
recorded as "flat because nothing painted". An empty card's content rectangle
comes out 920x**-12**, and the probe refuses it.

### No explicit `height` on any screen

`ScreenFrame.qml`'s contract forbids it — the card sizes from its content, and a
`Layout.fillHeight` child inside a content-sized card measures 0. Each screen is
given `width` only. This is why the cells are sized generously (620px) rather
than to a measured screen height: the cell is the fixed frame, the screen sizes
itself inside it.

### The demonstration pair is two files differing by one line

**Chosen:** `RenderProbeHealthy.qml` and `RenderProbeShadowed.qml` in the tests
directory, identical except that the second declares `property var data`.

The spec requires the two subjects to differ **only** in whether their children
reach the scene graph, because any other difference leaves two explanations for
the difference in verdict. Two files with one line between them make that
checkable by diffing them, which is stronger than a comment claiming it.

`data` is `Item`'s default property. Declaring a property of that name shadows
it, so declared children are assigned to the new `var` property instead of
becoming scene-graph children. This is the sibling repo's real `CommitView.qml`
bug, reproduced here rather than cited: measured on this tree, `children.length`
is 2 on the healthy twin and **0** on the shadowed one, and the grabs are
`#1e3a5f` non-flat against `#ffffff` flat.

**Alternative considered:** an inline `Component` in the spec file with the
shadowing property, avoiding two extra files. Rejected because the "differ only
in one line" property is then asserted in prose rather than visible in a diff,
and because the two components would have to be written out twice in full inside
one file — which is where a stray difference creeps in unnoticed.

**The one-line property is checked mechanically, by a shell gate rather than by
the QML spec.** `check_probe_twins.sh` compares the two files' code lines
(comments and indentation excluded — each twin rightly explains its own half) and
requires the shadowed one to add exactly `property var data`.

It is a shell gate because QML blocks `XMLHttpRequest` against a local file
unless `QML_XHR_ALLOW_FILE_READ=1` is set, and an environment-variable prefix is
a shape this repo's permission checker cannot statically analyse. The property is
static anyway, so a shell gate is the right layer — it is where this repo's other
structural checks (`check_qml_names.py`, `check_qml_members.sh`) already live.

`tst_check_probe_twins.sh` pins both directions over 10 cases, including the two
that matter: a twin whose colours *also* differ must be rejected (that is the
second explanation the spec forbids), and two empty files must be rejected rather
than reported clean — a comparison over an empty corpus passes while measuring
nothing.

**Why the twins are plain `Rectangle`s rather than a real screen.** A screen
carrying the `data` shadow would be a modified copy of production code, which is
a second difference from its healthy twin and defeats the point. The twins test
the *probe*, not the screens.

### Stride, and why 4

**Chosen:** a stride of 4 pixels in both axes, compared against the first sampled
pixel, stopping at the first difference.

The spec requires the stride to be fine enough that the smallest element a
covered screen paints cannot fall between sampled points. The smallest painted
elements in this view are `DTheme`-scale borders and separators at 1px, but they
run the width or height of a card, so a 4px grid crosses them many times over. No
covered screen paints an isolated feature smaller than 4x4.

Measured cost: the four screens each reach a differing pixel after **252 sampled
reads** out of a 1000x620 cell. An exhaustive scan of one cell is 620,000 reads
and returns the same verdict.

The flat case is the expensive one and cannot exit early — the shadowed twin
samples its whole 120x80 cell at 600 reads to conclude FLAT. That asymmetry is
correct: a passing probe is cheap, and a failing one is about to fail the build
anyway.

## What this probe does not cover

Restated here from the spec because it is the part most likely to be over-read
when someone cites a green run: a passing probe is evidence that more than one
colour was painted, and evidence of nothing else. Not that the layout, colours,
text or completeness are right. A screen rendering entirely wrong content in
entirely wrong colours passes.

**Two specific blindnesses worth naming.**

The four screens all sample from the same paper colour (`#efe9dc`) because they
share `DTheme`'s card tokens, so the probe would not notice one screen rendering
another screen's content. That the readings are genuine per-screen paint rather
than a shared background is pinned by `test_an_empty_cell_is_flat`: a cell in the
same grid with nothing in it grabs `#ffffff` flat.

And the probe sees only the card's **content area**. A defect confined to the
card chrome — a border that stopped painting, the wrong paper colour — is outside
the sampled rectangle by construction. That is the deliberate trade for not
passing on chrome, and it is the right way round: chrome is one shared component
with its own geometry spec, while content is four screens' worth of children that
can silently fail to reach the scene graph.

## Migrated from PLAN.md §10

The anti-false-green reasoning this change acts on moved here from `docs/PLAN.md`
§10, per the flow's PLAN-sheds rule.

**The principle, in full, as PLAN.md carried it.** Radicle's CI is built around
the observation that **a green gate which cannot see the thing it claims to check
is worse than no gate.** Copy the habits, not just the jobs: assert test *counts*
against spec counts, assert packaged manifests kept their entry points, assert
dev and portable variants were not transposed, and verify a new test fails before
it passes.

PLAN.md §10 now points here rather than keeping a second copy, so this paragraph
is the only one.

This probe is that principle applied to a specific blindness. The `qml` job's
existing gates — `check_bindings` for undefined bindings, `check_qml_names.py`
for host name collisions — cannot see a component whose children never enter the
scene graph. Nothing warns, no binding is undefined, no name collides, and the
symptom is a blank view, which CLAUDE.md records as indistinguishable from a
plugin that failed to load.

**The principle's last clause — verify a new test fails before it passes — is
what saved this change from shipping the very defect it closes.** The first
working version grabbed the whole card, was green across every test, and was
wrong: `ScreenFrame` paints paper and a border whatever happens to its content,
so an empty card reported 38,487 differing pixels. Only the mutation run found
it. With `ScreenFrame`'s body hidden, so every screen keeps its card and loses
all content:

| version | mutated tree |
|---|---|
| whole-card grab | **9 passed, 0 failed** |
| content-area sampling | **4 screen tests fail**, each flat `#efe9dc` |

A gate is not finished when it is green. It is finished when it has been seen to
go red for the reason it exists.
