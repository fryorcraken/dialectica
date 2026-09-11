# Design: the moderation resolver, and where authority is decided

## Context

See proposal.md — Why. The pieces all exist: `op.rs` answers authenticity,
`log.rs` stores everything and decides nothing, `arrival.rs` orders, `stoa.rs`
names a creator. Nothing joins them, and the join is the security boundary.

This change is the first code in the project that can decide authority, so the
decisions below are mostly about *where each check lives* and *what the type
system refuses to let a caller skip*.

## Goals / Non-Goals

**Goals**

- An answer to "is this hidden?" that a peer reaches from the ops it holds and
  nothing else, identically to every other peer holding those ops.
- A shape where the authority check cannot be forgotten by a future caller, rather
  than one where forgetting it is merely discouraged.

**Non-Goals**

- **The revision resolver.** §5.7 is explicit that a moderator's hide and an
  author's edit "are about different things and do not contend". Built in
  parallel, against the same trait, by another change.
- **Threshold / N-of-M moderation** (§6.2). Strictly downstream of a mutable
  moderator set — with one creator-moderator there is no threshold to take.
- **Author-scoped suppression** (§6.1). Deferred with the mutable moderator set,
  "not rejected".
- **A mutable moderator set.** §13's one genuine merge question. See the decision
  below on holding that line.
- **The materialised view.** §3.3 makes the log the authority and the view a cache
  rebuilt from it. This resolver is a function over the log; caching its answers is
  the view's job, and caching them *here* would reintroduce exactly the "decided
  once, trusted thereafter" failure this change exists to prevent.

## Decisions

### Decision: The moderator set is a type, and "unknown" is unrepresentable

The question the task posed was fail-closed versus fail-open when the genesis
record is missing. The answer is **fail closed**, but the more useful answer is
that the resolver should never be in that position, and the type is what arranges
it.

A genesis record is **not an op**. `op.rs` says why: "Creating a Stoa is not here
— a Stoa is a genesis record its creator publishes (`stoa.rs`), and hashing that
record is what creates it; there is nothing for a *signed op* to add." So a
resolver that took a log and a target and went looking for the genesis record
would be looking somewhere it provably is not. Any "genesis record missing"
branch inside the resolver would be a branch that is always taken.

So `Moderators` is constructed from a `Genesis` and from nothing else:

```rust
Moderators::of(&genesis)   // the only constructor
```

and `is_hidden(log, &moderators, target)` takes one. A caller without a genesis
record cannot build the argument, so the call does not compile. That is the
strongest available form of failing closed: the failure is at the call site, at
build time, rather than a runtime value the resolver has to decide what to do
with.

**Why fail closed is right, and not merely cautious.** The two directions are not
symmetric:

- **Fail open** (no genesis → treat every signer as authorised) is the Appendix A
  defect with an extra step. Any peer can then forge a moderation against any Stoa
  whose genesis a reader happens not to hold, and a reader that has not yet
  received a genesis record is the *ordinary* state of a peer joining from a
  pasted address — so the window is not an edge case.
- **Fail closed** (no genesis → no authority, nothing hides) loses moderation for
  a Stoa the peer cannot identify. That is recoverable: the record arrives, and
  the hide applies from then on. It is also the weaker failure, because §6.1 is
  candid that moderation's ceiling is what conforming peers render — a hide
  arriving late renders content that will shortly be hidden, where a forged hide
  arriving at all removes content nobody moderated.

`stoa.rs` set the precedent and stated the same asymmetry for the policy field:
refusing an unknown policy "means an old client cannot display a Stoa it does not
understand, which is the recoverable direction". Defaulting to permissive is not.

**The rejected middle.** `is_hidden(log, Option<&Moderators>, target) -> bool`
returning `false` when the set is absent. It fails closed at the value level and
still loses: `false` from "nobody moderated this" and `false` from "I could not
check" are the same byte, so a caller cannot tell a rendered post from an
unchecked one, and the natural UI — render it — is fail-open behaviour reached
through a fail-closed implementation.

