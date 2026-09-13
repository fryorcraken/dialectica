# design.md — generated-names

## Context

See `proposal.md` for why. What shapes the approach, and is not in the proposal:

- **Core already has the key at the point it builds a feed row.** `feed.rs`'s
  `list_threads` reads `entry.op.op.author`, which is a `PublicKey`, and calls
  `.address()` on it to fill `FeedRow::author`. So the row already holds the
  derivation's input and throws it away one line later. Nothing has to be
  plumbed; one call is added beside an existing one.
- **`identity.rs` owns every domain separator in the crate**, all four of them
  fixed 32 bytes in one style, and `the_wire_constants_are_pinned_to_known_answers`
  is the test that stops any of them drifting. The name's separator belongs in
  the same family and the name's pin in the same pattern.
- **`docs/IDENTICON.md` already records the mark/abbreviation overlap as a known
  imperfection** and says moving the window "would fix it, at the cost of the
  name scheme's reserved range". That cost does not exist — the reserved range
  was the invented mechanism both documents describe — so the fix is free and
  this change takes it.

## Goals / Non-Goals

**Goals:**

- A derivation `public_key -> name` that is deterministic, bounded, domain
  separated, and pinned to written-down values.
- The three wordlists at the sizes the spec's arithmetic rests on.
- The name on every feed row, beside the address.
- The mark's byte window moved to `4..11`, making the three channels disjoint.

**Non-Goals:**

- **No name anywhere in `identity-onboarding`.** That capability forbids one and
  its slate carries `publicKey` so a holder can compute one.
- **No rendering.** `dialectica-ui/` is out of scope beyond `Identicon.qml`.
- **No name-based lookup, ever.** No method takes a name; see Decisions.

## Decisions

### D1. The wordlists are Rust `&[&str]` arrays in source, not a data file

`include_str!` with parse-at-startup was considered and refused. The lists are
**consensus-critical**: the spec makes removing, adding or reordering one entry
a scheme change that renames every identity drawing at or after that index. A
parsed data file puts a parser between the bytes and the indices — one more
place for two builds to disagree — and it moves a length error from compile time
to run time. As `&[&str]` the length is a `const` a test asserts, and the
compiler guarantees every entry is valid UTF-8 in a fixed order.

The cost is compile time on a 10,240-entry literal, which measured as noise.

### D2. `NAME_PREFIX` joins the family in `identity.rs`, but the derivation lives in its own module

The separator is `b"/dialectica/1/Name/Display\0\0\0\0\0\0\0"` — 32 bytes, the
same padded style as the three existing prefixes, distinct from all of them, and
carrying the scheme version the spec requires. It is defined in `names.rs`
rather than `identity.rs` because it is the only constant this scheme owns and
splitting it from its users buys nothing; `identity.rs`'s pin test asserts it
does not collide with the address prefixes, which is the coupling that matters.

### D3. Slot selection is a function of a digest, separate from hashing a key

`display_name(&PublicKey) -> DisplayName` is the caller's entry point.
`name_from_digest(&[u8; 32]) -> Result<DisplayName, NameError>` is the step
under it. The split is the spec's explicit testability obligation: a refused
first draw, an exhausted reserve and a read past the bound are reachable through
a chosen digest and reachable through a chosen *key* only by grinding for one.

Both are public. The digest-taking half is not a hazard the way
`identity::signing_digest` is — it cannot produce an unauthenticated anything —
and making it `pub(crate)` would put the spec's required tests in this module
only, which is where the tester is least likely to look.

### D4. The byte budget: 6 bytes drawn, 6 in reserve, bound at 12

Three slots × 16 bits = 6 bytes per draw. One redraw of all three slots is 6
more. So:

| bytes | role |
|---|---|
| `0..1` | adjective draw 1 |
| `2..3` | noun draw 1 |
| `4..5` | place draw 1 |
| `6..7` | adjective draw 2 (reserve) |
| `8..9` | noun draw 2 (reserve) |
| `10..11` | place draw 2 (reserve) |
| `12..31` | **never read** |

The bound is byte 12. Reading past it is not merely unused, it is a
`NameError::ReserveExhausted` — the spec's "fail loudly rather than read on".

