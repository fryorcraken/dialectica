# Design: an interim engagement score, and the trigger that retires it

## 1. What the projection schema must reserve — read this first

This section is separated and placed first because the SQLite projection schema
is being designed in parallel, and §7.2 rule 5 says the decay decision belongs
to that schema: "retrofitting an index onto a time-varying score is the
expensive version."

### The finding: there is no age, and decay cannot be computed

§7.2's reserved shape is `score = f(engagement) · decay(age) · weight(claims)`.
**`decay(age)` is not computable in dialectica today**, and this is not a matter
of not having got to it. Three specs close every route:

- `op-format`: "An op SHALL NOT carry a Lamport timestamp, a transport message
  id, a **wall-clock timestamp**, a sequence number, a session counter" —
  because "a wall clock is a field the adversary sets." Appendix A measured
  exactly that failure in the kin project: decay read an author-asserted
  timestamp with nothing clamping it, so a post claiming a future time got an
  unbounded multiplier above 1.
- `op-ordering`: a peer "SHALL NOT substitute a **local clock reading**, an
  arrival counter, or a default value for a metadata value it did not receive."
- The only clock the transport offers is `channelMessageReceived`'s `timestamp`,
  and the `op-ordering` design records what it actually is: "this peer's wall
  clock at the moment its C++ callback fired... Two peers receiving the same op
  record different values." It is a *receive* time, not an *authorship* time.

And the Lamport timestamp, where the transport supplies one, is **a counter, not
a duration.** You cannot compute "days old" from it. Lamport 900 against Lamport
100 says one op follows the other; it does not say whether an hour or a year
passed, and in a quiet Stoa the same gap spans months.

**So decay is deferred, not designed** — and the honest statement is that it is
blocked on the transport supplying authorship time, which is not on any roadmap
here.

### What follows for the schema — three things to reserve

The schema agent needs these three and nothing else. Each is cheap now and is
the expensive retrofit rule 5 warns about later.

**(a) The decay epoch is the op's Lamport timestamp, `-1` when absent — and
decay stays disabled.**

The schema agent has already chosen this and it is the right choice.

**An earlier draft of this document proposed a write-once `first_seen_unix`
instead, and that was wrong.** The reasoning was that a receive-clock reading is
at least unforgeable by an author, where a Lamport counter cannot express
duration. But `arrival.rs` refuses the transport's `timestamp` because it is a
per-peer `CLOCK_REALTIME` read, and storing one anyway would make two peers rank
the same ops differently **for a reason unrelated to which ops they hold** —
which is not the divergence rule 1 blesses. Rule 1 blesses divergence that
follows from different op sets; a clock skew is just a wrong answer that looks
like one. Take the Lamport epoch.

This makes the finding sharper rather than softer: **there is no decay input at
all.** A Lamport timestamp orders two ops without saying whether an hour or a
year separated them, so `decay(age)` is uncomputable from it, and the honest
`-1`-for-absent encoding means most ops today have no epoch either. Decay ships
as `1.0` and is blocked on the transport supplying an authorship time, which
nothing currently plans to. The column pair is reserved so that the day one
arrives is a scorer change and not an index retrofit.

**(b) A stored decay-free score column plus that timestamp, per rule 5 —
not a score computed at read time.**

Rule 5's requirement is unchanged and this design honours it: store
`engagement_score` (decay-free, a pure function of the vote ops held) and apply
decay in the `ORDER BY` expression against `first_seen_unix`. Since decay is
deferred, v1's `ORDER BY` is over `engagement_score` alone — but the column
pair must exist now, because adding a *second* column to an index later is the
retrofit.

**Index: `(stoa_id, engagement_score DESC, op_id ASC)`.** The `op_id` tail is
not decoration — it is what makes the ordering total and identical on every
peer when scores tie, which they will constantly at low vote counts (every
zero-vote post ties). Without it, `LIMIT`/`OFFSET` pagination returns
overlapping or skipped rows between pages, which is the §2.5 paginated contract
silently broken.

**(c) A `moderator_votes` count kept separate from `plain_votes`, not pre-summed
into one weighted total.**

