# Readability findings: delivery-wiring

Reviewed at `7a2a3335` (`piece/176-delivery-wiring`), diff `origin/main...HEAD`.
Dimension covered: readability only (code, comments, spec deltas' prose,
`design.md`, `proposal.md`). Every entry below is a low-severity defect in what a
reader is told; none changes behaviour except entry 4's log wording.

- [x] **`dev-writer`** — `delivery.rs:330` — `panic_detail` is a byte-for-byte copy of
      `wire.rs:2631`, and `wire.rs`'s doc on its copy is now false.
      **Scenario:** `wire.rs:2626-2629` says the function is "Shared by the two sinks
      that contain a panic ... so the two cannot come to render the same payload
      differently." `delivery.rs:330-336` renders the same payload with an identical
      body, `wire.rs:71-75` (`guarded`) still spells it a third time, and three of
      `delivery.rs`'s callers (`:546`, `:823`, `:1035`) use the private copy. Changing
      the fallback text in one place (`"non-string panic payload"`, which
      `wire.rs:3620` also cites) leaves three renderings.
      **Fix shape:** one `pub(crate)` function in `wire.rs` (or a small shared spot),
      used by all three; then the doc is true. Defect, not taste: the sentence that
      claims the duplication is prevented sits beside the duplication.
      **Severity:** low.
      **Fixed** in `cd5d7c82`: one `pub(crate)` `wire::panic_detail`, called by
      `guarded`, both sinks and the delivery threads; its doc now names all of
      them. No behaviour change, so nothing goes red without it.

- [x] **`dev-writer`** — `delivery.rs:13` — heading says "Three threads" and lists four.
      **Scenario:** the bullets are dispatch, worker, listener, processor. The third
      bullet names two threads (`spawn_listener` and the processor's `spawn` are
      separate `Builder::spawn` calls at `:952` and `:955`), so a reader counting from
      the heading looks for a third thread and finds a fourth.
      **Fix shape:** "Four threads", or split the third bullet.
      **Severity:** low.
      **Fixed** in `89b3b552`: "Four threads", and the listener and processor are
      two bullets.

- [x] **`dev-writer`** — `delivery.rs:315` — "a seventh refusal forces a name here":
      there are already seven arms.
      **Scenario:** `refusal_kind` matches `UnknownChannel`, `TooLong`, `Undecodable`,
      `FailsVerification`, `StoaMismatch`, `AheadOfTime` and `Storage` (7), and
      `design.md` Decision 12 lists the same seven. The comment reads as if the next
      addition is the seventh; it is the eighth. `transport.rs:316` still says "six
      refusals", so a reader has two counts to reconcile and this comment picks a
      wrong third.
      **Fix shape:** "so a new refusal forces a name here". No number, so it cannot go
      stale (CLAUDE.md, "Do not write down anything a command can answer").
      **Severity:** low.
      **Fixed** in `89b3b552`, as shaped. `transport.rs:316`'s "six refusals" is
      in a doc this piece did not write and counts the spec's list, not the enum;
      left alone.

- [ ] **`tester`** — `delivery/tests.rs:1681` — "One case per refusal the boundary makes"
      is six cases of seven.
      **Scenario:** `cases` is `[(..); 6]`; `Storage` is the seventh refusal and is
      covered by `an_op_log_that_will_not_open_is_a_storage_refusal` (`:541`), not
      here. The comment, then the assertion that `distinct` names equal `names`,
      reads as covering all names, and the table would stay green if a future refusal
      were added to `refusal_kind` and not to this table.
      **Fix shape:** say "one case per refusal reachable without breaking a store;
      `Storage` is `an_op_log_that_will_not_open_is_a_storage_refusal`'s". Nit.
      **Severity:** nit.

- [x] **`dev-writer`** — `delivery.rs:866` — the log says "stored inbound op" for an op
      the log already held, and the test named for the opposite pins it.
      **Scenario:** `transport::Admitted` carries `appended: Appended`, "Whether the log
      already held it" (`transport.rs:426`). `decide` matches `Ok(admitted)` and logs
      `Note::Stored(&admitted.id)` without looking at it. Deliver the same op twice
      (a peer re-sending is ordinary under SDS repair): both times the module log
      reads `dialectica: stored inbound op <id>`. The second was not stored.
      `an_appended_twice_arrival_is_reported_already_present` (`tests.rs:1879`) asserts
      `with("stored inbound op").len() == 2`; nothing is reported "already present",
      so the test's name says one thing and its last line pins the other.
      #102 items 5-7 are checked by reading this log by hand, so the line is read as a
      fact.
      **Fix shape:** a `Note` variant for the already-held case (or make `Stored`
      say which), and rename or re-assert the test. If "stored" for a duplicate is
      the intended wording, rename the test so it does not claim a report that is
      never made.
      **Severity:** low.
      **Fixed** in `89b3b552`: `Note::AlreadyStored` reads "inbound op <id> already
      held; nothing new was stored" when `appended` is `AlreadyPresent`. The test
      keeps its name, which is now true, and asserts one "stored" line and one
      "already held" line naming the op; red first (two "stored" lines).

