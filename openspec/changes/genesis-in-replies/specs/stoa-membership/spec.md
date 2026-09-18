## ADDED Requirements

### Requirement: A reply naming a Stoa carries the record that Stoa's address is the hash of

Every reply of this capability that names a Stoa address MUST also carry the
genesis record that address is the hash of, encoded in the same form the calls
that accept a genesis record require.

**A caller cannot obtain the record any other way.** A Stoa address is a one-way
hash of the record, so an address is enough to verify a record handed over and
never enough to reconstruct one. A caller given only an address is therefore
permanently unable to perform any operation that takes a record — reading a feed,
reading a thread, joining, or producing a shareable reference — for a Stoa it can
nonetheless see and name.

The record MUST be the one the accompanying address is the hash of. A reply
carrying a well-formed record of some other Stoa satisfies "the field decodes"
and is useless: what makes the field usable is exactly that the pair verifies.

**The encoding MUST be the one those calls accept**, so that a record reported by
one call is a record another call takes without transformation. Two encodings for
one record is how a reply and the request that quotes it come to disagree.

Where the record cannot be encoded, the call MUST report the failure in the
module's failure shape and MUST NOT report success carrying an empty or absent
record. An empty record is precisely the input that fails to decode as
"ended mid-field", so emitting one would report a defect as a success and move
the resulting error to a later call that cannot explain it.

This requirement adds no storage obligation. The retention requirement above
already requires the record be kept for every Stoa the peer is in; this fixes
that the kept record is reported rather than withheld.

#### Scenario: A creation reports the record its address names

- **WHEN** a Stoa is created
- **THEN** the reply carries the genesis record
- **AND** the record decodes
- **AND** the address computed from the decoded record equals the address the same
  reply names

#### Scenario: A listed Stoa carries the record its address names

- **WHEN** a Stoa is listed
- **THEN** the item carries the genesis record
- **AND** the address computed from the decoded record equals the address the same
  item names

#### Scenario: A reported record is one the record-taking calls accept

- **WHEN** a Stoa is listed and the record from that item is supplied to a call
  that takes an address and its genesis record
- **THEN** that call accepts the record
- **AND** it is not refused as malformed or as failing to match the address

#### Scenario: A record that cannot be encoded is a failure, not an empty field

- **WHEN** a reply would name a Stoa whose retained record cannot be encoded
- **THEN** the call reports a failure
- **AND** no success reply is emitted carrying an empty or absent record

## MODIFIED Requirements

### Requirement: Creating a Stoa produces a genesis record the creator can moderate

The module MUST offer a call that takes a human-readable title and creates a Stoa: it MUST construct a genesis record whose creator key is a key this peer holds the secret half of, MUST compute that record's address, MUST record the Stoa as one this peer is in, and MUST return the address to the caller.

**The creator key MUST NOT be derived from the Stoa's own address, and this is forced rather than chosen.** A Stoa's address is a hash of its genesis record, and the creator key is one of the fields inside that record, so the address does not exist until the creator key is fixed. A creation-time key is therefore the one key in this system that cannot be per-Stoa, whatever scheme governs keys used *inside* an existing Stoa. A capability specifying which key signs an op inside a Stoa MUST NOT be read as constraining this one.

The creator key MUST NOT be a parameter. A call that accepted one would be a call that can be asked to create a Stoa moderated by somebody else, which is a Stoa the caller cannot moderate and did not mean to make.

The address MUST be returned. A creation reporting only success leaves the caller unable to name, share, or read what it just made.

**The genesis record MUST be returned alongside the address**, per the requirement that every reply naming a Stoa carries the record its address is the hash of. Returning the address alone leaves the caller in the state the previous paragraph exists to prevent — able to name the Stoa and unable to read or share it — because the address is a one-way hash and the caller can invert nothing.

The record MUST be retained, not discarded once the address is computed — the retention requirement below states what that is for. No moderation call exists yet and this requirement does not assert one; what it fixes is that creation leaves the peer holding everything a moderator set would later be derived from, rather than an address it can never invert.

**The creator key recorded in a genesis record is fixed at creation and is never re-checked against the peer's current signing key. This capability MUST NOT claim, in any reply, that the peer can moderate a Stoa it lists.** The two can diverge: the record is immutable and the signing key is not, so a peer whose signing key changes — restored from a different backup, or re-created — holds Stoas it created and can no longer moderate. Every answer this capability gives remains truthful in that state, because none of them is about moderation: a listed Stoa means the user chose it, not that the user governs it.

