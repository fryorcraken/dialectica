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

## Re-review round 3 `2cb71aaf..7462ded8`

Dimension: readability only. Reviewed at `ea37a706` (the range's code and prose are
unchanged by the findings commits after `7462ded8`): the range's diff of `delivery.rs`,
`delivery/tests.rs`, `design.md` (Decisions 4, 6, 10, 11 and Risks), `proposal.md`,
`tasks.md` and the `op-transport` delta. Round 2's four entries are all confirmed
fixed (see below). Three new defects, all low or nit, all in comments or prose; none
changes behaviour.

- [x] **`tester`** — `delivery/tests.rs:2299` — "the work of four small decisions" in
      `each_unanswered_opens_wait_is_its_own_and_not_one_shared_across_opens`: the test
      decides three.
      **Scenario:** the comment justifies the three-limit bound on the valid op ("the
      right answer takes two [limits] plus the work of four small decisions, leaving a
      limit of slack"). The test offers three messages (`:2317-2331`: `on_first`,
      `on_second`, `after`), so the processor makes three decisions, two of them
      refusals. "Four" is the count in
      `many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`
      (four stuck messages), carried over. The margin argument is the comment's reason
      for the bound, and it cites a number of decisions the test does not contain.
      **Fix shape:** "three small decisions", or drop the count ("plus the work of
      deciding three messages").
      **Severity:** nit.
      **Outcome (`tester`): fixed.** The comment now reads "plus the work of
      deciding three messages, leaving a limit of slack", the second of the fix
      shape's two wordings, so the comment no longer carries a count to drift
      from the three messages the test offers.

- [x] **`tester`** — `delivery/tests.rs:2289-2290` — a quotation of the spec that is
      not in the spec.
      **Scenario:** the comment quotes `op-transport`'s MUST as "judged only once that
      open is settled … unless that channel's own wait expires first". The delta
      (`spec.md:188`) ends "and against the channels open then, unless the wait below
      expires first"; `git grep -F "own wait expires"` finds the phrase only in the
      test comment and in `findings/spec-test.md:612`, which paraphrased it. A reader
      searching the spec for the quoted words finds none, in a comment whose job is to
      say which sentence the test enforces. (`findings/spec-test.md:630` has the real
      words.)
      **Fix shape:** quote the spec's words ("unless the wait below expires first"),
      or drop the quotation marks.
      **Severity:** nit.
      **Outcome (`tester`): fixed.** The comment now quotes two fragments of the
      requirement and the exception separately, each verbatim from the delta:
      "judged only once that open is settled" and "unless the wait below expires
      first", and says the wait in the exception is the second open's own. The
      invented phrase is gone (`git grep -F "own wait expires first" --
      dialectica/` finds nothing).

- [x] **`dev-writer`** — `design.md:134` — "(re-run on the final tree)" is the stale-label
      shape round 2's second entry removed everywhere else.
      **Scenario:** Decision 4's new sentence says
      `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call` "is
      red with `DELIVERY_CALLBACK_TIMEOUT` at 10 s and `CALL_TIMEOUT` at 20 s, which
      compiles (re-run on the final tree)". "The final tree" is whichever tree its reader
      thinks it is; it names no commit, and it is the same relative label
      ("re-run on this change's last round") that went stale within a round before.
      The claim itself holds (10 < 20 < 40 satisfies both compile-time asserts; the test's
      literal 30 s fails against a `CALL_TIMEOUT` of 20 s) and needs no label.
      **Fix shape:** drop the parenthesis; the mutation is stated as a command a reader
      can run.
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Restate Decision 11 for the ask, and
      drop a stale label`: the parenthesis is gone, as the fix shape says. The claim
      is unchanged and needs no label. The new mutation claims written in the same
      commit (Decision 11, the ask) carry none either: each names the change and
      the tests it turns red.

### What I checked, and found clean

- **Round 2's four entries are fixed as their outcomes say.** Decision 11's "What still
  bounds the stall" is two bounds with the K = 20 working (735 s, not 800 s); the
  rejection marks the square-of-K claim withdrawn and says what the limits differ in;
  the arithmetic holds by hand: 40m = 35(m+1) at m = 7 (5m = 35, both 280 s); 21 x 35 =
  735; at a 36 s limit 20 x 36 = 720 against 735; K x 40 s reachable only while
  K <= 7. The 914 / 917 / 612 ms figures match `findings/security.md:403-406`. The
  counts are gone from Decisions 6, 10 and 11 (`git grep` for "twelve", "27", "last
  round", "re-run on this change" finds none; the one survivor is entry 3 above).
  "Neither value is visible to a test" and "No test can see" appear nowhere now; the
  comment beside the asserts, `CALL_TIMEOUT`'s doc, Decision 4, Decision 11, `tasks.md`
  9.4 and 10.4 name the test that holds delivery's 30 s. `tests.rs:2228` gives the full
  test name.
- **Mutation claims, run (each restored with `git checkout -- <file>`; 95
  `delivery::tests::` tests, all green unmutated):**
  - `declined` ignoring `error` (`.filter(|_| false)` on the `callee_error` read):
    13 red, among them `declined_reads_delivery_s_three_shapes_of_no` and the
    "already exists" tests, as Decision 6 says (and it names no count).
  - `>=` to `>` in `InboundQueue::offer`: four red, the three Decision 10 names
    (`a_full_queue_keeps_what_it_holds_and_discards_the_arrival`,
    `the_waiting_messages_never_exceed_the_bound`,
    `the_queue_the_running_wiring_builds_is_bounded_at_the_pinned_count`) and
    `every_discard_is_counted_and_logged_apart_from_refusals`, which the text does not
    name and does not claim to be alone.
  - `now_ms` from `message.timestamp as u64` in `Processor::pass`:
    `the_window_is_judged_by_this_peers_clock_not_the_events_timestamp` red.
  - the wait's `left` fixed at an hour: seven red, each timing out in `eventually`;
    the six Decision 11 and `tasks.md` 10.4 name, plus the new
    `each_unanswered_opens_wait_is_its_own_and_not_one_shared_across_opens`, which no
    prose cites and which the prose does not claim is absent.
  - the loop condition `!open || pending` in `await_settled`: the reworked
    `a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer` red,
    timing out at ten seconds ("the message to be refused once the open is declined"),
    as its comment says; `a_message_arriving_while_its_channel_opens_is_judged_after_the_answer`
    stays green, which is why the declined case is the one that pins it.
- **The one-predicate claim is true.** `wait_ends` guards on `!self.is_opening(..)` and
  `await_settled` loops on `book.is_opening(..)`; the doc's "the question
  `Channels::await_settled` asks again on each wake-up" is what the code does.
- **Prose matches across the three places that state it.** `SETTLE_LIMIT`'s doc, Decision 11
  and the spec's "consequence" paragraph agree on "at most ... once each" and "never past
  the moment the last of those opens settles"; the new spec paragraph says it adds no
  requirement and uses no MUST; the two new requirements' MUST stays uniform inside their
  block. `proposal.md`'s rewritten sentence and its fifth open question ("`logos.test`
  and `Edge`", `delivery.rs:120-121`) are true. Scenario names quoted in test comments
  (`A message's wait outlasts ...`, `Each unanswered open's wait is its own`) match the
  delta headings exactly.
- **Gates.** `openspec validate delivery-wiring --strict` passes;
  `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml --check` is clean.

### Not boxes (taste; say so and move on)

- `tasks.md` section 11 lists the refactor and the design change, and not the two test
  changes in `7462ded8` (the new two-open test and the reworked decline test), although
  the file's own header says each regression test is "seen red". The tests cite their
  scenarios, so nothing is lost; a reader going tasks to test misses them.
- Decision 11 (`design.md:374-377`) reads "the four tests named below, `a_message...` and
  `opens_settling...`": it means four plus two, and the comma list could be read as
  naming four in all. Clear from the list at `:435-451`.
- `design.md` Decision 11 is now long (about 160 lines) with the same cost figures
  worked in three places; one place would carry them. Round 1 said the same about the
  wait's cost, and the rewrite added a section rather than merging.
- Mutation testing was by hand on the claims above, not `cargo mutants`. No mutation
  is left in the tree: each was reverted with `git checkout -- <file>`.

## Re-review round 4 `7462ded8..58460b02`

Dimension: readability only. Reviewed at `adba1a22` (the range's code and prose are
unchanged by the findings commits after `58460b02`): the range's diff of `delivery.rs`
(`OpenTime`, `Wait`, `Pending::asked`, `Channels::asked`, `Opening::asked`,
`Processor::new`, the reworded `SETTLE_LIMIT` and `await_settled` docs),
`delivery/tests.rs` (the six new tests and the two reworded comments), `design.md`
(Decisions 4, 11 and Risks), `proposal.md`, `tasks.md` and the `op-transport` delta.
Round 3's three entries are all confirmed answered as their outcomes say. Two new
defects, both low, both stale references in prose; neither changes behaviour.

- [x] **`dev-writer`** — `design.md:473`, `design.md:733` — two citations of a commit
      that will not exist on `main`.
      **Scenario:** Decision 11 says the ask's requirement is "from the spec-writer's
      `d240ebdc`" and that the narrower promise was "rejected ... in `d240ebdc`".
      `d240ebdc` is a commit on `piece/176-delivery-wiring`; `main` is squash-merged
      (`git log origin/main` subjects all end in `(#NNN)`), so after the merge the
      SHA names nothing, and `design.md` is archived with the change. A reader of the
      archived design who wants to see what the spec-writer rejected cannot open it.
      Round 1's fifth entry removed this class (`findings/security.md`, "PR #190's
      first-pass mutation list"); this is the same shape with a SHA in place of a
      file. The sentences stand without it: the requirement is named by its heading
      ("Asking delivery to create a channel starts the open's time again"), and the
      rejection is stated with its reason in the same sentence.
      **Fix shape:** drop both SHAs, or say "the spec-writer's callback after
      round 3". (`tasks.md:95` heads section 12 with the same SHA, as section 11's
      heading does with `de35f026`; those are a task log, not an argument, and I do
      not box them.)
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Extend only a wait that has not
      ended, and drop two stale references`: the first now reads "added by the
      spec-writer after the round-3 security re-review", the second "The
      spec-writer rejected it in its callback after that round". `git grep -E`
      for a backticked hex string in `design.md` finds only upstream revisions
      (`logos-delivery`, the builder pin, the `logos_protocol.h` header), none a
      commit on this branch. `tasks.md`'s section headings keep theirs, as you
      said.

- [x] **`dev-writer`** — `tasks.md:92` — task 11.1 describes `ChannelBook::wait_ends`
      as a function that "asks `is_opening`"; `wait_ends` no longer does.
      **Scenario:** 11.1 reads "`ChannelBook::wait_ends` asks `is_opening` rather than
      spelling 'pending and not open' a second time, so whether a message waits and
      whether it keeps waiting are one predicate". In `a59e9c6d` the function that
      makes that call became `ChannelBook::begin_wait` (`delivery.rs:643-660`), and
      `wait_ends` (`:663`) is now `fn wait_ends(&self, channel_id, wait: WaitId)`, a
      read of one message's end that never calls `is_opening`. A reader following 11.1
      to `wait_ends` finds a two-line lookup and no predicate; the claim is true of
      the function under its new name. No other document names the old symbol
      (`git grep -n wait_ends` outside `delivery.rs`, `tests.rs` and this line finds
      nothing), so this is the one site a rename sweep missed.
      **Fix shape:** "`ChannelBook::begin_wait` (then `wait_ends`) asks `is_opening`".
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Extend only a wait that has not
      ended, and drop two stale references`, in your fix shape's words:
      "`ChannelBook::begin_wait` (then `wait_ends`) asks `is_opening`".

### What I checked, and found clean

- **Round 3's three entries are answered as recorded.** `tests.rs` now reads "plus the
  work of deciding three messages" and quotes the requirement as "judged only once that
  open is settled" and "unless the wait below expires first" (both verbatim in
  `spec.md`); Decision 4's "(re-run on the final tree)" is gone, and the new mutation
  claims in Decision 11 carry no such label.
