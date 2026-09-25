# Security review — feed-row-contract

Dimension: security only. No defects found; no unticked boxes.

## What was reviewed

`git diff 28645a0...HEAD` confirms the change is spec, `design.md`, `proposal.md`,
`tasks.md` and tests/doc-comments only. `feed.rs` and `wire.rs` gain no new
production logic — every changed line outside `#[cfg(test)]` modules is a
doc-comment rewrite pointing at the new spec requirements instead of at
"no capability owns this shape" / `NO SPEC:` markers. Confirmed by reading both
files' full diffs line by line, not just the stat.

Read in full for security-relevant contract: `specs/feed-read/spec.md`,
`specs/thread-read/spec.md`, `design.md`, `proposal.md`, and the promoted
`openspec/specs/feed-read/spec.md` delta. Read the underlying (unchanged)
production code the new spec contracts: `feed.rs::list_threads`,
`wire.rs::list_threads_inner`, `parse_stoa`, `genesis_for`, `parse_index`,
`feed_page_json`, `thread_page_json`.

## What the spec now contracts, and why it holds for untrusted peer content

- **Authenticity gate is first and absolute.** `feed.rs:330`: `if
  !entry.op.verify() { continue; }` runs before any field of the op is read,
  including `parent`/`thread`. The new spec requirement *authentic ... names no
  parent* and its "A forged root is not a row" scenario match this.
- **The peer-supplied `thread` field is never trusted for row membership**
  (`feed.rs:334-339` filters on `parent: None` only). The new spec requirement
  and its "A parentless post carrying a thread field is a row" scenario pin
  this down where it was previously unwritten. Verified live: reverting the
  filter to `OpKind::Post { parent: None, thread: None, .. } => {}` (i.e.
  making the code start trusting the field) turns
  `a_parentless_post_carrying_a_thread_field_is_a_row_of_its_own` red
  (`left: 1, right: 2`). Reverted; `git diff` on `feed.rs` is empty again.
- **The closed field set prevents a derived-value channel.** The requirement
  that a row carry exactly its eight (or nine) keys, with a display name, an
  author address, a vote score, a time and the deciding moderation op all
  named as forbidden, is exactly the class of field CLAUDE.md's "moderation
  must be authenticated" / "a derived value beside its source can disagree"
  concerns are about (`generated-names`). Verified live: adding `"authorKey":
  row.author` next to `"author": row.author` in `feed_page_json` turns
  `the_feed_reply_is_the_ecosystems_pagination_shape` red (diff of the actual
  vs. expected key sets shown in the panic). Reverted; `git diff` on `wire.rs`
  is empty again.
- **`includeHidden` is client-controlled with no separate authorisation
  check**, but this is the core module's own local read API (JSON in, JSON
  out, called by the paired UI process), not a peer-facing RPC — hiding
  enforcement (who may hide, `moderation-resolution`) is a different
  capability and out of this change's scope. The existing code's own comment
  at `wire.rs:1655-1665` already states the danger of flipping the `null`
  default to permissive, and the spec's "A null include-hidden flag includes
  nothing hidden" / "A wrong-typed include-hidden flag is refused" scenarios
  pin the restrictive-default rule this comment argues for. No change asked
  for here.
- **Explicit `null` on required fields (`stoa`, `genesis`) is refused as
  wrong-typed, not reported as missing**, per `parse_stoa`/`genesis_for`
  (`Some(_) => "... must be a string"` catches `Value::Null` before `None`).
  This matches the new requirement and is already covered by existing
  handler-level tests (`a_null_required_field_is_refused_as_a_wrong_type_and_not_as_missing`,
  referenced at `wire.rs:13380`), not new to this diff.
- **Pagination indices avoid the integer-truncation trap named in this
  project's own prior findings.** `parse_index` uses `usize::try_from`, not
  `as usize`, and `feed.rs`'s page-slice arithmetic (`page.saturating_mul(per_page)`)
  avoids overflow. Both unchanged by this diff; the new tests
  (`the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`,
  `malformed_pagination_fields_are_refused_by_name`) pin exactly this
  boundary and pass against the real code.
- **Genesis record decoding is bounded before allocation**
  (`genesis_for`'s length check against `MAX_CANONICAL_BYTES * 2` before
  `hex::decode`), unchanged, guards against an oversized-payload DoS from a
  malicious `genesis` field.

## Open question already surfaced, not re-raised here

`proposal.md`'s "A post naming no parent is a row whatever its `thread`
field says" is flagged to the owner as an open question. I considered whether
this is a security gap (a peer spamming rows disguised as replies) rather
than a design question, and concluded it is not exploitable beyond ordinary
spam already possible with any parentless post — the `thread` field carries
no privilege and `thread-read` already never trusts it for membership. Correctly
scoped as an open design question, not a hidden security defect; not
duplicating it here as a checkbox.

## Gates run in this tree

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`:
  1153 + 30 tests, all passed, 0 failed.
- `nix build ./dialectica#lgx`: succeeded (exit 0, no output).

## Mutations left in the tree

None. Both mutations described above (`feed.rs`'s row filter, `wire.rs`'s
`feed_page_json`) were applied, observed to turn the cited test red, and
reverted with `Edit`; `git status --porcelain` is clean and `git diff` on
both files is empty as of this commit.
