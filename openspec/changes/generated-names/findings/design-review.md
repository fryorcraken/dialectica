# design-review — generated-names

Scope: do D1–D13 match the code, were decisions worth recording recorded, does
anything contradict `docs/PLAN.md`, and did reasoning migrate out of PLAN.md.

**The shape of the decisions is good.** D3, D4, D6, D7, D8, D9, D10 and D13 each
match what the code does, verified by reading the code rather than the prose:
the `display_name`/`name_from_digest` split is real and both are `pub`; the byte
budget is `0..12` with `draw_at` at offsets 0 and 6 and a `ReserveExhausted`
beyond; `DisplayName` is three `&'static str` with the connector existing only in
`render()`; `display_name_from_bytes` parses then derives and returns no
placeholder; the mark reads `_byte(4)`…`_byte(11)` in `Identicon.qml`, disjoint
from the abbreviation's `{0,1,2,3}`, `{14..17}`, `{29,30,31}`; the wire asserts
the feed row's whole key **set**, so an added field fails. The withdrawal of the
semantic exclusions is complete and clean — every surviving mention of the
single-word rule, familiarity, legibility or tone is a historical marker or an
*anti*-regression assertion (`names.rs:1250` pins `platon`, `sokrates`, `stoa`
and `archon` as **present**), and `docs/UI-BRIEF.md` matches both the code and
the spec. The spec cites no PLAN section number.

What follows is where the prose and the code part company.

## The code contradicts a recorded decision

- [ ] **`dev-writer`** — `design.md:66` (D2) — the recorded separator is not the
      one in the code, and would not be 32 bytes.
      D2 states the separator is `b"/dialectica/1/Name/Display\0\0\0\0\0\0\0"` —
      **seven** NULs. `names.rs:97` is
      `b"/dialectica/1/Name/Display\0\0\0\0\0\0"` — **six**.
      **Measured:** `/dialectica/1/Name/Display` is 26 characters; 26 + 6 = 32,
      which is the `&[u8; 32]` the code declares and the fixed width D2's own
      argument rests on. The recorded literal is 33 bytes and would not compile
      as written. This is the consensus-critical constant of the whole scheme,
      recorded wrongly in the document a future reader will reach for when asking
      what version 1 was.

- [ ] **`dev-writer`** — `design.md:70` (D2) — the cited test does not exist.
      D2: *"`identity.rs`'s pin test asserts it does not collide with the address
      prefixes, which is the coupling that matters."* It does not.
      **Verified:** `grep -rn "Name/Display\|dialectica/1/Name"` across
      `dialectica/`, `docs/` and `openspec/` returns exactly two hits — the
      definition at `names.rs:97` and the (wrong) quotation in `design.md`.
      `the_wire_constants_are_pinned_to_known_answers` in `identity.rs:823`
      pins the author address, the Stoa address, the signing digest and two
      HKDF path values; `NAME_PREFIX` is never mentioned in it or anywhere else
      in that file. D2 uses this non-existent assertion as the *justification*
      for defining the constant in `names.rs` rather than `identity.rs` — so the
      stated reason for the placement is a check nobody wrote. Either write the
      assertion or record the placement's real reason.

## Decisions taken in the code but not recorded