Each draw is `u16::from_be_bytes` reduced by `%`. Exactly uniform because
`65,536 / 8,192 = 8` and `65,536 / 1,024 = 64`, both integers, so every index of
every list is produced the same number of times. This is why the list sizes are
powers of two and why the spec forbids changing them without a version bump.

Big-endian so that a hand-computed test vector reads in the order the bytes are
written.

### D5. The denylist is on noun–place *index pairs*, stored as a sorted array

A refused combination is `(noun_index, place_index)`. Stored as a
`&[(u16, u16)]` sorted by the pair, looked up with `binary_search`. Not a
`HashSet`: a hash set's iteration order is unstable, which invites a future
change that iterates it into something observable, and construction cost at
every call is worse than a 10-comparison binary search over ~1,000 entries. A
sorted array is also checkable — a test asserts it is sorted and deduplicated,
which is the property the binary search needs and which a `HashSet` would hide.

**The adjective is not part of the key.** The spec: "Only the pair is refused,
never the words." So a refused pair is refused under every adjective, and both
halves stay in their lists.

### D6. `DisplayName` is a struct of three `&'static str`, not a `String`

The three drawn words are indices into `'static` lists, so the natural type
borrows rather than allocates. The struct makes the shape structural: there is
no way to build a `DisplayName` with two words or four, and `to_string()` is the
one place the connector is emitted. The spec's "the connector is not a slot" is
then true by construction rather than by a test on every call site — the
connector literally does not exist in the data.

`words()` returns the three, so a caller too cramped to render `of` has the
spec's permitted relaxation without core deciding for it.

### D7. Malformed key material never reaches the derivation

`display_name` takes a `&PublicKey`, which cannot be constructed from malformed
bytes — `PublicKey::from_bytes` already refuses a wrong length, a non-point and a
low-order point. So the spec's "malformed key material is refused rather than
crashed on" is satisfied **by construction at the type level** for every caller
holding a parsed key.

For a caller holding raw wire bytes there is `display_name_from_bytes(&[u8])`,
which is `PublicKey::from_bytes` then `display_name` — a thin function that
exists so the refusal is one call rather than two steps a caller can get wrong,
exactly as `verify_authored_op` exists for the address binding. It returns the
same `Result`, and there is no placeholder name on any path.

### D8. The feed row carries `display_name`; the wire calls it `displayName`

`FeedRow` gains one field beside `author`, and `feed_page_json` one key. The
address stays. The name is computed in `list_threads` from
`entry.op.op.author` — the key the signature was verified under, two lines
above — and never from anything an op carries.

**A `Post` op cannot carry a name field**, so "a name does not travel as
content" needs no filter: `OpKind::Post` has `body`, `attachments`, `thread` and
`parent`, and a name-shaped string in the body is body text. This is
satisfied-by-construction and the test says so.

### D9. The identicon reads bytes `4..11`

Verified against the code rather than the comments:

| Consumer | Address bytes | Derivation |
|---|---|---|
| Abbreviation head | 0,1,2,3 | `body.substr(0, 8)` — hex chars 0..7 |
| Abbreviation middle | 14,15,16,17 | `start = floor((64-8)/2) = 28`; chars 28..35 |
| Abbreviation tail | 29,30,31 | `body.slice(-6)` — chars 58..63 |
| Mark, **was** | 12..19 | one byte per dimension |
| Mark, **now** | 4..11 | the same eight dimensions, shifted by 8 |
| Name | *none* | a different digest — see below |

The old window overlapped the abbreviation's middle group on exactly
`{14,15,16,17}` — four of the mark's eight bytes were already on screen. `4..11`
is disjoint from all three groups and lies wholly inside the 21 bytes the
abbreviation hides (`4..13` and `18..28`).

**The window moves by a uniform shift of −8, preserving which dimension reads
which relative position.** `_form` was byte 12 and is byte 4; `_weave` was 19
and is 11. Nothing about the dimension assignment changes, so the mark's own
reasoning — that the perceptual space rather than the input is the binding
constraint — is untouched, as is every modulus.

**Nothing is preserved across the move and nothing needs to be.** No user, no
persisted glyph, no compatibility surface. Every address renders a different
mark after this change than before, which is the intended and only effect.