This is the one that is genuinely expensive to retrofit, and it is the reason
this section exists. The weighting constant (§3) is a tuning knob rule 1 says we
can change without a protocol bump — **but that promise is only true if the
schema kept the inputs separable.** A schema storing `weighted_total = plain +
K·moderator` has baked `K` into stored data: changing `K` then means a full
replay of every vote op, and the projection cannot even tell which stored rows
used which `K`. Storing the two counts separately makes re-weighting an
`ORDER BY` expression change, which costs nothing.

The same argument extends one step: the columns should be **counts of distinct
voting identities per direction and per credential class**, so that
`(plain_up, plain_down, mod_up, mod_down)` are four separate integers. Four
columns now; any weighting later.

### (d) The filter problem: exclusion is not rule 5 wearing a different hat

The schema agent asks whether a score must be indexable **jointly** with the
hidden/not-hidden filter, observing that `moderation.rs` resolves hidden-ness on
read from the genesis record plus the ops held, so "the top 20 non-hidden posts
by score" cannot be answered by an index over score alone — you rank, filter,
come up short, and rank again.

**The observation is correct and the diagnosis is not. It is a different problem
with a much cheaper answer, and the difference is boundedness.**

Rule 5's problem is that *every row's sort key changes continuously*. A clock
advances, so at every instant every post's decayed score is a different number;
an index over it is stale the moment it is written, and there is no finite set of
rows you could fix up.

The moderation filter is nothing like that. **The set of hidden posts in a Stoa
is small, changes only when a moderation op arrives, and is enumerable.** A Stoa
has one moderator today (the creator, per `moderation-resolution`), moderation
ops are rare relative to posts and votes, and every change to the hidden set is
triggered by a specific arriving op. Nothing varies in between. That is not an
unindexable property — it is a **derived set with explicit invalidation points**,
which is the ordinary case for a materialised column.

**So the answer is: reserve `is_hidden` as a stored boolean on the per-target
view row, maintained on moderation-op arrival, and index it as the leading
column: `(stoa_id, is_hidden, engagement_score DESC, op_id ASC)`.**

Putting `is_hidden` **before** the score is what makes the index answer the whole
query: SQLite seeks to `(stoa, 0)` and walks descending score, so `LIMIT 20`
reads twenty rows. Putting it after — or leaving it to a `WHERE` over a
read-resolved function — is the rank-filter-repeat loop the schema agent
describes.

Three constraints, or the column becomes a second source of truth that can
disagree with `moderation.rs`:

- **`moderation.rs` stays the only implementation of the rule.** The column is a
  cache of its answer, written by calling it, never by a second copy of the
  authority logic in SQL. Two implementations of one rule is the failure
  `post-revision` already names: they produce no error, only two peers rendering
  differently.
