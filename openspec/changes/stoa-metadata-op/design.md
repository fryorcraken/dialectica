# Design

## Context

See proposal.md — Why. `op.rs` defines a signed envelope with four kinds and a
canonical encoding whose kind byte is inside every signature. `stoa.rs` defines
the immutable genesis record. Nothing connects them: a Stoa has a founding title
and no way to have a current one.

PLAN.md §5.7 supplies the shape — moderator-owned, last-write-wins, falling back
to the genesis values — and leaves one question open, which this change answers.

## Goals / Non-Goals

**Goals**

- One op kind for a Stoa's mutable display metadata, in the existing canonical
  format, with the same hostile-input posture as every other decoder here.
- A definite answer to §5.7's open `policy` question, with the alternatives and
  what ruled them out.

**Non-Goals**

- **No resolution.** "Prefer the latest valid op, fall back to genesis" needs an
  ordering, and ordering is the `op-ordering` capability's — `arrival::cmp_ops`
  applies §5.7's rule, with a defined degraded order for ops the transport did
  not order. Resolution is a store concern and there is no store; this change
  defines the op and states the rule, and implements neither.

  Written when that ordering was still an open question, this non-goal was
  originally justified by the gap. The gap is now characterised rather than
  open, and the non-goal survives it for a simpler reason: nothing here has
  anywhere to put a resolved title.
- **No moderator-set check.** Authority needs the moderator set at the op's
  position in the order; the type does not have it and §3.3 puts the check on
  read.
- **No store.** Nowhere for a resolved current title to live yet.
- **No policy mutation.** The decision below.

## Decisions

### A metadata op does not carry `policy`

This is §5.7's open question — *"whether a metadata op may change `policy` as
well as the display fields"* — and the answer is no, for this op, now.

**Alternative A: one op carrying title, description and policy.** Rejected on
three counts, the first of which is the one that would have bitten in
production.

The reader rule §5.7 gives is *prefer the latest valid op and fall back to the
genesis values*. That rule is well-behaved for a display field and dangerous for
an authorisation field, and the asymmetry is not a matter of degree. A peer that
has not yet received the latest metadata op — one that joined late, one whose
SDS-Repair window did not reach back far enough (§13 flags exactly this as
unmeasured), one that was simply offline — falls back to the genesis value. For
a title that shows a stale name. For a policy it means a Stoa that tightened
from `open` to token-gated is treated as `open` by every peer that missed the
tightening. That is the same silent widening `stoa.rs` refuses when it declines
to default an unknown policy discriminant, arriving by a different door:

> NOT defaulted to Open. Treating an unrecognised policy as open is how a
> token-gated Stoa silently becomes world-postable on an older client.

A design that refuses to default a policy on decode, and then hands one back by
fallback on resolve, has closed the front door and left the back one open. The
fallback direction is only safe when missing the update is *recoverable*, and
"posted to a Stoa that had excluded you" is not.

Second: a rename is cosmetic and a policy change is retroactive. Tightening a
policy alters who may post, for everyone, and reaches backward over content
already published — a materially different act from changing a display string,
and one that warrants its own audit trail, its own op kind and quite possibly
its own authority rule (§6.2's threshold certificate is the obvious candidate:
a rename by one moderator is cheap to correct, a policy tightening by one rogue
moderator key is not).

Third, and decisive for *shipping it today*: `Policy` has exactly one accepted
variant. A policy field in this op could only ever carry `Open`, so "a metadata
op changed the policy" is a scenario that cannot be exercised through the API.
The agents README is explicit that a field with one variant cannot be varied and
that a scenario which cannot be tested must not be written. The field would
arrive untested, untestable, and carrying the fallback hazard above.

**Alternative B: a separate `SetPolicy` op kind, now.** Rejected on the same
third count — it is the *right* eventual shape and it is equally untestable
today. Nothing is lost by waiting, which is the next decision.

**Alternative C: omit it, which is what this change does.** The cost of omitting
is what had to be checked, because the genesis record's own history is the
cautionary tale: §13 records that `policy` landed in the genesis record early
specifically because "adding the field later would have changed the address of
every Stoa already created". That pressure does **not** transfer here, and the
reason is structural rather than lucky:

- A genesis record is hashed to produce a Stoa's *address*, so every field is
  address-determining and none can be added after the fact.
- An op's id is the hash of that one op. Adding a new op kind changes the id of
  no existing op, the address of no Stoa, and the encoding of nothing already
  written.
- `op.rs` allocates kind discriminants explicitly and appends rather than
  inserts, so a future policy-changing act takes **the next free value**, which
  is whatever `OpKind`'s constants say it is at the time. An older client
  meeting an unallocated discriminant gets `OpError::UnknownKind` — the legible
  refusal the version byte exists to give — rather than a misparse.

  Deliberately not naming the number here: if another kind lands first, a
  sentence saying "discriminant 5" goes stale while every test still passes.
  The property that matters is "appended, never inserted", which stays true
  however many kinds arrive.

So the reservation is free and requires no marker in the format to keep it free.
What would *not* be free is landing a `policy` field inside this op's encoding
and later needing to remove or re-mean it: that is a change to an existing
kind's layout, and it would invalidate every metadata op already signed.

**What would have to be true to add it.** Three things, all checkable:

1. `Policy` has a second accepted variant, so a change is expressible and a test
   can vary it.
2. Metadata resolution is actually built on `arrival::cmp_ops`, so "the latest
   policy op" is a determinable thing rather than a phrase. The ordering itself
   is settled — that was open when this was written and is not any more — but a
   rule nothing implements still cannot be relied on to gate posting.
