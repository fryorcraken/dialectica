# Readability review — `machine-identity-scope`

Scope: names, comments that argue from premises the code disproves, stale
comments describing the per-Stoa model this change retires (Rust and QML),
functions doing two jobs, and numbers in comments nobody measured. This is the
only dimension covered by this pass.

Method: read the full three-dot diff against `041eaff` file by file
(`dialectica/rust-lib/src/lib.rs`, `dialectica-core/src/wire.rs`,
`examples/seed_store.rs`, `tests/end_to_end.rs`, `.github/workflows/ci.yml`,
every touched `.qml`/`.qml` test file, `design.md`, `proposal.md`, `tasks.md`),
then swept the untouched files these changes reference by name
(`keystore.rs`'s `stoa_key`/`identity_key` docstrings, `DStoaListScreen.qml`)
to check whether their claims still hold after the rest of the piece landed,
and grepped for dangling references to every identifier this piece deletes
(`createIdentityFor`, `closeOnboarding`, `identityWasKept`, `root.onboarding`,
`NO_CHOICE_FOR_THIS_STOA`, "no identity has been chosen for this Stoa").
`cargo test -p dialectica-core --lib wire::` runs green (272 passed) as a
baseline; no mutation was needed for a readability-only pass and none is left
in the tree.

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/Main.qml:121` — a comment still
      names the deleted function `createIdentityFor` as if it exists, and its
      own twin comment elsewhere in this piece's diff was updated to drop the
      name while this one was not.
      **Scenario:** the block at lines 110–124 explains why `enterOnly`
      replaced hand-written per-transition clears, citing `openThread` and
      `` `createIdentityFor` `` as the two functions that had been written with
      only three of the four clears "in the first place." `createIdentityFor`
      no longer exists anywhere in `Main.qml` — this piece renamed it to
      `acquireIdentity()` and changed what it does (D9). The *identical*
      historical sentence appears in `dialectica-ui/tests/tst_navigation.qml`
      (near `test_the_identity_route_from_moderation_clears_the_moderation_state`),
      and there this piece's own diff changed it from
      `` them (`openThread`, `createIdentityFor`) had been written naming only ``
      to `` them (`openThread`, and the identity route) had been written naming only ``.
      So the sweep that fixed the test file's copy of this sentence missed the
      source file's copy of the same sentence. A reader who greps `Main.qml`
      for `createIdentityFor` to understand this comment finds nothing, and a
      reader who trusts the comment believes a deleted function is live. This
      is not the historical reference at line 213 ("This used to be
      `createIdentityFor(stoa, …)`"), which correctly marks itself as past
      tense and names what replaced it — line 121 does neither.
      **Measured:** confirmed via `git grep -n -F "createIdentityFor"` against
      `dialectica-ui/src/qml/Main.qml` (two hits: 121, 213) and against
      `dialectica-ui/tests/tst_navigation.qml` (zero hits — its copy was
      already fixed), and against the diff hunk that made that fix.

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:550-554` — a comment edited
      in place to add the new (envelope-check) reason for keeping
      `core::stoa_of` produces a run-on sentence whose pronoun no longer has a
      clear antecedent.
      **Scenario:** the sentence reads: "The adapter reads `stoa` before it
      opens anything -- it once needed the value to derive a per-Stoa key, and
      since machine-identity-scope needs only the refusal of a malformed
      request ahead of the keystore -- and it was doing that with its own bare
      `serde_json::from_str` plus a four-arm ladder -- which shadowed the whole
      request envelope on the shipped path[...]". "it was doing that" is meant
      to refer back to "reads `stoa`" (the pre-fix adapter read it by hand
      rather than through `core::stoa_of`), which is the antecedent in the
      pre-piece version of this same paragraph
      ("The adapter must read `stoa` before it can derive a per-Stoa key, and
      it was doing that with its own bare `serde_json::from_str`..."). Splicing
      in the second reason ("and since machine-identity-scope needs only the
      refusal...") between the setup and "it was doing that" leaves "that"
      reading as if it refers to "needs only the refusal of a malformed
      request," which does not parse against "with its own bare
      `serde_json::from_str`". This is a comment inside a CI gate step whose
      whole documented purpose (the surrounding paragraph says so explicitly)
      is to be read by whoever the gate turns red for — the exact reader this
      sentence now costs a re-read.
      **Measured:** read in place at `.github/workflows/ci.yml` lines 543-560;
      confirmed against the diff that the pre-piece sentence had a single clear
      antecedent and the post-piece insertion is what breaks it.

## What was clean

The two largest files in this diff — `dialectica/rust-lib/dialectica-core/src/wire.rs`
(nearly 2000 diff lines) and `dialectica/rust-lib/src/lib.rs` — are unusually
thorough: every removed per-Stoa-derivation doc comment is either deleted or
explicitly rewritten under a "# History" / "In this release" heading that says
what changed and why, every renamed test explains what its old name claimed
and why the new name is accurate, and every function whose signature narrowed
(`posting_identity`, `publishing_key`, `whoami_for`, `who_am_i`,
`get_capabilities_from_stores`) carries a doc comment stating what parameter is
gone and why "gone rather than ignored" was the chosen shape (`design.md` D3).
I did not find a single case in these two files where a comment's premise is
contradicted by the code beside it.

`examples/seed_store.rs` and `dialectica-core/tests/end_to_end.rs` both invert
comments and assertions that were pinning the *old* three-derivations gap
(self-invalidating by design), and both do so correctly — the renamed test
`the_per_stoa_derivation_still_yields_two_authors_for_the_store_to_persist` is
a good example of a name fixed because its old name read as a claim about
"who posts" that this release no longer supports (flagged in `design.md`
Risks and resolved by the tester's pass, commit `e4b7813`).

`DIdentityChip.qml`, `FeedScreen.qml`, `qmldir` and the QML test files
(`tst_identity_chip.qml`, `tst_navigation.qml`, `tst_thread_navigation.qml`)
are consistent with the new model; I found no stale per-Stoa language in any
of them.

`design.md` D9 discloses two stale comments this piece deliberately leaves
alone in `DStoaListScreen.qml` (~line 633: "the only thing that writes one is
per-Stoa onboarding"; ~line 662: "`DOnboardingScreen` is where a per-Stoa
identity is chosen"), naming issue #150 as the piece that owns them. I checked
both lines still exist as described and are still stale under this change's
own model, but since this piece explicitly identifies them and defers the fix
by name, I am not opening a duplicate finding for them here — that disclosure
already does the job a checkbox would.

I checked `keystore.rs`'s `stoa_key` and `identity_key` docstrings (referenced
by a comment `seed_store.rs` used to carry, since rewritten): both docstrings
claim things that were false before this piece (the adapter called `stoa_key`
directly) and are true again now that `wire::publishing_key` resolves through
`identity_key`. No fix needed there — this piece's own effect made them
accurate rather than leaving them stale.

No functions-doing-two-jobs findings: `keep_selection`'s `(encrypted,
wrote_the_keystore)` tuple return and `undo_a_keystore_this_keep_wrote`'s
narrow, single-purpose shape are both explicitly justified in comments and in
`design.md` D8, and each reads as one job (settle a fact about this call,
undo only what this call wrote) rather than two unrelated ones.

No unmeasured-number findings: the one numeric claim this piece removes
(`SEEDED_PATH`'s "measured over 2000 nonces: never" comment in
`seed_store.rs`) is deleted along with the constant it justified, not left
behind attached to different code. `design.md`'s own "measured" claims (D8,
Risks) are dated and attributed to the tester's pass with the actual
mutation and observed output quoted, which is the standard this project asks
for rather than a number nobody checked.
