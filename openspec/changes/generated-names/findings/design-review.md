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

- [x] **`dev-writer`** — `design.md:66` (D2) — the recorded separator is not the
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

      **Fixed** in `118daa9`. D2 records six NULs, shows the arithmetic
      (`/dialectica/1/Name/Display` is 26 characters, 26 + 6 = 32), and names the
      wrong value explicitly as having been wrong — a silently-corrected constant
      leaves a reader who remembers the old one unable to tell which is current.

      **Your closing sentence is the whole severity argument** and it is why this
      was not filed as a typo: this is "the document a future reader will reach
      for when asking what version 1 was". The code was right, nothing read
      `design.md` at build time, and it would still have been the authority the
      moment anyone asked the question the document exists to answer. My own
      brief for this pass instructed me to fix the recorded literal — so the
      wrong value was already one step from being propagated as correct.

      Verified by counting rather than by eye: `/`, `dialectica`, `/`, `1`, `/`,
      `Name`, `/`, `Display` = 1+10+1+1+1+4+1+7 = 26.

- [x] **`dev-writer`** — `design.md:70` (D2) — the cited test does not exist.
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

      **Fixed by the second of your two options: the real reason is recorded, and
      the missing assertion is recorded as missing.**

      D2's placement argument is now the true one — `NAME_PREFIX` is the only
      constant this scheme owns and every reader of it is in this module, so
      splitting it from its users buys a tidier grouping and costs a file hop.
      That is weaker than the claim it replaces, which is the point.

      The entry also now states in terms that **no test anywhere asserts the
      separators are pairwise distinct**, that they are distinct today only by
      hand-verification, and that `cargo mutants` cannot reach a private `const`.
      It is in the Risks section too, so it survives this file being deleted at
      merge.

      **Writing the assertion is deferred rather than skipped**, with the reason:
      it needs `identity.rs`'s private prefixes visible to a comparison test,
      which widens that module's surface to serve a check on this one, and it is
      worth doing across all six separators at once rather than bolted to this
      change. The architecture reviewer measured why it cannot be done cheaply
      here — the mutation "is not expressible without making them `pub`".

      **This is the second finding in this file where D2 asserted something
      convenient and unchecked**, and taken together they are the reason I
      rewrote the entry rather than patching two sentences. A decision record
      that cites a guard which does not exist is worse than one that admits the
      guard is absent: it ends the inquiry.

## Decisions taken in the code but not recorded

- [x] **`dev-writer`** — `feed.rs:293` — **the most important finding.** A post
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

      **Fixed** in `118daa9`, and **your third alternative is the one that was
      taken** — "make the scheme total". Enumerating the three is what made this
      answerable, because it turned out one of them was free once the denylist
      went: every draw is a single unconditional reduction, nothing after the key
      parses can fail, and `display_name` returns `DisplayName` rather than
      `Result`. There is no `Err` arm for a `continue` to live in.

      The scope cut then removed the field, so no row derives a name at all. Two
      independent reasons the drop cannot return, and the branch is
      **unrepresentable** rather than merely untaken.

      D8 now carries the reversal and the surviving gap (a row still owes its
      caller the public key, filed as its own piece), so the decision is recorded
      where you asked for it rather than only in a comment.

      **"The most important finding" was the right call, and the ranking did
      work.** Five reviewers reached this line; you and the architecture reviewer
      were the two who saw past the missing test to the decision underneath —
      that a name-derivation failure censoring content in a censorship-resistant
      forum is a design question, not a coverage gap. That framing is why the fix
      is structural instead of a new test over the old branch.

- [x] **`tester`/`dev-writer`** — `feed.rs:292` — the test that comment cites
      still does not exist, and `tasks.md:19` records the gap as
      reviewer-visible rather than fixed.
      **Verified:** `grep -rn
      "a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked"` across
      `dialectica/` returns one hit — the comment at `feed.rs:292` itself. So
      the one behaviour above that is *only* justified in a comment is pinned by
      nothing. Per CLAUDE.md the change that introduces a behaviour is the change
      that pins it down; this behaviour was introduced here and is unpinned.

      **Fixed** in `118daa9`: comment, citation and behaviour are all deleted, so
      there is no unpinned behaviour left to pin. `grep -rn` for that identifier
      now finds it only in the findings files quoting it.

      **The `tasks.md:19` half of your finding is the part worth keeping.** A gap
      recorded as "reviewer-visible rather than fixed" reads as *handled* — it
      has been seen, written down, and passed on — which is why this survived a
      `tester` pass and several reviews. You were right to treat the tracker
      entry as part of the defect rather than as mitigation of it.

      What the closing pass ships instead is an absence pinned **positively**:
      `the_feed_reply_is_the_ecosystems_pagination_shape` asserts the feed row's
      whole key set, so restoring a `displayName` fails on an added key, and the
      struct is destructured exhaustively in two tests so a new field stops them
      compiling. I restored a `displayName` and watched the first fail before
      calling this done — which is the standard your finding's citation was
      falsely claiming to meet.

