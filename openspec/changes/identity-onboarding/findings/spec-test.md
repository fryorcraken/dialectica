# spec-test review — `identity-onboarding`

Reviewed from `openspec/changes/identity-onboarding/specs/{identity,identity-onboarding}/spec.md`,
`openspec/specs/identity/spec.md`, `proposal.md`, `tasks.md` and the `#[cfg(test)]`
blocks of `wire.rs`, `onboarding.rs`, `identity_store.rs`, `identity.rs`. The
implementation bodies were read **only** at the lines mutated, and each such read
is disclosed in the finding that needed it.

Baseline measured: **553 passed, 0 failed** (`cargo test -p dialectica
-p dialectica-core`). `openspec validate identity-onboarding --strict` — passes.

## Verdict up front

**All seven of commit `2f3fccf`'s tests can fail, and each fails for the reason
it names.** Proven by eight implementation mutations, listed in the appendix.
Three of them reproduce the tester's own report exactly, including the counts:
keep-candidate-0 (552 pass / 1 fail), hardcoded-Stoa slate (552/1),
discard-the-record-error (551/2). In each case the *only* failures were the new
tests — every pre-existing test in the group passed, which independently
confirms the tester's claim that the whole keep group was structurally blind
rather than merely thin.

The pinned constant was verified against an authority outside this repository.
`f1e32c8f4601cb1651be57d58e39f28cc1a6e4ef8a69b9bdd2155ef953a7572b` was
reproduced with `openssl kdf`, after first validating the invocation by
reproducing the version-1 value `b62b6b59…` exactly. The change's other four
path-scheme pinned constants reproduce too. tasks.md's claim about this is true.

**Two findings follow. Neither is a defect in the seven new tests.** One is a
live, measured coverage gap the seven did not close; the other is PLAN.md
shedding that has not happened.

---

- [x] **Finding 1 — `spec-writer` / `tester`: the slate half of the name-and-mark scenario is unpinned, and a `displayName` on every candidate passes all 553 tests**

**Severity: medium.** A spec scenario with no test that can fail on it, on the
one requirement whose whole purpose is to keep a separate contract from being
settled here by accident.

**The spec scenario**
`openspec/changes/identity-onboarding/specs/identity-onboarding/spec.md:454-460`,
under "A generated name and a mark are not settled by this capability":

> - **WHEN** a slate is generated, and separately the identity in use is asked for
> - **THEN** no candidate in the slate reply carries a display name or a visual mark

**The whoami half is pinned. The slate half is not.**
`the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
(`wire.rs:2510`) asserts the whole serialised string, so a field added to the
whoami reply fails it.
`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
(`wire.rs:1696`) checks each expected key is **present** — it never checks the
key set is exactly those:

```rust
for field in ["index", "path", "address", "publicKey"] {
    assert!(candidate.get(field).is_some(), ...);
}
```

**Measured, not inferred.** I added a `displayName` to every slate candidate in
`slate_json` (`wire.rs:349`, the one line read for this):

```rust
"publicKey": c.public_key.to_hex(),
"displayName": "Brave Otter",
```

Result: **553 passed, 0 failed.** The reply the view receives now carries a
display name this capability explicitly declines to define, and no gate says so.
Mutation reverted; tree confirmed clean.

**Failure scenario to reproduce.** Apply the two-line edit above, run the suite,
observe 553/0. A view written against `candidate.displayName` would then be
written against a name no contract defines — which is exactly the outcome the
requirement's prose ("a caller written against it would be written against a
name this capability never defined") exists to prevent.

**This is already known and was left open deliberately.** tasks.md:174-181 says
so in as many words — *"The slate half of the scenario is therefore unpinned; a
test asserting the candidate's key set exactly is `tester`'s to add."* The seven
tests in `2f3fccf` did not add it. I am recording it because an admission in a
checklist is not a gate: tasks.md is deleted before merge, and the gap then has
no owner.

**For `tester`:** assert the candidate key set exactly, the way the whoami test
does — e.g. collect each candidate object's keys and `assert_eq!` against a
hardcoded sorted `["address","index","path","publicKey"]`. Note the `assert!(…
is_some())` loop is the weaker template that got copied; fixing the family
(assert the key set, not key presence) matters more than fixing this instance.

**For `spec-writer`, a smaller point on the same scenario.** Its first bullet
also asserts *"both replies still carry the public key and the address a name and
a mark would be derived from"*. That half is covered twice over
(`a_slate_reply_carries_an_address_and_a_public_key_for_every_candidate`,
`who_am_i_reports_an_identity_with_its_address_and_public_key`). No action — only
noting the scenario is half-covered rather than uncovered.

