# A name for a key: three words, derived, and never sent

## Why

Every screen that attributes a post needs a name, and core has none. `grep` for
`wordlist`, `adjective`, `generated_name` or `display_name` across
`dialectica/rust-lib/dialectica-core/src/` returns nothing but two test comments.
Meanwhile `dialectica-ui/src/qml/PostHeader.qml` declares
`property string generatedName` and `FeedScreen.qml` passes `generatedName: ""`
into it, because there is nothing to pass.

**The reason it is missing is a false claim recorded in core.** `feed.rs`
documents the feed row's author field as:

> **An address and never a name.** There are no names in core — the generated
> name is a pure function of this address and is the view's to derive.

The second half is right and the first is wrong. The derivation *is* the view's
to invoke — but a name is a pure function of the **public key**, not of the
address, so a caller holding only an address cannot invoke anything. There is no
scheme in core for it to call, and no key on the row for it to call one with.

Deriving from the key rather than the address is deliberate and is not an
accident of which value was convenient: a name tracks **the key that signs**,
which is what a reader is actually being shown. The address is a hash of a
genesis *record* kept extensible against a future key log, so deriving a name
from it would pre-answer "does the name change under rotation?" by accident.

## What Changes

**Owner ruling, and the whole of the scope: define three words derived
deterministically from an author's public key. Nothing else. The three words do
not go over the wire.**

- **A derivation from a public key to a three-word name**, in the form
  *pensive aporia of lampsakos*. A fixed 32-byte domain separator, distinct from
  every address prefix, hashed with the key; three disjoint 16-bit draws over
  bytes `0..6` select an adjective from 8,192, a noun from 1,024 and a place
  from 1,024, joined by the fixed connector `of`. Deterministic, stateless, and
  identical on every peer forever.

- **A name is never sent over the wire.** Not on a feed row, not on a thread
  item, not on an onboarding slate. It is derived by whoever holds the key, at
  the point of rendering. A derived value beside the material it derives from is
  two values that must agree and could disagree, with no way for the recipient to
  tell which is wrong; a name on the wire is also one a relay could strip or
  forge, which determinism exists to make impossible.

- **Core exposes the derivation, and that becomes the only way a name is
  obtained.** This matters more under the ruling, not less: the QML sandbox
  denies the view the network and the filesystem outside its plugin directory, so
  a view holds none of the wordlists. Without a core call, a second
  implementation of a consensus-critical scheme gets written in QML, which is the
  silent-divergence failure the pinning requirements exist to prevent.

- **Nothing filters a drawn name.** Every draw is one unconditional reduction:
  no denylist, no refused combination, no retry, no second draw. The derivation
  is total over well-formed keys, consumes a fixed six bytes, and reaches the
  `2^33` space exactly. **Owner decision: no filter on any word and no filter on
  any output.**

- **Three curated wordlists** — 8,192 English adjectives, 1,024 ancient Greek
  nouns, 1,024 Greek places real and mythological. **Exactly three screens apply,
  all mechanical**: ASCII-transliterable, deduplicated, and attested. A word is
  never kept out of a list for what it means, names or connotes. The lists and
  the scheme are versioned together and effectively frozen: removing one word
  reindexes the list and renames every identity that drew at or after it, on
  updated peers only.

- **The noun slot is any attested ancient Greek noun**, across four pools on equal
  footing: abstractions, named historical Greeks, mythological figures, and
  ordinary concrete nouns. An earlier reading of the slot as "the vocabulary of
  Greek thought" was a narrow technical vocabulary; the last two pools were absent
  from it and are the larger half.

- **No noun entry may carry the connector as a word**, so that no name renders as
  *pensive zenon of kition of lampsakos*. This is a rule about one literal
  substring, checkable against the shipped list, and it is what the widened noun
  slot makes reachable: a source supplying named Greeks is liable to supply them
  already qualified by a place. **It is the only rule in this change that keeps
  anything out**, and the bare form — `zenon` — stays in the list and draws
  normally.

- **The false comment in `feed.rs` is corrected**, because a spec that contradicts
  a doc comment loses to the comment for the next reader of the code.

**Cut by the ruling, and previously in this change:**

- **The feed row does not gain a `displayName`.** The requirement that a reply
  reporting an author carry either the name or the key is gone entirely, along
  with every scenario about what a feed row or any other reply carries. The
  contradiction with `thread-read` that requirement created dissolves rather than
  being narrowed: `thread-read` says no item carries a display name, and now
  nothing does. Both capabilities agree without either being amended.

- **What a reply owes is therefore the derivation's input**, and that is each
  reply's own capability's business rather than this one's. `thread-read` and
  `identity-onboarding` already ship the public key. **The feed read does not**,
  and closing that is filed as its own issue rather than smuggled in here — see
  *The address, and the issue this raised*.

**Moved out of this change's spec delta, because nothing implements them:**

