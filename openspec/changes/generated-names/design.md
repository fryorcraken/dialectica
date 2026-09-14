# design.md — generated-names

## Context

See `proposal.md` for why. What shapes the approach, and is not in the proposal:

- **Core already has the key at the point it builds a feed row.** `feed.rs`'s
  `list_threads` reads `entry.op.op.author`, which is a `PublicKey`, and calls
  `.address()` on it to fill `FeedRow::author` — so the row holds the
  derivation's input and throws it away one line later. **That observation led
  this change the wrong way once** (D8): it was read as "core should derive the
  name here", where the right reading is "the row should carry the key". The
  observation is kept because it is still what makes the eventual fix a small
  one; the conclusion drawn from it is not.
- **`identity.rs` owns the crate's other domain separators**, all fixed 32 bytes
  in one style. The name's separator follows that style and its pin follows that
  pattern — but nothing couples them, and no test compares them (D2).
- **`docs/IDENTICON.md` already records the mark/abbreviation overlap as a known
  imperfection** and says moving the window "would fix it, at the cost of the
  name scheme's reserved range". That cost does not exist — the reserved range
  was the invented mechanism both documents describe — so the fix is free and
  this change takes it.

## Goals / Non-Goals

**Goals:**

- A derivation `public_key -> name` that is deterministic, total, bounded, domain
  separated, and pinned to written-down values.
- The three wordlists at the sizes the spec's arithmetic rests on, generated from
  tracked sources rather than transcribed.
- **The derivation exposed to callers, and the name kept off every reply.** These
  are one goal rather than two: the name is obtained by deriving it, so core must
  answer, and nothing carries it.
- The mark's byte window moved to `4..11`, making those two channels disjoint.

**Non-Goals:**

- **No name on any reply**, and this is the scope reduction that reshaped the
  change. Not a feed row, not a thread item, not a slate candidate. See D8.
- **Not the public key on a feed row either.** That is what a caller needs to
  use the derivation, and it is a change to the `author` field's contract across
  several merged specs — its own piece, filed rather than done here.
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

**The real cost is a 144 KB `adjectives.rs` that no reviewer will ever read**,
and naming that is more honest than the compile-time figure this entry used to
lead with. Compile time on a 10,240-entry literal measured as noise; the cost
that matters is that the shipped artefact is past human review by construction,
so its only defence is the provenance chain below. That makes the chain
load-bearing rather than a nicety — which is why the next paragraph is about
where it lives.

**The arrays are generated, not typed.** `examples/gen_wordlists.rs` reads the
curated text files in `wordlists/` and emits the three modules. That is a
decision about *provenance* rather than convenience, and it is the one thing that
makes the spec's attestation screen auditable: a reviewer can diff a generated
array against the text file it came from, where a hand-transcribed array can only
be read and believed. It also removes the defect both census agents hit and one
of them caught in itself — **Cyrillic homoglyphs in hand-typed Greek**, where the
habit is reliable and the typing is not. Generation cannot introduce a character
that was not in the source, and the generator asserts ASCII before it writes.

**The generator and its inputs are tracked, and that is the correction that makes
this entry true.** Both previously lived under `tmp/`, which `.gitignore`
excludes — so on every checkout but the author's, `git ls-files tmp/` returned
nothing, the three `sha256sum` commands `names.rs` tells a reviewer to run all
failed, and the generated arrays were exactly the hand-transcribed arrays this
decision rejected: auditable by nobody, re-derivable by no one, and
consensus-critical. The argument for generating rather than parsing requires the
source to survive the merge; it now does.

Verified rather than assumed: `sha256sum` over the three tracked files reproduces
`PINNED_ADJECTIVES_SHA256`, `PINNED_NOUNS_SHA256` and `PINNED_PLACES_SHA256` byte
for byte, so the move preserved the exact bytes the pins were taken over.

### D2. `NAME_PREFIX` lives in `names.rs`, beside its only user

