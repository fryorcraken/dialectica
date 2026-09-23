# get-stoa — correctness review

Scope: correctness only (this dispatch named one dimension). Security,
readability and architecture are each another instance's job and are not
covered here.

## Tester's commit `b6740f4`: confirmed test-only

Checked `git show b6740f4` line by line against `mod tests {` boundaries in
both touched files:

- `identity.rs`: `mod tests {` opens at line 753; the diff's only hunk
  (new fn `an_address_in_uppercase_or_mixed_case_parses_to_the_same_address`)
  sits at lines 997–1022, and the file's single closing brace is at EOF
  (line 1594) with nothing after `mod tests`. Every changed line is inside
  the test module.
- `wire.rs`: `mod tests {` opens at line 3379; the diff's hunks (two new
  `#[test]` fns and a comment rewrite above an existing test) sit between
  lines 12677 and 15584, and the file's single closing brace is at EOF
  (line 15720) with nothing after `mod tests`. Every changed line is inside
  the test module.

`git diff --stat 3fb56b1 b6740f4` (the whole piece's implementation-bearing
range) touches only `identity.rs`, `wire.rs` and the openspec docs — no
production module outside those two files' test blocks. No finding.

## Mutation checks run independently (not merely re-reading the tester's claims)

Three mutations, applied and reverted in this worktree, each caught by the
test the tester's commit added:

1. `Address::from_hex` forced to reject any uppercase ASCII hex digit →
   caught by all three of `an_address_asked_for_in_uppercase_is_answered_in_lowercase`
   (wire, getStoa), `a_join_reply_reports_an_address_in_its_display_form_whatever_case_it_was_asked_in`
   (wire, joinStoa) and `an_address_in_uppercase_or_mixed_case_parses_to_the_same_address`
   (identity level). All three failed with `"stoa: address is not valid hex"`
   or the identity-level equivalent.
2. `CurrentMetadata::is_genesis_fallback` forced to always return `false` →
   caught by `a_peer_that_has_never_stored_an_op_is_answered_with_a_fallback`
   (`left: Bool(false), right: true`).
3. Reran the full `stoa_metadata` module (24 tests) clean as a baseline
   before mutating, and the full scoped suite (1063 + 30 tests) clean after
   restoring — `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
   dialectica -p dialectica-core`.

All mutations were reverted; `git status` and `git diff --stat` show a clean
tree before this findings commit.

## Implementation review (the whole piece, not only the tester's commit)

Read `stoa_metadata.rs` (new), the `get_stoa` handler and `stoa_metadata_json`
in `wire.rs`, the `Moderators::authorises` visibility change in
`moderation.rs`, and the adapter forward in `dialectica/rust-lib/src/lib.rs`.
Traced the attacker-facing paths specifically:

- `parse_stoa` → `Address::from_hex`: length checked before `hex::decode`,
  so an oversized `stoa` string cannot force a large allocation before being
  refused.
- `genesis_for`: hex length checked against `MAX_CANONICAL_BYTES * 2` before
  `hex::decode`, same shape, and the record is verified against the address
  (`Membership::verified`) before `get_stoa`'s handler ever calls `store()` —
  confirmed by reading the handler: `parse_stoa` and `genesis_for` both run
  and return before `store()` is invoked.
- `guarded()` (the panic backstop) correctly downcasts both `&str` and
  `String` panic payloads, so a `panic!("{msg}")` inside a handler is not
  silently swallowed into "non-string panic payload" — verified by reading,
  not just trusting the comment.
- `Moderators::authorises`'s three checks (scope, authority, authenticity)
  are unchanged by this piece; only its visibility widened to `pub(crate)`.
  Short-circuit order (scope, then authority/`contains`, then `verify()`)
  matches the documented cost rationale.
- `stoa_metadata::resolve`'s `find_map` skips non-binding entries rather than
  taking-then-checking the leading entry, which is what makes
  `a_binding_rename_behind_higher_counter_ops_of_other_kinds_still_decides`
  and the two `_does_not_displace_a_binding_one` tests meaningful rather than
  vacuous — read the implementation and confirmed it is `find_map`, not
  `next()` + a check.
- `SqliteOpLog::iter_stoa` scopes with `WHERE stoa = ?1` on the full 32-byte
  blob (no prefix match), and a freshly-created (schema-only, zero-row)
  database returns `Ok(vec![])` from that query with no special-casing
  needed — confirmed the new
  `a_peer_that_has_never_stored_an_op_is_answered_with_a_fallback` test
  exercises this real path (`SqliteOpLog::open` on a nonexistent file) rather
  than `MemoryOpLog`, and that `WireTempDir` (the fixture it borrows) predates
  this piece (introduced in #50), so no new untested scratch-dir machinery was
  added.
- `op.rs` (`OpKind::StoaMetadata` encode/decode, its length caps) and
  `stoa.rs` (`Genesis::decode`, `MAX_TITLE_BYTES`) are untouched by this
  piece (confirmed via `git log --name-only main..HEAD`) — out of scope for
  this review; their coverage was established by earlier pieces.
- The envelope sweep fixture `get_stoa_m` (wire.rs) is registered in
  `every_request_taking_method()`'s `vec![...]` and served from
  `a_log_renaming_agora()`, whose rename is signed by `feed_key(1)` — the
  same key `feed_genesis()` names as `creator`. Confirmed this makes the
  sweep exercise the `Declared` path, not only the fallback, matching
  design.md decision 14's claim.

No correctness defects found. The `CurrentMetadata` enum genuinely makes an
inconsistent fallback/values pairing unrepresentable (not merely
undocumented), the kind-check-before-authority-check ordering in
`binding_metadata` is real (not just commented), and every claim in
`design.md` I spot-checked by independent mutation held.

## Open findings

None from this dimension.
