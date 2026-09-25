# Architecture review — time-pegged-clock

Dimension covered: **architecture only** (correctness, security and readability
are separate reviewer instances).

## Summary

The change is unusually well organised: one predicate (`exceeds_receive_window`)
with one caller (`transport::receive`), one function (`next_counter`) with one
production call site (`authoring::publish`), no new dependencies, no wire-format
change (`op.rs`'s diff is comments only), and the adapter genuinely untouched as
`design.md`'s Non-Goals claim. `design.md` Decision 4 gives a real, checkable
reason the window sits at `receive` and not in `OpLog::append` or on
`InboundMessage` (the rebuild/replay/restore paths have no time to give a
predicate that needs one), and Decision 10 walks all three readers of the
ordering rule's leading entry through the new hour-of-lead exposure rather than
leaving it asserted once and unverified.

One gap found, in the doc comments of a module `design.md` itself analyses.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/moderation.rs:101-107`
      — stale "causal, not temporal" reasoning survives in the one reader of
      `cmp_ops`'s leading entry that the sweep (tasks.md 4.1) did not touch, and
      it now contradicts what six sibling modules say.

      **Scenario:** tasks.md 4.1 claims "Doc comments citing `ADVANCE_BOUND`, or
      'causal, not temporal' as the reason a counter is not a time, rewritten in
      `op.rs`, `asserted_time.rs`, `thread.rs`, `feed.rs`, `log/mod.rs`,
      `revision.rs` and `wire.rs`" and verifies it with
      `git grep -n -e ADVANCE_BOUND -- dialectica/rust-lib` finding only
      "replaced" sentences. That grep is scoped to `ADVANCE_BOUND` and cannot
      see the second phrase it names in the same sentence. `moderation.rs` is
      absent from the list, and its module doc still reads (lines 103-104):

      > A Lamport counter is causal rather than temporal, and among ops
      > carrying none `cmp_ops` falls back to *ascending op id* …

      That is the exact phrasing this change withdraws everywhere else — the
      counter is now pegged to its author's clock and is an unverified *claim*
      about the time (`arrival.rs`'s module doc, `revision.rs`, `thread.rs`,
      `feed.rs`, `wire.rs`, `log/mod.rs` all say so in the words this diff
      introduces). A reader who opens `moderation.rs` after `arrival.rs` is told
      two incompatible things about the same field by the same crate.

      This is not a stylistic nit: `design.md` Decision 10 names
      `moderation::resolve` as one of exactly three readers it walks through the
      new exposure for — "A moderator can sign a `hide` or `unhide` up to an
      hour ahead and beat an opposite action that another moderator published
      within that hour without having received it" — and says the property "is
      specified once, in `op-ordering` … because every reader inherits it from
      the rule. No reader has code of its own for it." `revision.rs` was in fact
      given a reader-specific line reflecting this ("a version signed ahead of
      it leads the author's other devices' versions for up to the receive
      window's hour"). `moderation.rs`, the reader `design.md` spends the most
      words on (a moderator race, not merely an author's own devices racing
      themselves), was given nothing — not the corrected general claim, and not
      a reader-specific one. `git grep -i hour -- moderation.rs` and
      `git grep -i "causal, not temporal" -- moderation.rs`-style checks (I used
      `git grep -n -i -F -e hour` and `-e "advance bound"`) both confirm: zero
      hits for "hour" anywhere in the file, and the old phrasing is the only
      thing there.

      **Fix is a doc-only change**: reword `moderation.rs:103-104` the way
      `revision.rs` and `thread.rs` were reworded, and consider adding the
      moderator-race sentence from `design.md` Decision 10 near `resolve`'s own
      doc comment (around line 422, where the last-write-wins bias for the
      degraded branch is already discussed), so the risk lives beside the code
      that carries it rather than only in an OpenSpec change folder that will be
      archived.

      **Fixed** in the commit that ticks this box: both halves as suggested. The
      module doc (`moderation.rs:101-111`) now gives the counter as its
      author's unverified claim about the time, in the wording the sibling
      modules use. `resolve`'s doc carries the moderator race from Decision 10
      beside the last-write-wins paragraph it qualifies. The same sweep found
      two more stale sites the first pass missed, both in `revision.rs`: the
      module doc (line 84, "says nothing about wall-clock time") and
      `CurrentVersion::current` ("Neither is a temporal one"). Both are
      reworded. `tasks.md` 4.1 now lists `moderation.rs` and adds the second
      grep (`git grep -n -i -e temporal -- dialectica/rust-lib`). Every hit
      that grep has left is about op ids, which do carry no recency. **No test:**
      the change is doc-only, and no layer can see a doc comment. What checks
      it is that grep, and it cannot tell a stale sentence from a correct one
      that uses the same word.

## What I checked and found clean

- **No duplicate window logic.** `RECEIVE_WINDOW_MS` / `exceeds_receive_window`
  has exactly one production caller; the one-hour literal appears only in that
  pair and in tests that deliberately hardcode it independent of the constant
  (`arrival.rs`, `transport.rs`), which is the intended mutation-proofing, not
  duplication.
- **`asserted_time::FUTURE_ALLOWANCE_MS` and `arrival::RECEIVE_WINDOW_MS` are
  deliberately kept apart**, and `asserted_time.rs` now says so explicitly
  ("Not the receive window, and not to be merged with it"). Two similar-looking
  saturating-arithmetic clamps solving two different problems (display clamp vs.
  admission control) — correctly not merged into one "time bound" abstraction.
- **No wire-format change.** `op.rs`'s diff is comments only; `MAX_FIELD_LEN`,
  `LAYOUT_VERSION` and field widths are untouched, matching the owner's "Out of
  scope: changing the op format."
- **No new dependency.** No `Cargo.toml` or `Cargo.lock` diff.
- **The adapter is genuinely untouched.** `git grep -n receive\( -- dialectica/rust-lib/src`
  and a search for `InboundRefusal::` outside `dialectica-core` both return
  nothing, matching `design.md`'s claim that `receive` has no caller yet and
  needed no adapter change.
- **`next_counter`'s widened signature has exactly one production call site**
  (`authoring::publish`), so the API widening is not a "flag right at every call
  site" situation CLAUDE.md warns about.
- **`receive`'s new `now_ms: u64` parameter** is a bare argument rather than a
  new wrapper struct; given `receive` already takes three positional arguments
  and `now_ms` is a single primitive with a well-documented reason not to live
  on `InboundMessage` (Decision 2), this is proportionate rather than
  under-engineered.

## Branch / commit

Branch: `worktree-agent-a5f9454b6d2e4f956`. Findings committed as this file only
(`openspec/changes/time-pegged-clock/findings/architecture.md`); no other paths
touched, no mutation left in the tree for this dimension.