- **Arithmetic, by hand.** 40 + 35 = 75 s ("in practice under 40 + 35 = 75 s");
  2 x 40 = 80 s ("under 80 s"); K = 20: 21 x 35 = 735 s; open m is asked when open
  m-1 settles, at m x 35 s, and settles at (m+1) x 35 s, so the ask's end (m x 35 + 40)
  is past the settle by 5 s for every m and the stall is (K+1) x 35 s for every K;
  K = 1: min(40, 70) = 40 s before the ask and 2 x 35 = 70 s after ("70 s where it was
  40 s"); min(40m, (m+1) x 35) meets at m = 7 (280 s each), so "fewer than 7 Stoas" is
  the range where the ask lengthens it. The 5 s margin is `SETTLE_LIMIT` 40 s less
  `CALL_TIMEOUT` 35 s (`delivery.rs:98,710`).
- **"Turns red" claims, run** (each restored with `git checkout -- <file>`; 102
  `delivery::tests::` tests, all green unmutated):
  - the wait's `left` fixed at an hour: nine red, exactly the nine Decision 11 lists
    ("removing the bound altogether"), each timing out in `eventually`;
  - `Option::take` replaced by a copy in `Wait::extend_from` (uncapped): only
    `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again` red, "refused
    1.700s after the first ask" against the text's 1.71 s (the same measurement, to
    timing noise);
  - `Wait::extend_from` a no-op: three red, the three Decision 11 names
    (`a_message_waiting_when_delivery_is_asked_is_judged_after_delivery_answers`,
    `a_second_ask_...`, `the_worker_marks_its_ask_of_delivery_in_the_channel_book`);
  - `Pending::asked` not setting `OpenTime::StartedAt`: only
    `asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
    red, as stated;
  - both of those at once: `an_earlier_message_on_a_queued_open_...` red, and green
    with either alone (the two earlier runs), which is the "red only with both halves
    gone" claim exactly;
  - `opening.asked()` removed from `Worker::open`: only
    `the_worker_marks_its_ask_of_delivery_in_the_channel_book` red, as the worker's
    comment and Decision 11 say;
  - `OpenTime::end` giving a later message `now + limit`: `many_messages_on_one_...`
    red ("the op behind the stuck open waited 4.017s: more than one wait of 1s", the
    text's 4.02 s) and `an_opens_time_is_the_same_for_every_message_...` red.
- **Code prose matches the code.** `OpenTime`'s three-state doc, `Wait::extend_from`'s
  "once" (`Option::take`), `Pending::asked`, `Channels::asked`, `Opening::asked`'s "called
  by the worker ... and by nothing else outside tests" (one call site, `delivery.rs:1004`),
  `await_settled`'s "a wait gone from the book ends the wait", and `SETTLE_LIMIT`'s
  bounds all say what the code does. The spec's quoted phrases in the comments
  ("MUST NOT move that message's end again", "Only a later create, join or startup
  asking for that channel, or this peer asking delivery to create it, lets a message
  wait on it again") are verbatim.
- **Spec, proposal and design agree** on "at most once for each start of an open's
  time", "less than twice the fixed time", and the two losses that remain (a wait that
  ran out before the ask; a message an earlier ask already extended). The new
  scenarios' WHEN clauses match the tests' timing comments (0.6 of a limit, answer at
  1.2). The four scenarios amended with "this peer does not ask ..." match the code's
  only way to start a time.
- **Gates.** `openspec validate delivery-wiring --strict` passes; `cargo fmt
  --manifest-path dialectica/rust-lib/Cargo.toml --check` is clean.

### Not boxes (taste; say so and move on)

- `tests.rs` (`a_request_made_while_a_message_waits_does_not_extend_that_messages_wait`)
  has one comment line about 100 columns among 80-column neighbours, an edit residue
  from extending the paragraph in place (round 1 noted the same in `delivery.rs`).
- `ChannelBook::end_wait` is the one method of the new set with no doc line; its name
  and its caller say it, so I did not box it.
- Comments carrying review labels ("spec-test re-review round 3", "the security
  re-review's round-3 probe") in `delivery.rs` and `tests.rs` will read oddly once
  `findings/` is deleted; each states its reason beside the label, so nothing is lost
  (round 1 made the same call).
- Decision 11 grew again (it now runs from `design.md:357` into the 700s), with the
  cost figures worked in several places; round 3 said the same of a shorter one. Every
  figure I checked is accurate.
- Mutation testing was by hand on the claims above, not `cargo mutants`. No mutation
  is left in the tree: each was reverted with `git checkout -- <file>`.

## Re-review round 5 `58460b02..1d5e2e37`

Dimension: readability only. Read at `e5268cc2`: the range's `delivery.rs` diff
(`Wait::extend_from`'s guard and doc), the new test
`an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again` and its
comment, `design.md` Decision 11 (the ask paragraph, the new "Why only a wait that
has not ended" paragraph, the "What breaks without each part" list, the rejected
alternatives), `tasks.md` 13. Round 4's two entries are confirmed answered as their
outcomes say: `git grep -E "d240ebdc|de35f026"` finds nothing in `design.md`,
`proposal.md` or the Rust tree, and `tasks.md:93` now reads "`ChannelBook::begin_wait`
(then `wait_ends`)", the function that calls `is_opening`. One new defect, low, in prose.

- [x] **`dev-writer`** — `design.md:538` — the new mutation claim states a bound the
      test does not have.
      **Scenario:** the "What breaks without each part" entry reads: "the extension
      given to a wait whose end has passed turns
      `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again` red — judged
      1.00 s after the ask at a 1 s limit, where the test allows 0.5". The test asserts
      `judged_after < limit * 3 / 4` (`tests.rs:2989`), which is 0.75 s at a 1 s limit,
      and its own comment says "The bound is three quarters of a limit ... 0.75 s of
      slack". A reader reproducing the mutation and checking the design against the
      test, or re-tuning the test from the design's number, finds 0.5 against 0.75.
      The red reading (1.00 s) fails either bound, so nothing is hidden; the sentence
      is wrong about the test it cites, in the one list whose job is to say what each
      test is held by. It looks like the first draft's bound, left when the comment
      argued for three quarters.
      **Fix shape:** "where the test allows 0.75", or drop the second figure ("judged a
      full limit after the ask, where the right answer is judged at once").
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Correct the ask-after-end test's
      allowance, and cite round 5's scenarios`, in your first fix shape: "where the
      test allows 0.75", the `limit * 3 / 4` the assertion holds at a 1 s limit.
      Prose only; no test changed. The same commit takes your "not a box" note:
      `Pending::asked`'s doc now says every message waiting "whose end has not yet
      passed" is extended, and that `waits` can still hold an ended wait which
      `Wait::extend_from` leaves alone; `design.md`'s ask paragraph gains the same
      qualifier and points to "Why only a wait that has not ended".

