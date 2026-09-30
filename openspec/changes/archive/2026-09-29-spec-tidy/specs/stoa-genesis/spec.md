## MODIFIED Requirements

### Requirement: An address verifies the record it names

A peer given a Stoa address and a candidate genesis record MUST be able to determine, without consulting any registry or third party, whether the record is the one that address names.

This is what makes a pasted address self-authenticating, and it is a security boundary: a Stoa address can appear inside a post, which is attacker-supplied content, and `stoa-navigation-view`'s requirement "Joining shows what is being joined, and joins nothing until the user acts" names an address inside a post as one of the two routes to a preview.

#### Scenario: A matching record verifies

- **WHEN** a record is checked against the address computed from it
- **THEN** verification succeeds

#### Scenario: A substituted record fails verification

- **WHEN** a record differing in any field is checked against the original address
- **THEN** verification fails

#### Scenario: Verification consults nothing external

- **WHEN** a record is verified against an address
- **THEN** the check uses only the address and the record
