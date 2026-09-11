# Tasks

## 1. Spec

- [x] `proposal.md` — why the change, which capabilities it touches.
- [x] `specs/moderation-resolution/spec.md` — the delta, eight requirements.
- [x] `design.md` — the fail-closed argument, the three checks and why the third
      is not redundant, what has to change when the moderator set becomes mutable.

## 2. The resolver

- [x] `Moderators`, constructed only from a `Genesis`, holding the Stoa address
      and the creator key. **Fallible**, because main's genesis title cap makes
      `Genesis::address` fallible, and a record with no address names no Stoa.
      `Genesis` is a plain struct a caller may build directly, so an over-cap
      title never passes the decoder that would refuse it — `unwrap`ping inside
      the library would turn peer input into a process abort.
- [x] `Moderators::authorises` — scope, authority and authenticity in one
      predicate, with one call site.
- [x] `Moderation<'a>` — `Unmoderated` / `Hidden(&Entry)` / `Unhidden(&Entry)`,
      with `is_hidden()` and `deciding_op()`.
- [x] `resolve(log, moderators, target)` — first binding `Moderate` over
      `iter_target`, generic over `OpLog`.
- [x] `pub mod moderation;` in `lib.rs`.

## 3. Tests

Each spec requirement's scenarios, plus the fixture work the requirement did not
itself demand. For the count, run the suite — CLAUDE.md: do not write down what a
command can answer. (An earlier draft of this section said "27 tests", which was
stale and contradicted §10 of this same file.)

- [x] **Authority on read.** `a_moderation_by_a_non_moderator_does_not_hide_anything`
      (authentic, no authority — asserts `verify()` is true first, so it is the
      authority check under test and not the signature check);
      `a_moderation_forging_a_moderators_authorship_does_not_hide_anything` (claims
      the moderator, signed by someone else — asserts `contains()` is true first,
      so it is the signature check under test). The pair is deliberate: each
      fixture defeats exactly one check, so neither test can pass because the
      other's check happened to fire.
- [x] **The positive case.** `a_moderation_by_the_creator_hides_the_target`.
      Without it every negative test would pass for a resolver that hides nothing.
- [x] **Skip, not stop.** `an_unauthorised_op_does_not_displace_an_authorised_one`
      — the forgery is strictly newer, so "skip and continue" and "take newest
      then validate" demand opposite answers. Pins the **transport-ordered**
      branch only; `..._in_the_degraded_order` is its twin on the branch
      production actually runs, with the Stoa searched so the forgery really
      does sort first.
- [x] **Convergence.** `the_answer_depends_on_nothing_but_the_ops_and_the_moderator_set`
      — two logs, opposite append sequences, same answer and same named op.
- [x] **The moderator set.** `the_creator_is_a_moderator_and_nobody_else_is`,
      `the_moderator_set_names_the_stoa_the_record_addresses`,
      `a_different_creator_yields_a_different_moderator_set`,
      `a_moderator_set_cannot_be_built_from_a_record_that_has_no_address` — the
      title-cap boundary as a **pair** (exactly at the cap accepted, one byte
      over refused with the specific variant), so a cap that drifted upward
      cannot leave a one-sided test green.
- [x] **Scope.** `a_moderators_op_naming_another_stoa_does_not_bind` (Agora's
      moderator signs an op naming Lyceum — authentic, real moderator key);
      `each_stoas_moderator_binds_only_in_their_own_stoa` (**one creator, two
      Stoas**, so the Stoa comparison is the only rule that can separate the ops).
- [x] **Ordering.** `the_order_is_by_lamport_and_not_by_op_id` (higher op id
      carries higher Lamport, so the two rules disagree);
      `a_non_binding_op_leading_the_degraded_read_is_skipped`.

      The second was called
      `the_degraded_order_decides_when_the_transport_ordered_nothing` and this
      bullet claimed it covered "the order production actually runs today".
      **That claim is retracted** — see §9. It never decided anything, because
      its candidate vector held one element. The op-id fallback between two
      *binding* candidates remains uncovered and is listed below.
- [x] **The degraded tie-break.** `a_hide_is_not_defeated_by_the_unhide_hashing_lower`
      (the veto, with the hash order determined not assumed);
      `the_hide_bias_applies_whichever_way_the_hashes_fall` (the opposite
      arrangement, searched, so the bias removes the coin flip rather than
      flipping it); `a_transport_ordered_unhide_still_reverses_a_hide` and
      `the_ordered_branch_is_chosen_by_the_leading_op_not_by_all_of_them` (the
      confinement, in both its directions);
      `the_hide_bias_searches_only_ops_that_already_bind` and
      `a_hide_that_binds_still_wins_over_hides_that_do_not` (that the preference
      cannot resurrect a forgery — see §9).