- **It is recomputed, not toggled.** On a moderation op's arrival, re-resolve the
  target through `moderation.rs` and store the result. Do not flip the boolean
  from the op's own action — the arriving op may not be the deciding one
  (`moderation-resolution` has a whole requirement about unauthorised ops not
  displacing authorised ones, and about the degraded order's `hide` preference).
- **It must be recomputable in bulk**, because the genesis record can arrive
  *after* the ops it authorises. `moderation-resolution` says a reader with no
  genesis record "cannot report that Stoa's targets as either hidden or not
  hidden" — so a Stoa's rows may need a sweep when its genesis lands. Treat that
  as ordinary replay, not a special case.

**Why this does not collide with rule 4.** Rule 4's requirement is that hidden
posts are *excluded rather than demoted*, and a stored boolean used as a leading
index column excludes exactly as completely as a read-time filter does. Rule 4
constrains the semantics; this is the storage. The rule it must not break is
that the boolean never becomes a *score adjustment* — which is why it is a
separate indexed column and not, for instance, a large negative score offset.
That encoding would be indexable too and would silently reintroduce the haircut,
since a sufficiently upvoted hidden post would climb back.

**And this is why the two halves of the schema agent's question really were one
question.** The joint-indexability answer is cheap *only because* a moderator's
downvote is a pure ranking term (§6) rather than an exclusion-like one. Had the
moderator's downvote been given exclusion-like force — a threshold at which a
post disappears — then hidden-ness would depend on a **continuously accumulating
vote count** rather than on a rare moderation op, and it really would become
rule 5's problem: a filter whose value changes every time anyone votes, over an
unbounded set of rows, with no enumerable invalidation points. The `K`-applies-to-
upvotes-only decision and the score floor are what keep the exclusion set small
and event-driven, and they should be understood as load-bearing for the schema
and not only for the semantics.

### Summary for the schema agent

| Reserve | Why |
|---|---|
| Decay epoch = Lamport timestamp, `-1` absent — **as you have it**; my `first_seen_unix` draft was wrong | a per-peer `CLOCK_REALTIME` read diverges for reasons unrelated to which ops a peer holds, which is not the divergence rule 1 blesses |
| `engagement_score` stored decay-free, `ORDER BY` applies decay (`1.0` today) | rule 5 |
| **Index `(stoa_id, is_hidden, engagement_score DESC, op_id ASC)`** | the `op_id` tail keeps the order total, so `LIMIT` pagination cannot repeat or skip — equal scores are the ordinary case, since every unvoted post ties |
| **`is_hidden` stored on the per-target view row**, written by calling `moderation.rs`, recomputed on moderation-op arrival and on late genesis | the filter is a bounded, event-invalidated set, not rule 5's continuously-varying key — see (d) |
| Vote aggregation partitioned by voter, **never pre-weighted or pre-summed** — **as you have it** | weight is not a property of a vote (authority resolves on read), and baking `K` in makes rule 1's retune-freely promise false |

**On the per-target aggregate table you flagged as not reserved:** yes, the
scoring shape needs one, and it should become a view-change column rather than
something the log grows. Its shape is per `(stoa_id, target_op_id)`:
`is_hidden`, `engagement_score`, `decay_epoch`, and the vote counts partitioned
by voter class. The log storing raw `(target, voter, direction)` facts and
deciding nothing about them is right and should not change — this table is the
projection of those facts, rebuilt by replay like everything else.

Everything else in this document can change without touching the schema. These
cannot.

## 2. The scoring shape, and where §7.2's reserved shape does not fit

### The reserved shape does not accommodate this, and the mismatch is instructive

§7.2 line 1330 reserves:

```
score = f(engagement) · decay(age) · weight(author_claims)
```

The owner's proposal does not fit it, for a reason worth stating rather than
patching over: **`weight(author_claims)` weights the post by its *author's*
credential. The owner is weighting a *vote* by its *voter's* credential.** Those
are different terms in different positions, and conflating them is precisely the
error Appendix A measured — OpChan's `verification_multiplier` multiplies the
whole post score by the author's ENS status, which is why an ENS holder's
advantage is "a flat 25% however many votes are in play" and why three free
votes beat it.

An author multiplier scales a quantity the attacker already controls. A voter
weight changes what the quantity is made of. The shape must therefore move the
weight **inside** the engagement term:

```
engagement = Σ_voters  w(voter) · dir(vote)
score      = engagement · decay(age)          [decay deferred, = 1.0 today]
```

with `w` a function of the voter and `dir ∈ {+1, −1}`. There is **no author term
at all**, and its absence is deliberate: nothing about who wrote a post should
multiply how much other people liked it. If author standing ever matters it
belongs as its own additive term, never as a multiplier on engagement.

§7.2's reserved shape should be corrected to this. It is a documentation change,
not a migration — nothing was built against the old shape.

### The concrete v1 function

```
engagement = (mod_up − mod_down)·K + (plain_up − plain_down)
```

counting **distinct voting identities**, most recent vote per identity per
target (by the ordering rule — Appendix A's fourth finding is that the kin
project deduped votes by inequality rather than `>`, so an older vote could
overwrite a newer one; use the ordering rule already specified, do not invent a
second).

Hidden posts are **excluded before scoring**, not scored and demoted (rule 4).

## 3. What a moderator's upvote is worth: K = 3, and why a number at all

The brief asks for a justified number, on the grounds that "a multiplier nobody
can justify is a number someone will change arbitrarily." The honest answer has
two halves.

**No principled derivation of K exists, and anyone claiming one is fitting a
curve to a preference.** There is no quantity "a moderator's opinion is worth
N users' opinions" measured anywhere, and inventing a formula would dress a
preference as a finding — the failure mode Appendix A catches OpChan's
architecture doc in (it "documents a completely different set of constants... it
does not describe what runs").

**What can be derived is the constraint K must satisfy**, and that is the useful
part:

- **K > 1**, or the moderator's vote is not a credential at all.
- **K must not exceed the typical engaged-thread vote count**, or one moderator
  vote dominates every organic signal and the ranking is just "what the
  moderator liked" with extra steps — at which point say so and build a pinning
  feature, which is an honest thing to want and a different feature.
- **K must be small enough that being wrong is survivable**, because there is no
  evidence behind it.

In a Stoa where an active thread draws single-digit to low-double-digit votes,
that brackets K to roughly 2–5. **K = 3** sits in it. The justification for the
specific value is: it is the smallest value that visibly moves a post without
being able to dominate a thread that has any organic engagement at all, and
**it is a number we expect to change.**

That last clause is the actual design decision, and it is why §1(c) matters: K
lives in the `ORDER BY` expression, never in stored data. **K is checked in as a
named constant with a comment saying it is unjustified and which observations
would justify changing it** — an interface where a moderator's endorsement is
routinely invisible (K too low) or where moderator-touched posts fill the top of
every feed regardless of engagement (K too high).

A number with a recorded provenance of "chosen, not derived" is safe. A number
with a fabricated derivation is not, because the next reader trusts it.

## 4. The sybil arithmetic, and what it actually shows

### The arithmetic

With `K = 3`, **three minted identities equal one moderator upvote.** Four beat
it.

That is numerically the same failure Appendix A measured: "three free sybil
upvotes outrank holding an ENS name." Reproducing the kin project's exact
number, by coincidence of constants, is worth sitting with rather than
explaining away.

**But the ratio Appendix A calls "the durable point" is different, and that
difference is the whole argument.** In OpChan the credentialed voter's premium
was +0.10 against a raw vote's +1.00 — **a tenth of the signal it rode on**. The
credential garnished the attacker's own lever. Here the credentialed vote is
**3× the raw vote**, so the credential is 30× better placed than OpChan's. That
is a real improvement and it is still nowhere near enough, because:

**Against an attacker willing to mint, no finite K helps.** Minting is free and
unbounded. K = 3 costs the attacker four identities; K = 100 costs 101; K = 10⁶
costs a loop. **Every K is defeated at the same price: the attacker's time to
mint, which is approximately zero.** This is rule 3's argument, it is correct,
and nothing in this design refutes it.

### So the defence is not arithmetic — and that must be said plainly

**Nothing in this design stops the sybil attack.** The interim score is safe for
exactly one reason: **there is currently nobody to attack it.** Phase 2 of §4.8
(broadcast discovery) is unbuilt, the transport is unbuilt, and a Stoa is
reachable only by someone pasting an address a human gave them.

That is a real answer and a temporary one. It is a statement about the
environment, not about the design — and a defence that rests on the environment
**expires when the environment changes**, which is what §6 exists to catch.

Any claim of the form "K is large enough" would be false. The claim this design
makes is "no attacker is present, and here is how we will know when one is."

## 5. Downvotes: counted, and one shape refused

### What a downvote means when identities are free

Appendix A found the real-world signal is upvote-only — the kin project
collected downvotes and filtered them out of every scorer. The op format already
carries both directions, deliberately, leaving the decision to the scorer.

**Downvotes are counted in the interim score, and the asymmetry the brief asks
about is real: a downvote enables an attack an upvote does not.**

Upvote sybilling **promotes** a chosen post. The attacker must have a post they
want promoted, and the result is visible — a post at the top of a feed is the
most examined object in the forum, so the attack advertises itself.

Downvote sybilling **buries an arbitrary post**, chosen by the attacker, and the
result is *invisible by construction*. Nobody notices a post that is not there.
A moderator cannot review what they never see, and the author cannot tell
brigading from indifference. It is a censorship primitive with no audit trail,
in a project whose §1 premise is censorship resistance.

That asymmetry demands an asymmetric fail-safe, and it is the **floor**:

**A post's score is clamped at zero from below for ordering purposes. Negative
engagement orders a post last among visible posts; it never removes it, and it
never orders it behind a hidden post, because hidden posts are not in the
projection at all (rule 4).**

The reasoning: the worst outcome of unchecked downvoting is invisibility, which
is moderation's effect achieved without moderation's authority. The floor caps
the attack's payoff at "ranked last in a list you are still on", which is
recoverable — a reader can reach it, an author can see it, and a moderator can
observe the brigade. The worst outcome of unchecked *upvoting* is a bad post at
the top, which is annoying and self-announcing.

**The fail-safe direction differs from the positive case, and this is the
general rule**: when a signal is uncertain, fail toward *visible* — a wrongly
promoted post is seen and corrected; a wrongly buried post is neither.

### The shape refused

**Downvotes must never trigger auto-hiding at a threshold.** A score that
crosses a line and removes a post is moderation performed by whoever can mint
the most identities, and it converts a ranking nudge into the binding effect
§6.1 reserves for a signed moderator op. The floor above exists precisely so
this is structurally unavailable rather than merely not implemented.

## 6. Rule 4 and the moderator's downvote: a real collision, and the resolution

Rule 4 says "moderation filters, it does not penalise" — a hidden post is
excluded, not demoted, because "a percentage haircut does not bind."

**A moderator's downvote collides with this, and the collision is not cosmetic.**
It is a moderator using their credential to reduce a post's visibility by a
percentage — which is exactly the haircut rule 4 rejects, arriving from the
other direction. Rule 4 stopped `hide` from being *weakened* into a ranking
nudge; nothing stopped a ranking nudge from being *strengthened* by moderator
authority into a soft hide.

Three properties make it worse than an ordinary downvote:

- It has **no reversal op**. A `hide` has `unhide`, specified and reversible
  (`moderation-resolution`). A moderator's downvote is reversible only by the
  moderator's own vote flip, which the vote ops support — but nothing *names*
  it as a moderation decision, so there is no deciding op to point at. The
  moderation capability's requirement that "the deciding moderation is named,
  not merely counted" has no counterpart here.
- It is **not distinguishable from an ordinary user's downvote** in the UI
  unless we build that, so the reader cannot tell a moderation signal from a
  peer's opinion.
- It **does not bind**, in exactly rule 4's sense: a well-upvoted post survives a
  moderator's downvote, so a moderator reaching for it to suppress something
  gets an unreliable tool and may not notice.

**Resolution: `K` applies only to a moderator's upvote. A moderator's downvote
counts as 1, exactly like anyone else's.**

This is the narrowest cut that resolves the collision. A moderator's endorsement
is a curation signal the design wants; a moderator's suppression is what `hide`
is for, and it binds. Giving suppression an amplified but non-binding form
offers a moderator a worse version of a tool they already have — which is how a
moderator ends up using the weak tool because it is one click nearer, and
believing something is suppressed when it is not.

**Stated as a rule: a credential may amplify a signal that promotes; suppression
is a binding judgement or it is nothing.** That is rule 4's principle applied to
the case it did not anticipate.

## 7. Reversibility: rule 1's promise, tested rather than quoted

Rule 1 says a score is "a local projection, never an op... rebuilt by replay",
so "ranking can be retuned without a protocol version bump." The brief asks
whether a *later* switch to credential-gated counting costs anything beyond
changing the scorer.

**Tested against the switch that will actually happen** — from "count everyone,
weight moderators by K" to rule 3's "count only credentialed votes, claimless
contribute zero":

| What changes | Cost |
|---|---|
| The `ORDER BY` / score expression | A code change. Free. |
| Stored vote ops | **Nothing.** Every vote op ever published stays valid and correctly signed; only how they are counted changes. |
| Wire format | **Nothing.** No version bump. Vote ops already carry both directions. |
| The projection | Rebuilt by replay from the op log — which is what a projection is. |
| **History** | **Nothing needs reinterpreting.** This is the load-bearing answer and it is genuinely true: the switch is a *filter* applied to a set that was already recorded in full. A claimless vote from 2026 does not need a credential retroactively; it simply stops counting. |

**Rule 1's promise holds — with one hole this change closes.**

The hole is §1(c). If the projection stores a *pre-summed weighted total*, the
history claim above becomes false: `K` is then baked into stored rows, the
projection cannot say which rows used which `K`, and re-weighting means a full
replay that the schema gives you no way to verify completed correctly. Rule 1
promises retuning is cheap **on the assumption that the projection stored
inputs rather than conclusions**, and it never states that assumption. It should.

**One genuine cost the table does not show, and it is not technical.** Retiring
the interim score changes what users see, after they have learned to read it.
A Stoa's top posts reshuffle; contributors who accumulated votes lose standing
that had no credential behind it. That is a product cost, it is unavoidable, and
the way to keep it small is to make the interim score's status legible from the
start — which is what rule 2's rewrite does, and the reason the label matters
more than the arithmetic.

## 8. What rule 2 becomes

The current rule 2 ships no score, and the claim it makes for itself is "a
smaller claim than 'we have a relevance model' and it is the true one, in the
same spirit as §7's refusal to claim sybil resistance."

The replacement must keep that spirit while shipping a score. The way to do it
is to **stop calling it relevance**, because it is not one:

> **2. v1 ships an engagement ordering, and calls it that.**
>
> `new` and `active` ship as before. Alongside them, a **`top`** ordering counts
> vote ops: a moderator's upvote weighs `K` (see the scorer; `K` is chosen, not
> derived), every other vote weighs 1, and a post's score is floored at zero so
> that downvoting can order a post last but never remove it.
>
> **This is an engagement ordering, not a relevance signal, and the distinction
> is the whole of the claim.** It reports how many distinct identities voted,
> weighted by the one credential that cannot be minted. It does **not** report
> how good or how relevant a post is, because the identities behind it are free
> to create and nothing establishes that two votes came from two people.
>
> **It is safe for exactly one reason, and it is not a property of the design:
> nobody is attacking a forum with no transport, no discovery and no users.** A
> defence resting on the environment expires when the environment changes; rule
> 6 names the conditions that expire it. Until one fires, the cost of an
> orderable forum is worth an ordering that an adversary could turn, because
> there is no adversary.
>
> **What must never be claimed for it:** that it resists sybils (it does not —
> with `K = 3`, four minted identities outvote a moderator, and no finite `K`
> changes that); that two peers agree on it (they do not, and rule 1 says that
> is correct); or that it measures quality. §7 refuses to claim sybil resistance
> it does not have, and this ordering is inside that refusal, not an exception
> to it.

The key move: `new`/`active` were defensible because they *cannot* be gamed by
minting. `top` **can** be, and the honest version says so in the same sentence
that ships it, rather than in a footnote.

## 9. Rule 6: the trigger, and why it is the most important part

"The forum won't get spammed just yet" is true and has an expiry date nobody has
named. **A staged decision without a named trigger is a permanent decision that
nobody admitted making** — the interim becomes the answer by default, because
no moment ever arrives that obliges anyone to revisit it.

So the trigger is stated as an observable condition, in PLAN.md where it is
read, not only in an archived design doc:

> **6. The interim engagement ordering (rule 2) expires on any of four
> conditions. Whichever fires first, `top` is withdrawn or re-gated before the
> next release.**
>
> - **A Stoa becomes discoverable without a human passing an address.** §4.8
>   Phase 2's broadcast topic is the sharp line: it is "unauthenticated and
>   spammable" by its own description, and it is the moment an attacker can find
>   a Stoa to attack. **This one is a precondition, not a warning** — `top` must
>   not ship enabled in the same release as broadcast discovery.
> - **A Stoa exceeds a few hundred participating identities.** Below that, a
>   moderator reading the Stoa notices a brigade; above it, nobody is reading
>   everything and the signal is load-bearing rather than decorative.
> - **The first sybil attempt is observed** — a burst of votes from identities
>   with no posting history, in either direction. One is enough; the question was
>   never whether an attacker *could*, only whether one had bothered.
> - **A nullifier-bound vote credential lands** (RLN, §7). Then rule 3's end
>   state is available and the interim has no remaining justification.
>
> The first three retire `top`. The fourth replaces it.
>
> **Whoever proposes broadcast discovery (§4.8 Phase 2) owns this check.** It is
> written here rather than only in a change document because the first condition
> fires inside someone else's change, and they will not read this one.

The third condition is the one with teeth and the one at risk of being ignored,
so it needs a cheap way to be observed: the projection's separate
`plain_up`/`plain_down` counts (§1(c)) plus identity-first-seen make "votes from
identities with no posts" a query rather than an investigation. That is a
second, smaller reason to keep the counts separable.

## 10. Is the owner right? Yes, with two corrections

**Yes, and the staging reasoning is sound.** Rule 2's "smaller claim and it is
the true one" is honest, and honesty about a product nobody can use is not a
virtue. §1's argument that a forum where nothing can be removed is a firehose
applies equally to attention: an unranked forum is a firehose with a timestamp.
Rule 1 makes the experiment cheap, rule 3's end state is genuinely blocked on
RLN rather than on anyone's effort, and waiting for a credential that does not
exist means shipping nothing for an unbounded interval.

**Rule 3 was right about the end state and was never asked about the interim.**
It is not being overturned. Its argument — that a credential decorating an
unmetered signal is worse than useless — remains exactly correct, and §4's
arithmetic confirms it rather than dodging it: four minted identities beat a
moderator at `K = 3`, and no `K` fixes that. What rule 3 could not have
considered is that the moderator credential is **unmintable today**, which makes
it a better interim weight than anything rule 3 had available — and still not a
defence.

**Two corrections to the proposal as stated:**

1. **A moderator's *downvote* must not carry the weight** (§6). The proposal
   said "a moderator's upvote being stronger", and read literally as "a
   moderator's vote" it would collide with rule 4 by making suppression a
   non-binding haircut with moderator authority behind it. Amplify promotion
   only.
2. **The score must be floored at zero** (§5). Without it, free identities get a
   censorship primitive with no audit trail, which is the one outcome this
   project's premise cannot absorb.

**And one condition on the whole thing:** it is acceptable *only* with rule 6's
trigger attached, and specifically only if `top` does not ship enabled alongside
§4.8 Phase 2 broadcast discovery. Without the trigger this is not a staged
decision, it is a permanent one taken quietly — and the reason to insist is that
the interim's entire safety argument is "no attacker is present", which nobody
will re-examine unless something makes them.

## 11. Vouching: recorded here, specified elsewhere

The owner extended the scope mid-change: alongside the moderator class, a reader
should be able to **decide for themselves** whose judgement to weigh, for someone
producing good content who holds no system credential. PLAN §7.3 carries the
decision; this section records only what a reader of *this change* needs, and the
mechanism is deliberately **not specified or tasked here** — see §12.

**The vocabulary is the part that was actually asked for**, and the rejected
options each encode a different mechanism, which is why the choice is not
cosmetic. "Follow" already means *show me their posts*; reusing it welds feed
subscription to vote weighting, and those are separable wants (plenty of people
are worth reading and unreliable at judging others, and the reverse is commoner).
"Friend" implies reciprocity and a social graph, where this is one-directional
and the other party is never told. "Trust" collides with §7's cryptographic
sense — a "trusted user" reads as a system property rather than one reader's
opinion. **Vouch** carries none of those. Weight classes become
**moderator / vouched / plain**.

**The one finding that changes an interface**: a vouch is per-reader, so a
vouched voter's weight is not a property of the vote *or of the Stoa*. The
schema's decision to partition aggregation by voter and join weights at query
time already accommodates this with no change — but it upgrades that decision
from convenient to required, since no per-Stoa stored weight could ever express
it. The class count in that join goes from two to three.

**Why it is not an op**, which is the substantive design decision: §5.2 makes
identities unlinkable across Stoas, so a published or travelling vouch list
would re-link the pseudonyms §5.2 protects — using the reader's own social graph,
which is worse than the linkage being prevented. A vouch therefore names a
Stoa-scoped identity and stays in that Stoa. Independently, a published vouch
graph is a sybil amplifier (identities vouching for each other manufacture
standing, which is rule 3's unmetered signal with extra steps), and keeping it
local means an attacker can only affect their own ranking, which is not an
attack.

**It amplifies promotion only**, for rule 4's reason plus one of its own: private
suppression is not better than public suppression, it is a filter bubble with a
ranking engine behind it.

**It does not expire under rule 6.** When the credential gate lands and plain
votes drop to zero, vouched votes survive — a vouch *is* a credential, issued by
the reader rather than the system. That is the argument for building it rather
than treating it as a stopgap: it is the only weight class that stays meaningful
in the end state, and it is what stops that end state from counting nobody but
token holders.

## 12. Why vouching is not specified in this change

This change carries one argument — that an interim engagement ordering is worth
shipping and must carry a named expiry. Vouching is a second argument, and
putting both in one delta would make each harder to review and impossible to
revert independently, which is the repository's own rule about reshaping and
altering behaviour in one diff.

It also raises a real design question this change has no answer for: **a vouched
set is per-reader local state, and no capability currently owns any.** Every
capability so far projects from the op log, which is shared, replayable and
attacker-supplied. A vouched set is none of those — it is authored by the user,
never published, and must survive replay rather than be derived by it. Where it
lives, how it is persisted, whether it is exported when a user moves devices, and
what happens to a vouch naming an identity the reader no longer holds ops for are
all questions with more than one defensible answer.

So PLAN §7.3 records the decision and the vocabulary; the mechanism gets its own
proposal.

## 13. Two axes, and why only one of them ranks

The owner raised a second extension: the upvote/downvote model conflates *quality*
(spam versus good content) with *agreement* ("I don't agree with you but it's
still engaging, genuine, constructive content"), and asked for an original design
— noting that web2's "report spam" is the wrong frame, because the reader is
**evaluating content, not filing a report**. PLAN §7.4 carries it.

**The objection is correct and it attacks §1 rather than merely the UX.** One
control collecting two unrelated judgements means a downvote for disagreement
and a downvote for spam are the same byte. A forum named for dialectic that
ships the mechanism by which web2 forums punish disagreement has lost its own
argument before anyone posts.

**The axes are genuinely independent**, which is the test worth applying before
adding a second control to anything: all four corners are populated
(agree/well-made, agree/badly-made, disagree/well-made, disagree/badly-made),
and *disagree + well-made* is the cell a single axis cannot express at all. It
is also the single most valuable thing a dialectic forum can surface — the best
argument against your position — so its being unrepresentable is not a corner
case, it is the product.

**The design decision is the asymmetry, not the second axis.** Two vote buttons
that get added together would be worthless: if agreement fed ranking, the forum
would rank consensus, and the dialectic would die with an extra click. So:

- **Assessment alone ranks.** A disagree costs a post nothing.
- **Response never sums.** Its jobs are to calibrate the reader's own weights
  (§11's vouching) and to be displayed **as a distribution, never as a net**.
  "Constructive; 40% agree" is informative; "+3" hides which of the four corners
  produced it, and a net figure is precisely what makes disagreement feel like
  damage. The display is where the pressure actually lands, so the prohibition
  belongs in the design and not only in the UI's taste.

**Evaluation, not report — and the distinction has teeth.** A report is a
petition to an authority, and §6.1 establishes there is no authority whose reach
extends past rendering, so a report has nobody to petition. An assessment takes
effect immediately in the assessor's own ranking and trust graph, without
anyone's permission. That means the control **always does something visible to
the person who used it**, which is what keeps it honest: a report button that
changes nothing observable trains people to use it as a super-downvote, which
would reintroduce the disagree button by the back door.

**Encoding is free.** The vote op is `target + one direction byte` with 254
unused discriminants, so both axes fit with **no wire-format version bump**. And
because `VoteDirection::from_byte` *refuses* unknown discriminants rather than
defaulting them, an older peer rejects an assessment it cannot interpret instead
of miscounting it — fail-closed, which is the direction that makes this cheap
rather than a migration.

**What I am least sure of, stated plainly:** two controls are harder than one,
and most readers will use neither if asked to think. The mitigation is
prominence rather than persuasion — assessment primary because it ranks,
response secondary and optional, with both degradations acceptable. Unlike the
sybil arithmetic, this cannot be settled by reasoning; it wants observation, and
the first finding that contradicts §7.4 should be treated as §7.4's answer.

## 14. What this document got wrong, corrected in review

Recorded rather than silently fixed, because the reasoning that produced the
error is the useful part.

**The decay epoch.** An earlier draft proposed a write-once `first_seen_unix`
receive-clock column (§1(a)). The schema agent's Lamport-plus-`-1` encoding is
correct and this was not. The error was reasoning only about *forgeability* — a
receive clock is at least not author-controlled — and forgetting *convergence*:
a per-peer `CLOCK_REALTIME` reading makes two peers rank identical op sets
differently, which rule 1 does not bless. Rule 1 blesses divergence that follows
from holding different ops; a clock skew is a wrong answer wearing that
divergence's clothes. `arrival.rs` already refuses the value for this reason.

**The filter-indexability question was raised by the schema agent, not found
here** (§1(d)), and the first instinct — that it is rule 5 in another hat — was
wrong in a way worth keeping: it treated "resolved on read" as the property that
makes something unindexable, when the property that actually matters is whether
the set of invalidation points is **enumerable**. A decayed score has none; a
hidden set changes only when a moderation op arrives. Two things can both be
computed on read and have completely different storage answers.
