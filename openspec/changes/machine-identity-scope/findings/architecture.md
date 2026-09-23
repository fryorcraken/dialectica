# Architecture review — machine-identity-scope (#149)

Scope: architecture only (wire-contract deliberateness, per-Stoa derivation
left re-activatable for #108, view-forwards-only, collision with #150). No
correctness, security or spec/test-correspondence findings are in scope here.

No findings. Every claim checked against the diff (`git diff 041eaff...HEAD`)
held.

## What was checked and confirmed clean

- **The wire contract change is deliberate, not incidental.** `Whoami::Identity`
  in `dialectica/rust-lib/dialectica-core/src/wire.rs` (~line 1083) drops `path`
  with a doc explaining why (the machine key derives from no path), and
  `identity-onboarding/spec.md`'s delta forbids the field in the same reply
  (closed-field-set requirement). No QML consumer reads `.path` off a `whoAmI`
  reply (`git grep -n "\.path\b"` across `Core.qml`, `FeedScreen.qml`,
  `DIdentityChip.qml`, `Main.qml` returns nothing), so the drop is a clean
  break with no dangling reader. `posting_identity`, `publishing_key`,
  `whoami_for`, `who_am_i` and `get_capabilities_from_stores` all had their
  `stoa`/record parameters removed (verified against their signatures at
  wire.rs:344, 399, 415, 1140, 1172) rather than merely left unread — matching
  design.md D3's stated reason (a parameter present is one edit from being read
  again). `NO_CHOICE_FOR_THIS_STOA` is gone entirely (`git grep` returns
  nothing), matching D5.

- **The per-Stoa derivation is left intact and cleanly re-activatable for
  #108.** `Keystore::stoa_key_at_path` / `stoa_public_key_at_path`
  (`keystore.rs:846,857`), the `chosen_paths` table and `record_path`
  (`identity_store.rs`), and `generate_identity_slate` / `keep_selection`
  (`wire.rs:659,911`) are untouched in shape and still exercised by their own
  wire-level tests. `dialectica/rust-lib/src/lib.rs`'s `keep_identity` and
  `generate_identity_slate` handlers still open the record via `Self::paths`
  (lib.rs:1000) — only the three identity-*in-use* paths (`publish`,
  `get_capabilities`, `who_am_i`) stopped opening it. This is exactly the
  "slate/keep keep working, nothing in 0.0.1 reads what they record" shape
  design.md D2 describes, and it is what makes #108 "switching the call back
  on" rather than a rebuild.

- **The view still only forwards to core.** The `Main.qml` diff removes the
  `onboarding` navigator state, `createIdentityFor`, `closeOnboarding` and
  `identityWasKept`, replacing them with `acquireIdentity()` = `enterOnly("",
  null)` (the Stoa list) — pure navigation-state removal, no new business
  logic introduced. `DIdentityChip.qml`'s change is a caption string swap.
  `FeedScreen.qml` still asks `whoAmI(stoa)` unchanged and just renamed a
  signal's doc comment. None of the touched QML files add network, disk or
  decision-making logic that belongs in core.

- **No collision with #150.** `piece/home-screen-key-states` (`gh issue view
  150`) rebuilds `DStoaListScreen.qml` around a `hasMachineKey` flag; this
  piece's diff touches no line of `DStoaListScreen.qml`
  (`git diff 041eaff...HEAD --stat -- dialectica-ui/src/qml/DStoaListScreen.qml`
  is empty). design.md D9 explicitly flags two comments in that file (~line
  633, ~662) as now stale and left for #150 to fix — I read both and confirmed
  they say what design.md claims ("the only thing that writes one is per-Stoa
  onboarding" / "`DOnboardingScreen` is where a per-Stoa identity is chosen"),
  so this piece correctly identifies rather than silently leaves the drift.
  The `DOnboardingScreen` uninstantiation this piece performs
  (`qmldir`'s `# UNINSTANTIATED:` line, `Main.qml` no longer mounting it) is
  orthogonal to #150's `hasMachineKey` two-state redesign of the Stoa list —
  they touch different files and different flags (`hasIdentity` on the feed
  chip vs. the list's own `hasMachineKey`), so no rebase collision is expected
  from the architecture shape, only from ordinary file-adjacency if both PRs
  touch `Main.qml` (this one does; #150's own description does not list
  `Main.qml` among its files).

- **CI adapter gate kept in sync.** `.github/workflows/ci.yml`'s
  "the adapter derives the creator and the poster in one place" grep-gate was
  updated in the same commit to require `core::wire::publishing_key` (not the
  old `Self::paths` + `core::wire::publishing_key(&stoa, &keystore, &paths)`
  three-arg form) and to explain why `core::stoa_of` remains required for
  ordering rather than derivation. Both `WANTED` strings it checks for
  (`core::wire::publishing_key`, `core::stoa_of`) are present verbatim in
  `lib.rs`, so the gate does not silently pass over a mismatch.

- **One function, one job** holds for the touched code: `keep_selection` /
  `undo_a_keystore_this_keep_wrote` (D8) is a decision separated from its JSON
  shaping, consistent with the existing `capability_for` pattern the file's
  own docs cite as precedent. `Main.qml`'s `acquireIdentity()` is named for
  what it does rather than reusing `closeFeed()`'s name for a different
  request, matching D9's own stated reasoning.

## Dimension coverage

Architecture only, as instructed. Correctness, security and
spec/test-correspondence are other reviewers' rows.
