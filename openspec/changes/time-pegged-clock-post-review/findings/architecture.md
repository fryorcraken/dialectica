# Architecture findings — `time-pegged-clock-post-review`

Reviewed **only** the architecture dimension, against both diffs the piece
names:

1. This change's own diff, `git diff origin/main...HEAD` (three dots).
2. The four #165 commits that merged without review,
   `git diff 2eada33 c1f1a8f` (`d6208fb`, `ae0c30d`, `c34ec46`, `c1f1a8f`).

I read issue #162's owner decision comment
(`https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926`)
first; its first line is:

> **Decision (owner, 2026-09-25): peg the Lamport counter to wall-clock time,
> as SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155
> and 184-192).**

## Findings

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/arrival.rs:199`,
      `op.rs:67`, `op.rs:568`, `transport.rs:477` (diff 1) — the four repointed
      doc comments cite the archived design by its **full folder path**
      (`` `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`,
      Decision 11 ``), which is a citation style found nowhere else in
      `dialectica-core`. Every other archived-design citation in this crate
      names the **change**, not the path: `` the `op-ordering` change's
      `design.md` `` (`transport.rs:60`, unchanged by either diff),
      `` that change's `design.md` `` (`identity_store.rs:5`, `keystore.rs:5`,
      `onboarding.rs:4`, `wire.rs:139`), `` the `stoa-metadata-op` change's
      `design.md` `` (`op.rs:103`), `` the `get-stoa` change's `design.md` ``
      (`wire.rs:2326`), and dozens of bare `` `design.md` decision N `` cites
      elsewhere in the same files these four sit in (`arrival.rs:228`,
      `authoring.rs:264` — the latter touched by diff 2's own four commits and
      left in the old bare style). This piece's own `design.md`, Decision 1,
      argues *why* the archived design is cited rather than extended or
      repeated — a good argument — but never addresses *how* to cite it, so
      the four rewrites landed a fifth citation shape into a crate that
      already had a working one. **Scenario:** a reader who has learned the
      crate's convention ("look for `` `<name>` change's `design.md` ``, then
      grep the archive by that name") greps for `openspec/changes/archive/` in
      three of these four comments and finds it — but the fourth citation
      style is the one place the crate spells out a literal path, which is
      brittle if the archive is ever reorganised (`OPENSPEC-ARCHIVE.md` gives
      no promise the folder name is stable beyond "moved, not deleted") and
      inconsistent with the crate's usual "name the change" idiom everywhere
      else. **Not a behaviour defect** — the four citations resolve correctly
      today (verified: Decision 11 in the archived design.md is exactly the
      "receiver whose own clock is wrong" analysis these four comments point
      at, and Decision 1 is exactly the "why pegged, rejected alternatives"
      analysis `transport.rs:477` also points at). This is a consistency/style
      finding, not a correctness one: either reword the four to
      `` the archived `time-pegged-clock` change's `design.md`, Decision 11 ``
      to match the rest of the crate, or record in `design.md` why this one
      case gets the fuller form (e.g. because the passage being repointed used
      to name a *spec*, `op-ordering`, which is a live, moving document, so
      the replacement deliberately over-specifies to avoid the same staleness
      recurring) — right now neither is written down.
      **Outcome (`dev-writer`): fixed** in the commit that ticks this box. I
      took the first option. All four now read "the archived
      `time-pegged-clock` change's `design.md`", followed by the same Decision
      numbers as before. `git grep -n -F -e "2026-09-25-time-pegged-clock" --
      dialectica/rust-lib` returns nothing. I found no reason for the fuller
      form worth keeping. The staleness the old pointer suffered came from
      citing a live spec whose text was then removed. A change name picks out
      an archived `design.md` either way, and the date-prefixed path adds only
      a string that breaks if the folder is renamed. `design.md`, Risks, first
      bullet, now says the four cite by change name to match the crate, and
      names three existing citations of that form. Doc comments only. No test
      can see a doc comment, so there is no failing test to name.

## What I checked and found clean

- **No duplicated reasoning.** This piece's `design.md`, Decision 1, chose to
  *cite* the archived design's Decision numbers rather than restate them
  wherever the archived design already argues a passage (its own table:
  "O3, O4 → archived design Decisions 1, 2 and 7", etc.). That is the right
  call against `OPENSPEC-ARCHIVE.md`'s "Two copies drift" warning, and I
  checked several of the cited Decision numbers (1, 3, 5, 7, 9, 10, 11)
  against `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`
  and each argues what this piece's `design.md` says it does.
- **The archived folder is genuinely untouched.**
  `git diff origin/main...HEAD -- openspec/changes/archive/` is empty, matching
  `tasks.md` 1.2's claim.
- **No behaviour change.** Diff 1 touches only doc comments in three files
  (`arrival.rs`, `op.rs`, `transport.rs` — 7/10/6 lines each, all inside `///`
  or `//!` blocks). Diff 2's code changes (`authoring.rs`, `moderation.rs`,
  `revision.rs`, `transport.rs`) are doc-comment rewording plus one new test
  and two spec deltas that turn two `NO SPEC:` markers into requirements — I
  checked all three quoted scenario names
  ("One reading of the time signs the counter and the wall-clock alike",
  "A counter taken from the clock leaves the wall-clock at the current time",
  "A held op arriving again beyond the window is refused, and stays held")
  against the live specs and all three resolve
  (`openspec/specs/op-ordering/spec.md:360,366`,
  `openspec/specs/op-transport/spec.md:365`) — so these are behaviour the spec
  now documents rather than reasoning a spec still carries.
- **`design.md`'s label table is complete.** Every label `proposal.md` lists
  under "Reasoning removed from the specs" (O1–O18, F1–F4, T1–T5, V1, R1, H1–H2
  — 31 labels) appears exactly once in `design.md` Decision 1's destination
  table, either argued in a numbered Decision here or cited to the archived
  design. No orphaned label, no duplicate.
- **`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`**: 1180 + 30 tests, 0 failed — consistent with the "no
  behaviour change" claim.
- **`nix build ./dialectica#lgx`**: succeeded.

Only architecture was in scope for this instance; correctness, security and
readability are separate `code-reviewer` rows.
