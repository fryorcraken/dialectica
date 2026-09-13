# Findings — readability review, `op-transport`

Read: `dialectica/rust-lib/dialectica-core/src/transport.rs` in full (2,506 lines),
`design.md` (378), `tasks.md` (261), the spec delta, and `docs/UI-BRIEF.md` where
`tasks.md` §10 points at it. Every count, name and magnitude below was checked with
a command rather than taken on plausibility, per the brief's instruction and the
recurring defect it names.

Baseline confirmed: **626 tests passing**.

## Findings

- [ ] **`dev-writer`** — `design.md:230` — **a third phantom name in this file's
      `design.md`, and it is the one the brief asked for a sweep for.** The sentence
      reads "the test `the_timestamp_reaches_nothing_that_is_recorded` is what holds
      that." No such test exists.
      **Measured:** `grep -rn "the_timestamp_reaches_nothing_that_is_recorded"` over
      `openspec/`, `dialectica/` and `docs/` returns **exactly one hit — that line
      itself**. The two tests that actually hold the property are
      `the_arrival_timestamp_is_not_recorded_as_ordering_metadata`
      (`transport.rs:1110`) and `the_timestamp_handed_in_does_not_change_what_is_
      recorded` (`transport.rs:1138`).
      **What a reader would wrongly conclude:** that one named test guards the
      `timestamp`-is-dropped property, and — following the brief's own warning about
      unread citations — that they can find it by name. They cannot, so the check
      either stops (the claim is taken on trust) or the reader concludes the property
      is unguarded, when in fact six tests killed the Lamport mutation. This is the
      **third** phantom in this one document after `PublishOutcome` (cited twice) and
      the "cannot be done" paragraph, which makes it a pattern rather than a slip.
      **Severity: medium** — no wrong behaviour, but the document's remaining
      citations now carry less weight than they earn, and two of the three previous
      ones were found by a reviewer rather than by the author.

