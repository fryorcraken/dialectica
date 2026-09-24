# Security review — `join-preview-getstoa`

Dimension: **security only** (untrusted peer input at every boundary this
change touches — genesis records, metadata ops, the `getStoa` request/reply,
the join preview; impersonation/display-spoofing through titles; whether a
refusal can be bypassed or turned into denial of service). Correctness,
readability and architecture are other reviewers' rows.

## What I checked

Read the full `git diff origin/main...HEAD` (three dots) for every file in
the piece. Read `gh issue view 143 --repo fryorcraken/dialectica --comments`
for the owner's rulings (lookalike stays keyed to founding titles; blank
title invalid everywhere; a store already holding a blank-titled record is
neither migrated nor skipped) and checked the code against each.

Traced the blank-title guard through every layer it claims to cover:

- `dialectica-core/src/stoa.rs` — `BLANK_CHARACTERS`/`is_blank_title`,
  `Genesis::canonical_bytes` (encode) and `Genesis::decode` (decode), both
  refusing a blank title; the decode-side check runs **after** the structural
  checks (length prefix, trailing bytes), so a title that is both blank and
  malformed reports the structural fault, matching `stoa-genesis`'s and
  `op-format`'s "blank title is the last refusal" requirement. Verified this
  against the tests (`a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes`
  in both `stoa.rs` and `op.rs`).
