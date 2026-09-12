# The mark: what it distinguishes, and what it cannot

This note records the reasoning behind `dialectica-ui/src/qml/Identicon.qml`.
It exists because the *numbers* are the whole argument, and a reader who takes
the mark for an identifier will be wrong in a way the code cannot tell them.

## The claim that must never be made

**The mark is not unique and uniqueness is not achievable.** Addresses are 32
bytes; any fixed-size visual encoding collides by pigeonhole, and the visual
encoding here is very much smaller than 32 bytes. Two identities *will*
eventually share a mark, and an attacker who can regenerate addresses freely
can search for one that resembles a target's.

This is the same argument UI-BRIEF obligation 6 makes about generated names,
and it applies with equal force here. **The address is the identity. The mark
is a recognition aid.** Wherever recognition carries weight — above all
wherever a moderator is named — the address must be on screen, not one click
away. A better mark does not relax that requirement; it is a second forgeable
channel, and a second forgeable channel is still forgeable.

What the mark buys is real and worth stating precisely: it raises the cost of
*casual* impersonation, and it puts bytes of the address on screen that nothing
else shows.

## Three numbers, and only one of them is the collision space

These get conflated, so they are separated here deliberately.

| | before | after |
|---|---|---|
| **bytes consumed** | 6 (bytes 0..5) | 8 (bytes 12..19) |
| **perceptually distinct marks** | ~1,700 | ~8,900 |
| **combined with the generated name** | ~2^36 | ~2^38 |

> **These numbers were wrong in an earlier revision of this note and the
> corrections are recorded in place rather than quietly replaced.** The claimed
> figures were 20 bytes, ~66,000 marks and a 0.080 colour floor. All three were
> wrong: the code reads 8 bytes not 20, the outline factor was counted at full
> cardinality after being described as barely legible, and the colour floor was
> hand-computed with a gamma error that hid a pair measuring **0.006** under
> simulated protanopia — a pair that would have shipped as one colour for ~8% of
> male readers. Every figure below is now computed in the target engine by
> `dialectica-ui/tests/tst_identicon.qml` and `tmp/render/tst_palette.qml`
> rather than by hand.

**Only the middle row is what a human collides on.** Reading more bytes into
unchanged dimensions yields exactly as many distinguishable marks as reading
fewer — the byte count is the input budget, the perceptual space is the output,
and the birthday arithmetic runs on the output. The byte widening below is
necessary and not sufficient; the dimension work is what moved the number.

## Counting the perceptual space, before

The original mark read six bytes (2^48 states) and rendered them through these
dimensions. The count is *perceptual*: where two parameter values produce marks
a person cannot separate at the rendered size, they are counted once. The
governing size is `Theme.markInFeed` = **19px**, because that is where
recognition actually happens; `markInList` = 40px is the generous case.

**Contour.** Byte 0 drove three things at once — `_sides()` (period 6),
`_cutCorner()` (period 7) and `_curvedForm()` (period 5) — so two of the three
were always dead on any given branch. On the person branch only
`_curvedForm()`'s 5 values were reachable.

Of those 5, `case 0` (circle) and `case 4` (capsule) render *identically*: the
mark is always square, so a `_roundRect` with every corner radius at `h/2` on a
square box is a circle drawn with Bezier arcs instead of `ellipse()`. At 19px
the difference is well under a pixel. **5 parameter values → 4 perceptual.**

On the Stoa branch, `_sides()` gave 3..8 plus the cut-corner form. A regular
n-gon's silhouette is governed by `cos(pi/n)`: n=6 → 0.866, n=7 → 0.901,
n=8 → 0.924. At 19px the radius is ~7.5px, so 7-vs-8 differs by
0.023 x 7.5 = **0.17px** of silhouette — a sixth of a pixel. 6-vs-7 is 0.26px.
**{6,7,8} collapse to one class. 7 parameter values → 5 perceptual.**

**Outline ink.** 3 values, drawn as a 2px ring at feed size. All three palette
inks are dark on light paper; the ring's *hue* is legible at 40px and barely at
19px. Counted generously as 3.

