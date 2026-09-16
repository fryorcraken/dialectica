<!--
Nothing about Stoa metadata changes. A Stoa address is not touched by this
change: `STOA_ADDRESS_PREFIX`, `identity::stoa_address` and `Genesis::address`
all survive byte-for-byte, and a Stoa is still identified by the hash of its
genesis record.

The single edit here is one sentence, scoped. `A displayed title is never an
identifier` closed with the bare "The address is the identity." The requirement
is Stoa-scoped throughout — its own preceding clause says "show the Stoa address
alongside any name" — so the sentence was never about an author. But this change
deletes the author address, and an unscoped sentence naming "the address" as
"the identity" is exactly what a later reader would cite as authority for
reinstating one. The proposal named this sentence and committed to scoping it;
this delta is that edit.
-->

## MODIFIED Requirements

### Requirement: A displayed title is never an identifier

A Stoa's displayed title SHALL NOT be treated as identifying it. Two Stoas may
carry identical titles, and a title is chosen freely by whoever signed the op.

Display text reaches a reader exactly as its author wrote it — the encoding
validates UTF-8 and deliberately applies no normalisation, because normalising
would break the canonicality every peer's agreement on op ids depends on. A
title may therefore contain bidirectional controls, zero-width characters, or
homoglyphs of an established Stoa's name, and while no authority check exists
any peer may sign such an op for any Stoa.

Whoever renders a title SHALL therefore mitigate at the point of display:
strip or visibly mark bidi and zero-width controls, show the Stoa address
alongside any name, and never resolve or match a Stoa by title. The Stoa
address is the identity.

#### Scenario: Two Stoas may share a displayed title

- **WHEN** two distinct Stoas declare the same current title
- **THEN** both remain distinct Stoas with distinct addresses
- **AND** neither is treated as the other

#### Scenario: Display text is preserved rather than sanitised in the data layer

- **WHEN** a metadata op carries a title containing control or zero-width characters
- **THEN** decoding preserves them unchanged
- **AND** the obligation to render them safely rests with the renderer
