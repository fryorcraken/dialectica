## REMOVED Requirements

### Requirement: A title the genesis record cannot carry is refused before a Stoa exists

**Reason**: The owner ruled that a Stoa title of `""`, or one made only of
whitespace or zero-width characters, is invalid in every respect. This
requirement required creation to accept an empty title, and carried a scenario,
"An empty title is accepted rather than refused", asserting it. A MODIFIED block
cannot drop a scenario, so the requirement is replaced rather than amended.

**Migration**: Replaced by "A title the genesis record cannot carry, a blank one
included, is refused before a Stoa exists", added in this same change. Its text
is this requirement's with these edits: the paragraph naming what `stoa-genesis`
owns now includes the blank title, which that capability defines and which
includes the empty one; the paragraph requiring an empty title be accepted is
replaced by one requiring a blank title be refused, keeping its opening sentence
that creation imposes no bound the record lacks, and adding that a title with a
character outside the blank list is neither refused for the blank characters it
carries nor altered; and the accepting scenario is replaced by scenarios
asserting the refusal of an empty title and of a title made only of blank
characters, with boundary scenarios beside them for a one-character title and
for one visible letter among blank characters; and the scenario "A title
carrying control or bidirectional characters is not rejected for that reason"
has its title narrowed to one carrying those characters alongside at least one
character that is not blank, since a title made only of zero-width characters is
now refused. Every other clause and scenario is unchanged. A caller that created a Stoa with a blank title now receives the
error shape and no Stoa.

## ADDED Requirements

### Requirement: A title the genesis record cannot carry, a blank one included, is refused before a Stoa exists

Creation MUST refuse a title the genesis record has no encoding for, and MUST do so before recording any membership.

The bound itself — what a valid title is, that a record whose title exceeds the maximum has no encoding at all, and that a record whose title is blank has none either — is the `stoa-genesis` capability's. So is what blank means: its requirement "A blank title is not a valid title" lists the blank characters exactly, and the empty title is blank. What this requirement adds is the ordering: the refusal happens first, so a failed creation leaves nothing behind.

**The bound is a count of bytes in the title's UTF-8 encoding, not of characters**, and this surface MUST report it as such. A title arrives as a JSON string, where the natural reading of a maximum is characters, and the two counts differ for every title outside ASCII: a title of 400 CJK characters is 1200 bytes and MUST be refused against a 1024-byte bound, even though a caller counting characters would expect it to fit. Stating the unit here is not a restatement of `stoa-genesis`'s bound — it is the part a caller of *this* call needs in order to predict which titles it will refuse.

**A blank title MUST be refused**, the empty one included, in the module's failure shape, carrying a reason that names the title as blank, and creation MUST NOT record a Stoa for it.

Creation MUST NOT impose a bound the genesis record does not have. In particular, a title carrying at least one character that is not a blank character MUST NOT be refused on account of the blank characters it also carries, and MUST NOT be trimmed or otherwise altered before it is encoded.

The invalid-UTF-8 case the genesis decoder refuses is deliberately not a scenario here. A title arrives as a JSON string, which cannot carry bytes that are not valid text, so that refusal is unreachable through this surface and a scenario claiming to exercise it would be exercising something else.

#### Scenario: An over-long title creates nothing

- **WHEN** creation is attempted with a title whose UTF-8 encoding is longer in bytes than a genesis record can carry
- **THEN** the call reports a failure
- **AND** the peer is in no new Stoa afterwards

#### Scenario: A title at exactly the maximum number of bytes creates a Stoa

- **WHEN** creation is attempted with a title whose UTF-8 encoding is exactly the maximum number of bytes a genesis record carries
- **THEN** the call succeeds
- **AND** the peer is in the Stoa it names

#### Scenario: The bound is counted in bytes, not in characters

- **WHEN** creation is attempted with a title of multi-byte characters whose count of characters is well within the bound but whose UTF-8 encoding exceeds it in bytes
- **THEN** the call reports a failure
- **AND** a title of multi-byte characters encoding to exactly the bound in bytes — and therefore to fewer characters than the bound — is created successfully

#### Scenario: An empty title creates nothing

- **WHEN** creation is attempted with a title that is the empty string
- **THEN** the reply is the error shape
- **AND** its reason names the title as blank, distinguishably from the reason given for an over-long title
- **AND** the peer is in no new Stoa afterwards

#### Scenario: A title made only of whitespace or zero-width characters creates nothing

- **WHEN** creation is attempted with the title of three U+0020 spaces, again with the title U+200B U+200B, and again with the title U+3000 U+FEFF U+0009
- **THEN** each reply is the error shape
- **AND** each reason is the one an empty title is refused with
- **AND** the peer is in no new Stoa afterwards

#### Scenario: A title of one character creates a Stoa

- **WHEN** creation is attempted with a title that is a single ASCII letter
- **THEN** the call succeeds
- **AND** the Stoa's founding title is that letter

#### Scenario: One visible letter among whitespace and zero-width characters creates a Stoa

- **WHEN** creation is attempted with the title U+0020 U+200B, the letter `a`, U+3000 U+FEFF
- **THEN** the call succeeds
- **AND** the founding title reported afterwards is those five characters in that order, with none removed at either edge

#### Scenario: A title carrying control or bidirectional characters is not rejected for that reason

- **WHEN** creation is attempted with a title containing zero-width or bidirectional control characters alongside at least one character that is not blank, within the length bound
- **THEN** the call succeeds
- **AND** the founding title reported afterwards carries those characters unchanged

## MODIFIED Requirements

