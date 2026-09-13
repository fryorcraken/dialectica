# Architecture review — generated-names

Dimension covered: **architecture only**. Correctness, security and readability
are each held by another reviewer instance.

Measured on `review/generated-names/architecture`, cut from `piece/generated-names`
at `24faad7`. Baseline re-run in this worktree: `918 + 28 = 946`, 0 failed.

## Findings

- [x] **`dev-writer`** — `wire.rs:1724` / `thread.rs:148` — the second
      author-reporting surface was never considered, and the two now enforce
      *opposite* architectures
      **Scenario:** `feed_page_json` computes the name in core and ships
      `displayName`; `thread_page_json` ships `authorKey` — the raw public key —
      and ships no name. `thread.rs:144` states the policy as a rule: *"**No name
      is sent from here.** A name is a pure function of this field, so sending
      one would put a second, derivable identifier on the wire beside the
      material it is derived from — where the two could disagree."* `feed.rs` now
      does exactly what that sentence forbids, and `names.rs:20` states the
      contrary rule as *"the reason `FeedRow` carries a name at all."* Neither
      document cites the other. A view rendering a feed reads `displayName`; the
      same view rendering the thread it opens must derive the name itself from
      `authorKey` — so one screen's name is core's and the next screen's is the
      view's, for the same identity. Two derivations that can drift is the exact
      failure `names.rs:31` calls "one identity rendering as two people, which
      users report as impersonation".
      **Measured:** `grep -n "thread_page\|ThreadItem\|authorKey\|author_key\|list_thread"`
      over `design.md`, `proposal.md` and `tasks.md` returns three hits, all
      three about `list_threads` (the *feed*); `ThreadItem`, `thread_page_json`
      and `authorKey` appear in none of the three change documents. The
      alternative — one seam, `authorKey` everywhere, view derives — was never
      weighed, and the seam actually chosen (`displayName` on one row shape) was
      chosen without noticing the other row shape exists.
      **Severity: high.** This is the API-widening decision CLAUDE.md says must
      be made on purpose. It was made for one of two surfaces by accident of
      which file the change started in. See the next finding: the two surfaces'
      policies are not merely undocumented, they are written down as two
      contradictory live requirements.

      **Fixed** in `118daa9`, and **`thread.rs` is the surface that was right.**
      The feed row's `displayName` is deleted; no reply carries a name, on any
      surface. The two architectures no longer disagree because one of them is
      gone, and the one that survived is the one you identified as coherent.

      **This finding changed the shape of the change**, and it is worth being
      precise about how. You wrote that the seam actually chosen "was chosen
      without noticing the other row shape exists" — which was true, and the
      owner's scope cut then went further than converging the two: it ruled that
      the name never travels at all. `thread.rs:144`'s sentence, the one you
      quoted as the policy the feed now violated, is the contract for every
      reply.

      **Your diagnosis of the mechanism is the part I want recorded**, because
      it is the reusable half: an API widening made "by accident of which file
      the change started in". The derivation reached one author-reporting surface
      because that was the file the work began in, and the second surface was
      never weighed. The `design.md` D8 entry now records both the wrong turn and
      why the premise behind it (a feed row carries an address, from which no key
      is recoverable) was correct while the conclusion was not — the fix is to
      stop dropping the key, not to start sending names.

      That fix is **not** in this change: putting `authorKey` on a feed row
      changes the `author` field's contract and touches several merged specs, so
      it is filed as its own piece. Recorded as a live gap in D8 and in the
      Risks section rather than left implicit.