- [x] **`dev-writer`** — `delivery.rs:668`, `design.md:207` — "decides a typical op in
      about a millisecond" has no source.
      **Scenario:** it is the premise of the 256 bound ("so 256 waiting is a burst
      hundreds of messages deep arriving faster than SQLite appends"). No
      measurement, benchmark or test in the tree produces it, and each message opens
      the op log afresh (`decide` -> `stores.op_log()` -> `SqliteOpLog::open`, a
      `PRAGMA user_version` read plus `check_layout`) before verifying a signature
      and committing an append. I did not measure it either. CLAUDE.md: a number in a
      comment is a claim.
      **Fix shape:** measure it (one `Instant` around `decide` in a scratch test), or
      say "not measured" in both places, as Decision 7 does for the SDS reading.
      **Severity:** low.
      **Fixed** (the second shape) in `89b3b552` and the docs commit: both places
      now say the processor's speed per op has not been measured, and that 256 is a
      judgement. The per-message database open you describe is also gone for
      refused payloads (`3b72e546`).

- [x] **`dev-writer`** — `delivery.rs:351` — `Stores` says "The three files the delivery
      threads open"; one of the three is opened by no delivery thread.
      **Scenario:** `stores.memberships()` is called from `Delivering::start`
      (`:981`), on the dispatch thread from `on_context_ready`. `design.md` Decision
      13 states it exactly ("`stoas.sqlite` ... opened by one thread only (dispatch,
      ...)"), so the design and the doc disagree. The doc is where a later change
      will look before deciding whether a third thread may open `stoas.sqlite`.
      **Fix shape:** "The three files delivery's code opens: the op log (worker and
      processor), the sender identifiers (worker), memberships (dispatch, at start
      only)".
      **Severity:** low.
      **Fixed** in `89b3b552`, as shaped, pointing at Decision 13 for why each is
      safe.