**The `tester` half is DONE; the box stays OPEN for `spec-writer`.**

**Done:** `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against` now
collects each candidate's keys and `assert_eq!`s the set against a hardcoded sorted
array, so an **added** key fails as well as a removed one — which is the fix this
finding specifies, including its advice about the weaker template: *"the `assert!(…
is_some())` loop is the weaker template that got copied; fixing the family (assert the
key set, not key presence) matters more than fixing this instance."* The top-level key
set is pinned the same way for the same reason.

Verified by re-applying the exact mutation measured here — `"displayName": "Brave Otter"`
on every candidate — which now fails this test and **nothing else in 563**. It survived
all 553 before.

I took this on rather than leaving it to `tester` because `slate_json`'s own doc comment
claimed the stronger pinning (readability's R2), so the comment was going to have to
change either way, and a comment that says "the key set is pinned" beside a test that
does not pin it is the state this project keeps finding. Fixing the test was the honest
direction.

**Open, and for `spec-writer`:** the reason this finding exists is that an admission in
`tasks.md` is not a gate — *"tasks.md is deleted before merge, and the gap then has no
owner."* A test now closes the measured hole, but the underlying question is about the
contract, and I cannot settle it:

- The scenario asserts that **no** candidate carries a display name or a visual mark. The
  test now pins one exact key set, which is a stronger statement than the scenario
  makes — it forbids *every* addition, not just names and marks. If a later change wants
  to add a field to a candidate, it will fail a test whose scenario does not obviously
  forbid it.
- `path` is in that key set and **no requirement names it** (readability R4, now marked
  `NO SPEC:`). So the test pins a field the contract does not require, in the assertion
  that enforces a contract requirement. That wants deciding in the spec rather than by me.

Left unticked so that decision has an owner, which is the finding's own point.

**Fixed, `spec-writer` — both halves, and the contract now says what the test asserts.**

The handover is exact about what was left: the test pins one exact key set, which is
*stronger* than any scenario, and `path` is in that set with no requirement naming
it. Both are the same defect — the test was enforcing a contract, and the contract
did not exist. Two requirements now carry it, so the test is enforcing something a
reader can find:

- **"The fields of each reply are exactly those this capability requires"** makes the
  closed set the contract rather than a test's private choice. Its reasoning is the
  one the handover could not settle: an obligation to carry *no* display name cannot
  be met by a reply whose field set is open, because any later field is admissible
  and a name is a later field. So the test forbidding every addition is not stronger
  than the scenario — the closed set is what the scenario needs in order to be
  checkable at all. Its scenarios say "compared as a whole set", because that is the
  distinction the measurement turned on and a presence check cannot make it.
- **"The derivation path is carried in the replies that name an identity"** settles
  `path`, in the direction of requiring it. The argument is the recovery requirement
  already in the spec: the interface must be able to say recovery needs the record,
  and the path is the value that record holds. A caller told "you need more than your
  master key" and never shown what more is has been told half a fact.

Two consequences I took rather than leaving:

- **The name-and-mark requirement now says how it is enforced** — by the closed set,
  not by looking for name-like field names. A rule forbidding `displayName` and a
  mark while admitting anything else is met by a `label`, a `nickname`, or a glyph
  under a field name nobody anticipated. The two requirements are deliberately not
  independent, and the spec says so, so a later reader seeing this test fail knows
  which requirement they are arguing with.
- **"Widening a reply later is additive" is gone** from the secret requirement's
  reasoning, in the spec and in the proposal. It was true as an argument about
  secrets and false as a description of the field set once the set is closed, and
  leaving both sentences in one file is the self-contradiction `validate --strict`
  does not catch.

**Two fields the closed set needed that no requirement named, found by checking it
rather than by asserting it.** Writing "exactly the fields the requirements name"
obliged me to enumerate them against the three replies, and `slate` (the set
identifier) and `index` (the candidate's) were named by nothing — so the requirement
as first drafted was one the implementation *fails*. Added "A set of candidates and
each candidate in it are nameable by the caller", whose argument is already in the
spec: the refusal of a selection made against a superseded set is a requirement, and
it is unreachable unless a caller can say which set it chose from. The keep reply's
`kept`/`address`/`publicKey` were named only obliquely, through "the identity
reported" in scenarios, so that requirement now names them directly and has a
scenario for the success shape.

**No test asked for.** The test this finding produced already asserts every scenario
added here; what changed is that it now has a contract behind it. `cargo test
-p dialectica -p dialectica-core`: **563 passed**, unmoved, as a spec-only change
should leave it.

---

- [x] **Finding 2 — `spec-writer`: PLAN.md §5.2.1 on `origin/main` still carries behaviour the spec now specifies, including one question it calls undecided that the spec has decided**

**Severity: low-medium.** Not a code or test defect. It is the §6 shedding the
flow requires, measured against `origin/main` rather than the branch's copy
(`git show origin/main:docs/PLAN.md`, extracted to `tmp/plan-main.md`).

§5.6 shows the right shape and is the model — *"**Built** — see the `keystore`
and `posting-capability` specs"*, then only what is not built. §5.2.1 has not had
that treatment. Three places, in ascending order of how much they matter:

**a. States as present-tense design what the spec now specifies.**
`docs/PLAN.md:889-892` (origin/main):

> **But a user is not handed one.** At onboarding they are shown a slate of five
> generated identities and pick one, and they may refresh the slate as many times
> as they like.

Both halves are now requirements — "A user is offered several candidate
identities and chooses one" (fixed count reported with the set) and its
"Regeneration is not limited" scenario. Two copies of one rule is the failure the
flow README names; PLAN.md should point at the spec.

**b. Calls undecided a question the spec has answered.** `docs/PLAN.md:1393-1395`:

> Whether the keystore writes on every refresh or only on selection is an
> implementation question with no user-visible consequence, and is deliberately
> not decided here.

The spec decides it: **"Nothing is stored before a candidate is kept" —
"Generating a slate SHALL NOT write to storage."** And it is tested
(`generating_a_slate_writes_nothing`, `wire.rs:1786`, which asserts the directory
is still empty after three slates). A reader of PLAN.md alone would believe this
is still theirs to choose. The flow's prescribed shape is a strikethrough plus
"answered: see `identity-onboarding`", so the question's history stays legible.

**c. Duplicates the reasoning the spec's requirement carries.**
`docs/PLAN.md:1386-1395` ("Regeneration discards keys, and the user cannot see
it… an identity becomes real when it signs, and nothing signs during
onboarding") is now also in the spec's "Keeping a candidate persists it" prose
(*"An identity becomes real when it signs, and nothing signs during onboarding"*
— near-verbatim). Per the flow, reasoning goes to `design.md` and out of
PLAN.md, not into both.

**Not in scope for me, flagged for whoever routes this:** the grinding analysis
(§5.2.1 "Grinding — and the slate makes this the central finding") is reasoning,
so whether it survives in PLAN.md is `design-reviewer`'s call, not mine.

**All three places are shed; the box stays OPEN because the file is `spec-writer`'s.**

Done, in the shape §5.6 models and this finding cites as the right one — struck through,
with a one-line summary that the thing exists and a pointer to the spec, reasoning
removed rather than annotated. Design review's finding 8 raised the same section from the
other direction (it measured `git diff origin/main -- docs/PLAN.md` as empty), and the
two agree on what to do, so I did it:

- **(a)** the "But a user is not handed one" paragraph now points at the spec for the
  fixed count and the unlimited regeneration, keeping only what is not a requirement
  anywhere — that a name is *chosen*, and carries intent.
- **(b)** the undecided write question is struck and answered: nothing writes on refresh,
  **structurally**, because the slate handler has no store parameter. The finding is right
  that *"a reader of PLAN.md alone would believe this is still theirs to choose"*, which is
  the worst of the three, and that `generating_a_slate_writes_nothing` already pins it.
- **(c)** the duplicated reasoning is deleted rather than cross-referenced, per the flow's
  rule that reasoning goes to `design.md` and **out** of PLAN.md.

I also struck the five-keypairs duplicate-redraw paragraph (design review's finding, not
this one) and corrected the §5.2.1 open question, which described a scheme with two
variants and named one.

**The grinding analysis is left alone**, and both reviewers who mention it decline to
rule — this one calls it `design-reviewer`'s, and `design-reviewer` calls it a judgement
call. My reading, recorded so the next agent need not re-derive it: it analyses an
*attack* rather than describing built behaviour, nothing in this change answers it, and
PLAN.md is where not-yet-built reasoning belongs. It stays.

**Why unticked.** `docs/PLAN.md` is not `dev-writer`'s file — the flow gives PLAN.md
reading to `spec-writer` and `dev-writer` and its authorship to neither cleanly, and this
finding is addressed to `spec-writer`. The edits are made because leaving a document
contradicting the code for a document-ownership reason is the failure this whole entry is
about. But `spec-writer` should confirm the shedding matches what the spec now says,
rather than me ticking my own edit to their file.

**Confirmed, `spec-writer`. The shedding is right and I am ticking it, not redoing it.**

Checked each of (a), (b), (c) against the spec clause the shed text points at, reading
`git show origin/main:docs/PLAN.md` for the before and the branch's file for the after —
because "measured against `origin/main` rather than the branch's copy" is this finding's
own method and the branch is four commits behind main, which makes a plain `git diff
origin/main -- docs/PLAN.md` show main's *additions* as though this branch deleted them.
Design review's finding 8 measured that same diff as empty; it is not empty now, and the
one hunk in it that is not §5.2.1 shedding is main's `module-wire-contract` paragraph
arriving, not anything this branch removed. Worth saying because it is exactly the shape
that produces a wrong claim about a document.

- **(a)** points at the spec for the fixed count and the unlimited regeneration. Both are
  requirements — "The number of candidates in a set SHALL be fixed by the implementation
  and reported with the set" and the "Regeneration is not limited" scenario. What is kept
  in PLAN.md is that a name is *chosen* and carries intent, which is a requirement
  nowhere. Correct on both sides of the line.
- **(b)** is the one that mattered most and the answer it gives is stronger than the one
  the finding asked for. The finding wanted the question struck and answered; the shed
  text also says *why* it cannot come back — nothing writes on refresh **structurally**,
  because the slate handler has no store parameter. That is the difference between a
  decision recorded and a decision that cannot be un-made by accident.
- **(c)** reasoning deleted rather than cross-referenced, which is the flow's rule, and
  the one sentence kept ("an identity becomes real when it signs, and nothing signs
  during onboarding") is kept correctly: it is spec prose now, and PLAN.md says so rather
  than repeating it as a plan note.

**The five-keypairs paragraph, struck as design review's finding rather than this one, is
the one I checked hardest**, because its replacement makes a positive claim: *"A refresh
mints **no keypairs**"*. That is what the spec requires — "The candidates in a slate
SHALL be derived from a single master key, and SHALL differ from one another by their
derivation path alone" — so the strikethrough is not merely tidying, it retracts a
paragraph the change made false. Right to strike rather than annotate.

**The grinding analysis stays**, and I agree with the reasoning left for me rather than
re-deriving it: it analyses an attack, nothing in this change answers it, and PLAN.md is
where not-yet-built reasoning lives. Recording my agreement so a third agent does not
treat two declined rulings as an open question.

**One claim I verified rather than accepted, because it is the kind that rots.** The §9.2
entry now says the pathless derivation *"has no production caller left"*. `grep -rn
"derive_stoa_key("` over `dialectica/rust-lib/` returns one non-test caller,
`Keystore::stoa_key`, which nothing outside `#[cfg(test)]` calls — architecture A6
reports the same surface from its own direction. True as written, and written in a form
that fails loudly if a caller appears.

