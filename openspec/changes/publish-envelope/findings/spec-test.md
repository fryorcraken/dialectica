# Spec-test findings — `publish-envelope`

Reviewed at `16af013`, in a worktree of my own. Dimension: **do the tests pin
what the change requires, and can they fail?**

**Where I departed from the blind-to-implementation discipline, and why.** This
piece is `skip_specs: true` with no delta, so the contract is
`openspec/specs/module-wire-contract/spec.md` and
`openspec/specs/content-authoring/spec.md` as merged. I read those, `proposal.md`,
`design.md`, `tasks.md` and the four sibling findings files as the brief directs.
I read implementation in four places, each named where it is used below:
`wire/request.rs`'s `Request::parse` (to mutate the size-check ordering and the
non-object arm), `wire.rs`'s `required_string` and `publishing_key` (to mutate
them), the dispatch trait declaration in `dialectica/rust-lib/src/lib.rs` (to
attack the sweep classifier), and — outside this repo — the `.lidl` generator's
Rust frontend, to settle a claim the design rests on. I did not read the three
publish handlers.

**Suite**: 737 + 26 = 763 green before any mutation, which matches `tasks.md` 8.1
and 8.5. The fresh worktree needed the gitignored
`dialectica/logos-rust-sdk-src` symlink; I linked it to the main checkout's
`/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src` target. Every
mutation below was confirmed to land (each produced a distinct compile-and-run),
and the tree was `git status --porcelain`-clean after restoring. Worktree deleted
on completion.

## Findings

- [x] **`spec-writer`** — the request size cap is in **no merged spec at all**,
      and this change extends an unspecified obligation to three more methods
      **Scenario:** `module-wire-contract` states the non-object rule, the
      three-distinct-messages rule and the three null readings, and says nothing
      about a size limit. I grepped `openspec/specs/` for `MAX_REQUEST_BYTES`,
      `size cap`, `byte limit` and `oversized`: the only hit anywhere in the
      merged spec set is `keystore/spec.md:193`, about keystore *file* content,
      not requests. The archived `wire-request-envelope` change's own spec delta
      (`openspec/changes/archive/2026-09-12-wire-request-envelope/specs/module-wire-contract/spec.md`)
      has no cap requirement either — so the cap has never been specified, only
      implemented. The test itself says so at `wire.rs:8414`: *"NO SPEC: the spec
      set says nothing about a size limit on a request — not that there is one,
      not that there is not ... the number is `dev-writer`'s choice pending the
      spec-writer: 4 MiB"*.
      **Why it is this change's to surface:** `proposal.md` lists the cap as one
      of three `module-wire-contract` obligations the publish handlers were
      outside, and `tasks.md` 2.1/3.3 count
      `every_request_taking_method_refuses_an_oversized_request` among the three
      sweeps that went red then green. Two of those three are genuinely the
      contract's; the third is a `dev-writer` default now pinned across fourteen
      methods with a 4 MiB constant hardcoded in
      `an_oversized_request_is_refused_before_it_is_parsed` — and the change's
      headline claim ("three envelope obligations") reads as three *contract*
      obligations. **Severity: medium.** The behaviour is almost certainly right;
      nobody decided it on purpose, and a growing sweep is the moment it becomes
      expensive to revisit. The requirement to write is the cap's *existence* and
      that the length check precedes the parse — the ordering is the security
      property, and it is the half a reader would not infer.
      **Closed — PROMOTED rather than softened, and the proposal corrected too.**
      `specs/module-wire-contract/spec.md` now `ADDED`s "A request is bounded, and
      the bound is checked before the request is parsed": a bound exists, it is
      **one** number for the surface rather than per method, it is checked
      **before** the parse, and it is bracketed — above the largest op
      `op-format` permits, materially below what costs the module its process.
      The spec deliberately does **not** state 4 MiB; per CLAUDE.md a number in a
      spec is a claim no gate reads. The reason for promoting rather than
      correcting-only: the property that matters is the ordering, a bound checked
      after the parse bounds nothing, and what failing to pay costs is the process
      rather than the call — which is exactly what a behaviour contract is for.
      Leaving it in a comment leaves the next implementation free to reorder with
      every test green. `proposal.md` is corrected as well and now says two of the
      three were the contract's, so the headline no longer overcounts.
      `skip_specs: true` is removed from `.openspec.yaml`, which keeps its old
      reasoning alongside what it missed. `wire.rs:8414`'s `NO SPEC:` marker is
      replaced by a pointer to the new requirement, and `wire.rs:5710`'s marker —
      which cited the cap as "the same absent decision, one layer in" — is
      corrected, since that half stopped being true.