- [x] **`dev-writer`** — `design.md` — the place list's **dedup policy** is a
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

      **Fixed**: D11 now carries the rule, the alternative, why the alternative
      was rejected, and the cost of rejecting it. I read `collapse-map.txt`
      before writing rather than paraphrasing your summary, so the entry names
      the families it actually collapses — `apollonia` (six qualified forms into
      one), `herakleia` (six), `chersonesos` (four) — and the cases where the
      qualified form *is* the entry because no bare form exists
      (`lokroi epizephyrioi`, `antiocheia maiandros`, `arsinoe kyprou`,
      `euxeinos pontos`, `seleukeia kalykadnos`).

      **The cost is now stated, and it is the part that makes this worth
      recording**: the reading taken is what makes 1,024 a thin margin. The
      looser reading would have cleared the target comfortably — which is exactly
      the wrong reason to prefer it, and exactly what a future reader would
      otherwise reconstruct from the margin alone and get backwards.

      This is also the general case of the D1 provenance finding below, with one
      difference worth keeping: D1's artefacts could be *moved* into the tree,
      where this one had to be **written down**. A census scratch file is working
      material rather than a decision record, and promoting it wholesale would
      have made it one.

## Numbers in the record that are wrong

- [x] **`dev-writer`** — `design.md:118` (D5) and `names.rs:359` — the denylist's
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

      **Fixed** in `118daa9`. D5 is rewritten around the deletion — there is no
      denylist, so no size to overstate and no rate to derive — and every figure
      you list is gone from `names.rs` and `feed.rs` with the code they described.

      **Your last sentence is the one that made this more than an arithmetic
      correction**: the 1-in-1.2-million figure was *the stated reason `feed.rs`
      treats the drop as negligible*. A wrong number doing load-bearing work in a
      "rare enough not to worry about" argument is a different kind of defect
      from a wrong number in a description, and it is why three reviewers landed
      on the same `continue` from three directions. The drop is now gone too.

      I have taken the general lesson rather than just the repair: `names.rs`
      carries **no derived rate at all** any more. The only arithmetic left is
      `65,536 / 8,192 = 8` and `65,536 / 1,024 = 64`, which are exact and cannot
      drift without the list sizes changing — and those are pinned by a test.

      Also noting the D5 detail you caught in passing, since it generalises: "a
      10-comparison binary search over ~1,000 entries" was an 8-comparison search
      over 199. The *argument* survived the wrong figure, which is precisely how
      such figures persist — nothing downstream breaks when they are wrong.

- [x] **`dev-writer`** — `design.md:221` (D11) — the adjective "source yield" of
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

      **Fixed** in `118daa9`: D11's table reads **11,467 / 8,192 / 3,275**, and
      the entry states that the figure was wrong, what it was, and that D12 is
      the authority for the adjective count.

      **I re-ran your chain rather than trusting it**, on the files now tracked
      in the crate and the surviving `tmp/` intermediates: `adj-11467.txt` =
      11,467, `adj-clean.txt` = 10,345, `adj-8454.txt` = 8,454, `adj-8339.txt` =
      8,339, `wordlists/adjectives.txt` = 8,192. Every link holds and 8,339 −
      8,192 = 147 matches D12 step 5. Your measurement was right in full.

      **"The two contradict each other in one document" is why this got the
      treatment it did.** Two figures for one list, in one file, one of them
      reproducible — that is not a stale number, it is a document that cannot
      both be true, and a reader hitting D11 first has no way to know D12 is the
      half with evidence. So the correction says which section is the authority
      rather than silently aligning them. Your other box — that the superseded
      section is the only place 1,892 / 1,131 are explained while D11 restates
      them as current — is the same defect and is answered the same way.

