# Design review: sdk-staging (#168)

No findings. The code matches every recorded decision, and nothing material
was decided in the code without a Decisions entry.

## What was checked

- Read issue #168 with `gh issue view 168 --json body,comments`, including the
  owner's 2026-09-25 comment. The issue body had already been edited to
  incorporate that comment (confirmed against the comment text), and
  `proposal.md`/`design.md` cite the comment as the standing instruction where
  the two might have disagreed (e.g. "the relative form must be run, not
  assumed").

- Compared each of design.md's five Decisions (D1–D5) against the actual diff
  (`git diff 8368b2f...HEAD` — three dots, per the brief):
  - **D1** (`--inputs-from ./dialectica`, no `flake.nix` re-export): the
    command appears verbatim in README.md and `.github/workflows/ci.yml`, and
    nothing under `dialectica/flake.nix` changed.
  - **D2** (`-o` out-link, not a copy): README and CI both use `-o
    dialectica/logos-rust-sdk-src`; re-running was verified to replace the
    link (`tasks.md` 1.4), and I independently re-ran the command in a live
    tree and confirmed it lands the SDK.
  - **D3** (README and CI carry the command byte for byte, `-L` dropped): `git
    grep -F` of the exact command string finds it identical in `ci.yml` and
    `README.md`; the old step's `-L` flag is gone from the diff.
  - **D4** (README documents once; `.gitignore`/`CLAUDE.md` point there, no
    second command): both files were edited to point at README "Building"
    with no command repeated; `git grep -F` of the staging command finds zero
    occurrences in `.gitignore` or `CLAUDE.md`.
  - **D5** (Nix-version comment re-grounded, not dropped): the `rust` job's
    `install-nix-action` comment now cites `--inputs-from` and
    `dialectica/flake.lock`'s node types instead of the old "one `github:`
    flake" reasoning. Verified against the lock directly: `jq "[.nodes[] |
    .locked.type] | unique" dialectica/flake.lock` → `null`, `git`, `github`
    — no `path` node, matching the comment's claim.

- Looked for choices made in the diff without a Decisions entry: the CI diff
  also drops the `echo "builder pin from dialectica/flake.lock: $rev"` line
  (no longer meaningful once the rev is never read out) and rewords the
  step's comment block — both are direct consequences of D1/D3 rather than
  separate undocumented decisions.

- Checked the diff against the issue's "Done when" and against scope in
  `proposal.md`'s "Out of scope": no `flake.nix` change, `ci.yml` line 1768
  and archived changes left untouched, nothing under `.claude/` touched — all
  confirmed by `git diff --stat` and targeted greps.

- Ran the three verification commands myself rather than trusting `tasks.md`'s
  recorded output:
  - `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o
    dialectica/logos-rust-sdk-src` — succeeded, staged the SDK.
  - `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
    -p dialectica-core` — `dialectica` 1180 passed, `end_to_end` 30 passed, 0
    failed, matching `tasks.md` 2.1's recorded counts.
  - `nix build ./dialectica#lgx` — succeeded (exit 0).

- `.openspec.yaml` carries `schema: spec-driven` and `skip_specs: true`; the
  `tasks.md` stage block strikes the spec and test rows with reasons that
  match this document's scope (no wire/op/view/storage change).

The Decisions entries themselves are in reasonable shape against the "what a
good entry contains" checklist: each names what was chosen, the constraint,
the alternatives considered and what ruled them out, and what it costs (the
Risks/Trade-offs section). D1 in particular carries measured evidence for its
guard claim (the `--inputs-from` table), which is the kind of mutation-style
evidence this review looks for, even though nothing here is a guard in the
usual code sense.

## Re-review of 67a6605

Read issue #168 fresh with `gh issue view 168 --repo fryorcraken/dialectica
--json body,comments`, including the owner's 2026-09-25 comment.

This commit answers the readability finding's dangling "shallow-`//` trap"
reference by dropping it from both `design.md` D1's re-export alternative and
`proposal.md`'s matching out-of-scope bullet, rather than reconstructing an
explanation nothing ever committed.

Checked:

- **D1's re-export alternative still gives a reason, not just a ruling.**
  After the edit it reads: "The owner's comment on #168 ruled it unnecessary
  if `--inputs-from` works. It does, so the build's own flake is not changed
  to serve developer tooling." That is a complete "what was chosen /
  constraint / what ruled it out" — the reason is not the trap, it is "the
  build's own flake is not changed to serve developer tooling", which was
  already present before this commit and does not depend on the dropped
  sentence.
- **The issue comment itself never defines the trap either** ("If it holds,
  the re-export and the shallow-`//` trap in the proposal are unnecessary" —
  named, not explained), so dropping rather than reconstructing is the
  correct call against the source, not only against `git log`.
- **No dangling reference survives.** `git grep -n -F "shallow"` across
  `openspec/`, `CLAUDE.md`, `docs/`, `README.md`, `.gitignore`, and
  `.github/` matches only the readability findings file itself (which
  documents the removal) — matches the commit's own claim.
- **`proposal.md`'s out-of-scope bullet now points to D1** ("`design.md` D1
  records it among the alternatives") rather than repeating the trap
  reference, keeping the two documents in one place of truth rather than two
  independent mentions.
- **Matches the issue as corrected.** The owner's comment's two corrections
  (the #91 runs did use the pinned SDK; `--inputs-from` alone plausibly
  suffices) are both still reflected in D1's alternatives #4 and #5 — this
  commit touched neither of those, and they were not asked to be revisited.

No new findings. Ran both verification commands from the brief on this tree:
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core` — 1180 + 30 tests passed, 0 failed, matching the commit's
own recorded count. `nix build ./dialectica#lgx` — succeeded (one retry
needed after a transient `SQLite database ... is busy` eval-cache warning on
the first invocation, which is unrelated to this change and did not fail the
build; the immediate retry produced a clean, silent success).
