## ADDED Requirements

### Requirement: The submit control is disabled while a publish is outstanding

The view SHALL disable its submit control when a publish is submitted, and SHALL NOT re-enable it until that publish has reported an outcome.

**This obligation arrives with the op clock and is a consequence of it.** An op's bytes now carry a counter that advances between two publishes, so two submissions of one draft are two distinct ops rather than one. The deduplication that previously absorbed a double-tapped submit no longer does, and **core cannot restore it**: at that layer a double tap and a person deliberately posting the same line twice are identical acts producing identical-looking ops, correctly signed and correctly ordered. The view is the only layer at which the two are distinguishable, because only the view knows that one gesture occurred.

The failure this prevents is not cosmetic. A person who taps twice publishes twice, both posts appear, and **neither can be removed** — this system has no delete, and a revision replaces a post's content rather than withdrawing it. So the duplicate is permanent, visible to every peer, and the author's only remedy is to edit one into an apology.

The control SHALL be re-enabled on every outcome, including a refusal, because a refused publish stored nothing and the user must be able to retry.

Disabling SHALL be a property of the control rather than a message asking the user to wait. A prompt not to double-tap relies on the person reading it in the moment they are least likely to.

#### Scenario: The submit control is disabled once a publish is submitted

- **WHEN** a publish is submitted
- **THEN** the submit control is disabled

#### Scenario: A second activation during an outstanding publish submits nothing

- **WHEN** the submit control is activated twice in succession with no outcome reported between the two
- **THEN** exactly one publish is submitted to core

#### Scenario: The control is re-enabled when the publish succeeds

- **WHEN** a publish reports success
- **THEN** the submit control is enabled

#### Scenario: The control is re-enabled when the publish is refused

- **WHEN** a publish reports a refusal
- **THEN** the submit control is enabled
- **AND** the draft is retained, so the user can retry

## MODIFIED Requirements

### Requirement: The draft is cleared when the op was newly stored, and kept otherwise

Where a publish succeeds reporting the op was **newly stored**, the view SHALL
clear the draft.

Where a publish succeeds reporting the op was **not** newly stored, and on every
refusal, the view SHALL retain the draft. The three cases are therefore not
uniform, and the asymmetry is the decision rather than an inconsistency.

Clearing on a newly stored op removes an affordance that is ready to produce an
outcome the user did not intend. **The reason has changed, and the behaviour has
not.** It previously rested on the same text submitted again being the same op
id, so that a second submission was a no-op the user reached through a control
that looked ready to publish. That is no longer so: an op's bytes carry a counter
that advances between publishes, so **the same text submitted again is a second
post.** Leaving a published draft in the composer now offers a control that is
ready to duplicate the post rather than one ready to do nothing, which is the
worse of the two outcomes and makes clearing more clearly right than before.
Nothing is lost by clearing, because the text is published and readable.

Keeping it in the other two cases follows from the same reasoning applied to
different facts. On a refusal nothing was published, so the draft is the only
copy. On a publish reporting the op was not newly stored, this peer already held
that exact op, and a user whose intention was to publish something different
needs the text in front of them to edit — clearing would take away exactly what
they need.

The cost of clearing is named: a user writing a near-identical follow-up loses
their starting point and retypes it. That is a convenience, weighed against an
interface offering a control whose use publishes a duplicate that cannot be
removed.

#### Scenario: A newly stored publish clears the draft

- **WHEN** a publish succeeds reporting the op was newly stored
- **THEN** the composer holds no draft

#### Scenario: The draft's fate differs across the three outcomes

- **WHEN** the same submission is made against a core reporting a newly stored
  op, against one reporting an op that was not newly stored, and against one
  returning a refusal
- **THEN** the draft is cleared in the first case
- **AND** retained in the other two

### Requirement: An already-published op is reported as its own outcome

Where a publish succeeds reporting that the op was not newly stored, the view
SHALL display a message stating that this content was already published.

That message SHALL be distinguishable from the message shown for a newly stored
op, and SHALL be distinguishable from the message shown for a refusal. Nothing
failed, so a refusal is wrong; nothing new was stored, so reporting a fresh
success leaves the user looking for a post that will never appear.

The view SHALL NOT display a progress indicator that resolves without a message,
and SHALL NOT report this outcome as an error.

**This outcome has become uncommon and SHALL NOT be dropped on that account.**
It previously arrived whenever one person submitted the same body twice, which
was the ordinary case. It now arrives only when this peer already holds the exact
op — every field matching, counter included — which is a re-publish rather than a
re-authoring. The outcome still reaches the view, core still reports it, and a
view that stopped handling it would present a state it cannot render. **A rarer
branch is a branch that is harder to notice breaking**, which is the reason to
keep its scenarios rather than a reason to trim them.

#### Scenario: A deduplicated publish says the content was already published

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the view displays a message stating the content was already published

#### Scenario: A deduplicated publish keeps the draft

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the draft the composer holds is the text the user entered

#### Scenario: The three outcomes are mutually distinguishable

- **WHEN** the same submission is made against a core reporting a newly stored
  op, against one reporting an op that was not newly stored, and against one
  returning a refusal
- **THEN** the view reaches three states
- **AND** no two of the three display the same message

#### Scenario: An already-published op is not an error state

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the view is not in the state it reaches for a refused publish