### Requirement: Joining takes an address and the record it names, and verifies rather than trusts

The join call MUST take both a Stoa address and a candidate genesis record, and MUST verify the record against the address before recording anything.

**An address alone is not joinable, and that is a property of the address rather than a limitation of this call.** The address is a one-way hash of the record: it is sufficient to verify a record somebody hands over and insufficient to reconstruct one. Since a membership must retain the record — the retention requirement below says what for — the record has to arrive with the address, because there is nowhere else for it to come from. This call's shape says so rather than leaving a caller to discover it after joining.

A join MUST return what the caller needs to show what was joined, including the address and the Stoa's founding title.

**A record whose title is blank MUST be refused**, the empty title included and with blank meaning what the `stoa-genesis` capability's requirement "A blank title is not a valid title" defines, as a record the genesis encoding refuses to decode, and MUST NOT be recorded as a membership. The record arrives from whoever handed it over and is attacker-controlled, so this holds however the pair was obtained.

#### Scenario: A matching record joins

- **WHEN** a peer joins with an address and the genesis record that address names
- **THEN** the join succeeds
- **AND** the Stoa appears among the ones the peer is in
- **AND** the reply carries the address and the record's founding title

#### Scenario: A record that does not match the address is refused

- **WHEN** a peer joins with an address and a genesis record differing from the one that address names in any field
- **THEN** the join reports a failure
- **AND** the peer is not in that Stoa
- **AND** the peer is not in the Stoa the supplied record would name either

#### Scenario: A malformed record is refused without a membership

- **WHEN** a peer joins with a record the genesis encoding refuses to decode
- **THEN** the join reports a failure
- **AND** the peer is in no new Stoa

#### Scenario: A record whose title is blank is refused without a membership

- **WHEN** a peer joins with a record that is otherwise a well-formed genesis record but whose title is the empty string, together with the address computed by hashing that record's bytes — and again with such a record whose title is U+0020 U+200B, with the address computed the same way
- **THEN** each reply is the error shape
- **AND** each reason names the title as blank
- **AND** the peer is in no new Stoa

#### Scenario: Verification consults nothing but the two inputs

- **WHEN** a join verifies a record against an address
- **THEN** the decision uses only the supplied address and the supplied record
- **AND** no index, registry, peer or network call is consulted

### Requirement: A joined Stoa's genesis record is retained, not only its address

For every Stoa the peer is in, the peer MUST retain the complete genesis record and not merely the address.

The address cannot be inverted, and the `moderation-resolution` capability requires the record before a reader may decide whether any moderation of that Stoa's content binds. A peer that retained only addresses would hold a list of Stoas whose content it can store and cannot judge, and the deficiency would be invisible until a moderation op arrived.

The retained record MUST remain the one that verifies against the address it is retained under. Retrieving it and recomputing the address MUST yield that same address.

**A retained record that does not verify against the address it is retained under MUST be reported as a failure, and MUST NOT be skipped, repaired, or omitted from an otherwise successful answer.** This is the case the sentence above says must not happen, and saying what happens when it does is the difference between a peer that reports it cannot read its memberships and one that quietly lists fewer Stoas than the user joined. A listing that dropped the offending Stoa would be indistinguishable from a listing of a peer that never joined it.

**A retained record the genesis encoding refuses to decode MUST be reported as a failure in the same way, and MUST NOT be skipped, repaired, migrated, rewritten, or omitted from an otherwise successful answer.** This includes a record retained before the genesis encoding refused a blank title, whose title is blank: its bytes still hash to the address it is retained under, and the genesis encoding refuses to decode it. A listing that reaches such a record MUST answer in the module's failure shape, carrying the reason the decoding gave, and MUST leave the retained record as it was.

**The posting policy is reported by a stable name**, so a caller can branch on it without decoding the record. It is spelled in the reply as a string under the field `policy`, and the name for the policy a created Stoa declares is `"open"`.

Creation takes no policy parameter and every Stoa created through this surface declares `open`; widening that is a later change's to make. **A policy this build has no name for therefore does not arise from creation, and does arise from joining** — the record is supplied by whoever hands it over, and the `stoa-genesis` capability owns which policy bytes decode at all. Where such a policy is reached, this capability MUST report a failure rather than substituting `"open"`. Telling a view that a Stoa restricting who may post is world-postable is the one error in this area with a security consequence, and a default is how it would happen.

#### Scenario: The retained record still verifies against its address

- **WHEN** the genesis record retained for a Stoa the peer is in is read back and its address recomputed
- **THEN** the result is the address the Stoa is known by

#### Scenario: Founding values are answerable from what was retained

- **WHEN** a peer is asked about a Stoa it is in
- **THEN** the founding title and the posting policy from the retained record are answerable
- **AND** the policy of a Stoa created through this surface is reported as `"open"` under the field `policy`
- **AND** no network call is needed to answer them

#### Scenario: A retained record that does not verify is reported, not skipped

- **WHEN** a listing or a lookup reaches a retained record that does not verify against the address it is retained under
- **THEN** the call reports a failure
- **AND** the answer does not silently omit that Stoa and report success

#### Scenario: A retained record whose title is blank is reported, neither skipped nor migrated

- **WHEN** a peer's membership state retains, under the address its bytes hash to, a record that is otherwise a well-formed genesis record but whose title is the empty string, and the peer lists its Stoas — and again with such a record whose title is U+0020 U+200B
- **THEN** each reply is the error shape
- **AND** each reason names the title as blank
- **AND** no reply is a listing that omits that Stoa and reports success
- **AND** the retained record's bytes afterwards are the bytes that were retained before the listing