- [x] **Reversibility, both directions.** `a_later_unhide_reverses_an_earlier_hide`,
      `a_later_hide_reverses_an_earlier_unhide`.
- [x] **Kinds.** `a_revision_does_not_clear_a_hide`;
      `a_revision_by_the_moderator_themselves_still_does_not_moderate` (asserts
      `authorises()` passes on that very entry, so only the kind check refuses
      it); `a_vote_by_a_moderator_does_not_moderate`.
- [x] **Partial sets.** `a_target_no_op_names_is_not_hidden`,
      `an_empty_log_answers_and_does_not_error`,
      `a_hide_binds_even_when_the_target_op_never_arrived`,
      `a_peer_missing_the_newest_unhide_still_reports_hidden` (asserted as a pair).
- [x] **Naming the op.** `the_deciding_op_carries_its_author_and_action`,
      `an_unmoderated_target_names_no_op`,
      `a_restored_target_is_distinguishable_from_one_nobody_moderated`.
- [x] **Hostile input.** `a_log_full_of_forgeries_resolves_without_a_panic_and_hides_nothing`
      — seven ops naming the target, each failing exactly one check, under five
      arrival shapes. Asserts `Unmoderated`, not merely "no panic".
      `one_genuine_hide_among_the_forgeries_still_binds` is its complement, and
      is what stops the first being satisfiable by a resolver that returns
      `Unmoderated` unconditionally.

### Coverage this change does NOT have

Stated rather than implied, because a false coverage claim is worse than a
missing test.

- **Two distinct moderators.** Needs a mutable moderator set, which does not
  exist. `the_authority_predicate_consults_the_set_and_not_the_earlier_ops_author`
  therefore tests the checkable half — that authority is a membership test
  consulting nothing about who placed earlier ops — and carries a `NO SPEC:`
  marker saying so. It does not test two-moderator behaviour, because that
  behaviour cannot be built today. (It was named
  `a_moderator_may_reverse_a_moderation_they_did_not_place`, which overstated
  what its fixture could show; see §9.)
- **The op-id fallback between two binding candidates.** `resolve`'s third case,
  `unwrap_or(first)`, decides when every binding candidate is an `Unhide`. Two
  binding `Unhide`s of one target need two moderators, so the arm is unreachable
  from any fixture buildable today. Recorded in `design.md` as well as here,
  because this file is archived with the change and that one is the durable
  record.
- **The moderator set as of an op's Lamport position.** Indistinguishable from
  the constant set today. `design.md` records it as the first thing to change.
- **`Genesis::matches` being enforced by the resolver.** It is not, by design,
  and the requirement on `Moderators::of` is documentation rather than a check.
  That is stated in `design.md`'s Risks.

## 4. Mutation verification

Every property claimed above was broken in the source, the suite run, the
failures recorded, and the source restored. Seven mutations; the table is in the
report.

- [x] Authority check removed → 5 tests fail
- [x] `verify()` removed → 3 fail
- [x] Stoa scope check removed → 4 fail (2 before a fixture defect was fixed)
- [x] Ordering reversed → 5 fail
- [x] `Unhide` no longer reverses → 5 fail
- [x] Kind filter widened → 8 fail
- [x] Validate-after-select instead of skip-and-continue → 4 fail
- [x] `Hide`-wins tie-break removed → 1 fails (the veto regression test)
- [x] Tie-break applied unconditionally, not just in the degraded branch → 3
      fail. Both directions matter: the first proves the fix does something, the
      second proves it does not break reversibility once real ordering arrives.