The separator is `b"/dialectica/1/Name/Display\0\0\0\0\0\0"` — **six** trailing
NULs. `/dialectica/1/Name/Display` is 26 characters, so 26 + 6 = 32, which is the
`&[u8; 32]` the code declares and the fixed width this decision rests on. It is
distinct from every other prefix in the crate and carries the scheme version the
spec requires.

*An earlier version of this entry recorded the literal with **seven** NULs — 33
bytes, which would not compile.* Corrected rather than quietly fixed, because
this is the consensus-critical constant of the whole scheme and a reader
reconstructing the preimage from this document would have computed a digest that
matches no name the network produces. A transcribed constant is worth checking
by arithmetic, not by eye.

It is defined in `names.rs` rather than in `identity.rs`, which owns the other
separators, for one reason: it is the only constant this scheme owns, and every
reader of it is in this module. Splitting it from its users would buy a tidier
grouping and cost a reader a file hop.

**What this entry previously claimed as its justification does not exist**, and
naming that is the point of recording it. The old text said *"`identity.rs`'s pin
test asserts it does not collide with the address prefixes, which is the coupling
that matters."* It does not: `the_wire_constants_are_pinned_to_known_answers`
pins four derived values and never mentions `NAME_PREFIX`, and **no test anywhere
asserts the separators are pairwise distinct**. They *are* distinct today,
verified by hand — but the property is enforced by nothing, and all of them are
private `const`s, which `cargo mutants` cannot reach either.

That gap is real and is left open deliberately rather than closed here. Closing
it means making the `identity.rs` prefixes visible to a test that compares them,
which is a widening of that module's surface to serve a check on this one; it is
worth doing as its own piece, against all six separators at once, rather than
bolted to the side of this change. **Recorded as a known gap so the next reader
inherits the question rather than the false reassurance.**

### D3. Slot selection is a function of a digest, separate from hashing a key

`display_name(&PublicKey) -> DisplayName` is the caller's entry point.
`name_from_digest(&[u8; 32]) -> DisplayName` is the step under it. Both are
infallible; see D7 for why neither returns a `Result`.

The split is the spec's explicit testability obligation, and what it buys changed
when the denylist went. It used to be the only way to reach a refused draw, an
exhausted reserve or the bound. Those paths no longer exist — but **reaching a
chosen slot combination still means grinding for a key**, so every requirement
about *which words come back* is checkable only through a supplied digest. That
is what `every_combination_the_draws_select_is_returned`, the span pins and
`a_real_figures_canonical_citation_is_returned_like_any_other_draw` all rest on:
each names the indices it wants and builds the digest that selects them.

Both are public. The digest-taking half is not a hazard the way
`identity::signing_digest` is — it cannot produce an unauthenticated anything —
and making it `pub(crate)` would put the spec's required tests in this module
only, which is where the tester is least likely to look.

### D4. The byte budget: 6 bytes drawn, bound at 6, nothing in reserve

Three slots × 16 bits = 6 bytes. That is the whole budget:

| bytes | role |
|---|---|
| `0..1` | adjective |
| `2..3` | noun |
| `4..5` | place |
| `6..31` | **never read** |

**The bound is 6 and it names no byte the derivation does not read.** An earlier
version of this scheme set it at 12, reserving `6..12` to fund one whole-name
redraw when a draw hit the denylist. The denylist is gone (D5), so the reserve
funds nothing — and a bound stated wider than the draws is a boundary nothing
enforces: a range read by nothing, which the next reader takes as load-bearing
and designs around. That is precisely the byte reservation this scheme has
already had to retract once, in a different document, about a different pair of
channels.

**The bound bounds by being what the derivation slices, not by being asserted.**
`name_from_digest` takes `digest[..NAME_DIGEST_BOUND]` and reads its three draws
from that slice, so moving the constant moves the read and a draw past it does
not compile. The previous constant was compared against its own literal in a
`debug_assert_eq!` — a tautology that compiled out in release and could not fail
under any edit to the draws, so the invariant a reader was told the constant
enforced was in fact enforced by two literal offsets elsewhere.