**Fill pair.** `_fillInkA` and `_fillInkB`, ordered, always different: 3 x 2 = 6
ordered pairs. But the weave was a 50% duty cycle — `fillRect(..., pitch)` every
`pitch * 2` — so A and B covered equal area, and swapping them produced the same
two-colour field shifted by half a period. A phase shift of an *unanchored*
periodic field is invisible. **The order did not survive rendering**, despite
`SPEC.md` claiming "Order matters". 6 ordered pairs → **3 unordered**.

**Angle.** 24 steps of 15 degrees. Parallel stripes have period 180 degrees,
not 360: a stripe lattice at 15 and at 195 degrees is the same lattice up to a
phase shift, and the phase is invisible for the reason just given.
**24 → 12.**

**Pitch.** 2..7px, 6 values. At 19px the interior is ~15px, giving
`15 / (2 * pitch)` stripe pairs: pitch 2 → 3.75 pairs, 3 → 2.5, 4 → 1.875,
5 → 1.5, 6 → 1.25, 7 → 1.07. At 6 and 7 the mark reads as "ground with one bar
across it" and the two differ by a pixel of bar width; 4 and 5 differ by about
as little. **{4,5} and {6,7} each collapse. 6 → 4.**

### The product, before

Person (curved): 4 x 3 x 3 x 12 x 4
= 4 x 3 = 12; 12 x 3 = 36; 36 x 12 = 432; 432 x 4 = **1,728**

Stoa (angular): 5 x 3 x 3 x 12 x 4
= 5 x 3 = 15; 15 x 3 = 45; 45 x 12 = 540; 540 x 4 = **2,160**

A forum participant is a person, so **1,728** is the number that governed
collisions between two people in a feed.

log2(1728): 2^10 = 1024, and 1728 / 1024 = 1.6875, log2(1.6875) ~ 0.755, so
log2(1728) = **10.75 bits**. The mark read 48 bits and rendered 10.75 of them:
**37 bits were discarded.**

## What changed, and why each addition is perceptually independent

The rule applied throughout: **a dimension earns its place only if a reader can
separate its values at 19px.** Two additions were tried and rejected on that
test; they are recorded below rather than quietly dropped.

### 1. Six inks, hand-picked and measured (3 → 6)

The largest win, and the right place to spend the budget: colour is the dimension
the eye separates fastest and the one that survives shrinking, where fine
geometry is already mud at 40px.

The palette is **chosen, not computed**. Slicing a hue wheel into N steps puts
adjacent steps inside a just-noticeable difference — which would manufacture
exactly the indistinguishable parameter states this change exists to remove.

#### The palette is a lightness ladder, and that shape is forced

This is the single most important thing to understand about it, and an earlier
revision of this note got it wrong.

Under deuteranopia and protanopia the **red/green axis collapses**. What
dichromats retain is **lightness** and the **blue/yellow axis**. So two inks
separated mainly by hue are *one colour* for roughly 8% of male readers, however
far apart they measure in normal vision. The only reliable way to separate a
categorical palette for those readers is to space it in **lightness**.

Ordering the palette as a monotonic L ladder is therefore not an aesthetic
choice; it is what makes the set work at all:

| role | hex | L | C | hue |
|---|---|---|---|---|
| `markInk` | `#1c1a16` | 0.219 | 0.008 | — |
| `markViolet` | `#4a2a86` | 0.380 | 0.146 | 294 |
| `markRust` | `#8a3a1c` | 0.452 | 0.117 | 40 |
| `markGreen` | `#2f7f5c` | 0.537 | 0.096 | 161 |
| `markLime` | `#8fb520` | 0.720 | 0.169 | 124 |
| `markSky` | `#8ecbe8` | 0.811 | 0.075 | 229 |

**Measured floor: 0.205 OKLab in normal vision, 0.109 under simulated
dichromacy**, both at `markRust`/`markGreen`. **No pair falls under 0.10 in
either condition.**

#### Why six and not eight

