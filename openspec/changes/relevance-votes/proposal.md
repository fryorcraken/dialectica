# Count votes now, and name the date that stops being safe

## Why

§7.2 rule 2 ships `new` and `active` and **no score at all**, because "with no
sybil resistance (§7), a vote-weighted score is not a relevance signal — it is a
dial the cheapest attacker turns." Rule 3 goes further: when a credential does
arrive, count **only** credentialed votes, because "the obvious approach — count
all votes, add a bonus for credentialed ones — leaves minting identities the
cheapest available lever."

Those rules are right about the **end state** and this change does not disturb
them. What they never addressed is the **interval before it** — a forum with no
transport, no discovery and no users, where the attack rule 2 refuses to enable
has nobody to launch it. Rule 2 reasons entirely from the adversary's cost and
never prices the alternative: a forum whose only orderings are "newest" and
"most recently replied to" is a firehose with two sort buttons, and §1 says a
forum that cannot shape what it surfaces is not a forum.

Two facts have also changed since the rules were written.

**A sybil-proof credential now exists, and rule 3 did not consider it.** The
`moderation-resolution` capability derives a Stoa's moderator set from its
genesis record, where the creator's key sits inside the address preimage.
Minting identities does not mint a moderator. Rule 3 waits on RLN for a
*voter* credential and is correct to; but a *moderator's* vote is credentialed
today, by a check already specified and already verified on every read.

**Rule 1 makes a wrong score cheap to unwind** — a score is a local projection,
never an op. This change verifies that claim rather than quoting it, and finds
it holds for the scorer and **fails for one thing the scorer does not own**: the
age input decay needs. That gap is this change's main finding and it has a
deadline, because the projection schema is being designed now.

## What changes

- **§7.2 rule 2 is rewritten.** It no longer says "no score at all". It ships a
  score built from votes, under a claim narrow enough to be true: this is an
  **engagement ordering, not a relevance signal**, and it is safe only for as
  long as a named condition holds.
- **§7.2 gains rule 6, the expiry trigger.** "The forum won't get spammed just
  yet" is a statement with an expiry date and nobody has named it. A staged
  decision without a named trigger is a permanent decision nobody admitted
  making. Rule 6 names four observable conditions, any one of which retires the
  interim score.
- **§7.2 rule 3 is narrowed, not overturned.** It remains the end state. What it
  gains is the distinction the interim needs: a credential that **cannot be
  minted** may weight a vote today; a credential that can be **re-presented** may
  not, and that is still what RLN is for.
- **Rule 4 is extended to cover a case it did not anticipate** — a moderator's
  *downvote*, which is a ranking nudge wearing moderation's authority.
- **A spec delta is warranted** (`relevance-ordering`), because the scorer is
  behaviour a reader depends on and two peers must agree on it given the same
  ops. What is *not* specified is the arithmetic: the constants live in the
  scorer where rule 1 can change them without a spec change.

## What this is not

It is not a claim of sybil resistance, and the wording is chosen so that nobody
can later read it as one. §7 refuses that claim; so does this.

It is not a decision that votes become ops — they already are, merged, both
directions.

It does not schedule the credential-gated end state. It schedules the **check**
that says the interim has expired.

## Impact

- `docs/PLAN.md` §7.2 — rules 2, 3, 4 rewritten; rule 6 added; the reserved
  shape corrected.
- New capability `relevance-ordering` (delta only; not merged here).
- **The projection schema must reserve three columns before it is written.**
  See `design.md` §1. This is the only part of this change with a deadline.
