## MODIFIED Requirements

### Requirement: A displayed time is marked as the author's assertion, never as a verified instant

Where the feed renders a post's time, it SHALL render alongside it an indication that the value is what the post's author asserted, and SHALL NOT present it as an instant this peer or any other has verified.

**The feed renders no time today, and this requirement is written to bind when it does rather than to describe something that exists.** The feed listing's rows carry no asserted time: `thread-read` contracts that field on a **thread's** items, and the feed lists threads through a separate reply shape that carries no time. So the requirement's force today is a **prohibition** — the feed MUST NOT render a time it has not been given, which the absent-time clause below states — and its positive half has nothing to bind for exactly as long as the field is absent, becoming unconditional the moment a feed row carries one. It is stated now because the field's arrival is the moment the marking is easiest to omit, and because a screen that acquired a bare time would be making the forgeable-as-verified claim with nothing in the spec to stop it. That the field is *expected* to arrive — deferred rather than declined, and why — is recorded in the `op-clock` change's `design.md`, archived under `openspec/changes/archive/`, in its section "What this deliberately does not do".

Nothing verifies it and nothing can. The wall-clock sits inside the signed preimage, so a relay cannot forge it, but the **author** sets it freely and an author is not trusted — `op-ordering` requires that the value decide no ordering, comparison, resolution or gating for exactly that reason, and hands it to a reader as display text rather than as a number a comparison would accept. A time rendered bare reads as an observed fact, and a reader who takes it as one has been given a forgeable value with the authority of a verified one.

Where `thread-read` reports a time as **clamped**, the feed SHALL NOT present the clamped value as the time the author asserted. The clamp is this peer's substitution for an implausible claim, so rendering it unmarked would attribute to the author a time they did not assert.

Where `thread-read` reports a time as **absent** — an op encoded before the clock fields existed — the feed SHALL render no time for that post and SHALL NOT substitute one. A substituted value is indistinguishable from an asserted one once rendered.

#### Scenario: A rendered time carries its author-assertion marking

- **WHEN** the feed renders a post whose item carries an asserted time
- **THEN** what is rendered indicates the time is the author's claim
- **AND** nothing rendered presents it as verified, confirmed, or observed by this peer

#### Scenario: A clamped time is not attributed to the author

- **WHEN** the feed renders a post whose item reports its time as clamped
- **THEN** what is rendered does not present the clamped value as the time the author asserted

#### Scenario: An absent time renders as no time at all

- **WHEN** the feed renders a post whose item reports its time as absent
- **THEN** no time is rendered for that post
- **AND** no substitute value is rendered in its place
