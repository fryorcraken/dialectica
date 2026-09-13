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

**The arrays are generated, not typed.** `tmp/gen.rs` reads the curated text
files and emits the four modules. That is a decision about *provenance* rather
than convenience, and it is the one thing that makes the spec's attestation
screen auditable: a reviewer can diff a generated array against the text file it
came from, and the text file against the census it came from, where a
hand-transcribed array can only be read and believed. It also removes the defect
both census agents hit and one of them caught in itself — **Cyrillic homoglyphs
in hand-typed Greek**, where the habit is reliable and the typing is not.
Generation cannot introduce a character that was not in the source, and the
generator asserts ASCII before it writes.

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

### D11. The lists are built by cutting a surplus down, never by padding up

The spec requires each list be **cut down to the nearest power of two below what
its source yields, never padded up to reach one**, and the direction is the whole
point: a list with headroom discards real candidates, which costs only choice,
where a padded list ships entries invented to fill it. Every list here was built
that way, and the margin is recorded because it is the evidence the rule was
followed:

| list | source yield | shipped | discarded |
|---|---|---|---|
| adjectives | 17,349 | 8,192 | 9,157 |
| nouns | 1,892 | 1,024 | 868 |
| places | 1,131 | 1,024 | 107 |

The counts are from `grep -c .` on the files, not from an estimate.

**The place list is the one with thin margin, and its 107 cuts are each a defect
the spec names rather than a judgement about what a place connotes.** Latinised
doublets sitting beside their transliterations (`piraeus` beside `peiraion`,
`ocalea`, `tricolonoi`, `halaesa`), near-duplicate transliterations of one place
(`gortynia` beside `gortyn` and `gortys`), entries that are not toponyms at all
(`lelantine` is an adjective; `asphodel` is the English form of `asphodelos`),
one Latin name (`avernus`), and Greek exonyms for non-Greek regions
(`india`, `persis`, `sarmatia`, `baktria`). The first two families are the
deduplication screen applied *in substance* — shipping them would pass a naive
string comparison while violating dedup in fact, which the spec names as a way to
fake a full list.

**The noun list's 868 cuts include 24 that matter structurally**: qualified forms
like `zenon kitieus` and `straton lampsakenos`. A source supplying named
historical Greeks supplies them already attached to a place, and such an entry
renders *measured zenon kitieus of lampsakos* — two places on one name, with no
way for a reader to tell which the place slot supplied. The bare names stay, so
no figure is lost; what changes is that the noun–place denylist becomes the thing
that handles this family, at the pair level where the spec puts it. Eight
Roman/Latin entries (`lucretius`, `agrippa`, `antoninus`, `quintus`, `rufus`,
`musonius`, `josephos`, `aelianus`) were also cut: the spec's source is any
attested ancient *Greek* noun.

### D12. The adjective list is derived from a dictionary, not from recall

8,192 English adjectives is past what anyone writes down honestly, and the
failure mode is specific: an agent asked for 8,192 words produces perhaps 2,000
real ones and then fills. So the list is derived mechanically from
`/usr/share/dict/words` — Webster's unabridged, present on the build host — by
suffix, and the selection rules are stated here so the result is reproducible
rather than merely asserted:

1. Words matching `[a-z]{4,8}(able|ible|ous|ive|ful|less)` — the six
   near-unambiguously adjectival English suffixes. 11,467 matched.
2. Less the technical `-ous` tails (`-aceous`, `-iferous`, `-ivorous`,
   `-icolous`, `-ogenous`, …) and the `-eable` doublets. 10,345 left.
3. Less the `un-`, `non-`, `ir-`, `il-` negations and the `over-`, `under-`,
   `counter-`, `pseudo-`, `semi-`, `multi-`, `inter-`, `super-` compounds — each
   a mechanical derivative of a base already in the list, which is the same
   near-duplicate family the dedup screen targets. 8,454 left.
4. Less the 115 `-isable`/`-izable` British/American spelling doublets. 8,339.
5. Less 147 by a **deterministic stride** over line numbers, to land on exactly
   8,192.

**Step 5 is the honest part and is worth naming as such.** Steps 1–4 are
defensible rules; step 5 is not a rule, it is arithmetic to reach a power of two,
and no amount of regex tuning would have made it one. A stride was chosen over
cutting the alphabetical tail precisely because the tail would have deleted every
adjective from `v` to `z` — a visible, arbitrary bias — where a stride treats
every letter alike. The alternative considered and rejected was to widen step 1
until some *other* rule happened to land on 8,192, which is the same arbitrariness
with its arithmetic hidden.