- [x] **`dev-writer`** — `delivery.rs:974-978` — `start` returns `true` when the worker
      thread could not be spawned, with no comment and no doc for `true`.
      **Scenario:** the doc says only "A second call does nothing and returns
      `false`". On the `spawn` failure path the call has done half its job (listener
      and processor running, no worker, `outbox` left `None`) and still returns `true`
      from a bare `return true;`. The reader cannot tell whether `true` means "this call
      ran", "delivery is wired" or "the worker exists"; the last is false here, and the
      log then says "delivery wiring has not started" for every later request although
      `started` is set.
      **Fix shape:** a sentence on the return value ("`true` on the call that ran,
      whether or not every step succeeded") and a line at the early return saying
      why it is not `false`.
      **Severity:** low.
      **Fixed** in `89b3b552`: both sentences, and the state is now
      `Wiring::NoWorker`, whose requests log "the delivery worker could not be
      started" (the architecture review's third entry).

- [x] **`dev-writer`** — `sender.rs:187` — `create_schema` uses a plain `BEGIN` with an
      unlocked read of `user_version`, the exact shape `log/sqlite.rs` was just fixed
      for, and nothing beside it says why it is left.
      **Scenario:** this piece's own `two_connections_opening_a_fresh_store_at_once_both_open_it`
      exists because two connections opening a fresh store both read version 0 and
      both `CREATE`. `SenderStore::open` is `pub` and has the same window. The reason
      it is safe today is `design.md` Decision 13 ("opened by one thread only"),
      which is not on the function. A reader comparing `sender.rs` with `sqlite.rs`
      sees two fresh-store creations, one locked and one not, and no note.
      **Fix shape:** two sentences on `create_schema` or `SenderStore::open`: single
      opener by design, and the op log's `create_schema` is the model if a second
      thread ever opens it.
      **Severity:** low.
      **Fixed** in `89b3b552`, on `create_schema`, as shaped.

## What I checked and found clean

- **The fmt claim.** `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml --all
  --check` exits 1, as the tester reported. With `--message-format short` the
  files it reports are `dialectica-core/src/identity.rs` and sixteen files of the
  SDK under `/nix/store` (`logos-rust-sdk-src`); none is in
  `git diff origin/main...HEAD --stat`, so **this piece introduced nothing it
  reports**. The two gates that matter agree: `cargo fmt --manifest-path
  dialectica/rust-lib/Cargo.toml --check` (no `--all`, which is what `ci.yml:1285`
  runs) is clean, and `cargo fmt --manifest-path
  dialectica/rust-lib/dialectica-core/Cargo.toml --check` reports only
  `identity.rs`. The piece did reformat two unrelated `wire.rs` statements (the
  `record_failure` `let` near `:6323` and an `assert!` near `:17711`), both in the
  formatting-only "Make room" commit `6a69ce4c`; harmless, and they leave `wire.rs`
  clean under `--all`.
- **The MUST/SHALL mix.** Judged acceptable, no box. The repo's rule is MUST
  (`.claude/agents/spec-writer.md` "Keywords") and "requirement text moves
  verbatim". The live `op-transport` spec is SHALL (49 uses, 2 MUST) and the live
  `stoa-membership` spec is MUST (47 uses, no SHALL). The split the proposal states
  (`proposal.md:128`: MODIFIED keeps SHALL, ADDED uses MUST) means `stoa-membership`
  stays uniform and `op-transport` becomes mixed across requirements after
  archive, but **each requirement is uniform inside itself**: the new paragraphs in
  the two MODIFIED blocks are SHALL like their neighbours, and each ADDED
  requirement is MUST throughout. RFC 2119 treats them as synonyms, and keeping
  verbatim text verbatim is the stronger rule. The only cost is that a reader of
  post-archive `op-transport` meets both words with no difference in force.
  If the owner wants one word there, that is a separate sweep of a 49-use spec, not
  this piece's job.
- **Cross-references resolve.** Every quoted requirement or scenario name in the
  deltas (`Verification consults nothing but the two inputs` is a scenario, not a
  requirement, in `stoa-membership`, and the delta's phrasing does not claim
  otherwise), the test names cited in `design.md` and `tasks.md`, and the module
  paths in the doc comments exist. Every test name cited exists in
  `delivery/tests.rs` or `log/sqlite.rs`.
- **Claims I re-ran and that hold.** `logos.test` is cluster 2 with `maxMessageSize:
  "150KiB"` and `logos.dev` is cluster 3 with the default size
  (`logos-delivery/.../networks_config.nim:72-106`); delivery v0.2.1 has
  `CALLBACK_TIMEOUT{30}` (`delivery_module_plugin.h:298`), rejects a second
  `createNode` with "Context already initialized" (`:330`), stamps
  `currentTimestampNs()` at receipt (`:167`) and returns early on `RET_STALE_WARN`
  (`api_call_handler.h:80,99`); the staged SDK's `recv` polls at `DEAD_POLL` =
  200 ms and honours `Abandoned` (`plugin.rs:262,465`); #30 had 1024 slots, dropped
  the oldest, and used `Edge` on `logos.test` (`origin/core/transport`);
  256 x 150 KiB = 37.5 MiB and 1024 x 150 KiB = 150 MiB;
  `tests.rs:18`'s `NOW_MS` is 2026-09-18T11:01:44Z (checked by hand: 20714 days +
  39704 s = 1,789,729,304 s); `sender.rs`'s "no `unwrap`, `expect` or indexing"
  holds outside `#[cfg(test)]`; no `"ops.sqlite"` literal remains in the adapter.
  All 69 delivery, sender and log tests named above pass at this commit.
- **Prose in `design.md`, `proposal.md` and the two deltas** is consistent with the
  code on every decision I traced (Decisions 4-13 against `delivery.rs`, the
  `Note` wording against the scenarios' "records ... " clauses). The rejected
  alternatives are each stated with a reason.
- **Comments that earn their place.** The seam doc (why `stop` is missing), the
  `ChannelBook` race explanation, `SETTLE_LIMIT`, `CALL_TIMEOUT` and the
  `INBOUND_BOUND` derivation say what a command cannot. `lib.rs` comments name what
  moved and why `on_context_ready` is longer than one line.

## Not boxes (taste, say so and move on)

- `pub struct Stderr` in `delivery.rs` shares a name with `std::io::Stderr`;
  `StderrJournal` would read better at the adapter's one use.
- `a_clock_that_is_wrong_until_the_open_is_answered` (`tests.rs:1492`) is a helper
  named like a test sentence and reads a process-global `static`; a reader
  scanning for `fn a_...` expects a `#[test]`.
- `Peer::post` has a sibling publish-reply block copied twice
  (`tests.rs:1145`, `:1213`); a `Peer::reply` would remove it.
- Mutation testing was **not** run: the harness refused an edit to
  `delivery.rs` in my own worktree ("Modify Shared Resources"), so the
  "removing the wait makes X red" and "zero busy timeout turns both red" claims
  in `delivery.rs`, `design.md` and `sqlite.rs` are **unverified by me**, not
  disproved. No mutation of any kind is left in the tree.