Each draw is `u16::from_be_bytes` reduced by `%`. Exactly uniform because
`65,536 / 8,192 = 8` and `65,536 / 1,024 = 64`, both integers, so every index of
every list is produced the same number of times. This is why the list sizes are
powers of two and why the spec forbids changing them without a version bump.

Big-endian so that a hand-computed test vector reads in the order the bytes are
written.

**The consequence that matters most is totality.** With one unconditional
reduction per slot there is no path that can refuse, retry or run out, so the
derivation is total over well-formed keys — see D7 for what that does to the
signature, and D8 for the branch it deleted.

### D5. There is no denylist, and the deletion is the decision

An earlier version of this change shipped 199 refused noun–place pairs — the
combinations that spell how a real historical figure is conventionally cited,
*straton of lampsakos* and the like — together with a whole-name redraw that ran
when a draw hit one, the `6..12` reserve bytes that funded the redraw, and a
`NameError::ReserveExhausted` for when both draws were refused. **All of it is
deleted, by owner ruling, and the ruling is recorded here rather than argued.**

**The reasoning, because it decides edge cases the spec does not enumerate:**
drawing a name that coincides with a real historical figure's citation is a
coincidence rather than a harm. Straton of Lampsacus died around 269 BC; nobody
is impersonating him, and the system asserts nothing whatever about a name's
bearer. The same ruling had already killed every semantic exclusion in the
wordlists, and this is that ruling applied to the other place a filter can live —
leaving every word in and refusing what they combine into.

**Exactly one rule still keeps anything out**: no wordlist entry may contain
` of ` as a word. That survives because it is about an entry's *spelling* rather
than any word's meaning — `zenon of kition` as a noun would render *pensive zenon
of kition of lampsakos*, two places on one name with no way for a reader to tell
which the place slot supplied. The bare `zenon` stays and draws normally.

**What the deletion buys is a scheme with no conditional path at all**, and that
is worth more than the filter was:

- The derivation is **total** over well-formed keys, which removes an error
  variant, a `Result` from the caller's signature, and a content-dropping branch
  in `feed.rs` (D8).
- A name consumes a **fixed** six bytes rather than a data-dependent 6 or 12.
- The `2^33` space is reached **exactly** rather than approximately, so the
  collision figures in the spec hold without a caveat.
- A second implementation agrees with this one by reducing three 16-bit values.
  There is no table of pairs it must also hold — and **no shared table that
  could drift between peers and rename somebody**, which was the deleted
  mechanism's worst property: its completeness was a claim about the world that
  no test could check, and a divergent copy of it would have renamed identities
  silently.

The cost is accepted and is exactly what the ruling accepts: an identity may
derive to a real figure's canonical name and, there being no rotation, sign every
post with it.

*The sorted-array-over-`HashSet` reasoning this entry used to carry went with the
array. It was sound and is not worth preserving for a structure that no longer
exists.*

### D6. `DisplayName` is a struct of three `&'static str`, not a `String`

The three drawn words are indices into `'static` lists, so the natural type
borrows rather than allocates. The struct makes the shape structural: there is
no way to build a `DisplayName` with two words or four, and `to_string()` is the
one place the connector is emitted. The spec's "the connector is not a slot" is
then true by construction rather than by a test on every call site — the
connector literally does not exist in the data.

`words()` returns the three, so a caller too cramped to render `of` has the
spec's permitted relaxation without core deciding for it.

**The module's `pub` surface is wider than any caller needs, and narrowing it is
deferred to the piece that adds the wire method.** Right now `names` has **zero**
non-test consumers: nothing in `feed.rs`, `wire.rs` or `thread.rs` reaches
`ADJECTIVES`, `NOUNS`, `PLACES`, `CONNECTOR`, `words()`, `name_digest`,
`name_from_digest` or `display_name_from_bytes`. Exposing the derivation is the
only way a name is ever obtained, and it is filed as issue **#81** rather than
required by this change's spec — the requirement was moved out of the delta
because nothing implements it. The public entry point is therefore about to be
decided by that piece, and narrowing one task before it needs widening would be
two churns in opposite directions.

