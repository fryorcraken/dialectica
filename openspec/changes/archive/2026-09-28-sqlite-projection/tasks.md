# Tasks

Three commits, each green on its own. The order is CLAUDE.md's "make the change
easy, then make the easy change", with the behaviour change isolated in the
middle so a reviewer can see it without a database in the diff.

## 1. The ownership refactor — no behaviour change

- [x] `log.rs` becomes `log/mod.rs`; its test fixtures move to `log/fixtures.rs`
      so a second test module can share rather than copy them.
- [x] `OpLog`'s reads return owned values: `Vec<Entry>` and `Option<Entry>`.
- [x] `MemoryOpLog::sorted` clones into its `Vec` rather than collecting
      references.
- [x] `CurrentVersion<'a>` → `CurrentVersion`; `Moderation<'a>` → `Moderation`.
      Both lose `Copy`, which the references were carrying.
- [x] `moderation::resolve` selects the deciding entry **by index** so it can be
      moved out of the candidate vector, and reads it back with `nth` rather
      than `[]` so a broken index cannot panic.
- [x] **Nothing weakened**: `resolve`'s three checks, its filter and its
      degraded-order `Hide` preference are byte-identical, as are
      `is_valid_revision`'s four conditions and their order.
- [x] Gates green on this commit alone: 349 tests (the same 349), clippy clean,
      `cargo fmt --check` reports 0.

## 2. The trait becomes fallible — one behaviour change, no database

- [x] `OpLogError`, with three variants: `Storage`, `UnknownLayoutVersion`,
      `CorruptEntry`. `Display` and `std::error::Error`.
- [x] Every `OpLog` method returns `Result<_, OpLogError>`; `get` returns
      `Result<Option<Entry>, _>` and **not** `Option<Result<..>>`.
- [x] `MemoryOpLog` returns `Ok` unconditionally, documented as the cheap half
      of a deliberate trade rather than a wart.
- [x] Both resolvers propagate it: `current_version` returns
      `Result<Option<CurrentVersion>, _>`, `resolve` returns
      `Result<Moderation, _>`.
- [x] `design.md` argues `Result` against a `Moderation::Unreadable` variant,
      and records **what a caller should do with the error** — there is no
      caller yet, and a `Result` nobody handles becomes `.unwrap()`.
- [x] Gates green on this commit alone: 349 tests, clippy clean, fmt 0.

## 3. `SqliteOpLog` — the second implementor

### The dependency

- [x] `rusqlite` with `bundled` only; `design.md` argues it against the keystore
      change's posture and records the four things refused.
- [x] `Cargo.toml` carries only what a reader editing that file needs — why
      `bundled` is load-bearing — and points at `design.md` for the rest.

### The schema

- [x] `op_id BLOB PRIMARY KEY`, chosen for **both** §3.1's idempotence and
      `cmp_ops`'s precondition, with `design.md` stating which came first —
      the `op-log` design was found claiming its map was chosen for a property
      it acquired by accident, and this does not repeat it.
- [x] `op_bytes` is `SignedOp::to_bytes()` verbatim; every other column is
      derived from it and is an index, not authority.
- [x] The sort columns materialise `cmp_ops`, with `sort_msg_present` as its own
      column because the **empty** message id is a legal value.
- [x] **`sort_ordered` is a separate leading column, not a sentinel.** The first
      draft reserved `i64::MAX` inside `sort_lamport`, and `lamport_sort_key(0)`
      is exactly that value — so a Lamport-0 op interleaved with the unordered
      ones. `u64` and `i64` have the same cardinality, so no sentinel inside one
      column can work.
- [x] **`arrival_lamport` / `arrival_msg` are separate from the sort columns.**
      They differ in exactly one case — an unordered arrival that nonetheless
      carries a message id — and collapsing them loses a fact the peer received.
- [x] Four indexes: ordering, by-Stoa, by-target, and the reserved
      `(target, author)`.
- [x] **§7.2 rule 5 answered**: `score_epoch` is a Lamport timestamp, `NULL`
      when absent, never a wall clock. **`NULL` and not `-1`**: `u64::MAX as
      i64` IS `-1`, so a `-1` sentinel made an unordered op and one ordered at
      the maximum store the same value. That was the first draft, it is the
      defect this change fixed, and it is spelled out here because this is the
      schema section a future reader consults first. See the Tests section's
      `score_epoch` entry and `sqlite.rs`'s column comment. **No `score`
      column** — a stored weight is unanswerable under per-reader vouching, not
      merely stale.
- [x] `PRAGMA user_version` checked on open; an unknown layout is **refused by
      name** in both directions, never read best-effort.
- [x] **The declared version is also CHECKED, not just compared.** A file
      stamped with our number whose `ops` table is missing or altered used to
      open `Ok` and fail every later read as `Storage("no such table: ops")` —
      an error blaming the disk for a mislabelled file. `check_layout` names
      every column a read or an `ORDER BY` touches and refuses as
      `LayoutDoesNotMatchItsVersion`, a variant distinct from
      `UnknownLayoutVersion` because "find the build that wrote this" is advice
      this case cannot act on.
