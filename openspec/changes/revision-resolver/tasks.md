# Tasks

## 1. Settle the chain question before writing any of it

- [x] Read §5.7 in full, plus §3.3 (partial op sets), §5.2 (per-Stoa identity),
      §13 (the ordering gap), and `op.rs`'s two tests that pin what verification
      deliberately does not decide.
- [x] Decide whether a `Revise` may name another `Revise`. **Chose flat** — every
      version names the original post. Decided on two grounds: a chain is not
      resolvable over a partial set (a peer missing an intermediate link cannot
      reach versions it already holds), and a chain forks with no defined answer
      because `cmp_ops` orders ops and has no notion of a branch. Recorded in
      `design.md` with what the flat rule gives up and what would reverse it.
- [x] Confirm the op format permits the chained form, so the rule is enforced
      where it is decided rather than assumed from the encoding. It does; the
      resolver enforces it on read and a test pins it.

## 2. The spec

- [x] `specs/post-revision/spec.md` as a delta with `## ADDED Requirements`.
- [x] Require the authorship rule, stating the consequence of its absence
      (anyone rewrites anyone's post) as requirement rationale rather than
      leaving it to the code.
- [x] Require verification *before* the authorship comparison, and say why the
      order is load-bearing. Both are observable, so both belong in the spec.
- [x] Answer all four questions the spec was required not to leave to judgement:
      a target that is not a `Post` (absence), a `Revise` naming an op the log
      lacks (absence for that op id), a complete tie under `cmp_ops` (impossible
      between distinct entries — the log is keyed by op id, so a full tie is the
      same op), and whether revisions chain (they do not).
- [x] Require the order to be delegated, not reimplemented.
- [x] Require a defined answer over a partial set, with no error outcome.

## 3. The code

- [x] `revision.rs`: `CurrentVersion`, `current_version`, `is_valid_revision`.
- [x] Generic over the `OpLog` trait, not over `MemoryOpLog` — the SQLite
      implementation §3.3 names is the reason the trait exists.
- [x] `find` over `iter_target`, never a `sort` or a `max_by`. No reference to
      `lamport()` anywhere in the module.
- [x] The three validity conditions in one function, in a fixed order, so "is it
      checked everywhere?" has an answer.
- [x] `body()`/`attachments()` on the type, so the `Post`-or-`Revise` match is
      written once rather than at each call site.
- [x] The unreachable match arm answers rather than panics — this module runs on
      attacker-supplied content (PHASE0-FINDINGS §3).
- [x] `lib.rs`: one `pub mod` line.
- [x] Op format, `arrival.rs`, `log.rs` and transport wiring all untouched.

## 4. The `iter_target` recency correction, raised mid-change

The coordinator reported that `iter_target`'s doc comment — which said resolvers
want "the ops about this subject, most recent first" — was **wrong**, and had
already misled the moderation resolver. "Most recent first" holds only on the
transport-ordered branch; production never reaches it, and the degraded branch is
ascending op id, which carries no recency at all.

- [x] Correct every place this change asserted recency: the module docs, the
      `find` comment, the `CurrentVersion::current` field doc, the spec's
      ordering requirement, and the proposal.
- [x] State the property that IS delivered, rather than deleting the false one:
      two peers agree on which version is current even though neither knows which
      was written last. Convergence, not recency.
- [x] **Find and fix a claim the correction invalidated.** The module documented
      that "the answer only ever moves forward as ops arrive" and a test named
      `the_answer_only_moves_forward_as_ops_arrive` pinned it — using a
      transport-ordered fixture. Monotonicity is **false** under the degraded
      order, so that test was pinning a regime-specific property under a
      regime-neutral name. Renamed to
      `under_a_transport_order_the_answer_only_moves_forward` and paired with
      `under_the_degraded_order_a_late_arrival_can_change_the_answer`, which
      asserts the *opposite* outcome. Both pass; the pair is the honest statement.
- [x] Rebase onto `origin/phase2/op-log`, and build with `--all-targets` (main
      made `Genesis::address` fallible, and the breakage is in `#[cfg(test)]`
      code that plain `build` does not compile).

## 5. Tests, and mutation-verifying them

- [x] Expectations hardcoded, never read back from the implementation.
- [x] The two-revisions fixture *determines* which op id is lower rather than
      assuming it, following `arrival.rs` and `log.rs`.
- [x] Every forgery fixture asserts it is genuinely a forgery, and every
      stranger fixture asserts the key genuinely differs — or the test proves
      nothing.
- [x] Unspecified behaviour marked: one `// NO SPEC:` comment (writing the
      resolver generically over the trait — an internal Rust API shape, not
      observable behaviour, so it stays a marker rather than becoming a
      requirement). The chain decision's marker was **removed**: the spec delta
      now requires it, so a `NO SPEC:` prefix would have told the next reader it
      was an unpinned default when it is a contract. The rationale stayed and now
      points at the requirement.
- [x] Mutation-verify every asserted property. Ten mutations, each caught:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | The **authorship check** removed (`candidate.author == original.author` → `true`) | `a_revision_by_a_stranger_is_dropped`, `a_strangers_revision_loses_to_an_older_one_by_the_author`, `a_strangers_revision_is_dropped_when_it_is_the_only_one`, `every_key_but_the_authors_is_rejected`, `two_peers_holding_the_same_ops_resolve_to_the_same_version` (5) |
| 2 | The **`verify()` call** removed | `a_forged_revision_is_dropped`, `a_forgery_does_not_displace_a_genuine_older_revision`, `a_revision_with_a_garbage_signature_is_dropped`, `a_revision_lifted_into_another_stoa_is_dropped`, `resolving_against_a_log_of_junk_never_panics` (5) |
| 3 | **Ordering delegation broken**: takes the *last* valid entry instead of the first, ignoring the position `cmp_ops` assigned | **12** tests, including `the_highest_lamport_revision_is_current`, `the_lamport_timestamp_decides_currency_against_op_id_order`, `a_lamport_tie_is_broken_by_ascending_message_id_against_op_id_order`, `under_the_degraded_order_the_lower_op_id_is_current`, `an_ordered_revision_beats_an_unordered_one_whatever_its_op_id`, and both regime pairs |
| 4 | The **`Revise` kind check** removed, so a moderation or vote can be a "version" | `the_kind_check_holds_at_the_top_of_the_order`, `a_moderation_or_a_vote_on_the_post_is_not_a_version_of_it`, `resolving_against_a_log_of_junk_never_panics` (**3**) |
| 5 | The **"only a post has versions" guard** removed | `an_op_that_is_not_a_post_has_no_current_version`, `a_revision_of_a_revision_is_not_a_version_of_the_post`, `an_op_naming_itself_terminates` (3) |
| 6 | **`body()` reads `original`** instead of `current` | 14 tests |
| 7 | **`is_revised()`** always `false` | `a_reply_is_a_post_and_has_versions`, `an_empty_revision_body_is_an_edit_and_not_an_absence`, `the_highest_lamport_revision_is_current`, `under_a_transport_order_a_peer_missing_the_newest_revision_resolves_to_an_older_one` (4) |
| 8 | **`attachments()` reads `original`** instead of `current` | `the_attachments_come_from_the_current_version` (1) |
| 9 | **`attachments()` falls back to `original` when `current`'s list is empty** — the empty-means-unset mistake | `a_revision_clearing_the_attachments_removes_them` (1) |
| 10 | **`body()` falls back to `original` when `current`'s body is empty** — the same mistake on the other field | `an_empty_revision_body_is_an_edit_and_not_an_absence` (1) |

**Mutation 4 found a real gap in the tests, which is why it is worth doing.** On
its first run only ONE test failed. `resolving_against_a_log_of_junk_never_panics`
held a moderation and a vote naming the post, so it *looked* like it covered the
kind check — but both arrived `unordered()`, which sorts them below every
Lamport-ordered revision in the same fixture, so neither could ever have won and
the kind check was silent. The exact "a fixture where two rules cannot be told
apart" defect the agents README records. Fixed by giving both the **top** of the
order (`Lamport u64::MAX`, lowest message id) and the post's own author, so
everything except the kind check says they should win. The mutation then failed 2.

**On making the two rules disagree.** Every currency test assigns its Lamport
values and message ids *against* op-id order — the revision with the lower op id
carries the higher Lamport value, or the higher message id — so a resolver that
ignored the arrival metadata and fell back to op id returns the opposite of what
is asserted. Mutation 3's 11 failures are what confirms those fixtures bite.

## 6. What the blind spec-test review found, and what it cost

A sighted correctness reviewer approved this change with no Important findings.
A **blind** spec-test review — reading the spec and tests without the
implementation — then found a surviving mutation on the same code. Recorded
because it is the clearest evidence this project has for why that role reads
blind.

- [x] **[IMPORTANT] The attachments-clearing bug survived every test.**
      Mutation 9: `attachments()` returning `current`'s list only when non-empty
      and falling back to `original` otherwise. **All 37 tests passed.** §4.6
      makes attachments Logos Storage CIDs, so the consequence is a deleted
      image still being fetched and rendered.

      What made it airtight rather than a judgement call: the reviewer built the
      **body analogue** of the identical bug and it died to exactly one test,
      `an_empty_revision_body_is_an_edit_and_not_an_absence`, whose comment
      already said "a resolver treating an empty body as absent would silently
      restore text the author deleted". **The reasoning existed for `body` and
      had no counterpart for `attachments`** — the only fixture varying
      attachments went non-empty to non-empty, so the empty branch was never
      taken. Fixed by `a_revision_clearing_the_attachments_removes_them`, written
      first and confirmed failing under the mutation before the code was
      restored. Now specified too, as "The current version's content wholly
      replaces the original's".

- [x] **[IMPORTANT] A regime-neutral name on a regime-specific property.**
      `a_peer_holding_fewer_revisions_resolves_over_the_ones_it_has` asserted
      that the peer holding MORE revisions resolves to a LATER one — true only
      under a transport order. The reviewer measured the degraded version: with
      `two_revisions_by_ascending_id` controlling the hashes, **both peers
      resolve identically** and "holding more" becomes unobservable. A comment
      acknowledging this was judged insufficient against **this change's own
      precedent** — the rename of `the_answer_only_moves_forward_as_ops_arrive`
      established name-plus-counterpart, not a footnote. Corroborated by the fact
      that this test fires under mutation 3, so it was acting as an ordering test
      under a partial-set name.

      Renamed to `under_a_transport_order_...` and paired with
      `under_the_degraded_order_a_peer_holding_more_may_resolve_identically`,
      which asserts the regime-neutral half: both peers answer, neither errors,
      and the answer is a function of the set held rather than arrival sequence.

- [x] **[MINOR] The kind check was half-hidden in the panic test.** Measured:
      under mutation 4, `resolving_against_a_log_of_junk_never_panics` failed at
      its final body assertion, **not on a panic** — so half the coverage of one
      of only two guards on the `Revise` kind check lived in an assertion the
      test's name does not describe. Split out as
      `the_kind_check_holds_at_the_top_of_the_order`; mutation 4 now kills 3.

- [x] **[MINOR] A stale `NO SPEC:` marker.** See §5 above.

### Three properties the spec requires that no test can pin

Raised by the reviewer as unjudgeable blind. No code change; each now says so in
the spec, because each previously read like a testable requirement.

- **Verification preceding the authorship comparison is unobservable.** A
  resolver doing them in either order rejects the same set. `a_forged_revision_is_dropped`
  pins that *both* checks happen, which is as close as an external test reaches.
- **"Defines no order of its own" is unpinnable.** A *correct* second
  implementation of `cmp_ops` gives identical answers, so the requirement guards
  against future drift rather than present behaviour. Verifying it means reading
  the code for the absence of a comparison — the `tasks.md` checkbox above is a
  grep, not a test, and is recorded as such.
- **The unreachable match arm is untestable by construction**, and is required
  anyway because the cost is asymmetric: an empty answer is a rendering, an abort
  is a denial of service.

## 7. PLAN.md

- [x] One line in §5.7 that the resolver exists, per the document model.
- [x] Reasoning stays in `design.md`; not duplicated into PLAN.md.

## What the green gate structurally cannot see

Carried in `design.md` under that heading, since it is reasoning rather than a
checklist. In short: the ordered branch is never exercised by a real transport,
the authorship rule binds only conforming *readers* and cannot prevent
publication, no op has ever reached this code from a network, per-post resolution
cost is untested at realistic history sizes, and the convergence test builds two
logs in one process rather than on two machines.
