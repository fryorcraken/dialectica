# Security review — position-and-index (#166 / PR #180)

Scope: security dimension only (untrusted inbound input at the wire boundary,
what a malformed request can reach before it is refused, and what error
messages disclose). Read the owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024,
2026-09-25): item 2 requires a malformed `index` (negative, fractional, or not
a number) to be refused with an error naming `index`, a test must fail when
the name is dropped from the message (shown by mutation), and leaving the
message unspecified was ruled out. Item 1 (position) is unchanged by that
comment.

## What I checked

- `git diff origin/main...HEAD` in full: the spec deltas
  (`specs/identity-onboarding/spec.md`, `specs/thread-read/spec.md`),
  `proposal.md`, `design.md`, `tasks.md`, and the test additions in
  `dialectica/rust-lib/dialectica-core/src/wire.rs`. Confirmed the diff is
  **purely additive** to `wire.rs` — no production line was changed, added or
  removed outside `mod tests` (`git diff ... | grep '^-'` shows only the
  diff-header line). `parse_index`, `keep_identity` and `thread_page_json` are
  pre-existing and unchanged by this piece; this piece pins their behaviour
  with tests.
- `parse_index` (`wire.rs:1780-1833`): the malformed-`index` boundary check.
  Every non-`None`/`Null` branch either returns `Ok(Some(usize))` or an
  `Err(error_json(...))` naming `field` — no panic path, no coercion of a
  malformed value to a candidate. `usize::try_from` (not `as usize`) is used,
  so a too-large value is refused rather than truncated on a narrower target;
  the code comment is honest that this arm is unreachable on the 64-bit
  targets CI builds, and the test suite's `18446744073709551616` case reaches
  the *other* arm (`as_u64` returns `None` because serde falls back to `f64`
  above `u64::MAX`), which the new test correctly tolerates since it only
  asserts the message `contains("index")`, not which branch produced it.
  Ordering in `keep_identity`: `index` is parsed and refused **before**
  `session.keystore_for(open)` is called, so a malformed `index` costs no
  keystore file I/O.
- `keep_identity` / `keep_selection`: nonce/liveness is checked before
  anything is derived or written; `index` is resolved to a candidate only
  after that; store writes happen only on the success path. No path lets a
  malformed `index` reach `Keystore` or `MembershipStore`.
- `thread_page_json` (`wire.rs:2145`): emits `item.position` — a
  whole-thread-relative string index computed in `thread.rs`
  (`resolve_item`/the page-assembly walk), unchanged by this piece. Nothing
  here echoes attacker-controlled peer content into an error message or a
  position value; `position` is a locally-computed sequence index, not
  copied from the item.
- `error_json` (`wire.rs:30-35`) JSON-escapes its message via `serde_json::json!`,
  so a message built with `format!("{field} ...")` cannot break the wire
  shape even though `field` is a fixed literal (`"index"`/`"page"`/`"perPage"`)
  here, never attacker text.
- Mutation testing, scoped to the three pinned functions
  (`cargo mutants -f dialectica-core/src/wire.rs -F "parse_index"` and
  `-F "keep_identity|thread_page_json"`): 7 of 7 generated mutants caught
  (3 for `parse_index`, 4 for `keep_identity`/`thread_page_json`). Note this
  tool version only generates whole-function-body replacement mutants for
  these functions (no match-arm-level mutants were listed), so this is a
  coarse pass, not exhaustive.
- Targeted, hand-written mutation directly on the owner's decision: dropped
  `{field}` from `parse_index`'s "must be a non-negative integer..." message
  (the arm that `1.5`, `0.0`, `1e2` and the too-large value hit) and reran
  `each_malformed_kind_of_index_is_refused_by_name`. It went red immediately
  (`the refusal of an index that is negative (-1) must name \`index\``),
  confirming the test genuinely enforces the owner's requirement rather than
  passing regardless. Restored afterward; `git diff --stat` on `wire.rs`
  confirms the tree is clean of this mutation.
- Confirmed the `keep_with_raw_index` test helper (`wire.rs:4566`) splices
  each `MALFORMED_INDEXES` entry into a fixed JSON template
  (`r#"{{"stoa":"...","slate":"...","index":{index}}}"#`); every entry
  (`-1`, `1.5`, `0.0`, `1e2`, `"two"`, `[]`, `true`,
  `18446744073709551616`) produces valid JSON at that splice point, so this
  is a faithful simulation of what a caller could actually send over the
  wire, not a test artefact that only "works" because of how the harness
  builds the request.

## Findings

None. I found no security defect in this piece along the assigned dimension.

- The piece is test-only (plus spec/design/proposal/tasks prose); it changes
  no production code and introduces no new inbound-parsing path.
- The malformed-`index` boundary behaviour the owner's decision requires is
  implemented (pre-existing, unmodified by this piece) and is now genuinely
  pinned: refusal is the error shape, names `index`, stores nothing, and is
  reached before any keystore file is touched. Verified by mutation, both
  the tool's coarse pass and a hand-written targeted one on the exact
  message the decision cares about.
- No error message inspected discloses filesystem paths, key material, or
  anything beyond a description of which field was wrong.
- No panic-reachable path was found in the code these tests exercise
  (`parse_index` uses `try_from` and total `match`es throughout; no
  `unwrap`/`expect`/indexing on attacker-influenced lengths in the diff or
  the pinned functions).

Nothing under `.claude/` changes in this piece, and none should as far as I
can tell from the security dimension.

## Gate record

- [x] **none** — re-checked on the current tree: every `wire.rs` hunk since `1fd91a4f` (`70bef050`, `4e2b8c0f`) lies at line 8558 or later, inside `mod tests` (opens at 3531), so `parse_index`, `keep_identity` and `thread_page_json` are unchanged and the verdict holds.
