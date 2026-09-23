# Security review — machine-identity-scope

Dimension covered: **security only** (key material on disk / D8 undo path and
permissions, whether wire input can steer which key signs, creator-key vs
signing-key consistency, and whether the withheld per-Stoa slate path is
abusable). Correctness, readability and architecture are other reviewers'
lanes and are not covered here.

No security defects found. No checkbox items to report.

## What was checked, and how

**Key material on disk (D8's undo path).** Read `keep_selection` and
`undo_a_keystore_this_keep_wrote` (`dialectica/rust-lib/dialectica-core/src/wire.rs:911-1061`).
The write order (keystore first, record second) and the `wrote_the_keystore`
flag correctly gate the undo: a keep's own freshly-created keystore is removed
on a record-write failure, and a pre-existing master key is never touched.
Re-ran the flag-discrimination mutation independently of the tester's
2026-09-24 note in `design.md` (Risks) rather than trusting that note as
given: changed `if wrote_it {` to `if true {` in
`undo_a_keystore_this_keep_wrote`, ran
`cargo test -p dialectica-core a_keep_whose`, and confirmed
`a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`
fails (`left: None, right: Some([...keystore bytes...])`, "a failed keep
removed or rewrote a master key it did not write"), while the other two
`a_keep_whose_*` tests stayed green. Reverted immediately; `git status
--short` showed a clean tree before committing this file. File permissions
(`keystore.rs`'s `OWNER_ONLY = 0o600` and its `PermissionsTooOpen` checks) are
untouched by this diff (not in the changed-files list) and are not evaluated
further here.

**Whether wire input can steer which key signs.** `posting_identity`,
`publishing_key`, `whoami_for` and `who_am_i` (`wire.rs:344`, `399`, `1172`,
`1140`) take no Stoa and no record after this change — confirmed by reading
the diff and the current signatures, not only by trusting `design.md`'s D3
account. `authoring::Authorship` (`authoring.rs:209`) — untouched by this
piece — takes `key: &SecretKey` as a plain parameter with "no field a caller
could set that reaches it", and `Dialectica::publishing` in
`dialectica/rust-lib/src/lib.rs` derives that key from
`core::wire::publishing_key(&keystore)` alone; no field of `PublishRequest`
(`stoa`, `body`, etc.) reaches key selection. Grepped `wire.rs` and `lib.rs`
for every remaining `stoa_key` / `stoa_public_key_at_path` call and confirmed
the only hits outside doc comments are inside `#[cfg(test)]` fixtures — no
live call path derives a per-Stoa key for signing or reporting any more.

**Creator key vs signing key consistency (moderation authority).**
`creator_key_in` → `identity_public_key`, and `publishing_key` →
`keystore.identity_key()` — the same root, one derivation position
(`wire.rs:399`, D4). `seed_store.rs`'s asserts now pin
`moderators.contains(&founder.public_key())` (was the opposite before this
change), i.e. a Stoa's creator is now provably also its publishing key, which
is the property `posting-capability` and the new `identity` requirement
("A Stoa's creator posts as its creator") both require. Ran the full
workspace suite (`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
dialectica -p dialectica-core`): 1094 + 30 tests pass, none skipped or
ignored.

**Whether the withheld per-Stoa slate path is abusable now that it is
unreachable from the view.** `generateIdentitySlate` and `keepIdentity` stay
callable on the wire (`design.md` D2, deliberate) and `keepIdentity` can still
mint the machine keystore file on a fresh install exactly as before this
change — that side effect is pre-existing and not introduced here. What
changed is that nothing downstream (`posting_identity`, `publishing_key`,
`whoami_for`) consults what a keep records any more, so a caller reaching
these two methods directly (bypassing the withdrawn UI route) can at most
write an ignored `chosen_paths` row or (on the very first call) mint the same
one machine key `createIdentity` would have minted — it cannot select a
different signing identity, cannot make posting depend on a per-Stoa choice
again, and cannot desynchronise the probe/report/publish from each other. Grepped
the QML tree (`git grep -n "DOnboardingScreen" -- dialectica-ui/src`) and
found only comments and the `qmldir` registration — no remaining
instantiation anywhere in the view, confirming the screen that used to drive
these two methods is genuinely unreachable from the UI, not merely hidden
behind a flag.

## Environment note

Ran entirely within the shapes the brief allows (`git ls-files`, `git grep -n
-F`, `Read`, plain `cargo test`, one `Edit` + revert for the mutation). No
check was skipped for tooling reasons.
