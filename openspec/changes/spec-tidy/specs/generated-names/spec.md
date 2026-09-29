## MODIFIED Requirements

### Requirement: The name SHALL NOT travel

No reply SHALL carry a display name: not a feed row, not a thread item, not an
onboarding slate candidate, not any other reply in which core reports an author.
A name is derived by whoever holds the key, at the point of rendering.

**A derived value beside the material it derives from is two values that must
agree and could disagree**, where the recipient has no way to tell which is
wrong. A name on the wire is also a name a relay could strip or forge, which
determinism exists to make impossible. So the name does not travel, in either
direction, under any reply shape.

What a reply owes is therefore the derivation's **input**, and that obligation
belongs to each reply's own capability rather than to this one. This capability
says only that a name is never the thing carried.

**How a caller reaches a name is this capability's requirement "Core exposes the
derivation to a caller"**: a caller supplies a public key it already holds and
receives that key's name. That is a request answered, not a reply carrying a
name about somebody else's authorship, so it is not what this requirement
forbids. It is also the view's only route to a name: the QML sandbox denies the
view the network and the filesystem outside its plugin directory, so it holds
none of the wordlists and cannot derive a name for itself.

#### Scenario: No reply carries a display name

- **WHEN** a reply in which core reports an author is read and every field of the
  item reporting that author is enumerated
- **THEN** no field carries a display name

#### Scenario: A name does not travel as content

- **WHEN** a post arrives carrying a field that spells a display name
- **THEN** that field is not treated as the author's name anywhere
- **AND** the name for that author remains the one derived from the signing key

### Requirement: A name is never unique, never an identifier, and never numbered

A display name SHALL NOT be treated as unique, SHALL NOT be accepted anywhere an
identity is named, and SHALL NOT be disambiguated by appending a number or any
other distinguishing suffix.

Uniqueness is unavailable rather than merely unbuilt. There is no registry and no
authority to hold a namespace: a Stoa has no membership list, peers join and
leave without announcing it, and each peer knows only what has reached it. Two
peers can each believe a name is free.

Numbering is worse than leaving a collision alone. Appending a suffix requires
agreeing which of two identities was second, which is arrival order — a per-peer
fact — so two peers would number the same pair oppositely and each would be sure
the other was the impostor.

No operation SHALL accept a display name in place of an author's public key: not
a lookup, not a moderation target, not a vote target, not a request naming an
author. **The public key is the identity.** An author is identified by the key
that signs, and by nothing derived from it — the name and the mark are both
recognition aids computed from that key, and neither is the thing being
identified.

This sentence replaces an earlier rule rather than restating it. An author was
once identified by an address derived from the key, and "the address is the
identity" was the rule everywhere an author appeared. With the author address
deleted, a requirement still pointing at it would point at a value no longer
carried. **Stoa addresses are untouched**: a Stoa is still identified by its
address, and nothing here reaches that.

Collisions are rare rather than expected at the specified space, and that changes
nothing here. The rule is not a response to the rate.

#### Scenario: Two keys deriving one name each derive it unchanged

- **WHEN** two distinct public keys that derive the same name are each put through
  the derivation
- **THEN** each yields that name exactly as derived
- **AND** neither carries a number, a suffix or any other added distinguishing
  mark

#### Scenario: No method accepts a name where an identity is required

- **WHEN** a request supplies a display name in a field that names an author, a
  moderation target or a vote target
- **THEN** the request is refused rather than resolved to any identity