3. The fallback rule for policy is specified *separately* from the display
   fallback, and specified as fail-closed: a peer that cannot establish the
   current policy treats the Stoa as more restrictive than genesis, not less —
   or declines to post rather than guessing. Until that rule exists, the honest
   position is that policy is immutable, which is exactly what the genesis
   record already says.

### Where the requirements live: split, not folded and not standalone

Asked as a binary — fold into `op-format`, or keep `stoa-metadata` standalone —
both answers are wrong, and the requirement text says so more clearly than any
argument about concepts.

Written standalone, five of seven requirements were **per-kind restatements of
general `op-format` requirements**: canonical encoding, the kind inside the
signature, authenticity-not-authority, the malformed-input surface, and the
no-ordering-field rule. `op-format` states each of those for *all* kinds, some
emphatically — "whatever the two have in common". A narrowed copy adds no
constraint and rots in the worst direction: a later change to the general rule
leaves the per-kind copy quietly contradicting it, with both validating.

Written folded, the two requirements that are *not* wire facts would have been
buried in a wire-format spec: that a Stoa has founding values and current ones
and both stay answerable, and that the policy is excluded. Neither is a fact
about the envelope, and no other kind has anything like them.

So: wire facts to `op-format`, concept to `stoa-metadata`.

**The amendment to `op-format` was not optional either way.** Its requirement
"An op is the unit that crosses the wire" enumerates the kinds closedly, and
there are now five. Leaving that sentence saying four would have made a merged
spec false the moment this landed — which is the concrete answer to "does a new
op kind touch `op-format`?": yes, always, at minimum there.

**Rule for the next kind's author**, so this is not re-derived: a new op kind
amends `op-format`'s kind enumeration and puts its encoding facts there. It
earns its own capability only for propositions that are not about the envelope
— a reader rule, a relationship to another record, a deliberate exclusion. If
the draft requirements read as `op-format`'s with one kind's name substituted
in, they belong in `op-format`.

### The extraction that was declined, and why it stays declined

A general "a record has a canonical encoding" capability, extracted from
`stoa-genesis` and `op-format`, was evaluated when `op-format` landed and
declined. Having now written a second set of encoding requirements, this change
agrees, and adds one observation the first evaluation could not have:

the overlap that *is* real turned out to be between `op-format` and **this
change**, not between `op-format` and `stoa-genesis` — and the right response
to it was to put the requirements in `op-format`, which needs no new capability
at all. The generality that looked like it wanted extracting was an op-format
generality all along, and `op-format` already expresses it.

`stoa-genesis` remains genuinely different: self-identifying by hash, no field
cap, no presence tags, no signature envelope, and versioned independently. Two
formats that version together mean a change to one invalidates the other for no
reason, which `op-format` now states as a requirement.

### The op carries a description, which the genesis record does not have

§5.7 names the metadata as "title, description, policy". `stoa.rs` carries a
title and no description, deliberately — a description is not identity and has
no business in an address preimage, where it would make every re-wording mint a
new Stoa.

Carrying it here rather than nowhere is what makes this op a metadata op rather
than a title-override op. It also keeps the two records honestly asymmetric: the
genesis record is the minimum needed to *identify* a Stoa, and the metadata op
is what a reader needs to *render* one. A future display field (a topic, a
pinned notice) joins this op and disturbs nothing.

### The kind byte, not a separate format

The new kind reuses `Op`'s encoding wholesale — same version byte, same kind
byte at offset 1, same Stoa and author fields, same length-prefix and cursor
helpers. This is not merely convenient: it is what puts the kind inside the
signature, which for a *moderator-signed* op is load-bearing. A metadata op and
a moderation op are both acts by the same key in the same Stoa naming things;
without the kind byte a signature over one is a signature over the other.

`a_signature_over_a_metadata_op_does_not_verify_as_a_moderation` builds that
case directly, mirroring the existing `a_signature_over_one_kind_does_not_verify
_as_another`.

### Discriminant 4, appended, never reordered

`OpKind`'s discriminants are explicit constants precisely so they do not follow
declaration order. `STOA_METADATA` takes 4, the next free value. Inserting it
anywhere in the used range would re-mean every op already signed.

## Risks / Trade-offs

- **A Stoa can be renamed by anyone, as far as this type is concerned.** →
  Deliberate and pinned by a test. Authority is the store's question, and
  `verification_answers_authenticity_and_not_authority` already establishes the
  precedent for moderation. The risk is not that the code is wrong but that a
  reader takes `verify() == true` for "this rename is binding"; the test and the
  doc comment exist to stop that reading.
- **A metadata op has no effect until resolution exists.** → It accumulates in
  the op log, exactly as votes do (§13: "collected and read by nothing"). The
  format landing early is what lets resolution arrive without a version bump.
- **`policy` omitted may look like an oversight later.** → Recorded here and in
  PLAN.md §13 as a decision with the conditions for revisiting it, rather than
  left as silence.
- **Two variable-length fields adjacent.** → The concatenation trap, closed the
  same way `stoa.rs` and the other op kinds close it: length-prefix both, and
  test the split-point case rather than assuming it.

## Open Questions

None that belong to this change.

The one it touches and does not close — ordering — was open while this was
written and has since been settled by the `op-ordering` capability:
`arrival::cmp_ops` applies §5.7's rule, and ops the transport did not order fall
back to a defined degraded order identical on every peer. That does not make
resolution this change's to build; it means the thing resolution was waiting on
now exists, and a store is what is still missing.