Both were written into the spec during this change and neither was reached by the
implementing pass. They are out of the delta rather than struck through, so that
archiving this change promotes only behaviour the code actually has — a
requirement in the live contract that no implementation answers is a contract
nobody can trust, and a spec-test reviewer reading it finds a gap that is really
a bookkeeping error.

- **Every noun and every place carries a one-sentence gloss** — the requirement
  and all five of its scenarios. `grep -rn "gloss"` across `dialectica/rust-lib/`
  and `dialectica-ui/` returns only the wordlist entries `glossa` and `glossless`.
  It is ~2,048 sourced glosses, a curation pass on the scale of the wordlists
  themselves. Tracked as issue **#82**, which records the shape the spec had given
  it so the requirement can be restored to a delta by the change that builds it.

- **Core exposing the derivation to a caller** — the sentence *"Core SHALL expose
  a way to derive a display name from a public key"* and the scenario that a name
  is returned on request. Core has `display_name(&PublicKey) -> DisplayName`, but
  no wire method reaches it. Tracked as issue **#81**.

  **Only the positive half moved.** The prohibition that requirement also carried
  — that no reply carries a display name, in any shape — **is implemented and
  tested**, and stays, renamed to *The name SHALL NOT travel*. The feed reply's
  key set is asserted exactly, so a restored `displayName` fails on an added key
  rather than passing quietly, and `FeedRow` is destructured exhaustively in
  `feed.rs` and the e2e test, so re-adding the field stops them compiling.

## Capabilities

**New Capabilities**

- `generated-names` — the derivation from a public key to a display name: the
  domain separation, the byte budget, the list sizes, the screens, the determinism
  contract, and what a name may never be used for. Named for what it defines
  rather than for any surface: nothing here is about a reply.

**Modified Capabilities**

None. Under the ruling this change alters no other capability's replies, which is
the whole difference from its previous shape.

## Impact

- New wordlist data and a new derivation module in
  `dialectica/rust-lib/dialectica-core/src/`.
- `feed.rs` loses a false doc comment. **It gains no field**, which is the change
  from the previous shape of this proposal.
- **10,240 curated words are the bulk of the work**, and they are curation rather
  than design. The sources and the three screens are fixed. The job is looking
  words up — tedious and checkable rather than a judgement per entry. The ~2,048
  glosses are a comparable pass and are **not** in this change; see issue #82.
- `docs/PLAN.md`'s section on what an identity is called sheds its derivation,
  byte budget, arithmetic and list sizes here, and its reasoning to `design.md`.
  What stays there is what is not built, plus the threat-model conclusion about
  grinding and the rendering obligations, which are about the product rather than
  about this derivation.
- `docs/UI-BRIEF.md`: Obligation 6 needs no correction — it already states the
  position the ruling adopts, that core hands the view an address and a public key
  and the view derives, and already records the feed read as a known gap rather
  than a design decision. **Its gloss paragraphs do need correcting**, and this
  change makes it: they asked a designer to design an affordance around a per-word
  gloss that no longer ships here, and a brief is designed against rather than
  merely read, so a stale one costs work that has to be thrown away.

## The word count: third recorded position, and why this one

**The shape is three drawn words — adjective + noun + `of` + place — on the
owner's decision.** The count has now taken three recorded positions and all
three stay visible, because the reason each moved is more useful than the
conclusion:

1. **An early draft: three words**, at 256 adjectives and 512 nouns.
2. **PR #22 (merged): four words.** It overturned the first on arithmetic that
   was entirely correct — at those list sizes, three words give
   `256 x 256 x 512` = 2^25 and a **31%** collision probability for 5,000
   identities in one Stoa, which fails any bar. A fourth word was the only lever
   the constraints then in force left available.
3. **Three words again, now adopted** — PR #27's shape, on the owner's decision.
   Not a return to position 1: it reaches a *larger* space than position 2 with
   one word fewer.

**What moved was a premise, not a calculation.** The four-word case rested on
capping the adjective list at 256 to hold a uniform Greek-adjacent register. That
cap was self-imposed — nothing in the design required a Greek adjective — and
withdrawing it moves the slot from 2^8 to 2^13. PR #27's own summary is the part
worth keeping: *"the arithmetic is sound and the premise is a preference wearing
the costume of a fact."* The register did not disappear with the cap; it moved to
the *X of Y* shape and the two Greek words in it, so *luminous stasis of delos*
still does not read as a gamertag.

**I re-derived every figure by hand rather than taking them from #27**, and all
of them hold. With `S = 2^33 = 8,589,934,592` and
`1 - exp(-k(k-1)/2S)`: at k=5,000, `k(k-1)/2 = 12,497,500`, ratio
`1.45491 x 10^-3`, less `x^2/2 = 1.06 x 10^-6`, giving **0.145%**; at k=10,000,
`49,995,000 / S = 5.81949 x 10^-3` less `1.693 x 10^-5` giving **0.580%**; at
k=1,000, **0.0058%**; at k=100, **0.0000576%**. The doubling accounting also
holds: adjective +5, second-adjective-to-place +2, noun +1 is eight doublings,
and `2^25 x 2^8 = 2^33`. **These are unaffected by the widened noun slot**, which
changes what the 1,024 nouns are and not how many there are.