What that piece should settle, recorded so it is not re-derived:

- `display_name`, `display_name_from_bytes` and `render()` are what a caller
  needs. `name_from_digest` and `name_digest` are the testability seam D3 and D7
  record.
- **The three wordlist arrays should not be `pub`.** They are the scheme's
  private data, their *indices* are the consensus, and exporting them invites a
  second reader of a list that must never be read twice. `pub(crate)` serves
  every present use, and the crate's own tests reach them through `use super::*`
  regardless.
- `words()`'s justification above is "a caller too cramped to render `of`" —
  which is the view, which this document's own Non-Goals put out of scope. Either
  the reason or the method should go.

### D7. Malformed key material never reaches the derivation, and `display_name` returns no `Result`

`display_name` takes a `&PublicKey`, which cannot be constructed from malformed
bytes — `PublicKey::from_bytes` already refuses a wrong length, a non-point and a
low-order point. So the spec's "malformed key material is refused rather than
crashed on" is satisfied **by construction at the type level** for every caller
holding a parsed key.

For a caller holding raw wire bytes there is `display_name_from_bytes(&[u8])`,
which is `PublicKey::from_bytes` then `display_name` — a thin function that
exists so the refusal is one call rather than two steps a caller can get wrong,
exactly as `verify_authored_op` exists for the address binding. **That is where
the only `Result` in this capability lives**, and there is no placeholder name on
any path.

**`display_name` itself returns a bare `DisplayName`.** Once the denylist went
(D5), nothing after the key parses could fail — so a `Result` there would be an
error variant no input can produce, which the spec forbids by name: *"an error
variant no input can produce is an unreachable branch that a reader takes as
evidence the failure exists, and a caller handles a case that cannot arrive."*

The signature is the enforcement. `NameError` has one arm, reachable from one
function, and "an unrecognised key is refused" is not expressible in the type.
This is the change that made D8's deletion mechanical rather than a judgement
call: a caller cannot write a fallback path for a failure the type does not
admit.

### D8. No reply carries a name, and the feed row is where that was got wrong first

**The name does not go over the wire, on any reply.** Not a feed row, not a
thread item, not an onboarding slate candidate. It is derived by whoever holds
the key, at the point of rendering.

An earlier version of this change did the opposite: `FeedRow` gained a
`display_name`, `feed_page_json` gained a `displayName`, and the reasoning was
that a feed row reports an *address*, no key is recoverable from an address, so a
view holding one cannot derive a name and core must supply it. **The premise was
right and the conclusion was wrong.** The gap is real; the fix is to stop
dropping the key, not to start sending names.

Two arguments settle it, and the second is why the first is not merely a
preference:

- **A derived value beside the material it derives from is two values that must
  agree and could disagree**, with no way for a recipient to tell which is wrong.
  A `displayName` beside an `authorKey` is a forgery surface: a relay that
  rewrites the name while leaving the key intact produces a reply that renders a
  false attribution and verifies fine.
- **So the name is not a thing a reply carries, in either direction.** What a
  reply owes is the derivation's *input*. That obligation belongs to each reply's
  own capability, not to this one — which is why the fix is filed rather than
  done here: putting the public key on a feed row changes the `author` field's
  contract and touches several merged specs.

**This is a recorded gap, not a closed one.** `list_threads` still returns an
address per row and drops the key it verified the signature under, so a view
reading a feed still cannot render an attribution. `docs/UI-BRIEF.md` obligation
6 states the rendering obligation and names this same gap. What this change does
is stop the wrong fix from shipping, and pin the absence so it cannot creep back
(below).

**The absence is asserted positively rather than left as the current shape.**
`the_feed_reply_is_the_ecosystems_pagination_shape` asserts the row's whole key
set with `assert_eq!` on a sorted `Vec`, so a restored `displayName` fails on an
*added* key — measured, by putting one back and watching it fail. On the struct
side, `a_row_carries_the_address_and_no_derived_display_name` and the e2e feed
test both destructure `FeedRow` exhaustively, so a new field stops the tests
*compiling*. That is louder than an assertion and cannot be skipped.