- [x] **`spec-writer`** — the defaulted-method escape in the sweep classifier
      rests on an **upstream** behaviour that no requirement records and no gate
      in this repo pins
      **Scenario:** `findings/architecture.md`'s open `design-reviewer` box
      measured that a defaulted trait method with a semicolon-free body leaves the
      sweep green and unswept. **I reproduced it**: I added
      `fn publish_moderation(&mut self, request: String) -> String { request }` to
      the dispatch trait and
      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      **passed**. The defence is `lib.rs:287-288`'s comment that the generator
      skips defaulted methods.
      **Measured — and the claim is TRUE, which is why this is a `spec-writer` box
      and not a `dev-writer` one.** I read the generator rather than trusting the
      comment: `lidl-gen/src/rust_frontend.rs:350-354` in the SDK source is
      `// Default-bodied methods (framework hooks, helpers) are not part of the
      IPC contract.` / `if f.default.is_some() { continue; }`, and its module doc
      at lines 10-13 says the same. So a defaulted method genuinely never reaches
      the wire and the sweep owes it nothing.
      **What is missing is that this is load-bearing and unrecorded.** "A method
      with a default body is not on the module's wire surface" is now a premise of
      this repo's only anti-staleness gate, and it lives in one code comment and
      one flake input's source. `logos-rust-sdk` is a pinned input; a version bump
      that started emitting defaulted methods would put a request-taking method on
      the wire with the sweep green and every gate in this PR passing.
      `module-wire-contract` is the spec that defines what "the surface" is, and it
      does not say this. **Severity: medium.** Write it as a requirement (or a
      stated assumption with its citation) so the premise is reviewable when the
      SDK moves. The `design-reviewer`'s open box covers whether `design.md`
      should say it; this box is that the *contract* does not.
      **Closed — recorded in `module-wire-contract` as a stated assumption with
      its citation, not as a bare claim.** The `MODIFIED` "Every method takes JSON
      and returns JSON" now says what *the surface* is: it SHALL be derivable from
      one declaration, no method SHALL be on the wire without appearing there, and
      the generator's rule for excluding plumbing is that a method with a
      **default body** is not emitted — quoted from `lidl-gen`'s Rust frontend
      module doc, at the revision `dialectica/flake.nix` pins
      (`logos-module-builder` at `9f420c2`, which supplies both the generator and
      the SDK source). Two scenarios carry it: one requiring an omission from the
      derived check to be reported naming the method, and one stating the
      defaulted-method exclusion with its citation. Written as "we depend on
      upstream behaviour X, verified at this revision" rather than "X is true"
      precisely so a pin bump makes it visibly re-checkable — CLAUDE.md's
      self-invalidating rule. I re-read the generator myself rather than relying
      on the citation: `lidl-gen/src/rust_frontend.rs:350-354` is
      `// Default-bodied methods (framework hooks, helpers) are not part of the
      IPC contract.` / `if f.default.is_some() { continue; }`, and lines 10-13 of
      its module doc say the same. The reviewer's reading is accurate.
      `design.md` saying it is still the `design-reviewer`'s open box; this one
      was the contract's silence and is closed.

