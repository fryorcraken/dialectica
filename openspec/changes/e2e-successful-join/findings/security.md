# Security review — `e2e-successful-join`

Scope: security dimension only, per the five areas named in the review brief
(fixed seed, committed genesis record as CI input, workflow matrix change,
`joinCancelButton`, and where the seeder writes). Diff reviewed:
`git diff 2bda577...HEAD` (piece tip `1102393f`).

## What was checked

**The committed fixed seed `[0x5e; 32]`.** No production code path treats
this seed, its derived public key, or the address `a4b3e43d…` as anything but
ordinary input. `git grep` for `a4b3e43d`, `CREATOR_SEED` and `0x5e` across the
tree (excluding `openspec/`) finds the seed only in
`dialectica-core/tests/seeded_reference.rs` and the address only in that file
and `dialectica-ui/tests/ui/seeded-join.yaml` — nowhere in `src/`, no
allowlist, no default config. The file lives under `tests/`, not `src/`, so it
is not compiled into the shipped library or app.

The publicity is stated in two independent places a future reader would hit:
the module-level doc comment ("The creator's secret is public, on purpose" —
anybody can sign as this Stoa's moderator, harmless because nobody posts in
it, must never be offered to a user as anything but a test) and design.md's
Risks section (same claim, same caveat). Both are clear and neither hedges.

I independently verified the underlying claim rather than trusting the
docstring: built the crate (`cargo test --test seeded_reference`, after
restoring the gitignored `logos-rust-sdk-src` symlink dropped by `git
worktree add` — a known worktree gap, not a defect in this piece) and all
three tests pass, confirming the committed address really is the hash of the
genesis record built from that exact seed, policy and title, and that
`get_stoa`/`join_stoa` really do accept it. I then mutated the committed
address's first hex digit in `seeded-join.yaml` (`a4b3…` → `b4b3…`) and reran:
all three tests fail, two on the core's own refusal
("the genesis record does not hash to the Stoa address it was given with")
and one on the record/address mismatch — matching design.md's D1 claim
exactly. I reverted with `git checkout -- dialectica-ui/tests/ui/seeded-join.yaml`
and reran clean (3 passed) before finishing. The drift-detection property
design.md claims is real, not asserted.

**The committed genesis record as a CI input.** It reaches only
`dialectica-core/tests/seeded_reference.rs` (via `Genesis::decode`,
`get_stoa`, `join_stoa` against in-memory stores) and, at the UI layer, the
paste field of a running Basecamp instance in `ui-tests.yml` — the same paste
path an ordinary user reference goes through. Nothing gives it elevated
trust; it is validated by the same `stoa-genesis` checks any pasted reference
gets, which the test confirms by exercising the real handlers rather than a
stub.

**The workflow matrix change.** `spec: [join, seeded-join, create, feed,
thread, moderation]` in `ui-tests.yml` is a static literal, not
attacker-influenced, and the existing job body already reads matrix values
only through `env:` rather than splicing `${{ }}` into script text (per the
comment directly above the diff). No injection surface.

**`joinCancelButton`.** A four-line addition of `objectName: "joinCancelButton"`
on the existing Cancel `FlatButton` in `DJoinScreen.qml`. No new binding, no
new capability, no change to what the button does — it only makes the
existing control addressable by the test harness.

**Whether the seeder writes anywhere outside a disposable test context.**
Confirmed by reading the constructors it calls: `SqliteOpLog::in_memory()`
(`log/sqlite.rs:219`) and `MembershipStore::in_memory()` (`membership.rs:348`)
both open with `Connection::open_in_memory()` — SQLite's `:memory:` backing
store, never a file. Nothing in `seeded_reference.rs` touches a path, a
profile directory, or `lgs`-managed state. Writing further than that would
have required the file to call `SqliteOpLog::open` or `MembershipStore::open`
against a real path, which it never does.

## Verdict

- [x] **none** — no security defect found in the areas reviewed. The fixed
      seed is real, deliberately public, and documented clearly in two places
      a maintainer would read before regenerating anything; the drift check
      that guards it was mutation-tested and caught the break; the seeder
      never leaves memory; the workflow and QML changes carry no new
      injection or authorization surface.
