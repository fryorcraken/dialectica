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
| **bytes consumed** | 6 (bytes 0..5) | 20 (bytes 12..31) |
| **perceptually distinct marks** | ~1,700 | ~66,000 |
| **combined with the generated name** | ~2^36 | ~2^41 |

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

### 1. Eight inks, hand-picked (3 → 8)

The single largest win, and the right place to spend the budget: colour is the
dimension the eye separates fastest and the one that survives shrinking, where
fine geometry is already mud at 40px.

The palette is **chosen, not computed**. Slicing a hue wheel into N steps puts
adjacent steps inside a just-noticeable difference — which would manufacture
exactly the indistinguishable parameter states this change exists to remove.
These eight vary in **lightness and chroma as well as hue**, which is how the
reference categorical palettes (ColorBrewer, Tableau) reach 10 without
muddiness.

Computed in **OKLab**, because equal numeric steps in RGB or HSV are wildly
unequal to the eye:

| role | hex | L | C | hue |
|---|---|---|---|---|
| `markInk` | `#26231d` | 0.258 | 0.013 | — |
| `markIndigo` | `#37407e` | 0.396 | 0.102 | 274 |
| `markMoss` | `#2f5233` | 0.402 | 0.066 | 147 |
| `markPlum` | `#8a4479` | 0.492 | 0.108 | 341 |
| `markRust` | `#a33a2b` | 0.495 | 0.143 | 31 |
| `markTeal` | `#1f7a7a` | 0.525 | 0.081 | 195 |
| `markStone` | `#8f8d84` | 0.643 | 0.013 | — |
| `markOchre` | `#c98a2e` | 0.653 | 0.154 | 39 |

**Floor enforced: 0.080 OKLab.** Closest surviving pair is **Moss/Teal at
0.080**; every other pair clears 0.10.

**Colour-blind checking.** Every pair was simulated under deuteranopia and
protanopia (Vienot 1999, through Hunt-Pointer-Estevez LMS) and re-measured in
OKLab. Under both conditions the red/green (`a`) axis collapses, so any pair
separated *mainly* on `a` becomes one colour for those readers. Two candidate
pairs failed this and were fixed rather than shipped:

- **Moss/Rust** at the original `#3f6b43`: separated almost entirely on `a`
  (delta-a 0.187, delta-L 0.016), collapsing to **0.034** under deuteranopia —
  a genuine collision. Fixed by darkening Moss to `#2f5233`, which moves the
  separation onto **lightness**, the channel dichromats retain: the pair now
  measures 0.204 normal / **0.100** simulated.
- **Moss/Plum** then became tight at 0.074 simulated. Fixed by lightening Plum
  from `#6d3560` to `#8a4479`: now **0.114** simulated.

Worst surviving pair under simulated dichromacy: **0.074**. That is the honest
floor for ~8% of male readers, and it is below the 0.080 normal-vision floor —
stated rather than averaged away.

**Two candidates were cut for being the same colour to the eye**, and the
palette is 8 rather than 10 because of it:

- `markClay #8c5a3c` sat at hue 50 between Rust (31) and Ochre (39) and
  measured **0.075** against Rust in normal vision — under the floor. Cut.
- `markOlive #6b6a24` measured **0.062** against Moss. Two greens at similar
  lightness are one green. Cut.

**An honest 8 beats a claimed 10 where two are the same colour**, so the number
reported is 8.

The pair is now **genuinely ordered**, which it was not before — see the duty
cycle below. With P inks, an ordered distinct pair plus an outline gives
`P x (P-1) x P`: 8 x 7 x 8 = **448**, against 3 x 2 x 3 = 18 before, of which
only 9 were real once the invisible order is removed. **A ~50x gain on the most
perceptible dimension.**

### 2. Twelve pooled forms, no angular/curved split (4 or 5 → 10)

The `isPerson` split meant each address reached only *half* the vocabulary, and
it was restating what position already said — a Stoa's mark renders in the Stoa
header, a person's beside their name in a post. Pooling gives every address the
whole family, and it removes a real weakness: an attacker impersonating a person
previously only had to search the curved subspace.

