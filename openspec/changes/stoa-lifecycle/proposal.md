# Creating, joining and listing Stoas through the module's wire API

## Why

The `stoa-genesis` capability contracts what a Stoa **is** — a record, its
canonical encoding, and an address that self-authenticates against it. Nothing
contracts the three acts a user performs with one: creating it, joining someone
else's, and seeing which ones they are in. The view has no filesystem and no
network of its own, so a Stoa a method does not return is a Stoa no screen can
show; the MVP's items 2, 6, 7 and 10 (create a Stoa, share its address, join by
address, persistence on disk) are all on the far side of that gap.

There is also a structural fact the specs currently assume away, and it is the
reason this is its own capability rather than a widening of `op-log`: **which
Stoas a peer is in is not derivable from the ops it holds.** A Stoa joined and
still quiet has no ops, so an op-derived answer omits it. A single gossiped op
addressed to an unknown Stoa would, symmetrically, invent a membership nobody
asked for. Membership is peer-local state about what the user chose, and the log
is a record of what arrived.

## What Changes

- **A create call**, taking a title and producing a Stoa the caller is in: a
  genesis record whose creator is the caller's own key, its address, and
  membership recorded so the Stoa survives a restart.
- **A join call taking an address *and* a genesis record**, not an address
  alone. This is the shape the change most wants to fix in advance: a bare
  address cannot be joined usefully. An address is a one-way hash of the record,
  so it is sufficient to *verify* a record handed to you and insufficient to
  *reconstruct* one — and `moderation-resolution` requires the record, not the
  address, before a reader may resolve moderation for a Stoa at all. Joining by
  address alone would therefore mint a membership that can never moderate and
  can never be checked.
- **A list call** over the Stoas the peer is in, following the pagination
  envelope `module-wire-contract` sets. This is the first instance of that
  envelope on the wire surface.
- **A membership store that is not the op log**, contracted by behaviour only:
  what survives a restart, what it retains per Stoa, and that it neither gains
  nor loses a Stoa because ops arrived or did not.
- **A defined answer for an op arriving for a Stoa the peer has not joined.**
  The receive path another change owns needs this settled, and settling it is
  the only way "the log holds what arrived" and "membership is what the user
  chose" can both stay true.

Explicitly not in this change:

- **No store layout version bump, and no refusal of an older store.** A
  requirement is written the other way round: adding Stoa membership MUST NOT
  make a store holding ops unreadable. Refusing to open an existing store is a
  data-loss behaviour, and there is no migration to justify it when the new
  state is additive.
- **No JSON request-shape hardening.** A separate change owns the whole wire
  surface's treatment of valid-JSON-that-is-not-an-object. Nothing here states
  what a non-object request does, so nothing here contradicts it.
- **No metadata resolution.** What a Stoa is *called today* comes from a
  moderator-signed metadata op, and `stoa-metadata` already contracts that
  resolution rule while recording that nothing implements it. This change reads
  founding values and says which they are; it does not resolve metadata and does
  not restate the rule.
- **No feed, no thread read, no compose, no moderation.**

## Capabilities

### New Capabilities

- `stoa-membership` — creating a Stoa, joining one by address plus record,
  listing the ones the peer is in, and what that peer retains about each so that
  the answer survives a restart and does not drift with the ops that arrive.

### Modified Capabilities

None, and each near-miss is a deliberate decline rather than an omission:

- **`stoa-genesis`** owns the record, its canonical encoding, address derivation,
  the posting policy, the version discriminant, and the rule that an address
  verifies its record without consulting any registry. The create and join
  requirements here *depend* on all of it and restate none of it. Where a
  requirement needs the property, it names it — "the address verifies the record
  it names" — rather than re-asserting how.
- **`moderation-resolution`** owns "A Stoa's moderator set is derived from its
  genesis record", including that a reader holding no record for a Stoa may not
  resolve moderation for it. That requirement is the *reason* membership retains
  the record. It is cited, not copied.
- **`op-log`** owns the log. This capability names the boundary in its Purpose:
  the log records what arrived and decides nothing about it; membership records
  what the user chose. Neither derives from the other, and the two disagreeing
  is normal rather than a fault.
- **`module-wire-contract`** owns JSON-in/JSON-out, the single `{"error":...}`
  failure shape, and the paginated envelope. The list requirement conforms to
  the envelope and does not restate its fields as requirements of its own.
- **`posting-capability`** owns the probe a view asks before showing an
  affordance that publishes. Creating a Stoa needs a key, which makes it the
  same question; this capability says creation fails without a usable key and
  leaves the probe's shape and its reason vocabulary where they are.
- **`stoa-metadata`** owns current-versus-founding metadata and its resolution
  rule. See above.

## Impact

- **Widens the wire surface by three methods.** This is the deliberate act
  `CLAUDE.md` asks for: the surface currently reaches no Stoa, no op and no
  store, and these are the first methods that reach state a user created.
- **Introduces peer-local persistent state that is not an op.** Every existing
  capability's requirements are written over ops; this one is not, and the
  `op-log` boundary paragraph exists so that the next reader finds that stated.
- **First instance of the paginated envelope.** Whatever lands next inherits the
  precedent.
- **Sets the receive path's precondition.** The change specifying inbound ops has
  a defined answer for an op addressed to an unjoined Stoa rather than having to
  invent one.
- **No change to the genesis encoding, the op format, or the op log's
  requirements.** Nothing here alters an address, an op id, or what the log
  stores.