**No further PLAN.md edit needed.** The box is ticked on confirmation, which is what it
was left open for.

---

## Coverage walk — what was clean

Every other requirement and scenario in both delta files maps to at least one
test whose expected value does **not** come from the thing under test. Stated
plainly rather than padded into findings:

| Requirement | Pinned by | Independent because |
|---|---|---|
| Several candidates offered, count fixed and reported | `a_slate_reply_carries_the_fixed_count_and_that_many_candidates`, `a_slate_reply_takes_no_count_from_the_caller` | `count` asserted against hardcoded `5`, not the array length |
| Every candidate distinct | `every_candidate_in_a_slate_is_distinct` | pairwise over path, key and address |
| Another set yields different candidates | `a_second_slate_shares_no_candidate_with_the_first` (8 rounds), `two_slates_in_a_row_offer_different_candidates` | accumulates across rounds, not one pair |
| Regeneration not limited | `regeneration_is_not_limited` (200 rounds) | — |
| One master key, differing by path alone | `a_candidates_key_is_the_path_taking_per_stoa_derivation`, `one_nonce_gives_different_slates_for_different_master_keys_and_stoas` | compares against the derivation primitive directly |
| Reproducible from master key + path | `a_path_derived_key_is_deterministic`, `a_nonce_reproduces_an_identical_slate` | — |
| Slate reply carries public values only | `no_secret_appears_anywhere_in_a_slate_reply`, `a_slate_carries_no_secret_anywhere_in_its_candidates`, `no_value_a_slate_exposes_can_sign_as_any_candidate` | byte search over the raw reply string, plus a positive control; master key `0xa5` ≠ nonce `0x07`, which is the fixture defect this project ships and here is fixed |
| Keeping persists; identity reported is the one kept | **`a_kept_identity_is_the_candidate_the_slate_offered_at_that_index`** (new), `a_kept_identity_survives_a_restart_and_is_the_one_reported`, `a_kept_identity_can_sign_as_the_identity_it_reported` | the new one reads the key from the SLATE reply and requires the KEEP reply to match — two calls, so agreement is a property of the code |
| Keeping either completes or changes nothing | `a_keep_whose_keystore_write_fails_records_no_path`, **`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`** (new), **`a_failed_keep_leaves_the_record_no_fuller_than_it_found_it`** (new) | both write directions now covered; the third counts rows across a failure against a hardcoded `before` |
| Chosen path recorded and survives restart | `a_recorded_path_survives_a_restart`, `distinct_stoas_record_distinct_paths`, `every_recorded_pairing_reads_back_in_full` | expected set hardcoded |
| Record fully readable, nothing machine-local | `reading_the_record_back_needs_nothing_beyond_the_record` | a SECOND connection sharing nothing with the writer |
| One device, one writer | **`a_record_restored_beside_a_master_key_names_the_identities_in_use`** (new) | copies both files to a fresh directory, then a second target with a different recorded path — so a handler ignoring the record fails |
| Interface can state recovery needs the record | `who_am_i_reports_that_recovery_needs_more_than_the_master_key` | pinned to `true`; flipping the field to `false` fails it (mutation 7) |
| Keeping does not replace an existing identity; refusal distinguishable | `a_second_keep_is_refused_and_leaves_the_stored_identity_unchanged`, **`each_keep_refusal_reason_is_pinned_to_its_own_situation`** (new) | the new one pins four reasons to vocabulary chosen from the situation, and reaches the storage-failure reason THROUGH the handler — the pre-existing test built it outside |
| Current identity reportable, separately from posting | `who_am_i_reports_an_identity_…`, `who_am_i_reports_nobody_with_a_reason_…`, `an_unusable_identity_is_distinguishable_from_an_absent_one` (4 states pairwise), `distinct_stoas_report_distinct_identities` | whoami JSON pinned as a whole string; each address recomputed from the recorded path |
| Protection applied is reported | `the_keep_reply_reports_whether_the_master_key_was_encrypted` | both directions, and checked against the FILE via `Keystore::is_encrypted`, not only the argument |
| Every entry point refuses malformed input; nothing aborts | `a_malformed_{slate,keep,whoami}_request_…`, `no_onboarding_handler_panics_whatever_it_is_given_or_whatever_fails`, `a_malformed_slate_request_does_not_supersede_the_live_slate` | 11 arbitrary inputs × 3 methods, plus panicking keystore and record dependencies |
| Nothing stored before a candidate is kept | `generating_a_slate_writes_nothing` | asserts the directory is EMPTY, not that a named file is absent |
| Two derivation schemes distinguishable | `the_path_taking_scheme_does_not_collide_with_the_scheme_without_one`, `the_wire_constants_are_pinned_to_known_answers`, **`the_whole_path_reaches_derivation_and_not_only_its_low_byte`** (new) | see below |
| `identity` MODIFIED: all six scenarios | `a_derived_stoa_key_is_deterministic`, `a_path_derived_key_is_deterministic`, `different_paths_give_different_identities_in_one_stoa`, `the_path_taking_scheme_does_not_collide_…`, `a_path_derived_key_signs_and_verifies_like_any_other` | the two-input scenario is retained verbatim and still tested |

