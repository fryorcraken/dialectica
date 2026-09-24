# Readability review — `thread-reply-order` (#147)

Scope: readability only (one of four `code-reviewer` dimensions on this piece).
Diffed `origin/main...HEAD`. Files in scope: `dialectica-core/src/thread.rs`,
`dialectica-core/tests/end_to_end.rs`, and the `openspec/changes/thread-reply-order/`
prose (`proposal.md`, `design.md`, `specs/thread-read/spec.md`) plus the amended
`openspec/specs/thread-read/spec.md`.

## Findings

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/thread.rs:63-65`,
      `:77`, `:690`; `openspec/changes/thread-reply-order/design.md:92`, `:139`
      — prose edited in place without re-flowing the paragraph, leaving ragged
      short/long lines against the file's own wrap width.
      **Scenario:** in `thread.rs`'s module doc, lines 63-65 read `There is no
      \`sort\`, no \`cmp\` and no` / `` \`max_by\` — the same `` / `discipline
      [...] hold` — a sentence broken into a near-empty middle line. Three
      lines later (line 77), a single line runs 134 characters — `` `cmp_ops`
      falls back to ascending op id, so this read returns them in DESCENDING op
      id, a hash carrying no temporal meaning at all. Two peers holding the
      same ops return the same sequence; neither can `` — where every
      neighbouring line in the same paragraph sits at 70-85 characters. The
      same pattern repeats at the inline loop comment (`thread.rs:690`, `` //
      re-sorted: this read compares `` as an orphaned short line) and twice in
      `design.md`, whose own paragraphs elsewhere hold to roughly 67-85
      characters (measured against this file's own distribution and against
      the archived `2026-09-16-op-clock/design.md`, which peaks at the same
      width outside one long inline-code line): `design.md:92` runs 122
      characters (`` feed's `latestReply` is defined as the rule's "places
      first". So the rule keeps its direction and the thread read presents ``)
      and `design.md:139` runs 91 (`` thread, after every answer to it. The
      bound, and ordering by an over-bound counter, are ``).
      **Why it matters here specifically:** this codebase treats these
      comments as load-bearing documentation of an invariant ("no comparison is
      written here"), not decoration — CLAUDE.md's own bar is that a comment
      "earns its place," and the module header this sits in is the one place a
      future editor reads before touching the ordering logic. Ragged wrapping
      is cosmetic and does not change what any sentence asserts, but it is the
      visible symptom of an edit made without reflowing its paragraph, and it
      is the kind of thing that compounds: the next edit to the same paragraph
      either re-wraps the whole block from scratch or adds a sixth ragged line
      next to the other five.
      **Severity:** low / cosmetic. Does not affect correctness, compiles and
      tests cleanly, `cargo fmt --check` does not touch doc comments' prose.
      Not blocking on its own; worth a pass to re-wrap the five spans named
      above.

## Clean

- Test naming throughout the new tests (`a_reply_orders_after_the_reply_it_answers`,
  `a_reply_carrying_a_lower_counter_than_the_reply_it_answers_comes_before_it`,
  `the_lowest_counter_leads_the_replies_and_the_highest_ends_them`,
  `replies_with_equal_counters_are_reversed_rather_than_re_sorted`,
  `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`) states the
  scenario each test pins, matching the file's existing convention and each
  requirement's title in `specs/thread-read/spec.md`.
- The new `a_root_at` helper (`thread.rs:3088`) is defined immediately before
  its first use rather than grouped with `a_root`/`a_reply` near the top of the
  test module — checked against `a_reply_at` (an existing helper with the same
  "defined near first use, not near its sibling" placement), so this follows
  an established convention in the file rather than deviating from one.
  `for (what, root) in [...]` in `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`
  (`thread.rs:3260`) is likewise an existing table-driven-test idiom in this
  crate (`arrival.rs:869`, `wire.rs:8411`, `wire.rs:16521`), not a one-off.
- `end_to_end.rs:2215`'s `expected_replies.sort_by(|a, b| b.cmp(a))` for a
  descending sort is the only descending sort in the crate (every other `.sort()`
  in `dialectica-core` is ascending), so it introduces no inconsistency to flag
  — noted as a stylistic observation only, not a defect: `sort_by_key(Reverse)`
  or `.sort(); .reverse()` would read slightly more intention-revealing, but
  this is a judgment call, not a correctness or clarity problem, and I am not
  opening a box for it.
- The core loop change itself (`log.iter_stoa(stoa)?.into_iter().rev()`,
  `thread.rs:698`) is a one-line, single-purpose change exactly as `design.md`
  Decision 1 describes it; the accompanying comment block explains *why*
  reversal was chosen over a re-sort rather than restating what the line does,
  which is the bar CLAUDE.md sets for a comment earning its place.
- `thread.rs` is `cargo fmt`-clean (verified: the only `cargo fmt --check`
  diffs in the `dialectica-core` package are in `identity.rs` and `wire.rs`,
  the pre-existing drift `design.md`'s Non-Goals explicitly excludes from this
  change).