- [x] **`spec-writer`** — `openspec/specs/thread-read/spec.md:192` vs
      `openspec/changes/generated-names/specs/generated-names/spec.md:597` — two
      **live** requirements now contradict each other outright
      **Scenario:** the merged `thread-read` capability requires
      *"#### Scenario: No item carries a derived display name — WHEN a thread is
      read and every field of an item is enumerated, THEN the only fields
      describing the author are the address and the public key, AND the item
      carries no third author-describing field, which is what a name or a mark
      would have to be."* The new `generated-names` capability requires
      *"Every reply in which core reports who authored something SHALL carry that
      author's display name alongside the author's address."* A thread item is a
      reply in which core reports who authored something. The two requirements
      cannot both be satisfied: satisfying `generated-names` at
      `thread_page_json` violates `thread-read`, and the present code satisfies
      `thread-read` by violating `generated-names`. This is a cross-capability
      conflict, so it is `spec-writer` work — one of the two has to be amended in
      this change, and the amendment is the decision the previous finding says
      was never made.
      **Measured:** `wire.rs:5946`,
      `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others`, asserts
      the nine-key item set and cites the `thread-read` scenario by name in its
      own comment — so the contradiction is not latent, it is actively enforced
      by a passing test. 946 of 946 pass. `openspec validate --strict` does not
      see this: CLAUDE.md's `docs/OPENSPEC-ARCHIVE.md` note records that strict
      validation passes a spec that contradicts itself, and this contradiction
      spans two capability files, which is weaker still.
      **Severity: high** — a merged change would ship two requirements that
      cannot both hold, with the losing one enforced by a green test.

      **FIXED — `generated-names` amended, `thread-read` left alone.** You framed
      this as "one of the two has to be amended in this change", and the one that
      moved is this change's. The requirement now keys on what the reply carries:
      a reply with only an address owes the name, a reply carrying the public key
      is forbidden from carrying it. `thread-read`'s requirement and its scenario
      at :192 are both untouched and both now describe a reply that *complies*.
      Your previous finding — that the two surfaces enforce opposite
      architectures — is the `dev-writer`'s, and this resolution settles which
      architecture it should converge on: `thread_page_json` is already right,
      and what needs a recorded decision is `feed.rs` shipping a name because the
      feed row carries no key, not the thread shipping none.

- [x] **`dev-writer`** — `names/adjectives.rs`, `names/nouns.rs`,
      `names/places.rs`, `names/denylist.rs` — the provenance chain D1 rests on
      is entirely outside the repository, and disappears when the author's
      worktree is pruned
      **Scenario:** D1's argument for generated `&[&str]` arrays over a data file
      is auditability — *"a reviewer can diff a generated array against the text
      file it came from, and the text file against the census it came from,
      where a hand-transcribed array can only be read and believed."* Every one
      of those artefacts lives in `tmp/`, which `.gitignore:64` excludes.
      `git ls-files tmp/` returns nothing. In this review worktree,
      `/tmp` does not exist at all; the 34 source files (`gen.rs`, `pin.rs`,
      `attributions.txt`, `adj-final-sorted.txt`, `nouns-normalized.txt`,
      `places-final-sorted.txt`, the two census directories …) exist only under
      `.claude/worktrees/piece-generated-names/tmp/`. CLAUDE.md requires that
      worktree be removed once the branch merges. After that, the generated
      arrays are exactly the hand-transcribed arrays D1 rejected — auditable by
      nobody, re-derivable by no one — and the lists are consensus-critical and
      unfixable after release without a scheme version bump.
      **Measured:** `names.rs:583-585` instructs a reviewer to run
      `sha256sum tmp/adj-final-sorted.txt`, `sha256sum tmp/nouns-normalized.txt`
      and `sha256sum tmp/places-final-sorted.txt` to reproduce the three pinned
      hashes. All three commands fail with "No such file or directory" on any
      fresh checkout of this branch. The hash pins detect array drift; the route
      the code documents for checking what the arrays should have been is
      unreachable.
      **Severity: high.** The decision to generate rather than parse may well be
      right, but its stated justification requires the generator and its inputs
      to be tracked, and they are not.

      **Fixed** in `118daa9`. The generator and its inputs are now tracked inside
      the crate:

      - `dialectica-core/wordlists/{adjectives,nouns,places}.txt`
      - `dialectica-core/examples/gen_wordlists.rs` — emits the three modules
      - `dialectica-core/examples/pin_name.rs` — computes a pinned name from a
        digest without linking the crate

      **Verified rather than asserted**, since the whole point is that the source
      must reproduce the pins: `sha256sum` over the three tracked files gives
      `8c998df4…de8b2`, `9c082a49…30c79` and `bed08389…815f1`, matching
      `PINNED_ADJECTIVES_SHA256`, `PINNED_NOUNS_SHA256` and
      `PINNED_PLACES_SHA256` byte for byte. So the three commands `names.rs`
      tells a reviewer to run now work on a fresh checkout — repointed at the new
      paths in the same commit.

      Both programs are `cargo run --example`, which is a stronger form of the
      fix than committing the old files would have been: the earlier `tmp/gen.rs`
      also hardcoded one machine's absolute worktree path, so it was unrunnable
      by anyone but its author even where it existed. Paths are now
      `CARGO_MANIFEST_DIR`-relative.

      `attributions.txt` is deliberately **not** carried over — it was the
      denylist's source and the denylist is deleted.

      **"Its stated justification requires the generator and its inputs to be
      tracked, and they are not" is the finding in one line**, and it is a
      decision-versus-artefact mismatch rather than a missing file: the argument
      for `&[&str]` over `include_str!` was auditability, and auditability that
      evaporates at merge is not a reason for anything. D1 now records the
      correction, including the sharper version of your point — that after the
      worktree is pruned the generated arrays *are* the hand-transcribed arrays
      the decision rejected.

