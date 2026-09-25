# Readability review — time-pegged-clock

Scope: dimension **readability** only, per dispatch. Read the owner's decision
comment on issue #162 (2026-09-25); its scope — pegging the counter to
wall-clock time, the one-hour receive window, item 1 (`op-ordering`
self-contradiction) and item 3 (`thread-read` position wording) still in
scope, op format unchanged — is what this review was read against, not the
issue body's original "What this needs" list.

Reviewed against `git diff origin/main...HEAD` (three dots): every source file
it touches —`arrival.rs`, `authoring.rs`, `transport.rs`, `op.rs`, `feed.rs`,
`thread.rs`, `revision.rs`, `wire.rs`, `asserted_time.rs`, `log/mod.rs`,
`log/sqlite.rs` — plus the new tests each adds. Design/spec prose
(`design.md`, `proposal.md`, `specs/*`) is out of this lane; that is the
design-reviewer's and spec-test-reviewer's material.

## No checkbox findings

I looked for the usual readability defects — a function doing two jobs, a
stale comment left after a rename, a doc claim the code no longer matches, a
`handle`/`process`-style name masking a merged responsibility, an orphaned
reference to the old `ADVANCE_BOUND` — and did not find one that rises to a
blocking finding. Specifics:

- **No dangling references to the removed `ADVANCE_BOUND`.** The two mentions
  left in `arrival.rs` are deliberately historical ("This replaced
  `ADVANCE_BOUND`, which stored every counter..."), and `git grep -F
  ADVANCE_BOUND` across `dialectica/` and `dialectica-ui/` finds only those
  two.
- **`next_counter`'s signature change (`clock` → `clock, now_ms`) and
  `receive`'s (`+ now_ms`) are threaded consistently.** Every call site in
  `arrival.rs`, `authoring.rs` and `transport.rs` (production and tests) was
  updated; `transport::receive` has no caller outside `dialectica-core` yet
  (`git grep -F "::receive("` across `rust-lib` returns nothing), so there is
  no stale call site elsewhere in the tree for this piece to have missed.
- **The six-step judgement order documented on `receive`'s doc comment
  ("Is its counter within the receive window of `now_ms`? Last...") matches
  the code**: the window check is the last guard before `log.append`, after
  channel lookup, size, decode, verify and Stoa match, exactly as numbered.
- **One function, one job holds.** `exceeds_receive_window` takes a bare
  `u64` counter and nothing else (deliberately, per its own doc, so the
  wall-clock field cannot reach the predicate); `receive` gained one more
  guard clause in the same shape as its existing five, not a second
  responsibility; `next_counter`'s `now_ms.max(clock.saturating_add(1))` is a
  single expression doing the one job its doc describes, not a branch grown
  into two.
- **Comments earn their place.** Nearly every doc block that changed is
  answering a "why", not restating the line under it (why `saturating_sub`
  and not `now_ms + RECEIVE_WINDOW_MS`, why the window is a parameter and not
  a field of `InboundMessage`, why `append` and `clock()` take no time, why
  `SqliteOpLog::clock` can't be `SELECT MAX(score_epoch)`). I did not find a
  restated-code comment among the additions.
- **Terminology is consistent across files.** "the author's unverified claim
  about the time, raised above every counter the author held" (or a close
  paraphrase) recurs in `arrival.rs`, `feed.rs`, `thread.rs`, `revision.rs`,
  `wire.rs` and `op.rs` — each file restates it locally rather than
  cross-referencing, which matches this codebase's existing convention of
  self-contained module docs (see CLAUDE.md's own style) rather than being
  drift.

One cosmetic-only observation, not filed as a checkbox because there is
nothing to act on: `thread.rs`'s test
`a_reply_carrying_a_lower_counter_than_the_reply_it_answers_comes_before_it`
has a comment paragraph where a parenthetical aside ends and the next
sentence starts on the very next line with no blank line
(`...for the dishonest one.)` immediately followed by `` `thread.rs` takes
counters ``, continuing on the line after that). It reads correctly as
continuous prose once you know it's one paragraph, but the line break lands
mid-thought in a way the rest of the file's doc comments don't. Not worth a
box.

## Verification run from this tree

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: all green (1168 passed in the lib suite, 30 in
  `end_to_end.rs`, 0 failed).
- `nix build ./dialectica#lgx` (run from the worktree root): succeeded, no
  output, clean exit.
