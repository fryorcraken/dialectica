# Design review — get-stoa

No findings. `design.md`'s Decisions section is in good shape; nothing below is a
box to tick.

## What was checked

**Decisions vs. code.** Each of the 16 entries under Decisions was traced to
where the code implements it:

- Decision 1 (resolver reads by Stoa via `iter_stoa`, not `iter_target`) —
  `stoa_metadata.rs`'s module doc and `resolve()` match exactly; the
  `log/mod.rs` `iter_target` doc it cites is unchanged and does predict this
  shape, confirmed by reading it directly rather than trusting the quote.
- Decision 2 (`authorises` made `pub(crate)`, reused unchanged) — confirmed at
  `moderation.rs:220`. Every test name decision 2 cites as mutation evidence
  (`a_metadata_op_by_anyone_but_the_creator_does_not_bind`,
  `the_resolver_does_not_trust_the_read_to_have_scoped_the_ops`, and the rest)
  exists verbatim in `stoa_metadata.rs`.
- Decisions 3–7 (ordering, no degraded-branch preference, the `CurrentMetadata`
  enum, `Founding::of`, `Result` never a fallback) — all present in
  `stoa_metadata.rs` as described, including the "kind checked before
  authority" ordering inside `binding_metadata`.
- Decisions 8–13 (request shape `{stoa, genesis}`, no state-changing
  capability, verify-before-open, no `foundingTitle`, `isGenesisFallback`,
  `policy_name` reuse) — all present in `wire::get_stoa` and
  `stoa_metadata_json` at `wire.rs:2207–2265`, in the order design.md
  describes.
- Decision 14 (envelope sweep entry; adapter gated by `cfg(logos_scaffold)`) —
  confirmed: `dialectica/rust-lib/src/lib.rs:808` gates
  `impl DialecticaModule for Dialectica`, and `get_stoa`'s handler
  (line 954) opens only the op log, matching `list_threads`' shape.
- Decision 15 (#98 lands without #125) — read issue #125 fresh, including its
  one comment ("Whoever picks either up should check whether they're better
  landed as one piece"). Decision 15's argument (read side complete and
  testable alone; publish side needs its own spec delta, moderator-authoring
  path and UI, and is 0.0.2) is a direct, adequate answer to that comment
  rather than a decision taken in passing.
- Decision 16 (address case: parse either case, always report lowercase) —
  matches `identity`'s spec delta and the wire code (`stoa.to_hex()` from the
  parsed `Address`, never the request string), and the cited test
  (`an_address_asked_for_in_uppercase_is_answered_in_lowercase`) exists and
  checks both halves.

No entry was found partially applied, and no gap between a recorded decision
and its call sites was found.

**Reasoning migration (the three items named for this review).** All three are
present in `design.md` and faithful to the issue text read fresh:

- `isGenesisFallback` rationale (decision 12) — carries the issue's own
  sentence ("this could be years stale") and the retired-plan citation behind
  it.
- "The moderation resolver with a different subject" / why `iter_stoa` not
  `iter_target` (decision 1) — carries the exact `log/mod.rs` doc quote, which
  I re-read directly and confirmed still says what the quote claims.
- Why #98 lands without #125 (decision 15) — addresses issue #125's actual
  coordination request, not a paraphrase of it.

**Code decided but not recorded.** Checked for constants, discriminants,
refusals, and comment-length justifications not backed by a Decisions entry.
Found none: `policy_name`'s `"open"` literal is marked `NO SPEC` in the code
and correctly attributed to `design.md` (which does discuss it, decision 13);
the `MAX_TITLE_BYTES`-derived genesis cap is inherited, not decided here.

**Contradiction with issue #98.** Read issue #98 fresh via `gh issue view`.
The implementation matches its "What this needs" section exactly: the
resolver shape, the `getStoa` wire shape (module accepted the spec-writer's
`{stoa, genesis}` widening from the issue's `{stoa}}` sketch, and design.md
decision 8 argues it rather than doing it silently), and `isGenesisFallback`.
No undisclosed departure from the issue's stated scope.

**Entry quality.** Every entry has what-was-chosen, the forcing constraint,
alternatives considered (with what ruled each out), and cost. Every
guard-shaped decision (2, 3, 4, 7, 9, 10, 16) carries "what breaks without it"
mutation evidence naming specific tests, and each cited test name was checked
to exist.
