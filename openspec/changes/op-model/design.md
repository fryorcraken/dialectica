# Design

## Context

See proposal.md — Why. `sign_op_bytes` and `verify_authored_op` existed in
`identity.rs` and were correct as far as they went; what was missing was any
definition of the bytes they operate on. This change supplies that, and leaves
the cryptography untouched.

It is the same shape of gap the `stoa-genesis` change closed one level down —
`stoa_address` hashed bytes nobody had defined — which is why the two encodings
come out looking alike. Whether that likeness is a capability is settled below.

**These documents were written after the code.** That ordering has a specific
hazard: it is easy to produce a spec that narrates the implementation rather
than stating a contract, since the implementation is sitting there to be
paraphrased. Two things were done to resist it. The spec is written in terms of
what a caller may rely on and what a hostile peer cannot do, naming no function
and no type. And the properties it claims were **mutation-verified** — broken in
the source, the suite run, the failures recorded, the source restored — so that
each requirement is known to be pinned by something rather than merely believed
to be. The table is at the end of this document.

## Goals / Non-Goals

**Goals**

- One canonical encoding per op, with decoding strict enough that a hostile op
  is refused rather than misread.
- A signature that commits to *which* op it authorises, closing the gap
  `identity.rs` recorded and could not close itself.
- A content-derived id available wherever the op is.

**Non-Goals**

- **No validation beyond authenticity.** Whether a moderator was a moderator,
  whether a reviser owns the post, whether a policy admits a poster — each needs
  state an op does not carry, and each is settled on read.
- **No ordering.** Ops carry no Lamport value and no message id; see below.
- **No op log, no store, no resolvers.** This is the format, not what holds it.
- **No module-boundary API.** Nothing in `wire.rs` exposes an op yet.
- **No attachment resolution.** An op carries references; fetching is elsewhere.

## Decisions

### The kind goes inside the signed preimage

**This is the change's reason for existing, and it is a correctness fix rather
than a layout preference.**

`identity.rs` uses one `OP_SIGNING_PREFIX` for every op. Domain separation at
that level distinguishes "a dialectica op signature" from a signature over
anything else, which is worth having — but it cannot distinguish a signature
over a vote from a signature over a moderation action, because both are
dialectica ops and both go under the same prefix. `identity.rs` says so
explicitly and names the fix as the serialiser's job.

Putting the kind discriminant second in the encoding, immediately after the
version and ahead of every other field, makes the separation structural: two ops
of different kinds differ in their second byte, so no signature over one is a
signature over the other, whatever else they have in common.

**Alternative considered: a per-kind signing prefix in `identity.rs`.** Give
each kind its own 32-byte prefix and the separation comes from the cryptographic
layer. Rejected because it puts knowledge of the op kinds into the identity
module, which currently knows nothing about ops beyond "some bytes were signed",
and because it makes adding a kind a two-file change with a silent failure mode
if the second file is forgotten. The encoding already has to be unambiguously
typed for decoding to work at all; making the signature ride on that costs
nothing extra.

**Alternative considered: leaving it, since kinds have different lengths
anyway.** Rejected because it is not true in general and was not true here: a
moderation op and a vote op are the *same length* and differ only in a
discriminant whose values coincide (`HIDE` and `UP` are both 0). Those two
encode identically without the kind byte. This is not a hypothetical — it is
what the regression test constructs, and it fails when the kind byte is removed.

### An op carries no ordering field

Ordering is the transport's, and an op that asserted its own would be asserting
something its author could forge. The revision rule orders by the transport's
Lamport timestamp with its message id as tiebreak, and both are assigned on
delivery; recording them alongside an op rather than inside it is what keeps
them unforgeable by the author they order.

**This is currently owed by the transport rather than supplied by it.** The
delivery contract we actually have exposes a channel message event carrying no
Lamport clock and no message id — its `timestamp` is delivery's own, whose units
differ per event. So the ordering rule has no input at the contract in hand.

That gap is real and is recorded in PLAN.md §13. It was not papered over with a
field here, because a wrong field is worse than an absent one: an op format is
immutable in the sense that matters (ops already published cannot be re-encoded),
so a field added to satisfy today's missing transport metadata would persist
after the transport supplied the real thing. **The gap blocks the op log and the
revision and moderation resolvers; it does not block the format**, which is why
the format landed without it.

