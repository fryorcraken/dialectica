# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer` — The four lists are written and the crate is
      green. Sizes are exactly 8,192 / 1,024 / 1,024 and the denylist holds 199
      pairs. **All four modules are generated from text files by `tmp/gen.rs`
      rather than transcribed**, which is what makes the attestation screen
      auditable: a reviewer can trace an entry to a census file and a census file
      to its source. Yields were 17,349 / 1,892 / 1,131, so every list was cut
      down rather than padded up.
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

## Implementation

> **The sizes are settled at 8,192 / 1,024 / 1,024 and the options in `design.md`
> are closed.** The owner has ruled; the powers of two are load-bearing for the
> modulo argument and are not reopening.
>
> **The earlier censuses do not measure the current scope.** Both were written
> against a noun slot read as "the vocabulary of Greek thought plus named
> thinkers", which is a narrow technical vocabulary. The slot is now **any
> attested ancient Greek noun**, and the two pools that were absent entirely —
> mythological figures and ordinary concrete nouns — are the larger half. A count
> taken under the old scope is not evidence about the new one, in either
> direction. The place census (`tmp/places-census/`, 1,070 deduplicated) is under
> unchanged scope and does still apply, with the caveat its own author recorded:
> about 850 entries they stand behind and about 220 they would cut first, so 1,024
> clears only by keeping soft material.
>
> **What every remaining list task must not do is pad.** Near-duplicate
> transliterations, Latinised doublets and invented toponyms each fail the spec's
> attestation screen, and **a fabricated Greek word is invisible to a reviewer and
> uncatchable by any test** — it is the one requirement the suite structurally
> cannot fail on. Build the lists from sources, not from recall.

## 1. The derivation

- [x] 1.1 `names.rs` carries `NAME_PREFIX`, `NAME_DIGEST_BOUND = 12` and
      `name_from_digest`. `the_derivation_reads_no_byte_past_its_bound` builds two
      digests agreeing inside the bound and differing on **every** byte beyond it
- [x] 1.2 `DisplayName` is three `&'static str`; `render()` is the one place the
      connector is emitted, so "the connector is not a slot" holds **by
      construction** — it does not exist in the data.
      `a_name_is_three_drawn_words_and_a_fixed_connector` counts by **slot, not by
      space-separated token**, because a two-word place makes the token count 5
- [x] 1.3 `display_name(&PublicKey)` hashes `NAME_PREFIX || key`;
      `the_name_digest_is_neither_the_address_nor_a_bare_hash` pins it against both
- [x] 1.4 `display_name_from_bytes(&[u8])` parses first. Lengths `0, 1, 31, 33, 64,
      1024`, a non-point, and the all-zero low-order key are each refused. **The
      wrong-length case is satisfied-by-construction for every caller holding a
      parsed key**: `display_name` takes `&PublicKey`, which cannot be built from
      malformed bytes

## 2. The wordlists

- [x] 2.1 `names/adjectives.rs` — 8,192 entries, derived from
      `/usr/share/dict/words` by suffix (see design D12). Swept for size, ASCII,
      lowercase, well-formed spacing and duplicates
- [x] 2.2 `names/nouns.rs` — 1,024 entries from the four pools, 1,892 counted.
      Same sweep, plus `no_noun_entry_carries_the_connector_as_a_word`. **No
      exclusion test**: the three that asserted `stoa`, `platon`, `sokrates`,
      `archon` and `strategos` were ABSENT are deleted, and
      `the_lists_carry_no_exclusion_of_any_kind` asserts they are **present** —
      their absence would now be the defect. That test is pinned positively rather
      than merely omitted, so a later curation pass that quietly dropped them
      cannot reintroduce the withdrawn screen with the suite still green
- [x] 2.3 `names/places.rs` — 1,024 entries, 1,131 counted, 10 multi-word.
      `a_multi_word_place_entry_is_accepted_and_renders_as_one_place` asserts the
      list holds at least one, so the single-word screen cannot creep back
- [x] 2.4 `every_index_of_every_list_is_reachable_and_uniformly_so` sweeps the full
      16-bit range and **counts into a vector** rather than asserting from the
      arithmetic, since the arithmetic is what is under test

## 3. The denylist and the redraw

- [x] 3.1 `names/denylist.rs` holds 199 sorted `(noun, place)` pairs, generated
      from `tmp/attributions.txt`. Sortedness is asserted with a strict `<` over
      `windows(2)`, which checks sorted **and** deduplicated in one pass — and it
      is the precondition `binary_search` needs, which an unsorted array would
      break *silently*
- [x] 3.2 Whole-name redraw from the reserve.
      `a_refused_pair_redraws_every_slot_from_the_reserve` constructs the digest so
      the two draws differ in **all three** slots, so a scheme substituting only
      the offending pair fails it
- [x] 3.3 `NameError::ReserveExhausted` on two refused draws, reached through a
      constructed digest — through a key it is reachable only by grinding
- [x] 3.4 `an_unrefused_draw_never_consults_the_reserve` varies only bytes 6..11

## 4. The pins

- [x] 4.1 Three written-down names, produced by `tmp/pin.rs`, which reads the
      wordlists from the **text files** and does the index arithmetic itself rather
      than calling `name_from_digest`. `the_name_scheme_is_pinned_to_known_answers`
      pins the common path and the digest separately;
      `the_redraw_path_is_pinned_to_a_written_down_name` pins the redraw;
      `the_pinned_name_is_derivable_by_hand_from_the_pinned_digest` re-derives the
      indices in the test and asserts them as literals.
      **Proved by mutation**: renaming one noun entry (`karpos`) failed both pins
      and the character sweep — 3 failures, 29 passes

## 5. The feed row

- [x] 5.1 `FeedRow::display_name` is filled from `entry.op.op.author` and the false
      doc comment is gone. Pinned to the **written-down** name for the signing key,
      not to the derivation's own output
- [x] 5.2 `displayName` is in `feed_page_json` and in the key-set test; the address
      is unchanged
- [x] 5.3 `a_rows_name_follows_the_key_that_signed_not_the_rows_position` exchanges
      the two posts' order, and asserts the two pinned names differ so the check is
      not trivially satisfied by one name matching both branches

## 6. The identicon window

- [x] 6.1 Move `Identicon.qml`'s eight reads from `12..19` to `4..11` and correct the
      false "bytes 0..11 are reserved" comment; verify `tst_identicon.qml`'s pinned
      selectors move with it — `e62ddde`. The ink-distinctness sweep moved too: it
      varied bytes 13/14/15, which the mark no longer reads, so it would have swept
      a constant and passed while proving nothing.
- [x] 6.2 Update `docs/IDENTICON.md`'s byte-layout section and its "two flaws" note;
      verify by grep that no `12..19` claim about the mark survives — `e62ddde`.
      The surviving mentions are historical and read as history.

## 7. Documents

- [x] 7.1 Update `docs/PLAN.md`'s overlap paragraph and `docs/UI-BRIEF.md`'s stale
      four-word sentence; verify by grep that no four-word claim survives —
      `4a42336`. PLAN.md's correction of the old "reserved for the identicon"
      draft keeps its quoted window deliberately: the point it makes is that
      *which* eight bytes the mark reads has never mattered, so editing the
      window out of it would have orphaned the argument.
