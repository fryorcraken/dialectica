# Correctness review — `unauthorised-rules`

Scope: correctness dimension only, per dispatch. Is each withdrawal justified
(the rule really lacked authority), and did any true measured fact or trap get
removed with it?

## No defects found

Every withdrawal was independently checked against `openspec/specs/`,
`openspec/changes/archive/`, and `docs/`, and every one of the writer's three
self-reported corrections was independently verified rather than accepted:

- **`FeedScreen.qml`** — the withdrawn clause ("worse than absent... reads as
  a working control") does contradict `composer-view/spec.md`'s inert-affordance
  requirement ("the view SHALL render that affordance inert... general and
  SHALL NOT be read as being about any one field"), confirmed present at
  `openspec/specs/composer-view/spec.md`. The retained text ("nothing to
  select, and `reload()` does not read `ordering`") is the measured mechanism
  and survives unchanged in substance.
- **`DComposer.qml`** — the measured defect account (a post accepting and
  silently dropping `parentOp`) is retained verbatim in substance; only the
  generalized claim about how comments in this repo must be written is gone.
- **`check_qml_members.sh` / `check_qml_names.py`** and their `tst_*`
  counterparts — the "worse than no gate" absolutism is softened to cite
  `docs/PLAN.md` §10 (confirmed titled "CI", confirmed at line 3627, confirmed
  the "Anti-false-green" passage at line 3667 is phrased as describing
  Radicle's CI's own built-in observation — "Radicle's CI is built around the
  observation that..." — not asserted first-person as a dialectica rule). The
  measured claim itself (an empty corpus makes a gate pass vacuously) is
  unchanged in every case grepped.
- **`DIdentityChip.qml`** — verified independently that no spec pairs an
  author address with an identicon mark: grepped `openspec/specs/` for
  identicon/address pairing and found none. Confirmed `op-format/spec.md:496`
  pre-emptively scopes "show the Stoa address alongside any name" to Stoas and
  refuses the author-address reading ("The bare sentence is scoped because it
  is the one a later reader would cite out of context as authority for an
  author address, which this change deletes"). Confirmed `docs/IDENTICON.md:20-21`
  does cite a `SPEC.md` for the address-on-screen claim, and confirmed no
  `SPEC.md` file exists anywhere in this checkout (`find` returned nothing) —
  so "sources that to a `SPEC.md` no longer in this repo" is accurate. Confirmed
  `generated-names/spec.md:507` carries the "SHALL NOT be treated as unique...
  SHALL NOT be accepted anywhere an identity is named" text cited for the name
  half. The grounded fact (mark ≠ identifier) is kept and now cited to its
  source rather than restated as an unqualified standing rule.
- **`DStatusBar.qml` / `DTheme.qml`** — the "only green/orange" claim is
  demoted from a binding prohibition ("spending them anywhere else spends the
  signal...") to a statement of present fact ("as things stand, the only...").
  The `markGreen`/`markSage` carve-out and its reasoning are untouched.
- **`tst_stoa_screens.qml`** — diff is comment-only; the retained mutation-
  testing finding (an absence assertion is only as strong as its corpus) and
  the pinning test name are both unchanged; only the "generalises the rule ...
  written accordingly" framing changed, which is not a claim of binding
  authority over other files.
- **`run-qml-tests.sh:19`** — not in the diff at all (verified: file absent
  from `git diff origin/main...adda12c --name-only`). Its citation ("PLAN.md
  §10 calls worse than no gate at all") is accurate against §10's actual text
  and was correctly left alone.

**`VoteControl.qml` is untouched** — confirmed via `git diff
origin/main...adda12c -- dialectica-ui/src/qml/VoteControl.qml` (empty output)
and absence from the changed-file list. Its ratifying spec citation
(`composer-view/spec.md`'s "The vote control displays no score") was not
re-verified line-by-line since the file carries no diff to review, but the
proposal's non-modification claim is confirmed structurally.

**No spec, archive entry, or doc was found ratifying any of the eight withdrawn
generalizations.** Searched `openspec/specs/`, `openspec/changes/archive/`, and
`docs/` for each site's characteristic phrasing (`worse than absent`, `never to
raise`, `standing rule wherever`, `worse than no gate`, `the only green and
orange`, `should not be dropped to save a row`); the only near-hits were
unrelated uses of the same rhetorical template in archived design docs and
`dialectica/rust-lib/keystore.rs` (outside this change's scope, and about a
different property — encryption-that-reads-as-encryption, not a comment-style
rule).

**No reachable-panic, boundary-validation, or moderation-authority concern
applies** — this piece is a comment-only sweep in `dialectica-ui/`, confirmed by
diffing every changed file: all hunks are `//`/`#`/docstring lines, no logic,
no renames, no assertion changed. `git diff --stat` against `origin/main`
confirms the touched files are limited to five `.qml` sources, four test/gate
scripts, and the four `openspec/changes/unauthorised-rules/` change documents.

## Gates run directly (not piped, not trusted from the report)

All run from this worktree against the absolute paths under
`dialectica-ui/tests/`:

- `python3 dialectica-ui/tests/check_qml_names.py dialectica-ui` →
  `ok: 41 QML file(s) and 22 qmldir entries checked...`
- `sh dialectica-ui/tests/check_qml_members.sh dialectica-ui/src/qml` →
  `ok: 23 QML file(s) checked, every member read off a known type exists`
- `python3 dialectica-ui/tests/tst_check_qml_names.py` → `check_qml_names: all
  cases passed` (17 sub-cases)
- `sh dialectica-ui/tests/tst_check_qml_members.sh` → `check_qml_members: all
  cases passed` (5 sub-cases)
- `sh dialectica-ui/tests/run-qml-tests.sh` → exit 0, 18 spec files ran,
  0 `FAIL` lines across the full output (grepped the saved output, not piped
  the live run).

All five pass. The comment edits inside the two gate scripts and their two
test files did not break the containing file.

## What this review did not cover

Security, readability and architecture are out of scope for this dimension and
are not commented on here. Whether the *decision* to demote each site (rather
than delete or fully keep it) was well-recorded in `design.md` is the
design-reviewer's question, not correctness's.