**Because eight could not hold the floor.** Every candidate eight-ink set left at
least one pair under 0.10 simulated, and the mechanism is arithmetic rather than
bad luck: after hue collapse, separation is dominated by ΔL, so eight rungs need
ΔL ≈ 0.10 between neighbours — about 0.70 of lightness range. Pushing the top
rung that high puts it near white, where it stops reading as an *ink* on
`#efe9dc` paper. Six rungs fit; eight do not.

Candidates tried and rejected, with their worst simulated pair:

| set | worst simulated pair | verdict |
|---|---|---|
| the shipped eight | **0.006** Moss/Rust (protanopia) | two colours for 92% of readers, one for the rest |
| 8, saturated ladder | 0.047 Indigo/Plum | under floor |
| 8, even L spacing | 0.087 Stone/Ochre | under floor, and Ochre near white |
| 7 | 0.098 Plum/Rust | at the line, not over it |
| **6** | **0.109** Rust/Green | **ships** |

**An honest six beats a claimed eight where two are the same colour.**

#### How the earlier figures went wrong

Worth recording, because the failure was methodological rather than arithmetical.

The first pass hand-computed the dichromat simulation and **fed gamma-encoded
sRGB into the Viénot LMS matrix, which is defined against linear light.** That
single step error shifted every simulated value, and it did so *plausibly* — the
numbers looked reasonable, so nothing flagged them. It hid a pair measuring
**0.006**: `markMoss`/`markRust` under protanopia, effectively the same colour,
which the note claimed measured 0.100 and declared safe.

Two further hand-computed claims were also wrong and are withdrawn:

- `markOchre` was recorded at L 0.653 / C 0.154 / hue 39. Hue 39 would have sat
  8° from Rust, which contradicts the same paragraph's claim that every pair
  cleared 0.10 — an internal inconsistency that should have been caught.
- `markOlive` was cut claiming it measured 0.062 against Moss. **It measured
  0.122** — the cut was not numerically justified. It is still out, on the design
  judgement that two greens at similar lightness read as one green, but that is a
  judgement and is now stated as one rather than dressed as a measurement.

The lesson is recorded in the instrument rather than in prose: the palette is now
measured by `tmp/render/tst_palette.qml`, which **self-tests against
`#808080` → L 0.5998 before reporting anything.** A pipeline that cannot
reproduce a known value is not trusted to produce unknown ones.

**An honest 6 beats a claimed 8 where two are the same colour**, so the number
reported is 6.

#### All three inks are now distinct by construction

The ink pair is **genuinely ordered**, which it was not before — see the duty
cycle below.

A defect found in review and fixed here: the outline was drawn from an
independent index, with nothing preventing it equalling ink A. **One mark in six
therefore had ring and ground the same colour and no visible contour at all** —
the outline silently vanished, on a component whose comment promises "the outline
is always drawn, so a mark never bleeds into the row behind it".

All three indices are now derived by **offsets from one another** rather than
independent draws, so distinctness holds by construction rather than by a guard
that has to be right at every call site. `tst_identicon.qml` asserts it across a
sweep of byte values, and that test was watched failing against the old
independent draw before the fix went in.

With six inks: A has 6 choices, B has 5 (never A), the outline has 4 (never A or
B). That is the ink contribution, before the deflation the next section applies to
the outline.

### 2. Eleven pooled forms, no angular/curved split (4 or 5 → 10)

The `isPerson` split meant each address reached only *half* the vocabulary, and
it was restating what position already said — a Stoa's mark renders in the Stoa
header, a person's beside their name in a post. Pooling gives every address the
whole family, and it removes a real weakness: an attacker impersonating a person
previously only had to search the curved subspace.

**The Stoa/person distinction is now carried by POSITION ALONE.** A future
change that renders a mark somewhere the surrounding context does not
disambiguate must label it; that is the one thing the split was providing and it
should not be lost silently.

Eleven forms are in the array. **Ten survive the 19px test**, and that is the
number used in the arithmetic:

rounded square, leaf, teardrop, triangle, inverted triangle, diamond, pentagon,
hexagon, four-point star, six-point star, cut-corner.

Every one separates on a feature that survives shrinking: flat ends, vertex
orientation, concave points, or one cut corner.

