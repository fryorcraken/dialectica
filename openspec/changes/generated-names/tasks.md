# Tasks

## Stages

- [x] spec — `spec-writer`
- [ ] design + code — `dev-writer` — **Unblocked on the owner decision; the
      remaining work is curation.** The sizes are settled at 8,192 / 1,024 /
      1,024 and are not reopening. What moved is the *source*, not the arithmetic:
      the noun slot is now any attested ancient Greek noun across four pools —
      mythological figures and ordinary concrete nouns included — where the census
      that counted 551 was written against a noun slot read as the vocabulary of
      Greek thought alone. That census does not measure the current scope and is
      not evidence about it. Every exclusion is withdrawn, which removes the
      screening that dominated the estimate. Four wordlists remain unwritten and
      **must be built from sources rather than recall**: the spec's attestation
      screen is the one requirement no test can check, so a fabricated entry ships
      silently.
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

- [ ] 1.1 Add `names.rs` with `NAME_PREFIX`, the 12-byte budget and
      `name_from_digest`; verify with a test that two digests agreeing inside the
      bound and differing past byte 12 yield equal names
- [ ] 1.2 Add `DisplayName` as three `&'static str` with `to_string()` emitting the
      connector; verify a name has three drawn words and the connector between the
      second and third
- [ ] 1.3 Add `display_name(&PublicKey)` hashing `NAME_PREFIX || key`; verify the
      name digest differs from the key's address and from a bare SHA-256 of the key
- [ ] 1.4 Add `display_name_from_bytes(&[u8])`; verify every length in
      `[0, 31, 33, 64]` and the all-zero low-order key return an error, not a name
      and not a panic

## 2. The wordlists

- [ ] 2.1 Add `names/adjectives.rs` — exactly 8,192 entries; verify the length
      assertion and the ASCII/lowercase/well-formed/no-duplicate sweep
- [ ] 2.2 Add `names/nouns.rs` — exactly 1,024 entries, any attested ancient Greek
      noun across the four pools; verify the same sweep plus that no entry contains
      the connector surrounded by spaces. **No exclusion check**: there is no
      project-vocabulary list and no excluded figure, so `stoa`, `platon` and
      `sokrates` are all admissible and a test asserting otherwise is wrong
- [ ] 2.3 Add `names/places.rs` — exactly 1,024 Greek places; verify the same sweep,
      and that a multi-word entry is accepted rather than rejected
- [ ] 2.4 Verify every index of each list is reachable and reduction is uniform, by
      sweeping the full 16-bit draw range and counting

## 3. The denylist and the redraw

- [ ] 3.1 Add the sorted `(noun_index, place_index)` denylist; verify it is sorted,
      deduplicated and in range for both lists
- [ ] 3.2 Implement the whole-name redraw from the reserve; verify over a
      constructed digest that all three slots change, not just the pair
- [ ] 3.3 Implement `ReserveExhausted` when both draws are refused; verify over a
      constructed digest that it errors and returns no name
- [ ] 3.4 Verify an unrefused draw ignores the reserve, by varying only bytes 6..11

## 4. The pins

- [ ] 4.1 Pin `display_name` for a fixed key to a **hardcoded** expected name, and
      pin the redraw path to a second hardcoded name; verify both fail if a
      wordlist entry is exchanged

## 5. The feed row

- [ ] 5.1 Add `FeedRow::display_name`, filled from `entry.op.op.author`, and correct
      the false doc comment; verify a row's name equals the hardcoded pinned name
      for the signing key
- [ ] 5.2 Add `displayName` to `feed_page_json` and to the key-set test; verify the
      address is unchanged and no other field moved
- [ ] 5.3 Verify a name follows the key that signed rather than the row position, by
      exchanging two posts' order

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
