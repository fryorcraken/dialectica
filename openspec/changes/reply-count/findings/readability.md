# Readability review — `reply-count`

Scope: readability only, per dispatch. Diffed `origin/main...HEAD` (three dots)
in a worktree on branch `worktree-agent-a42f67d5db09aa3db`, forked from the
piece branch tip (`228dd89`). Read `feed.rs` in full (both the diff and the
surrounding file, since this module's header prose is part of its contract),
`wire.rs`'s diffed hunks plus the `thread_page_json` precedent it cites,
`end_to_end.rs`'s diff, `proposal.md`, `design.md`, and `specs/feed-read/spec.md`.
Built with `cargo test --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml`
after creating the gitignored SDK symlink — 60 unit tests plus 5 end-to-end
tests pass.

## Findings

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:1744` —
      `feed_page_json`'s guard conflates a guaranteed-`Some` with the one that
      actually varies, where the file's own precedent two hundred lines away
      keeps them visibly separate.
      **Scenario:** `thread_page_json` (line 2030) writes
      `if let Some(map) = object.as_object_mut() { if let Some(parent) = &item.parent { ... } }`
      and its comment explains *why* the outer `Some` is unconditional
      ("`as_object_mut` cannot fail on a value this function just built as an
      object"), then nests the field that is genuinely optional inside it. The
      new `feed_page_json` collapses both into one tuple pattern —
      `if let (Some(map), Some(latest)) = (object.as_object_mut(), row.latest_reply())`
      — so a reader sees two `Some`s of equal apparent weight and has to
      reconstruct, unaided by the code shape, that only the second one can ever
      be `None`. The accompanying comment ("`if let` rather than an unwrap, for
      the reason `thread_page_json` gives") points at the sibling function's
      reasoning but does not carry it over, so the reader who does not follow
      the pointer loses the distinction the sibling function took care to make
      visible. This is a legibility regression relative to a pattern the same
      file already established for the same problem, not a correctness issue —
      both branches behave identically today, and no test distinguishes the two
      shapes (confirmed: rewriting the new code as nested `if let`s, matching
      `thread_page_json`'s shape, is behaviour-preserving).
      **Severity:** low — stylistic drift from an established in-file
      precedent, not a defect. Worth a one-line fix (nest the two `if let`s as
      `thread_page_json` does) rather than a required change.
      **Fixed** in the commit "Nest feed_page_json's two if-lets as
      thread_page_json does". The outer `if let` is on `as_object_mut` and the
      inner one on `latest_reply()`. The comment now gives the reasoning itself
      rather than pointing at the sibling function. `design.md` Decision 4 says
      why the shape is nested. **No test fails without it**, because the change
      preserves behaviour, as the finding says. The full suite (1046 + 30),
      clippy `-D warnings` on both packages and `cargo fmt --check` are green
      after it.

## What was clean

- **`feed.rs`'s module header** is heavily rewritten (the section "A row
  reports its thread's visible replies") and reads well: it states plainly
  that the old argument against a reply count is withdrawn, gives the three
  rules that decide what the fold sees, and does not restate `thread_of` or
  `moderation::resolve`'s own logic — it points at them. No stale reference to
  the old paragraph's wording survived the rewrite.
- **No stale test-name citations.** The old pinning test
  `a_feed_row_carries_no_display_name` was renamed to
  `a_row_carries_the_public_key_and_no_derived_display_name` in an earlier
  change, and every place in `wire.rs` and `feed.rs`'s comments that cites it
  by name uses the current name — a plausible trap (rename a pinning test,
  leave a comment elsewhere quoting the old name) that did not happen here.
  Spot-checked five test names `design.md` cites by name
  (`the_count_agrees_with_the_thread_read`,
  `the_latest_reply_is_the_one_the_ordering_rule_places_first`,
  `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
  `a_hidden_reply_is_neither_counted_nor_latest`,
  `a_store_failure_on_a_reply_off_the_page_fails_the_page`) and all five exist
  verbatim in `feed.rs`.
- **`FeedRow::replies: Option<Replies>` reads clearly.** The doc comment on the
  field states the invariant it enforces (a row with replies has a latest one,
  a row without has neither) and points at the two accessor methods a caller
  should use instead of touching the field's shape directly. The accessors
  (`reply_count`, `latest_reply`) are short, single-purpose, and named for what
  they return rather than for the internal representation.
- **`visible_replies_by_thread`'s doc comment** is long but each of its five
  subsections (what is counted, latest-is-first, whole-Stoa scope, termination)
  answers one question a reader would actually ask before touching this
  function, rather than restating the loop body in prose. This matches the
  file's established documentation density elsewhere (`list_threads`,
  `FeedRow` itself) rather than being uniquely verbose to this change.
- **Test naming and structure in `feed.rs`'s new section** ("The reply count
  and the latest reply") follows the file's existing convention: names state
  the property under test as a sentence, comments explain *why* a fixture is
  shaped the way it is (e.g. why counters rather than op-id hashes decide
  order, why the forged reply also carries the highest counter), and the
  section banner comment at the top states the general shape every fixture
  below follows (assert the subtraction and its counterfactual). This is
  consistent with the rest of the file and made the diff easy to follow
  against the spec's scenarios.
- **`end_to_end.rs`'s diff** is minimal and legible: one field added to an
  exhaustive destructure, one assertion with a comment explaining why a vote
  produces no reply.

## Not filed as findings (out of lane)

- The design.md/spec.md cross-reference accuracy is design-reviewer /
  spec-test-reviewer territory; noted above only as "clean" because I read it
  in the course of checking comment citations, not as an independent finding.
- I did not run `cargo mutants` — that is squarely a correctness-dimension
  tool (per this agent's own brief, mutation testing belongs to whichever
  instance covers correctness) and this dispatch named readability only.
