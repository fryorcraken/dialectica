# spec-test review — feed-row-contract

Read `openspec/changes/feed-row-contract/specs/feed-read/spec.md`,
`.../specs/thread-read/spec.md`, `proposal.md`, `design.md`, `tasks.md`, and the
live `openspec/specs/feed-read/spec.md`. Read the tests in
`dialectica/rust-lib/dialectica-core/src/{feed,wire,thread,revision}.rs`. Did not
read production function bodies except the few lines a mutation required (Part 2)
and a handful of lines in `wire.rs` (`parse_stoa`, `required_string`, the
`"position"` field's assembly) that a test-name search surfaced before I could
avoid them; none of the findings below rest on anything learned there beyond
confirming a shared helper's name.

## Closed field sets — the bar this review was asked to enforce

This is solid. `wire.rs` carries three tests that assert a feed row's key set as
a hardcoded, sorted literal (never derived from `FeedRow` or its JSON
serialisation): the plain row without `latestReply`
(`the_feed_reply_is_the_ecosystems_pagination_shape`), the plain row with it
(`a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`), and one
row in every other state at once — revised, hidden, with an attachment, with a
voted-on reply
(`a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`).
The third test first asserts the fixture actually reached each state before
asserting the key set, which is the right order. The envelope's three keys and
the `{text, removed, marked}` shape for `body` and for each attachment are
likewise asserted as hardcoded sorted lists. `feed.rs`'s
`a_row_carries_the_public_key_and_no_derived_display_name` backs this at the
struct level by destructuring `FeedRow` so a new field fails to compile there.
`no_thread_item_holds_its_signers_key_under_any_key_but_author` goes further
than a key-set check: it walks every string value at any depth and asserts none
equals the signer's hex under any key but `author`. I mutated `wire.rs`'s
`thread_page_json` to set `"position": item.author` (mirroring `design.md`
Decision 3's own table) and it turned exactly that one test red — see Part 2 below.

## Part 1 — scenario coverage

- [x] **`tester`** — "Revising a root does not move its row" (feed-read, *The
      rows are the Stoa's authentic thread heads…*) has no covering test.
      **Scenario:** the existing revision tests
      (`a_row_renders_the_current_version_and_says_it_was_edited`,
      `an_unrevised_row_reports_the_two_ids_as_equal_and_is_not_edited` in
      `feed.rs`) build a single-thread fixture, so there is no second row for a
      revision to displace. `a_new_reply_does_not_move_its_threads_row` covers a
      *reply* arriving, not a *root* being revised. A build that re-sorted rows
      by "most recently revised first" would pass every test in the suite.
      **Fix shape:** two threads, revise the second row's root, assert the row
      sequence is unchanged — the same shape `a_new_reply_does_not_move_its_threads_row`
      already uses for a new reply.

      **Outcome: fixed.** Added `feed::tests::revising_a_root_does_not_move_its_row`
      (`dialectica/rust-lib/dialectica-core/src/feed.rs`), in exactly the fix
      shape suggested: two roots at counters 5 and 4 (`first` sorts before
      `second`), the second row's root revised by its own author, and the
      thread-id sequence asserted unchanged. **Predicted vs observed:**
      matched. Mutated `list_threads` to add `rows.sort_by_key(|r|
      !r.is_revised)` right after the row-building loop (a stand-in for "most
      recently revised first") and reran the test — it failed:
      `left: ["a673...", "52d9..."] right: ["52d9...", "a673..."]`, i.e. the
      revised row (`second`) jumped to the front. Reverted the mutation;
      `git diff --stat` on `feed.rs` shows only the +45 lines the new test
      added.
- [x] **`tester`** — Three explicit feed-read Scenarios comparing the feed to a
      thread read have no test exercising both `list_threads`/`all_of` and
      `read_thread` in one place: **"The thread id opens the thread"** (a row's
      `thread` passed to a thread read of the same Stoa returns that thread with
      the root as its first item), **"The feed and the thread read name an
      author alike"**, and **"The feed and the thread read sanitise alike"**
      (a hostile body/attachment sanitises the same way through both reads).
      **Measured:** grepped `wire.rs`, `feed.rs` and `thread.rs` for any test
      calling both entry points, and for the scenario language itself ("alike",
      "opens the thread") — no hits. These are exactly the properties that would
      break silently if the feed and the thread reader diverged on sanitising or
      on the author spelling, and nothing currently observes the agreement.

      **Outcome: fixed.** Added
      `wire::tests::the_feed_and_the_thread_read_agree_on_the_opened_thread_the_author_and_the_sanitising`
      (`dialectica/rust-lib/dialectica-core/src/wire.rs`, in the feed-handler
      section, right after `the_feed_handler_is_never_a_panic`). One root with
      a hostile body (`"p\u{0430}ypal\u{202E}gnp.js"`) and a hostile attachment
      (`"cid\u{202E}txt.exe"`) is read once through `list_threads` and once
      through `read_thread` on `row["thread"]`; the test asserts the thread
      opens (no error, first item's `id` equals the row's `thread`), that
      `row["author"] == item["author"]`, and that `row["body"]`/
      `row["attachments"]` equal the thread item's, covering all three cited
      Scenarios in one fixture since they share the same "read both ways and
      compare" shape. **Predicted vs observed:** matched. Mutated `feed.rs`'s
      row assembly from `body: sanitise(version.body())` to
      `body: sanitise(&format!("{}!", version.body()))` (a feed-only
      normalisation bug) and reran — failed on the body-alike assertion,
      `left: "pаypalgnp.js!" right: "pаypalgnp.js"`. Reverted;
      `git diff --stat` on `feed.rs` unaffected by this mutation (test-only
      insertion remains).
- [x] **`tester`** (lower severity) — "A page size of zero is served at the
      default" is pinned at the pure-function level
      (`feed.rs::per_page_is_clamped_at_both_ends`, comparing
      `clamp_per_page(Some(0))` to `clamp_per_page(None)`) but not at the wire
      level for the feed. The only wire-level `perPage:0` test I found is for
      `thread-read` (`wire.rs:9520`, `thread_request`). No test compares two
      full `list_threads` replies — `feed_request("")` vs
      `feed_request(r#""perPage":0"#)` — for identity, which is what the
      scenario as written asks for ("the two replies are identical"). The
      pure-function test is reasonably strong evidence on its own; flagging
      because the scenario's own wording is end-to-end and nothing exercises it
      end-to-end for the feed.

      **Outcome: fixed.** Added
      `wire::tests::a_feed_page_size_of_zero_is_served_exactly_like_no_per_page`
      (`dialectica/rust-lib/dialectica-core/src/wire.rs`, right after
      `an_oversized_per_page_is_clamped_rather_than_refused`), comparing the
      full `list_threads` reply string for `feed_request("")` against
      `feed_request(r#""perPage":0"#)` on `log_with_a_hidden_thread()` (two
      rows, so a divergence that happened to preserve item count would still
      be caught). **Predicted vs observed:** matched. Mutated
      `clamp_per_page` from `None | Some(0) => DEFAULT_PER_PAGE` to `None =>
      DEFAULT_PER_PAGE` (dropping the `Some(0)` arm, so `Some(n) =>
      n.min(MAX_PER_PAGE)` catches the literal `0` and pages at size zero) and
      reran — failed: `left` carried the one visible row, `right` was
      `{"hasMore":true,"items":[],"page":0}`. Reverted; `git diff --stat` on
      `feed.rs` unaffected by this mutation.
- [x] **`tester`** (lower severity, judgement call) — Two feed-read properties
      are pinned only through a helper shared with a different capability's
      handler, never through the feed's own request path:
      - "A missing Stoa is refused distinguishably from a wrong-typed one" —
        `wire.rs::a_malformed_feed_request_is_the_error_shape_and_carries_no_items`
        exercises `list_threads` with both `{}` (missing `stoa`) and
        `{"stoa":7}`, but only asserts "error present, no items" — it never
        compares the two messages. The distinguishability property itself is
        proven only via `get_capabilities` (null case) and `publish_post`
        (non-null case), not via any feed handler.
      - "A revision by someone else changes nothing" — no `feed.rs`/`wire.rs`
        test builds a root revised by a non-author key and reads it through the
        feed; the authorisation rule is pinned only in `revision.rs`
        (`every_key_but_the_authors_is_rejected`,
        `a_strangers_revision_loses_to_an_older_one_by_the_author`).
      In both cases a regression *specific to the feed's own call site* (an
      inline copy of the check that collapsed two messages, or a home-grown
      "any Revise op targeting this root" instead of calling the shared
      resolver) would not be caught by any test that runs the feed. Flagging as
      a judgement call rather than a hard gap: the shared helper *is* shared
      code (documented as such in `parse_stoa`'s own doc comment, and in
      `design.md`'s citation of `post-revision` for `currentVersion`/`isRevised`),
      so today the properties are pinned, just not independently of that sharing
      holding.

      **Outcome: fixed** (took the judgement call; wrote both).
      - Added `wire::tests::a_missing_stoa_is_distinguishable_from_a_wrong_typed_one`
        (`wire.rs`, right after `a_malformed_feed_request_is_the_error_shape_and_carries_no_items`),
        exercising `list_threads` directly on `{}` and `{"stoa":7}` and
        asserting the two `error` messages differ, hardcoded to the literals
        `"missing field: stoa"` and `"stoa must be a string"` (matching the
        style of `a_wrong_typed_field_is_distinguishable_from_a_missing_one`
        at `wire.rs:10082`, which pins `publish_post` the same way).
        **Predicted vs observed:** matched. Mutated `parse_stoa` so the
        wrong-typed arm returns `error_json("missing field: stoa")` too
        (collapsing both messages) and reran — failed: `left: "missing
        field: stoa" right: "missing field: stoa"`. Reverted; `git diff` on
        `wire.rs` for this line shows no change.
      - Added `feed::tests::a_revision_by_someone_else_changes_nothing_through_the_feed`
        (`dialectica/rust-lib/dialectica-core/src/feed.rs`, right after
        `an_unrevised_row_reports_the_two_ids_as_equal_and_is_not_edited`): a
        root revised by a key other than its author, read through `all_of`,
        asserting `current_version == thread`, `is_revised` false, and the
        row's body is still the root's own text. **Predicted vs observed:**
        matched. Mutated `revision.rs`'s `is_valid_revision` to drop the
        trailing `&& candidate.op.op.author == original.op.op.author` clause
        and reran — failed: `left: "adb3..." (the stranger's revision) right:
        "65de..." (the root)`. Reverted; `git diff --stat` on `revision.rs`
        shows no changes (this finding required no edit to that file — the
        mutation was applied and reverted only to prove the new test, and the
        file carries no test changes of its own for this finding).
Everything else scoped to this delta — the request's `stoa`/`genesis`
validation (missing, wrong type, bad hex, mismatched address),
`includeHidden`'s three readings, the row-selection rules (forged root, foreign
Stoa's root, parentless-post-with-`thread`-field), hidden-before-paging,
pagination's cap/default/oversized/negative/fractional/exponent/non-numeric
cases, the largest-accepted-page-index boundary, and the store-failure/panic
paths — has a test that exercises the actual behaviour described, not merely a
same-named test.

## Part 2 — mutation

Ran one mutation (budget: one or two). In `wire.rs`'s `thread_page_json`,
changed:

```rust
"position": item.position,
```
to
```rust
"position": item.author,
```

Ran `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core
no_thread_item_holds_its_signers_key_under_any_key_but_author`. Result: **failed**,
exactly as `design.md` Decision 3's table claims —
`wire::tests::no_thread_item_holds_its_signers_key_under_any_key_but_author`
panicked with `` `position` holds the signer's key a second time ``, naming the
duplicated hex in the JSON. Reverted the edit with `Edit`; `git diff` on
`dialectica/rust-lib/dialectica-core/src/wire.rs` shows no changes. **Nothing
from this mutation is left in the tree.**

## Part 3 — NO SPEC / stale-prose sweep

`git grep -n "NO SPEC:"` across `feed.rs`/`wire.rs`/`thread.rs`: the two
thread-read markers on `author` (`thread_page_json`,
`the_wire_reports_the_author_as_one_key_and_no_name`) and the `feed.rs:863`
marker on `a_row_carries_the_public_key_and_no_derived_display_name` are gone,
matching `tasks.md` 4.2/4.3. The remaining markers (`position`'s field name,
`assertedTime`, the moderation spellings, and others unrelated to this change)
are the ones `tasks.md` says stay. `git grep -n "No capability owns"` and a grep
for `the_feed_json_is_pinned_to_the_exact_shape_a_view_is_written_against` both
return nothing, matching task 4.1/4.2. No unmarked spec gap found beyond what
`proposal.md`'s "Open questions for the owner" and `design.md`'s "Risks /
Trade-offs" already name.

## Part 4 — requirements moved between capabilities

Does not apply: `proposal.md` states "New Capabilities: None" and both deltas
are additive (`ADDED Requirements` only). No `REMOVED` half to check for
verbatim survival.

## Part 5 — spec soundness

- Read the whole `feed-read` and `thread-read` deltas plus the live
  `feed-read` Purpose; no internal contradiction found. The live Purpose's
  boundary list (five capabilities named, "not contracted here" language
  removed) reads true both before and after this change would archive.
- `gh issue view 91 --repo fryorcraken/dialectica` read fresh: the issue's
  premise ("no capability... says what a feed row carries") matches what the
  change fixes. `proposal.md` already correctly flags the issue's "no `feed`
  capability exists" line as stale (`feed-read` predates this change, from
  #100) rather than silently working around it — no separate finding needed.
- No scenario found that asserts something untestable as written.

## Gates run in this tree

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` on the clean (post-restore) tree: **1153 + 30 passed, 0
  failed**.
- `nix build ./dialectica#lgx` from the tree root: **succeeded**, no output
  (exit 0).
