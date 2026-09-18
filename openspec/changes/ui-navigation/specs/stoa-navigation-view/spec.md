## REMOVED Requirements

### Requirement: The view holds no Stoa of its own, and the feed is reached from the list

**Reason**: The requirement is about a transition — where the feed's Stoa comes
from and what travels with it — rather than about what the Stoa screens render,
which is what this capability's Purpose scopes it to. It is extracted to
`view-navigation`, which is added in this same change, because routing is now
asserted by three capabilities that each say in their own text that they cannot
own it: this one names the feed-to-list return as outside itself, and
`view-identity-onboarding` states that the onboarding screen "SHALL NOT navigate
anywhere itself" and that neither its route in nor its route out is contracted by
it. The generality is demonstrated rather than anticipated, which is the
condition for moving requirements rather than duplicating them.

**Migration**: The requirement text moves to `view-navigation` verbatim, with
every scenario unchanged. Nothing about the required behaviour changes and no
implementation needs to move: a reader looking for what the feed's Stoa may be,
and what must never be invented in place of a genesis record, finds the same
words under the new capability.

### Requirement: Every state a user can enter has a specified way out

**Reason**: Extracted to `view-navigation` for the same reason as the requirement
above — a way out of a state is a transition, and the requirement's own text
already says so, stating the rule "as a property of the screens rather than as a
list of buttons". It also carried the clearest evidence that this capability was
the wrong home for it: the one transition it could not contract, the return from
a Stoa's feed to the list, was excluded on the ground that closing it "is a change
to a screen this capability does not own".

**Migration**: The requirement text moves to `view-navigation` with its three
existing scenarios unchanged. **One change rides with the move and is not part of
it**: the feed-to-list carve-out is removed and the transition is required, with a
fourth scenario covering it, because the route it said did not exist now does.
That correction is stated in `view-navigation`'s copy of the requirement rather
than left as a silent difference between the two versions — a reader comparing
them sees which clause changed and why. Every other clause, including the rule
that a return must not be withdrawn by the state it exists to leave, is identical.
