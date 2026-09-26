## MODIFIED Requirements

### Requirement: A join is reported from the core's reply, never assumed

The screen MUST report a join as having happened only on a successful reply from
the core, and MUST NOT render success on the strength of having made the call.

**A successful reply to a join MUST be reported as a join**, and the screen MUST
NOT render a failure of that join. This holds for a Stoa the peer was not in
before the join, which is the ordinary case, exactly as it holds for one the
peer was already in.

Failure is always the single error shape and never a partial success, which is
what lets one branch decide this. A screen that navigated onward as soon as it
dispatched the call would show the user a Stoa they are not recorded as being in,
and the discrepancy would survive the restart — membership is what the core
retained, not what the screen displayed.

**Joining a Stoa the peer is already in MUST be rendered as success.** The core
reports the same success either way, deliberately: a pasted address is exactly
the input a user supplies twice, and `stoa-membership`'s requirement "Joining a
Stoa the peer is already in changes nothing and is not a failure" makes the
second attempt succeed. The reply does not say whether the join was new, and the
screen MUST NOT infer newness from anything else — from the Stoa's presence in a
listing it fetched earlier, above all — nor present the repeat as an error, a
warning, or a collision to resolve.

#### Scenario: Success is rendered only after a successful reply

- **WHEN** the user acts on the join affordance and the core answers with a
  failure
- **THEN** the screen does not report the Stoa as joined
- **AND** it renders the core's failure

#### Scenario: Joining a Stoa already held is success

- **WHEN** the user joins a Stoa that is already among the ones the peer is in,
  and the core answers successfully
- **THEN** the screen reports success
- **AND** it renders no error, warning, or collision

#### Scenario: Joining a Stoa not held is reported as joined

- **WHEN** the preview's lookup answers a fallback reply for a Stoa that is not
  among the ones the peer is in, and the user then acts on the join affordance
  and the core answers successfully
- **THEN** the screen reports the Stoa as joined
- **AND** it renders no failure of the join
