# Design

## Context

PLAN.md **§7.3 "Vouching: a reader's own trust, and why it is not published"**
and **§7.4 "Two axes: assessment and response"**, read from
`origin/docs/relevance-votes` — **unmerged at the time of writing**, so every
citation below is by requirement name rather than by line, and nothing here
depends on its wording surviving review.

What §7.3 settles, and what this design takes as given without reopening:

- a vouch is never published;
- a vouch names a **Stoa-scoped** identity and stays in that Stoa;
- vouching confers no status on the person vouched for — no counts, no badges;
- vouching amplifies `constructive` only, never `noise`;
- vouching is **not transitive**;
- under **"Vouching accrues from what a reader already does"**, weight also
  accrues **earned** from the reader's own `constructive` assessments, is kept
  distinguishable from a **declared** vouch, is **capped below `K_vouch`**, is
  undoable by the reader, and decays with disuse (deferred in mechanism with
  §7.2 rule 5's absent age);
- under §7.4 **"Only assessment ranks. Response never sums."**, it is the
  **assessment** axis that accrues, never response — so weight can be earned by
  someone the reader consistently disagrees with and consistently finds worth
  reading.

The question left open, and the one this document is for, is **what owns a
reader's weight state.** It is not a storage detail. Every capability built so
far — `op-log`, `op-ordering`, `moderation-resolution`, `post-revision`,
`stoa-genesis` — projects from the shared op log, and §3.3's rule governs all of
them: "Ops are the authority; the view is a cache that can be rebuilt by replay."

**The brief that commissioned this work framed the whole thing as one category
error**: a vouched set is authored by the user, never published, and must survive
replay rather than be produced by it, and no capability currently owns any state
of that kind. That framing was right about the *declared* half and wrong about
the whole, and working out exactly where it stops being right is what produced
the shape below. §7.3's accrual half is a projection after all — with one
exception that is not, and that exception is the sharpest thing in this document.

## Goals / Non-Goals

**Goals**

- Decide what owns declared vouches and what owns earned weight, and argue each
  rather than assert it.
- State what replay does to each, and what happens when that invariant breaks.
- State what a vouch resolves to when the reader holds no ops for the identity
  named — which §3.3 makes the ordinary case, not an edge one.
- Decide the fail direction when the state is unreadable, distinguishing "safe"
  from "correct".
- Decide whether weight state exports between devices, and whether that
  reintroduces the §5.2 linkage risk.
- Fix the minimum wire surface, weighed against §7.3's no-vouch-counts rule.

**Non-Goals**

- **No scorer.** How `K_vouch`, `K_mod` and the earned cap are applied is §7.2
  rules 2 and 5's problem. This change supplies the weight class and nothing
  else, and chooses none of the constants.
- **No accrual function.** *How much* one `constructive` assessment earns, and
  the shape of the curve under the cap, is the accrual change's decision. This
  design decides only where the answer is computed and what it may read.
- **No decay.** §7.3 defers it in mechanism with §7.2 rule 5's absent age.
- **No transitivity, no vouch op, no SQLite DDL, no UI.**

## Decisions

### The two halves have different provenance, so they get different homes — and
### §7.3's accrual half is a projection, which is what resolves the §3.3 tension

The brief's framing was that a vouched set is a genuinely new category: authored
by the user, never published, must survive replay. That is exactly right for a
**declared** vouch and exactly wrong for **earned** weight, and the reason is
one observation:

> **A reader's own assessments are ops.** §7.4 encodes an assessment as a
> discriminant on the existing vote op — "both axes fit in the existing wire
> format with no version bump". So every input to earned weight is a signed,
> published op, authored by the reader's own Stoa identity, sitting in the
> reader's own op log alongside everyone else's.

Earned weight is therefore **a fold over the reader's own assessment ops in one
Stoa** — the same shape as every other resolver in this crate, restricted to
`author == my key for this Stoa`. It is derived, it is rebuildable by replay, and
it belongs in the derived region with the rest of the projection. Nothing about
it is a new category.

That is worth stating as a principle, because it is the part that generalises:

> **State authored by the user is not automatically outside replay. State
> authored by the user *as an op* is inside it, because the op log already holds
> it. What is outside replay is state the user authored that was never
> published** — which, by §7.3, is exactly the state whose whole purpose is not
> being published.

So the split is not "user-authored versus derived". It is **published versus
not**, and §7.3's never-published rule is what draws the line. Two homes:

| | provenance | region | survives a rebuild because |
|---|---|---|---|
| **declared vouches** | the reader, unpublished | authoritative | it is not in the derived set |
| **earned weight** | the reader's own assessment ops | derived | it is recomputed from them |
| **earned suppressions** | the reader, unpublished | authoritative | see below — this is the one that surprises |

**The third row is the finding.** §7.3 requires that earned weight is "something
they should be told has happened and be able to undo". An undo is not derivable
from the op log, and it cannot be: the assessments that earned the weight are
still there, still signed, still valid. Replay would faithfully re-earn the
weight the reader just dismissed, every time, with no error. **A reader who
dismisses an earned weight and then triggers any rebuild gets it straight back**
— and since a rebuild is the routine recovery for a schema change or a scorer
retune, they get it back at a moment nobody is watching.

So the undo is a **suppression record**: unpublished, authored, small, and
authoritative. It says "do not accrue for this identity in this Stoa", and it
lives with the declared vouches because it is the same kind of thing — a reader's
unpublished decision that no op can express.

This is a genuine cost of §7.3's accrual design that its text does not mention,
and it is recorded under **What §7.3 costs more than it reads** rather than
treated as a detail, because the failure mode is silent restoration of a thing
the user explicitly removed.

**One consequence that falls out and is worth taking.** With earned weight in the
derived region, §7.2 rule 5's schema requirement is satisfied for free. Rule 5
requires "vote counts partitioned by voter rather than pre-weighted or
pre-summed", and says of §7.3 specifically that "a vouched voter's weight is not
even a property of the *Stoa*, since it differs per reader, so weights must be
joined at query time". Earned weight recomputed from the reader's own assessments
is a join-time quantity by construction; there is no stored weighted total to go
stale when the accrual curve is retuned. Retuning accrual is then a rebuild of
the derived region — the cheap operation — exactly as rule 1 promises for
ranking generally.

### The authoritative half lives in the local store's authoritative region — not
### in the keystore, and not in the projection

This is the crux the brief named, and it applies to the declared vouches and the
suppression records. The §3.3 tension is real and resolves like this.

**First, a distinction §3.3 makes that is easy to read past.** §3.3 says:

> Each peer is to keep a **local SQLite store** holding every op it has seen,
> plus a materialised view of the forum derived from it. Ops are the authority;
> the view is a cache that can be rebuilt by replay.

The "cache that can be rebuilt by replay" is the **materialised view**, not the
store. The store also holds the op log, and the op log is emphatically *not*
rebuildable by replay — it **is** replay's input. So the local store already
holds two regions with opposite durability properties, and §3.3 already
distinguishes them. What it does not do is *name* the distinction, because until
now the two regions were "the ops" and "everything derived from the ops", which
is the same line as "authoritative" versus "cache".

Declared vouches break that coincidence: authoritative, and not an op. So the
honest statement of the rule is not "ops are authoritative, everything else is
cache" but:

> **The store holds an authoritative region and a derived region. The derived
> region is a function of the authoritative one and may be dropped at any time.
> The authoritative region is everything nothing else can reproduce: the op log,
> which arrives from peers, and unpublished local decisions, which arrive from
> the user and were deliberately never published.**

That is a generalisation of §3.3 rather than an exception to it, and it is the
part worth keeping from this change even if vouching is never built.

**Why not the materialised view.** Same file, wrong region, and the difference is
operational rather than philosophical. The view's defining property is that it
can be dropped and rebuilt — which is what makes retuning the scorer a non-event
(§7.2 rule 1: ranking "can be retuned without a protocol version bump"). `DROP`
and replay is the recovery procedure for a schema change, a corrupt index, or a
scoring change. **The moment declared vouches sit in a table that procedure
touches, retuning the ranking destroys them** — and it destroys them exactly when
someone is confident the operation is safe, because the whole point of the derived
region is that dropping it is safe. A data-loss bug whose trigger is a routine
maintenance action is the worst kind there is.

Note this cuts the other way for earned weight, and correctly: earned weight
*should* be dropped and recomputed by that same procedure, because recomputing it
is how a retuned accrual curve takes effect.

**Why not the keystore.** The keystore is the closest analogue and the one to
study hardest — local, user-authored, never published, survives restarts, and it
already solved atomic writes, a fixed name in a caller-supplied directory, and
permission refusal. Reusing it is genuinely attractive. It is still wrong, for
four reasons in ascending order of weight:

1. **The file format has no room and gains nothing by making some.** The
   keystore is a fixed-length record — magic, version, protection, KDF
   parameters, salt, nonce, 32 bytes of ciphertext, tag — and `MAX_KEYSTORE_LEN`
   is a compile-time sum of exactly those. Weight state is unbounded, per-Stoa,
   and grows. Adding it means a second, variable-length, attacker-influenced
   section whose parser is then on the same path as the root secret's.
2. **It is a rewrite-the-whole-file store, and vouching is a frequent small
   write.** `write_to` is stage-to-temp, `sync_all`, rename — right for something
   written once at setup. Every vouch, unvouch and dismissal would rewrite the
   root secret's ciphertext, opening a crash window over the root secret on an
   operation that has nothing to do with it.
3. **It would couple two independent failure modes.** The keystore refuses to
   open on a too-open mode, a too-open directory, a wrong passphrase, a corrupt
   header. Every one of those would then also mean "your vouches are
   unavailable"; and in the other direction a corrupt vouch record would mean
   "you cannot post". Two capabilities failing together for reasons neither
   shares is the shape the keystore's own design.md already refused once, when it
   separated `PermissionsTooOpen` from `DirectoryWritableByOthers` because "the
   fix is a chmod on a different path".
4. **The security postures are genuinely different, and conflating them makes
   both worse.** The keystore's threat model is a stolen disk and a hostile local
   process substituting the identity the user posts under; the load-bearing half
   is *authentication*, because a substituted root secret means posting under a
   key an attacker holds. A substituted vouch set is a smaller harm: the attacker
   reorders the user's feed. Meanwhile the vouch state has a property the root
   secret does not — it is a **social graph**, and §7.3's argument for never
   publishing it is precisely that a reader's social graph re-links pseudonyms.
   Neither posture contains the other. One file protected to the stronger of the
   two is not a simplification; it is one file whose reader must keep two threat
   models straight.

The fourth point has a corollary most likely to be got wrong later: **the vouch
state is not a secret in the keystore's sense, but it is not ordinary
application data either.** It is exactly as sensitive as a follow list, and the
harm from disclosure is the §5.2 linkage §7.3 exists to prevent, arriving from
the disk rather than from the wire.

**What a third region costs, honestly.** It is not free: a second thing to back
up, to migrate, and whose absence must mean something. The alternative on offer —
a third *file* — is worse, because two files that must stay consistent and are
not written together is a new class of bug, and there is nothing to be consistent
*about* here: vouches reference identities, not rows.

**What would reverse this.** If the local store never becomes a single SQLite
file — if the op log stays in memory and unpublished local decisions are the only
thing that ever needs to persist — then a small separate file authored the way
the keystore's is becomes the simpler answer and this decision is overbuilt.
That is a real possibility, named here so the next reader can check it against
what was actually built rather than infer the question was never asked.

### Replay must preserve the authoritative region, and the invariant is enforced
### by placement rather than by remembering

§3.3 says the view is rebuilt from the op log. Declared vouches and suppressions
have no ops, so there are exactly two designs:

- **(a)** they live where replay does not reach — replay is defined over the
  derived region, and the authoritative region is outside it;
- **(b)** replay preserves them explicitly — reads them out, rebuilds, writes
  them back, or takes care to skip them.

**(a), and the difference is the whole point.** Under (b) the invariant is a
thing replay has to *remember*, and CLAUDE.md's standing instruction is exactly
about this: "Prefer reshaping state so an invariant holds by construction over
adding a branch that checks it. A branch must be got right at every call site."
Replay has more than one call site — a schema migration, an accrual retune, a
scorer retune, a corruption recovery, a test fixture reset, and eventually a
snapshot import (§4.7). Under (b) every one must know about a table that has
nothing to do with what it is doing, and the first that does not know is a silent
loss.

Under (a) enforcement is structural: **rebuild is defined as an operation on the
derived region**, and the authoritative region is not in it. A rebuild that
dropped it would have to reach outside its own definition.

Concretely, that is the difference between a rebuild spelled "drop and recreate
the tables in the derived set" and one spelled "drop everything except the tables
I remembered". The first is a list that must be *added to* when a derived table
appears — and a derived table left off it is a stale cache, which is visible and
recoverable. The second is a list that must be added to when an authoritative
table appears — and one left off is deleted user data, which is neither.

**What happens when the invariant is violated.** Worth being exact, because the
answer is "nothing observable, and that is the problem":

- Every declared vouch silently drops to **plain**; every dismissed earned weight
  silently comes **back**. Neither produces an error, an exception or a log line
   — a plain identity is a legitimate state, and an accruing one is the default.
- The user's `top` ordering changes. But §7.2 rule 1 already says two peers rank
  differently and that is correct, and the same reader's ranking legitimately
  moves whenever ops arrive, so **a changed ordering is evidence of nothing**.
  The user cannot distinguish "my vouches were deleted" from "the feed moved".
- Nothing can restore the declared half. §7.3 forbids publishing, so no copy
  exists anywhere.

That asymmetry — losses that are silent, and restorations that are also silent —
is why placement has to be structural rather than procedural. **A loss with no
symptom cannot be caught by a person noticing; it has to be made unreachable.** A
test asserting "a rebuild preserves declared vouches" is worth having and is in
the spec delta, but it checks one code path, and the paths that matter are the
ones not yet written.

**One honest limit, since this design is about invariants and should hold itself
to the standard.** Placement makes the loss unreachable *by a rebuild*. It does
not make it unreachable by a user deleting the database, by a Basecamp profile
being recreated (the keystore's own design.md flags that
`instance_persistence_path` is per-instance, so "two profiles get two
identities"), or by a machine dying. Those are ordinary data loss with ordinary
answers, and the export verb argued for below is the only one this design
addresses.

### A vouch for an identity the reader holds no ops for is recorded, and
### resolves to "vouched"

§3.3 makes partial op sets the normal case, so this is the ordinary path. It
arises two ways, both routine: the reader vouches for someone and later loses or
never receives their posts, and the reader is handed an identity out of band and
vouches before anything from them arrives.

**The declared set stores the identity and nothing else.** No cached display
name, no first-seen marker, no post count. A vouch names a public key in a Stoa;
that is a complete fact about the reader's intention and needs nothing from the
log to be true. The moment it caches something derived, the cache is a second
copy of a fact the op log owns, and §3.3's discipline is one authority per fact.

**So resolution is a set-membership test and cannot fail.**
`weight_class(stoa, identity)` asks: is this key the Stoa's moderator (from the
genesis record, resolved on read exactly as `moderation.rs` already does)?
Otherwise, is it in this Stoa's declared set? Otherwise, does it carry earned
weight? Otherwise, plain. Only the third arm consults the log, and it consults
only the reader's *own* ops; a declared vouch for a key the log has never seen
answers `vouched` like any other.

Two consequences worth stating, because the alternatives are tempting:

- **A vouched identity with no posts contributes nothing to any ranking, and
  that is not a special case.** There are no assessments from them to weigh. The
  weight class is a property of the assessor; an assessor who has not assessed is
  absent from the sum by ordinary arithmetic rather than by a guard.
- **Refusing a vouch for an unknown identity would be wrong**, and it is the
  reflex to avoid. It would make vouching depend on propagation timing — the same
  vouch succeeding or failing depending on whether a post had arrived — which is
  exactly the per-peer accident §3.3 and `arrival.rs` spend their length keeping
  out of everything. It also forecloses the out-of-band case, which is arguably
  the main one: someone tells you offline whose judgement to trust.

**Earned weight has the mirror-image property and it needs no rule.** An identity
can only earn weight from assessments the reader made, and the reader can only
assess what they hold, so earned weight for an unseen identity is not reachable.
The asymmetry is correct: a declaration is about intent and needs no evidence,
an accrual is evidence and is exactly as available as the evidence.

**Validation is on the key's shape, not its history.** A vouch naming something
that is not a well-formed public key is refused — a malformed request, which is
the wire contract's business. A vouch naming a well-formed key nobody has seen is
accepted, because it is a valid intention.

### Fail-open when the state is unreadable — and "safe" is not why

The keystore refuses to open rather than defaulting, and §5.6's reasoning is that
"defaulting is how a token-gated Stoa silently becomes world-postable". The
question is whether that argument transfers.

**It does not, and the reason is what each default asserts.**

A keystore that defaulted would assert a **capability the user does not have**:
"you may post", when the key to post with is unavailable. The claim is about
authority and it is false. §7.1's token-gated Stoa is the sharp version — the
default silently grants what the gate exists to withhold.

Unreadable weight state asserts that **every identity is plain**, which is not a
claim about authority at all. It is the state of every new reader, of every
reader in a Stoa they have not curated, and it is the state the system has today
with no vouching. Nothing is granted; a weight the reader chose to add is not
added.

So: **fail open.** Unreadable or absent weight state yields the empty set, and
ranking proceeds with everyone plain.

**Now the part that matters, because "safe" and "correct" are different claims.**

*Safe* is the easy half and is argued above: the failure grants nothing, and the
resulting behaviour is behaviour the system ships as normal. No privilege
escalation, no false claim.

*Correct* is not established by that, and on its own the safety argument would be
a bad one — it is the same argument that would justify swallowing any error whose
consequence is mild. What makes fail-open correct here is a different property:
**the alternative refuses a service that has nothing to do with the failure.**
Failing closed on unreadable weight state means the reader cannot read the Stoa
at all, because ranking cannot complete and the feed will not render. That is a
total outage of the forum caused by a corrupt preference whose entire function is
to reorder a list the reader could have read unordered. Refusing to show a forum
because a weighting is unavailable is a worse answer than showing it unweighted,
and no amount of care about the weighting changes that.

**The two failures are asymmetric in a way the keystore's are not, and that is
the whole transfer argument**: a keystore failure makes posting *impossible*, so
refusing to offer posting reports the truth. A weight-state failure makes ranking
*less good*, so refusing to rank reports something false.

**What fail-open must not do, and this is the load-bearing half.** Failing open
*silently* is what would make it wrong. Three obligations come with it, and they
are requirements in the delta rather than advice:

1. **The failure is a distinct state, not folded into "you have no vouches".** "I
   could not read your vouch state" and "your vouch state is empty" must be
   distinguishable at the API, because a reader who sees an empty list after
   vouching for twelve people needs to know which happened. Same distinction the
   keystore draws between "no passphrase was ever set" and "your passphrase is
   wrong", for the same reason.
2. **A failed read never becomes a write.** The one way fail-open turns into
   permanent loss: read fails, set treated as empty, next vouch writes the
   now-one-element set over the file that could not be read. **Unreadable weight
   state is read-only until a human acts**, and a vouch attempted against it is
   refused with an error naming the fix. This is the sharpest requirement in this
   change and the one most likely to be lost in implementation, because "treat it
   as empty" reads as though it applies to both directions and must not.
3. **The error names the fix**, on the keystore's existing obligation — its
   `Display` must name a fix on every variant, enforced by a test that checks the
   guidance clause at a word boundary. Same bar here; the fixes differ (restore
   from an export, or start again knowingly) but the obligation is the same.

**The earned half fails differently and more gently, and the difference is worth
recording.** Earned weight is derived, so an unreadable derived region is a
rebuild, not a loss — the assessments are still in the log. The one thing that
does *not* survive is the suppression records, which is the third row of the
provenance table again: they are authoritative and fail like declared vouches.
So obligations 1–3 attach to the authoritative half, and the earned half needs
none of them.

### Export between devices is a deliberate manual act, and it does not
### reintroduce the §5.2 risk — provided it is one Stoa at a time

One person, two machines, one Stoa identity. If vouches do not travel, that
reader ranks differently on laptop and phone. §7.2 rule 1 blesses two peers
ranking differently — but it blesses it *for different op sets*, and this is not
that: the op sets could be identical and the ranking still differ, for a reason
unrelated to what either peer has seen. Rule 1 does not cover it and should not
be stretched to.

**The earned half already travels, and that is a real and under-appreciated
consequence of §7.3's accrual design.** Earned weight is a fold over the reader's
own assessment ops, which are published. A second device that joins the Stoa
under the same identity and receives the same ops recomputes the same earned
weight with nothing carried across. So §7.3's accrual half is *self-syncing*, for
free, and only the authoritative half has a portability problem at all. That
shrinks the question and is worth saying out loud, because the instinct is that
all of it is stranded.

**For the authoritative half, the instinct is that §7.3's never-published rule
forbids export. It does not**, and the distinction matters in both directions.
§7.3's argument is about **publishing**: a vouch that travels over the Stoa's
channel, or is visible to anyone but its author. The harm is that a reader's
social graph becomes observable and re-links pseudonyms §5.2 keeps apart. **A
file the user carries to their own second device is observed by nobody** and puts
no vouch on any wire the protocol defines.

So: **export and import are supported, as an explicit user action producing an
opaque local artefact — never automatic, never over the network, never through
any Logos transport.**

The linkage risk is real, and it is a property of the artefact's *scope* rather
than of its existence:

- **A single-Stoa export cannot link pseudonyms**, because every key in it is a
  key in one Stoa and it says nothing about any other. Whoever obtains it learns
  the reader's opinions about one Stoa's participants — a privacy loss, bounded
  by that Stoa.
- **A multi-Stoa export is exactly the artefact §7.3 forbids, arriving by another
  route.** A file naming the reader's vouches across every Stoa they read is a
  cross-Stoa list; anyone obtaining it holds the linkage §5.2 is built to
  prevent, and — §7.3's sharper point — reconstructed through the reader's own
  social graph, which is worse than the linkage §5.2 prevents. One file, and an
  attacker has everything at once.

**So export is per-Stoa and the artefact names exactly one Stoa.** "Export all my
vouches" is not offered. The convenience of one file over five is not worth
manufacturing the exact object the design exists to not have — manufactured *by
us*, at rest, for the user to then mishandle.

Three properties the artefact must have:

- It **must** name its Stoa, so an import into the wrong Stoa is refusable rather
  than silently applying keys from elsewhere. A vouch is Stoa-scoped by §7.3; an
  import ignoring scope would be the first cross-Stoa vouch in the system.
- It **must** be refused on import when it names a different Stoa than the one
  being imported into. Refused, not merged — a wrong-Stoa import is either a
  mistake or an attack, and neither has a sensible partial outcome.
- It **must not** carry the identity's secret key or anything derived from the
  root secret. An export is a preference file, not a second keystore. Conflating
  them turns "share my vouches with my phone" into "copy my identity by email",
  and the keystore already has a considered answer about how a root secret moves,
  which is: it does not (§5.3, no rotation; `create` refuses to overwrite).

**It should carry the suppression records too**, and forgetting them is the
subtle bug: a reader who dismissed an earned weight on one device and imported
their vouches to another would find it re-earned there, because the second device
holds the same assessment ops and nothing told it the reader had said no. That is
the same silent restoration the suppression record exists to prevent, arriving
across devices instead of across a rebuild.

**The residual risk, stated rather than mitigated**: an export is a file on a
disk, and it is a social graph for one Stoa. More sensitive than its size
suggests, less sensitive than a keystore. Whether it should itself be encrypted
is an open question below, not a decision this change makes.

### The wire surface, and why the reader's own list is safe to expose

CLAUDE.md: the core API is the deliverable, every method takes and returns JSON,
failure is always `{"error":"..."}`, paginated calls take `(page, perPage)` and
return `{"items":[...],"page":N,"hasMore":bool}`. The question is the minimum set
of methods, and the sharp one is whether listing is safe at all.

**The listing question first, because §7.3 makes it non-obvious.** §7.3 forbids
surfacing a vouch *count* or an "N people vouch for this author" badge, because
such a display is a published vouch graph reconstructed by eye. Listing looks
adjacent. It is not, and the line is sharp:

- What §7.3 forbids is showing a reader information about **other readers'**
  vouches. That is the published graph, whether it arrives as an op or as a
  number rendered beside a name.
- What listing exposes is **a reader's own vouches, to that reader**. The reader
  authored every entry. Nobody learns anything about anyone else, and no
  aggregate over readers exists to compute.

The failure mode to guard is not listing; it is an API shape that makes the
forbidden aggregate *computable*. So the constraint is stated directly: **no
method takes an identity and answers a question about vouches by anyone other
than the caller.** No "who vouches for this key", no count, no aggregate — and,
because the module serves exactly one local user, **no method takes a "which
reader" parameter at all**. The caller is always the local reader, implicitly,
and that implicitness is a feature: a parameter naming whose vouches to read is
the first step towards a method that could answer for someone else.

Listing is also not optional. Without it the state is write-only: a reader cannot
see whom they have vouched for, cannot audit it, and cannot unvouch someone whose
key they no longer have to hand — because unvouching needs the key, which is in a
list they cannot read. §7.3's accrual half makes this much stronger: earned weight
is something the reader "should be told has happened and be able to undo", and a
reader cannot be told about something with no method to ask.

**The minimum surface:**

| Verb | Takes | Answers |
|---|---|---|
| vouch | Stoa, identity | success, or the error shape |
| unvouch | Stoa, identity | success, or the error shape |
| list | Stoa, page, perPage | the reader's own weighted identities, paginated, **each carrying its provenance** |
| dismiss | Stoa, identity | success, or the error shape — writes a suppression record |

Paginated because §2.5 says paginated calls look like that, and the set has no
bound — a reader in a busy Stoa may hold hundreds, and earned weight makes that
far more likely than a hand-curated list would. An unpaginated list verb would be
the one unpaginated collection in the API, which is how a convention stops being
one.

**Provenance is in the list reply, and it is not a nicety.** §7.3 requires
declared and earned to stay distinguishable, on the ground that "one is something
the reader chose and can revoke in a click, the other is something they should be
told has happened and be able to undo". A view cannot honour that from a flat list
of keys — it would have no basis to label anything, and would end up presenting an
accrual as a declaration, which is precisely the misrepresentation §7.3 is
guarding against. So each item carries whether it is declared, earned, or both.
**Both is a real state**, not an edge case: a reader may declare a vouch for
someone who is also accruing, and the reply must not collapse the two, because
un-declaring leaves the accrual in place and the view has to be able to say so.

**What is deliberately not in the surface, and why each was considered:**

- **No `isVouched(stoa, identity)` probe.** The list answers it, and a
  per-identity probe is the shape that makes a bulk query natural later — feed
  the participant list through it and you have reconstructed a per-author vouch
  display, which is the rendering §7.3 forbids, assembled client-side from a
  method that never said no. The absence is a guard and costs a view nothing: a
  view rendering a Stoa already holds the list.
- **No `vouchCount`.** Not because a reader may not know their own total — they
  may, and pagination reports it structurally — but because a method with that
  *name* is the one an implementer later generalises to take an identity. The
  name is the risk.
- **No weight-class query, and no numeric weight in the reply.** "What class is
  this identity in, for me" is the scorer's internal question, answered inside
  core; exposing it puts a per-identity vouch probe back under another name. And
  a numeric earned weight would hand a view a number to render beside an author,
  which is a vouch display in all but name — the provenance flag says what §7.3
  requires the reader to be told, and no more.
- **No export/import verbs decided here.** Export is argued for above as a
  capability; its wire shape turns on whether the artefact is JSON returned
  inline or a file at a caller-supplied path, which turns on whether it is
  encrypted, which is open. Deciding half of it now would fix the wrong half.

**`vouch`, `unvouch` and `dismiss` must be idempotent, and must say so.**
Vouching for someone already vouched for is success; unvouching someone not
vouched for is success; dismissing an accrual already dismissed is success. A
view that must track whether it already sent one — across a restart, a reconnect,
or a double tap — is a view carrying state the module owns. Same reasoning
`OpLog::append` records for its idempotence, arriving from a different direction:
there because retransmission is ordinary traffic, here because a user pressing a
button twice is ordinary use.

### What §7.3 and §7.4 cost more than they read

The brief asked for this specifically. Four things qualify. None makes a settled
decision impossible; each is more expensive than its sentence suggests.

**1. "Be able to undo" requires a second kind of authoritative state, and §7.3
does not say so.** Argued in full in the first Decision. The one-line version:
the assessments that earned a weight remain valid ops, so replay re-earns what
the reader dismissed, and the dismissal must therefore be recorded as
unpublished authored state of its own. §7.3 reads as though accrual is a pure
projection; it is a projection **plus a suppression set**, and the suppression
set is the part with the silent failure mode. This is the single largest thing
this design pass surfaced.

**2. "Confers no status" is a UI obligation with no enforcement anywhere, and it
is the one most likely to be violated by a well-meaning change.** Every other
rule here is enforceable in core: never-published is enforced by there being no
op kind; `constructive`-only is enforced by the scorer; per-Stoa scoping is
enforced by every method taking a Stoa. "No vouch counts" is enforced by **nobody
adding a feature**, and core cannot tell whether a view has added one. The only
structural lever available is to refuse to make the aggregate cheap to compute —
which is why the absent per-identity probe above is a decision rather than
minimalism. That is a real mitigation and it is not enforcement. It is recorded
as an obligation in PLAN.md §11.1, the honest place for a requirement nothing can
check.

**3. `constructive`-only is not a property of the weight state and cannot be
tested here.** §7.3's "vouching amplifies `constructive` only" is a statement
about how the scorer consumes a weight class. This change supplies the class and
has no scorer, so **no scenario in the delta can pin it** — writing one would be
a scenario about behaviour that does not exist, which `.claude/agents/README.md`
names explicitly as the thing not to do. It belongs to the scorer's change and is
flagged in `tasks.md` so it is inherited rather than lost at the boundary, which
is where an obligation split across two changes usually goes.

**4. "Never published" and "exports between devices" are in genuine tension, and
§7.3 does not address export at all.** The decision above resolves it — publish
means over the protocol; a user-carried per-Stoa file is not that — but §7.3 as
written does not distinguish the two, and a strict reading forbids export
outright. That reading has a real cost, now smaller than it first appears: the
earned half self-syncs, so what is stranded is declared vouches and dismissals
only. **If the owner intends the strict reading, that is a decision to take
deliberately**, and the consequence is that declared vouching is per-device and a
user replacing a machine re-declares. This design takes the permissive reading
and bounds each artefact to one Stoa; the finding is that the rule as written does
not decide the question, and this document is where the answer now lives.

### What cannot be tested, said plainly

`.claude/agents/README.md`: "Never write a scenario that cannot be tested." Five
things here are not expressible as a checkable WHEN/THEN, and are stated here
rather than smuggled into the delta as requirements.

- **"No vouch counts are ever displayed."** A property of a view that does not
  exist, unobservable from core. What *is* testable is the surface's shape — that
  no method answers a question about anyone else's vouches — and that is in the
  delta. The display rule is a PLAN.md obligation.
- **"Vouching amplifies `constructive` only."** No scorer. See above.
- **"Earned weight is capped below `K_vouch`."** The cap is a scorer constant and
  the accrual function does not exist; a scenario asserting it would be asserting
  against a value this change does not define. What the delta *can* require is
  that the surface keeps the two provenances distinguishable, which is the
  precondition for the cap being expressible at all.
- **"Replay never touches the authoritative region", as a universal.** What is
  testable is that *the rebuild operation that exists* preserves declared vouches
  and dismissals — one path, pinned. The universal ranges over code not yet
  written, so the delta requires the property of the operation rather than of all
  future operations, and the structural argument above covers the rest. Saying
  otherwise would be a false coverage claim.
- **"An export is observed by nobody."** A claim about what the user does with a
  file. The testable parts — export is never automatic, never crosses a
  transport, names exactly one Stoa, and is refused on a Stoa mismatch — are in
  the delta. What the user does with the file afterwards is outside every
  boundary this project controls, and the residual is under Risks.

## Risks / Trade-offs

- **A third region in the local store is a concept the project did not have.** →
  One sentence of rule plus a placement discipline, against a vouched set in a
  table that a routine `DROP` destroys. The §3.3 generalisation stands on its own
  even if vouching never ships.
- **Splitting weight state across two regions is more machinery than one store.**
  → The split is not invented here; it follows the provenance line §7.3 already
  drew between declared and earned. Putting earned in the authoritative region
  would make retuning accrual a migration instead of a rebuild, which is the
  property §7.2 rule 1 is built to keep.
- **Suppression records are a third concept in a feature that reads like two.** →
  They are the price of §7.3's "be able to undo", and the alternative is silent
  restoration of something the user removed. Recorded as a finding rather than
  absorbed.
- **Fail-open means a silently degraded ranking.** → Mitigated by the distinct
  unreadable state, which makes the degradation reportable, and by the read-only
  rule that stops it becoming permanent. Not fully mitigated: a view that ignores
  the state still shows a quietly worse feed.
- **A per-Stoa export is still a social graph at rest.** → Bounded to one Stoa by
  construction, which is what keeps it out of §5.2's way. Encryption is open.
- **Listing exposes the reader's own graph to whatever holds the module.** →
  Anything holding the module holds the keystore's directory, so this adds no
  attacker who did not already have more. Worth stating rather than assuming.
- **No enforcement of the no-status rule.** → The API refuses to make the
  aggregate cheap; the rest is an obligation with a home. Honest about being an
  obligation, not a guarantee.
- **The whole change assumes a SQLite store that does not exist.** → The decision
  is about which region owns what, and that argument does not depend on the store
  being built. The reversal condition is named under the ownership decision.

## Open Questions

- **Should an exported artefact be encrypted?** For: it is a social graph on a
  disk, possibly a removable one. Against: the passphrase available on the import
  side is the keystore's, which would couple the two stores this design spent
  four arguments separating, and an export encrypted under a passphrase the user
  must type on a new device is a passphrase they will write down. Not decided; it
  determines the export verb's wire shape, which is why that verb is unspecified
  here.
- **Does weight state need its own durability tier in §4.7?** §4.7's three tiers
  are all about *ops*. Unpublished local decisions are in none of them, and the
  Logos Storage snapshot tier is about sharing history backwards to newcomers,
  which vouch state must never do. Probably the local tier with a note that it is
  not reproducible from the other two; raised because §4.7's table reads as
  exhaustive and now is not.
- **Does a dismissal expire?** §7.3 says earned weight decays with disuse, which
  is deferred with §7.2 rule 5's absent age. A suppression that never expires
  means an identity dismissed once can never accrue again, however the reader's
  assessments change after; one that expires means a dismissal quietly stops
  meaning what the reader meant. Both are defensible and the choice wants the
  same age input decay does, so it is deferred alongside it — flagged because a
  permanent dismissal is the conservative *implementation* and is not obviously
  the conservative *behaviour*.
- **What happens to a Stoa's weight state when the reader leaves?** Kept,
  presumably — rejoining is ordinary, and losing it on a transient departure is
  the silent loss this design is built to avoid. But "leaving" is not a concept
  the protocol has (§6.1: there is no membership to revoke), so there may be
  nothing to decide. Named so that whoever adds a leave affordance to a view
  checks it rather than discovering it.
- **Whether `K_vouch` and the earned cap should be user-adjustable.** §7.3 fixes
  `K_vouch < K_mod` and the earned cap below `K_vouch`, and suggests 2 against 3.
  A per-reader weight is unpublished local state of exactly the kind this design
  now has a home for, so the question becomes cheap to answer later — which is a
  reason not to answer it now.
