# A name for a key: four Greek words, derived, and returned beside the address

## Why

Every screen that attributes a post needs a name, and core has none. `grep` for
`wordlist`, `adjective`, `generated_name` or `display_name` across
`dialectica/rust-lib/dialectica-core/src/` returns nothing but two test comments.
Meanwhile `dialectica-ui/src/qml/PostHeader.qml` declares
`property string generatedName` and `FeedScreen.qml` passes `generatedName: ""`
into it, because there is nothing to pass. Every post in the feed is attributed
to a 64-character hex string.

**The reason it is missing is a false claim recorded in core.** `feed.rs`
documents the feed row's author field as:

> **An address and never a name.** There are no names in core — the generated
> name is a pure function of this address and is the view's to derive.

Both halves are wrong. The name is a pure function of the **public key**, not of
the address; and a view holding only an address therefore **cannot** derive it.
`docs/UI-BRIEF.md` obligation 6 already carries the correction — "a view holding
only an address **cannot** compute the name itself, and core must return the
rendered name alongside the address" — but the comment it corrects is still in
the code, and it is the reason no one added the field.

Deriving from the key rather than the address is deliberate and is not an
accident of which value was convenient: a name tracks **the key that signs**,
which is what a reader is actually being shown. The address is a hash of a
genesis *record* kept extensible against a future key log, so deriving a name
from it would pre-answer "does the name change under rotation?" by accident.

## What Changes

- **A derivation from a public key to a three-word name**, in the form
  *measured aporia of lampsacus*. A fixed 32-byte domain separator, distinct from
  every address prefix, hashed with the key; three disjoint 16-bit draws select an
  adjective from 8,192, a noun from 1,024 and a place from 1,024, joined by the
  fixed connector `of`. Deterministic, stateless, and identical on every peer
  forever — a name is not published and cannot be, because a published name is
  one two peers could disagree about.

- **A true-attribution denylist on noun–place pairs.** The *X of Y* shape can
  spell a real figure's canonical name — *straton of lampsacus* — which would sign
  a user's every post with a real person's identifier. About 960 forbidden pairs,
  0.092% of draws, roughly 4.6 identities per 5,000. Only the pair is refused;
  both words stay in their lists.

- **The feed row gains a `displayName` beside its `author`.** This is the crux of
  the change. `list_threads` returns an address; the name comes from a key; so
  the view is stuck unless core closes the gap. It can: the signed op carries
  `author: PublicKey`, and the feed row's address is a lossy projection core
  computes from a key it already holds. Core renders the name at the point it
  builds the row. **The address stays** — the name is added beside it, never in
  place of it.

- **The false comment in `feed.rs` is corrected**, because a spec that contradicts
  a doc comment loses to the comment for the next reader of the code.

- **Three curated wordlists** — 8,192 English adjectives, 1,024 Greek nouns,
  1,024 Greek places real and mythological. **Exactly two screens apply**
  (ASCII-transliterable, deduplicated) plus the tone and authority exclusions;
  there is no familiarity, register, length or pronounceability screen, both
  earlier attempts at a third filter having been withdrawn. The lists and the
  scheme are versioned together and effectively frozen: removing one word
  reindexes the list and renames every identity that drew at or after it, on
  updated peers only.

- **The three recognition channels must read disjoint inputs**, and one pair
  currently does not. See below — this is the part with a possible `dialectica-ui`
  consequence.

Not changed: no reply of `identity-onboarding` gains a name. That capability
already **forbids** one — its slate carries the public key precisely so that a
name is computable by anything holding one, and its closed-field-set requirement
is what makes the prohibition checkable. Adding a name there would settle in one
contract what a separate contract settles, and it is unnecessary: the slate
already carries the input. No identicon, no rotation, no UI work.

## Capabilities

**New Capabilities**

- `generated-names` — the derivation from a public key to a display name: the
  domain separation, the byte budget, the list sizes, the denylist redraw, the
  determinism contract, and what the name may not be used for.

**Modified Capabilities**

None by delta. The feed read that gains the name has **no capability of its
own** — `openspec list --specs` shows no feed or thread-read capability, and the
`thread-read` change is in flight on a parallel branch. The obligation on the
feed reply is therefore stated in `generated-names`, as a requirement on where a
name must appear rather than on what a feed row otherwise contains. Whichever
change first writes a feed capability inherits it rather than restating it.

## Impact

- New wordlist data and a new derivation module in
  `dialectica/rust-lib/dialectica-core/src/`.
- `wire.rs`'s feed page serialisation gains one field per row; `feed.rs`'s
  `FeedRow` gains one field and loses a false doc comment.
- **10,240 curated words plus a ~960-entry denylist are the bulk of the work**,
  and they are curation rather than design. The sources, the two screens and the
  exclusion rules are fixed; the words are not written. The true-attribution
  denylist needs a canonical place for each of ~800 named Greeks, which is a
  lookup per entry rather than a judgement per entry.
- `docs/PLAN.md` §5.2.1's derivation, byte budget, arithmetic and list sizes
  become spec prose and are struck through there, pointing here.
- `docs/UI-BRIEF.md` obligation 6's parenthesis says core "must return" the name.
  After this change it does, so the obligation reads as met rather than
  outstanding.

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
the *X of Y* shape and the two Greek words in it, so *brittle kairos of abdera*
still does not read as a gamertag.

