## Purpose

Defines the three acts a user performs with a Stoa through the module's surface — creating one, joining one someone else created, and listing the ones they are in — and what a peer retains about each Stoa so that those answers survive a restart.

The boundary with the `op-log` capability is the load-bearing part of this contract, and it is stated here rather than restated as a requirement. `op-log` owns a record of **what arrived**: its requirement "The log records what arrived, and decides nothing about it" is what makes it safe to store an op before anything has judged it. This capability owns a record of **what the user chose**. Neither is derivable from the other, and the two disagreeing is ordinary rather than a fault: a Stoa the user joined and that has since been silent has no ops at all, while an op addressed to a Stoa the user never heard of arrives whether or not they want it. Nothing below re-specifies what the log stores, how it orders, or what it refuses to decide.

Two further boundaries, named so that no requirement here duplicates one:

- The record, its canonical encoding, address derivation, the posting policy and the rule that an address verifies its record without consulting any registry are all the `stoa-genesis` capability's. This capability *uses* that verification and does not restate how it works.
- That a reader may not resolve moderation for a Stoa whose genesis record it does not hold is the `moderation-resolution` capability's requirement "A Stoa's moderator set is derived from its genesis record". It is the reason membership retains the record, and it is cited rather than copied.

## ADDED Requirements

### Requirement: Creating a Stoa produces a genesis record the creator can moderate

The module MUST offer a call that takes a human-readable title and creates a Stoa: it MUST construct a genesis record whose creator key is a key this peer holds the secret half of, MUST compute that record's address, MUST record the Stoa as one this peer is in, and MUST return the address to the caller.

**The creator key MUST NOT be derived from the Stoa's own address, and this is forced rather than chosen.** A Stoa's address is a hash of its genesis record, and the creator key is one of the fields inside that record, so the address does not exist until the creator key is fixed. A creation-time key is therefore the one key in this system that cannot be per-Stoa, whatever scheme governs keys used *inside* an existing Stoa. A capability specifying which key signs an op inside a Stoa MUST NOT be read as constraining this one.

The creator key MUST NOT be a parameter. A call that accepted one would be a call that can be asked to create a Stoa moderated by somebody else, which is a Stoa the caller cannot moderate and did not mean to make.

The address MUST be returned. A creation reporting only success leaves the caller unable to name, share, or read what it just made.

The record MUST be retained, not discarded once the address is computed — the retention requirement below states what that is for. No moderation call exists yet and this requirement does not assert one; what it fixes is that creation leaves the peer holding everything a moderator set would later be derived from, rather than an address it can never invert.

**The creator key recorded in a genesis record is fixed at creation and is never re-checked against the peer's current signing key. This capability MUST NOT claim, in any reply, that the peer can moderate a Stoa it lists.** The two can diverge: the record is immutable and the signing key is not, so a peer whose signing key changes — restored from a different backup, or re-created — holds Stoas it created and can no longer moderate. Every answer this capability gives remains truthful in that state, because none of them is about moderation: a listed Stoa means the user chose it, not that the user governs it.

Whether that divergence is reportable is deliberately **out of scope here, and the silence is a decision rather than an omission.** Reporting it needs a call that compares a retained creator key against the peer's current one, and the vocabulary for "you cannot moderate this" belongs with the moderator set, which is the `moderation-resolution` capability's. Specifying it here would put a requirement no call in this capability can discharge into a contract about membership. **A capability that adds such a call MUST NOT assume this one detected the divergence: nothing here does, and the state is reachable, silent, and indistinguishable from the ordinary case in every answer above.**

#### Scenario: Creation returns the address of the record it built

- **WHEN** a Stoa is created with a title
- **THEN** the call returns a Stoa address
- **AND** the address is the one the genesis record just built verifies against

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

### Requirement: Creating a Stoa requires a usable signing key, and says so rather than inventing one

Creation MUST fail when the peer has no usable signing key, and MUST NOT proceed by generating a key for the occasion or by using a placeholder.

A Stoa's creator key is what makes the creator its sole moderator, and it is fixed inside the address preimage forever. A Stoa created under a key the user does not hold is a Stoa nobody can moderate and whose address cannot be un-minted.