**Alternative considered: a wall-clock timestamp.** Rejected: nothing in the
design calls for one, the author picks it freely, and a forum ordered by it is
ordered by a field its adversary sets.

**Alternative considered: a per-author sequence number.** Rejected for the same
reason the channel id carries no per-peer state. A value varying with one peer's
history, inside bytes every peer must agree about, is how two peers compute
different ids for one op — a failure nobody observes, because each peer stays
internally consistent.

### The op carries the Stoa address, never a channel id

Carrying the address keeps channel identity out of payloads, so that splitting
one channel per Stoa into one per thread later is a routing change rather than a
migration of every op ever published. The address is also a pure function of the
addressed object, which is the property that makes the derivation reproducible.

Putting it inside the signed bytes buys a second thing: an op lifted from one
Stoa's channel and replayed on another's fails verification, rather than
arriving as a valid post in a Stoa its author never addressed.

### The op carries no sender identifier

The transport's sender id is not an author identity and the design should not
treat it as one — it binds at channel creation as a transport self-filter. The
author identity in an op is the public key it carries and the address derived
from that. Carrying the transport's identifier too would add a second, weaker
claim of authorship for a reader to confuse with the real one.

### The author's public key travels, not just the address

Verification happens on read, and there is no directory to resolve an address
against. A peer holding only an address could not check a signature. Carrying
the key costs 32 bytes and makes every op independently checkable; the address
is re-derived from the key during verification, which is what binds the key to
the claimed author rather than letting any key vouch for any address.

### The op id is content-derived, not transport-assigned

An op id is needed where no transport envelope exists: replaying a local store,
applying a snapshot, deduplicating on ingest. An id assigned on delivery is
absent in exactly those settings, which are the ones that need it.

It is deliberately **not** the transport's message id. The ordering tiebreak
refers to that one. Two identifiers, two jobs — and naming this one distinctly
is what keeps a future reader from substituting one for the other.

The id is domain-separated under its own prefix because an op id, an author
address and a Stoa address are all 32 bytes and a moderation op names one of
them by value. Without separation, a byte string could be a valid instance of
two of them.

### One moderation kind with an action field, rather than two kinds

A threshold moderation certificate signs one `(target, action, epoch)` tuple
across several moderators' signatures. A tuple needs the action inside it; if
hide and unhide were separate kinds, the action would be spread across which
envelope arrived and could not be signed as one value.

Naming the inverse also answers a question the plan had left open. Ordering
moderation ops on the same target by last-write-wins means nothing over a set of
one, and a moderation system with no correction path makes every mistake
permanent.

### Both vote directions are carried, though the signal is upvote-only

The measured comparable project collected downvotes, attached them to posts, and
then filtered them out of every scorer. Recording the direction anyway costs one
byte and keeps the decision about what to count in the scorer, where it can be
changed freely, rather than in the wire format, where changing it costs a
version bump for every peer.

### There is no `Reply` kind

A reply is a post that names a parent. The thread and parent identifiers live in
the payload, so a separate kind would make "is this a reply?" two questions —
which kind arrived, and then which fields are set — instead of one.

A thread-opening post carries no thread identifier. It cannot: a thread is named
by the id of the op that started it, and that id is the hash of the bytes being
signed, so the value is not known at signing time. The store fills the thread in
as the op's own id on ingest.

### The signature trails the op rather than leading it

The signed preimage is then a prefix of the wire form, so a decoder never skips
over the signature to find the bytes it covers.

The signature is a separate type wrapping the op rather than a field on it,
because an op containing its own signature would have to define whether the
signature covers itself. Keeping them separate makes the preimage exactly "the
op", with no carve-out anyone has to remember.

### The field length cap is checked before allocating, not after

A four-byte length prefix can claim four gibibytes. The transport caps a message
at 150 KiB as a network-wide validation limit, so a field longer than that could
never have arrived legitimately.

The ordering is the whole point. A bounds-checked read would refuse the read
afterwards — but only after the allocation had happened, which makes it a remote
memory-exhaustion lever rather than a defence. The same reasoning applies to a
list's element count, which is why the decoder grows the attachment vector as
elements actually arrive instead of reserving on the claimed count.

