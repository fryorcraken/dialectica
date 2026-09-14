# Allocate the public key's bytes across the three recognition channels

## Why

Issue #80 makes the Ed25519 public key the sole author identifier, deleting the
author address. That collapses three recognition channels — the generated
**name**, the **mark**, and the **abbreviated identifier on screen** — onto one
32-byte value, and it changes what makes them independent.

**Today their independence comes from domain separation, not byte allocation.**
The name is `H(NAME_PREFIX ‖ key)` and the mark reads bytes of the *address*,
itself `SHA256(AUTHOR_ADDRESS_PREFIX ‖ 0x01 ‖ key)`. Two different digests
cannot overlap, so byte 3 of one and byte 3 of the other are unrelated values.
Three documents in this tree say so explicitly, each correcting an earlier claim
that a byte range was "reserved" for the name: `generated-names`' spec (the
requirement *The derivation is domain-separated from every other use of the key*,
and the *bounded slice* requirement), `docs/IDENTICON.md`'s byte-layout section,
and `Identicon.qml`'s header, which states in capitals that the name "reserves
none of it" and that the mechanism the old comment described "does not exist".

**The owner's ruling on #80 is raw disjoint slices with no prefixes at all** —
no author address, therefore no `AUTHOR_ADDRESS_PREFIX`, and no `NAME_PREFIX`
either. With no hash between the key and either channel, the three channels read
one shared 32-byte space, and **byte-disjointness becomes the only thing keeping
name-grinding and mark-grinding independent searches whose costs multiply rather
than add**. A byte the abbreviation *displays* is worse than a merely shared one:
an attacker reads their progress off the screen while grinding, so that byte
contributes nothing an observer could not have been handed directly.

So the reservation those three documents correctly call fictional becomes real,
and this change is what makes it so. That is a decision worth its own review
pass, which is why it is separated from the mechanical removal of the address.

## What Changes

- **The public key's 32 bytes are allocated across the three channels**, with the
  three byte sets pairwise disjoint and the allocation stated as a contract
  rather than as an artefact of where each channel's arithmetic happens to land.
- **The name derives from raw key bytes**, with no hash and no domain separator
  between the key and the draws. **BREAKING** for every derived name: the same
  key yields a different name than under the current `H(NAME_PREFIX ‖ key)`
  scheme. No name is persisted or published, so nothing migrates — but every
  pinned expected name changes, and both peers must run the same build.
- **BREAKING**: `NAME_PREFIX` and the scheme version it carried are removed, and
  with them the wordlists' versioning seam. A wordlist can no longer change
  without every name changing at once and no way to tell the two schemes apart.
  The owner has taken this deliberately: the name is a pure local function, never
  published, so divergence between two peers is a client-side rendering
  difference rather than a consensus failure. Recorded as a consequence, not
  re-argued.
- **The mark reads a window of the public key** rather than of the address, and
  the requirement that pins its disjointness is restated against the key.
- **The gate becomes three-sided.** `tst_identicon.qml` measures the mark's and
  the abbreviation's byte sets by *probing* the two components, and asserts they
  are disjoint. It has no third probe because today the name reads an unrelated
  digest. The name joins the same shared space, so it needs the same treatment:
  a probe that measures which key bytes the name actually reads, checked pairwise
  against the other two.

### Explicitly not touched

- **`OP_SIGNING_PREFIX` stays.** It separates the signing digest and is what
  stops a signature over one preimage being replayed as a signature over
  another. "No prefixes" is about the identity derivations and reaches nothing on
  the signing path.
- **Stoa addresses stay.** #80 is the *author* address only.
- **Deleting the author address** from core, from the JSON replies that carry it,
  and from the specs that name it is the follow-on `key-identity-sweep` piece.
  This piece states the property the allocation must hold; the sweep performs the
  removal.

## Capabilities

### New Capabilities

None. The three-channel disjointness requirement already lives in
`generated-names`, which is where it belongs — putting the allocation in a new
capability would split one property across two spec files.

### Modified Capabilities

- `generated-names`: the derivation's input changes from a domain-separated
  digest of the key to raw bytes of the key; the requirement asserting that the
  name's independence rests on domain separation rather than byte allocation is
  inverted, because under one shared value the opposite is true; the mark and
  abbreviation disjointness requirement is restated against the key and widened
  to three channels; the scheme-versioning requirement loses the seam it rested
  on.

### Two stale scenario names the delta keeps deliberately

OpenSpec keys a `MODIFIED` requirement's scenarios by their headings and refuses
a delta that drops one, so a scenario cannot be renamed inside a `MODIFIED`
block — and `RENAMED` operates on requirements, not scenarios. Two scenario
headings therefore keep the word *digest* where the input is now the key:

- *A pinned name is reproducible from its digest by hand*
- *The bytes a name consumes do not vary with the digest*

Both bodies state the current contract and read the key; only the labels are
stale. The requirement heading that had the same problem — *The scheme and its
wordlists are versioned together and frozen* — is handled properly, through a
`RENAMED` block.

## Impact

- `dialectica/rust-lib/dialectica-core/src/names.rs` — the derivation's input,
  `NAME_PREFIX`, and the doc comment that argues independence from domain
  separation.
- `dialectica-ui/src/qml/Identicon.qml` — the mark's input and its header, which
  currently states the name reserves none of what it reads.
- `dialectica-ui/tests/tst_identicon.qml` — the third probe and the pairwise
  assertion. The existing probes are **inherited, not re-derived**: a competing
  version that recomputed `abbreviate`'s arithmetic instead of measuring the
  component passed under a mutation that genuinely broke disjointness, and was
  deleted for it.
- `docs/IDENTICON.md` — the byte-layout section and the passage recording the
  reservation as fictional.
- `docs/PLAN.md` §5.2.1 — points at the `generated-names` spec for the byte
  budget, so the pointer stays true and only the consequence needs recording.