- [ ] **`dev-writer`** — `feed.rs:293` — **the most important finding.** A post
      whose author's name cannot be derived is **silently dropped from the
      feed**, and this is nowhere in `design.md`.
      `let Ok(display_name) = crate::names::display_name(&entry.op.op.author)
      else { continue; };` — the `continue` removes the row. The code's own
      comment marks it `NO SPEC` and argues it, but `design.md` has no Decisions
      entry for it: D7 and D8 both say only that no placeholder is returned, and
      D8 describes the field as simply added beside the address.
      **Why it matters beyond being unrecorded:** this is a *name-derivation*
      failure censoring *content* in a censorship-resistant forum. An author
      whose key exhausts the denylist reserve becomes unable to be read at all,
      by every peer, deterministically and permanently — the derivation is a pure
      function of the key, so it is not a transient drop. Three alternatives
      existed and none is recorded as considered: serve the row with the address
      and omit the name field (which the spec's "a reply that reports no author
      carries no name field" shape already permits), fail the page, or widen
      `DisplayName` to carry an unnamed variant. Silently vanishing a
      participant's posts is the one option that is invisible to the reader, and
      it was chosen without the alternatives being written down.

- [ ] **`tester`/`dev-writer`** — `feed.rs:292` — the test that comment cites
      still does not exist, and `tasks.md:19` records the gap as
      reviewer-visible rather than fixed.
      **Verified:** `grep -rn
      "a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked"` across
      `dialectica/` returns one hit — the comment at `feed.rs:292` itself. So
      the one behaviour above that is *only* justified in a comment is pinned by
      nothing. Per CLAUDE.md the change that introduces a behaviour is the change
      that pins it down; this behaviour was introduced here and is unpinned.

- [ ] **`dev-writer`** — `design.md` — the place list's **dedup policy** is a
      decision with a real alternative and is recorded only in a gitignored
      scratch file.
      `tmp/places-census/collapse-map.txt` states the rule applied: *"KEEP the
      qualified form as the single entry for that toponym family"*, collapsing
      `thebe hypoplakia` into `thebai`, `larisa phrikonis` into `larisa`,
      `elea velia` into `elea`, and so on. That is the reading "one entry per
      **place**"; the alternative reading — one entry per **name**, keeping both
      spellings as distinct draws — was available and would have yielded a larger
      list. D11 gestures at "near-duplicate transliterations of one place" among
      the cuts but never states the rule or why this reading was chosen. `tmp/`
      is gitignored (`.gitignore:64`), so after the merge the only record of this
      decision disappears.

## Numbers in the record that are wrong

- [ ] **`dev-writer`** — `design.md:118` (D5) and `names.rs:359` — the denylist's
      size is overstated by a factor of five, and the two rates derived from it
      are wrong by factors of five and twenty-three.
      **Measured:** `grep -c "^    ("` on `names/denylist.rs` gives **199**
      pairs. `tasks.md:7` records 199 correctly; D5 and the code comments do not.
      - D5: *"a 10-comparison binary search over ~1,000 entries"*. 199 entries is
        an 8-comparison search. The argument survives, the figure does not.
      - `names.rs:359`: *"roughly 800 of the 1,024 nouns are named Greeks, each
        with about 1.2 canonically associated places, so about 960 pairs …
        about 0.092% of draws — roughly 4.6 identities in every 5,000."*
        **Measured:** 199 / 1,048,576 = 0.019%, not 0.092%; that is about 0.95
        identities per 5,000, not 4.6.
      - `names.rs:196` and `feed.rs:282`: *"about once in 1.2 million
        identities: a first draw is refused about once in 1,090"*. **Measured:**
        1 / 0.00019 = once in **5,270**, and the square gives once in
        **27.8 million**, not 1.2 million.
      This matters because PLAN.md deliberately *withdrew* the 800 × 1.2 = 960
      premise ("**The premise is withdrawn.** That 800 followed from a noun slot
      holding only abstractions and thinkers") — the code comments imported the
      retracted arithmetic and now state it as current. The 1-in-1.2-million
      figure is also the stated reason `feed.rs` treats the drop as negligible.

- [ ] **`dev-writer`** — `design.md:221` (D11) — the adjective "source yield" of
      17,349 is not reproducible and contradicts D12's own step 1.
      D11's table claims a source yield of 17,349 with 9,157 discarded, and says
      *"The counts are from `grep -c .` on the files, not from an estimate."*
      D12 step 1 says **11,467** matched the suffix regex — and every later step
      chains from that number, not from 17,349.
      **Measured**, by `grep -c .` on the surviving files in
      `.claude/worktrees/piece-generated-names/tmp/`: `adj-11467.txt` = 11,467;
      `adj-clean.txt` = 10,345 (D12 step 2 ✓); `adj-8454.txt` = 8,454 (step 3 ✓);
      `adj-8339.txt` = 8,339 (step 4 ✓); `adj-final-sorted.txt` = 8,192, so
      8,339 − 8,192 = 147 (step 5 ✓). **No file yields 17,349**; the nearest is
      `adj-strong.txt` at 18,837 and `adj-raw.txt` at 89,003. D12's chain is
      sound and verifiable end to end; D11's headline figure for the same list is
      not, and the two contradict each other in one document.

- [ ] **`dev-writer`** — `design.md:227` (D11) — "its 107 cuts are each a defect
      the spec names" is not supported by the cut list.
      **Measured:** `grep -c .` on `tmp/places-census/dedup.txt` = **1,131** ✓
      and on `places-final-sorted.txt` = **1,024** ✓, so 107 were cut. But
      `tmp/places-census/cut-list.txt` holds **61** entries. The enumerated,
      defect-justified cuts account for 61 of 107; the remaining 46 are
      unexplained. D11 presents the margin as "the evidence the rule was
      followed", so the gap is exactly where the evidence is load-bearing.
      (The noun yield 1,892 and place yield 1,131 both verify against
      `nouns-census/COMBINED-DEDUP.txt` and `places-census/dedup.txt`.)

## Provenance: the argument for D1 does not survive the merge

- [ ] **`dev-writer`** — `design.md:54` (D1) — the auditability the whole
      generated-not-typed decision rests on is destroyed by `.gitignore`.
      D1's load-bearing claim: *"a reviewer can diff a generated array against
      the text file it came from, and the text file against the census it came
      from, where a hand-transcribed array can only be read and believed."*
      **Verified:** `.gitignore:64` is `tmp/`. The text files, the census
      directories, `tmp/gen.rs`, `tmp/pin.rs` and `tmp/attributions.txt` are all
      untracked, and none exists in this review worktree — I could only read them
      by reaching into `.claude/worktrees/piece-generated-names/`, which vanishes
      when that worktree is pruned. `names.rs:580` tells a reviewer to re-run
      `sha256sum tmp/adj-final-sorted.txt`; after merge there is no such file, so
      the three `PINNED_*_SHA256` constants become unreproducible and the pin
      degrades from "checkable against a source" to exactly the "read and
      believed" state D1 was written to avoid. Either commit the source files (or
      the generator plus its inputs) somewhere tracked, or record in D1 that the
      provenance chain is a build-time artefact that does not survive, and say
      what a future reviewer is meant to do instead.

## PLAN.md still carries what this change settled

`docs/PLAN.md` was substantially rewritten here and the migration is mostly
well done — the exclusions section, the noun-pool table, the 800 × 1.2 premise
and the combination-screen paragraph were all correctly withdrawn or struck
through. Four places were missed, and all four read as live.

- [ ] **`dev-writer`** — `docs/PLAN.md:1088`, `:1091`, `:1199`, `:1982` — PLAN.md
      says the mark reads bytes `12..19` in four places. This change moved it to
      `4..11`.
      **Verified:** `Identicon.qml:123` `_form()` = `_byte(4)` through `:193`
      `_weave()` = `_byte(11)`; `IDENTICON.md:32`, `:47`, `:652` all say `4..11`
      and mark `12..19` as historical. PLAN.md was not updated to match and none
      of the four is struck through. Line 1982 is the worst: *"It reads bytes
      `12..19` of the **address**"* sits in a bullet that presents itself as the
      settled pointer to `IDENTICON.md`, so a reader is sent to the correct
      document by a sentence that contradicts it.

- [ ] **`dev-writer`** — `docs/PLAN.md:1197`–`1210` — the section states the
      disjointness requirement **does not hold** and leaves the allocation to
      this change. Both are now false.
      *"**Today the requirement does not hold, and the gap is measured rather
      than suspected.** … only `{12, 13, 18, 19}` of the 21 bytes the
      abbreviation hides reach a reader through the mark"* and *"**The allocation
      itself is not decided here** … the byte ranges are the `generated-names`
      change's to settle"*. This change settled them and closed the gap — D9
      records it and `tst_identicon.qml` moved with it. Left as written, the next
      agent reads an open requirement that was closed on this branch.

- [ ] **`dev-writer`** — `docs/PLAN.md:1957`–`1961` — under the heading **"What
      is not decided here"**: *"What is not written is the **10,240 words**
      themselves, nor the true-attribution denylist, both of which are curation
      work rather than design work."* This change wrote all four lists.
      **Measured:** 8,192 + 1,024 + 1,024 = 10,240 entries plus 199 denylist
      pairs, all in `dialectica/rust-lib/dialectica-core/src/names/`. PLAN.md is
      meant to be left carrying what is *not built yet*; this is the clearest
      case of a built thing still listed as outstanding. (The adjacent
      `> The lists are now written` block at `:1486` shows the right treatment —
      this bullet needs the same.)

## Thin entries

- [ ] **`dev-writer`** — `design.md:41` (D1) — the entry records the alternative
      (`include_str!` with parse-at-startup) and what ruled it out, but not what
      the choice **costs**. The cost stated is compile time, "which measured as
      noise" — with no measurement given, and the real cost is elsewhere: a
      144 KB generated `adjectives.rs` that no reviewer will read, whose only
      defence is the provenance chain the finding above shows does not survive.
      Name that cost.

- [ ] **`dev-writer`** — `design.md:303` — the superseded section is correctly
      marked (*"That argument is **gone rather than struck through**"*) and reads
      unambiguously as history; it is not a live-reading hazard. But it is the
      only place the 1,892 / 1,131 figures are explained, and D11's table repeats
      them as current. Given the two findings above about those figures, say in
      one line which of the two sections is the authority for a count.

## What a gate cannot see here, and what I could not run

- **I could not execute the suite.** `cargo test` against both
  `dialectica/rust-lib/Cargo.toml` and `.../dialectica-core/Cargo.toml` fails in
  this worktree with `failed to read
  .../dialectica/logos-rust-sdk-src/Cargo.toml` — the SDK is a flake input and
  is not vendored here. Every "Measured" figure above comes from `grep -c` over
  the shipped files and from reading the code, not from a green or red run.
- **No gate can see any finding in this file.** The denylist-size arithmetic, the
  wrong `NAME_PREFIX` literal, the missing `identity.rs` assertion, the
  17,349/11,467 contradiction, the 61-of-107 cut list, PLAN.md's four stale
  `12..19`s and the gitignored provenance chain are all statements in prose or in
  comments. The suite asserts the lists' sizes, their hashes and the denylist's
  sortedness — it cannot assert that a sentence describing them is true.
- **The feed drop is the one behavioural finding**, and it is invisible to the
  suite for a specific reason: reaching it needs a key whose digest is refused
  twice, which is roughly one in 27.8 million by the corrected arithmetic, so no
  seed sweep will ever hit it. It is reachable only through a constructed digest,
  which is precisely the testability seam D3 exists to provide — and which
  `feed.rs` does not use, because `list_threads` takes a key and not a digest.
