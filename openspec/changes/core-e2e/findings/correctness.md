# core-e2e — correctness

Reviewed at `8bfe77d` (the piece tip), **not** at `9bb2bc1`. The dispatch pointed
me at `origin/piece/core-e2e`, which is nine commits behind the local branch; I
re-ran every measurement against the current tree after discovering that. Where a
finding I had measured at `9bb2bc1` was already fixed by `5a1719b` or `1342aa9`,
it is recorded under "already fixed" rather than as a box, so nothing here asks
for work that is done.

Baseline measured in this worktree: `506 passed` (in-crate) + `24 passed`
(`tests/end_to_end.rs`) = **530**, matching `tasks.md` §7.1.

- [ ] **`tester`** — `end_to_end.rs:1041` — `a_hide_by_a_non_moderator_leaves_the_thread_visible`
      still dies at its fixture guard, so the resolution claim it is named for is
      never exercised. This is the **same shape the file's own note 1 diagnosed,
      fixed in only one of its two places**: the new test
      `the_moderator_set_of_a_genesis_record_is_exactly_its_creator` was added
      beside it, but the guard `assert!(!moderators.contains(&rando.public_key()))`
      was left sitting in front of the two assertions that carry the requirement.
      The readability and architecture passes did not touch these lines.
      **Scenario:** mutate `Moderators::contains` (`moderation.rs:190`) to return
      `true` unconditionally. `a_hide_by_a_non_moderator_leaves_the_thread_visible`
      goes red at line 1041, "the fixture's outsider must not be a moderator" —
      aborting before line 1045 (`moderation::resolve(..) == Ok(Unmoderated)`) and
      before line 1051 (the feed still lists `"survives"`). Nothing in this test
      ever observes what an unauthorised hide does to resolution or to the feed;
      the suite goes red for the right reason by accident, which is exactly what
      note 1 says it set out to stop.
      **Measured:** the mutation kills 2 of 24 (`22 passed; 2 failed`). With the
      guard deleted and nothing else changed, the same mutation fails the test at
      line 1041 on `left: Ok(Hidden(Entry { .. action: Hide .. }))` vs
      `right: Ok(Unmoderated)` — the substance of the requirement. So the deeper
      assertions **do** discriminate; the guard is the only reason they are not
      reached.
      The guard is not worthless — under a defect in `Moderators::authorises`
      rather than in `contains` it passes and the deeper assertions fire — so the
      fix is to move it **after** the resolution and feed assertions, not to delete
      it. Same remedy note 2 applied to the `Appended` values.
      **Severity:** medium — a genuine defect in the new file, of the precise
      family the file's header says it was shaped to avoid.

- [ ] **`tester`** — `end_to_end.rs:263-272` — the `TempDir` doc gives two reasons
      for the fixture's design and both are false against the code beneath them.
      Survived the readability pass unchanged. A reader trusting either will draw a
      wrong conclusion about the fixture, and one of them invites removing a line
      that is load-bearing.
      **Scenario (a):** "so a failure leaves one identifiable directory" —
      `Drop::drop` (line 333) calls `remove_dir_all` unconditionally and runs on
      unwind, and a panicking `#[test]` unwinds by default, so a **failing** test
      removes its directory too and leaves nothing to inspect. The stated benefit
      cannot be obtained as written.
      **Scenario (b):** "`Keystore::create` refuses a keystore whose containing
      directory is group- or other-writable" — `Keystore::create`
      (`keystore.rs:683`) checks only `path.exists()` and then writes. The
      directory-mode check `check_directory_mode` (`keystore.rs:1060`) is reached
      only from `read_checked`, i.e. from `Keystore::open` and
      `Keystore::is_encrypted`. So the `0o700` **is** load-bearing, but for the
      reopen in `a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`
      and for the `Keystore::open` in
      `the_same_keystore_posts_under_different_addresses_in_two_stoas` — not for
      `create`. Someone removing the `set_permissions` call on the strength of this
      comment breaks both keystore tests at `open`, and the comment tells them to
      look at the wrong call when they debug it.
      **Severity:** low as behaviour, medium as a claim — both halves are
      falsifiable statements about the code and both are false, which is the defect
      family `1342aa9` was specifically addressing elsewhere in this same file.

## Already fixed on this branch — no box