Whether that divergence is reportable is deliberately **out of scope here, and the silence is a decision rather than an omission.** Reporting it needs a call that compares a retained creator key against the peer's current one, and the vocabulary for "you cannot moderate this" belongs with the moderator set, which is the `moderation-resolution` capability's. Specifying it here would put a requirement no call in this capability can discharge into a contract about membership. **A capability that adds such a call MUST NOT assume this one detected the divergence: nothing here does, and the state is reachable, silent, and indistinguishable from the ordinary case in every answer above.**

#### Scenario: Creation returns the address of the record it built

- **WHEN** a Stoa is created with a title
- **THEN** the call returns a Stoa address
- **AND** the address is the one the genesis record just built verifies against

#### Scenario: Creation returns the record as well as the address

- **WHEN** a Stoa is created with a title
- **THEN** the reply carries the genesis record it built
- **AND** the record carries the title the creation was given

#### Scenario: The creator is the caller's own key

- **WHEN** a Stoa is created
- **THEN** the genesis record's creator key is a key this peer holds the secret half of
- **AND** no creator key was accepted from the request

#### Scenario: The creator key is one the peer can sign with

- **WHEN** a Stoa is created and the retained creator key is read back from membership
- **THEN** a signature this peer produces as the Stoa's creator verifies under that key

#### Scenario: The created Stoa is listed immediately

- **WHEN** a Stoa is created
- **THEN** it appears among the Stoas the peer is in
- **AND** it appears while the peer holds no op at all for it

### Requirement: Listing reports the Stoas the peer is in, and no others

The module MUST offer a paginated call listing the Stoas the peer is in — those it created and those it joined — each item carrying the Stoa's address, its founding title, and the genesis record that address is the hash of.

The envelope and the pagination arguments are the `module-wire-contract` capability's; this requirement conforms to them rather than restating them. What it fixes is the contents: exactly the Stoas membership records, and nothing derived from the ops the peer happens to hold.

**The listing is where the record matters most**, and is the reason the requirement above is not satisfied by creation and joining alone. A caller can only remember records for Stoas it created or joined during the current session; a caller that restarts holds none. Without the record on the listing item, every Stoa a peer was already in before the caller started is one it can see, name, and do nothing with.

A listing MUST report each Stoa once. Every Stoa the peer is in MUST be reachable by paging through the listing.

**The order MUST be total and MUST NOT depend on the order the Stoas were joined or created in.** Reachability-by-paging and once-only both rest on it: a page boundary that moved between two calls could show one Stoa twice or none at all. It is also a cross-peer property, which is why it is stated rather than left to follow — two peers in the same Stoas paging with the same arguments must see the same page boundaries, and an order derived from local join sequence would give the same address a different page on every peer. **The order carries no meaning and MUST NOT be presented as a ranking, a recency, or a join order.**

**Paging MUST terminate.** A caller paging until the envelope reports no more pages MUST reach that answer in finitely many calls for every page size the call accepts, including a page size of zero: a page size of zero MUST report an empty page that is also the last one, since paging further reaches nothing whatever the peer holds. Reporting an empty page that claims a page after it is what makes a caller loop forever, and it is the failure this clause exists to name.

An empty listing MUST report that there are no further pages.

#### Scenario: Only joined and created Stoas are listed

- **WHEN** a peer has created one Stoa, joined a second, and holds ops for a third it never joined
- **THEN** the listing contains the first two
- **AND** it does not contain the third

#### Scenario: A peer in no Stoa lists nothing

- **WHEN** a peer that is in no Stoa lists its Stoas
- **THEN** the listing is empty
- **AND** no failure is reported
- **AND** the envelope reports that there are no further pages

#### Scenario: Every Stoa is reachable by paging and appears once

- **WHEN** a peer that is in more Stoas than one page holds pages through the whole listing
- **THEN** every Stoa it is in appears
- **AND** none appears twice

#### Scenario: The order does not depend on the sequence of joins

- **WHEN** two peers join the same set of Stoas in different orders, and each pages through its listing with the same arguments
- **THEN** each page holds the same Stoas in the same positions for both peers
- **AND** the order is the same after a restart as it was before

#### Scenario: A page size of zero terminates rather than paging forever

- **WHEN** a listing is requested with a page size of zero, on a peer that is in at least one Stoa
- **THEN** the page is empty
- **AND** the envelope reports that there are no further pages

#### Scenario: Each item carries the address, not only the title

- **WHEN** a Stoa is listed
- **THEN** the item carries the Stoa's address
- **AND** the item carries the founding title from the retained record

#### Scenario: A Stoa is openable from the listing alone

- **WHEN** a peer that retained a Stoa's record across a restart lists its Stoas
- **THEN** the item carries the genesis record for that Stoa
- **AND** the caller can read that Stoa's feed using only what the listing reported