Whether a key is usable, and the vocabulary for why it is not, is the `posting-capability` capability's probe. This requirement says only that creation refuses without one; it does not restate the probe's answer shape and does not add a reason of its own.

**"MUST NOT generate a key" is stated as a prohibition on this call and not as an observable outcome, because it is not one at this surface.** Creation is handed the means of obtaining a key rather than reaching for a keystore itself, so no caller of this capability can observe whether a key was minted; a scenario asserting that none was would be asserting something no test here can distinguish from the same call succeeding. Whether a key exists afterwards is the `keystore` capability's to answer, and constraining what creates one belongs there. What is checkable here, and what the scenario below states, is that a peer without a usable key gets a failure and no Stoa.

#### Scenario: No key means no Stoa

- **WHEN** creation is attempted on a peer with no usable signing key
- **THEN** the call reports a failure
- **AND** the failure carries the reason the key is unusable
- **AND** no Stoa is recorded as one the peer is in

### Requirement: A title the genesis record cannot carry is refused before a Stoa exists

Creation MUST refuse a title the genesis record has no encoding for, and MUST do so before recording any membership.

The bound itself — what a valid title is, and that a record whose title exceeds the maximum has no encoding at all — is the `stoa-genesis` capability's. What this requirement adds is the ordering: the refusal happens first, so a failed creation leaves nothing behind.

**The bound is a count of bytes in the title's UTF-8 encoding, not of characters**, and this surface MUST report it as such. A title arrives as a JSON string, where the natural reading of a maximum is characters, and the two counts differ for every title outside ASCII: a title of 400 CJK characters is 1200 bytes and MUST be refused against a 1024-byte bound, even though a caller counting characters would expect it to fit. Stating the unit here is not a restatement of `stoa-genesis`'s bound — it is the part a caller of *this* call needs in order to predict which titles it will refuse.

Creation MUST NOT impose a bound the genesis record does not have. In particular an empty title MUST be accepted: the record has no minimum length, the title is not an identifier, and refusing one here would make a record other peers decode and verify without complaint unreachable through this surface.

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

#### Scenario: An empty title is accepted rather than refused

- **WHEN** creation is attempted with an empty title
- **THEN** the call succeeds
- **AND** the Stoa's founding title is empty

#### Scenario: A title carrying control or bidirectional characters is not rejected for that reason

- **WHEN** creation is attempted with a title containing zero-width or bidirectional control characters, within the length bound
- **THEN** the call succeeds
- **AND** the founding title reported afterwards carries those characters unchanged

### Requirement: Creating the same Stoa twice yields one Stoa, not two

A genesis record carries only values every peer agrees on, so a creator making a record with the same title twice makes the **same** record and therefore the same address. The second creation MUST report that address and MUST leave the peer in one Stoa for it, not two.

This is stated because the intuitive expectation is the opposite one, and the honest behaviour is worth pinning rather than discovering. It follows from the record carrying no per-peer state — a requirement the `stoa-genesis` capability owns and that this contract must not quietly contradict by promising uniqueness the encoding cannot provide. A user who wants two Stoas gives them two titles.

#### Scenario: The same creator and title reach the same Stoa

- **WHEN** a Stoa is created with a title, and then created again with that same title by the same peer
- **THEN** both calls return the same address
- **AND** the peer is in exactly one Stoa for that address

#### Scenario: Two titles are two Stoas

- **WHEN** one peer creates Stoas with two different titles
- **THEN** the two addresses differ
- **AND** the peer is in both

### Requirement: Joining takes an address and the record it names, and verifies rather than trusts

The join call MUST take both a Stoa address and a candidate genesis record, and MUST verify the record against the address before recording anything.

**An address alone is not joinable, and that is a property of the address rather than a limitation of this call.** The address is a one-way hash of the record: it is sufficient to verify a record somebody hands over and insufficient to reconstruct one. Since a membership must retain the record — the retention requirement below says what for — the record has to arrive with the address, because there is nowhere else for it to come from. This call's shape says so rather than leaving a caller to discover it after joining.

A join MUST return what the caller needs to show what was joined, including the address and the Stoa's founding title.

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

#### Scenario: Verification consults nothing but the two inputs

