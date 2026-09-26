# Design review — position-and-index (#166)

Read the owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024,
2026-09-25) before this review. It settles item 2: a malformed `index` is
refused with an error naming `index`; leaving the message unspecified is
ruled out.

Checked:

- `design.md` D1–D5, Goals/Non-Goals, Risks against the production code
  (`parse_index`, `keep_identity`, `thread_page_json`, `read_thread` in
  `thread.rs`/`wire.rs`) and against the tests added in
  `git diff origin/main...HEAD`.
- `design.md`, `proposal.md` and both spec deltas (`identity-onboarding`,
  `thread-read`) against issue #166's body and the owner's decision comment.
- The reconciliation commit (`fe9ea3d`, "Bring position-and-index's design in
  line with the place rule") against the current `thread-read` delta, for
  anything still describing the pre-reconciliation state.

## Result: no blocking findings

Every Decision holds up against the code:

- **D1** (`index` keeps sharing `parse_index`): confirmed — `keep_identity`
  calls `parse_index(&parsed, "index")` and every arm formats `{field}` into
  its message, so the shared guard is real, not asserted.
- **D2/D3** (wire-level property tests, shared-author fixture): confirmed —
  `a_thread_log_with_shared_authors` has two author-sharing pairs and asserts
  `authors.len() < len`; `no_two_items_of_a_thread_share_a_position_…` and
  `a_position_is_the_same_whatever_page_size_…` exist as described, with the
  "what these two cannot see" gap (an op id) correctly called out and pointed
  at D5.
- **D4** (raw-JSON-text keep helper): confirmed — `keep_with_raw_index`
  exists, `MALFORMED_INDEXES` is the shared table both by-name and
  stores-nothing tests iterate, matching the spec's 8-entry scenario list
  exactly (`-1`, `1.5`, `0.0`, `1e2`, `"two"`, `[]`, `true`,
  `18446744073709551616`).
- **D5** (place rule, two-reads test): confirmed against both the code and
  the spec text. `thread.rs`'s `resolve_item`/`read_thread` genuinely drop a
  hidden reply before enumerating (`placed.at(index)`), so "position is
  determined by place alone" is what the code does, not just what the test
  asserts. `the_item_at_a_place_carries_that_places_position_in_every_read`
  matches the scenario's two clauses precisely.

The spec moved once mid-piece (`2ac71ef` adding the `thread-read` place-rule
scenario, `fe9ea3d` reconciling `design.md`). Nothing in the current
`design.md` still describes the pre-reconciliation state: Context, Goals,
D2, and Risks were all rewritten in `fe9ea3d`, and the current text
(uniqueness narrowed to "one read", the place-rule paragraph, D5) matches
`specs/thread-read/spec.md` as it now stands line for line.

`identity-onboarding`'s new requirement matches `feed-read`'s existing list
of malformed kinds for `page`/`perPage` (`spec.md:555`) exactly, which is the
right basis for "no spec requires it yet" → "add the requirement to the
capability that owns `keep_identity`" that issue #166 and the owner's
decision comment call for. The owner's comment names three example kinds
(negative, fractional, not a number); the requirement's fuller list (also
covering an integer written with a decimal point/exponent, and one too
large to represent) is justified in `proposal.md` as following `feed-read`'s
existing list rather than narrowing or contradicting the owner's decision.

`proposal.md`'s "Deliberately left unspecified" section and `design.md`'s
Risks entry agree on the `u64::MAX + 1` wrong-reason issue, and both record
that a fix belongs in a separate issue against the parser — this is the
"reasoning migrates" case done correctly: the issue's mid-piece discovery
(the op-id gap in the position requirement) was moved into the spec itself
(`thread-read`'s new place-rule paragraph and scenario), not left sitting
only in `design.md` or the issue thread.

## One non-blocking suggestion

- [ ] **`dev-writer`** — `design.md` D5's "What breaks without it" still
      reads in the future tense: *"is for the `tester` to measure, under
      task 3.4"*. Task 3.4 is now done (`tasks.md` ticked, commit `1889eaf`),
      and its commit message already carries the actual mutation evidence
      ("Proven red with the position set to the item's op id … Proven green
      against the untouched implementation"). Not a defect — CLAUDE.md
      accepts a commit message as the mutation evidence's home — but a
      one-line update to D5 pointing at `1889eaf` instead of describing the
      proof as pending would stop a future reader from wondering whether the
      guard was ever actually measured. Purely cosmetic; does not block
      merge.