**This is better for the attestation screen than hand-authoring would have
been**, which is the argument for the whole approach: every entry traces to a
line in a dictionary file on disk, so "is this a real English adjective?" is a
question a reviewer can answer with `grep` rather than by trusting the author's
memory. The cost is register — `abdominous` and `viraginous` draw alongside
`luminous` — and that cost is accepted rather than argued away, because the spec
forbids a familiarity screen and a register screen by name.

### D13. The denylist is generated from a named-figure table, and its gaps are visible

`tmp/attributions.txt` lists each named historical Greek in the noun list beside
the place that figure is canonically cited with; the generator resolves both
halves to indices and emits the sorted pair array. An attribution naming a word
that did not survive curation is **reported and skipped rather than failing the
build**: the pair is unreachable because one half is absent, so there is nothing
to refuse, and treating it as an error would make a curation change look like a
defect.

The completeness of that table is the one claim in this change that no test can
check, and it is stated in Risks below rather than implied to be covered.

## What was superseded, and why it is not here

An earlier draft of this document carried a long escalation arguing that 1,024
was unreachable for both Greek lists and that the sizes should fall to 512. That
argument is **gone rather than struck through**, and the reason is worth one
paragraph because the conclusion was wrong in an instructive way.

It rested on two censuses, both written against a noun slot read as *"the
vocabulary of Greek thought plus named thinkers"* — a narrow technical
vocabulary. The spec's slot is any attested ancient Greek noun across four pools,
and the two pools those censuses omitted entirely (mythological figures, ordinary
concrete nouns) turned out to be the larger half: 341 and 811 against the 443 and
304 of the two they did count. A census taken under the old scope was not
evidence about the new one, in either direction, and the 551 it reported against
a target of 1,024 became 1,892 once the scope was measured as written.

The place census was under unchanged scope and did apply, and its author was
honest that the margin was thin. It held: 1,131 deduplicated, 1,024 shipped, 107
discarded. Thin is not short.

**The lesson kept rather than the argument**: a count is evidence only about the
scope it was taken under, and "the source does not yield N" is a claim to
re-measure whenever the source definition moves.

## Risks / Trade-offs

- **The wordlists are the change's real surface and cannot be reviewed by
  reading them.** 10,240 entries is past what a reviewer will check word by
  word. → What *can* be checked mechanically is checked, as tests over the whole
  of every list: the exact size, ASCII, lowercase, well-formed spacing, no
  duplicates, and no noun carrying the connector. **What no test can check is
  attestation** — whether entry 6,000 is a real word. That is stated here rather
  than implied to be covered, and it is why the lists are generated from files a
  reviewer can trace rather than transcribed by hand.

  Note there is **no tone, register, familiarity or exclusion check to write**,
  and their absence is the requirement rather than a gap: every draft that added
  a fourth screen was withdrawn. A test asserting `platon` is absent would now be
  a defect, and the test that replaced the three such tests asserts the opposite.

- **The denylist's completeness is not testable, and this is the change's largest
  uncheckable claim.** "Every noun–place pair that spells a real figure's
  canonical name" is a claim about the world. A test can assert the list is
  sorted, deduplicated, in range, and that the pairs on it are refused; it
  **cannot** assert that a pair missing from it should have been on it. → The
  table covers the named historical Greeks in the shipped noun list against the
  places canonically associated with them, built to under-include rather than
  guess: a figure whose canonical place the author was unsure of was left out. A
  missed pair renders one identity under a real person's name — a curation gap
  rather than a code defect, and one that cannot be repaired after release
  without a scheme version bump.

- **The adjective list's register is uneven, and that is the contract working
  rather than failing.** A dictionary-derived list contains `abdominous` beside
  `luminous`. The spec forbids a familiarity screen and a register screen by
  name, and the long tail is described there as "deliberately in, and the
  consequence is accepted rather than argued away — some names will be legible
  but hard to tell apart". → Accepted. The alternative is a fourth screen, which
  is the mistake this contract has made and withdrawn four times.

- **The final 147 adjectives were cut by a stride rather than by a rule.** →
  Recorded in D12 rather than dressed up as mechanical. No rule lands on a power
  of two by itself, and the honest options were a stride or a rationalised regex
  tuned until its count matched; the stride is the one whose arbitrariness is
  visible.

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