- **Pentagon and hexagon are one class at 19px** by the `cos(pi/n)` argument
  above (0.809 vs 0.866, ~0.4px of silhouette on a 7.5px radius). Counted as
  one. They are kept in the array because they *do* separate at 40px, but the
  honest feed-size count is what the collision arithmetic uses. **This is the
  only merge in the list.**
- **THE CIRCLE WAS REMOVED**, and it is worth recording why rather than just
  that it happened. A circle is *the absence of corners* — which is precisely
  what every polygon degrades toward as it shrinks. So it was at once the least
  informative form (no silhouette to name) and the form its neighbours collapse
  into at feed size: it cost its neighbours distinctness, not only its own.
  The bundle's "capsule" was the same shape in disguise — a `_roundRect` with
  every corner at `h/2` on a square box **is** a circle — so it is absent for
  the same reason rather than as a separate judgement.
- Side-counts 7 and 8 are **deliberately absent**. They would have inflated the
  array without widening what a reader can see, which is the failure this
  change exists to fix.
- **Stars were added instead**, because a star is never mistaken for a polygon
  at any size that draws at all — it separates on a feature (concave vertices)
  that survives shrinking where vertex *count* does not.
- Triangle and inverted triangle are distinct: orientation of a 3-gon is the
  most legible orientation cue there is.
- Leaf and teardrop were **deepened** (corner radii 0.16 and 0.10 rather than
  0.20 and 0.18) once the circle was gone. They had been sitting in its
  neighbourhood; with nothing round to compete against, a sharper contrast
  between their round and square corners is free.

So: **10 perceptual forms**, up from 4 (person) or 5 (Stoa).

Note the honest bookkeeping: removing the circle did **not** lower the surviving
count, because the circle was one of the forms that merged. The cut removed a
dead entry from the array, not a live one from the count.

### 3. Duty cycle (new, 3 values)

What fraction of each weave period ink B covers: 0.30, 0.45, 0.62.

This is what makes the ink pair **genuinely ordered** — at the old 50% duty
cycle, swapping A and B produced the same field half a period along, which is
invisible. At 30% the mark is mostly A with thin B lines; at 62% it is mostly B
with thin A lines. Those read differently *independently of which two inks they
are*, so it is a dimension in its own right as well as the thing that recovers
the ordering.

### 4. Weave kind (new, 3 values)

Parallel bands / crossed lattice / **dot lattice**. This changes the
**topology** of the fill rather than the orientation or spacing of one band
family, which is why it is perceptually independent of both angle and pitch.

The dot lattice is a staggered grid — every other row offset by half a period,
so it reads as a texture rather than as two crossed band families. Its dots are
**square, not round**, deliberately: at 19px a radius-1 arc rasterises to an
ambiguous 2x2 smudge, whereas a `fillRect` is exactly two pixels wide on every
renderer, which is also what keeps it pixel-identical across peers.

#### A concentric-ring variant was tried here and removed

It is worth recording because it failed in two ways that *looked* like two
separate bugs, and fixing only the visible one would have left the other.

Ring radii step by `period`, which is 4 to 12px. On a 19px mark, **at most one
band is ever visible** — so whether the outermost disc landed on ink A or ink B
decided whether the mark read as a **bullseye** (a different visual idiom from
the rest of the family, looking like it came from another design) or as a **flat
disc carrying no pattern at all**. Both outcomes are a mark that has stopped
distinguishing anything, and which one you got depended on the parity of a
radius count.

**A pattern whose legibility depends on the parity of a radius count is not a
dimension.** The replacement covers the whole face at constant density, so it
cannot degenerate that way: every cell of the lattice carries the same amount of
ink B regardless of where the mark's centre falls.

### 5. Angle and pitch, corrected downward

- **Angle: 12 steps of 15 degrees**, not 24. The original 24 were 12 in
  disguise, for the 180-degree-period reason above. Writing the honest number
  into the code means nobody later "improves" it back to 24 and believes they
  doubled something.
- **Pitch: 4 values** (2, 3, 4, 6) rather than 6. The collapsed pairs are gone
  rather than being retained as parameters nobody can see.