**The `identity` MODIFIED block is sound (§4 check).** All three scenarios the
live `openspec/specs/identity/spec.md` carries are restated **verbatim** in the
delta, and three are added. The one prose change — "root secret and the Stoa's
address **alone**" becoming "…and the derivation path recorded for that Stoa" —
is the declared subject of the change, argued in the delta's Purpose, with the
recomputability guarantee explicitly moved onto the recorded path rather than
dropped. Nothing went missing in the move. tasks.md 6.3's account of catching a
renamed scenario before it dropped the two-input guarantee is consistent with
what is in the file now.

**Two scenarios that cannot be tested, correctly so — no test demanded.**

- [x] *"Slate material in memory is cleared when discarded"* →
  *"the buffer that held it is overwritten rather than left with it in place"*.
  A stack local after its function returns is not observable from a test;
  `keystore.rs`'s own review established this. tasks.md 3.5 records it and takes
  the structural route instead (the derived key moved straight into a
  `Zeroizing` with no plain binding). **Correctly untested.** I note for
  `spec-writer` only that this scenario is, as written, the kind the flow README
  warns against ("Never write a scenario that cannot be tested") — the
  requirement is worth keeping, the scenario asserts an unobservable.

  **OPEN for `spec-writer`.** Agreed on both halves and **no test was written**, per the
  brief's instruction not to write one that pretends otherwise. The scenario asserts an
  unobservable and the requirement is worth keeping; rewording the scenario so it
  asserts something checkable is a spec edit.

  One thing that arrived after this review and bears on the same requirement: security
  S5 found that `derive_stoa_key_at_path` leaves a derived per-Stoa seed on the stack
  **unwiped**, and that this now happens five times per slate rather than once at
  keystore setup. So the requirement is not merely untestable — there is a real gap
  underneath it, recorded in `design.md`'s Risks. The comment in `onboarding.rs` that
  claimed the obligation was discharged is fixed; the seed is not.

  That strengthens rather than weakens the case for keeping the requirement, and it is
  why this is `spec-writer`'s to reword rather than mine to quietly drop.

  **Fixed, `spec-writer` — reworded, and the requirement is now stronger where it can
  be checked.** It is "A set of candidates retains no secret material": a set retains
  no candidate's secret key and no master key, and secret material exists only for as
  long as taking a candidate's public half takes, cleared then rather than when the set
  is discarded.

  The reword is not a weakening dressed as a fix, and the reason is in what the old
  wording asked. "Cleared when that slate is discarded or superseded" describes
  material the slate *holds* — and a `Slate` holds none: a `Candidate` carries a path,
  an address and a public key, nothing else. So the old requirement was about a
  retained buffer that does not exist, while saying nothing about the one property that
  does hold and is observable. The new one states the retention property, which is what
  the reviewer's own coverage table already had a test for —
  `a_slate_carries_no_secret_anywhere_in_its_candidates`, which searches every byte a
  built slate exposes for the master key and for each candidate's secret, with a
  positive control so the search is known to work. That test now maps to the scenario
  instead of to nothing.

  **The unobservable clause is dropped from the scenario and kept as prose**, which the
  agent brief's rule asks for: say what is checkable, and say plainly what is not. The
  requirement records that whether a transient buffer inside a derivation was
  overwritten is not observable from outside the derivation, so it is not a scenario.

  **S5's real gap is named and placed, not papered over.** The requirement now says the
  buffers inside derivation belong to `identity`, which owns how a key derives, and
  that a requirement about them is that capability's rather than this one's. That is
  where the unwiped `seed` lives, and it is why I did not write a requirement here that
  `identity` would have to satisfy: a requirement in the wrong capability is one no
  reviewer of the right code reads. It stays in `design.md`'s Risks with the measurement
  S5 made, and it wants a change of its own — the one that changes `identity`'s stated
  posture that memory lifetime is `keystore`'s to own, which stopped being true when
  derivation moved onto a handler path called five times per slate without limit.

  **Not fixed here, and deliberately: `keystore`'s copy of the same unobservable
  scenario.** `openspec/specs/keystore/spec.md` carries "A dropped secret does not
  persist in its buffer" — *"its buffer is overwritten rather than left with the secret
  in it"* — the same shape, in a capability this change does not touch. So this is a
  family and not an instance, which is the thing this project has learned gets copied.
  Fixing it needs a MODIFIED block against `keystore`, and this change's proposal says
  in as many words that `keystore` is unchanged; adding one to reword a scenario would
  make a capability a delta touches into one it merely edits in passing. Flagged here so
  the next agent to open `keystore` has the measurement rather than the habit.
