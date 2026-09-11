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
- [x] Settle that **only upvotes accrue** — accruing negative weight would let a
      reader's disagreements quietly build a filter that hides a viewpoint from
      them, which is worse for being invisible.
- [x] **Record the gap the single axis leaves**, rather than papering over it: an
      upvote blends "worth reading" with "I agree", so a reader who upvotes only
      what they agree with builds a vouched set that agrees with them. Nothing in
      v1 prevents this; the honest claim is that vouching is explicit and
      revocable, not viewpoint-neutral.
- [ ] **Its own proposal**, covering the question this change does not answer:
      where per-reader local state lives, how it persists across replay, whether
      it is exported between devices, and what a vouch naming an identity the
      reader holds no ops for resolves to. It must also settle the earned-weight
      cap below `K_vouch`, and decay-with-disuse — which wants the same age
      input §7.2 rule 5 says does not exist, so it is a property specified and a
      mechanism deferred.

## 6. One axis, vouch, report (PLAN §7.4)

- [x] Commission a literature review before committing to a vote model, and
      **follow it when it contradicted this document's own recommendation**.
- [x] Withdraw the two-axis design; keep the rejected argument in `design.md`
      §13 with the reason it failed, so it is not re-proposed.
- [x] Withdraw `contested` — a 50/50 split is where a voter network is *most*
      polarised, which is not "good arguments I disagree with", and the sort has
      never been studied.
- [x] Record bridging as the evidenced future direction, and why it is
      downstream of vouch rather than parallel (PLAN §7.4, §13; `design.md` §14).
- [x] Record the report's own limitations rather than dissolving them: it
      petitions, the queue has one reader, it is weaponisable with no backstop,
      and its precision is unmeasured in any decentralised system.
- [ ] **Parked, deliberately undesigned:** whether a disagreeing reply is itself
      a quality signal (§7.4, §13). Weigh against bridging, which addresses the
      same gap with measured results but needs a rating-matrix density and a
      sybil-resistance layer this forum lacks.

## 7. Requirement → test, with the gaps named

**No requirement below is covered today. Nothing in this change is code**, so
this is a plan for the implementing change, not a coverage claim. The right-hand
column says what would make each scenario fail — a scenario with no answer there
is one that should not have been written (agents README: "never write a scenario
that cannot be tested").

| Requirement | Testable by | Fails if |
|---|---|---|
| An ordering is derived from ops — **no op carries a score** | **Testable today, and written** (`op::tests::a_vote_carries_no_score_field`). Pins the vote encoding's length against its enumerated fields, the pattern `an_op_carries_no_ordering_fields` already uses | a score field is added to the vote op |
| — **and peers never reconcile** | A peer's ordering changes with **no new op arriving**. Observable after a transport lands: a reconciling implementation emits traffic a conforming one does not | scores are gossiped rather than derived |
| A hidden post is excluded, never ranked down | Hidden post with more votes than any other; assert absent at every position | exclusion becomes a score penalty |
| Exclusion follows the moderation resolver | Compare stored `is_hidden` against a direct resolve; feed a non-binding `hide` | a second copy of the authority rule drifts |
| Counts distinct voting identities | One identity votes twice; assert count is one | repeated votes accumulate |
| A vote counts only where authentic and in scope | Forged vote; cross-Stoa vote; assert neither moves a position | verification or scope check is skipped |
| Weight comes from system or reader, never voter | Op asserting its author's standing; assert no weight change | a self-asserted property is honoured |
| A weight amplifies promotion, never suppression | Moderator upvote vs downvote, mirrored; assert magnitudes differ | `K` is "tidied" into applying both ways |
| Negative engagement floors at zero | **Pair**: largest downvote total that still places last, and one past it, both present | a one-sided test lets the floor drift |
| Neither vote nor report becomes moderation | Unanimous downvotes; many reports; assert present and not hidden for a non-reporting reader | a threshold sneaks in |
| A report affects only the reporter's view | One reader reports; assert every other ordering unchanged | a local action leaks into shared state |
| An ordering is total | Paginate an all-zero-score Stoa; assert each post once | the op-id tiebreak is dropped as "redundant" |
| Age only via a value no author supplies | Op carrying a time-like field; assert unread | a wall clock is reintroduced |
| Never aborts | Adversarial log; votes naming votes; overflow the accumulator | a panic reaches the module boundary |

**Gaps to state plainly rather than imply away:**

- **One row is testable today and now written** — the no-score-field pin above.
  **Every other row waits on a scorer**, which this change does not ship. An
  earlier draft said *every* row was untested, which was both false and falsely
  *negative*: an over-broad disclaimer tells the next reader not to write a test
  that is writable now, which is the false-coverage problem inverted.
- **"Age only via a value no author supplies" is _vacuously satisfied_ today**,
  not merely untested: no op carries a timestamp, so the field it forbids
  reading does not exist. Recorded as vacuous deliberately — this is the §6
  "a new check retires an old test" trap pre-armed, and the day an authorship
  time arrives, this requirement stops being free and needs a real test.
- **The `is_hidden` backfill on late genesis** needs a genesis record arriving
  after the ops it authorises — constructible, but it depends on the projection,
  which is another change.
- **The weight ordering is _not expressible in this delta_.** An earlier draft
  claimed a test could pin `plain < earned < K_vouch < K_mod`. It cannot:
  `earned` and `K_vouch` live in PLAN §7.3 and are deferred to the vouch
  proposal (§5), so two of four terms are out of scope — and the one in-scope
  relation is under `MAY` ("a vote by a moderator **may** weigh more"), which
  `K_mod == plain` satisfies. **Nothing in this spec pins the ordering at any
  point**, and it becomes testable when the vouch proposal lands. What *is*
  pinnable here, and is scheduled above, is the **asymmetry** under a `SHALL`:
  a moderator's upvote and downvote differ in magnitude.
- **Nothing here tests the vouch mechanism**, which is a separate proposal (§5).
- **The single axis's conflation is not testable at all.** It is a claim about
  user behaviour, and the only honest response is instrumentation, not a unit
  test.

## 8. Not in scope

- Decay. There is no age to decay; the column pair is reserved and unused.
- Any system credential other than the moderator set. Rule 3 owns that.
- Auto-hiding at a vote threshold. Refused, structurally, by the floor.
- Transitive vouching. A web of trust needs loop detection, depth limits,
  per-hop decay and a rule for contradictory paths, and it recreates the sybil
  amplifier locally — one bad vouch imports a stranger's whole graph. Its own
  design with its own evidence, never a parameter added to this one.
- Any UI surfacing vouch counts. That is a published vouch graph reconstructed
  by eye, with all three of §7.3's problems.
- **A second vote axis**, in any form. Rejected on evidence (`design.md` §13,
  §14), not on taste — it has no published evaluation anywhere, and the measured
  constraint on distributed moderation is latency rather than expressiveness.
- **A `contested` or controversial ordering.** A 50/50 split is mechanically the
  polarisation maximum, and the sort has never been studied for whether it
  surfaces anything worth reading. Bridging is what it was reaching for.
- **Bridging-based ranking**, for now. It is the evidenced answer and it is
  downstream of vouch data existing and of a sybil-resistance story (§7.4).