- [x] **`dev-writer`** — `design.md:64-71` (D2) — the module boundary for
      `NAME_PREFIX` is justified by a coupling that does not exist
      **Scenario:** D2 puts `NAME_PREFIX` in `names.rs` rather than in
      `identity.rs` (which owns the other three separators) and defends the split
      with: *"`identity.rs`'s pin test asserts it does not collide with the
      address prefixes, which is the coupling that matters."* It does not.
      `grep -n "NAME_PREFIX\|names::" identity.rs` returns nothing;
      `the_wire_constants_are_pinned_to_known_answers` pins four derived values
      and never mentions the name prefix. `grep -n
      "AUTHOR_ADDRESS_PREFIX\|STOA_ADDRESS_PREFIX\|OP_SIGNING_PREFIX" names.rs`
      returns one hit, in a doc comment. No test anywhere asserts the four
      separators are pairwise distinct. The four *are* distinct today (each 32
      bytes, verified by hand), but the property D2 claims holds the family
      together is enforced by nothing, and the three `identity.rs` prefixes are
      private `const`s — which `cargo mutants` cannot see either. A fifth
      separator added later can duplicate one with every gate green.
      **Measured:** the mutation `NAME_PREFIX` → any of the three `identity.rs`
      values is not expressible without making them `pub`; the closest available
      probe, `the_name_digest_is_neither_the_address_nor_a_bare_hash`, compares
      *digests for one key* and cannot distinguish a colliding prefix from a
      distinct one in the general case.
      (Passing note for `design-reviewer`, not a box of mine: D2's quoted
      literal `b"/dialectica/1/Name/Display\0\0\0\0\0\0\0"` has seven NULs and is
      33 bytes; `names.rs:97` has six. The design doc's literal would not
      compile.)
      **Severity: medium** — the boundary choice is defensible, its recorded
      reason is not, and the invariant that would have made it defensible is
      absent.

      **Fixed in the record; the invariant is deliberately deferred.** D2 now
      states the real reason for the placement — `NAME_PREFIX` is the only
      constant this scheme owns and every reader of it is in this module, so
      splitting it from its users buys a tidier grouping and costs a file hop.
      That is a weaker justification than the one it replaces, which is the
      point: it is the true one.

      D2 also now records, in terms, that **no test asserts the separators are
      pairwise distinct** and that the entry previously cited a check nobody
      wrote. It is in the Risks section too, so it survives the findings file
      being deleted.

      **Deferred** rather than fixed, with the reasoning: closing it means making
      `identity.rs`'s private prefixes visible to a comparison test, which is a
      widening of that module's surface to serve a check on this one. It is worth
      doing across **all six** separators at once rather than bolted to the side
      of this change — and your own measurement is why it cannot be done cheaply
      here, since you showed the mutation is "not expressible without making them
      `pub`".

      **Your distinction between the choice and its stated reason is what made
      this answerable.** A finding that said "put the constant in `identity.rs`"
      would have been arguable; "the boundary is defensible, the recorded reason
      is false, and the invariant that would justify it is absent" splits into
      three separate dispositions — keep, correct, defer — which is what I did.