### What I checked, and found clean

- **`Wait::extend_from`'s doc** says what the code does: the guard is `asked >=
  self.ends`, so "an ask at the end itself counts as after it"; `await_settled` does
  break on `left.is_zero()` (`delivery.rs:864-867`), so "counts no time left as ended"
  is the same reading. The cited test exists (`tests.rs:2922`) and is cited nowhere
  else outside `design.md` and `tasks.md`.
- **The new `design.md` paragraph** matches the code and the spec: the `Wait` stays in
  the book until the waiter re-takes the lock (`end_wait` runs after the loop);
  "less than twice `SETTLE_LIMIT`" resolves to the bound at `:607-613` and
  `SETTLE_LIMIT`'s doc, which says "under it before the ask, or its wait would have
  expired"; the spec quotation ("MUST be judged without waiting on that open") is
  verbatim in `spec.md:190`; the correctness re-review's round 4 entry
  (`findings/correctness.md:561`) describes the same probe, which the test ports. The
  rejected alternative (the waiter dropping its own `Wait`) gives a reason that holds:
  dropping needs the lock the ask holds.
- **The test's comment** agrees with its arithmetic: the book taken at 0.5 of a limit,
  held 1.0 more (1.5 total, asserted `> 1.25`), past the message's end at 1.0; "three
  quarters of a limit" and "0.75 s of slack" are the same bound as the assertion; the
  spec phrase it quotes is verbatim. `Pending::asked` is what `Channels::asked` runs
  under the lock (`delivery.rs:813-819`), as the comment says.
- **`tasks.md` 13.1 and 13.2** say what the commits did (the guard, the test, the two
  stale references) with no SHA of this branch.
- **Not run:** no mutation or test run this round; the one new mutation claim
  (extension given to an expired wait) is the guard removed, which the correctness
  reviewer's probe measured red in round 4.

### Not boxes (taste; say so and move on)

- `Pending::asked`'s doc (`delivery.rs:630-631`) and `design.md:478` still say "every
  message waiting" is extended, and the only exception they list is the once-only cap;
  the expired-wait exception is on `extend_from` and in the paragraph 25 lines on.
  Read with "waiting" meaning not yet ended, which the spec does, they are true; but
  `Pending::waits` now holds expired waits too, so a reader of the loop in
  `Pending::asked` alone could take it to extend them. One clause at each site would
  close it; I did not box it.
- Decision 11 grew again, with another paragraph of the same shape as the rounds
  before; every figure I checked is accurate.

## Re-review round 6 `1d5e2e37..3d34f15e`

Dimension: readability only. Read at `63989d66`: the range's `delivery.rs` diff
(`Pending::asked`'s doc), `tests.rs` (the ask-after-end test's comment and the
four new scenario citations), `design.md` (the ask paragraph, "Why only a wait that
has not ended", "What breaks without each part"), `proposal.md`, the `op-transport`
spec delta (the ask paragraph, the timeout-order paragraph, the amended and the four
new scenarios, the new requirement), `tasks.md`. Round 5's box is confirmed answered
as its outcome says: `design.md:540` reads "where the test allows 0.75", which is
`limit * 3 / 4` at `tests.rs:2990`; `Pending::asked`'s doc and `design.md:478-481`
now carry the "whose end has not yet passed" qualifier and the pointer. All four
scenario names cited in `tests.rs` (2923, 3067, 3100, 3139) and the requirement
name match `spec.md` character for character, and the long quotation at `tests.rs:2924-2926`
is verbatim from `spec.md:192`. One new defect, low, in prose.

- [x] **`dev-writer`** — `design.md:547-549` — the closing count of tests "red
      against a no-op ask" now takes in a test that is green against one.
      **Scenario:** the paragraph reads "The five tests named in this paragraph and
      the list above — the four new scenarios' and the worker's — were each red
      against a no-op ask before the change." The list above gained
      `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again` in round 5,
      so "this paragraph and the list above" now names six tests, not five. That sixth
      is not red against a no-op ask: with `Pending::asked` doing nothing no wait is
      extended, so the message is judged at the end it began with and `judged_after`
      reads about zero against the `limit * 3 / 4` bound (reasoned from
      `delivery.rs:618-625` and the test's own comment, "Right: judged as soon as the
      processor has the book"; I tried to confirm by mutating `Pending::asked` to a
      no-op and the edit was refused by the harness, so this is **not measured**).
      It is red only against the mutation the list already states for it, the guard
      removed. A reader who takes the sentence at its word and runs all six against a
      no-op ask finds one passing, and cannot tell from the page which one the five
      exclude.
      **Fix shape:** name the five, or say "the five ... other than the ask-after-end
      test, which is red only with the guard removed".
      **Severity:** low.
      **Fixed** (`dev-writer`) in the commit `Record why each wait leaves the book, and
      correct the no-op-ask count`, in both of your shapes: the sentence now says
      "Five of the tests named in this paragraph and the list above", names the five
      (the four new scenarios' and the worker's), and says the sixth,
      `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again`, is not
      one of them — it came after the ask was built and is turned red by the
      `Wait::extend_from` guard's removal (its bullet). It does **not** claim the
      test is green against a no-op ask: I tried the same mutation of
      `Pending::asked` and the harness refused the edit as you found, so that stays
      unmeasured and the page says only that a no-op ask is not the mutation the test
      answers. `tasks.md` 12.2 already names exactly the five, so it needed no edit.

### What I checked, and found clean

- **`Pending::asked`'s doc** says what the loop does: `extend_from` is what skips an
  ended wait (`delivery.rs:619`), `waits` is what holds it until the waiter
  re-takes the lock (`end_wait` after the loop), and `[`Wait::extend_from`]` resolves.
- **The test comment at `tests.rs:2944-2950`** agrees with the assertion and with
  `design.md:540`: three quarters of a limit, 0.75 s of slack.
- **The spec delta** is self-consistent: the amended scenario "A message waiting when
  this peer asks delivery for its channel is judged after delivery answers" adds "before
  the message's end has passed", which the timeout-order paragraph's "where its end has
  not yet passed" mirrors; the new scenario's two THEN lines are what the test asserts
  (refused unknown-channel, and before the bound); the new requirement's consequence
  ("at most one message's wait at any time") follows from "a message waits on an open
  only while it is the one being judged". `proposal.md`'s ADDED list names the
  requirement as `spec.md:424` does.
- **`tasks.md`** says nothing of round 6 yet, which is the runner's row.
- **Unrun:** `cargo test` of the one test passes on the unmutated tree; no mutation
  landed.

### Not boxes (taste)

- `proposal.md:70-71`: the amended sentence runs one line past the file's wrap
  width ("whose end has not yet passed, once for that message; a message so extended
  holds the other Stoas up for"), where the paragraph before is wrapped at about 75.
  Formatting only.

## Re-review round 7 `3d34f15e..ae44f364`

- [x] **re-review round 7 `3d34f15e..ae44f364`: no findings** — read the range's
      `design.md` (the reworded no-op-ask count; the new "Why each wait leaves the
      book" section), the three reworded `op-transport` scenarios, and `tasks.md`,
      against `delivery.rs` (`Wait`, `Pending::asked`, `begin_wait`, `end_wait`,
      `settle`, `await_settled`) and `delivery/tests.rs`; clean. Round 6's box is
      answered as its outcome says: the sentence now names five tests red against
      a no-op ask and says the sixth,
      `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again`, is held
      by the `Wait::extend_from` guard; counting the names in the paragraph and the
      list above gives those six. All nine test names cited in the new prose exist
      as `fn`s in `tests.rs`; the requirement name quoted in `design.md:589` and
      `proposal.md:168` matches `spec.md:424`; the prose's `(K+1) × CALL_TIMEOUT`
      and "(Risks)" resolve (`CALL_TIMEOUT` is 35 s, the Risks bullets name the
      unbounded outbound queue); `end_wait` after the loop, the one early `return`
      before any `Wait` exists, the shared id counter and `settle` removing the
      entry only at the last request all match the code; the test helper
      `waits_in_the_book` does answer `None` once the entry goes, as the prose says.
      The scenario rewordings are consistent with the requirement's "that message's
      wait", and no test comment quotes their THEN lines.

Not boxes (taste): `design.md:609` wraps one line far past its neighbours ("book's
lock (`Pending::asked`). Removed as each wait ends, with the one processor judging
one"), an edit residue. The new section names "round 4" and "round 5" re-reviews,
which read oddly once `findings/` is deleted; Decision 11 already cites "the
security re-review" the same way, so nothing is lost.

## Re-review round 8 `ae44f364..43844f2b`

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/transport.rs:942` —
      a test comment cites the wrong line of delivery's source.
      **Scenario:** the comment reads "`DefaultContentTopic` in the same file, line
      15"; at `bfdb5afd263c5ff634ef8c59b2fe1ebbbcd0f306`,
      `git grep -n "DefaultContentTopic\* ="` on
      `logos_delivery/waku/waku_core/topics/content_topic.nim` answers line **16**
      (line 15 is blank). Every other citation in the range holds at that rev:
      `content_topic.nim:60-123` (`parse` starts at 60, its last `return err` is
      123), `channel_lifecycle.nim:46-48` (the subscribe is lines 45-48),
      `sharding.nim:20-30` (`getGenZeroShard`), `channel_api.nim:19-23`, and the
      LIP-23 and relay-sharding line numbers. A reader following this one lands on
      the wrong line of a file the comment is the only pointer into.
      Low severity; prose only.
      **Fixed** (`dev-writer`) in the commit `Take getShard's generation step
      into the topic rule, and fail a test on a delivery pin bump`: line 16,
      confirmed with `git grep -n -F "DefaultContentTopic* ="` at `bfdb5afd`.
      The taste note on `transport.rs:103-104` is taken too: the parser pointer
      is its own sentence and says "further down this file".