The shape is deliberate: an absence nobody asserts is an absence a later pass
restores with every test still green, which is how the withdrawn wordlist
exclusions nearly came back, and why `the_lists_carry_no_exclusion_of_any_kind`
pins words as **present**.

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
| adjectives | 11,467 | 8,192 | 3,275 |
| nouns | 1,892 | 1,024 | 868 |
| places | 1,131 | 1,024 | 107 |

**The adjective figure was wrong here and is corrected**: this table read 17,349,
which is reproducible from no file and which contradicted D12's own step 1 in the
same document. D12 chains every later step from **11,467** — the count that
matched the suffix regex — and each link in that chain verifies by `grep -c .`
against a file: 11,467 → 10,345 → 8,454 → 8,339 → 8,192. The nearest figures to
17,349 are 18,837 (`adj-strong.txt`) and 89,003 (the raw dictionary), neither of
which is a step. **D12 is the authority for the adjective count and this table
now restates it rather than competing with it.** Where the two disagreed, the one
with a reproducible chain was right.

The noun and place figures do verify: 1,892 against the combined deduplicated
census and 1,131 against the place census, both by `grep -c .`.

**The place list is the one with thin margin.** Its 107 cuts are recorded as
defects the spec names rather than judgements about what a place connotes:
Latinised doublets sitting beside their transliterations (`piraeus` beside
`peiraion`, `ocalea`, `tricolonoi`, `halaesa`), near-duplicate transliterations of
one place (`gortynia` beside `gortyn` and `gortys`), entries that are not
toponyms at all (`lelantine` is an adjective; `asphodel` is the English form of
`asphodelos`), one Latin name (`avernus`), and Greek exonyms for non-Greek
regions (`india`, `persis`, `sarmatia`, `baktria`). The first two families are
the deduplication screen applied *in substance* — shipping them would pass a
naive string comparison while violating dedup in fact, which the spec names as a
way to fake a full list.

**The place list deduplicates by PLACE, not by NAME, and that is a decision with
a real alternative.** A toponym family that a source names several ways —
`apollonia illyria`, `apollonia pontike`, `apollonia kyrenes` and three more —
collapses to a single entry, `apollonia`. Where the qualified form is the only
form a source offers, the qualified form *is* the entry: `lokroi epizephyrioi`,
`antiocheia maiandros`, `arsinoe kyprou`, `euxeinos pontos`, `seleukeia
kalykadnos` all ship, because there is no bare name for them to collapse into.

The alternative was one entry per **name**, keeping `apollonia illyria` and
`apollonia pontike` as two distinct draws. It was available and would have
yielded a larger list — six extra entries from `apollonia` alone, six from
`herakleia`, four from `chersonesos`, and so on. It was rejected because the
spec's dedup screen asks for "one entry per place", and six spellings of one
Illyrian colony is one place. Shipping all six would pass a naive string
comparison while violating the screen in substance, which the spec names as a way
to fake a full list.

**The cost is honest and worth stating**: the reading taken is what makes 1,024
a thin margin rather than a comfortable one. The looser reading would have
cleared the target easily — which is exactly why it is the wrong reason to
choose it.

*This rule lived only in `tmp/places-census/collapse-map.txt`, which is
gitignored, so the decision would have vanished at merge along with the
alternative it ruled out. Recorded here for that reason.*

**The enumerated cut list accounts for 61 of those 107, not all of them**, and
that gap is stated rather than papered over. This entry presents the margin as
"the evidence the rule was followed", so the 46 unenumerated cuts are exactly
where that evidence thins out. What can be said honestly: the list was cut down
from a larger pool rather than padded up, which is the direction the spec cares
about and which the shipped-versus-yield counts do establish. What cannot: that
every individual cut has a written justification. A reviewer wanting to audit the
place list has 61 reasoned cuts and 46 that were made without one being recorded.