- [x] **`dev-writer`** — `feed.rs:293` — a row is silently dropped from the
      feed when its author's name cannot be derived, and this is a censorship
      filter introduced without a recorded decision
      **Scenario:** `let Ok(display_name) = … else { continue; };` sits in the
      same loop as the signature check and the moderation filter, and does the
      same thing they do: removes the post from the feed. An author whose key
      exhausts the denylist reserve (about one in 1.2 million, per
      `NameError::ReserveExhausted`) is invisible in every feed, on every peer,
      deterministically and permanently — the derivation is a pure function of
      the key, so it is not a transient failure and there is no retry that helps.
      They can post, their ops verify, other peers store them, and no one ever
      sees them. In a censorship-resistant forum, "we could not name you" is a
      novel reason for content to not exist, and it arrives as a two-line `else`
      block. The code comment labels it `NO SPEC` and reasons about it, which is
      the right instinct — but a decision this shaped belongs in `design.md`
      where the alternatives (serve the row with the address as its own name;
      make the reserve deeper; make the scheme total) can be weighed, and
      `design.md` D8 does not mention the failure path at all.
      **Measured:** the comment cites
      `a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked` as the
      test for this; `grep -rn` over `dialectica/` finds that identifier exactly
      once — in the comment citing it. No test exercises the branch. (The
      `tester` has the missing-test half of this; mine is that the decision is
      unrecorded and its shape is a filter.)
      **Severity: medium.**

      **Fixed** in `118daa9` by making the branch unrepresentable rather than by
      recording the decision.

      **You framed this as "a decision this shaped belongs in `design.md` where
      the alternatives can be weighed", and listing those alternatives is what
      answered it** — because one of them turned out to be free. Of the three you
      named (serve the row with the address and omit the name; fail the page;
      make the scheme total), **the scheme is now total**: with the denylist
      deleted every draw is one unconditional reduction, nothing after the key
      parses can fail, and `display_name` returns `DisplayName` rather than
      `Result`. There is no `Err` arm for a `continue` to sit in.

      The scope cut then removed the field, so `list_threads` derives no name at
      all. Two independent reasons the filter cannot return.

      **"In a censorship-resistant forum, 'we could not name you' is a novel
      reason for content to not exist, and it arrives as a two-line `else`
      block"** is the sentence that made this the finding it was rather than a
      coverage note. A deterministic, permanent, per-key erasure sitting in the
      same loop as the signature check and the moderation filter — doing what
      they do, for a reason nobody decided — is the right thing to have been
      alarmed by. The type now refuses to express it.

      Recorded in D8 rather than only here, since the findings file is deleted at
      merge.

- [x] **`dev-writer`** — `names.rs:66-69`, `names.rs:107`, `names.rs:149` — the
      public surface was widened past what any caller needs, with no recorded
      decision for the widest parts
      **Scenario:** the module exports `ADJECTIVES`, `NOUNS`, `PLACES`,
      `TRUE_ATTRIBUTION_PAIRS`, `CONNECTOR`, `DisplayName::words()`,
      `display_name_from_bytes`, `name_digest` and `name_from_digest`. Of these,
      only `display_name` and `DisplayName::render()` have a non-test caller —
      `grep -rn "display_name_from_bytes\|words()\|CONNECTOR"` over `feed.rs`,
      `wire.rs`, `thread.rs` and `onboarding.rs` returns nothing, and
      `grep -rn "names::ADJECTIVES\|names::NOUNS\|names::PLACES\|names::TRUE_ATTRIBUTION"`
      over `dialectica/` returns nothing. `design.md` records a reason for
      `name_from_digest` (D3, testability) and for `display_name_from_bytes`
      (D7, one-call refusal), both sound. It records none for the four `pub`
      wordlists, for `CONNECTOR`, or for `words()` — and D6's stated reason for
      `words()` is *"a caller too cramped to render `of`"*, which is the UI,
      which `design.md`'s own Non-Goals put out of scope. That is the speculative
      half of "make the change easy, then make the easy change". Four `pub`
      consensus-critical arrays are the part that matters: they are the scheme's
      private data, and exporting them invites a second reader of a list whose
      indices are the consensus.
      **Measured:** 946 of 946 tests pass; the tests are the only consumers. The
      crate's own test module reaches these through `use super::*`, so `pub` is
      not what makes them testable — `pub(crate)` would serve every present use,
      and `DisplayName`'s fields are already `pub`.
      **Severity: low to medium** — a narrowing now costs one commit; after the
      API is a contract it costs a version.

      **Deferred**, and the scope cut has made your finding stronger rather than
      resolving it. `TRUE_ATTRIBUTION_PAIRS` is gone with the denylist, so one of
      the four `pub` arrays no longer exists. But **the module now has zero
      non-test consumers of any kind** — I re-ran your greps against `feed.rs`,
      `wire.rs` and `thread.rs` after removing `displayName` and they return
      nothing at all, where before at least `display_name` had one caller.

      That is why I am not narrowing here. The spec's new requirement is that
      **core expose the derivation to callers** — it is now the *only* way a name
      is obtained, since no reply carries one. So the right surface is about to
      be decided by the piece that adds the wire method (`tasks.md` 5b.5), and
      choosing `pub(crate)` now would be narrowing a module one task before it
      needs a public entry point. Two churns instead of one, in opposite
      directions.

      **Where it lands, with what I would carry into it:**

      - `display_name`, `display_name_from_bytes` and `DisplayName::render()`
        need to be reachable by the wire layer. Not in question.
      - `name_from_digest` and `name_digest` are D3/D7's testability seam, both
        recorded and both sound.
      - **The three wordlist arrays are the part of your finding I think is
        simply right**, and the argument is yours: they are the scheme's private
        data, their indices *are* the consensus, and exporting them invites a
        second reader of a list that must never be read twice. Nothing outside
        the module has ever touched them.
      - `CONNECTOR` and `words()` — `words()`'s recorded reason in D6 is "a
        caller too cramped to render `of`", which is the view, which D1's own
        Non-Goals put out of scope. You caught a decision justified by a caller
        the document says does not exist.

      Flagged to the piece that adds the method, and left in D6/Risks so it does
      not evaporate with this file. **Your cost framing is why it must not be
      forgotten rather than why it must be done now**: a narrowing costs one
      commit today and a version once the API is a contract — and that contract
      does not exist yet, which is exactly the window.

