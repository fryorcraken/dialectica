# Design

## Context

See proposal.md — Why. `stoa_address(genesis_bytes: &[u8])` exists in
`identity.rs` and hashes a domain-separated prefix over caller-supplied bytes.
It is correct as far as it goes; what is missing is any definition of the bytes.
This change supplies that and leaves the hashing untouched.

## Goals / Non-Goals

**Goals**

- One canonical encoding, and decoding strict enough that a hostile record is
  refused rather than misread.

**Non-Goals**

- **No policy enforcement.** The record *declares* a policy; nothing yet checks
  a poster against it. `open` needs no check, which is why it is the first
  variant.
- **No mutable Stoa metadata.** Title and description as moderator-editable
  state is separate work under the moderator-scoped last-write-wins rule; the
  genesis title is immutable and address-determining.
- **No moderator set.** The creator is the sole moderator, which follows from
  the creator key without anything further being stored.
- **No discovery.** Broadcast and curation are later phases.

## Decisions

### Length-prefix every variable-length field

The title is variable-length, and a second variable-length field will arrive
with invite lists or a token identifier. Concatenating them lets two distinct
records collide: `("ab", "c")` and `("a", "bc")` produce identical bytes, and
therefore an identical address for two different Stoas.

Prefixing costs a few bytes and removes the class. This is the same trap
`identity.rs` guarded against by putting the fixed-width field first — here the
fix has to be prefixes, because two variable-length fields cannot be ordered out
of ambiguity.

### Reject unknown policy discriminants rather than ignoring them

The tempting alternative — treat an unrecognised policy as `open` — is how a
token-gated Stoa silently becomes world-postable on an older client. Refusing to
decode means an old client cannot *display* a Stoa it does not understand, which
is the safe direction: not showing a Stoa is recoverable, misrepresenting its
access policy is not.

### Reject trailing bytes

Accepting them would mean two byte strings decode to the same record while
hashing to different addresses — the exact ambiguity canonical encoding exists
to remove.

### Keep `stoa_address` as it is, and add a typed companion

Callers holding canonical bytes (a record received from a peer, already decoded
and re-encoded) still want the byte-oriented entry point. Adding
`Genesis::address()` alongside it means the common path cannot forget to
canonicalise, without narrowing the primitive.

### The record carries only what every peer agrees on

Every peer hashes the record to obtain the address, so any value varying with
one peer's history gives that peer a different address for the same Stoa — two
Stoas that cannot see each other, with no error anyone observes. §4.3 states the
same rule for the channel id derived from this address.

## Risks / Trade-offs

- **A future field changes every address.** → Inherent to hashing an immutable
  record, and the whole reason `policy` lands now. Any later field needs a
  version discriminant, which is why the encoding starts with one.
- **Strict decoding rejects records from a newer client.** → Deliberate, per the
  policy decision above. The version discriminant is what makes that failure
  legible rather than a parse error.
- **Title is immutable.** → A Stoa that wants a new title creates mutable
  metadata later; the genesis title stays as the creation-time record.

## Open Questions

None.