### Rejected, and why

- **More polygon side-counts (7, 8, 9).** Sub-pixel silhouette differences at
  feed size. This was the original design's main way of spending shape entropy
  and most of it was never visible.
- **A per-mark rotation of the whole contour.** At 19px a rotated hexagon is a
  hexagon; only the 3-gon and the cut-corner form have legible orientation, and
  those are already covered as separate forms. Adding a rotation dimension
  would have added parameter states with no perceptual counterpart — the exact
  defect being repaired.

## Counting the perceptual space, after

| dimension | parameter values | counted at 19px | why deflated |
|---|---|---|---|
| form | 11 | **10** | pentagon/hexagon merge |
| ink A | 6 | **6** | the ground fills the face; fully legible |
| ink B | 5 | **5** | never A; covers 30–62% of the face |
| outline ink | 4 | **2** | see below — a 2px ring is barely legible at 19px |
| weave kind | 3 | **3** | topologically different fills |
| duty | 3 | **3** | thin-B vs thick-B is a clear read |
| angle | 12 | ~2 | see below |
| pitch | 4 | ~2 | see below |

**The outline is deflated, and this corrects an inconsistency review caught.**
The "before" count in this note rates the outline ring "legible at 40px and
barely at 19px" and generously counts it as 3. An earlier revision of the "after"
count then took it at **full cardinality 8** — at the same governing 19px. That
asymmetry flattered the improvement: the old space was deflated across five
dimensions and the new one across none. A dimension described as barely legible
cannot contribute its full parameter count. At 19px a 2px ring on the lightness
ladder reads as roughly *dark ring* versus *light ring*, so it is counted as **2**.

**Angle and pitch are deflated hard, for the same reason.** At 19px the interior
is about 15px. Twelve 15-degree steps do not give twelve readable orientations on
a field that narrow — near 0 and 90 degrees they read as "horizontal" or
"vertical" with a pixel of stair-stepping — and pitch 4 versus 6 is about one
pixel of bar width. Together they contribute perhaps a factor of **4** at feed
size, not 48. They earn their place at 40px, where they separate properly.

All three weave kinds are now rotatable periodic fields, so angle and pitch apply
uniformly and the product is a flat multiplication. (That was not true while the
rings variant existed, which needed per-kind arithmetic — another small argument
for having removed it.)

**Raw parameter product:** form x A x B x outline x weave x duty x angle x pitch
= 11 x 6 x 5 x 4 x 3 x 3 x 12 x 4

11 x 6 = 66
66 x 5 = 330
330 x 4 = 1,320
1,320 x 3 = 3,960
3,960 x 3 = 11,880
11,880 x 12 = 142,560
142,560 x 4 = **570,240**

**That number must not be quoted as the perceptual space.** It is the parameter
count, and the whole point of this note is that the two differ by nearly two
orders of magnitude.

**The honest headline number.** Take every dimension at its deflated 19px value,
with no dimension allowed its full parameter count:

form (10) x A (6) x B (5) x outline (2) x weave (3) x duty (3)
= 10 x 6 = 60; 60 x 5 = 300; 300 x 2 = 600; 600 x 3 = 1,800;
1,800 x 3 = **5,400**

and then the angle/pitch texture contributes a factor that is real but
size-dependent — about **1.64x** at feed size, substantially more at 40px:

**~8,900 perceptually distinct marks at feed size.**

log2(8,900): 2^13 = 8,192, and 8,900 / 8,192 = 1.086, log2(1.086) ≈ 0.12, so this
is **~13.1 bits**, against 10.75 before.

**A 5.2x improvement.** That is a far smaller claim than the 38x an earlier
revision of this note made, and the difference is entirely the outline deflation
plus the honest six-ink palette. It is worth stating plainly: **the headline gain
is modest.** What the change mostly bought was not raw space but *correctness* —
removing dimensions that did not exist (the invisible stripe order, the 24 angles
that were 12), fixing a palette that had one pair indistinguishable to 8% of
readers, and closing an outline that vanished on one mark in six.