The refusal is reported as its own error rather than collapsing into "the input
ended", so that a test can demonstrate the cap is what rejected the input. That
distinction is not cosmetic: with the cap removed, the over-long cases still
fail — as truncation — and a test asserting only "some error" would pass over
the removed defence.

### Why the shared encoding rules were not extracted into a general capability

This was evaluated seriously, because the bar for it had arguably been met.
There are now two canonical encodings in this crate, and the process rule is to
reorganise specs "when a second instance shows that requirements written for one
capability are really about a general one" — moving them `REMOVED` from the old
and `ADDED` to the new, verbatim, in one change — and to do it "when the
generality is demonstrated, not predicted."

What the two genuinely share: a leading version discriminant with unknown
versions refused; four-byte big-endian length prefixes on variable-length
fields; strict rejection of truncation, trailing bytes and a length prefix lying
in either direction; unknown discriminants refused rather than defaulted; strict
UTF-8; invalid public keys refused; a distinct error per malformation; no
per-peer state. That is a substantial overlap and it is not a coincidence — the
op encoding was written by deliberately following the genesis record's.

It was declined, for three reasons.

**The requirement text does not survive a verbatim move, which is the only
primitive available.** OpenSpec has no capability move or rename, so an
extraction has to be composed from `REMOVED` plus `ADDED` of the *same text*.
But the genesis requirements are written about genesis records specifically — "A
genesis record SHALL have exactly one valid byte encoding", "a creator key that
is not a valid public key", "a title that is not valid UTF-8". Moved verbatim,
the general capability would talk about genesis records; rewritten to generalise,
it is no longer the verbatim move the rule describes. The primitive the process
offers does not fit the operation being contemplated, which is a signal that the
operation is not the one the process had in mind.

**The two formats version independently, and a shared capability would imply
otherwise.** Each encoding carries its own version discriminant, and they are
deliberately not the same one: a change to the op format has no reason to
invalidate every Stoa address in existence, and vice versa. A single
`wire-format` capability stating "the encoding SHALL begin with a version
discriminant" invites the first reader to ask which version — and the honest
answer is "there are two, and they are unrelated", which is a capability
describing nothing in particular.

**The overlap is thinner than it looks once the differences are counted.** The
op format has a field-length cap, optional-field presence tags, a list count, a
signature envelope, a content-derived id, and a kind discriminant that is
load-bearing for signature separation. The genesis record has none of these, and
is self-identifying by hash — its encoding *is* its address preimage — where an
op is addressed by its own id and carries a signature. PLAN.md §3.3 anticipated
exactly this: the fit was expected to be awkward, and for this reason.

What *was* extracted is the part where the generality is real and demonstrated
at the level of code rather than contract: the bounds-checked read head, which
both decoders now share. That happened in its own refactor commit, changing no
behaviour, before the op landed — which is the shape a demonstrated generality
should take. Copying a bounds check is how the second copy acquires the
off-by-one the first one fixed; copying a *requirement* costs nothing until the
two requirements need to differ, and here they already do.

This is recorded rather than silently skipped so that a third encoding has
somewhere to start. **If one arrives, revisit.** Three instances with the same
overlap, and the differences above still confined to the op, would be a
materially stronger case than two — and the third author should know that the
question was asked once and answered this way, rather than assuming nobody
looked.

### The module documentation stayed in `op.rs`

The document model puts reasoning in `design.md` and warns that keeping a second
copy is the failure mode, since two copies drift and the wrong one gets read.
Taken literally that argues for stripping `op.rs`'s module docs, which are
extensive and hold most of what is now in the Decisions above.

**They were kept, and the duplication is deliberate.** Three reasons.

The audiences differ in a way that matters here. `design.md` is read by someone
asking "why was this decided?", and is found by grepping the archive.
`op.rs`'s module docs are read by someone about to *add an op kind* — and the
single most important thing that reader must know (the kind byte is inside the
signed preimage, so its discriminant is a security-relevant constant and not a
declaration-order accident) needs to be in front of them at the moment they are
editing the enum. A reader who has to know to go looking will sometimes not know.