- `dialectica-core/src/op.rs` — `Op::check_admitted`, called from both
  `Op::encode` and `Op::decode`, deliberately **not** from `canonical_bytes`,
  `id`, `sign` or `SignedOp::verify`. This is the right shape: it keeps
  "authentic" and "admitted by the format" separate, which is exactly what
  lets `stoa_metadata::binding_metadata`'s own blank check be exercised by a
  test (an op that is signed, verifies, and is authorised, and is still
  refused only by the resolver's own guard).
- `dialectica-core/src/stoa_metadata.rs` — `binding_metadata`'s fourth
  condition (blank title never binds), checked before `authorises` (cheap
  check first, matches CLAUDE.md's boundary-before-state-machine framing for
  a resolver that must not trust an inbound log to have decoded).
- `dialectica-core/src/transport.rs` — `receive()` refuses a blank-titled
  metadata op at `Op::decode`, before touching the state machine
  (`a_blank_titled_metadata_op_from_a_peer_is_refused_at_the_boundary`).
  `publish()` refuses one before the append
  (`a_blank_titled_metadata_op_is_not_published_or_stored`), so a local bug
  that tried to construct one cannot leak it onto the wire either.
- `dialectica-core/src/log/sqlite.rs` and `log/mod.rs` — `append` refuses an
  unencodable op before the write (`an_op_with_no_encoding_is_refused_and_nothing_is_written`);
  a row that predates the refusal (owner's "store already holds one" case)
  fails **every** read that would return it — `get`, `iter`, `iter_stoa` — as
  `CorruptEntry`, never silently filtered and never repaired/rewritten
  (`a_creator_signed_blank_titled_op_stored_before_the_refusal_fails_every_read`,
  and the pre-existing `a_read_fails_rather_than_returning_the_surviving_op_beside_a_corrupt_one`
  generalizes this to "fails the whole Stoa read, does not return the
  survivor" — the correct DoS-resistant shape: an attacker who gets one
  poisoned row into a peer's store cannot use it to selectively hide other
  ops, only to make the read of that Stoa fail loudly).
- `dialectica-core/src/membership.rs` — `Membership::verified` refuses a
  blank-titled record via `Genesis::canonical_bytes`/`address` before the
  store is reached; a retained pre-existing blank-titled row is reported as
  `CorruptEntry` by `list()`, with the row left byte-for-byte unchanged
  (`a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated`)
  — matches the ruling exactly (neither migrated nor skipped).
- `dialectica-core/src/wire.rs` — `get_stoa` orders its checks correctly:
  parse request → parse `stoa` → decode+verify `genesis` (`genesis_for`,
  which bounds the hex string's length **before** `hex::decode` allocates, so
  an attacker-chosen length cannot force an oversized allocation before the
  format's own cap is applied) → `Founding::of` → open store → resolve. A
  malformed or mismatched request never reaches disk. `create_stoa` refuses a
  blank title via `genesis.address()` before the store is opened, so "a
  failed creation leaves nothing behind" holds for blank titles the same way
  it already held for over-long ones.
- `dialectica-ui/src/qml/Core.qml` — `blankCodeUnits`/`isBlankTitle` and
  `stoaMetadataFrom` treat the core's own `getStoa` reply as untrusted too:
  a reply carrying a blank `title`, or missing/mistyped fields, is normalised
  to a failure rather than rendered. Good defense in depth even though the
  core is same-process.
- `dialectica-ui/src/qml/DJoinScreen.qml` — every title/description binding
  renders with `textFormat: Text.PlainText`, matching `stoa-navigation-view`'s
  "MUST be rendered as plain text and MUST NOT pass through any rich-text or
  markup-interpreting path". The lookup is stored keyed to
  `{stoa, genesis}` and only rendered while that exact pair is still on
  screen (`currentLookup`), the same anti-impersonation shape the pre-existing
  join outcome uses, so an answer for one reference cannot be shown over a
  different one. The lookalike comparison (`lookalikes`) reads only
  `foundingTitle`, which a non-fallback reply never fills and which is never
  blank (both a blank join title and a blank lookup title are discarded
  before reaching it) — matches the owner's ruling that the comparison stays
  keyed on founding titles and is not extended to current titles, and that a
  blank title cannot spuriously match.
- Confirmed by search (`git grep -n -F "OpKind::StoaMetadata"`) that no
  production (non-test) code path in `wire.rs` constructs a `StoaMetadata` op
  — there is no rename/set-metadata authoring endpoint in this piece, so the
  only ways a blank-titled metadata op can reach the resolver in practice are
  (a) a pre-existing row from before this change, handled above, or (b) a
  hand-built fixture in tests. Nothing here lets a peer smuggle one past
  `check_admitted` through some other authoring path.

## Mutation probes (reverted; `git status` clean before and after)

1. Disabled the resolver's own blank check in
   `stoa_metadata::binding_metadata` (`if false && is_blank_title(title)`).
   Caught: 2 of 25 `stoa_metadata::` tests failed
   (`a_metadata_op_carrying_a_blank_title_does_not_bind`,
   `a_later_blank_titled_op_does_not_displace_a_binding_one`). Confirms the
   resolver's guard is load-bearing on its own, independent of the
   encode/decode guard, as `design.md` decision 2/3 claims.
2. Made `stoa::is_blank_title` always return `false` (simulating a format
   that never refuses a blank title). Caught broadly across the
   `dialectica-core` suite (many failures across `stoa.rs`, `op.rs`,
   `stoa_metadata.rs`, `log::sqlite`, `membership.rs`). Confirms the shared
   predicate is exercised from every layer that claims to depend on it, not
   just the layer each test file happens to sit in.

Both mutations were reverted before writing this file; `git status --short`
is clean.

## Findings

None. Every boundary this change touches — genesis decode, metadata-op
decode, the transport `receive`/`publish` pair, the SQLite and in-memory
logs, the membership store, the `getStoa`/`join_stoa`/`create_stoa` wire
handlers, and the QML join-preview rendering — refuses a blank title before
it reaches a state machine or a render target, refuses it the same way on
every path, and does not let a pre-existing blank-titled row escape as a
false success. The lookalike comparison is scoped exactly as the owner
ruled, cannot be fed a blank founding title, and is not silently skipped
without the screen saying so. Titles and descriptions render as plain text
everywhere they reach the view. I could not construct a bypass, and the two
guards I tried to remove by mutation were both caught by the existing suite.

Areas outside this dimension (test-scenario completeness beyond what a
security bypass would need, spec/code correspondence beyond the owner's
rulings, readability/architecture of the new code) are left to the other
reviewers.