- **WHEN** a join verifies a record against an address
- **THEN** the decision uses only the supplied address and the supplied record
- **AND** no index, registry, peer or network call is consulted

### Requirement: Joining a Stoa the peer is already in changes nothing and is not a failure

A join naming a Stoa the peer is already in MUST succeed, MUST leave the peer in exactly one Stoa for that address, and MUST NOT disturb what was already retained for it.

A pasted address is exactly the input a user supplies twice. Reporting the second attempt as an error would make a harmless action look broken, and replacing the retained state would make a join a way to overwrite what a peer already holds.

**"Not disturbed" is specified as what a later read answers, which is the form that can be checked.** Whether the second join rewrote the retained bytes or left them untouched is not observable through this capability: a record that does not verify against its address cannot be recorded at all, so the only record a repeated join could ever write over a retained one is byte-for-byte the record already there. The requirement therefore constrains the answers — the founding values, the retained record's verification, and the count of Stoas for that address MUST all be what they were before the repeated join — and deliberately does not constrain how the write is performed. A scenario asserting that no write occurred would be asserting something this surface cannot distinguish.

**The reply MUST NOT be required to say whether the join was new.** A caller learns it is in the Stoa either way, which is the question a join answers; reporting newness would invite a view to treat a second join as a failure, which is the behaviour this requirement exists to prevent.

#### Scenario: A repeated join is idempotent

- **WHEN** a peer joins a Stoa it is already in, with the same address and record
- **THEN** the call succeeds
- **AND** the peer is in exactly one Stoa for that address
- **AND** the founding values reported afterwards are unchanged
- **AND** the retained record still verifies against the address it is retained under

#### Scenario: Creating and then joining the same Stoa is one membership

- **WHEN** a peer creates a Stoa and then joins it with its own address and record
- **THEN** the peer is in exactly one Stoa for that address

### Requirement: A joined Stoa's genesis record is retained, not only its address

For every Stoa the peer is in, the peer MUST retain the complete genesis record and not merely the address.

The address cannot be inverted, and the `moderation-resolution` capability requires the record before a reader may decide whether any moderation of that Stoa's content binds. A peer that retained only addresses would hold a list of Stoas whose content it can store and cannot judge, and the deficiency would be invisible until a moderation op arrived.

The retained record MUST remain the one that verifies against the address it is retained under. Retrieving it and recomputing the address MUST yield that same address.

**A retained record that does not verify against the address it is retained under MUST be reported as a failure, and MUST NOT be skipped, repaired, or omitted from an otherwise successful answer.** This is the case the sentence above says must not happen, and saying what happens when it does is the difference between a peer that reports it cannot read its memberships and one that quietly lists fewer Stoas than the user joined. A listing that dropped the offending Stoa would be indistinguishable from a listing of a peer that never joined it.

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

### Requirement: Membership survives a restart

Every Stoa the peer is in MUST still be one the peer is in after the process stops and starts again, together with the genesis record retained for it.

This is the whole point of recording membership rather than deriving it: a peer that forgot its Stoas on restart would ask the user to re-paste every address they had ever joined, and for a created Stoa there is nobody to re-paste it from.

**Where membership is stored is fixed by the location the host supplies, and MUST be reached the same way on every run.** Membership state MUST be held separately from the op log rather than inside it — a membership layout this build cannot read must cost the user their Stoa list and never their ops, which the "refused rather than guessed at" requirement below relies on. The particular name of the file is not specified, and a caller MUST NOT depend on it. What is specified is that it does not move: **changing where a build looks, without a conversion, orphans every membership a user holds** — the Stoas are still on disk and the peer reports being in none of them, which is indistinguishable to the user from having been silently removed from all of them.

#### Scenario: Created and joined Stoas both survive

- **WHEN** a peer creates one Stoa, joins another, and is then restarted
- **THEN** both are among the Stoas the peer is in
- **AND** each still answers its founding values

#### Scenario: The retained record survives too

- **WHEN** a peer with a joined Stoa is restarted
- **THEN** the genesis record retained for that Stoa still verifies against its address

#### Scenario: A refused join leaves nothing behind a restart

- **WHEN** a join is refused for a mismatched record and the peer is then restarted
- **THEN** the peer is still not in that Stoa