Read: the whole range, `CLAUDE.md`'s new trap entry against that file's "Keeping
this file true" rules (it holds no count a command could answer, names its own
witnesses `transport::delivery_topic_rule` and `content_topic.nim` so a rename shows
as stale, and records why rather than what landed), the `TOPIC_PREFIX`,
`CHANNEL_PREFIX` and `delivery_topic_rule` docs, the fake's `channel_create`
comment, Decision 17 and its table row and Risks bullets, and tasks 7.3 and 14.
The test names Decision 17 cites all exist as `fn`s, `b8b9ac2f` is the rev the
lockfiles carry, and `ChannelId` is `SdsChannelID` as the `CHANNEL_PREFIX` doc says.
No stale `/dialectica/1/s/` form is left outside archived changes, findings, and the
places that name it as the old value on purpose. Not re-run: the "every `delivery::`
test went red" measurement.

Not boxes (taste): `transport.rs:103-104` splices two thoughts with a colon ("was
declined by every node: the test-only `delivery_topic_rule` below cites the
parser."), and line 104 runs to about 95 columns where its neighbours wrap at 80.
The "below" is some 750 lines away. An edit residue; the sentence reads better with
the parser pointer as its own sentence.

## Re-review round 9 `43844f2b..87596ac4`

Dimension: readability only. Reviewed at `286a2314` (the range's code and prose are
unchanged by the findings commits after `87596ac4`). Round 8's one box is answered as
its outcome says: `transport.rs:984`'s comment now reads line 16 (`git grep -n`
at `bfdb5afd` answers `content_topic.nim:16`), and the parser pointer on
`TOPIC_PREFIX` is its own sentence ("further down this file"). Three new defects, all
low, all prose.