- [ ] **`tester`** — `every_request_taking_method_refuses_an_oversized_request`
      cannot distinguish a cap checked before the parse from one checked after,
      and its comment claims the property it cannot observe
      **Scenario:** the test's own doc (`wire.rs:8421-8425`) argues the cap exists
      because *"a 64 MiB request padded with one ignored field was ACCEPTED and
      served, at 92.6 ms and ~2N transient heap"* — an allocation claim, which is
      only true if the length check runs first. Its fixture is
      `format!(r#"{{"junk":"{}"}}"#, "x".repeat(MAX_REQUEST_BYTES))`, which is
      **valid JSON**. Both orderings return the size refusal for valid JSON, so
      the test is satisfied by an implementation that parses 4 MiB first and then
      complains about the size — exactly the "fixture where two explanations give
      the same answer" family this repo has recorded eight variants of.
      **Measured:** I moved the `request.len() > MAX_REQUEST_BYTES` check to
      *after* `serde_json::from_str` in `wire/request.rs`. The suite went to 735
      passed / 2 failed:
      `an_oversized_request_is_refused_before_it_is_parsed` and
      `the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does`
      both went red naming `invalid JSON`. **This sweep stayed green.**
      **Severity: low, and it is a comment fix rather than a test fix.** The
      property *is* pinned — twice, by the two tests that use an oversized *and
      unparseable* fixture — so there is no coverage gap, and I say so in prose
      below. What must change is the sweep's doc: it currently reads as the test
      that catches the allocation defect, and a future author trimming tests would
      read it as covering the ordering. One sentence pointing at
      `an_oversized_request_is_refused_before_it_is_parsed` as the test that owns
      the ordering, and saying this one owns only "every method is bounded", makes
      the division honest. (Unticked because the `unfixed-test-patterns-get-copied`
      note applies: this is the doc the fifteenth method's author will read.)

- [ ] **`tester`** — `tasks.md` 8.6 is the only row standing between a typo and
      the wire, and the piece's last commit widened what it has to catch
      **Scenario:** `tasks.md` 8.6 is unticked deliberately and correctly, and it
      names the risk: 7.6 deleted `publishing`'s `method` parameter and changed its
      three call sites in `dialectica/rust-lib/src/lib.rs`, which `cargo test` does
      not compile. I confirmed the file is behind `cfg(logos_scaffold)` and that
      the local suite compiles none of it. So the change's last commit edited an
      adapter signature and three call sites with **no local gate able to see a
      typo**, and the CI adapter gate I ran (below) is a text gate that checks for
      the presence of four strings and the absence of two shapes — it would pass a
      file that does not compile.
      **Measured:** I ran the gate's own Python against three trees. Against
      `16af013` it passes. Against `35fc859` it fails on `MISSING WANT:
      core::stoa_of` and `BANNED PARSE: 1 call(s) to serde_json::from_str`.
      Against `origin/main` it fails on four counts including `BANNED ACCESSOR:
      stoa_key`. Every claim `design.md` decisions 5 and 7 and `tasks.md` 6.3 make
      about the gate is true, verified by execution rather than read.
      **Severity: low as a defect, and it is an instruction rather than a fix.**
      There is nothing to change in the tests — the instrument genuinely does not
      exist locally. What is needed is that whoever closes this piece **watches
      Build LGX specifically** rather than reading "CI green", because it is the
      only gate that compiles the edited file, and a green `cargo test` on this PR
      says nothing about it. Flagged as a `tester` box so it is not lost when
      `findings/` is deleted at merge.

## My judgement on the CI text gate as an instrument

**It is adequate for the property it is pointed at, and it is not adequate as
the only thing there — which is what the change already concluded and built
for.** The property is "the adapter's first touch of the request bytes goes
through `Request::parse`". That is a statement about the *order of operations
inside an uncompiled file*, and a text gate cannot express order at all: it can
see that `core::stoa_of` appears somewhere and that `serde_json::from_str`
appears nowhere. An adapter that called `core::stoa_of` *after* doing something
expensive with the raw bytes would pass it.

What makes the arrangement hold anyway is that the gate is not carrying the
ordering claim alone. The **ban** is the strong half, and it is strong for a
structural reason rather than a rhetorical one: with `serde_json::from_str`
banned outright, there is no way for the adapter to obtain a parsed request
except through a `core` entry point, and every such entry point is built on
`Request::parse`, whose ordering *is* pinned by a test that runs
(`an_oversized_request_is_refused_before_it_is_parsed`, which I confirmed goes
red when the check moves). So the ordering property is enforced by composition —
text gate closes the shape, unit test closes the ordering — rather than by the
text gate asserting something it cannot see. That is the right construction, and
`design.md` decision 7's "three things hold it in place" is an accurate
description of it.

**What it still cannot catch**, stated plainly because the change's own honesty
on this is the part worth preserving: a second expensive operation added *before*
the `core::stoa_of` call (the 64 MiB Argon2id unlock is exactly that, and is
deferred by decision 8); any parse spelled differently (`serde_json::Value::from`,
`from_slice`, `str::parse::<Value>()`, a `serde` derive) — the regex is
`serde_json\s*::\s*from_str` and closes one spelling of the shape, not the shape;
and whether the file compiles at all. The first is recorded and deferred with an
argument. The second is worth knowing about but is not this piece's to fix. The
third is 8.6.

## What I verified and found clean

**Every envelope obligation in `module-wire-contract` is pinned for the three
publish handlers, and pinned against meaning rather than distinctness.** I walked
the requirement scenario by scenario:

- *"A request that is an array is refused"* / *"a scalar is refused"* /
  *"Every field-reading method on the surface refuses a non-object"* —
  `a_request_that_is_not_an_object_is_refused_for_its_shape` sweeps all fourteen
  methods against six non-object values and asserts `error_message(&out) ==
  REQUEST_NOT_AN_OBJECT`, the literal constant. It explicitly does **not** assert
  `error.is_some()`, and its comment says why. This is the correction for the
  `pinned-literal-vs-distinguishable` family and it is done right.
- *"The non-object refusal is not the missing-field refusal"* / *"not the
  unparseable-request refusal"* — `the_three_refusals_a_caller_can_earn_are_three_different_messages`
  pins each of the three to what it must **say** before making the three pairwise
  `assert_ne!`s. The reworded-to-misinform evasion that defeated an earlier
  version of this shape does not work here.
- *"An empty object is refused for its missing field, not its shape"* —
  `an_empty_object_is_refused_for_its_missing_field_and_never_for_its_shape`, and
  it covers the no-required-field case separately so excluding `list_stoas` from
  the loop does not exclude it from the claim.
- *"One field has one null reading"* — `one_field_has_one_null_reading` now
  carries `body`, `parent` and `direction`.
- *"The one method outside this rule is the panic probe"* —
  `panic_probe_still_panics_on_a_non_object_rather_than_refusing_it`, and the
  sweep's `OUTSIDE_THE_ENVELOPE_RULE` names it with the spec's own words.

**`content-authoring`'s "The signing identity is the one the probe reports" is
pinned by a test that cannot agree with itself.**
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports` publishes
through `publish_post`, reads the op back out of the log, and compares its
author against the probe's `identity` field — rather than calling
`publishing_key` twice, which its own comment says would agree with itself
whatever it returned. It then asserts the *pathless* key disagrees with the
probe, which is what makes it able to tell the fix from the defect.
**Measured:** reverting `publishing_key` to `keystore.stoa_key(stoa)` turns it
red naming `656c6003a040…` against probe `b3c5502267a7…`. The mutation landed
and was caught. `a_publish_is_refused_when_no_identity_has_been_chosen_for_the_stoa`
covers the other half, so the fix cannot be satisfied by signing with something.

**The author's "the null tests passed on the unfixed code" claim is honest AND
the tests are not vacuous.** I verified the second half by execution rather than
by reading the design note: adding
`Some(serde_json::Value::Null) => Err(error_json(&format!("missing field: {field}")))`
to `required_string` turned **two** tests red —
`one_field_has_one_null_reading` naming `body: a null was reported as missing`,
and `a_wrong_typed_direction_is_refused_by_its_type_with_a_valid_stoa_and_target`.
So reading 3 is a property the code could fail and these tests would catch. The
`design.md` "What this change did NOT find" section describes them accurately as
coverage rather than regression.

**The non-object fix is pinned by eight tests.** Mutating `Request::parse`'s
`_ => Err(error_json(REQUEST_NOT_AN_OBJECT))` arm to
`_ => Ok(Request(serde_json::Map::new()))` — the original defect, reconstructed —
turned 8 of 737 red, including
`the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does`
reporting exactly the `missing field: stoa` vs `the request must be a JSON object`
pair the correctness findings file quotes. That file's account of the defect is
accurate.

**I could not defeat the sweep classifier in any way the design does not already
own.** I attacked it four ways beyond the three review has already measured:

| Probe added to the dispatch trait | Result |
|---|---|
| `request: std::string::String` (same type, same wire surface, fully qualified) | **caught**, loudly: `["publish_moderation (parameters \`request: std::string::String\`)"]` |
| `/* block comment */` doc rather than `///` | **caught**: reported missing from the sweep |
| `-> String where Self: Sized;` | **caught**: lands in the returns bucket |
| two request-taking methods at once | **caught**: both named, `["publish_flag", "publish_moderation"]` |
| defaulted body with no `;` (the architecture reviewer's) | **evaded** — filed above as a `spec-writer` box, because the `.lidl` claim that makes it safe is true but unrecorded |

The one shape that evades it is the one already filed, and I verified upstream
that it is behaviourally safe. Everything else errs toward red, which is the
property `design.md` decision 6 claims and — for the two of three preconditions
that are enforced by the bucket rather than by a `continue` — has.

**Fixtures are genuine.** `a_served_request`'s three publish entries are real
served requests (`publish_reply` and `publish_vote` name an op
`a_seeded_root_id` puts in the log), and `a_request_within_the_cap_is_still_served`
is the test that stops a cap of zero satisfying the oversized sweep. So the
sweeps are not vacuously green.

**The spec is self-consistent on this piece's subject and is not stale.**
`openspec validate --all --strict` passes 17/17. Reading
`module-wire-contract` in full, its three-cases scoping, its "exactly one method
in that third case" claim and its three null readings do not contradict each
other, and `content-authoring`'s signing-identity scenario does not contradict
"The author is derived from the Stoa, never supplied" — the second constrains
where the identity comes from, the first which derivation. I checked `docs/PLAN.md`
on `origin/main` (`468e716`) rather than the branch copy: §9.2's "The publish
prologue/tail wants reshaping into one place" is the paragraph this piece
addresses, and the branch strikes it through as **Done** while writing out the
unlock half as outstanding with the `{"stoa":"…","author":"x"}` case — which is
the right shape (strikethrough plus what now holds), and discharges question 6.
I found no place where PLAN.md still states as future intent something these two
specs now specify.

**No requirements moved between capabilities** in this change — there is no delta,
and I confirmed by diffing `733544d..piece/publish-envelope` that
`openspec/specs/` is untouched. So the `REMOVED`-here/`ADDED`-there check has
nothing to catch.

**No new `NO SPEC:` markers.** I diffed `wire.rs` against the merge base for
added lines matching `NO SPEC` and found none. The thirteen in the file are
pre-existing; the size-cap one at 8414 is the one this change made load-bearing
for three more methods, which is the first box above.

**The branch does not silently revert `origin/main`.** A two-dot diff against
`origin/main` looks alarming — it shows large reversals in `docs/PLAN.md` — but
that is the three-commit gap (`468e716`, `53f08b3`), not a revert: against the
true merge base `733544d` the piece changes PLAN.md by +24/−15. `git merge-tree
733544d piece/publish-envelope origin/main` produces no conflict markers, and
`origin/main` adds no new method to the dispatch trait, so the sweep's expected
set is unchanged post-merge. Checked because `stale-branch-silent-revert` is a
recorded trap here.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is behind `cfg(logos_scaffold)` and is compiled
by nothing I can run, so I could not execute the adapter or confirm that the
signature change in 7.6 compiles. That is 8.6's row and the fourth box above.
I did not exercise the module against a live basecamp host, and I did not run
Build LGX. I ran the adapter-derivation gate's Python and the test-count gate's
logic locally; I did not run the remaining CI jobs.
