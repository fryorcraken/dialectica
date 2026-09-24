# Security review — thread-reply-order (#147, PR #159)

Scope: security only, per dispatch. Diffed `origin/main...HEAD`. The change
touches exactly `dialectica/rust-lib/dialectica-core/src/thread.rs`,
`.../tests/end_to_end.rs`, and the OpenSpec proposal/design/spec/tasks files —
no wire.rs, no new dependency, no build/CI file.

## No actionable security findings

The entire behavioural change is one line in `read_thread`
(`thread.rs:698`): the loop that used to walk
`log.iter_stoa(stoa)?` now walks `log.iter_stoa(stoa)?.into_iter().rev()`.
Everything else in the loop body — signature verification, the `Post`-kind
filter, the `thread_of` membership chain-walk, moderator-authorised
moderation resolution, and the root/hidden-reply asymmetry in
`resolve_item` — is untouched by this diff and is evaluated once per entry,
independently of loop order:

- `entry.op.verify()` (line 701) still runs before any field of an untrusted
  op is read, exactly as before.
- `thread_of(log, &id)` (line 712) — the membership chain-walk that
  prevents an op's own unauthenticated `thread` field from placing it —
  is unchanged, still verifies every link before following `parent`
  (`thread_of`, `thread.rs:479`), and still terminates on a cycle via
  the per-walk `HashSet` (`thread.rs:448`) rather than an unbounded
  recursion. None of this is touched by the diff.
- `resolve_item` re-derives `current_version` and calls
  `crate::moderation::resolve(log, moderators, &id)` fresh for every
  entry (`thread.rs:796`, `802`) rather than accumulating state across
  the loop, so reversing the walk direction cannot change which ops are
  judged authentic, which reply belongs to which thread, or whether a
  moderator actually had authority to hide something. There is no
  order-dependent accumulator anywhere in this function for the
  reversal to disturb.
- The pagination arithmetic (`saturating_mul`/`saturating_add`/`.min()`,
  `thread.rs:765-770`) that turns a peer-supplied `page` of `usize::MAX`
  into an empty page rather than a panic or a wrapped slice index is
  unchanged by this diff.

I confirmed this isn't just a reading of the code: the baseline suite
(`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core`) passes in full post-diff — 1146 + 30 tests, 0 failed —
and that run includes the pre-existing forged-op and unauthorised-moderator
tests that exercise exactly the checks above through the now-reversed loop
(`a_forged_reply_is_not_returned_and_the_genuine_ones_are`,
`a_forged_op_in_the_chain_cannot_place_a_genuine_post`,
`a_forged_root_is_not_readable_as_a_thread`,
`a_forged_moderation_decides_nothing_and_is_named_by_nothing`,
`a_moderation_by_a_peer_with_no_authority_does_not_hide_a_root`,
`a_forged_hide_removes_nothing`). All pass, and they hold a forged or
unauthorised op in a multi-entry log alongside genuine ones, so they are not
vacuous against the direction the loop now walks.

`design.md`'s Risks section documents that an over-bound counter, or a
counter-less reply, can now land *before* the parent it answers. I read this
as a specified ordering/UX property, not a security defect: it does not let
an attacker read, hide, or forge anything they could not already reach —
the reply's own content and authorship are still gated by the same
verification and membership checks, unaffected by where it lands in the
sequence. The spec (`thread-read/spec.md`) explicitly forbids "fixing" this
with a second comparison inside `thread.rs`, which is the correct call: a
parent-aware reorder would be exactly the second ordering-rule
implementation CLAUDE.md and this module's own header warn against, and two
peers running two rules is the failure mode (silent divergence with no
error) rather than a one-off misplacement that both peers still agree on.

## What I tried and could not complete

I attempted to run `cargo mutants` scoped to `thread.rs` (three tries, two
different `--file` path spellings) to get a measured mutation-kill number
for this dimension, per the reviewer brief's "Also check" section. All three
attempts failed before testing a single mutant: the first two produced "No
mutants found under the active filters" (a `cargo-mutants` path-matching
issue I could not resolve against this workspace layout), and the third
(package-scoped, no `--file`, confirming 33 mutants exist for `thread.rs`)
failed with "cargo test failed in an unmutated tree" because the crate's
full test binary takes ~96s and my `--timeout 20` was too short even for the
baseline run. I did not retry with a longer timeout, since a real run here
would run 33 mutants at ~30s build + ~96s test each and blow well past the
"abandon if it runs past a couple of minutes" guidance. This is a gap in
*measured* mutation coverage for this file, not a defect — the manual
argument above (per-entry, order-independent security checks; pre-existing
tests that exercise the reversed loop and pass) is what this review rests
on instead.

I also considered directly disabling `entry.op.verify()` in the loop by
hand to confirm the forged-op tests would catch it — the highest-value
single mutation for this dimension — but the harness's auto-mode classifier
denied that edit as "Security Weaken" before it reached the file. I did not
pursue it through another tool or phrasing, per the classifier's own
instruction and the reviewer brief's mandate not to work around a denial.
The existing `a_forged_reply_is_not_returned_and_the_genuine_ones_are` test
(cited above) is written specifically to fail if verification is removed
("The forgery IS in the log, so this fails if verification is removed
rather than passing because nothing was stored") and passes today, which is
the closest evidence available without that mutation.

## Areas checked and clean

- Authenticity gate ordering (verify-before-read) at every op this module
  touches, including the root and every chain link `thread_of` walks.
- Membership: `thread_of` uses the verified parent chain, never the op's
  own unauthenticated `thread` field, and is unchanged by this diff.
- Moderation: resolved fresh per read from `moderators`/the op log, not
  cached or accumulated across the loop; the hidden-root vs. hidden-reply
  asymmetry in `resolve_item` is unaffected by iteration order.
- Peer-controlled arithmetic: `page`/`per_page` clamping and the pagination
  slice bounds are unchanged and still saturating/`.min()`-guarded.
- No new dependency, no wire/RPC boundary change, no secret-comparison code
  anywhere near this diff.
