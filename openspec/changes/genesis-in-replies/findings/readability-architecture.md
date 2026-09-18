# Readability and architecture review — `genesis-in-replies`

Scope: **readability and architecture only**, per dispatch. Both dimensions
covered in this one file, as instructed. Correctness, security, spec-test and
design-review already ran (see the sibling findings files and `design.md`) and
are not repeated here except where load-bearing for a readability/architecture
judgment.

Reviewed: `dialectica/rust-lib/dialectica-core/src/wire.rs` (`GENESIS`,
`stoa_reply`, `create_stoa`, `join_stoa`, `membership_page_json`, and the new
tests around line 12760-12870), `dialectica-ui/src/qml/DStoaListScreen.qml`
(`genesisByStoa`, `rememberGenesis`, `reload`, `create`, `genesisFor`,
`canShare`), `design.md`, `proposal.md`, and the delta spec
`specs/stoa-membership/spec.md`. Working tree confirmed clean before and after
two targeted mutation checks (both reverted; see below).

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DStoaListScreen.qml:84-90`
      (`rememberGenesis`'s comment and the mechanism it credits)
      **Scenario:** the comment reads "Reassigned rather than mutated: a QML
      `var` property does not notify on an in-place key write... and a share
      button would stay hidden," which attributes the fix to the
      *reassignment* on line 89. But `var next = screen.genesisByStoa` binds
      `next` to the **same object reference** `screen.genesisByStoa` already
      holds (JS objects are reference types, and nothing here clones); `next[stoa]
      = genesis` therefore mutates that object in place regardless of the later
      reassignment, and `screen.genesisByStoa = next` reassigns the property to
      the identical object it already pointed at. What actually forces
      re-evaluation is the explicit `screen.genesisByStoaChanged()` call on the
      following line — unconditional, and present regardless of whether Qt
      would otherwise suppress a no-op (same-reference) property assignment.
      **Measured:** I mutated the function in this worktree (reverted
      immediately after each check; `git status --short` is clean) two ways
      and ran `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_stoa_screens.qml` after each:
      1. Removed the explicit `screen.genesisByStoaChanged()` call entirely,
         keeping the reassignment — **81/81 still pass.**
      2. Removed the reassignment entirely (`screen.genesisByStoa[stoa] =
         genesis; screen.genesisByStoaChanged()`, i.e. in-place mutation plus
         the explicit signal) — **81/81 still pass**, which is the same result
         `design.md` and `findings/design-review.md` already record as the
         accepted gap for the reassignment question alone.
      Both mutations pass because no test in this suite drives a
      `canShare`-bound `visible` (or any other binding) through a live reload —
      exactly the gap `design.md` names. What is new here is that the gap is
      not confined to "reassignment vs. in-place mutation": the explicit
      `genesisByStoaChanged()` call is equally unwitnessed, and the comment's
      account of *which* line does the work is not accurate to what the code
      depends on for correctness (the unconditional explicit emit, not the
      reassignment). This is a comment-accuracy finding, not a behavioural
      bug — the code is correct on the merits of the explicit signal call
      alone, which is unconditional and present in every path. A future editor
      reading the current comment and "simplifying" by dropping the
      reassignment (keeping the explicit signal, which is what the comment
      implies is decorative) would in fact be safe under a naive reading, but
      arrives at that safety for a reason the comment doesn't state, and a
      future editor who instead drops the *explicit signal* (reading the
      reassignment as sufficient, since that's what QML's own semantics could
      plausibly justify) would silently reintroduce the defect with nothing
      catching it. Recommend the comment credit the explicit
      `genesisByStoaChanged()` call as load-bearing on its own, independent of
      the reassignment, since that is what the measurement shows.
      **Severity:** low — no behavioural defect, and the coverage gap itself
      is already disclosed in `design.md`; this refines that disclosure rather
      than opening a new one.

## Readability — areas reviewed and clean

- **`design.md` reads coherently cold.** The four Decisions entries each state
  what was chosen, why, the alternative considered, and (for the two with
  a testable failure mode) what breaks without it. The refused codec-bug
  diagnosis is placed first and its generalisation ("a decoder reporting
  truncation is a claim about the bytes it was given, not about where they
  were stored") is stated plainly enough to be useful to someone who has not
  read the incident. The relation-not-literal decision explains *why* a pinned
  hex literal would prove less, not just that a relation was chosen.
- **The `map`→loop rewrite in `membership_page_json`** (`wire.rs:2969-2991`)
  carries a comment immediately above the fallible line explaining the
  consequence of getting it wrong ("an item silently short of its record is
  the owner's bug wearing a success reply"), and `design.md`'s entry adds the
  alternative considered (`Result<Vec<_>,_>` via `map`) and why it was
  rejected as "the more obscure of two correct options" — a readability
  judgment stated as one, not dressed up as a correctness one. This is
  unlikely to be "simplified" back to a `map` by a later editor without
  noticing why, because the loop's `return error_json(...)` is visually the
  same shape as every other early-return in this file.
- **The spec cross-reference change** (citing "A joined Stoa's genesis record
  is retained, not only its address" by name rather than "the requirement
  above") reads correctly and was verified against the live spec
  (`grep -n "^### Requirement" openspec/specs/stoa-membership/spec.md` lists
  it at line 188, spelled identically). `findings/spec-test.md` already
  covers this exact change and separately reasons that the *other* two
  above/below references inside this delta (in the "Listing reports the
  Stoas..." MODIFIED requirement) are safe because they sit inside a MODIFIED
  block whose position in the merged spec is fixed by the block it already
  occupies — confirmed independently by reading the same requirement and its
  surrounding structure; no new finding there.
- **Numbers stated as numbers rather than commands.** Spot-checked
  `tasks.md` (1052 Rust tests, 411 QML tests, 81/81, 80/81) and `design.md`
  against CLAUDE.md's "do not write down anything a command can answer" —
  these are all measurements taken and reported as evidence for a specific
  claim (a proof-of-failure count, a mutation result), not restatements of
  ambient state like a version number or "what's merged." This is the
  distinction CLAUDE.md draws and these numbers are on the right side of it.
- **`GENESIS`'s docstring and `stoa_reply`'s doc comment** are shared correctly
  between `create_stoa` and `join_stoa` — one function, one docstring, stating
  the reason is a property of neither call individually. No copy-paste risk:
  confirmed by reading both call sites (`wire.rs:2330`, `wire.rs:2818`), both
  pass through `stoa_reply` with no per-call special-casing.

## Architecture — areas reviewed and clean

- **Capability placement.** `stoa-membership`'s Purpose section and its
  existing requirements already own the reply shapes for `create_stoa`,
  `join_stoa`, and `list_stoas` — this delta only widens replies that
  capability already defines, and adds no new call. Confirmed by reading the
  capability header (`openspec/specs/stoa-membership/spec.md:1-10`) and the
  requirement list — every requirement this delta touches already lived
  there. A new capability would have split one reply shape's definition
  across two files for no boundary reason. Agree with
  `findings/design-review.md`'s judgment on this.
- **The verification boundary does not move.** `Membership::verified` remains
  the single place an address/record pair is checked
  (`wire.rs:2785`'s doc comment states this and the code matches: `create_stoa`
  calls it once at `wire.rs:2325` against the record it just built, and
  `join_stoa` reaches it once through `genesis_for` at `wire.rs:2813`, which
  is itself the feed path's decoder reused rather than reimplemented). Core
  handing back its own stored record through `stoa_reply` does not add a
  second verification path: every record that reaches a reply either came
  from a `Membership` already constructed through `verified` (creation, and
  the join that produced the stored membership) or was re-decoded from the
  store via `decode_row`, which re-derives the same relation on the way in
  (per `design.md`'s reachability paragraph). A record travelling core → view
  → back to core via `join_stoa` is re-verified on that return trip
  regardless of where it came from, so nothing about *who* is responsible for
  verification changes — it is core's, before and after this change, and the
  view remains a pass-through that neither checks nor needs to check the
  pair. This matches the Non-Goal stated in `design.md` ("No verification
  moved into the view").
- **The error-path shape for `membership_page_json`** (fail the whole page
  rather than skip the one unencodable row or emit an empty field) is the
  right call for a paginated reply here, weighed against its own stated cost.
  The alternative — silently omitting or defaulting the one row — is exactly
  the shape of the bug this piece exists to close (a success reply carrying
  an unusable record), so it is excluded structurally rather than by
  argument. The cost that remains (one bad row fails an entire page of
  otherwise-good rows) is real in principle but is against a branch
  `design.md` and the correctness review both establish is unreachable through
  either `create_stoa` or `list_stoas` today, since every record that reaches
  either encode site was already encoded once (to compute its address, or on
  the way out of `decode_row`). Given the branch is defensive rather than
  live, the module's own "one failure shape, never a partial success" contract
  is the right thing to hold onto rather than trade away for a lower blast
  radius on a path that cannot currently execute.
- **The defensive, currently-unreachable arm in `membership_page_json` /
  `stoa_reply` is correctly kept rather than deleted.** `canonical_bytes()` is
  fallible in its type regardless of whether every call site today happens to
  supply an already-valid record; deleting the arm would force an `unwrap()`
  at both use sites, which is precisely the reachable-panic shape CLAUDE.md
  and this review's brief both flag as a denial-of-service risk (module abort,
  20s caller timeout, `MODULE_NOT_LOADED` on every later call per
  PHASE0-FINDINGS §3) — and it would do so to remove a branch whose only cost
  today is a few lines, not to fix anything broken. The test that reaches it
  (`a_record_that_cannot_be_encoded_is_a_failure_rather_than_an_empty_field`,
  `wire.rs:12821-12870`) calls `stoa_reply` directly with a title built to
  exceed `MAX_CANONICAL_BYTES`, and it asserts its own premise first
  (`assert!(over_cap.canonical_bytes().is_err(), ...)`), so the test cannot
  silently become vacuous if the cap changes — it would fail loudly on its own
  premise rather than passing for the wrong reason. Both the code comment and
  `design.md` state plainly that the arm is defensive rather than reachable
  through the two public handlers today, so nobody reading only the test
  would mistake it for live coverage of a real path. Endorsed as the right
  call over removing the arm.
- **No new dependency was introduced.** Confirmed by reading the change — this
  is a widening of existing reply shapes using types already in scope
  (`hex`, `serde_json`) with no new crate.