- [x] **Mutation I** — bias searches `iter_target` instead of the validated
      candidates → **2 fail** (was 0, and this one reinstates §6.2's defect)
- [x] **Mutation C** — ordered-branch condition becomes `all(...)` over every
      candidate rather than the leading one → **1 fails** (was 0)

**One defect found this way.** `each_stoas_moderator_binds_only_in_their_own_stoa`
originally gave the two Stoas different creators, so the authority check alone
excluded the other Stoa's op and the test survived the scope-check mutation. It
was the exact fixture trap `.claude/agents/README.md` describes — two candidate
rules producing the same answer — and it is fixed by giving both Stoas one
creator.

## 5. PLAN.md

- [x] §6 reasoning moved to `design.md`; §6 left with a one-line statement that
      the resolver exists plus what is still not built.
- [x] §5.7's moderation-flag bullet marked as built.
- [x] Appendix A's "Moderation authority is never checked on read" left as it is
      — it describes another project and does not rot.

## 6. Rebases onto `phase2/op-log`

**Two of them**, recorded separately because the first being marked done is what
made the second easy to miss. `git log --oneline -1 origin/phase2/op-log` is the
check that never goes stale; a "rebased ✓" tick is not.

- [x] **First**, `--onto origin/phase2/op-log 9e9e6c9` (op-log at `83bbad9`),
      one commit replayed, no conflicts.
- [x] **Second**, `--onto origin/phase2/op-log 83bbad9` (op-log now `931bf93`),
      three commits replayed, no conflicts. Design review caught the staleness:
      a two-dot diff against the *current* op-log showed this change apparently
      deleting five op-log tests, three spec scenarios and a design section —
      upstream additions made after the first rebase, one of which
      (`two_targets_sharing_an_op_id_prefix_are_not_confused`) guards
      `iter_target`, the read this resolver folds over. Nothing was ever deleted,
      but the diff as presented read as a moderation change removing a
      prefix-confusion test. The diff is now purely additive.
- [x] **The clean rebase did not compile**, exactly as warned. Main made
      `Genesis::address` fallible; `Moderators::of` and ~46 fixture call sites
      broke, all inside `#[cfg(test)]` for the fixtures — which a plain
      `cargo build` does not compile. `--all-targets` is what surfaced it.
      Fixed at the two producers (`address_of`, `moderators_of` helpers) rather
      than scattering `.expect()` across every call site.
- [x] **Checked whether the new title cap silently retired a test.** It did not:
      the only oversized values here are a `MessageId` and `u64::MAX` Lamport
      values, neither of which passes through a genesis title. Every fixture
      title is a short literal.
- [x] **Two assertions of ours were falsified** by the `iter_target` doc change
      that this change's own finding caused. Both corrected — see §7.

## 7. Assertions falsified by the upstream fix, and corrected

The degraded-order finding reported from this branch went into `arrival.rs` and
then into `log.rs`, whose `iter_target` doc no longer claims recency. That made
our own restatements of it false:

- `moderation.rs`'s module doc and `resolve`'s doc both said `iter_target` is
  "most recent first". Now: the first entry is taken **because that is the
  position the ordering rule defines as current**, with the recency point cited
  to `arrival.rs` and `log.rs` rather than restated.
- `design.md`'s ordering decision said the same, and said "Both are 'most recent
  first' as far as this fold is concerned". Now states the weaker property the
  fold actually needs — the rule defines a first position and every peer computes
  the same one — which is what holds under both branches.
- The spec's ordering requirement now says "whichever of them the ordering rule
  places first" and adds an explicit SHALL NOT against reading the leading op as
  the most recently published one.

## 8. Security review findings, addressed

- [x] **[HIGH] A `Hide` could be permanently defeated by hash luck.** A
      `Moderate` op carries no nonce, so exactly two ops can exist per
      {Stoa, moderator, target}, and under the degraded order the lower-hashing
      one won forever. A bare pre-emptive `Unhide` was therefore a permanent
      veto, and grindable through the Stoa title. **Verified independently
      before fixing**: in the shipped `agora()` fixture the unhide does hash
      lower. Closed by preferring `Hide` when neither candidate was
      transport-ordered — argued in `design.md`, and confined to the degraded
      branch so real ordering still reverses.
- [x] **[MEDIUM] A test could not fail for the reason its name gave.**
      `a_moderator_may_reverse_a_moderation_they_did_not_place` had one
      moderator, so the any-moderator and only-the-placer rules agreed on its
      fixture. Renamed to
      `the_authority_predicate_consults_the_set_and_not_the_earlier_ops_author`,
      which is what the body supports; the two-moderator claim moved into the
      marker as explicitly untestable, and `spec.md`'s scenario now says so too.
- [x] **[LOW] Blast radius recorded.** One compromised moderator key can
      `Unhide` every moderation in the Stoa, unrecoverably except by forking,
      until §6.2's thresholds land. Now in both the marker and `design.md`.
- [x] **Test gap closed.** `an_unauthorised_op_does_not_displace_an_authorised_one`
      pinned skip-and-continue only on the transport-ordered branch, which
      production never reaches. Added
      `..._in_the_degraded_order`, whose Stoa is **searched** for one where the
      forgery sorts first — `agora()` is not such a Stoa, and asserting blindly
      made the test fail on its first run.

### One test had to be reworked rather than kept

`the_degraded_order_decides_when_the_transport_ordered_nothing` used a
hide/unhide pair, which the tie-break now governs — so the old fixture would
have kept passing while measuring a different property. It now uses two
`Unhide`s (one binding, one not), where the tie-break has no opinion and the
op-id fallback is what is actually under test.

## 9. Blind spec-test review findings, addressed

The blind role found two surviving mutations the implementation-aware security
review had missed — which is the split working as `.claude/agents/README.md`
intends.

- [x] **[HIGH] Mutation I: the bias searching unvalidated ops survived the whole
      suite.** Had `resolve` looked for a `Hide` in `iter_target` rather than in
      the already-filtered candidates, any peer could forge a `Hide` of any
      target and every reader would report it — §6.2's defect restored on the
      degraded path, the only path in use. The shipped code was always correct;
      the test pinning it did not exist.

      Nothing caught it because the two hostile-input tests are blind in
      *different* ways: `a_log_full_of_forgeries_...` holds no binding op, so the
      fold returns before the bias runs; `one_genuine_hide_among_the_forgeries_...`
      uses ordered arrivals, so it takes the other branch. **No fixture combined
      unordered arrivals + a binding op + a non-binding `Hide`**, which is the
      whole degraded security surface. Added
      `the_hide_bias_searches_only_ops_that_already_bind` (each non-binding
      `Hide` fails a *different* one of the three checks) and its complement
      `a_hide_that_binds_still_wins_over_hides_that_do_not`. Mutation I now kills
      both.
- [x] **[MEDIUM] Mutation C: spec and code disagreed with no test able to tell.**
      Spec said "no competing moderation was ordered" (all candidates); code asks
      about the leading one. They differ on mixed arrivals — reachable via
      `Arrival::from_parts`, and the normal state during a transport upgrade.
      **The code is right**: an ordered leader won its position by a genuine
      comparison, and `cmp_ops` puts every ordered op ahead of every unordered
      one. Spec tightened to say so; added
      `the_ordered_branch_is_chosen_by_the_leading_op_not_by_all_of_them`.
- [x] **[MEDIUM] The previous rewrite did not land.**
      `the_degraded_order_decides_when_the_transport_ordered_nothing` used two
      `Unhide`s, but two unhides of one target need two authors and in a
      one-moderator Stoa the second cannot bind — so the candidate vector held
      one element and the op-id fallback decided nothing. Renamed to
      `a_non_binding_op_leading_the_degraded_read_is_skipped`, with a
      "not verifiable in this change" note naming the mutable moderator set as
      the blocker. **The fallback between two binding candidates is not covered
      and is now marked as such** rather than claimed.
- [x] **Marker 2 split.** The representational choice stays a `NO SPEC:`; the
      veto consequence is specified and tested, so it is now a cross-reference.
      Keeping it under a marker told readers "nobody decided this" about
      something decided.
- [x] **Spec names the fallback order.** It said "a defined order that carries no
      recency" without saying *ascending op id*, so a spec-only reader could not
      write a test for it.
- [x] **The 256 budget is justified.** ~2⁻²⁵⁶ exhaustion probability, stated on
      both helpers, with exhaustion panicking rather than skipping.

### The structural fix, not just the test

The reviewer could not construct either mutation from the spec alone — it had to
read the fold, which is exactly what a blind reviewer should not need. Both gaps
were spec gaps. So the spec now carries **"Only moderations that already bind are
eligible to decide, including under this preference"** as prose plus two
scenarios, rather than leaving it as an implementation detail. A future
implementer working from the spec is now forced into the fixture that was
missing.

## 10. Gates

- [x] `cargo test -p dialectica-core` — green. Run it for the count; it moves
      with every upstream rebase, which is exactly why this file should not
      assert one.
- [x] `cargo clippy -p dialectica-core --all-targets -- -D warnings` — clean
- [x] `cargo fmt -p dialectica-core --check` — **14 pre-existing hunks**,
      measured by stashing this change and re-running, 0 introduced. Three were
      introduced by the `address_of(&lyceum)` substitution pushing lines past
      100 columns, and were hand-fixed. Never bare `cargo fmt`.