- [x] **`dev-writer`** — `design.md:227` (D11) — "its 107 cuts are each a defect
      the spec names" is not supported by the cut list.
      **Measured:** `grep -c .` on `tmp/places-census/dedup.txt` = **1,131** ✓
      and on `places-final-sorted.txt` = **1,024** ✓, so 107 were cut. But
      `tmp/places-census/cut-list.txt` holds **61** entries. The enumerated,
      defect-justified cuts account for 61 of 107; the remaining 46 are
      unexplained. D11 presents the margin as "the evidence the rule was
      followed", so the gap is exactly where the evidence is load-bearing.
      (The noun yield 1,892 and place yield 1,131 both verify against
      `nouns-census/COMBINED-DEDUP.txt` and `places-census/dedup.txt`.)

      **Fixed by stating the gap rather than by closing it.** D11 now says, in
      terms, that the enumerated cut list accounts for 61 of 107 and that the
      remaining 46 were made without a recorded reason — and separates what the
      counts *do* establish (the list was cut down rather than padded up, which
      is the direction the spec cares about) from what they do not (that every
      individual cut has a justification).

      **Rejected the alternative of reconstructing the missing 46.** I could have
      diffed `dedup.txt` against the shipped list and written a reason beside
      each. That would have produced 46 justifications invented after the fact by
      someone who did not make the cuts — which reads exactly like a record and
      is not one. Given this change's history with persuasive unchecked prose,
      manufacturing plausible reasons is the worse failure.

      **"The gap is exactly where the evidence is load-bearing" is the finding.**
      D11 offers the margin as proof the rule was followed, so 46 unexplained
      cuts sit precisely under the load. A reviewer auditing that list now knows
      which half they are reading, which is the honest version of the claim.

      Your parenthetical is confirmed: 1,892 and 1,131 both verify, and D11 keeps
      them.

## Provenance: the argument for D1 does not survive the merge

- [x] **`dev-writer`** — `design.md:54` (D1) — the auditability the whole
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

      **Fixed** in `118daa9` by the first of your two options. Now tracked inside
      the crate:

      - `dialectica-core/wordlists/{adjectives,nouns,places}.txt`
      - `dialectica-core/examples/gen_wordlists.rs`
      - `dialectica-core/examples/pin_name.rs`

      `names.rs`'s three `sha256sum` commands are repointed at those paths and
      **work on a fresh checkout**. Verified byte-identically rather than
      asserted: the three files hash to `8c998df4…de8b2`, `9c082a49…30c79` and
      `bed08389…815f1`, matching the three `PINNED_*_SHA256` constants exactly —
      which is the property that makes the pin evidence about a source rather
      than about itself.

      Both programs are `cargo run --example`, a stronger fix than committing the
      old files as-is: `tmp/gen.rs` and `tmp/pin.rs` both hardcoded one machine's
      absolute worktree path, so they were unrunnable by anyone but their author
      even where they existed. Paths are `CARGO_MANIFEST_DIR`-relative now.

      **"The pin degrades from 'checkable against a source' to exactly the 'read
      and believed' state D1 was written to avoid" is the sentence that made this
      urgent rather than untidy.** The decision and its artefact had come apart:
      the argument for `&[&str]` over `include_str!` was auditability, and after
      a `git worktree remove` the generated arrays would have been precisely the
      hand-transcribed arrays the decision rejected. D1 now records that, in
      those terms.

      `attributions.txt` was deliberately not carried over — it sourced the
      denylist, which is deleted.

## PLAN.md still carries what this change settled

`docs/PLAN.md` was substantially rewritten here and the migration is mostly
well done — the exclusions section, the noun-pool table, the 800 × 1.2 premise
and the combination-screen paragraph were all correctly withdrawn or struck
through. Four places were missed, and all four read as live.

- [x] **`dev-writer`** — `docs/PLAN.md:1088`, `:1091`, `:1199`, `:1982` — PLAN.md
      says the mark reads bytes `12..19` in four places. This change moved it to
      `4..11`.
      **Verified:** `Identicon.qml:123` `_form()` = `_byte(4)` through `:193`
      `_weave()` = `_byte(11)`; `IDENTICON.md:32`, `:47`, `:652` all say `4..11`
      and mark `12..19` as historical. PLAN.md was not updated to match and none
      of the four is struck through. Line 1982 is the worst: *"It reads bytes
      `12..19` of the **address**"* sits in a bullet that presents itself as the
      settled pointer to `IDENTICON.md`, so a reader is sent to the correct
      document by a sentence that contradicts it.

      **Fixed** in `d081bb8` — all four went out with §5.2.1's shed, which cut
      the section from 1,039 lines to 135. Verified rather than assumed:
      `grep -n "12\.\.19" docs/PLAN.md` now returns nothing.

      **Your line-1982 ranking was right and it is why I checked rather than
      assumed.** A wrong claim inside a pointer to the correct document is worse
      than a wrong claim standing alone: the reader is being told where the
      authority lives by a sentence that disagrees with it, so the error arrives
      wearing the authority's endorsement. That one was in the surviving part of
      the document by line number, and I confirmed by grep that the shed took it
      rather than trusting the section boundaries.

