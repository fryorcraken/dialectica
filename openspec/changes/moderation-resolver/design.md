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

### Decision: The two resolvers' folds are not extracted into one

Recorded because the surface resemblance is strong and the pressure to unify will
recur. Both changes list the other under "deliberately does not build"; neither
said why they stayed apart, which leaves the next reader to rediscover it.

They look like one job: walk `iter_target`, keep the first entry passing a
predicate, fall back when none does. An extracted
`first_binding(log, target, P) -> Option<&Entry>` would serve both. Three things
say otherwise.

1. **This one is no longer a `find`.** It is filter → collect → read the leading
   candidate's `Arrival` → conditionally re-search for a `Hide`. The revision
   resolver is a genuine `find(..).unwrap_or(original)`. Only one of the two fits
   the extracted shape.

2. **The predicates differ in arity, which is the part a shared signature would
   erase.** `is_valid_revision(candidate, original)` is a *relation between two
   entries* — §5.7's authorship rule needs the target op to answer at all.
   `Moderators::authorises(&entry)` is a *property of one entry* checked against
   ambient state the caller supplied. A shared `Fn(&Entry) -> bool` can express
   the second and not the first, so unifying would mean widening it to
   `Fn(&Entry, &Entry)` and passing a dummy on this side — a signature admitting
   it serves two callers.

3. **The result types are not unifiable.** The revision resolver's `None` means
   "not a post, or not held". [`Moderation::Unmoderated`] means "held, and
   nothing bound". Different lattices: one is absence of a subject, the other is
   presence of a subject with no verdict.

**The strongest evidence is historical.** `first_binding` would have fitted this
resolver *before* the hide-bias commit, and would have had to be un-extracted
*after* it. An abstraction that a single security finding forces back apart was a
resemblance rather than a seam — and the resemblance was strongest exactly when
there was least reason to trust it, because both folds were young.

What would change this: a third resolver arriving with the one-entry predicate
shape and the same fallback semantics. Two instances are a coincidence; three are
a pattern, and CLAUDE.md's "do not refactor speculatively" stops applying once the
generality is demonstrated rather than predicted.

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

### Decision: Exhaustiveness sits on the action, not on the op kind

Found when `StoaMetadata` landed as a fifth op kind and the question "what would
a new kind do here?" was asked of each place this module inspects one. The build
passed; the answer was still a defect.

The filter is `matches!(entry.op.op.kind, OpKind::Moderate { .. })`, which is
right and deliberately non-exhaustive: it asks "is this a moderation?", a new
kind should simply not match, and forcing the compiler to ask about every future
kind at a *filter* would be noise.

The final conversion was not. It read:

```rust
match deciding.op.op.kind {
    OpKind::Moderate { action: Hide, .. } => Hidden(deciding),
    _ => Unhidden(deciding),
}
```

That `_` covered two unrelated things: a `Moderate{Unhide}`, which is correct,
and **any other op kind**, which is unreachable — but unreachable only because
the filter thirty lines earlier established it, not because anything at the match
said so. So a fifth kind reaching that line would have been reported as
`Unhidden`: a fail-open default, thirty lines from the only thing preventing it.

**Why nothing caught it.** The compiler had nothing to object to, because the
wildcard is exhaustive by construction. And **no test could notice, because every
test reaches that line through the filter** — the invariant that makes the arm
unreachable is the same invariant that stops any fixture exercising it. It
survived security, blind spec-test, design and architecture review; none of the
four is looking for an arm that cannot be reached.

This is the fourth instance of the defect family in
`.claude/agents/README.md`, and the only one the compiler *could* have caught.
The other three were fixtures where two rules agreed, or a check retiring a test
in front of it; this one the compiler was willing to check and was told not to.

**The fix moves the exhaustiveness rather than adding a guard.** Destructure the
kind, then match on the `ModerationAction`, so the compiler checks exhaustiveness
over the action — which is the enum whose variants actually determine the answer.
Verified by deleting the `Unhide` arm and watching the build fail with
`non-exhaustive patterns: &ModerationAction::Unhide not covered`, which makes it
the compiler's property rather than one this document asserts.

The kind mismatch becomes a stated impossibility with a chosen outcome:
`Unmoderated`, not `Unhidden`. A non-moderation op deciding a moderation question
is a bug in the filter, and the safe reading of a bug is that nothing was
moderated — the same asymmetry as the `Hide` tie-break, applied one layer down.

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

**When it is revisited, measure against this resolver rather than the revision
one.** `resolve` collects a second `Vec` on top of `iter_target`'s — the filter
to binding candidates — because the degraded branch has to re-search the
candidates after inspecting the leader, which a single pass cannot do. The
revision resolver allocates once. So the two are not interchangeable as cost
estimates, and the worse of them is the one to size a streaming read against.
Not worth restructuring now: §3.3 puts read traffic on the materialised view, so
a resolver over a peer's whole history is a rebuild rather than a render.

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
- **The blast radius of one compromised key.** Because any moderator may reverse
  any moderation (the marker on
  `the_authority_predicate_consults_the_set_and_not_the_earlier_ops_author`),
  a single compromised moderator key can `Unhide` **every** moderation in the
  Stoa — and with the creator as sole moderator there is no recovery short of
  forking the Stoa. That is the cost of the reversibility §13 asked for, and
  §6.2's threshold certificates are the intended answer: an action takes N of M
  signatures, so one key stops being enough. Recorded here because the choice is
  right and the exposure is real, and a reader weighing the marker should see
  both.
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
one from the ops alone.