**The name does not participate in this allocation at all.** It derives from
`H(NAME_PREFIX || public_key)`; the mark and abbreviation read the *address*,
`SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key)`. Two different digests, so
byte 3 of one and byte 3 of the other are unrelated and there is no shared space
in which to overlap. `Identicon.qml`'s claim that bytes 0..11 were "reserved for
the generated-name scheme" described a mechanism that never existed; PLAN.md and
`IDENTICON.md` both already carry the correction and the QML did not. It is
fixed here.

### D10. A name is never accepted as input, and that is enforced by absence

The spec forbids any method resolving a name to an identity. Nothing is added to
enforce it: no handler gains a name parameter, and `Request`'s existing
closed-field-set refusal means a request carrying a `displayName` field is
already refused for carrying an unknown field. The requirement holds because
there is nothing to reach, and the test asserts the refusal rather than
asserting a filter exists.

## The list sizes looked unreachable, and the screen was the defect

**RESOLVED — owner decision, recorded because the reasoning below is what made
the resolution visible, and because the census remains true of the rule it was
measured against.** The census that follows is sound arithmetic applied to a
screen the design never imposed: an earlier draft of this spec required every
entry to be a single word with no whitespace, and PLAN.md names exactly two
screens, ASCII-transliterable and deduplicated, adding that "if a word is being
excluded for any reason other than the two above, that is the mistake, not the
word."

The single-word rule was that mistake. It is what discards *alexandria troas*
and *heraclea pontica*, and what turns "Apollonia (northern Crete)" into a
collision rather than a distinct entry — attritions 1 and 2 below, and a large
part of 3. **The screen has been removed from the spec** and multi-word place
entries are accepted, which restores the supply the census had subtracted and
makes 1,024 reachable against merged PLAN.md's anchor of Hansen & Nielsen's
1,035 catalogued poleis.

**The sizes stand at 8,192 / 1,024 / 1,024, and every collision figure in the
spec is unchanged.** Options 1–4 below were the escalation written when the
blocker looked binding; none was taken, and they are kept because the reasoning
that ruled each in or out is not recoverable from the decision alone.

**What is kept as a live caution rather than a resolved one:** the place list's
headroom is thin either way — PLAN.md itself calls it "a little over 10%" and
"the one list where the estimate being wrong would matter" — so the person
writing the list must count, not estimate, and report a shortfall rather than
padding. The four ways to fake a full list, below, are all still defects.

The original finding follows, as written.

Two requirements in the spec cannot both be satisfied for the place list:

- *"The list sizes SHALL be exactly 8,192 adjectives, 1,024 nouns and 1,024
  places"* — from **A name is three drawn words in the form adjective noun of
  place**.
- *"Each list SHALL be **cut down to the nearest power of two below what its
  source yields, never padded up to reach one**. The direction matters — a list
  with headroom discards real candidates, which costs only choice, where a padded
  list ships entries invented to fill it."* — same requirement, four sentences
  later.

**The place source does not yield 1,024.** A census of the enumerated categories
puts the honest supply at **620-780 net-distinct entries**, with the estimate
anchored on two *complete* category audits (ancient Crete, 113 raw entries;
the ancient Aegean islands, 145 raw) that each independently yielded ~70-75%
net-new after the losses below. The categories counted are the full set: mainland
poleis across all seventeen regions, the 139 Attic demes, Crete, the Aegean
islands, the Asia Minor coast, Magna Graecia and Sicily, the Black Sea colonies,
Cyrenaica and the western colonies, sanctuaries, mountains and rivers, regions,
and mythological geography.

Three attritions do the damage, and the third is the one that is easy to miss:

1. **Multi-word names fail the single-word screen** — *alexandria troas*,
   *heraclea pontica*, *antioch on the orontes*.
2. **Sources disambiguate with parentheses rather than with distinct names** —
   "Apollonia (northern Crete)". Stripping the parenthetical is what *causes* the
   collision; keeping it violates the single-word screen.
3. **Cross-regional collision, which compounds.** The Greeks reused toponyms
   relentlessly: Minoa appears three times in the Aegean alone and twice more on
   Crete; Apollonia, Heraclea, Chersonesus, Naxos, Magnesia and Arsinoe recur
   across a dozen regions. The deduplication screen destroys all but one of each.
   **Yield therefore falls as the list grows** — each region added collides more
   with what is already held, so the last 200 entries are far harder than the
   first 200.