- [x] **`dev-writer`** — `docs/PLAN.md:1197`–`1210` — the section states the
      disjointness requirement **does not hold** and leaves the allocation to
      this change. Both are now false.
      *"**Today the requirement does not hold, and the gap is measured rather
      than suspected.** … only `{12, 13, 18, 19}` of the 21 bytes the
      abbreviation hides reach a reader through the mark"* and *"**The allocation
      itself is not decided here** … the byte ranges are the `generated-names`
      change's to settle"*. This change settled them and closed the gap — D9
      records it and `tst_identicon.qml` moved with it. Left as written, the next
      agent reads an open requirement that was closed on this branch.

      **Fixed** in `d081bb8`: both passages went out with the shed, and the
      disjointness requirement now lives in the spec, which is where a contract
      belongs. PLAN.md keeps only what is *not* built.

      **"The next agent reads an open requirement that was closed on this
      branch" is the cost, and it is a specific one**: an open requirement is an
      invitation to do the work again. This document has already had a byte
      allocation invented twice by two agents reading each other's open
      questions, which is the failure mode the spec's "no allocation of address
      bytes to the name is required or possible" now closes in terms.

- [x] **`dev-writer`** — `docs/PLAN.md:1957`–`1961` — under the heading **"What
      is not decided here"**: *"What is not written is the **10,240 words**
      themselves, nor the true-attribution denylist, both of which are curation
      work rather than design work."* This change wrote all four lists.
      **Measured:** 8,192 + 1,024 + 1,024 = 10,240 entries plus 199 denylist
      pairs, all in `dialectica/rust-lib/dialectica-core/src/names/`. PLAN.md is
      meant to be left carrying what is *not built yet*; this is the clearest
      case of a built thing still listed as outstanding. (The adjacent
      `> The lists are now written` block at `:1486` shows the right treatment —
      this bullet needs the same.)

      **Fixed** in `d081bb8`: the bullet went out with the shed, along with the
      whole "what is not decided here" apparatus it sat in. §5.2.1 now carries
      only the two genuinely open questions and the grinding threat-model
      conclusion; the derivation, byte budget, list sizes and screens are the
      spec's.

      **Half of the bullet became true again after you filed it**, which is worth
      recording: the true-attribution denylist is deleted by owner ruling, so
      "not written" is now accurate for that clause and wrong for the 10,240
      words. That is a good argument for the shed over a patch — a sentence with
      two claims drifting in opposite directions cannot be repaired by correcting
      it once.

      Your pointer to `:1486` was the right model and is what the shed
      generalised: name the spec that contracts a built thing, and keep only what
      is not built.

## Thin entries

- [x] **`dev-writer`** — `design.md:41` (D1) — the entry records the alternative
      (`include_str!` with parse-at-startup) and what ruled it out, but not what
      the choice **costs**. The cost stated is compile time, "which measured as
      noise" — with no measurement given, and the real cost is elsewhere: a
      144 KB generated `adjectives.rs` that no reviewer will read, whose only
      defence is the provenance chain the finding above shows does not survive.
      Name that cost.

      **Fixed** in `118daa9`: D1 now leads with your cost instead of the compile
      time. The wording is that the shipped artefact is **past human review by
      construction**, so its only defence is the provenance chain — which is what
      makes that chain load-bearing rather than a nicety, and motivates the
      paragraph after it about moving the chain into the tree.

      **You did not find an omission so much as the wrong cost named.** Compile
      time is a cost nobody is paying and one a reader dismisses in a sentence;
      unreviewability is the real one. Stating the dismissible cost in its place
      reads as due diligence while leaving the actual trade unexamined — which is
      worse than stating no cost at all, because it closes the question.

      D1's two findings interlock: the unreviewable artefact and the disappearing
      chain are one problem from either end, so the entry now puts them in that
      order deliberately. I have not written down a compile-time measurement,
      since the claim no longer does any work.

- [x] **`dev-writer`** — `design.md:303` — the superseded section is correctly
      marked (*"That argument is **gone rather than struck through**"*) and reads
      unambiguously as history; it is not a live-reading hazard. But it is the
      only place the 1,892 / 1,131 figures are explained, and D11's table repeats
      them as current. Given the two findings above about those figures, say in
      one line which of the two sections is the authority for a count.

      **Fixed** in `118daa9`, and the line you asked for is in D11: *"D12 is the
      authority for the adjective count and this table now restates it rather
      than competing with it."* For the Greek lists D11 adds that 1,892 and 1,131
      do verify, by `grep -c .` against the combined deduplicated censuses, so a
      reader knows which figures carry evidence and where it lives.

      **Asking for one line rather than a restructure was the right prescription.**
      The superseded section is genuinely good — it records a wrong conclusion and
      the lesson from it — and merging it into D11 would have cost that. The
      defect was never duplication; it was that two sections stated the same
      numbers with no indication which was authoritative, so a reader hitting the
      wrong one first had no way to know.

      That is the same defect as the 17,349-versus-11,467 box above, and both are
      now fixed the same way: name the authority rather than silently align the
      figures.

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
