# Readability review — `get-stoa`

Dimension covered: **readability only** (architecture, correctness and
security are separate reviewer passes; not duplicated here).

Scope read: `dialectica/rust-lib/dialectica-core/src/stoa_metadata.rs` (new),
the `wire.rs` diff (`get_stoa`, `stoa_metadata_json`, and their tests), the
`identity.rs`, `moderation.rs`, `lib.rs` (core) and `rust-lib/src/lib.rs`
diffs, `proposal.md`, `design.md`, and the four spec diffs under
`openspec/changes/get-stoa/specs/`.

## Overall impression

This is unusually easy to review. `stoa_metadata.rs`'s module doc states
exactly what a reader needs before touching the resolver ("the moderation
resolver with a different subject", "nothing here orders anything"), and every
non-obvious choice in the code has a doc comment that says *why* rather than
restating the line beneath it — e.g. the kind-before-authority ordering in
`binding_metadata`, the enum-not-struct shape of `CurrentMetadata` in decision
5, and the "verified before the store is opened" ordering in `get_stoa`. Names
read as intended: `Founding`, `CurrentMetadata::Fallback`/`Declared`,
`binding_metadata`, `resolve`. Tests are grouped under `// ─── section ───`
banners that match the spec's own scenario groupings, and fixture builders
(`a_rename`, `a_forged_rename`, `a_log_of`) are one job each with a doc
explaining what the fixture proves, not just what it builds.

`wire.rs`'s new section (`get_stoa`, `stoa_metadata_json`) follows the
existing file's conventions exactly — same `guarded` wrapper, same
`parse_stoa`/`genesis_for` reuse `listThreads`/`readThread` already use, same
one-function-builds-the-reply shape as `stoa_reply`. Nothing here fights the
established pattern.

`moderation.rs`'s change is a single visibility widening
(`fn authorises` → `pub(crate) fn authorises`) with a doc addition explaining
why a second caller arrived; no logic duplicated, no re-spelled conjunction —
exactly what the module doc promised a future resolver would do.

`proposal.md`, `design.md` and the spec diffs are dense but are prose
documents whose job is to carry reasoning; each decision in `design.md` is
independently checkable ("what breaks without it: `test_name` — measured").
I did not find loose or misleading prose there.

## Findings

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:15474` (and `:15589`), `dialectica/rust-lib/dialectica-core/src/stoa_metadata.rs:783` — a third near-identical hand-rolled `OpLog` stub is added by this change, on top of the two the file already had.
      **Scenario:** `wire.rs` already carries `BrokenLog` (~6850) and `PanickingLog` (~6914) for `list_threads`'s failure tests, each a ~30-line `impl OpLog` where five of six methods return the same fixed error or the same `panic!(...)`. This change adds a fourth and fifth such impl to the same file — `UnreadableOpLog` (:15474) and `PanickingOpLog` (:15589) — and a sixth, `UnreadableLog`, plus a seventh shape (`UnscopedLog`, whose only override is `iter_stoa`) in the new `stoa_metadata.rs`. None of the five "always-fail" stubs differ from each other except the struct name and the error string; nothing shares them. This is not a new pattern introduced by this change — the two pre-existing stubs establish it — but the change doubles the count of near-duplicate always-fail `OpLog` fakes in `wire.rs` alone (2 → 4) and adds a fifth in `stoa_metadata.rs`, with no shared `AlwaysFailingOpLog` or `PanickingOpLog` test helper anywhere in the crate to reach for instead. A future failure-path test for a sixth wire method will most likely paste an eighth copy rather than notice one exists to reuse, per the "unfixed test patterns get copied" pattern this codebase has already hit once.
      **Not a blocker for this change** — it follows existing local convention exactly and every copy is correct — but it is the kind of finding that compounds, and `stoa_metadata.rs`'s copy is the third `OpLog`-stub struct in the crate whose entire body is "return the same `Err`/`panic!` from every method." Worth a follow-up to extract one shared always-fail/always-panic `OpLog` test double (e.g. in a `test_support` module) that every module's failure tests construct with an error string or panic message, rather than a fix inside this change.
      **Severity:** minor / maintainability. No behavioural risk — each copy is independently correct and independently tested by its own scenario.
      **Outcome: deferred.** Agree with the finding's own recommendation not to fix this inside `get-stoa` — it names its own remedy as "a follow-up... rather than a fix inside this change." Read all five always-fail/always-panic stubs (`wire.rs` `BrokenLog`/`PanickingLog`/`UnreadableOpLog`/`PanickingOpLog`, `stoa_metadata.rs` `UnreadableLog`) plus `stoa_metadata.rs`'s `UnscopedLog`, which architecture.md already separated out as not interchangeable (it stubs `iter_stoa` only, to isolate the scope check, not storage failure). Consolidating the always-fail/always-panic shapes would mean a shared test-support type reachable from both `wire.rs` and `stoa_metadata.rs`, which is a cross-module test-infrastructure change with its own review surface (module placement, whether `wire.rs`'s two pre-existing stubs get touched too) — exactly the kind of change CLAUDE.md's "do not refactor speculatively" warns against taking on inside an unrelated feature's diff. Deferred to a follow-up piece that extracts one shared `AlwaysFailingOpLog`/`AlwaysPanickingOpLog` test double for `dialectica-core`'s test modules to reach for; no action taken on the test files of this change.

## Areas checked and clean

- **Naming.** No `handle`/`process`/vague-verb functions; `resolve`,
  `binding_metadata`, `get_stoa`, `stoa_metadata_json` each do one legible
  thing the name states.
- **Comments earning their place.** Every doc comment I checked answers "why
  this and not the alternative" rather than restating the code — including the
  ones that look like they might be filler (e.g. `Founding`'s "one record, so
  the two halves cannot come from two Stoas", `CurrentMetadata`'s "an enum, so
  the flag cannot disagree with the values"). I did not find a comment that
  merely repeats its line.
- **One function, one job.** `binding_metadata` is explicitly carved out so
  the kind filter and the authority check live in exactly one place;
  `stoa_metadata_json` is the reply builder and nothing else; `get_stoa` reads
  as five sequential steps (parse → parse_stoa → genesis_for → Founding::of →
  store → resolve), each a one-line match with an early return, no branch
  doing two things.
- **Consistency with the rest of the file.** `get_stoa` does not invent a new
  handler shape; it is built from the same primitives (`guarded`, `Request::parse`,
  `parse_stoa`, `genesis_for`, `error_json`) every neighbouring handler uses, so a
  reader who already knows `list_threads` or `read_thread` can read this one
  at a skim.
- **Test naming and structure.** Test names in both files state the scenario
  as a sentence (`a_binding_op_carrying_the_founding_values_is_not_a_fallback`,
  `a_request_that_fails_verification_never_opens_the_store`), and each maps
  onto one spec scenario, which makes the spec-to-test correspondence
  legible without needing to cross-reference by line number.

## Not reviewed here (other dimensions' lanes)

Correctness of the resolver logic, security implications of trusting
peer-supplied ops, and whether the architecture/module boundaries are the
right ones are left to the other three `code-reviewer` passes.
