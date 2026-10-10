# Proposal

## Why

`composer-view` does not say what an unsubmitted draft belongs to, and the view
answers by accident: one composer serves every Stoa and one serves every
thread, so text typed for one target is still in the field when another is
opened, and submitting it publishes it there. An op is signed and cannot be
deleted, so the text reaches a community, or answers a post, it was not written
for.

## What Changes

- A draft is defined as belonging to its **target**: the Stoa address for a
  post, the Stoa address together with the parent op for a reply. A composer
  holds only the draft entered for the target it would publish to.
- A separate safety rule: the view never submits text to a Stoa or parent other
  than the one it was entered for, judged on the publish call that goes out.
- An unsubmitted draft is kept per target for as long as the view stays open,
  with no bound on how many targets hold one, and is back in the field when
  that target's composer is rendered again. It is not kept beyond that, and it
  reaches core only as the body of a publish.
- A draft whose composer is not rendered (a shut posting gate, a failed read)
  stays held and is displayed nowhere until the composer is rendered again.
- A restored draft is not announced.
- The clearing of a draft by a newly stored publish is confined to the target
  that publish named.
- The sentence in *A publish outcome is displayed only on the visit in which the
  publish was made* saying the draft's fate is undecided is replaced by a
  pointer to the requirements that now decide it. The outcome rule itself is
  unchanged.

Out of scope, on purpose:

- Keeping a draft across a restart of the host. Nothing in the module surface
  stores one, and adding that widens the core API.
- Tying a draft to an identity. The view has one identity.
- A reply affordance on a post other than a thread's root. Drafts are defined
  per parent so that such an affordance inherits the rule; this change adds
  none.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `composer-view`: adds what a draft belongs to, the rule that text is submitted
  only to the target it was entered for, per-target retention for the session,
  the fate of a draft whose composer is not rendered, and that a restored draft
  is unannounced; modifies *A publish outcome is displayed only on the visit in
  which the publish was made* to drop its statement that the draft's fate is
  undecided.

## Impact

- `dialectica-ui/src/qml/`: the composer, and the feed and thread screens that
  mount it. View only.
- `dialectica-ui/tests/tst_publish_outcome_visits.qml`: its two `NO SPEC` tests.
  The same-Stoa one keeps its assertion and loses its marker; the cross-Stoa one
  now contradicts the spec and flips. Reply equivalents do not exist yet.
- No change to the core module, its API or the wire contract.