**I re-derived every figure by hand rather than taking them from #27**, and all
of them hold. With `S = 2^33 = 8,589,934,592` and
`1 - exp(-k(k-1)/2S)`: at k=5,000, `k(k-1)/2 = 12,497,500`, ratio
`1.45491 x 10^-3`, less `x^2/2 = 1.06 x 10^-6`, giving **0.145%**; at k=10,000,
`49,995,000 / S = 5.81949 x 10^-3` less `1.693 x 10^-5` giving **0.580%**; at
k=1,000, **0.0058%**; at k=100, **0.0000576%**. The doubling accounting also
holds: adjective +5, second-adjective-to-place +2, noun +1 is eight doublings,
and `2^25 x 2^8 = 2^33`. The denylist figures hold too —
`960 / 1,048,576 = 9.155 x 10^-4`, so `5,000 x 9.155 x 10^-4 = 4.58` identities
per 5,000, and a second consecutive refusal at `(9.155 x 10^-4)^2 = 8.4 x 10^-7`.

**So three words costs twice the collision rate of four (0.145% against 0.073%)
and buys a word off every feed row.** That is the trade, stated plainly; it is
not a free improvement.

## Byte disjointness across the three channels — and a decision for the owner

The requirement asked for is "no byte overlap between the name, the mark, and the
head/middle/tail bytes the abbreviation displays". Working it through splits it
into one half that is **vacuous** and one half that is **real and currently
violated**, and the two must not be stated the same way.

**Vacuous half: the name against the other two.** The name is derived from
`H(NAME_PREFIX || public_key)`. The mark and the abbreviation both read the
**address**, `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || public_key)`. These are
different digests, so byte 3 of one and byte 3 of the other are unrelated values
and **there is no shared space in which they could overlap**. No allocation of
address bytes to the name is required, or even possible.

This matters because the opposite claim is live in the code.
`dialectica-ui/src/qml/Identicon.qml:32-34` says *"Bytes 0..11 are reserved for
the generated-name scheme. That disjointness is load-bearing"*. **It is not, and
the mechanism it describes does not exist** — PLAN.md already corrects exactly
this ("both documents independently invented the same shared-digest story"), and
the correction never reached the QML. The independence is genuine but comes from
domain separation, and it would hold whichever bytes either side read.

**Real half: the mark against the abbreviation.** Both read the same 32 bytes,
verified against the code rather than the comments:

| Consumer | Address bytes | How derived |
|---|---|---|
| Abbreviation | **0..3, 14..17, 29..31** | `AddressLabel.qml`: head 8 / middle 8 / tail 6 hex chars; `start = floor((64-8)/2) = 28`, so chars 28..35 = bytes 14..17 |
| Mark | **12..19** | `Identicon.qml`, one byte per dimension |

**They overlap on `{14, 15, 16, 17}`** — half of what the mark reads is already on
screen in the middle group, so only `{12, 13, 18, 19}` of the 21 bytes the
abbreviation hides reach the reader through the mark. `Identicon.qml:36-42`
already admits this in full and calls it "a weaker version of the criticism this
design makes of the bundle's original, reduced rather than eliminated". This
change is the occasion to eliminate it.

**Proposed allocation — move the mark to bytes `4..11`:**

| Consumer | Bytes | Changed? |
|---|---|---|
| Abbreviation | 0..3, 14..17, 29..31 | **unchanged** |
| Mark | **4..11** (was 12..19) | moved |
| Name | none of the address | n/a — different digest |

`{4..11}` is disjoint from all three abbreviation groups, is eight bytes as the
mark requires, and sits entirely inside the region the abbreviation hides. **The
8-8-6 shape and the vanity-defeating middle group survive untouched**, which is
the property to preserve; moving the mark rather than the abbreviation is what
achieves that. Three slots rather than four also frees name-digest room, though
that is a different digest and so not what made this possible.

**This is a change to shipped behaviour and I have not made it.** Moving the
mark's window changes every existing mark: the same address renders a different
glyph before and after. It touches `dialectica-ui/src/qml/Identicon.qml` (which I
must not edit) and `docs/IDENTICON.md` (which documents 12..19). **Owner's
decision whether it rides in this piece or becomes its own change.** If it becomes
its own, the disjointness requirement in this spec is the contract it implements,
and it should land before any mark is treated as stable.

Two further things whoever takes it must do: correct `Identicon.qml`'s
"bytes 0..11 are reserved for the generated-name scheme" comment, which is false
whatever is decided about the window; and note that the mark's own reasoning for
reading eight bytes ("the perceptual space is the binding constraint, not the
input") is unaffected by *which* eight.

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
  slate. These are **three-word names from the withdrawn science-fiction draft** —
  PLAN.md names the first by name as the register it rejected. They are three
  words, so they now pass a word count, and they remain the wrong shape entirely:
  no `of`, no place, no Greek. **The correct form is *measured aporia of
  lampsacus*.** Treat that screen as a layout reference and never a content one.

- **`copy.json` line 4** claims the key "cannot be linked to you anywhere else",
  which PLAN.md §5.2 forbids saying of the MVP. Not this piece's to fix, but it is
  evidence for how much of that file needs checking rather than trusting.

## Sequencing

**`dialectica-ui/` is untouched by this change.** Four UI pieces are in flight in
parallel worktrees; rendering the new field is theirs. This change makes the
field exist.

**One open decision blocks nothing but must not be forgotten**: whether moving the
mark from bytes `12..19` to `4..11` rides here or becomes its own change. It
changes every rendered mark and touches `Identicon.qml` and `docs/IDENTICON.md`,
neither of which this piece edits. The spec states the disjointness requirement
either way, so a separate change has a contract to implement.

The `thread-read` piece is the one that can collide: if it writes a feed or
thread-read capability, the requirement here about where a name must appear is
the one to reconcile, and it should move there rather than be duplicated.