Reaching 1,024 requires one of four things, and each is a defect the spec names:

- shipping near-duplicate transliterations as distinct entries (*gortyn* and
  *gortyna*, *polichna* and *polichne*) — which passes a naive string comparison
  while violating deduplication in substance;
- shipping Latinised/Hellenised doublets (*knossos*/*cnossus*,
  *miletos*/*miletus*) — the same trap, and the easiest to fall into by accident
  because the sources themselves carry both;
- descending below the polis/deme level into unlocatable single-inscription
  fragments;
- **inventing entries**, which is what the spec forbids by name. The
  mythological bucket genuinely supplies 30-40; any list claiming 150 legendary
  Greek places is fabricating them.

The last is the failure mode that matters most here, and it is invisible on
inspection: **a fabricated Greek toponym reads exactly like a real one.** A
reviewer cannot catch it by reading the list, and no test can catch it at all.
That is precisely why the shortfall must be resolved by decision rather than
absorbed quietly by whoever writes the list.

### What the arithmetic becomes

The spec's collision figures are computed against `2^33`. Cutting the place list
to the nearest power of two below its yield gives **512**, and the space becomes
`8192 x 512 x 1024` = **2^32** — half the specified size, so:

| identities in one Stoa | at 2^33 (spec) | at 2^32 (512 places) |
|---|---|---|
| 5,000 | 0.145% | **0.291%** |
| 10,000 | 0.580% | **1.157%** |

Working for the 2^32 row, since the spec asks that a restated figure be
re-derived rather than copied: at k=5,000, `k(k-1)/2 = 12,497,500`;
`12,497,500 / 4,294,967,296 = 2.90982e-3`; less `x^2/2 = 4.2335e-6` gives
`2.90559e-3` = **0.2906%**. At k=10,000, `49,995,000 / 4,294,967,296 =
1.164037e-2`; less `x^2/2 = 6.7749e-5`, plus `x^3/6 = 2.63e-7`, gives
`1.157287e-2` = **1.1573%**.

Both were re-derived independently and agree to four significant figures; the
same derivation reproduces the spec's own 0.145% and 0.580% at 2^33 exactly,
which is what makes the pair trustworthy rather than merely plausible.

**0.291% at 5,000 is four times the four-word scheme's 0.073%** and twice what
the owner accepted when taking three words. Whether that is still acceptable is
the owner's call and not mine — which is exactly why this is reported rather
than decided.

### Options, for the owner

Each is a spec change, and none is available to an implementer:

1. **Accept 512 for both Greek lists** (space `2^31`, **0.5803%** at 5,000).
   Honest lists with real curation margin — at ~790 and ~700 available you
   discard the weakest third, which is the position curation should be in. The
   smallest change to the spec: two numbers, and it agrees with merged PLAN.md.
2. **Accept 512 for both and widen the adjective list to 32,768** (`2^33`
   restored, every collision figure in the spec unchanged). The adjective slot
   is English and uncapped, so this costs no Greek source material. **Unassessed
   — do not adopt without measuring whether 32,768 English adjectives survive
   the tone and authority screens.** This is the option that keeps the arithmetic
   the owner already accepted.
3. **Add a fourth slot**, the lever PR #22 used and PR #27 withdrew. Reopens a
   settled decision and puts a word back on every feed row.
4. **Keep 1,024 and relax the source rule**, accepting variant spellings as
   distinct entries. **This is the option to refuse**: it ships the same place or
   person twice, which is the deduplication screen defeated in substance while
   passing it in form — and it contradicts PLAN.md, which reached "1,024 is not
   reachable" independently and before this spec was written.

### The noun list falls short too, and PLAN.md already said so

Assessed the same way, by enumeration rather than estimate. Raw sweep across
twenty categories — presocratics, the four Hellenistic schools, the Academy,
Peripatetics, Neoplatonists, mathematicians, astronomers, physicians,
historians, orators, poets, playwrights, grammarians, geographers, sculptors,
engineers, naturalists, the sages — yields **745 lines, falling to 580 distinct
bare single-word names**, and to roughly **520-545** after removing Latin and
Christian-era figures, mythological names that are not thinkers, entries that
are already ordinary English words (`ion`, `bias`, `oros`), and true minimal
pairs (`kritias`/`kritios`, `zenodoros`/`zenodotos`, `theon`/`theano`).