At 40px the number is several times larger, because angle, pitch, outline hue and
the pentagon/hexagon distinction all come back. **The feed number is the one that
governs collisions**, so it is the one quoted.

## Birthday arithmetic

Using `1 - exp(-k(k-1) / 2S)`.

### Before, S = 1,728

**k = 100**: k(k-1)/2 = 100 x 99 / 2 = 4,950.
4,950 / 1,728 = 2.865. `1 - e^-2.865`.
e^-2.865: e^-3 = 0.0498, e^0.135 = 1.1445, so e^-2.865 = 0.0498 x 1.1445 = 0.0570.
**P = 0.943 — 94%.** In a Stoa of 100, a mark collision is near-certain.

**k = 1,000**: 1000 x 999 / 2 = 499,500. / 1,728 = 289.1.
e^-289 is astronomically small. **P > 0.99999...** — effectively 1.

**k = 5,000**: **P = 1** for all practical purposes.

The old mark could not distinguish 100 people.

### After, S = 8,900

**k = 100**: 4,950 / 8,900 = 0.5562. `1 - e^-0.5562`.
e^-0.5562: e^-0.5 = 0.6065, e^-0.0562 = 0.9454, product = 0.5734.
**P = 0.427 — about 43%.**

**k = 1,000**: 499,500 / 8,900 = 56.1. e^-56.1 is negligible.
**P ~ 1.**

**k = 5,000**: 12,497,500 / 8,900 = 1,404. **P = 1.**

### What this means honestly

**The mark alone does not solve collisions at any interesting scale, and the
earlier revision of this note overstated how close it came.** At 100 identities a
mark collision is a coin flip; by 1,000 it is a certainty. The improvement over
1,728 is real (94% → 43% at k=100) but it does not change the character of the
problem.

The name scheme yields 2^25 (~33 million) outcomes; UI-BRIEF records ~3% chance
of a name collision at 1,000 and better than even at 5,000.

The mark at 8,900 is **far worse than the name in raw space** — 2^13 against 2^25
— and that is the honest statement. **What the mark buys is not a larger space
than the name; it is an independent one.** Given disjoint byte ranges the two
multiply: 2^25 x 8,900 ≈ 2.99 x 10^11 ≈ **2^38**, and the pair collides only when
both collide.

At k = 1,000 against S = 2.99 x 10^11:
499,500 / 2.99e11 = 1.67e-6. **P ~ 1 in 600,000.**

At k = 5,000: 12,497,500 / 2.99e11 = 4.18e-5. **P ~ 1 in 24,000.**

**That is the number that matters**, and it is the argument for the mark: not
that the mark is a good identifier, but that a name-plus-mark bundle collides
far less often than a name alone, *provided the two are independent*.

## The byte layout, and why it is chosen rather than convenient

`AddressLabel.qml` abbreviates to head 8, middle 8, tail 6 of the **hex body**,
with the `k:`/`stoa:` prefix stripped *before* slicing — so the offsets are
prefix-independent. For a 32-byte address (64 hex characters):

- head: body chars 0..7 → **bytes 0..3**
- middle: `start = floor((64 - 8) / 2) = 28`, chars 28..35 → **bytes 14..17**
- tail: chars 58..63 → **bytes 29..31**

The abbreviation therefore shows **11 of 32 bytes**, and **21 bytes are
invisible** at feed density: bytes 4..13 and 18..28.

The mark reads **bytes 12..19** — eight bytes, one per dimension. Bytes 0..11 are
reserved for the generated-name scheme.

**Name and mark are independent**, which is the property worth having. Grinding
for a target's *name* searches bytes 0..11 and yields a random mark; grinding for
the *mark* searches 12..19 and yields a random name. The costs **multiply rather
than add**. With shared bytes, a near-miss on one correlates with a near-miss on
the other and the combined difficulty collapses toward the harder of the two.