The warning is about drift, and the drift risk here is asymmetric. These
particular facts are pinned by tests that fail loudly if the code stops matching
them — the kind byte's position, the id derivation, the encoding's exact length.
A doc comment that contradicts the code is caught by the same test that catches
the code changing. That is not true of the material the document model was
written about, which is decision history no test can check.

Stripping well-written documentation to satisfy a rule about *reasoning* would
also be a misreading of what the rule targets. What PLAN.md must not keep is a
second copy of the *decision record*; what `op.rs` keeps is an explanation of
the code it sits on top of. The parts that are purely historical — what was
rejected, what the alternatives were, what a previous project measured — are in
this document and not in the source.

**The one thing deliberately not duplicated** is this section and the extraction
analysis above: they are about the change, not about the code, and a reader of
`op.rs` has no use for them.

## Risks / Trade-offs

- **A kind discriminant can never be reordered.** → Inherent to putting it in
  the signed preimage, and the point. The constants are explicit and commented
  as wire values in both `OpKind` and the two action enums, so the trap is
  visible at the place someone would trip it.
- **Strict decoding rejects ops from a newer client.** → Deliberate, and the
  version discriminant is what makes that a legible refusal rather than a
  misparse. Refusing to display is recoverable; misreading a moderation action
  is not.
- **The op format is fixed before the ordering metadata exists.** → Accepted
  knowingly. The alternative was inventing a field to fill a transport gap, and
  that field would outlive the gap. Recorded in PLAN.md §13 as blocking the
  store rather than the format.
- **Votes are collected and read by nothing.** → Cheap now, and the history is
  available when scoring lands. The alternative was deciding by accident.
- **The 150 KiB cap is a transport constant living in this crate.** → It is
  self-invalidating in the sense that matters: it is commented with what it is
  and why, so a transport change makes the comment visibly wrong rather than
  quietly so. It is a refusal threshold, not a promise, so being conservative
  costs nothing.

## Mutation verification

Each property the spec claims was broken in the source, the suite run, the
failing tests recorded, and the source restored. The worktree was confirmed
clean afterwards.

| Mutation | Tests that failed | Verdict |
|---|---|---|
| Remove `out.push(self.kind.to_byte())` from the encoding | `the_kind_byte_is_inside_the_signed_preimage`, `a_signature_over_one_kind_does_not_verify_as_another`, `the_version_is_the_first_byte_and_the_kind_is_the_second`, `the_op_id_constant_is_pinned_to_a_known_answer`, and 16 others | Pinned. The two that name the property fail for the reason they name. |
| Remove the `MAX_FIELD_LEN` check from `take_checked_length` | `a_field_over_the_sds_message_cap_is_refused_before_allocating`, `an_attachment_count_over_the_cap_is_refused_before_allocating` | Pinned, and pinned *specifically* — both assert `FieldTooLong`, so neither passes on the `Truncated`/`LengthMismatch` the input would fail with anyway. |
| Make the option tag lenient (`_ => present`) | `an_invalid_option_tag_is_refused_rather_than_read_as_present` | Pinned. |

One property the spec states was checked by exhaustive probe rather than left to
inference: **every accepted input re-encodes to itself.** A temporary test
mutated every byte of a valid post through all 256 values, decoded each, and
re-encoded the 12,938 inputs that were accepted. All reproduced their input
exactly. The probe was removed afterwards; see Open Questions for whether it
should be kept.

## Open Questions

- **Should the re-encoding canonicity probe become a permanent test?** It found
  nothing, which is the argument against; but it is the only check of canonicity
  in the direction that op-id uniqueness actually depends on — every *accepted*
  byte string re-encoding to itself, rather than every op round-tripping. The
  existing round-trip test checks the other direction and would not catch a
  decoder that accepted two encodings of one op. Left out of this change because
  this change is documents only, and adding a test is a behaviour change to the
  suite. Worth adding when the next change touches `op.rs`.
- **Does the panic-freedom test cover the whole encoding?** It mutates the first
  80 bytes of a roughly 107-byte post, so the attachment count region is covered
  only by the arbitrary-input half of the test. Not a known defect — the
  attachment count path is exercised by its own cap test — but the bound is
  arbitrary and reads as if it were exhaustive.
