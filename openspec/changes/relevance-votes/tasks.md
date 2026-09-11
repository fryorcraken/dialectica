# Tasks

This change is documents only. No code, no schema. What follows is the work it
authorises, ordered so that the parts with an external deadline come first.

## 1. Coordinate with the projection schema (deadline-bound)

- [x] Establish that `decay(age)` is uncomputable today and say why, so the
      schema does not reserve for a decay it cannot feed.
- [x] Hand the schema agent the three things it must reserve, and take its
      correction on the decay epoch (Lamport, not a receive clock).
- [x] Answer its joint-indexability question: `is_hidden` is a bounded,
      event-invalidated derived set, not rule 5's continuously-varying key.
- [ ] Confirm the per-target aggregate lands as a **view** column set rather
      than growing the op log, per §3.3. The log stores raw
      `(target, voter, direction)` facts and decides nothing about them.

## 2. PLAN.md

- [x] Rewrite rule 2: ships `top`, claims engagement rather than relevance,
      states the one reason it is safe.
- [x] Narrow rule 3 to the end state; record that an unmintable credential may
      weight today and is still not a defence.
- [x] Extend rule 4 to the moderator-downvote collision.
- [x] Correct rule 5 with the no-age-exists finding and the filter answer.
- [x] Add rule 6, the expiry trigger, naming the §4.8 Phase 2 precondition.
- [x] Correct the reserved shape: weight the voter, not the author.
- [x] Fix §7's opening, which said scoring ships nothing.
- [x] Fix "Where the rules come from", which claimed every rule is an inversion.

## 3. Spec

- [x] Draft the `relevance-ordering` delta. Not merged; `openspec archive` is
      the change owner's step, not an agent's.
- [ ] `openspec validate` before archiving.

## 4. Implementation, when this is accepted

Not done here and deliberately not started.

- [ ] The scorer, with `K` as a named constant carrying a comment that it is
      chosen rather than derived, and naming the observations that would move it.
- [ ] The score floor, with a **pair** of boundary tests: the largest accumulated
      downvote total that still places a post last, and one past it, asserting
      the post is still present. A one-sided test proves a floor exists and
      nothing about where.
- [ ] A test that a moderator's upvote and downvote are **not** symmetric — this
      is the rule-4 collision resolved, and it is the assertion that fails if
      someone later "tidies" `K` into applying to both directions.
- [ ] A test that two peers holding identical ops produce identical orderings,
      including through the tie region where every post scores zero.
- [ ] A test that pagination over an all-equal-score Stoa returns each post
      exactly once. This is the one the op-id tiebreak exists for, and without it
      the index change looks like a free simplification to a later reader.
- [ ] Mutation-verify each of the above: break the property, record which tests
      fail, restore, report the table.

## 5. Vouching — decided here, built elsewhere

- [x] Choose the vocabulary and record why each alternative was rejected
      (PLAN §7.3). This is what the owner asked for.
- [x] Establish that a vouch is never published, and the three independent
      reasons — §5.2 re-linking, sybil amplification, no convergence needed.
- [x] Confirm it adds no schema requirement, and relay the class-count change.
- [x] Establish that an explicit-only vouch list ships dead, and that weight
      must accrue from what the reader already does (PLAN §7.3).
- [x] Settle that **assessments accrue and responses do not** — weight earned by
      agreement would build a machine that finds a reader more of what they
      already think.
- [ ] **Its own proposal**, covering the question this change does not answer:
      where per-reader local state lives, how it persists across replay, whether
      it is exported between devices, and what a vouch naming an identity the
      reader holds no ops for resolves to. It must also settle the earned-weight
      cap below `K_vouch`, and decay-with-disuse — which wants the same age
      input §7.2 rule 5 says does not exist, so it is a property specified and a
      mechanism deferred.

## 6. Two axes (PLAN §7.4)

- [x] Establish the axes are independent (all four corners populated) rather
      than assuming a second control is warranted.
- [x] Settle the asymmetry: assessment orders, response never sums.
- [x] Confirm both fit the existing one-byte discriminant, and that unknown
      discriminants already fail closed.
- [x] Spec the axis separation and the never-becomes-moderation property.
- [ ] Implementation, when accepted, needs these tests specifically:
  - [ ] A target's position is **identical** under unanimous agreement,
        unanimous disagreement, and no responses. This is the requirement that
        fails if anyone later "simplifies" the two axes back into one.
  - [ ] A response carrying no assessment moves nothing.
  - [ ] Unanimous negative assessment leaves a target present and not hidden,
        including when the assessor is the Stoa's moderator.
  - [ ] An older peer refuses an unknown discriminant rather than counting it —
        pin the discriminant values with hardcoded `assert_eq!`, since
        `cargo mutants` cannot see a wrong `const`.
- [ ] **Measurement, not reasoning**: ship able to observe whether `noise`
      drifts into meaning "disagree". §7.4 says the first finding that
      contradicts it is its answer, and that is only true if it can be observed.

## 7. Not in scope

- Decay. There is no age to decay; the column pair is reserved and unused.
- Any system credential other than the moderator set. Rule 3 owns that.
- Auto-hiding at a vote threshold. Refused, structurally, by the floor.
- Transitive vouching. A web of trust needs loop detection, depth limits,
  per-hop decay and a rule for contradictory paths, and it recreates the sybil
  amplifier locally — one bad vouch imports a stranger's whole graph. Its own
  design with its own evidence, never a parameter added to this one.
- Any UI surfacing vouch counts. That is a published vouch graph reconstructed
  by eye, with all three of §7.3's problems.
- **Any net agreement figure, anywhere.** A net is what makes disagreement feel
  like damage, and it discards which of §7.4's four corners produced it. The
  response axis is displayed as a distribution or not at all.
- A third axis. Two controls is already the part of this design most likely to
  be too much; adding a third on reasoning rather than evidence would repeat the
  mistake at greater cost.