**Why eight bytes and not more.** The mark's *output* is about 13 bits of
perceptually distinct results. Eight bytes of input is 64 bits, already exceeding
what the rendering can express by a factor of 2^51. Reading twenty bytes instead
would change nothing a reader could see. This is the note's own central point
turned on itself: **the input was never the binding constraint.** An earlier
revision of this document claimed the mark read bytes 12..31 — twenty bytes — and
that claim was simply false about the code, which has only ever read eight.
Widening the read to look thorough would have been the exact confusion this
document argues against.

### Two flaws this does not fix

**The mark does not cover what the abbreviation hides, as much as intended.** The
abbreviation shows bytes 0..3, 14..17 and 29..31. Intersecting the mark's actual
reads with the *hidden* set gives only **{12, 13, 18, 19} — 4 of the 21 hidden
bytes**, not the 13 an earlier revision claimed. Worse, **bytes 14..17 are half of
what the mark reads and are already on screen in the middle group**, so half the
mark's input sits on ground the abbreviation already covers. That is a weaker
version of the criticism this design makes of the bundle's original mark — reduced
from four-of-six to four-of-eight, not eliminated.

Moving the read to a fully hidden window would fix it, at the cost of the name
scheme's reserved range or of a coordination change; it is recorded here as a
known imperfection rather than silently improved on paper.

**The visible bundle is still not unique, and it is still grindable.** The
abbreviation shows the *same byte positions for every address*, so an attacker
grinding for a lookalike grinds only those fixed positions and gets the hidden
ones free.

**2^38 is grindable** with unlimited address regeneration. This defeats casual
impersonation, not a motivated attacker. The address remains the identity.

## Which dimensions resist grinding

The brief asks which dimensions have the property that "close in parameter
space" does *not* mean "looks close". This matters because a design where small
parameter changes are visually small is one an attacker can hill-climb toward a
target.

**Resistant — a near-miss looks obviously wrong:**

- **Form.** The forms are indexed, not ordered: index 4 (triangle) and index 5
  (inverted triangle) are adjacent in the array and maximally different on
  screen. There is no gradient to climb. An attacker either lands the exact
  index or produces a visibly different shape.
- **Weave kind.** Bands, crossed lattice and dot lattice are topologically
  different fills. There is no interpolation between them; missing by one index
  is a completely different texture.
- **Ink identity.** The palette is discrete and hand-separated with a measured
  floor. Landing an adjacent index gives a colour that is, by construction, at
  least **0.205 OKLab away in normal vision and 0.109 under dichromacy** — which
  is the *point* of the floor. This is the strongest dimension against grinding
  precisely because it is the one curated for separation, and the floor is now
  wide because the palette was cut to six to achieve it.

**Weak — a near-miss looks close:**

- **Angle.** 15 degrees off is visibly similar. An attacker who gets the angle
  wrong by one step has produced something a reader would plausibly accept.
- **Pitch.** Adjacent values (3 vs 4) are similar textures.
- **Duty cycle.** 0.45 vs 0.62 is a difference of degree, not of kind.

So **three of the seven dimensions are ordinal and hill-climbable, and four are
categorical and are not.** The categorical ones carry
10 x 6 x 5 x 3 = 900 of the space at feed size; that is the portion an attacker
must hit exactly rather than approach. This is why the budget went into colour and
form rather than into more angle steps.

## Why contour-and-weave rather than a symmetric cell grid

The classic alternative is a GitHub-style identicon: a 5x5 grid mirrored
horizontally, cells on/off from hash bits, one colour. It is trivially
deterministic, has a large raw space (2^15 patterns x colour), and is proven
legible small.

**Contour-and-weave is the right choice here, for three reasons.**

1. **Its space is better-shaped, not just large.** A mirrored 5x5 grid has
   2^15 = 32,768 cell patterns, but its *perceptual* space is far smaller than
   that number suggests: most random patterns are an undifferentiated speckle,
   and two patterns differing in one or two cells are genuinely hard to tell
   apart at 19px. The grid's dimensions are also all of one kind — every bit is
   an equally-weighted cell — so the whole space is hill-climbable, which is
   the grinding property we most want to avoid. Contour-and-weave separates
   into categorical dimensions (form, ink, weave kind) that an attacker must
   hit exactly.