**The noun list's 868 cuts include 24 that matter structurally**: qualified forms
like `zenon kitieus` and `straton lampsakenos`. A source supplying named
historical Greeks supplies them already attached to a place, and such an entry
would render *pensive zenon kitieus of lampsakos* — two places on one name, with
no way for a reader to tell which the place slot supplied. The bare names stay,
so no figure is lost. **With the denylist deleted (D5), the ` of ` spelling rule
is the whole of what handles this family**, which is why it is the one rule that
survived: it catches the shape at the entry level, where it is checkable against
the shipped list, rather than at the pair level where completeness was a claim
about the world. Eight Roman/Latin entries (`lucretius`, `agrippa`, `antoninus`,
`quintus`, `rufus`, `musonius`, `josephos`, `aelianus`) were also cut: the spec's
source is any attested ancient *Greek* noun.

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

### D13. The pins are computed off-implementation, by a program that does not link the crate

`examples/pin_name.rs` takes a name digest, reads the three wordlists from the
**text files** in `wordlists/`, does the index arithmetic from the scheme's
written definition, and prints the name. It deliberately does not `use
dialectica_core::names`: a pin read back from `display_name` would agree with
whatever `display_name` did, which is this repo's named test defect — an
assertion whose two sides come from one source.

So three independent routes must agree on every pinned name: that program, the
by-hand index arithmetic in
`the_pinned_name_is_derivable_by_hand_from_the_pinned_digest`, and the
derivation itself. A change to the separator, a wordlist entry, a list's order,
which bytes a slot reads, or the connector breaks that agreement loudly.

**The re-pin after the denylist came out is what this bought.** Every pinned name
had to be re-derived, because a name previously produced by a redraw would now
come from its first draw. Running the implementation and copying its output would
have converted every pin into a tautology in one pass. As it happens
`PINNED_NAME_FOR_KEY_4`, `_5` and the colliding pair were all unchanged — their
first draws had never been refused — but that was a *finding*, not an assumption,
and the two span pins added for the spec's new "span the lists" requirement were
both wrong on first guess and corrected against this program.

*This entry previously described a generator for the noun–place denylist and its
`attributions.txt` source. Both are deleted with the denylist (D5); the
completeness claim that made the table the change's largest uncheckable risk is
gone with them, which is one of the deletion's better consequences.*

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

- **An identity may derive to a real historical figure's canonical name, and
  there is no rotation.** *straton of lampsakos* is reachable, and whoever draws
  it signs every post with it. → **Accepted by owner ruling** (D5): the
  coincidence is not a harm, because the system asserts nothing about a name's
  bearer and the address is the identity. What was given up to accept it was a
  199-pair denylist whose completeness was a claim about the world that no test
  could check — the change's largest uncheckable claim, now deleted rather than
  mitigated. A shared table that could drift between peers and rename somebody
  was a worse risk than the one it addressed.

- **46 of the place list's 107 cuts have no recorded justification.** → Stated in
  D11 rather than left to be discovered. The direction of the cut is what the
  spec requires and it is established by the counts; the individual reasons are
  61-of-107 complete, and a reviewer auditing that list should know which half
  they are reading.

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
  trade is a word off every rendered attribution.

- **No reply carries the derivation's input, so no caller can yet render a
  name.** This capability exposes the derivation and forbids the name on the
  wire (D8); what it does not do is put the public key on a feed row, which is
  the change that would let a view actually call it. → **A recorded gap rather
  than an oversight.** The fix touches the `author` field's contract and several
  merged specs, so it is filed as its own piece. Until it lands the derivation is
  reachable and unused on the feed path, and `docs/UI-BRIEF.md` obligation 6
  states the obligation the view is owed.

- **No test asserts the six domain separators are pairwise distinct.** → D2
  records this, including that the entry's own former justification cited a check
  that does not exist. The separators are distinct today, verified by hand; the
  property is enforced by nothing, and `cargo mutants` cannot reach a `const`.

## Migration Plan

None. Nothing has been derived under any earlier version of this scheme, no mark
has been persisted, and no name has ever been returned. The scheme version in
`NAME_PREFIX` is `1` and exists for the *next* change, not this one.
