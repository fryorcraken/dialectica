## MODIFIED Requirements

### Requirement: Every state a user can enter has a specified way out

A user who reaches the join preview MUST be able to return to the list without
restarting, both before acting and after a join has succeeded; a Stoa created
through the create affordance MUST reach the list without a restart; a Stoa
joined through the join preview MUST reach the list without a restart; and a
user who has opened a Stoa's feed MUST be able to return to the list without a
restart.

**The general rule is that arriving somewhere is half a transition**, and a spec
that pins only the arrival leaves the return implemented by habit rather than by
contract. That is not hypothetical here: a return route that no scenario requires
can be deleted with every test still passing, so it is unprotected precisely
because it works. Stating it once, as a property of the view's states rather than
as a list of buttons, is what stops the next view piece rediscovering it.

**A return is specified as an outcome, not as a mechanism.** Whether it is a
cancel affordance, a back affordance, or an automatic return when a join
completes is a design decision; what is required is that the user reaches the list
again without restarting, and that the affordance which does it is not withdrawn
by the very state it exists to leave. A control hidden once the user has succeeded
is a control absent exactly when the return is needed.

The join preview's return MUST remain available after a successful join. Reporting
success and offering nothing further is a terminal state, and a user who has just
joined a Stoa is the user most likely to want to open it.

**The feed-to-list return is required here, and was previously carved out of this
requirement as unspecifiable.** That carve-out recorded that no route existed and
that none could be specified without the feed gaining a way to say it is finished.
The feed now has one, so the transition is contracted rather than named as a gap —
and until it was, it was a working route no scenario protected.

#### Scenario: The preview can be left without joining

- **WHEN** the preview is rendered and the user declines it
- **THEN** the list is rendered again
- **AND** no join call has been made

#### Scenario: The return is still available after a join succeeds

- **WHEN** a join has succeeded and the joined outcome is rendered
- **THEN** an affordance returning to the list is still offered
- **AND** acting on it renders the list

#### Scenario: A created Stoa reaches the list without a restart

- **WHEN** a Stoa is created successfully
- **THEN** the listing is read again
- **AND** the created Stoa is among the rows rendered, without the view being
  restarted

#### Scenario: A joined Stoa reaches the list without a restart

- **WHEN** a join of a Stoa the peer was not in has succeeded, and the user acts
  on the affordance returning to the list
- **THEN** the listing has been read again since the join succeeded
- **AND** the list is rendered
- **AND** the joined Stoa is among the rows rendered, without the view being
  restarted

#### Scenario: The list is reached again from a Stoa's feed

- **WHEN** a Stoa's feed is rendered
- **THEN** an affordance returning to the list is offered
- **AND** acting on it renders the list, without the view being restarted