### Requirement: Adding membership does not make an existing store unreadable, and a layout this build cannot read is refused rather than guessed at

A store already holding ops MUST remain readable once this peer records Stoa membership alongside it. **The absence of membership state MUST NOT be a reason to refuse**: a peer whose membership state has never been written MUST open, MUST report being in no Stoa, and MUST still read its ops.

Membership is state added beside the log rather than a change to what the log holds, so there is nothing to migrate and nothing whose absence makes an older store ambiguous. Refusing to open one would discard a user's ops to avoid a conversion that is not needed.

**Membership state claiming a layout this build does not have is a different case, and MUST be refused in both directions.** A store whose membership layout is stamped with any version other than the one this build writes MUST be refused at open, whether that version is higher or lower, and the refusal MUST name both the version found and the version expected. A layout claim is one integer that anything can write, so it is a claim rather than a fact: a store whose stamped version matches but whose actual layout does not MUST also be refused, rather than being read as though the claim were true.

These two rules do not conflict, and the distinction is the whole content of this requirement. *Never having recorded membership* is the absence of a version, which is the case the first rule protects and which MUST be created cleanly. *Claiming a version this build cannot read* is an assertion this build cannot honour, and reading such a store as though it could is how a user's memberships are silently mis-decoded. Refusing a lower version as well as a higher one is deliberate: an older layout is not a subset of a newer one, and accepting one would mean guessing at a conversion no requirement defines.

A refusal to open MUST be reported as this capability's failure shape and MUST carry the reason — an empty listing MUST NOT be reported in its place. The two are indistinguishable to a view, and a peer whose memberships cannot be read is being told it belongs to nothing, which invites the user to re-join Stoas they are already in.

#### Scenario: A store holding ops and no memberships opens

- **WHEN** a store that holds ops and records no Stoa membership is opened
- **THEN** opening succeeds
- **AND** its ops are readable
- **AND** the peer is reported as being in no Stoa

#### Scenario: A membership is recordable into such a store

- **WHEN** a Stoa is joined against a store that previously held no membership
- **THEN** the join succeeds
- **AND** the ops the store already held are still readable

#### Scenario: A membership layout this build does not write is refused either way

- **WHEN** membership state stamped with a layout version higher than the one this build writes is opened
- **THEN** the call reports a failure naming both the version found and the version expected
- **AND** membership state stamped with a version lower than the one this build writes is refused the same way

#### Scenario: A layout that does not match its own claim is refused

- **WHEN** membership state stamped with the version this build writes is opened, but its actual layout is not the one that version describes
- **THEN** the call reports a failure
- **AND** the failure names the version whose layout was expected

#### Scenario: Membership state that cannot be opened is a failure, not an empty listing

- **WHEN** a listing is attempted and the membership state cannot be opened
- **THEN** the reply is the error shape and carries the reason
- **AND** the reply is not an empty listing reported as a success

### Requirement: Listing reports the Stoas the peer is in, and no others

The module MUST offer a paginated call listing the Stoas the peer is in — those it created and those it joined — each item carrying the Stoa's address and its founding title.

The envelope and the pagination arguments are the `module-wire-contract` capability's; this requirement conforms to them rather than restating them. What it fixes is the contents: exactly the Stoas membership records, and nothing derived from the ops the peer happens to hold.

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

### Requirement: A listed title is a founding title, and is identified as such

A title reported by this capability MUST be the founding title from the retained genesis record, and the reply MUST make it distinguishable from a current title resolved from a moderator-signed metadata op.

The distinction is not cosmetic: a founding title is what the Stoa was created as, possibly long ago, and the `stoa-metadata` capability states both that the current title is carried by a separate op and that nothing resolves those ops yet. A reply presenting a founding title as the current one asserts something no peer has checked. Resolving current metadata is out of scope here; saying which of the two is being reported is not.

#### Scenario: The reported title is marked as founding

- **WHEN** a Stoa is listed, or a join reports what was joined
- **THEN** the title reported is the founding title from the genesis record
- **AND** the reply indicates that it is the founding value rather than a resolved current one

### Requirement: Membership is what the user chose, never what the ops say

