## Stages

- [x] spec — `spec-writer` — seven deltas, two of them withdrawing merged
      prohibitions (`op-ordering`'s own-clock ban, `op-format`'s
      no-ordering-field rule). Four were **not** in the original scope and are
      here because the change makes text they already carry false rather than
      merely incomplete: `moderation-resolution` (its degraded preference rested
      on "exactly two ops can ever exist", which free preimage bytes dissolve),
      `post-revision` and `thread-read` (both assert no Lamport value reaches
      us), and `composer-view` (which must now prevent a double-tapped submit,
      because core stops absorbing it). See `proposal.md` for the six answers and
      the "most recent first" verdict.
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## Implementation

<!-- The dev-writer's. Left empty by the spec-writer. -->

## Notes for the `dev-writer`, which are not tasks

Things the spec deliberately does not settle, and things the survey of the
current code found that will bite. Neither is a checklist; both are here so they
are met before they are discovered.

**Four files currently carry written arguments against exactly what this change
builds.** Each needs a retraction or a citation of why an author-asserted clock
inside the signed preimage is a different thing from a receiver-asserted one
outside it — leaving them standing is how the next reader concludes the code is
wrong. They are `op.rs`'s module header ("No wall-clock timestamp [...] a forum
that ordered by it would be ordering by a field its adversary sets"),
`arrival.rs`'s header ("Why a second Lamport clock is the one thing not to
build"), `transport.rs`, and `log/sqlite.rs`'s `score_epoch` comment.

**`moderation.rs`'s `Hide`-wins preference rests on a premise this change
falsifies.** Its doc states that a `Moderate` op has "no nonce, no timestamp and
no free byte", so "exactly two ops can ever exist" for one Stoa, moderator and
target. Adding the two fields makes that false, and the spec delta says so. The
preference itself stays correct — it is already keyed on whether the *leading*
candidate is ordered — but its justification and the tests built on the two-op
premise both need revisiting.

**`op.rs`'s `one_of_each_kind()` is a hand-maintained fixture list read by ten
tests**, and is the recorded stale-sweep shape. Every op it builds needs the new
fields; a kind omitted from it loses coverage in all ten silently.

**`log/mod.rs`'s `Entry::target` is deliberately exhaustive with no wildcard** and
is the one enumeration the compiler enforces. The sort keys the SQLite projection
materialises are derived from the `Arrival` at write time and must now come from
the op — that is a `LAYOUT_VERSION` bump, and the existing version check refuses
an unrecognised layout rather than migrating it.

**The constants the spec names but does not fix**: the counter advance bound, the
wall-clock future allowance, and the wall-clock floor. Each is required to be a
named constant, identical on every peer, and pinned against silent drift the way
`MAX_FIELD_LEN` and the channel prefix are. Choosing the values is the
`dev-writer`'s, and `design.md` is where the reasoning for each belongs.

**PLAN.md pruning this change owes**, since the spec now states the behaviour:
§13's Lamport entry and its `createdAt` entry are both answered and should be
struck through and pointed at the spec, with the reasoning moving to `design.md`
rather than being restated; §9.1 §8's feed-label bullet is resolved by the field
landing; §5.7's "no Lamport value reaches us today" parenthesis and §6's
suspended-reversibility note become false on merge.
