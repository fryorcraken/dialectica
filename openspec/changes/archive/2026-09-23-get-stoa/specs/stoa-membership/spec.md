## MODIFIED Requirements

### Requirement: A listed title is a founding title, and is identified as such

A title reported by this capability MUST be the founding title from the retained genesis record, and the reply MUST make it distinguishable from a current title resolved from a moderator-signed metadata op.

The distinction is not cosmetic: a founding title is what the Stoa was created as, possibly long ago, and the `stoa-metadata` capability states that the current title is carried by a separate op and is answered by its own call. A reply presenting a founding title as the current one asserts something no peer has checked. Resolving current metadata is out of scope here; saying which of the two is being reported is not.

#### Scenario: The reported title is marked as founding

- **WHEN** a Stoa is listed, or a join reports what was joined
- **THEN** the title reported is the founding title from the genesis record
- **AND** the reply indicates that it is the founding value rather than a resolved current one
