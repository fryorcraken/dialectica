# Weight state: what owns a reader's vouches, and what replay may do to them

## Why

PLAN.md §7.3, on branch `docs/relevance-votes`, settles what vouching *is*: a
reader marks identities whose votes then weigh more heavily in that reader's own
ranking, the classes being moderator / vouched / plain. Weight also accrues
**earned**, from the reader's own `constructive` assessments (§7.4's assessment
axis, never response), kept distinguishable from a **declared** vouch, capped
below `K_vouch`, and undoable by the reader. A vouch is never published, names a
Stoa-scoped identity, confers no status on the person vouched for, amplifies
`constructive` only, and is not transitive. None of that is reopened here.

What §7.3 does not settle, and what this change is for, is **where that state
lives**. It is a structural question rather than a storage detail, because §3.3
governs every capability shipped so far — "Ops are the authority; the view is a
cache that can be rebuilt by replay" — and a declared vouch has no ops. It is
authored by the user, never published, and must **survive** replay rather than be
produced by it.

The first framing of that question treated all of vouching as one new category.
It is not, and finding the seam is most of this change's value:

| | provenance | rebuildable by replay |
|---|---|---|
| op log | peers, attacker-supplied | it **is** replay's input |
| materialised view | derived from the log | yes — that is its definition |
| keystore | the user, unpublished | no |
| **declared vouches** | the user, unpublished | **no** |
| **earned weight** | the reader's own assessment **ops** | **yes** |
| **dismissals of earned weight** | the user, unpublished | **no** |

Earned weight folds over ops the reader published, so it is an ordinary
projection. A declared vouch is not. And a *dismissal* of earned weight is
neither obvious nor free: the assessments that earned it remain valid ops, so
replay re-earns what the reader dismissed unless the dismissal is itself recorded
as unpublished authored state. That third row is the thing this design pass
surfaced and §7.3's text does not mention.

This is planning only. No code is written by this change.

## What Changes

- A written argument for where each half lives, what replay does to each, what a
  vouch resolves to for an identity the reader holds no ops for, what happens
  when the state is unreadable, whether it travels between a user's own devices,
  and what the module exposes across the wire. Recorded in `design.md`, which is
  the deliverable.
- A `vouched-set` capability delta spelling the behaviour contract as testable
  scenarios. A delta only; promoting it is the implementing change's job.
- A home in PLAN.md for the **rendering obligations** this project keeps
  accruing and has nowhere to put. §6's "a hide cannot currently be reversed, so
  say so at the point of action" is one; §7.2 rule 4's default-omit of hidden
  posts is another; §7.3's no-vouch-counts and §7.4's never-show-a-net are two
  more. All are obligations on a surface nobody has built, recorded in four
  unrelated places and findable from none of them.

## Capabilities

**New Capabilities**

- `vouched-set` — what owns a reader's declared vouches and dismissals, what
  replay may and may not do to each region, what a vouch resolves to when the
  reader holds no ops for the identity named, how the state fails when
  unreadable, and what the module exposes across the wire.

**Modified Capabilities**

None yet. `op-log` is untouched: this change adds no op kind, and the point is
that the authoritative half is not in the log. The scorer that consumes weight
classes does not exist, so there is no spec of it to modify — §7.3 schedules
vouching after §7.2's interim ordering ships, and this change deliberately lands
the *storage* question ahead of the scorer rather than inside it.

## Impact

- Documents only. No source file changes and no wire-contract change today — the
  surface argued for in `design.md` is what the implementing change adds.
- PLAN.md gains a collected rendering-obligations section and cross references
  into it.
- §7.3 and §7.4 live on `docs/relevance-votes`, which is unmerged and has moved
  once during this change. Every citation is by requirement name rather than by
  line, and nothing here depends on that branch's text landing verbatim.