- [x] `create_schema` **rolls back explicitly** on failure. Dropping the
      connection already rolled back, so the old code was sound by accident
      rather than by invariant. `PRAGMA user_version` stays LAST in the batch —
      it is the layout claim, and a crash before it leaves version 0, which
      reopens as fresh; stamping it earlier strands a file that `check_layout`
      then refuses forever.
- [x] `LAYOUT_VERSION` pinned by a hardcoded `assert_eq!`, because
      `cargo mutants` cannot see a wrong `const`.

### No panics

- [x] No `unwrap`, no `expect`, no `panic!`, no indexing in `sqlite.rs`'s
      non-test code. Every `rusqlite` failure becomes an `OpLogError`.
- [x] `len` uses `try_from` rather than `as`.

### Tests

- [x] `log/contract.rs`: 36 behaviours asserted of **both** implementations as
      plain generic functions called from two `#[test]`s each — **not** a
      macro, because `ci.yml` counts `#[test]` textually and a macro invoked
      twice would fail that gate for the wrong reason. Verified: 72 + 17
      attributes, 72 + 17 tests run.
- [x] `both_implementations_agree_across_every_ordering_branch`. **It caught a
      real divergence on its first run** — see `design.md`'s "what a real
      database made awkward".
- [x] The same for a target-restricted read, because `ops_by_target` is a
      different index whose column order could be wrong on its own.
- [x] `an_empty_message_id_is_recorded_and_is_not_absence`, in the contract
      suite so both implementations answer it.
- [x] Persistence tests (`SqliteOpLog` only): ops survive a reopen **in order**,
      dedup survives a reopen, the ordered/unordered distinction survives, and
      a persisted op is byte-identical with its signature intact.
- [x] Layout-version tests in **both** directions, plus the fresh-store
      boundary where `0` must be accepted rather than refused.
- [x] Boundary pair for the sort key at `0`, `1`, `u64::MAX - 1`, `u64::MAX`,
      asserted order-reversing **and injective**, not sampled in the middle.
- [x] **Prefix-confusion fixtures are CONSTRUCTED, not hunted.** The first
      version picked titles whose hashes agreed in byte 0 and guarded that
      coincidence — a review found 2-byte (`stoa`) and 8-byte (`target`) prefix
      matches passing the whole suite. `SHARED_PREFIX_BYTES` is now a named
      constant, the keys are built from raw bytes, and the guards pin that the
      prefix agrees **and** that the next byte differs.
- [x] **The superseded copies in `log/mod.rs` are DELETED, not left beside the
      corrected ones.** A second review re-ran the experiment: narrowing
      `MemoryOpLog::iter_stoa` to a 2-byte prefix still passed `mod.rs`'s
      `two_stoas_sharing_an_address_prefix_are_not_confused` and failed
      `contract.rs`'s. `mod.rs` is the file a reader opens first, and this
      project has a recorded hazard that a known-weak test is the template the
      next one is written against — so the weak copies were the defect, not
      merely redundant. `mod.rs` keeps only the two tests that are NOT trait
      behaviours and so have nowhere else to live: the two-log convergence test
      and `an_entry_reports_the_target_its_kind_names`. Every other name it held
      was verified present in `contract.rs` before deletion.
- [x] `every_ordering_shape` carries **differing-length, prefix-related**
      message ids. With all ids 32 bytes, length-vs-lexicographic could never
      disagree, and the headline agreement test could not catch a dropped
      `sort_msg`. It now can.
- [x] `score_epoch` is `NULL` for an unordered arrival, **never `-1`** —
      `u64::MAX as i64` is `-1`, so the sentinel collided. Caught by a review in
      a column no read consults, which is where an untested defect is cheapest
      to leave and most expensive to ship.
- [x] The `i64` sign boundary and the Lamport value whose sort key collides with
      the unordered filler are both asserted, so removing `sort_ordered` fails
      with a name pointing at the cause.
- [x] `EXPLAIN QUERY PLAN` asserted to contain no temp B-tree, closing a gap an
      earlier draft declared open rather than closed.
- [x] Mutation-verified. **Thirteen mutations, six survivors**, every one of
      them the "one step weaker" form; all now killed by tests watched failing
      first. The table is in `design.md`, survivors included.

## 4. Documents

- [x] `proposal.md`, `specs/op-log/spec.md` (a delta), `design.md`, this file.
- [x] Every `## MODIFIED Requirements` heading matches an existing requirement
      **verbatim** — grepped against `changes/op-log/specs/op-log/spec.md`, not
      remembered. An agent once filed a `MODIFIED` against a heading no
      requirement had, and `archive` would have applied nothing while reporting
      success. All three match.
- [x] PLAN.md §3.3 and §4.7 **pruned, not appended**: both said SQLite does not
      exist, and this change makes those corrections false.
- [x] PLAN.md §9's Phase 1 line records that the store half is done and names
      what remains under "query indexing" — the materialised view.

## 5. Sequencing, which is a precondition rather than a preference

- [ ] The `op-log` change must archive **before** this one. Its spec delta still
      lives in `openspec/changes/op-log/`, so the requirements this change
      modifies exist only there — archive this one first and the modifications
      apply to nothing, silently.