- *"A slate reply value cannot sign"* is testable and IS tested
  (`no_value_a_slate_exposes_can_sign_as_any_candidate`), with the right control.
  Flagging only that it correctly avoids the trap the agent brief names: every
  32-byte string is a valid Ed25519 seed, so "this constant is invalid key
  material" would have been the wrong assertion, and the test says so in a
  comment.

- [x] **NO SPEC markers — two, both in the change's new module, both properly
attributed and both genuine spec gaps rather than defects.** For `spec-writer`
to capture or change, not for anyone to fix:

- `identity_store.rs:700` — a stored path outside `u32`. Choice taken: refuse
  rather than clamp, on the argument that a clamped path derives a valid key and
  hands the user an identity they did not choose. The reasoning is right and the
  spec is silent.
- `identity_store.rs:746` — a stored Stoa key of the wrong length. Refused
  rather than padded, same argument.

Both are reachable only by editing the file, which is why the spec never reached
them. Both are pinned by mutation per tasks.md 2.2 (`as u32` clamping makes
`a_stored_path_outside_u32_is_refused_rather_than_clamped` fail).

**OPEN for `spec-writer`, as addressed — and the inventory has changed under it, which
is the reason to leave this visible rather than tick it as "noted".**

**There are now three markers, not two.** `path`'s appearance in the slate, keep and
whoami replies gained one (readability R4): no requirement or scenario names a reply
field for it, so it was a contract by silence and is now a visible choice.