**What would reverse this.** If a later change puts genesis records in the log (a
`StoaAnnounce` op, say, for §4.8's discovery), the resolver gains a genuine
"record absent" case and `Moderators` gains a lookup that can fail. The decision
would then move from the type into a `Result`, and the argument above is what says
which way that `Result` must lean.

### Decision: Three checks, and the third is the one a reader will not expect

Every candidate op passes three independent gates before it can decide anything:

1. **Authenticity** — `SignedOp::verify()`. Is this really from the key it names?
2. **Authority** — is that key in the moderator set? This is the check `op.rs`
   explicitly does not make and Appendix A's kin project never makes.
3. **Scope** — is the op's own `stoa` the Stoa this moderator set governs?

The third deserves its own argument, because there is a plausible reading in which
it is redundant, and that reading is wrong.

**The tempting reasoning.** `op.rs` puts the Stoa address inside the signed
preimage, and `an_op_replayed_into_another_stoa_does_not_verify` proves an op
lifted from one Stoa's channel onto another's fails verification. So — the
argument goes — check 1 already guarantees the op belongs to the Stoa it claims,
and checking the Stoa again is belt and braces.

**Why it is not redundant.** The signed preimage stops an *attacker* rewriting an
op's Stoa. It does not stop the *author* choosing one. A moderator of Stoa A can
sign, entirely honestly, a `Moderate{stoa: B, target: t, action: Hide}`: the
signature is valid, the author key really is a moderator key, and the op is not a
moderation of anything, because that key moderates A and not B. Checks 1 and 2
both pass. Only check 3 refuses it.

The second route to the same hole: op ids are content-derived, so one target op id
is meaningful in whichever Stoa holds it. A reader that ran checks 1 and 2 with
Stoa A's moderator set over an unrestricted target read would apply A's moderator
to an op that named B.

What the signed Stoa address *does* buy is that check 3 is trustworthy: because
the field is inside the preimage, an op that passes check 1 has a Stoa field its
author chose and nobody since has altered. Checks 1 and 3 compose; neither
substitutes for the other.

The resolver therefore compares `entry.op.op.stoa` against the address stored in
`Moderators`, which is why `Moderators` carries the address rather than only the
key set. A set of keys with no Stoa attached would make check 3 impossible to
state, and a caller would have to pass the address separately — the fourth
slightly-different guard CLAUDE.md warns about.

### Decision: All three checks live in one predicate, called from one place

`Moderators::authorises(&entry)` runs all three and is the only route to a
"yes". The resolver's fold calls it once.

The alternative — the resolver spelling `verify() && set.contains(..) &&
stoa == ..` inline — is the shape that decays. It reads fine at one call site, and
a second resolver (author-scoped suppression, when §6.1's is built; the threshold
check, when §6.2's is) copies it and omits one conjunct. CLAUDE.md: "A guard is a
job. Keep it separate, so 'is it called everywhere?' stays a question with an
answer."

Keeping the three inside one method also means the mutation table below has one
place to break each of them, which is the practical test that they are all
load-bearing.

### Decision: The answer names the op, and is not a bool

`resolve` returns:

```rust
pub enum Moderation {
    Unmoderated,
    Hidden(&Entry),
    Unhidden(&Entry),
}
```

with `is_hidden()` as the one-line projection for callers that genuinely only want
the boolean.

Three reasons, in increasing order of how much they matter:

1. §5.7 already argues this for the other resolver — "a moderator acting on a post
   is acting on a version they can name" — and a moderator reversing a hide is in
   the same position: they are acting on a specific judgement, possibly someone
   else's.
2. `Unmoderated` and `Unhidden` are different facts. A target nobody moderated and
   a target a moderator deliberately restored are the same `false`, and an
   interface that wants to say "restored by a moderator" cannot recover the
   difference from a bool. Collapsing them would also make the hostile-input test
   unable to distinguish "the forgery was ignored" from "the forgery was applied
   as an unhide".
3. A bool invites the caller to cache it, and §6.2's whole finding is about a
   check that stopped being run.

The cost is that the return type borrows from the log. That is the same trade
`OpLog::iter` already makes with `Vec<&Entry>`, and the op-log design flags that
return type as the thing Phase 2's SQLite implementation may want to revisit; this
resolver will follow whatever it becomes.

### Decision: Hold the line on the moderator set being a set of one

§13 names concurrent moderator-set edits as "the one genuine merge question in the
design" and says plainly that it "does not arise while the creator is the sole
moderator". This change does not make it arise.

Concretely, that means `Moderators` is not a general set type with `add` and
`remove`. It holds the creator key and a Stoa address, and `authorises` is a key
comparison. `Moderators::of(&genesis)` is the only constructor.

**What would have to change when the mutable set lands**, recorded here so the
next author does not have to rediscover it:

- **`Moderators` stops being a value and becomes a function of time.** §6 makes an
  op valid "only when signed by a current moderator", and §5.7 sharpens that to
  "a moderator at *that* time". So the authority check needs the set *as of the
  candidate op's Lamport position*, not the set as of now. `authorises(&entry)`
  becomes `authorises(&entry, at)` or the set becomes something the fold carries
  and updates as it walks. Today the two are indistinguishable because the set is
  constant, which is exactly why the distinction must be written down rather than
  discovered when it starts mattering.
- **The fold direction may have to invert.** Today it walks the ordering rule's
  sequence and stops at the first binding op, which is sound because authority
  does not vary. Once it does, deciding whether op *N* was authorised may require
  having replayed the moderator-set ops that precede it — a reverse fold — with
  last-write-wins then applying to the ops that survived.
- **Two moderators editing the set concurrently is the open question**, and
  nothing here answers it. It is unreachable today: the only way into the set is
  the genesis record, which is immutable.
- **`log.rs`'s reason for not filtering on append becomes live.** Its second
  argument — "an op that is unauthorised under today's set may be authorised under
  the set that a not-yet-received op establishes" — is currently true but not
  exercised, because today's set is every set. It becomes load-bearing then.

### Decision: Correct under the degraded order and under a real one, with no branch

The resolver never looks at an `Arrival`. It folds over `iter_target`, which is
already in `cmp_ops` order, and takes the first entry that binds.

**The first entry is not the most recent, and this design does not assume it
is.** `cmp_ops` leads with the highest Lamport timestamp only where the transport
supplied one; otherwise — every op today — it falls back to ascending op id,
which carries no recency at all. `arrival.rs` and `log.rs`'s `iter_target` both
say so, and this design cites them rather than restating the argument.

What the fold actually needs is weaker than recency and holds under both
branches: the rule defines *a* first position, and every peer computes the same
one from the ops alone. So the same code is correct before and after the upstream
fix, and no branch here has to be revisited.

The tests exercise both: fixtures with `Arrival::ordered(..)` for the rule as
§5.7 states it, and fixtures with `Arrival::unordered()` for the order production
actually runs today. Testing only the first would leave the shipped path
untested; testing only the second would leave the rule untested.

The resolver invents no Lamport value and reads no clock, which is `arrival.rs`'s
standing rule applied one layer up.

### Decision: Ops of other kinds are filtered here, not by the log

`iter_target` returns revisions, votes and moderations alike, because the op-log
design made "what acts on this subject?" one question rather than one method per
kind. So the resolver filters to `Moderate` itself.

This is the right side of the seam — a fifth op kind should not widen the store's
API — and it has a security dimension worth stating: the filter is one of the
resolver's checks, not a property of its input. A test in which a `Vote` on a
target is the only op naming it proves the filter runs; without it, a resolver
that matched on "has a target and an author in the set" would pass every other
test in the suite, since votes are the only other kind a moderator plausibly
publishes.

## Risks / Trade-offs

- **The authority check is only as good as the moderator set's provenance.** →
  `Moderators::of` takes a `Genesis`, and `Genesis::matches(&address)` already lets
  a caller confirm a record is the one an address names (§4.8). This change does
  not force that check, because a caller holding a `Genesis` obtained by decoding
  its own stored record has already established it. A caller accepting a genesis
  record from a peer must call `matches` first, and that is stated on
  `Moderators::of`.
- **A late-arriving genesis record means a window of unmoderated rendering.** →
  Accepted, and argued in the fail-closed decision: it is the recoverable
  direction.
- **The return type borrows the log.** → Same trade as `OpLog::iter`; revisit
  together with that one, not before.
- **`Unhidden` is a public distinction nothing renders yet.** → It costs a variant
  and keeps a fact that cannot be reconstructed later. The alternative discards
  information at the one point in the system that has it.

## Open Questions

- **Should the resolver refuse a moderation whose target is in a different Stoa
  from the moderation op itself?** It cannot arise through `iter_target` alone,
  since the target op may be absent, and it is not a question this change needs to
  answer: check 3 already binds the *moderation* to the governing Stoa, which is
  where authority lives. It becomes real when the materialised view renders a
  target and a moderation side by side, and is noted so that change does not
  assume it was settled here.
