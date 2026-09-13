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

- **A derivation from a public key to a four-word name.** A fixed 32-byte domain
  separator, distinct from every address prefix, hashed with the key; fixed hash
  bytes select two adjectives from a 256-entry list and two nouns from a
  512-entry list. Deterministic, total, stateless, and identical on every peer
  forever — a name is not published and cannot be, because a published name is
  one two peers could disagree about.

- **The feed row gains a `displayName` beside its `author`.** This is the crux of
  the change. `list_threads` returns an address; the name comes from a key; so
  the view is stuck unless core closes the gap. It can: the signed op carries
  `author: PublicKey`, and the feed row's address is a lossy projection core
  computes from a key it already holds. Core renders the name at the point it
  builds the row. **The address stays** — the name is added beside it, never in
  place of it.

- **The false comment in `feed.rs` is corrected**, because a spec that contradicts
  a doc comment loses to the comment for the next reader of the code.

- **Two curated wordlists**, 256 adjectives and 512 nouns, drawn from Greek
  philosophy and letters, under a fixed transliteration convention and a set of
  exclusion rules, plus a denylist of refused combinations. The lists and the
  scheme are versioned together and effectively frozen: removing one word
  reindexes the list and renames every identity that drew at or after it, on
  updated peers only.

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
- **768 curated words are the bulk of the work**, and they are curation rather
  than design. PLAN.md fixes the sources, the transliteration convention and the
  exclusion rules; it deliberately does not write the words.
- `docs/PLAN.md` §5.2.1's derivation, byte budget, arithmetic and list sizes
  become spec prose and are struck through there, pointing here.
- `docs/UI-BRIEF.md` obligation 6's parenthesis says core "must return" the name.
  After this change it does, so the obligation reads as met rather than
  outstanding.

## The word count has reversed twice, and the counter-argument is live

**This spec pins four words because that is what main says.** The count is the
most contested number in the design and the record should be visible to whoever
reads this next, rather than rediscovered:

- **PR #22 (merged)** — "Make the generated name fully Greek, and four words".
  This is why main says four. It reversed an earlier three-word draft, on the
  ground that Greek-only sources could not sustain the larger lists a blended
  corpus had allowed.
- **PR #27 (closed, branch `docs/three-word-name` kept)** — "Make the generated
  name three words, and record why four was never necessary". It proposes
  adjective + noun + `of` + place at 2^13 x 2^10 x 2^10 = 2^33, giving 0.145% at
  5,000 identities.

**#27 was not closed on the merits.** The owner's closing comment says it was
closed "per the decision to build from specs rather than from these PRs", and
that the branch "is kept, so nothing here is lost if it is wanted later". So the
argument stands unrefuted and unadopted.

Its argument is worth stating because it identifies a soft premise in the
four-word case rather than an error in its arithmetic: every figure in the
four-word case is correct, but it rests on capping the adjective list at 256 to
hold a uniform Greek-adjacent register, and **that cap is a taste judgement
rather than a limit of the sources**. Withdrawing it moves one slot from 2^8 to
2^13. In its own words: *"the arithmetic is sound and the premise is a preference
wearing the costume of a fact."*

I re-derived both sets of figures by hand and they agree with main: three words
at 256/512 gives 2^25 and 31% at 5,000; four gives 2^34 and 0.073%. **So the
arithmetic does not decide this — the register rule does.** The spec therefore
states the 256 cap as a choice about register rather than as something the corpus
imposes, so that anything revisiting the count knows which premise to attack. A
change either way is a scheme version bump, which the spec already requires.

## A defect in the design bundle, for whoever writes the onboarding view

The mockup's copy contradicts the settled design on the exact number this change
pins down, and the string is written to be pasted:

- **`tmp/ui-design/handoff/copy.json`, key `onboarding.apparatus.uniqueness`**,
  says "Someone else in this Stoa may hold **the same three words**." The name is
  four words. **Correct the count wherever the string is finally used** — the
  sibling `ui-onboarding` spec deliberately paraphrases this apparatus string
  rather than pinning it verbatim, so the correction cannot happen there.

  **Treat this as a design disagreement rather than a typo.** Given the contested
  history above, copy saying "three words" may have been written against a
  three-word design rather than mistyped against a four-word one — which means
  the rest of the bundle's identity copy needs checking against main rather than
  trusting, not just this one count.
- **`tmp/ui-design/handoff/reference/reference-design.dc.html`** renders
  `vermilion patient sandworm` and `slow cobalt lamplighter` on its identity
  slate, and repeats "three words" at line 74. Those are **three-word names from
  the withdrawn science-fiction draft** — PLAN.md names that first string by name
  as the register it rejected, against *measured attic stoic*. The bundle's
  identity screen predates both the word-count decision and the Greek-source
  decision, so it is a layout reference there and not a content one.

The arithmetic was rechecked against the mockup rather than assumed: three words
gives 2^25 and a 31% collision probability at 5,000 identities in one Stoa, four
gives 2^34 and 0.073%. The conclusion survives a generous error in the one number
that is an estimate — the honest adjective count. **PLAN.md is right and the
bundle is stale.**

## Sequencing

**`dialectica-ui/` is untouched by this change.** Four UI pieces are in flight in
parallel worktrees; rendering the new field is theirs. This change makes the
field exist.

The `thread-read` piece is the one that can collide: if it writes a feed or
thread-read capability, the requirement here about where a name must appear is
the one to reconcile, and it should move there rather than be duplicated.