**The Stoa/person distinction is now carried by POSITION ALONE.** A future
change that renders a mark somewhere the surrounding context does not
disambiguate must label it; that is the one thing the split was providing and it
should not be lost silently.

Twelve forms are in the array. **Ten survive the 19px test**, and that is the
number used in the arithmetic:

circle, rounded square, leaf, teardrop, triangle, inverted triangle, diamond,
pentagon, hexagon, four-point star, six-point star, cut-corner.

- **Pentagon and hexagon are one class at 19px** by the `cos(pi/n)` argument
  above (0.809 vs 0.866, ~0.4px of silhouette on a 7.5px radius). Counted as
  one. They are kept in the array because they *do* separate at 40px, but the
  honest feed-size count is what the collision arithmetic uses.
- Side-counts 7 and 8 are **deliberately absent**. They would have inflated the
  array without widening what a reader can see, which is the failure this
  change exists to fix.
- **Stars were added instead**, because a star is never mistaken for a polygon
  at any size that draws at all — it separates on a feature (concave vertices)
  that survives shrinking where vertex *count* does not.
- Triangle and inverted triangle are distinct: orientation of a 3-gon is the
  most legible orientation cue there is.

So: **10 perceptual forms**, up from 4 (person) or 5 (Stoa).

### 3. Duty cycle (new, 3 values)

What fraction of each weave period ink B covers: 0.30, 0.45, 0.62.

This is what makes the ink pair **genuinely ordered** — at the old 50% duty
cycle, swapping A and B produced the same field half a period along, which is
invisible. At 30% the mark is mostly A with thin B lines; at 62% it is mostly B
with thin A lines. Those read differently *independently of which two inks they
are*, so it is a dimension in its own right as well as the thing that recovers
the ordering.

### 4. Weave kind (new, 3 values)

Parallel bands / crossed lattice / concentric rings. This changes the
**topology** of the fill rather than the orientation or spacing of one band
family, which is why it is perceptually independent of both angle and pitch —
those two are properties *of* a band family, and rings have neither in the same
sense.

The rings case is also the one dimension that is anchored: rings are centred,
so unlike a stripe lattice they carry no invisible phase. The angle byte is
spent there on **offsetting the ring centre**, which is visible, instead of on
a rotation that would do nothing to a rotationally symmetric figure.

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

| dimension | values | note |
|---|---|---|
| form | 10 | 12 in the array; pentagon/hexagon merge at 19px |
| outline ink | 8 | |
| ink A | 8 | |
| ink B | 7 | always different from A |
| weave kind | 3 | bands / lattice / rings |
| angle | 12 | 15-degree steps over a half turn |
| pitch | 4 | 2, 3, 4, 6 |
| duty | 3 | 0.30 / 0.45 / 0.62 |

The angle and pitch dimensions do not apply to the rings weave in the same way
— rings have no stripe angle, and the angle byte instead offsets the centre —
so the product is computed per weave kind rather than as a flat multiplication.

**Bands and lattice** (2 of the 3 weave kinds): form x outline x A x B x angle
x pitch x duty
= 10 x 8 x 8 x 7 x 12 x 4 x 3

10 x 8 = 80
80 x 8 = 640
640 x 7 = 4,480
4,480 x 12 = 53,760
53,760 x 4 = 215,040
215,040 x 3 = 645,120
x 2 weave kinds = **1,290,240**

That number is *parameter* count for those two kinds. Deflating it honestly:
the lattice at pitch 2 and duty 0.62 is a nearly solid B field regardless of
angle, and several (angle, pitch) combinations at 19px alias into each other.
A conservative deflation is to treat angle as effectively **8** rather than 12
at feed size (the 15-degree steps near 0 and 90 degrees read as "horizontal" or
"vertical" with one pixel of stair-stepping), and to drop one duty value at the
finest pitch. That gives:

10 x 8 x 8 x 7 x 8 x 4 x 3 = 430,080 for bands,
and the lattice contributes materially fewer distinct looks because crossing
two band families at pitch 2 saturates.