- [ ] **`dev-writer`** — `tasks.md:186` — **`tasks.md` contradicts itself about the
      `// NO SPEC:` marker within 47 lines.** §6 line 139 states: "**No `// NO SPEC:`
      marker remains in this change.**" §7 line 186 then says: "It is not, by anyone,
      and no test claims otherwise. **See the `// NO SPEC:` marker** and `design.md`."
      **Measured:** `grep -rn "NO SPEC" dialectica/rust-lib/…/transport.rs` returns
      one hit, `:2199` — *"This was a `NO SPEC:` marker until that requirement
      existed"* — i.e. prose *about* a marker that is gone. So §6 is right and §7
      points at nothing, in the same file, both ticked.
      **What a reader would wrongly conclude:** that an unspecified-behaviour marker
      is still live and still the record for the delivery-outcome gap — which is
      exactly the marker the spec-test review had removed and replaced with a
      requirement. A reader chasing §7's pointer to find out what is uncovered finds
      a sentence saying the marker was deleted, and has to reconstruct which of the
      two ticked claims is current.
      **Severity: medium** — the brief lists this exact hazard ("check any count,
      duration or magnitude against a command"), and §7 is the one section whose
      whole job is to say honestly what the gate cannot see.

- [ ] **`tester`** — `transport.rs:819` — **the test name
      `identity_does_not_vary_with_local_state` states a falsifiable claim the test
      does not establish**, and the requirement it answered no longer carries that
      wording.
      The spec-test reviewer measured this and left the rename as `tester`'s call
      (`findings/spec-test.md:171-175`): a mutation appending `std::process::id()` —
      local state, stable within a peer, different between peers — **passed this
      test**, and was caught only by `the_derivation_is_pinned_to_a_known_answer`,
      there reported as "the derivation changed". The spec has since been rewritten:
      the scenario is now **"Identity does not vary with the peer's history"**, and
      the test's own body comment already describes exactly that narrower property.
      **Measured:** `grep -n "Identity does not vary with local state"
      specs/op-transport/spec.md` returns **zero hits** — the scenario the test is
      named after is gone from the contract it answers.
      **What a reader would wrongly conclude:** that the derivation is pinned against
      local state generally, which is the strongest-sounding and least-held reading of
      the file — and the one that matters, because the requirement's own prose says
      this failure "produces no error". Rename to match the surviving scenario
      (`identity_does_not_vary_with_the_peers_history`), so the name describes the
      mechanism the body witnesses rather than the absence it cannot prove.
      **Severity: medium** — the strongest claim in the file sits on the weakest
      witness, and the file's other names are honest about their mechanism, which
      makes this one read as stronger than its neighbours rather than weaker.

- [ ] **`dev-writer`** — `transport.rs:841` — **a dead binding standing in for an
      assertion.** `let _another_peers_key = a_key(200).public_key();` with the
      comment "Another peer's identity in the same process, which is as close as a
      unit test gets to 'a different peer identity'."
      The binding is never read and reaches nothing; the derivation two lines below
      does not take a key. The line participates in no property.
      **What a reader would wrongly conclude:** that the assertion below varies a peer
      identity. It does not — and once the spec split the scenario (finding 3), the
      construction half is discharged by reading `of`'s signature, so this line no
      longer even gestures at an obligation. It is the shape CLAUDE.md warns about
      from the other direction: a comment saying what the code does not do.
      Delete the binding and let the comment above it say the thing plainly, or move
      the sentence into the module doc beside the "held by construction" argument
      where it is true.
      **Severity: low** — cosmetic, but it is the one line in a 59-test file that a
      reader can mistake for a check.

- [ ] **`dev-writer`** — `transport.rs:1911` — **`surrogputesque` is not a word**,
      in the comment for `a_hostile_channel_or_sender_identifier_does_not_panic`:
      "control bytes, lone surrogputesque sequences expressed as valid UTF-8".
      The sentence it sits in is doing real work — it is the honest scope statement
      that a `String` of raw invalid UTF-8 is unrepresentable in Rust and therefore
      not a reachable input — so the typo lands on the one clause a reader has to
      parse carefully to know what the test does *not* cover.
      **What a reader would wrongly conclude:** most likely nothing, but a reader
      checking whether surrogates are covered cannot tell whether "surrogate-esque"
      or "surrogate" was meant, and the difference is whether the fixture list at
      `:1919-1929` is claimed to include them (it does not — `\u{FFFD}` is the
      replacement character, not a surrogate).
      **Severity: low** — stylistic, listed separately from the defects above.

## Areas that are clean

Stated in prose, since none needs action.

**Every other quantity in the file and in `design.md` checks out.** I verified each
against a command rather than reading past it:

- `MAX_MESSAGE_BYTES = 150 * 1024` and `op.rs:146`'s `MAX_FIELD_LEN = 150 * 1024`
  are two constants with one value, exactly as `transport.rs:83-94` and
  `design.md:161-174` describe, and `op.rs`'s own docs do say the field cap "does
  NOT bound the total size of a decoded op" — the quotation is accurate.
- `the_derivation_is_pinned_to_a_known_answer:871`'s working re-checks: the
  `printf` preimage is 26 characters plus six NULs plus a 16-byte record = 48 bytes,
  and the comment carries the command so the next reader can repeat it rather than
  trust it. This is the citation discipline the brief asks for, done right.
- `receive`'s doc claim that `op-format`'s "own suite covers every prefix of a valid
  op" is true: `op.rs:1522` and `op.rs:1776` each run `for n in 0..len`.
- `design.md:318`'s "reused rather than re-written" naming `wire.rs`'s
  `channel_exists_reply` and `callee_error` — both exist, at `wire.rs:1089` and
  `wire.rs:1066`.
- `tasks.md:249`'s claim that `docs/UI-BRIEF.md` "gained the rendering obligation as
  its **obligation 7**" is true; it is at `UI-BRIEF.md:529`, and the wording matches
  ("a successful publish means 'saved here', not 'posted'"). I checked this because
  the brief warned that the last reviewer caught an invented UI-BRIEF section.
- `design.md`'s Decisions holds nine `###` sections against `tasks.md:47`'s claim of
  "the eight choices"; the ninth is "What is left in the adapter", which `tasks.md`
  lists separately in the same bullet, so the two agree and the count is not wrong.

**The comments earn their place, which is the harder half of this dimension.**
Almost none restates its code. The ones that matter say *why not the obvious
alternative*: why a second size constant rather than a re-export (`:83-94`), why
`HashMap` rather than `HashSet` (`:183-195`), why `Arrival::unordered()` rather than
`from_parts(None, None)` (`:476-479`), why the channel-id discriminant differs from
the topic's (`:106-116`), why `close_all` sorts (`:237-243`). Each of those is a
question a reader would ask, answered where they would ask it. The `// BEFORE the
decode.` and `// FIRST.` markers at `:448` and `:586` are the right length for what
they carry — an ordering that is a requirement, flagged at the line that could be
reordered.

**The absent comments are absent in the right places too.** `is_open`, `len`,
`stoa_of` and `new` carry one line or none, which is correct: nothing about them
would make a reader ask why.

**It reads like the rest of the codebase, and in one respect better.** The
`Refusal`/`Display`/`every_refusal_variant()` trio with its non-exhaustive `match`
is `stoa.rs`'s house style carried forward, including the specific lesson
(`stoa.rs` recording that a bare array drifted when `TitleTooLong` was added). The
`assert!(…, "the fixture must decode, or this is the decode test")` guards on the
four refusal fixtures are the habit this file establishes that the next capability
should copy — they are what stops a fixture silently drifting into testing an
earlier gate, and they are stated as one line rather than argued.

**The section dividers** (`// ─── Channel identity ───`) are a genuine navigation
aid in a 1,900-line test module and are consistent throughout.
