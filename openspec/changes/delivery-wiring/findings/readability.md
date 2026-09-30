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

- [x] **`tester`** — `delivery/tests.rs:1681` — "One case per refusal the boundary makes"
      is six cases of seven.
      **Scenario:** `cases` is `[(..); 6]`; `Storage` is the seventh refusal and is
      covered by `an_op_log_that_will_not_open_is_a_storage_refusal` (`:541`), not
      here. The comment, then the assertion that `distinct` names equal `names`,
      reads as covering all names, and the table would stay green if a future refusal
      were added to `refusal_kind` and not to this table.
      **Fix shape:** say "one case per refusal reachable without breaking a store;
      `Storage` is `an_op_log_that_will_not_open_is_a_storage_refusal`'s". Nit.
      **Severity:** nit.
      **Outcome (`tester`): fixed, by the other route:** the table now has the
      seventh case. `every_refusal_is_logged_under_its_own_name_and_none_carries_what_the_sender_chose`
      takes a fourth column, "break the op log first", and `storage` is a row
      (matched by the prefix `refused (storage`, since that line carries this
      peer's own error text). The comment says the table is the seven
      `refusal_kind` names. What the table still cannot do is notice an eighth
      `InboundRefusal` variant: `refusal_kind` is exhaustive without a wildcard,
      so a new variant forces a name there and not a row here. Also strengthened in
      passing: every row's payload, channel and sender now carry the marker
      `zzyzx`, and the test asserts each payload really contains it, then that no
      line contains it as text, as hex or as a decimal byte list. Mutation: the
      processor's fallthrough refusal arm logging the payload
      (`Some(&String::from_utf8_lossy(..))`) for every kind except
      `unknown-channel`. Predicted red at the first row that echoes, observed red
      at `too-long`. (The older `a_refusal_is_logged_by_kind_…` tests one kind,
      `unknown-channel`, which the mutation leaves alone, so it cannot see it — by
      construction, not run; that is the gap the table closes.)

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

## Re-review round 1 `7a2a3335..369561d1`

Dimension: readability only. Reviewed at `f37c6cc4` (piece tip; the range's code is
unchanged after `369561d1`): `delivery.rs`, `transport.rs`, `wire.rs`, `sender.rs`,
`delivery/tests.rs`, the two spec deltas, `proposal.md`, `design.md`, `tasks.md`
and `CLAUDE.md`'s five new trap entries. The nine entries of round 0 are all
confirmed fixed (see "What I checked"). Five new defects, all low, all in prose or
comments; none changes behaviour.

- [x] **`dev-writer`** — `delivery.rs:929-930`, `design.md:603` — "(logos-protocol 0.9)"
      names a version that does not discriminate the thing it is offered as the
      test for.
      **Scenario:** `listen`'s doc says `recv()` fails on a dead provider "on a
      runtime with the status channel (logos-protocol 0.9)", and the trap table in
      `design.md` says the same. A reader checking "do I have the status channel?"
      looks for 0.9. `logos_protocol.h` at `4638634`, the rev `dialectica/flake.lock`
      pins for `logos-protocol` (`git -C ~/src/logos-co/logos-protocol show
      4638634:cpp/logos_protocol.h`, lines 270-282), says the opposite: "Both 0.9
      cuts report MINOR 9, so `MINOR >= 9` is true of a protocol that has these
      four symbols and of one that does not. Guard on this instead:
      `LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE`", "Absent ... on the first
      cut of 0.9". So a runtime at 0.9 can lack the channel, and on it `recv()`
      parks forever, which is the leaked-thread case the same sentence describes.
      **Fix shape:** name the feature, not the number: "a runtime that defines
      `LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE` (the symbol
      `lp_client_set_subscription_status_cb`)", in both places. The number was
      also in round 0's code; I did not run it down then and should have.
      **Severity:** low (a comment, but it is a check a reader is told to make).
      **Fixed** (`dev-writer`) in the commit `Bound the wait on a pending open
      once per open, not once per message`. Re-read first: `git -C
      ~/src/logos-co/logos-protocol grep -n SUBSCRIPTION_STATE 4638634 --
      cpp/logos_protocol.h` finds the guard defined at `:282` and "Absent … on
      the first cut of 0.9" at `:280`, and `dialectica/flake.lock:9179` pins
      `463863446894…`. `listen`'s doc and the design.md trap row now name the
      feature (`LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE`, symbol
      `lp_client_set_subscription_status_cb`) and say why "0.9" does not
      discriminate.

- [x] **`dev-writer`** — `design.md:537` — Decision 15 quotes Decision 3 in words
      Decision 3 no longer contains.
      **Scenario:** Decision 15 says the old locking behaviour was "contradicting
      Decision 3's 'a mutex held for map lookups'". `git grep -F "map lookups"
      openspec/changes/delivery-wiring/design.md` finds only that line: Decision 3
      (`:84-86`) was rewritten this round to "share only the channel book's mutex,
      which is never held while a payload is decoded ... (Decision 15)". A reader
      following the quote to Decision 3 finds no such words, and Decision 3 now
      points back at Decision 15 for its own justification, so each cites the
      other for what the other does not say.
      **Fix shape:** drop the quotation marks and say what Decision 3 claimed
      ("Decision 3's claim that the book is only ever held for map lookups"), or
      cut the clause: Decision 15's next sentence already states the defect.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`, the first shape: the quotation is gone, and
      Decision 15 now says what Decision 3 claims ("that nothing the event loop
      or the worker needs is held while a payload is decided"), which is what
      Decision 3 says once its own box below is fixed.

- [x] **`dev-writer`** — `design.md:84-86` — "share only the channel book's mutex" is
      false as written, and it is the sentence Decision 3's event-loop argument
      stands on.
      **Scenario:** Decision 3 argues the event loop never waits on the worker
      because "the processor, the listener and the worker share only the channel
      book's mutex". Two other locks are shared: the listener and the processor
      share `InboundQueue.waiting` (`offer` and `take`, `delivery.rs:875,888`), and
      the **dispatch thread itself** takes the book's mutex, in
      `Channels::opening` (`delivery.rs:539`) from `Delivering::joined` and
      `startup_opens`. The second is the one the argument needs: it is why "the
      book is never held across a decision" (Decision 15) is what keeps the event
      loop from waiting, not merely the worker's opens. The prose omits the
      dispatch thread from the sharers, so a later change that holds the book
      across a slow step reads as safe for the event loop.
      **Fix shape:** "the dispatch thread, the worker, the listener and the
      processor share the channel book's mutex, and the listener and processor the
      queue's; neither is held while a payload is decoded, verified or appended".
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`: Decision 3 now names all four users of the book
      (the dispatch thread first, and where it takes the lock), the two users of
      the queue's mutex, that neither is held across a decision, and that the
      dispatch thread being a user is why the rule protects replies. The
      design-review box on the same sentence adds the one wait a reply can meet,
      and Decision 3 has that too.

- [x] **`dev-writer`** — `transport.rs:593-594` — a comment in `admit` names two
      variables `admit` cannot see.
      **Scenario:** after `receive` was split, `admit(judged: Judged, log)` has no
      `message`. Its comment, carried over unchanged, says "`message.timestamp` and
      `message.sender_id` reach nothing here, by design". In `admit` they are not
      in scope, so the sentence reads as describing a variable that does not exist;
      the property is now stronger and different (a `Judged` holds only the
      `SignedOp`, so nothing else *can* reach the append), and it is true in
      `judge`, where `message` is in scope and the two fields are never read.
      **Fix shape:** "`Judged` carries only the op, so neither the event's timestamp
      nor its sender identifier can reach the append, by design", or move the
      sentence to `judge`.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Put the processor through the
      boundary's one spelling`, with the first fix shape's words: "`Judged`
      carries only the op, so neither the event's timestamp nor its sender
      identifier can reach the append, by design".

- [x] **`dev-writer`** — `design.md:178,316,320,674` — four pointers to provenance
      that will not resolve once the change is archived.
      **Scenario:** `tasks.md:16` has the closer delete `findings/` before archive,
      and `design.md` is archived with the change. Line 674 ends "(`findings/security.md`)",
      naming a file the closer removes. Lines 178, 316 and 320 give a mutation
      result as "(the mutation list in PR #190's first pass)" / "(PR #190's first-pass
      mutation list)"; that is the PR body, which is edited until merge (and
      `tasks.md:56` has the same pointer, "listed in the PR body"). A future reader
      of the archived design sees "turned three tests red" with no source they can
      open. Each result is stated in the sentence, so only the parenthetical
      fails; three of them (`declined`, the bound check, the window) are
      re-derivable by the one-line mutation each names.
      **Fix shape:** drop `findings/security.md` from `:674` (the sentence stands
      without it), and either restate each mutation result as a command a reader
      can run or mark it "as measured in PR #190 at the time, not re-run". The
      rest of the tree already carries such pointers (`membership.rs:1114`,
      `stoa.rs:1079`), so this is a defect of degree, not a new one.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Bound the wait on a pending open
      once per open, not once per message`, by re-running rather than
      re-labelling. Each of the three mutations now stands in `design.md` as the
      one-line change and what it turned red on this round: `declined` ignoring
      `error` turns twelve delivery tests red (Decision 6; the first pass's
      "three" had gone stale as tests were added), `>=` → `>` in
      `InboundQueue::offer` four (Decision 10), and `now_ms` read from
      `message.timestamp` in `Processor::pass` 27 (Decision 10; "seven" before).
      Each is marked as a count that grows with the suite. `findings/security.md`
      is gone from Open Questions, which now points at `proposal.md`'s seventh
      question, and `tasks.md` 7.2 points at the decisions instead of the PR body.

### What I checked, and found clean

- **Round 0's nine entries are fixed as recorded:** `wire::panic_detail` is the one
  renderer (`guarded`, both sinks, the three delivery call sites); the module doc
  says "Four threads" over four bullets; `refusal_kind`'s doc says "a new refusal
  forces a name here"; the refusal table has seven rows with `storage`;
  `Note::AlreadyStored` exists and the test asserts one "stored" and one "already
  held" line; the processor-speed claim reads "has **not been measured**" in
  `delivery.rs:815` and `design.md:308`; `Stores` names the thread per file;
  `start`'s doc and early-return comment say why `true`; `create_schema` in
  `sender.rs` carries the single-opener note.
- **Claims a comment makes about a mutation, run.** Each "red without it" was made
  true by a one-line mutation in this tree, restored with `git checkout`:
  `await_settled` returning at once turns eight delivery tests red, including
  `a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` and
  `a_poisoned_channel_book_still_waits_for_this_channels_open`;
  `wait_timeout_while` in its place turns the poisoned-book test red (the two
  other wait tests I ran with it stay green, as the comment implies);
  `refused_on_hand_over` asking the size first turns
  `an_oversized_payload_on_an_unknown_channel_is_refused_as_an_unknown_channel` red
  (`TooLong` where `UnknownChannel` is expected); `>=` in `refuse_oversized` turns
  `a_payload_at_the_limit_waits_its_turn` and
  `transport::a_payload_at_the_limit_is_not_refused_for_its_size` red;
  `listen` returning after a caught panic turns
  `a_panic_reading_one_event_does_not_end_reception` red; moving
  `startup_opens` after `subscribe()` turns
  `a_restarted_peer_keeps_what_delivery_hands_over_before_startup_asks_for_its_channel`
  red in round 0 on both runs I made; and a zero busy timeout in `SqliteOpLog::open`
  turns both `two_connections_*` tests red with "database is locked", which round
  0 left unverified. All 172 `delivery::`, `sender::` and `transport::` tests pass
  at the tip.
- **Sources cited in comments, re-read.** `channel already exists: ` at
  `channel_lifecycle.nim:43` and the `"ChannelCreate failed: "` prefix at
  `channel_api.nim:24`, both at `bfdb5afd`, which `dialectica/flake.lock:1086`
  pins; cluster 2 for both presets at `4a85db1b` (`networks_config.nim:74,103`);
  `" callback timeout"` built at `api_call_handler.h:159` and the `RET_STALE_WARN`
  early returns at `api_call_handler.h:80,99` and `delivery_module_plugin.cpp:99,112`
  at `v0.2.1`; `Context not initialized` in the decline envelope
  (`PHASE0-FINDINGS.md:579`); "marshal onto it and block until it answers"
  (`logos_protocol.h:41`); `9f420c2` is the pinned builder (`dialectica/flake.nix:40`);
  `"≈ 400×"` and `Discarded(1)` are in `findings/security.md:26,60`. 256 x 150 KiB =
  38,400 KiB = 37.5 MiB and 1024 x 150 KiB = 153,600 KiB = 150 MiB check by hand.
- **Every test name a doc comment or `design.md` decision cites exists** (14 in
  `delivery.rs`, checked with one `git grep -F -e`).
- **`CLAUDE.md`'s five new entries against "Keeping this file true".** Each says why
  and what a command cannot, names the version it was read at ("at v0.2.1"), and
  the first says "Re-check when the pin moves", so all are self-invalidating; none
  records a count, a version of the repo, or merge state. The `declined` example
  envelope is `PHASE0-FINDINGS.md:579`'s. `cfg(logos_scaffold)` and the "only gate
  that compiles it" claim match `lib.rs` (unchanged in this range).
- **Spec deltas and `proposal.md`** agree with the code on each behaviour I traced
  (Decisions 10, 11, 14, 15 against `delivery.rs`; the scenario names quoted in
  test comments against the delta headings). MUST/SHALL stays uniform inside each
  requirement, as round 0 found.
- **Formatting.** `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml --check`
  is clean; the `dialectica-core` crate alone reports only `identity.rs`, which this
  range does not touch.

### Not boxes (taste; say so and move on)

- `delivery.rs:38` is the one over-long comment line in the three source files
  (about 125 columns among 80-column neighbours), an edit residue: the `catch_unwind`
  paragraph was extended in place. `delivery.rs:1062-1067` and `proposal.md:22,37-38,65-66`
  have the opposite artefact, a line broken short mid-sentence.
- `design.md` Decision 11 states the cost of the wait three times (`:389-398`,
  `:416-418`, `:433-435`) plus Decision 10's cross-reference; each is accurate, and
  one paragraph would carry it.
- The invariant "the book is never held across a decision" lives on
  `Channels::stoa_of` (`delivery.rs:601-610`) although it governs every method; it is
  found from two cross-references (`refused_on_hand_over`, Decision 15), so it is
  discoverable, but `ChannelBook`'s own doc is where a reader of the type looks.
- Several test comments open "Security review:", "Architecture review:",
  "Readability review:" (`tests.rs:1656,1678,2987,3024,3260`). Each carries its own
  reason, so nothing is lost when `findings/` is deleted; the label is what will
  read oddly. `tests.rs:1678` also names `Channels::receive`, a method that no
  longer exists (it is the defect being described, in the past tense).
- `tests.rs:1315-1317` breaks the test name `sender::tests::nothing_a_sender_...`
  across a comment line, so a `grep` for the name misses the citation.
  `sender_supplied`'s doc (`tests.rs:1266`) says "a worker over `dir`"; its parameter
  is `peer`.
- Mutation testing was run by hand on the claims above, not with `cargo mutants`;
  this is the readability lane. No mutation is left in the tree: every one was
  reverted with `git checkout -- <file>`, and `git status` shows only this file.

## Re-review round 2 `369561d1..2cb71aaf`

Dimension: readability only. Reviewed at `0a8f8639` (the code is unchanged after
`2cb71aaf`; later commits are findings): the range's diff of `delivery.rs`,
`transport.rs`, `arrival.rs`, `delivery/tests.rs`, `design.md`, `proposal.md`,
`tasks.md` and the two spec deltas. Round 1's five entries are all confirmed
fixed (see below). Four new defects, all low, all in prose or comments; none
changes behaviour.

- [x] **`dev-writer`** — `design.md:493-496` — the argument that rejects "a limit sized
      for a queue of opens" rests on a growth claim that is false, and the paragraph
      above it says why.
      **Scenario:** the text says a queue-sized limit makes one stuck open hold every
      Stoa up for up to (k+1) x 35 s, "and startup's K opens, each held in turn, grow
      as the square of K where 40 s grows as K". The waits are not additive, because
      the opens settle one after another on one wall clock: with an unresponsive
      delivery open j settles at (j+1) x 35 s, the processor reaches message k when
      message k-1's wait has ended, and message k then waits only until open k
      settles. Take K = 20, one message on each channel at t = 0, a limit that never
      expires: message 1 is decided at 70 s, message 2 at 105 s, ..., message 20 at
      735 s. The total stall is 21 x 35 = 735 s, linear in K, not a sum of
      (k+1) x 35 (which is 35 x (2 + ... + 21) = 35 x 230 = 8,050 s). The text's own
      lines 490-493 say the same thing for the per-message shape ("the worst case
      was the pending time under either limit"). Conversely, K x 40 s
      (`:471`, `:757`) is an upper bound the wall clock caps: at K = 20 it is 800 s
      against 735 s of pending time, so "can be held up K x 40 s in all" is not
      reachable once K > 7 (K x 40 > (K+1) x 35 when 5K > 35).
      **Why it matters:** this is the stated reason the alternative was rejected;
      the decision may still stand (40 s caps what a sender can do to one open, a
      queue-sized limit caps it at the open's queue position), but not on "square
      versus linear".
      **Fix shape:** say what the two limits actually differ in (per-open ceiling of
      40 s against a ceiling that grows with queue position, with the total for K
      opens bounded by the last open's pending time either way), or drop the
      comparison of growth rates.
      **Severity:** low (a design argument, not behaviour).
      **Fixed** (`dev-writer`) in the commit `Re-argue why the settle limit is
      40 s, and stop counting what grows`, along the first fix shape. Decision 11's
      "What still bounds the stall" is now two bounds, the smaller holding: one
      `SETTLE_LIMIT` per open, and the last open's settle, with your K = 20 working
      (735 s, not 800 s; K × 40 s reachable only while K ≤ 7, from 5K ≤ 35). The
      rejection records the square-of-K claim as withdrawn with your reason (each
      wait ends at a fixed instant on one wall clock), and then says what the two
      limits differ in: how far one stuck open can hold every Stoa, what 40 s
      loses, and that the spec asks for a fixed time. The Risks line (`:757` then)
      says "up to K × 40 s …, never past the last of them settling", the spec's
      `de35f026` wording, and `SETTLE_LIMIT`'s doc says the same. The value was
      re-decided and kept; see the security entry for the same box.

- [x] **`dev-writer`** — `design.md:193`, `design.md:339`, `tasks.md:84` — two mutation
      counts written as "re-run on this change's last round" are stale at the tip,
      the failure round 1's fifth entry was about.
      **Scenario:** `git checkout`-clean `2cb71aaf`, mutation as the text states it.
      (a) `declined` ignoring the `error` field (`.filter(|r| !r.is_empty())` replaced
      by `.filter(|_| false)` on the `callee_error` read at `delivery.rs:148`): the
      text says twelve delivery tests red; **13 fail** (the twelve plus
      `what_delivery_says_of_a_channel_decides_whether_the_channel_opens_and_a_post_is_sent`).
      (b) `now_ms` read from `message.timestamp` in `Processor::pass`
      (`message.timestamp as u64`, the `i64` needing a conversion to compile): the text
      says 27 red, with no "grows with the suite" hedge; **29 fail**, run twice, the
      extra two being that same test and
      `a_message_waits_for_the_last_of_two_requests_for_its_channel`. Both tests were
      added in `a7d0beaa`, after `c96fecae` wrote the counts and re-ran them.
      The other two hold: `>=` to `>` in `InboundQueue::offer` is four red, and the
      bound removed (the wait's `left` fixed at an hour) is six red, the six the text
      names. CLAUDE.md, "Keeping this file true": a number a command answers goes
      stale; it already did, within the round that wrote it.
      **Fix shape:** drop the two numbers (keep "the mutation, and that the named
      tests go red"), or qualify as "at least"; do not re-run and re-write, since the
      next test added repeats this.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Re-argue why the settle limit is
      40 s, and stop counting what grows`, by the first fix shape, and for all
      four mutations rather than the two that had gone stale: Decision 6, both
      Decision 10 claims and Decision 11's bound-removed claim now say the
      mutation and name the tests that go red, with no count and no "re-run on
      this change's last round" label (Decision 6 says why a count is left out).
      `tasks.md` 10.4 points at Decision 11's named tests. The named tests were
      re-run on this tree, each mutation reverted after: `.filter(|_| false)` on
      the `callee_error` read reddens `declined_reads_delivery_s_three_shapes_of_no`
      and every "already exists" test (13 red in all at this tree, the count you
      measured); `now_ms` from `message.timestamp`
      reddens `the_window_is_judged_by_this_peers_clock_not_the_events_timestamp`;
      `>` in `InboundQueue::offer` reddens the three tests Decision 10 now names
      (a fourth, `every_discard_is_counted_and_logged_apart_from_refusals`, also
      goes red and is not named); the wait's `left` fixed at an hour reddens the
      six tests Decision 11 names.