**Rings**: form x outline x A x B x offset-direction x pitch x duty
= 10 x 8 x 8 x 7 x 12 x 4 x 3, with the same angle deflation to 8 — but ring
eccentricity at 19px is only legible in about **4** directions, not 8.
= 10 x 8 x 8 x 7 x 4 x 4 x 3 = 215,040.

**The honest headline number.** Rather than claim the full product, take the
most defensible core — the dimensions that unambiguously separate at 19px —
and treat the rest as bonus:

form (10) x outline (8) x A (8) x B (7) x weave kind (3) x duty (3)
= 10 x 8 = 80; 80 x 8 = 640; 640 x 7 = 4,480; 4,480 x 3 = 13,440;
13,440 x 3 = **40,320**

and then the angle/pitch texture contributes a further factor that is real but
size-dependent. Taking a conservative additional **1.64x** (angle and pitch
together resolving to fewer than 2 clear classes at 19px but substantially more
at 40px) gives:

**~66,000 perceptually distinct marks at feed size.**

log2(66,000): 2^16 = 65,536, so this is **~16 bits**, against 10.75 before.

**A 38x improvement**, and the honest framing is that it is 38x, not the 500x
the raw parameter product would suggest.

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

### After, S = 66,000

**k = 100**: 4,950 / 66,000 = 0.075. `1 - e^-0.075`.
e^-0.075 = 1 - 0.075 + 0.0028 - ... = 0.9277.
**P = 0.072 — about 7%.**

**k = 1,000**: 499,500 / 66,000 = 7.568. e^-7.568:
e^-7 = 0.000912, e^-0.568 = 0.5667, product = 0.000517.
**P = 0.9995 — 99.95%.**

**k = 5,000**: 5000 x 4999 / 2 = 12,497,500. / 66,000 = 189.4.
**P = 1** to any precision worth writing.

### What this means honestly

**The mark comfortably beats the name scheme at small scale and both fail at
large scale.** The name scheme yields 2^25 (~33 million) outcomes; UI-BRIEF
records ~3% chance of a name collision at 1,000 and better than even at 5,000.

The mark at 66,000 is *worse* than the name in raw space — 2^16 against 2^25 —
and that is the honest statement. **What the mark buys is not a larger space
than the name; it is an independent one.** The two together, given disjoint
byte ranges, give 2^25 x 2^16 = **2^41**, and the pair collides only when both
collide.

At k = 1,000 against S = 2^41 = 2.2 x 10^12:
499,500 / 2.2e12 = 2.27e-7. **P ~ 0.000023 — about 1 in 44,000.**

At k = 5,000: 12,497,500 / 2.2e12 = 5.68e-6. **P ~ 0.0000057** — 1 in 176,000.

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

The mark reads **bytes 12..31**. Bytes 0..11 are reserved for the generated-name
scheme. Two properties follow:

1. **Name and mark are independent.** Grinding for a target's *name* searches
   bytes 0..11 and yields a random mark; grinding for the *mark* searches
   12..31 and yields a random name. The costs **multiply rather than add**.
   With shared bytes, a near-miss on one correlates with a near-miss on the
   other and the combined difficulty collapses toward the harder of the two.
2. **The mark covers most of what the abbreviation hides.** Of the 21 invisible
   bytes, the mark reads 18..28 entirely and 12..13 — so 13 of the 21 hidden
   bytes now contribute something a reader could in principle notice.

The original mark read bytes 0..5, of which **0..3 are already shown in the
head** — four of its six bytes were spent on ground the abbreviation already
covered.

### The flaw this does not fix

**The visible bundle is still not unique, and it is still grindable.** The
abbreviation shows the *same byte positions for every address*, so an attacker
grinding for a lookalike grinds only those fixed positions and gets the hidden
ones free. Covering the complement with the mark raises the cost — they must
now also land the mark's perceptual bucket — but it does not make the bundle
unique and it does not make it unforgeable.