2. **It degrades better.** A 5x5 grid at 19px gives cells under 4px, and below
   that it becomes noise with no graceful fallback — there is no "simpler
   version" of a speckle. The mark already has a documented degradation
   ladder: the weave drops below `markMinWeave`, leaving a solid two-ink
   silhouette that still carries form and two ink choices, and below
   `markMinDraw` the caller prints the address. **A mark that degrades to a
   recognisable silhouette is worth more at feed density than one that
   degrades to mud.**
3. **It is in the design's register.** The bundle is flat, 1-2px borders, no
   radii, no shadows, an apparatus-column typographic system. A speckled grid
   is from a different visual vocabulary, and the mark sits beside serif body
   text on warm paper on every row of the feed.

The grid's real advantage — proven legibility at very small sizes — is
answered by the degradation ladder rather than by adopting the grid.

## Determinism

The contract is **pattern-identity, not pixel-identity**, and the distinction
matters enough to state precisely. An earlier revision of this note claimed the
stronger thing.

**What is guaranteed:** the same address selects the same form, the same three
inks, the same weave kind, angle, pitch and duty on every peer, forever. What
secures it:

- Every dimension is an **integer** index derived from `parseInt(hex, 16)` and
  a modulus. No floating-point value is compared for equality, accumulated
  across iterations, or used as a loop bound derived from an earlier float.
- **No randomness, no clock, no locale, no system font, no external asset.**
  The QML engine is sandboxed with deny-all network access and no filesystem
  access outside the plugin directory, so it could not fetch anything even if
  the design wanted to.
- **Malformed input is handled at the boundary.** Peer-supplied strings reach
  this component. The hex body is stripped of non-hex characters and padded to
  64 characters, so a short, empty or hostile address renders something stable
  rather than throwing or producing `NaN` indices. `tst_identicon.qml` covers
  the empty, prefix-only, non-hex and truncated cases.

**What is NOT guaranteed, and why it does not matter:** byte-identical
rasterisation. Three reasons, and the first two were mis-analysed before:

- **Half-integer coordinates, not floating-point ulps, decide edge pixels.** At
  size 19 with stroke 2 the polygon radius is 7.5, so axis vertices land on exact
  half-integers where a fill rule picks the boundary pixel. An earlier revision
  argued about last-ulp `sin`/`cos` differences, which addresses the wrong case
  entirely — the ambiguity is geometric, not arithmetic.
- **Every weave rotates by a non-multiple of 90 degrees** for 11 of the 12
  angles, so edge coverage of any filled shape is an antialiasing detail.
- **Device pixel ratio changes the sample grid.**

None of these can change *which* shape or *which* inks are drawn. Recognition
depends on the pattern, so pattern-identity is the contract that matters and the
only one honestly available.

## Where the palette lives

The six inks are named roles in `Theme.qml`, in their own block separate from the
three interface inks. They carry the `mark` prefix rather than extending the
`accent` series because the mark's palette has a different job, and a change to
`accent2` must not silently change what every identity looks like.

**They are frozen wire-visible constants, not theme tokens**, and this is the one
thing about them most likely to be got wrong later. The mark binds to
`Theme.mark*`, so editing a value there changes every identity's appearance — and
two peers on different app versions would then render *different marks for the
same address*, which is precisely the failure the determinism contract exists to
prevent. That is far more likely in practice than any renderer difference. An
earlier revision of this note presented the theme binding as a benefit ("the look
can be iterated without touching this component"), which is true of interface
colours and false of these. **Add an ink if the mark needs one; do not retune an
existing one.** The obligation is recorded beside the values in `Theme.qml`.

### Removing isPerson touched two call sites

`isPerson` was hard-coded at both of its call sites and nothing read it back, so
removing the property was two deletions:

- `PostHeader.qml` — `isPerson: true`
- `FeedScreen.qml` — `isPerson: false`

Nothing replaced them. The Stoa/person distinction is carried by **position**:
`PostHeader` renders an author's mark beside their name, `FeedScreen` renders the
Stoa's mark in the Stoa header. A future placement where the surrounding context
does not disambiguate must label the mark, because the shape no longer does.