**That argument is correct about the fold and was wrong about the decision, and
the gap between those two is worth naming.** It establishes *convergence* —
every peer agrees — and then quietly treats convergence as sufficient. It never
asks whether last-write-wins is meaningful at all when there is no "last". It is
not, and the next decision is the consequence.

### Decision: `Hide` wins the tie where nothing was transport-ordered

Found by security review, after the reasoning above had been written and
believed.

A `Moderate` op is fully determined by `{stoa, author, target, action}`. No
nonce, no timestamp, no free byte. So for one Stoa, one moderator and one target
**exactly two ops can ever exist**, with two fixed op ids — and since every
arrival today is unordered, the comparison between them is a constant. Whichever
id is lower would win permanently: not "until something newer arrives", but
forever, because nothing newer can be constructed.

That turns a bare `Unhide` into a **pre-emptive veto**. Publish one naming a
target nobody has moderated, discard the key, and if the pair hashes the wrong
way that target can never be hidden by anyone on any conforming peer. It is
grindable, too: the creator picks the Stoa title, the title fixes the address,
and the address is inside both op ids — review found a favourable title in four
attempts, and `a_stoa_where_the_hide_hashes_lower` makes that search repeatable.

So when **neither** candidate was transport-ordered, a `Hide` beats an `Unhide`
regardless of op id.

**Why this direction.** The two errors are not symmetric. An `Unhide` winning
wrongly un-moderates content with no remedy any moderator can reach; a `Hide`
winning wrongly leaves something hidden that a moderator can lift the moment
real ordering arrives. Fail-safe is the recoverable side, and it is the same
asymmetry `stoa.rs` used to refuse an unknown policy rather than default to
`open`.

**Confined to the degraded branch**, which is the part most likely to be got
wrong by a later edit. Where the transport supplied Lamport values, §5.7's rule
is real and last-write-wins stands untouched — biasing there would make every
hide permanent, a worse bug than the one being closed.
`a_transport_ordered_unhide_still_reverses_a_hide` pins it.

**The condition asks about the leading candidate, not about all of them**, and
that distinction is now load-bearing outside this module — do not "simplify" it
without reading what depends on it.

The consequence for the log's API is **op-log's** to state, and it does, under
"The ordering regime: pressure this contract does not answer, and the shape to
avoid". It names this resolver as the motivating case. Not restated here: one
argument, one home, and that home is the contract the shape would have been
added to.

What belongs here is only why *this* resolver needs the leader's arrival rather
than the read's. A read can hold unordered ops that all fail authority while
every surviving candidate is transport-ordered; asking about the read would
demote that case to the degraded branch for nothing. The question is about the
op that is *about to decide*, so that is the op whose arrival is read.

**Why not in `cmp_ops`.** Two reasons. It is moderation semantics, and a general
comparator has no business knowing that one op kind's payload is safer to
prefer; and `cmp_ops` orders *all* ops, so a bias there would silently reach the
revision resolver too.

**The rule has three cases, not two, and the third is untested.** Written out:

1. the leading candidate was transport-ordered → it decides;
2. otherwise, if any candidate is a binding `Hide` → that one decides;
3. otherwise → the leading candidate decides, by the ordering rule's position
   alone (`unwrap_or(first)`).

Case 3 is the fall-through when every binding candidate is an `Unhide`, and
**nothing exercises it today**. Two binding `Unhide`s of one target require two
distinct authors who both moderate the Stoa, which requires the mutable
moderator set §13 defers — so the arm is unreachable from any fixture that can
currently be built. It is written rather than omitted because omitting it would
mean no answer at all in that case, and because it is what the ordering rule
already says; but a reader should know it is reasoning, not tested behaviour.

This is recorded here specifically because the honest statements about it live
in a test comment and in `tasks.md`, and **both of those are archived with the
change** while this file is the durable record. When the mutable moderator set
lands, the fixture to add is two moderators each publishing an `Unhide` of one
target, and case 3 is what it should pin.

**What it costs, stated plainly.** Until Lamport values arrive, an `Unhide`
cannot reverse a `Hide` of the same target — reversibility, which §13 asked for
and `op.rs` named `Unhide` to provide, is suspended in the degraded order. That
is a real loss and it is the smaller one. It resolves itself when the upstream
gap closes, with no change to this code.

**What would reverse this.** Real Lamport values reaching the resolver: the
tie-break then never fires, because the ordered branch takes every comparison.
It becomes dead code that should be deleted rather than left as a trap, and
`a_transport_ordered_unhide_still_reverses_a_hide` is the test that will still
pass when it is.

So the same code is correct before and after the upstream fix, and the one
branch that has to be revisited is named here.

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