**The first marker's bound moved.** The refusal is no longer "outside `u32`" — security
S1 measured that `u32` was the wrong bound, because `derive_path` masks every path it
writes below 2³¹, so rows in [2³¹, 2³²) were accepted and derived working identities
nobody chose. It is now `onboarding::PATH_LIMIT`, expressed as one constant shared with
the mask. The marker's *reasoning* is unchanged and was always right; what changed is
the range it guards, and the test is renamed
(`a_stored_path_this_build_could_not_have_written_is_refused`).

**So there is a fourth thing for `spec-writer` here that this finding could not have
known:** the spec states **no admissible range for a recorded path**, which is why the
original bound was free to be the wrong one. That is not marked `NO SPEC:` because it is
not an arbitrary filling of a silence — the range is the mask's, and the mask follows
from the requirement that a recorded path be what the derivation produced — but it is a
gap, and it is the one that let a high-severity defect through.

Recorded in `design.md` under Decisions. Unticked because all four are contract
questions and none is `dev-writer`'s to settle.

**Fixed, `spec-writer` — all four settled, three by one new requirement and one by
another. The markers stay in the code.**

The fourth thing is the one that mattered, and the handover is right that it is the gap
that let a high-severity defect through: **the spec stated no admissible range for a
recorded path, so the guard's bound was free to be the wrong one.** It now states one —
"A recorded path is one this module's derivation could have produced" — and states the
property that was actually missing rather than a number:

- The set of paths this capability can offer SHALL be bounded, and a value outside it
  refused rather than coerced, **on both the offering and the reading side**.
- **The bound SHALL be the same on both sides.** This is the requirement the defect
  needed and the one a literal would not have given. S1's own diagnosis is that the
  fix is not `u32::MAX` → `0x8000_0000`, because that leaves two rules that agree
  today; the contract now forbids the two-rule shape, not just today's wrong number.
  A future reader widening one has a requirement to violate.
- It is a bound on the **range**, not on the five paths a set offered — stated in the
  spec with the reason, which is the one the dev could not get from the contract:
  those five are not recoverable once the set is superseded, and the record outlives
  every set. Without that clause in the spec, "a path this build could have written"
  reads as a promise the store cannot keep.

**Markers 1 and 2 are the same decision and the spec now says it once.** A stored path
outside the range and a stored value that is not a Stoa address are both coerced-disk-
content questions, and the requirement covers both with the argument the markers make:
a clamped path derives a working identity that is not the user's and a padded address
names a different Stoa, and in both cases nothing says so. This matches how `design.md`
already treats them — "one principle, three applications" — and the spec was the one
place that principle was absent.

**Marker 3 (`path` in the replies) is settled in Finding 1 above**, in the direction of
requiring it. Same field, same silence, so it is answered once rather than twice.

**The markers stay in the code, all three, and that is a decision rather than an
oversight.** A `NO SPEC:` marker records that *the spec was silent when the choice was
made*, which stays true after the spec speaks; deleting them would erase the history of
a silence that produced a high-severity defect, which is exactly the record the next
reader of that guard wants. `design.md` already argues this for markers 1 and 2 and I
am agreeing rather than reversing it. What is now different is that the markers point at
requirements instead of at nothing.

**One thing in the handover that does not hold, for the record.** It says the first
marker's bound moved and "the test is renamed
(`a_stored_path_this_build_could_not_have_written_is_refused`)". Both tests exist:
`a_stored_path_outside_u32_is_refused_rather_than_clamped` (`identity_store.rs:830`,
carrying marker 1 and still asserting the `u32` cases) and the new one at `:873`
asserting the `PATH_LIMIT` boundary. So it is an addition, not a rename, and marker 1's
text is still accurate about its own test — which is why I have not touched it. Noted
because a reader going to check the renamed test would have found two and had to work
out which claim was wrong.

**For whoever counts them next.** `grep -rn "NO SPEC"` over `dialectica/rust-lib/` hits
seven files, and only three of the hits are this change's: `identity_store.rs:831`,
`identity_store.rs:1009` and `wire.rs:2124`. The rest belong to earlier changes
(`keystore.rs` four, `moderation.rs` four hits, `log/contract.rs` one) or are the
opposite of a marker — `arrival.rs:846`, `revision.rs:1858` and `moderation.rs:1844`
each say explicitly that something is **not** a `NO SPEC:` default, and
`moderation.rs:1429` is a marker that has been *retired* ("now specified"), which is the
shape a closed one leaves behind. `wire.rs` carries a
fourth hit at `:1724` that is `posting-capability`'s, not this change's. So the count
this finding gives is right for this change and a plain grep total is not, which is
worth knowing before quoting one.