Membership MUST be reported from the record of what the user chose — created or joined — and MUST NOT be derived, in whole or in part, from the ops the peer holds. The set of Stoas the peer is in MUST be exactly the set membership records, whatever ops are present alongside it.

This is the direction membership must not be derived from ops. An op is attacker-supplied and its Stoa address is a field the sender chose; a peer that joined a Stoa because an op mentioned it would be a peer any stranger can enrol, and a user who does not know they joined a Stoa is the exact harm the join confirmation exists to prevent.

The requirement is stated over the **material the answer is computed from** rather than over an op's arrival, because arrival is not an event this capability can be shown to experience: there is no receive path on this surface, and until one exists a scenario beginning "when an op reaches the peer" names something no test can stage. What is checkable today is that ops present where an implementation would look for them change no answer this capability gives. **The arriving-op direction becomes specifiable when a receive path exists, and constraining it is that change's to do, not this one's.**

Whether an op is stored, and what happens to it, is the `op-log` capability's; this requirement constrains only what it does to membership. The two answers are independent by design — the log may hold ops for Stoas the peer is not in, and that is not a contradiction.

#### Scenario: Ops for a Stoa the peer never joined enrol nobody

- **WHEN** a peer that has joined one Stoa holds ops addressed to a different Stoa it never joined, stored where this capability's own state is stored
- **THEN** the listing contains the joined Stoa and not the one the ops address
- **AND** the peer is reported as not being in the Stoa the ops address
- **AND** the call reports no failure

#### Scenario: An op store's contents do not disturb the memberships that exist

- **WHEN** a peer in one Stoa holds ops addressed to a different, unjoined Stoa
- **THEN** the peer is still in exactly the one Stoa
- **AND** that Stoa's retained record still verifies against its address

### Requirement: Membership is not lost because a Stoa has no ops

A Stoa the peer is in MUST remain listed however few ops the peer holds for it, including none at all. An op store that exists and holds nothing MUST NOT empty the listing.

This is the other direction of the same boundary. A freshly created Stoa has no ops by construction, and a joined Stoa has none until something propagates — so an answer derived from the log would omit precisely the Stoas a user has just acted on, which is when they are most certain they are in one.

#### Scenario: A Stoa with no ops is still joined

- **WHEN** a peer joins a Stoa and no op for it ever arrives
- **THEN** the Stoa is among the ones the peer is in
- **AND** it is still there after a restart

#### Scenario: An empty op store does not empty the listing

- **WHEN** a peer is in several Stoas and its op store exists, opens, and holds no ops at all
- **THEN** every one of those Stoas is listed

### Requirement: Every one of these calls answers in the module's failure shape and never aborts

Each call in this capability MUST report every failure as the module's single error shape and MUST NOT panic, whatever the request contains: an absent field, a field of the wrong type, an address that is not an address, a record that is not a record, or a record and address that disagree.

The shape itself, and that a failure carries no result alongside it, is the `module-wire-contract` capability's. What this requirement adds is that these calls are inside it — they are the first on this surface that reach persistent state, so a panic here aborts the module process and takes the user's session with it, and a partial reply would leave a view holding an address it cannot tell apart from one that was really recorded.

**A field these calls do not recognise MUST be ignored rather than refused.** A request carrying an unknown field alongside the fields a call requires MUST be answered as though the unknown field were absent. This is what lets a newer view call an older core without every added field becoming a breaking change, and it is the request-side counterpart of the reply-side tolerance `module-wire-contract` already requires. It applies only to fields the call does not read: a field it does read, carrying the wrong type, is a failure by the rule above.

#### Scenario: A hostile request is an error rather than an abort

- **WHEN** any call in this capability is given a request with a missing field, a field of the wrong type, or an address or record of the wrong length or wrong alphabet
- **THEN** the reply is the error shape
- **AND** the module answers subsequent calls

#### Scenario: An unrecognised request field is ignored rather than refused

- **WHEN** a call in this capability is given a well-formed request carrying an additional field the call does not read
- **THEN** the call succeeds
- **AND** the reply is the one the request would have received without that field

#### Scenario: A failed call records nothing

- **WHEN** any call in this capability reports a failure
- **THEN** the set of Stoas the peer is in is unchanged
- **AND** the retained record of every Stoa it is in is unchanged