## What was clean

The **identicon window move** (`Identicon.qml`, D9) is the best-shaped part of
this change. It is a uniform −8 shift that preserves which dimension reads which
relative position, so every modulus and the mark's own perceptual-space argument
survive untouched; the comment it replaces made a claim about a mechanism that
never existed and the replacement says so explicitly rather than quietly
correcting it. The disjointness argument it now makes — that the name and mark
are independent because they hash *different digests under different prefixes*,
not because address bytes were allocated between them — is the correct one, and
it is the kind of correction that removes a constraint rather than adding one.

The **`DisplayName` three-field struct** (D6) delivers what it claims: the
connector genuinely does not exist in the data, it is emitted only in `render()`,
and "the connector is not a slot" holds structurally rather than by a guard
repeated at call sites. This is complexity in the data structure rather than the
logic, done correctly.

The **denylist as a sorted `&[(u16, u16)]` with `binary_search`** (D5) is well
argued and well covered. Its reason — that sortedness is a property a test can
assert and a `HashSet`'s iteration order is not — is the right axis to decide on,
and it is enforced: neutering the guard (`if false && is_refused(…)`) fails 3 of
918 tests (`a_refused_pair_redraws_every_slot_from_the_reserve`,
`exhausting_the_reserve_fails_rather_than_reading_on`,
`the_redraw_path_is_pinned_to_a_written_down_name`). The whole-name redraw over a
per-slot redraw is also correctly reasoned: it keeps termination arithmetic
rather than making it a property of the denylist's shape.

`draw_at` as one function called at two offsets, rather than a first draw and a
redraw written twice, is the right factoring for the reason the doc comment
gives.

## What a gate cannot see

- **No gate sees the `thread_page_json` gap.** The suite pins that surface's key
  set exhaustively and positively asserts no derived name is there, so the two
  surfaces' disagreement is invisible to every gate by construction.
- **No gate sees the missing generator.** `tmp/` is gitignored, so CI never
  looks for it; the wordlist SHA pins compare the arrays against hashes typed
  into the test file, which detects drift but cannot detect that the files those
  hashes were taken over are gone.
- **`cargo mutants` cannot see any of the four domain separators**, nor the list
  sizes, nor the denylist contents — they are `const` values, and it mutates
  functions. The pins in `names.rs` are what covers them; there is no mechanical
  backstop.
- **CI's test-count gate is unaffected** by this change: it rglobs
  `dialectica/rust-lib` for `#[test]`, so `names.rs`'s 34 are counted and run.
  No QML spec file was added or removed, so the QML spec-count floor is
  unaffected too.