**No unmarked gaps found.** I checked each test in the four modules for behaviour
no scenario describes. The closest was
`identity_store.rs:473`'s primary-key refusal of a second choice for one Stoa,
which the test attributes to "Keeping an identity does not replace an existing
one" — that requirement is about the keystore, but the store-level refusal is a
strictly stronger reading of the same rule rather than a new decision, and the
wire-level behaviour it produces is covered by
`a_second_keep_is_refused_and_leaves_the_stored_identity_unchanged`. Not
reporting it as a gap.

---

## Appendix — the eight mutations run

Every one applied, measured, and reverted. `git status --porcelain` empty after
each and at the end of the review. Implementation lines read only where the
mutation required it, named per row.

| # | Mutation | Line read | Result | Caught by |
|---|---|---|---|---|
| 1 | `keep_selection` validates the index then keeps candidate `0` regardless | `wire.rs:520-523` | 552 pass / **1 fail** | **only** `a_kept_identity_is_the_candidate_the_slate_offered_at_that_index` (new) |
| 2 | `generate_identity_slate` derives against a hardcoded Stoa instead of the parsed one | `wire.rs:315` | 552 / **1** | **only** `a_slate_is_offered_for_the_stoa_it_was_asked_about` (new) |
| 3 | keep discards the path record's write error (`let _ = …`) and reports `kept:true` | `wire.rs:538-542` | 551 / **2** | **only** `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity` and `each_keep_refusal_reason_is_pinned_to_its_own_situation` (both new) |
| 4 | `whoami_for` ignores the recorded path, deriving at a fixed `0` | `wire.rs:680-681` | 549 / **4** | `a_record_restored_beside_a_master_key_names_the_identities_in_use` (new) + 3 pre-existing |
| 5 | a refused out-of-range keep records a path anyway | `wire.rs:520-523` | 551 / **2** | `a_failed_keep_leaves_the_record_no_fuller_than_it_found_it` (new) + `a_selection_outside_the_set_is_refused_and_stores_nothing` |
| 6 | `KeystoreError::AlreadyExists` message stops naming its situation while staying distinct from the others | `keystore.rs:580` | 552 / **1** | **only** `each_keep_refusal_reason_is_pinned_to_its_own_situation` (new) |
| 7 | `recovery_needs_the_record: true` → `false` | `wire.rs:707` | 552 / **1** | `who_am_i_reports_that_recovery_needs_more_than_the_master_key` |
| 8 | path encoded as its low byte only; then little-endian | `identity.rs:428` | 550 / **3**; then 551 / **2** | `the_whole_path_reaches_derivation_and_not_only_its_low_byte` (new) in both. **`the_wire_constants_are_pinned_to_known_answers` survives the low-byte mutation** — which is precisely the gap the new test was written for, confirmed |
| — | `displayName` added to every slate candidate | `wire.rs:349` | **553 / 0 — SURVIVES** | nothing. See Finding 1 |

Mutations 1, 2, 3 and 6 were caught by the new tests **and by nothing else**,
which is the strongest available evidence that the seven are not redundant with
what was already there. Mutation 8's row is the one worth keeping: the
pre-existing pinned-constants test passes under a derivation that reads only the
path's low byte, because paths 0 and 1 differ in the last byte alone — two
explanations, one answer, this project's defect family, and the new test's third
pinned value at `0x01000000` closes it.

**Independent verification of the new pinned constant.** Not read back from the
code:

```
$ openssl kdf -keylen 32 -kdfopt digest:SHA512 \
    -kdfopt hexkey:07…07 \
    -kdfopt hexsalt:2f6469616c6563746963612f322f4964656e746974792f53746f61 \
    -kdfopt hexinfo:6b1f…9cd8 01000000 HKDF
F1:E3:2C:8F:46:01:CB:16:51:BE:57:D5:8E:39:F2:8C:C1:A6:E4:EF:8A:69:B9:BD:D2:15:5E:F9:53:A7:57:2B
```

matching `identity.rs:1290`'s hardcoded
`f1e32c8f4601cb1651be57d58e39f28cc1a6e4ef8a69b9bdd2155ef953a7572b`. The
invocation was validated first by reproducing the version-1 salt's value
`b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf` exactly. The
change's other four path-scheme constants (paths 0 and 1 under salt version 2)
also reproduce. tasks.md 1.2 and `2f3fccf`'s commit message are accurate on this
point — verified, not taken on trust.