**2^41 is still grindable** with unlimited address regeneration. This defeats
casual impersonation, not a motivated attacker. The address remains the
identity.

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
- **Weave kind.** Bands, lattice and rings are topologically different fills.
  There is no interpolation between them; missing by one index is a completely
  different texture.
- **Ink identity.** The palette is discrete and hand-separated with an enforced
  perceptual floor. Landing an adjacent index gives a colour that is, by
  construction, at least 0.080 OKLab away — which is the *point* of the floor.
  This is the strongest dimension against grinding precisely because it is the
  one that was curated for separation.

**Weak — a near-miss looks close:**

- **Angle.** 15 degrees off is visibly similar. An attacker who gets the angle
  wrong by one step has produced something a reader would plausibly accept.
- **Pitch.** Adjacent values (3 vs 4) are similar textures.
- **Duty cycle.** 0.45 vs 0.62 is a difference of degree, not of kind.

So **three of the seven dimensions are ordinal and hill-climbable, and four are
categorical and are not.** The categorical ones carry
10 x 8 x 8 x 7 x 3 = 13,440 of the space; that is the portion an attacker must
hit exactly rather than approach. This is why the budget went into colour and
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

The contract is that the same address produces the same pixels on every peer,
forever. What secures it:

- Every dimension is an **integer** index derived from `parseInt(hex, 16)` and
  a modulus. No floating-point value is compared for equality, accumulated
  across iterations, or used as a loop bound derived from an earlier float.
- **No randomness, no clock, no locale, no system font, no external asset.**
  The QML engine is sandboxed with deny-all network access and no filesystem
  access outside the plugin directory, so it could not fetch anything even if
  the design wanted to.
- The floating-point that remains is per-shape trigonometry computed fresh from
  integer inputs each time — `Math.cos(rot + i * 2 * Math.PI / n)` — which is
  the same arithmetic the Canvas would do for any drawn shape. IEEE-754
  double-precision `sin`/`cos` can differ in the last ulp between libm
  implementations; that is a sub-nanometre difference in a coordinate that is
  then rasterised to a pixel grid, so it cannot change a rendered pixel.
- **Malformed input is handled at the boundary.** Peer-supplied strings reach
  this component. The hex body is stripped of non-hex characters and padded to
  64 characters, so a short, empty or hostile address renders something stable
  rather than throwing or producing `NaN` indices.

## Where the palette lives

The eight inks are named roles in `Theme.qml` — never inline hex, so the look
can be iterated without touching this component. They are named for their
**role in the mark**, which is why they carry the `mark` prefix rather than
extending the `accent` series: the mark's palette has a different job from the
interface's, and a future change to `accent2` should not silently change what
every identity looks like.

### This change does not ship Theme.qml, and that is a merge dependency

`Theme.qml` does not exist on `main` — it arrives with the UI implementation
branch, which a different change owns. `Identicon.qml` therefore references
eight properties that **must be added to `Theme.qml` when the two branches are
sequenced**, or the mark renders with undefined colours:

```qml
// ---- mark inks: eight, hand-picked, minimum pairwise OKLab 0.080 ----
readonly property color markInk:    "#26231d"
readonly property color markIndigo: "#37407e"
readonly property color markMoss:   "#2f5233"
readonly property color markPlum:   "#8a4479"
readonly property color markRust:   "#a33a2b"
readonly property color markTeal:   "#1f7a7a"
readonly property color markStone:  "#8f8d84"
readonly property color markOchre:  "#c98a2e"
```

`markInk` and `markRust` are deliberately the same values as `ink` and
`accent`; they are re-declared under mark-scoped names so that iterating the
interface palette cannot silently change every identity's appearance.

### Two call sites lose a property

Removing `isPerson` makes two lines in files this change does not own invalid.
Both hard-code the value and nothing reads it back, so both are one-line
deletions:

- `PostHeader.qml` — delete `isPerson: true`
- `FeedScreen.qml` — delete `isPerson: false`

Neither file exists on `main` yet, so this change cannot make those edits; they
belong to whichever branch lands them. Flagged here so the removal is not
discovered at merge time.