Two findings I measured at `9bb2bc1` are closed by `5a1719b`, which replaced the
three-root list with one rglob over `dialectica/rust-lib` excluding `target/`.
Recording them because the measurements agree and that is worth knowing:

- **The `examples/` paragraph stated its conclusion backwards.** At `9bb2bc1` a
  `#[test]` in an example was counted by neither `ran` nor `declared`, so the gate
  passed silently where the comment promised a loud failure. I reproduced this
  independently — added `dialectica/rust-lib/examples/probe_example.rs` with a
  `main` and one `#[test]`, then `cargo test --no-run` listed three test
  executables and **no binary for the example**. `5a1719b`'s commit message
  records the same measurement via `dialectica-core/examples/probe.rs`, and the
  rglob now makes the comment's claim true.
- **`dialectica/rust-lib/tests` was not a root.** I created
  `dialectica/rust-lib/tests/probe_gate.rs` with one `#[test]`; it compiled and
  reported `1 passed`, so cargo counted it in `ran` while `declared` ignored it —
  the gate's own documented failure mode, one directory over from the one the
  change fixed. The rglob covers it. Probe files removed; tree left clean.

## Verified as claimed — no action needed

**The `:memory:` mutation reproduces, and the survivors survive for the right
reason.** At `9bb2bc1` I measured **18 of 20** killed, exactly as the header
claimed, and confirmed *why* the two survivors survive rather than accepting the
count: both are pure-value tests that construct no `TempDir` and call no store
method, so there is nothing for the mutation to reach. They survive by being out
of scope, not by being weak. `tasks.md` §1.2 and §6.3 now report 21 of 24 on the
larger file, and §6.3 records that the split exposed a real weakness in the
past-the-end half — an empty page 99 is also what an empty store answers — which
is the same rival-explanation reasoning applied correctly.

**Note 2's fix is complete.** Under `INSERT OR IGNORE` → `INSERT OR REPLACE`,
`a_second_arrival_does_not_overwrite_the_first_ones_recorded_metadata` fails on
the recorded Lamport value (`Some(9)` vs `Some(1)`) — the claim the test is named
for — not on the `Appended` value. The reordering did what note 2 says.

**The forged-hide ordering guard is correctly placed**, despite sharing the
surface shape of the finding above. Applying the table's row-3 mutation (authority
checked after taking the leading `Moderate`) fails the test on `Unmoderated` vs
`Hidden(..)` — the named claim — not at the `iter_target` ordering guard. That
guard fires only under a store-level defect, where knowing the fixture's premise
broke is the more useful message. So the two guards are not the same case, and
only one needs moving.

**The boundaries are genuinely crossed.** `store_at`/`reopen_at` call
`SqliteOpLog::open` against a real path with no in-memory route available, and
`reopen_at` drops the connection before reopening, so "restart" is real. The
encode/decode seam is crossed for real in the over-cap test:
`SignedOp::from_bytes(&over.to_bytes()).is_err()` asserts the asymmetry in both
directions rather than only observing the decoder's limit. The keystore chain
reaches disk twice (`create`, then `open` from the path) and compares the feed's
author against an address re-derived from the reopened keystore, so the two halves
cannot agree with each other by construction. `1342aa9` extended the file through
`wire::list_threads_from_request`, so it now reaches the JSON boundary CLAUDE.md
calls the deliverable.

**The defect reproduction is accurate.** `MAX_FIELD_LEN` is `150 * 1024` =
153,600 (`op.rs:132`), matching the hardcoded `FIELD_CAP`. The at-cap/over-cap
pair makes it a fencepost test rather than an absurd-value one.

**Order-independence holds.** `--test-threads=1` passes the whole target, and
every `TempDir` name embeds a distinct per-test string, so the directories are
disjoint by construction rather than by scheduling. No test reads another's store.

## On pinning the defect rather than fixing it

Asked for a view, not a box. Written as it is, it is a characterisation test
rather than a trap: the assertion names the current outcome, the panic message
names what should replace it, and the at-cap half means a fix cannot pass by
loosening the cap. The one property a characterisation test needs and this had is
somewhere else to point — and `1342aa9` supplied it, since the test now says in as
many words that the defect is tracked nowhere and who should replace the
paragraph. A test that is the sole evidence of a bug is the wrong place for one,
and the file now says so itself.
