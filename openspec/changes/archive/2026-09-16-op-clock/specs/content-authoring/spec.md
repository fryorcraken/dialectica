## RENAMED Requirements

- FROM: `### Requirement: Publishing the same content twice publishes one op`
- TO: `### Requirement: Authoring the same content twice publishes two ops, re-publishing one publishes one`

## MODIFIED Requirements

### Requirement: Authoring the same content twice publishes two ops, re-publishing one publishes one

An op id is a function of the op's own bytes, and those bytes now carry a Lamport
counter. Two consequences follow, and they are stated together because the
difference between them is the whole of what this requirement settles:

- **Authoring the same content twice produces two ops.** The counter advances
  between the two publishes, so the bytes differ, so the ids differ. The log holds
  both and a reader renders both.
- **Re-publishing an op the peer already holds produces one op.** Every field
  matches, counter included, so the bytes are identical and the id is the one the
  log already has.

The rule underneath both is unchanged and is the same single rule it always was:
**identical bytes are one op.** What this change alters is not the rule but its
input — "the same content" used to be enough to make two publishes byte-identical
and no longer is.

**This reverses the prior behaviour for the authoring case**, which produced one
op, and the reversal is the point rather than a side effect. The prior text
recorded that the old behaviour was "correct for one case and wrong for another":
a double-submitted form was deduplicated, which was wanted, and a person
deliberately posting the same short reply twice published once, which was not. It
named the missing field as the fix and named `op-format`'s "An op carries no
ordering field and no per-peer state" as what forbade it. That prohibition is
withdrawn by this change, so the fix is available and is taken.

**The case the old behaviour served is now unserved, and that is a real cost
rather than a neutral trade.** A double-submitted form publishes twice. Nothing
in this capability prevents it, and nothing should: the two publishes are
genuinely two ops, correctly signed, correctly ordered, and indistinguishable at
this layer from a person posting "agreed" twice on purpose. **Suppressing a
double submission belongs to whatever handles the submission**, where the two
cases are distinguishable — a form that disables its own button, or a composer
that will not re-send while a send is outstanding. It is named here so that it is
a known gap with an owner rather than a regression discovered by a user seeing
their post twice.

The publish SHALL still surface to its caller the newly-stored-or-already-present
answer the append gives it. That answer is now almost always "newly stored", and
it is retained rather than dropped because the case it reports has not become
impossible: **an op received from the network before this peer publishes an
identical one is still deduplicated**, since the two are the same op only if
every field matches, counter included.

The reply SHALL NOT report a refusal for a repeated publish.

#### Scenario: The same content published twice yields one op

- **WHEN** an op the peer already holds is published again, every field matching
  including its counter
- **THEN** both publishes succeed
- **AND** both replies name the same op id
- **AND** the log holds one op for it

**This scenario keeps its name and its WHEN has gained the condition that was
previously implicit.** The rule it tests is unchanged — an op id is a function of
the op's bytes, so identical bytes are one op. What changed is that the bytes now
include a counter, so publishing the same *body* twice is no longer sufficient to
produce identical bytes. Every field matching is, and that is what re-publishing
an op already held means.

#### Scenario: The same body authored twice yields two ops

- **WHEN** one identity authors and publishes a post with the same Stoa and the
  same body twice, so that the counter advances between the two publishes
- **THEN** both publishes succeed
- **AND** the two replies name different op ids
- **AND** the log holds both

#### Scenario: The second authoring carries the higher counter

- **WHEN** one identity authors and publishes the same body into one Stoa twice
- **THEN** the second op's counter is greater than the first's
- **AND** the ordering rule places the second ahead of the first

#### Scenario: The caller is told the second publish was not new

- **WHEN** a publish stores an op the log already held, every field matching
  including the counter
- **THEN** the reply states the op was already present
- **AND** a publish that stored an op the log did not hold stated it was newly
  stored

#### Scenario: Two authorings of one body are both reported as newly stored

- **WHEN** one identity authors and publishes the same body twice, so that the
  counters differ
- **THEN** each reply states its op was newly stored
- **AND** neither is reported as already present

#### Scenario: A repeated publish does not disturb the stored op

- **WHEN** an op the peer already holds is published again
- **THEN** the op the first publish stored is readable unchanged
- **AND** it is byte-identical to what was stored

#### Scenario: Content differing in any way publishes a second op

- **WHEN** one identity publishes two posts into one Stoa whose bodies differ by
  a single character
- **THEN** the two replies name different op ids
- **AND** the log holds both

#### Scenario: The same body in two Stoas is two ops

- **WHEN** one identity publishes the same body into two different Stoas
- **THEN** the two replies name different op ids

#### Scenario: The same body from two identities is two ops

- **WHEN** two identities publish the same body into one Stoa
- **THEN** the two replies name different op ids

#### Scenario: Two replies with the same body to different parents are two ops

- **WHEN** one identity publishes the same reply body to two different parents
- **THEN** the two replies name different op ids