The abstractions pool is **not** the constraint: roughly 250-270 clean entries,
which *exceeds* the 224 the design assumed. **Combined honest ceiling: 770-815.**

**The structural cause is the single-word rule, and it is worth understanding
because it also explains the denylist.** The Greek canon individuates people by
*name plus place*: Zeno of Citium, Zeno of Elea, Zeno of Sidon, Zeno of Tarsus
and Zeno of Rhodes are five philosophers and **one** wordlist entry. The same
collapses Philo (×4), Diogenes (×5), Apollonius (×6), Dionysius (×6). That is
164 collapses in the raw sweep, 22% of everything enumerated.

So the noun–place denylist is **re-expanding exactly the distinctions the
single-word rule collapsed** — which is why the *X of Y* shape can spell a real
figure's canonical name at all. The two facts are the same fact.

**`docs/PLAN.md` lines 1363-1368 already record this, on merged main**, and I
read the passage rather than grepping it:

> **Nouns: 512.** The pooled list is deep: roughly 250 terms from the vocabulary
> of Greek thought, and roughly 250 thinkers, writers and makers once the handful
> of argument-move names come out. **1,024 is not reachable** without scraping
> every minor figure in Diogenes Laertius and every technical entry in
> Liddell–Scott […]

Line 1340 flags these as *"estimates, not counts"* and says the way to falsify
them is **"to write the list and count"**. That is what the two censuses did, and
the conclusion holds: PLAN underestimated the named-Greek half (250 against ~520)
while this spec overestimates it (800), and both land on 1,024 being out of reach.

**So the spec contradicts merged PLAN.md on a number PLAN.md had already settled**
— which is a stronger finding than the census alone, because it means the 1,024
was never sourced from an assessment that reached it.

### If both lists go to 512

Space is `8192 x 512 x 512` = **2^31** = 2,147,483,648, and at k=5,000:
`12,497,500 / 2,147,483,648 = 5.81964e-3`; less `x^2/2 = 1.69341e-5` gives
`5.80271e-3` = **0.5803%** — four times the spec's 0.145%, and equal to what the
spec quotes for *ten thousand* identities at 2^33.

**The adjective slot is the lever, and it is the cheap one.** It is English,
uncapped, and 8,192 was itself reached by withdrawing a self-imposed cap rather
than by hitting a source limit. Recovering both lost doublings there — 8,192 to
**32,768** — restores `32768 x 512 x 512` = 2^33 exactly, with the spec's
original collision figures intact and no Greek source strained. Whether 32,768
English adjectives survive the tone and authority screens has **not** been
assessed and must not be assumed; it is the first thing to measure if this route
is taken.

## Risks / Trade-offs

- **The wordlists are the change's real surface and cannot be reviewed by
  reading them.** 10,240 entries is past what a reviewer will check word by
  word. → The screens that *can* be mechanically checked are tests over the whole
  list (ASCII, lowercase, no whitespace, no duplicates, the exact size, no
  project vocabulary, no excluded figure). What no test can check is the tone
  screen on entry 6,000. Stated plainly rather than implied to be covered.

- **The denylist's completeness is not testable.** "Every noun–place pair that
  spells a real figure's canonical name" is a claim about the world. A test can
  assert the list is well-formed, sorted, in range and that the pairs on it are
  refused; it cannot assert that a pair *missing* from it should have been on
  it. → The named figures it does cover are drawn from each noun entry that is a
  person, and the spec's own arithmetic (~800 named Greeks × ~1.2 places) is the
  target. A missed pair renders one identity under a real person's name, which
  is the harm; it is a curation gap and not a code defect, and it cannot be
  repaired after release without a scheme version bump.

- **Moving the mark's window invalidates every rendered mark.** → Accepted and
  free: there is nothing in the field. `tst_identicon.qml` pins the old window
  and moves with it.

- **Three words is twice the collision rate of four** (0.145% vs 0.073% at
  5,000 identities). → The owner's decision, recorded in the proposal, and the
  trade is a word off every feed row.

## Migration Plan

None. Nothing has been derived under any earlier version of this scheme, no mark
has been persisted, and no name has ever been returned. The scheme version in
`NAME_PREFIX` is `1` and exists for the *next* change, not this one.
