# core-e2e — security

Reviewed at `8bfe77d` (the piece tip), **not** at `9bb2bc1` — the dispatch pointed
at `origin/piece/core-e2e`, nine commits behind. `TempDir::new` is byte-identical
across those nine commits, so both entries below hold on the current tree; I
re-read it at `8bfe77d` to confirm rather than assuming.

This target is test code, so the threat model is narrower than the crate's: nothing
here is reachable from a peer. What it *can* do is write a real Ed25519 root secret
in the clear to a real file in a shared, world-writable directory, which is what
both entries are about.

- [ ] **`tester`** — `end_to_end.rs:276-288` — `TempDir::new` builds a **fully
      predictable path** in the shared temp directory and then writes an
      unencrypted keystore root secret into it, where **the same crate's own
      in-crate fixture already uses 8 bytes of randomness for the identical job**.
      **Scenario:** the path is
      `${TMPDIR}/dialectica-e2e-<pid>-<fixed test name>`, and both components are
      knowable to any local user — the name is a literal in the source, the pid is
      readable from `/proc`. Two tests
      (`a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`,
      `the_same_keystore_posts_under_different_addresses_in_two_stoas`) then call
      `Keystore::generate().create(..)` inside it, writing a 32-byte root secret in
      the clear at `identity.key`. A local attacker who wins the window **between**
      the `remove_dir_all` on line 279 and the `create_dir_all` on line 280 can
      place a symlink at that name and capture the secret, or place a directory and
      make the test panic at line 285.
      **Measured:** I probed the single-actor cases and containment does hold —
      `std::fs::remove_dir_all` on a pre-placed symlink unlinks the symlink itself
      rather than following it (verified with a compiled probe: the victim
      directory and its contents survived, and the subsequent write landed inside
      the newly-created real directory, not in the victim), and a pre-placed real
      directory with a symlink child is wiped child-and-all. So the exploit
      requires **winning the TOCTOU race**, not merely pre-placing something. That
      is why this is low and not high — but the defence is accidental (a property
      of `remove_dir_all`, not a choice the fixture made) and the window is real.
      **The fix is already in the repo and needs no new dependency:**
      `keystore.rs:1319-1322`'s `TempDir::new` does
      `getrandom::fill(&mut [0u8; 8])` and hexes it into the directory name, which
      removes the race rather than narrowing it. `getrandom` and `hex` are already
      dependencies there. Diverging from the crate's own established fixture on the
      one path that writes a secret is the part worth acting on — and this repo's
      own recorded lesson is that an unfixed weaker pattern becomes the template
      the next one is written against.
      **Severity:** low.

- [ ] **`tester`** — `end_to_end.rs:279` — the unconditional `remove_dir_all` at
      the top of `TempDir::new` recursively deletes a caller-owned tree if the
      predictable name ever collides, and discards its error (`let _ =`).
      **Scenario:** pids are recycled. Any directory that happens to sit at
      `${TMPDIR}/dialectica-e2e-<pid>-<name>` — a leftover from an aborted run, or
      something unrelated — is removed recursively with no check that this fixture
      created it and no report if the removal fails. Randomising the name (the
      entry above) closes this too: a name nothing can predict is a name nothing
      else occupies, which is why these are two boxes rather than one and both are
      discharged by the same edit.
      **Severity:** low — a hazard to a developer's machine rather than to the
      product, but it is a recursive delete of an unverified path.

## Verified clean — no action needed

**Containment within the temp directory holds.** Every path the fixture writes is
built by `TempDir::file`, which is `self.0.join(name)` over a fixed set of literal
basenames. None is derived from test data, none contains `..` or an absolute
component, and nothing is taken from an environment variable other than
`std::env::temp_dir()` itself. `store_at`/`reopen_at` take a filename parameter as
of `1342aa9`, and every caller passes a literal — so the widening did not open a
path-traversal route.

**Leftovers are cleaned on both the pass and the fail path.** `Drop::drop`
(line 333) removes the tree, and because a panicking `#[test]` unwinds by default,
a *failing* test cleans up too. Only a `SIGKILL` or a `panic = "abort"` profile
would leave a directory behind, and neither is in play. (This is also why the doc
comment's "a failure leaves one identifiable directory" is wrong — filed under
correctness, since it is a false claim rather than a security gap.)

**No cross-test contamination, and no order dependence.** Each `TempDir` embeds a
distinct per-test string, so the directories are disjoint by construction rather
than by scheduling. `--test-threads=1` passes the whole target, so no test depends
on another's leftovers in either direction — an integration suite that passes only
in one order is the failure this checks for, and it is not present.

**No secret reaches an assertion message or a panic payload.** The tests that
handle a root secret assert on derived *addresses* (`stoa_address(..).to_hex()`),
never on key material, and the `expect` strings are fixed prose. The one place a
secret-adjacent value could have leaked — the `{other:?}` in the over-cap test's
panic — formats an `OpLogError`, which carries no key bytes. The crate already
pins this property for itself
(`keystore.rs::no_error_message_carries_key_material_or_a_passphrase`, passing).

**Directory mode is set before the secret is written.** `set_permissions(0o700)`
on line 284 runs inside `TempDir::new`, so it precedes every `Keystore::create`.
There is no window in which the directory holding a keystore is group- or
other-writable *after* the secret lands. (The comment attributing the requirement
to `Keystore::create` rather than to `Keystore::open` is wrong, but the ordering it
produces is correct — filed under correctness.)

**Fixture-only cryptographic shortcuts are appropriate and contained.** `a_key`
uses a fixed 32-byte seed via `SecretKey::from_bytes`, right for a reproducible
test and reaching no shipped path; the real `Keystore::generate()` is used wherever
a keystore file is actually written, so the atomic-write and permission logic under
test is the shipped logic.

**The hostile-input fixtures are the right ones and stay inside the sandbox.** The
forgery, the forged unhide, the not-a-database file, the foreign layout version and
the mislabelled store all construct attacker-shaped input and assert the reader
refuses it — including that the log *stores* the forgery, which is the §3.3
division of labour. The over-cap body is the one case where hostile input wins, and
it is filed as a known defect rather than hidden.

**No new dependency.** The target uses `rusqlite` (already a dependency of the
crate under test) for `rusqlite_stamp`, and deliberately avoids a `tempfile`
dev-dependency — consistent with the reasoning recorded at `keystore.rs:1311-1315`.
Nothing to review for licence compatibility.
