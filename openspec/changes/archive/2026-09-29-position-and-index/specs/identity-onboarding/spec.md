## ADDED Requirements

### Requirement: A malformed `index` in a keep request is refused with a message naming `index`

A request to keep a candidate MUST name the candidate under the key `index`, as a
non-negative integer.

An `index` that is any of the following MUST be refused with the wire contract's
error shape:

- negative;
- fractional;
- written with a decimal point or an exponent, even where the value it denotes is
  a whole number;
- not a number;
- an integer larger than this peer can represent.

The refusal's message MUST name `index`. The reply MUST NOT carry any field
reporting a keep. The refusal MUST NOT store a master key or record a choice that
were not there before.

A malformed `index` MUST NOT be coerced to any candidate. This requirement adds
only that the refusal names the field. The obligation to refuse rather than guess
is stated by *Every entry point refuses malformed input rather than guessing*.

**A well-formed `index` that names no candidate is not malformed.** A non-negative
integer this peer can represent, naming no candidate in the current set, is a
selection outside the current set. *A selection outside the current set is
refused* governs it. It is answered with the reply stating that nothing was kept
and giving a reason, not with the error shape.

**This requirement does not cover an absent `index` or one supplied as `null`.**
What those are refused as, and what the message says, is `module-wire-contract`'s
to state for every required field. It is not restated here.

#### Scenario: Each malformed kind of `index` is refused by name

- **WHEN** a keep request names a valid Stoa and the current set, and carries an
  `index` of `-1`, of `1.5`, of `0.0`, of `1e2`, of `"two"`, of `[]`, of `true`, and
  of `18446744073709551616`, each in turn
- **THEN** every reply is the error shape
- **AND** each reply's message contains `index`
- **AND** no reply carries a field reporting a keep

#### Scenario: A malformed `index` stores nothing

- **WHEN** a keep request names a valid Stoa and the current set, and carries a
  malformed `index`
- **THEN** no master key is stored and no choice is recorded that were not there
  before

#### Scenario: A well-formed `index` naming no candidate is a not-kept reply, not a malformed one

- **WHEN** a keep request names a valid Stoa and the current set, and carries an
  `index` that is a non-negative integer one greater than the last position in
  that set
- **THEN** the reply states that nothing was kept and carries a reason
- **AND** the reply is not the error shape