- [x] **`dev-writer`** — `delivery.rs:606-611`, `design.md:125`, `design.md:483` —
      "neither value is visible to a test" is false since `a7d0beaa`, and the comment
      overclaims what the compile-time asserts hold.
      **Scenario:** `delivery/tests.rs:479`
      `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
      reads `CALL_TIMEOUT` and `SETTLE_LIMIT` and asserts the order against a literal
      30 s. It exists because the two `const _` asserts compare the constants with
      each other and with `DELIVERY_CALLBACK_TIMEOUT`, so lowering
      `DELIVERY_CALLBACK_TIMEOUT` to 10 s with `CALL_TIMEOUT` at 20 s compiles and only
      the test goes red (the tester's measurement, `findings/correctness.md`). The
      comment beside the asserts says "Neither value is visible to a test ... so a
      build that breaks either relation fails to compile"; the relation the docs rest
      on is delivery's real 30 s, and a build that edits the named constant breaks it
      and compiles. `design.md:125` ("No test can see this constant") and `:483`
      ("No test can see either value") say the same. The test is named nowhere in
      `delivery.rs`, `design.md` or `tasks.md` (`git grep` finds it only in `tests.rs`
      and the findings), so the one guard for the literal is undiscoverable from the
      place a reader is told there is none.
      **Fix shape:** at `delivery.rs:606-611` say the asserts hold the two constants
      against each other and the test holds them against delivery's own 30 s, naming
      it; in `design.md` say "no test waits out either value" rather than "sees", and
      cite the test in Decision 4 and `tasks.md` 10.4.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Re-argue why the settle limit is
      40 s, and stop counting what grows`, as the fix shape says. The comment
      beside the asserts says they hold the constants against each other, that
      lowering `DELIVERY_CALLBACK_TIMEOUT` with `CALL_TIMEOUT` compiles, and names
      the test that holds both against the literal 30 s; `CALL_TIMEOUT`'s doc
      names it too. `design.md` Decision 4 and Decision 11 say "no test waits it
      out" and cite the test, and `tasks.md` 9.4 and 10.4 are corrected the same
      way. Re-run on this tree: `DELIVERY_CALLBACK_TIMEOUT` at 10 s with
      `CALL_TIMEOUT` at 20 s compiles, and the test goes red ("CALL_TIMEOUT (20s)
      does not outlast delivery's own 30s").

- [x] **`tester`** — `delivery/tests.rs:2205` — a comment cites a test by a name that is
      a prefix of the real one.
      **Scenario:** the comment in
      `many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`
      says "`an_opens_time_is_the_same_for_every_message_that_waits_on_it` pins the
      same property with no clock at all"; the function is
      `an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`
      (`:2256`). `git grep -F` on the cited name finds both the citation and the
      function, so nobody is lost, but `grep -w` or an IDE lookup of the quoted
      name finds no function.
      **Fix shape:** the full name.
      **Severity:** nit.
      **Outcome (`tester`): fixed.** The comment now gives the full name,
      `an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`,
      on a line of its own (it is too long to share one with the sentence under
      rustfmt's width). `git grep -F` for the full name finds the citation and the
      function. A comment, so no mutation applies.

### What I checked, and found clean

- **Round 1's five entries are fixed as recorded.** `listen`'s doc and the trap-table
  row name `LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE` and say why "0.9" does not
  discriminate; Decision 15 no longer quotes Decision 3 and Decision 3 names the
  four users of the book and the two of the queue; `admit`'s comment reads "`Judged`
  carries only the op ..."; `design.md` no longer points at `findings/security.md`
  or the PR body for any mutation (`git grep` for both finds nothing outside
  `findings/`). Fifth entry's counts: see the second box above, where two of the four
  re-run counts have already gone stale.
- **Mutations I ran for the new claims** (each restored with `git checkout -- <file>`;
  94 `delivery::` tests, all green unmutated): the wait's bound removed, six red
  as stated (the four named in the list and the two named beside them); `>=` to `>`
  in `offer`, four red as stated; a guard leaked on the no-sender return in
  `Worker::open` (`std::mem::forget(opening)`), red in
  `a_sender_identifier_that_cannot_be_retained_opens_no_channel` and
  `an_open_this_peer_gives_up_without_asking_delivery_does_not_hold_a_message_up`
  (timed out waiting for the refusal), as `design.md` says.
- **The per-open wait's prose matches the code.** `Pending::wait_ends`,
  `Channels::opening` (a request clears the time), `await_settled` (deadline read once,
  never re-read), the `ChannelBook` doc, `SETTLE_LIMIT`'s doc, Decision 11, the
  `op-transport` paragraphs and scenarios, and `proposal.md`'s wording say the same
  thing and quote the spec's phrases exactly ("for each open, not for each message";
  "Only a later create, join or startup asking for that channel lets a message wait
  on it again").
- **Arithmetic in Decision 11's "Why per open"**: 21 x 35 = 735 s; 735 / 40 = 18.4,
  ceiling 19; 256 x 40 = 10,240 s = 2 h 50.7 min, "about 2 h 51 min"; the probe's
  10 x 200 ms = 2.01 s is in `findings/security.md:246-248`. The rusqlite busy timeout
  claim (5 s, "nothing here sets it") holds: no `busy` call in `dialectica-core/src`.
- **Every test name cited in `design.md`, `tasks.md` and the new comments exists**
  (twelve checked with one `git grep -E`), apart from the prefix in the fourth box.
- **Cross-references to `judge`** after it became private: `arrival.rs:223`,
  `transport.rs` docs, `design.md` Decision 15 and `tasks.md` 10.1 all say private and
  called only by `receive_via`; no doc still links to it as public.
- **`cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml --check`** is clean.

### Not boxes (taste; say so and move on)

- `wait_ends` names three things: a field on `Pending`, a method on `Pending` that
  starts the time if it is absent (reads like a getter, takes `&mut self`), and a
  method on `ChannelBook` with a different signature. `start_or_get_deadline` on
  `Pending` would say what it does; the docs are clear enough that I did not box it.
- `arrival.rs:223-226` ends a line at "Rebuilding a store," after the rewrite, an edit
  residue like the ones round 1 noted in `delivery.rs` and `proposal.md`.
- `design.md:430` pairs a measurement at a 400 ms limit with "where the test allows
  under two limits", and the test now runs at 1000 ms; the sentence is true of both
  but a reader reproducing it at the test's limit gets about 4 s, not 1.61 s.
- Decision 15's "the boundary's ~55 tests" counts `receive(` call sites in
  `transport.rs` (54), not tests; it is approximate by its own mark.
- Mutation testing was by hand on the claims above, not `cargo mutants`; no mutation
  is left in the tree (every one reverted with `git checkout`).