- [x] **`spec-writer`** — `openspec/specs/composer-view/spec.md:322` — the live spec
      still says "delivery is not wired at all", the sentence this range just
      removed from the QML comment that quotes it.
      **Scenario:** `DPublishOutcome.qml:149-151` (and `FeedScreen.qml:1078`) were
      reworded this round because delivery is wired by this change. They are
      comments copied from `composer-view`'s requirement on the delivery denial,
      whose paragraph at `:321-327` reads "A publish reply carries no delivery
      outcome by design; delivery is not wired at all; and a post whose body is
      legal but near the cap ...". After this change merges, `git grep -F "wired at
      all" -- openspec/specs` still finds the live spec asserting what the code no
      longer does, and the comment and the spec it mirrors disagree. Nothing under
      `openspec/changes/delivery-wiring/` (proposal, delta, findings) mentions
      `composer-view`, so no one has decided to leave it. The user-facing string
      ("... not something this software can tell you yet.") is unchanged and still
      true, so the requirement itself stands; only its rationale paragraph is stale.
      **Fix shape:** a one-sentence MODIFIED delta on `composer-view` that carries
      the requirement verbatim with that clause replaced ("core hands the op to the
      network without hearing whether any peer received it"), or a recorded decision
      that the paragraph stays and a follow-up issue for it.
      **Severity:** low (rationale prose in a live spec; no behaviour).
      **Outcome (`spec-writer`):** fixed as a MODIFIED delta,
      `openspec/changes/delivery-wiring/specs/composer-view/spec.md`. The live
      requirement and all five of its scenarios are carried verbatim; the only
      change is the clause "delivery is not wired at all", now "where core hands
      the op to the network, it learns nothing of whether any peer received it"
      (`git diff --no-index --stat` against the live file: 4 insertions, the
      header and the three re-wrapped lines). Every obligation stands, so no test
      or UI string changes. `proposal.md` lists `composer-view` under Modified
      Capabilities. `openspec validate delivery-wiring --strict` passes.

- [x] **`spec-writer`** — `openspec/changes/delivery-wiring/specs/op-transport/spec.md:15`
      — "or read with segments other than `dialectica` and `1`" names a failure that
      a name beginning with `/dialectica/1/` cannot have.
      **Scenario:** the paragraph says beginning with the literal prefix does not
      meet the rule on its own, because such a name "has any other number of parts
      ... and is refused by that rule, or read with segments other than
      `dialectica` and `1` as its application and version". Take any string that
      begins `/dialectica/1/`: after the leading `/` its first two parts are exactly
      `dialectica` and `1`. In the four-part form they are the application and
      version, so the rule reads it as required. In the five-part form the first part
      is the generation, `dialectica` is not numeric, and the rule refuses it (the
      live message). Any other count is refused. So the second alternative never
      occurs; the first is the only way a prefixed name fails. The scenario's own
      bullet "the rule reads `dialectica` as its application and `1` as its version"
      is likewise implied by acceptance plus the prefix, which `the_content_topic_is_one_delivery_parses_with_dialectica_as_application`
      asserts anyway. A reader takes from the sentence that a prefixed name can be
      accepted and misread, and looks for the protection the requirement adds
      against it.
      **Fix shape:** drop the clause ("... has any other number of parts is refused
      by that rule"), or say the reading follows from the prefix once the rule
      accepts the name, so the version-and-application check restates the prefix.
      **Severity:** low.
      **Outcome (`spec-writer`):** fixed, both halves of the fix shape. The
      impossible alternative is dropped, so the sentence ends "... is refused by
      that rule.", and a new sentence says a name that begins with the prefix and
      is accepted is read with `dialectica` and `1` as its application and
      version, "so that half of the rule restates the prefix rather than adding to
      it." The requirement heading's SHALL and the scenario bullet are kept: both
      remain true, and the bullet costs nothing against the test that asserts it.
      `openspec validate delivery-wiring --strict` passes.

- [ ] **`dev-writer`** — `design.md:1097-1099` — the last sentence of "A pin bump
      fails a test" does not parse, and "(Decision 14's wording)" at `:1093` points at
      wording Decision 14 does not hold.
      **Scenario:** "Before it, the only prompt was 'a pin bump should re-read' in
      three places that no command triggered — the repo's own `.lidl` gap (`ci.yml`)
      in a second place." The last clause has no verb and its referent is unclear:
      `ci.yml:1783` is about nothing checking the checked-in `delivery_module.lidl`
      for drift, a different gap that this test does not touch, offered here as "a
      second place" of the same kind. The three places are not named, so the reader
      cannot count them (`git grep -F "pin bump should re-read"` finds none in the
      tree now, since this range replaced them). Separately `:1092-1093` says the
      test message names `content_topic.nim`, `sharding.nim` and
      `channel_lifecycle.nim` "(Decision 14's wording)", but Decision 14
      (`:927-928`) says only that the message "names `channel_lifecycle.nim` among
      what to re-read"; the three-file list is the test's own
      (`transport.rs:1050-1056`).
      **Fix shape:** "Before it, the only prompt was a sentence, 'a pin bump should
      re-read', in Decisions 14 and 17 and the Risks list, which no command
      triggered; `ci.yml`'s note on the unchecked `delivery_module.lidl` is the same
      kind of gap elsewhere." and drop "(Decision 14's wording)" or say "the list is
      the test's own".
      **Severity:** low.

Read: the whole range outside `findings/` (`CLAUDE.md`'s trap entry, `delivery.rs`'s
`ALREADY_EXISTS` doc, `transport.rs`, the spec delta against the live `op-transport`
requirement, `proposal.md`, `design.md`'s Decisions 5, 14, 17, Risks and Open Questions,
`tasks.md` 7.3 and section 15, both QML comments). The new `op-transport` requirement
is the live one verbatim plus the new paragraphs and scenario, as `proposal.md:152`
says. `CLAUDE.md`'s trap entry holds no count a command answers, says why, and
invalidates visibly (it names `transport::delivery_topic_rule`, a test, and
`dialectica/flake.lock`); "generation `0`" matches `subscribable`. Both QML edits
change comments only, and the user-facing strings are identical.
Citations into `logos-delivery` at `bfdb5afd263c5ff634ef8c59b2fe1ebbbcd0f306`,
re-read: `sharding.nim:32-51` holds both `getShard` overloads (`:32-43` the
`NsContentTopic` one with `Generation > 0 are not supported yet` at `:43`, `:45-51`
the `ContentTopic` one, as the doc says); `sharding.nim:20-30` is `getGenZeroShard`;
`content_topic.nim:16` is `DefaultContentTopic`, `:60-123` is `parse`;
`channel_lifecycle.nim:46-48` is the subscribe and `:43` the "channel already exists"
return; `subscription_manager.nim:157` is `getShardForContentTopic` and `:249` the
`subscribe(topic)` that calls it, reached from `channel_lifecycle` through the
`MessagingSubscribe` provider (`messaging_client_lifecycle.nim:29`) and
`waku/api/subscriptions.nim`; the doc's `channel_lifecycle.nim` → `getShardForContentTopic`
→ `getShard` skips those two hops, which reads as a path summary and is not wrong. The
`logos-delivery-module` pin `b8b9ac2f…` is `dialectica/flake.lock:169` and the delivery
rev `bfdb5afd…` is `:1086`; `include_str!("../../../flake.lock")` from
`src/transport.rs` resolves to `dialectica/flake.lock`. Not re-run: the tests'
red-before-fix measurements and the live-run figures in `tasks.md` 7.3 and Risks,
which only the owner's logs hold.

Not boxes (taste): `transport.rs:928-930` records "Two reviewers recall" Nim's
`parseInt` skipping `_`, unverified; the hedge is honest and the direction ("the
stricter side") is right, but the provenance is two review rounds that are deleted
with `findings/`, and a reader will want the Nim stdlib's `parseutils` cited or the
remark dropped. `DPublishOutcome.qml:150-152` leaves one line ending at "and a" after
the reword, the edit-residue shape earlier rounds noted. `design.md:1010-1011` does the
same at "LIP-23 states the four-part".
