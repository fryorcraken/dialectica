## MODIFIED Requirements

### Requirement: A successful publish claims local storage and never delivery

When a publish succeeds, the view SHALL state that the content was stored on this
machine. It SHALL NOT state, or imply, that it was sent, delivered, received by
any peer, propagated, published to the Stoa at large, or that anyone else can see
it.

`content-authoring` requires the reply to carry no delivery outcome at all, and
the outcome a user cares about arrives after the call has returned. An interface
claiming delivery would therefore be claiming something no part of the system has
checked.

The view SHALL NOT display a count of peers reached, a delivery state, or a
progress indicator that resolves into a delivery claim.

**Saying nothing about delivery is not sufficient, and the view SHALL positively
deny delivery knowledge.** Where a publish has succeeded, the view SHALL state,
in the screen's own body, that whether any other peer has received the content is
not something this software can report. That statement SHALL accompany the
success itself rather than being available only elsewhere in the interface.

Every other delivery rule in this capability is a prohibition, and a prohibition
is discharged by silence. Silence is the wrong answer here, because the reader's
default assumption on seeing a forum post submit successfully is that it went
somewhere — so an interface that merely declines to mention delivery lets that
assumption stand unchallenged while being fully compliant.

**The stakes are specific to this system rather than general good manners.** A
publish reply carries no delivery outcome by design; where core hands the op to
the network, it learns nothing of whether any peer received
it; and a post whose body is legal but near the cap encodes to more than the
transport will carry, so it is accepted locally and silently refused by every
receiving peer. An author therefore cannot distinguish a post nobody has received
from one everybody has, and the interface is the only place that fact can be
told to them.

A prohibition-only contract also cannot be tested for. A test can sweep for a
forbidden claim and pass when the honest sentence is deleted, which is how such a
statement is lost: not by someone deciding to remove it, but by it leaving
attached to something else.

#### Scenario: A success denies delivery knowledge rather than omitting it

- **WHEN** a publish succeeds
- **THEN** the view states that whether any other peer has received the content
  is not something it can report
- **AND** that statement is displayed with the success rather than only in
  another part of the interface

#### Scenario: The denial survives without the annotation column

- **WHEN** a publish succeeds and every region of the screen given over to
  annotating the design is disregarded
- **THEN** the denial is still displayed

#### Scenario: A success names local storage

- **WHEN** a publish succeeds
- **THEN** the message displayed states the content was saved on this machine

#### Scenario: A success claims nothing about delivery

- **WHEN** a publish succeeds
- **THEN** the message displayed contains no claim that the content was sent,
  delivered, received, propagated, or is visible to anyone else

#### Scenario: No delivery indicator is rendered

- **WHEN** a publish succeeds
- **THEN** the view displays no peer count, delivery state or delivery progress
  for that content