**These figures now hold exactly rather than approximately.** With no refused
region and no retry, every one of the `2^33` combinations is reachable and
equally likely, so the birthday arithmetic above describes the shipped scheme
without a correction term.

**So three words costs twice the collision rate of four (0.145% against 0.073%)
and buys a word off every feed row.** That is the trade, stated plainly; it is
not a free improvement.

## Byte disjointness across the three channels

The requirement asked for is "no byte overlap between the name, the mark, and the
head/middle/tail bytes the abbreviation displays". Working it through splits it
into one half that is **vacuous** and one half that is **real and was violated**,
and the two must not be stated the same way.

**Vacuous half: the name against the other two.** The name is derived from
`H(NAME_PREFIX || public_key)`. The mark and the abbreviation both read the
**address**, `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key)`. These are
different digests, so byte 3 of one and byte 3 of the other are unrelated values
and **there is no shared space in which they could overlap**. No allocation of
address bytes to the name is required, or even possible. That half is stated
inside the domain-separation requirement, where it belongs, rather than as a
disjointness rule.

This matters because the opposite claim was live in the code.
`dialectica-ui/src/qml/Identicon.qml` said *"Bytes 0..11 are reserved for the
generated-name scheme. That disjointness is load-bearing"*. **It was not, and the
mechanism it described does not exist** — PLAN.md already corrects exactly this
("both documents independently invented the same shared-digest story"), and the
correction had never reached the QML.

**Real half: the mark against the abbreviation.** Both read the same 32 bytes,
and they overlapped on `{14, 15, 16, 17}` — half of what the mark read was
already on screen in the middle group. `Identicon.qml` admitted this in full and
called it "a weaker version of the criticism this design makes of the bundle's
original, reduced rather than eliminated". This change eliminated it: the mark
moved from `12..19` to `4..11`, which is disjoint from all three abbreviation
groups and sits entirely inside the region the abbreviation hides. The 8-8-6
shape and the vanity-defeating middle group are untouched, which is the property
to preserve.

**The one requirement in this spec that is not about the derivation is the
contract for that move**, and it is kept because the move has shipped on this
branch and there is no identicon capability for it to live in. Under a strict
reading of the ruling it does not belong here; leaving shipped behaviour with no
spec anywhere is the worse of the two errors, so it stays and is flagged rather
than deleted quietly.

## The address, and the issue this raised

Working this change is what surfaced a question about the wire contract that is
too large to answer inside it: **why core sends an author address at all, when it
holds the public key.**

`Op` carries `author: PublicKey`. The address is
`SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key)` (`identity.rs:266`) — a
lossy one-way projection of a value core already holds — and core computes it and
sends that instead, in `feed.rs:300` and `thread.rs:628`. A holder of the key can
derive the name, the mark, and the address itself; a holder of the address can
derive only the mark. That asymmetry is exactly what forced the previous shape of
this change to render names in core, and it is why `feed.rs` held a key, called
`.address()`, and discarded the key one line later.

Filed as its own issue rather than acted on here — it touches `feed.rs`,
`thread.rs`, `keystore.rs`, `op.rs`, the wire contract's `author` field, the
`AddressLabel` QML component and several merged specs.

## A defect in the design bundle, for whoever writes the onboarding view

The bundle says "three words", which the adopted shape makes **accidentally
correct on the count and still wrong on the shape**. That is more dangerous than
a plain error: the number now checks out, which invites the rest to be trusted.

- **`tmp/ui-design/handoff/copy.json`, key `onboarding.apparatus.uniqueness`** —
  "Someone else in this Stoa may hold the same three words." The count is right.
  **Verify rather than assume the rest of the sentence**, since it was written
  before this decision and arrives at the right number by a different route. The
  sibling `ui-onboarding` spec deliberately paraphrases this apparatus string
  rather than pinning it verbatim, so any correction happens where it is used.

- **`tmp/ui-design/handoff/reference/reference-design.dc.html`** renders
  `vermilion patient sandworm` and `slow cobalt lamplighter` on its identity
  slate. These are **three-word names from a withdrawn science-fiction draft**.
  They are three words, so they now pass a word count, and they remain the wrong
  shape entirely: no `of`, no place, no Greek. **The correct form is *pensive
  aporia of lampsakos*.** Treat that screen as a layout reference and never a
  content one.

- **`copy.json` line 4** claims the key "cannot be linked to you anywhere else",
  which PLAN.md §5.2 forbids saying of the MVP. Not this piece's to fix, but it is
  evidence for how much of that file needs checking rather than trusting.

## Sequencing

**`dialectica-ui/` is untouched by this change except for the identicon window
move.** Rendering a name is a view's work and belongs to the UI pieces in flight;
this change makes the derivation exist and reachable.

**Nothing in this change now waits on another.** The cross-capability
contradiction with `thread-read` is gone with the requirement that created it,
and the feed's missing key is an issue rather than a dependency.
