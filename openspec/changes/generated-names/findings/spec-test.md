# spec-test review — `generated-names`

Read the spec and the tests; did not read `names.rs`'s non-test code,
`names/*.rs` bodies or `design.md` to judge adequacy. The implementation was
touched only to run the six mutations below, each restored before the next.

Baseline confirmed: `cargo test -p dialectica -p dialectica-core` = 918 + 28 =
946, 0 failed. QML: `dialectica-ui/tests/run-qml-tests.sh` ran all four spec
files here (42 assertions), so the QML findings below are measured, not inferred.

## Mutations run

| # | mutation | result |
|---|---|---|
| 1 | `places.rs` exchange `araden`/`araithyrea` (indices 100/101) | **caught** — only by `every_wordlist_is_pinned_entry_by_entry_and_in_order` |
| 2 | `NAME_PREFIX` version byte `1` → `2` | **caught** — 6 tests |
| 3 | delete two `TRUE_ATTRIBUTION_PAIRS` entries (`(68,603)`, `(69,603)`) | **SURVIVED** — 946/946 green |
| 4 | `Theme.middleChars` `8` → `24` | **SURVIVED** — 946 Rust + 42 QML green |
| 5 | `CONNECTOR` `"of"` → `"from"` | **caught** — 8 tests |
| 6 | redraw offset `draw_at(digest, 6)` → `draw_at(digest, 12)` (reads past the bound) | **caught** — 3 tests |
| 7 | `Theme.headChars` `8` → `24` | **SURVIVED** — 946 Rust + 42 QML green |
| 8 | `feed.rs:293` drop-the-row → placeholder `"unknown participant"` | **SURVIVED** — 946/946 green |
| 9 | add an `authorLabel` field to the thread item's JSON | **caught** — `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others` |

Four survived. Three of the four are `const` values or QML `Theme` properties —
where this change's risk is concentrated, and which `cargo mutants` structurally
cannot reach, since it mutates functions.

Mutations 7 and 8 confirm two findings other reviewers reported; I ran them
rather than relaying them, and the numbers above are mine. Mutation 9 refutes a
suspicion of my own — the thread item's field set *is* pinned exactly, which
makes the contradiction below sharper rather than softer.

## Findings

- [x] **`tester`** — `names/denylist.rs` — **the denylist's 199 pairs are pinned
      by nothing.** `the_denylist_is_sorted_deduplicated_and_in_range` checks
      sortedness, range and non-emptiness; `a_refused_pair_redraws_every_slot...`,
      `a_redraw_is_deterministic` and `exhausting_the_reserve...` reference only
      `[0]` and `[1]`. Nothing covers entries 2..198. The spec lists "changing the
      denylist" among the changes that mint a new version, so this is a
      consensus-critical constant with the same standing as a wordlist — and the
      wordlists *are* pinned entry-by-entry by SHA-256.
      **Scenario:** a curation pass, a bad merge or a regenerated
      `tmp/attributions.txt` drops a pair; those noun–place draws become
      reachable, and one identity in a few thousand renders under a real person's
      canonical name on updated peers and not on others.
      **Measured:** deleted `(68, 603) // anaximandros of miletos` and
      `(69, 603) // anaximenes of miletos` — both real figures' canonical names.
      All 946 tests passed. The fix is the shape already used for the wordlists:
      a SHA-256 over the pairs in order, written down from the generator's source
      text, not from hashing the array. Severity: high.

      **Note from `dev-writer`, not a tick — this box is yours.** The denylist is
      **deleted** in `118daa9` under the owner's no-filter ruling, along with
      `is_refused`, the redraw, the reserve bytes and `ReserveExhausted`. There
      are no 199 pairs left to pin, so the fix you prescribe has nothing to
      apply to.

      Your reasoning is why it could be deleted cleanly rather than needing the
      pin: a consensus-critical constant that nothing pins, and whose
      *completeness* was a claim about the world no test could check, is a shared
      table that could drift between peers and rename somebody. That is recorded
      in `design.md` D5 as one of the deletion's benefits, crediting the
      mechanism you identified.

      **MOOT BY DELETION — no test written, and none should be.** Verified in
      this worktree rather than taken from the note: `names/denylist.rs` does not
      exist, and `grep -rnw` for `TRUE_ATTRIBUTION_PAIRS`, `draw_at`,
      `ReserveExhausted` and `denylist` under `dialectica/` returns only prose
      comments describing the deletion, plus stale `target/` build-artefact
      dependency files. No source symbol survives. There are no 199 pairs to pin.

      The finding was right and it is the reason the deletion was clean rather
      than merely convenient. Recording what replaced it, so this box does not
      read as an unpinned constant that was waved through: the consensus-critical
      constants that remain — the three wordlists, `NAME_PREFIX`, `CONNECTOR` —
      are each pinned against a value produced off-implementation.
      `every_wordlist_is_pinned_entry_by_entry_and_in_order` hashes the source
      TEXT FILES rather than the arrays, which is the shape you prescribed here,
      applied to the constants that survived.

- [x] **`tester`** — `dialectica-ui/tests/` — **the abbreviation side of the
      channel-disjointness requirement has no test at all.** `AddressLabel.qml`
      has no `tst_` file. `tst_identicon.qml` asserts disjointness only from the
      mark's side (flipping displayed bytes leaves every selector alone) and
      derives its byte map — head 0..3, middle 14..17, tail 29..31 — from
      `Theme.headChars`/`middleChars`/`tailChars` **in a comment**, hardcoding
      the result. Nothing reads those properties, so the relation
      `mark_bytes ∩ displayed_bytes = ∅` is asserted against a frozen snapshot of
      one side of it.
      **Scenario:** either group is widened for legibility. `AddressLabel`
      centres the middle group, so at `middleChars: 24`,
      `start = floor((64−24)/2) = 20` — hex chars 20..43 = bytes 10..21, which
      overlaps the mark at bytes 10 and 11. `headChars: 24` is worse: the head
      becomes bytes 0..11 and puts the mark's *entire* window on screen. Both are
      precisely the overlap this change moved the mark from 12..19 to 4..11 to
      eliminate, and an attacker grinding a lookalike mark can again read their
      progress off the rendered address.
      **Measured:** both variants run separately — `middleChars: 24`, then
      `headChars: 24`. Each time all 42 QML tests and all 946 Rust tests passed.
      A test must compute the displayed byte set from the three `Theme`
      properties and assert it disjoint from `4..12`. Severity: high.

      **FIXED — and the prescribed fix turned out to be the weaker of the two
      shapes, which is the part worth recording.** `tst_identicon.qml` now
      carries `test_the_byte_probes_find_the_windows_they_should` and
      `test_the_mark_and_the_abbreviation_share_no_byte`. Neither computes the
      displayed set from the `DTheme` properties; both **measure `AddressLabel`**
      by varying one address byte at a time and watching the rendered text, and
      measure `Identicon` the same way by watching its selectors. Seven probe
      values per byte rather than one, because `_weave()` is `_byte(11) % 3` and
      `0x00 % 3 == 0xff % 3`, so a single flip concludes the mark does not read
      byte 11.

      **Measured on your exact mutation.** `DTheme.headChars: 24` → 2 of 11
      identicon tests fail, naming byte 4 and printing both windows. The
      `middleChars: 20` variant → the same 2, naming byte 11.

      **Why not the computed-from-`DTheme` version you prescribed.** A third
      test of that shape existed (added by the `dev-writer`, which offered to
      withdraw it). It catches both `DTheme` widenings, but it redoes
      `abbreviate`'s arithmetic in the test instead of asking `AddressLabel` what
      it shows — so it can only see its *inputs* move, never the arithmetic.
      Measured: shifting the head group to `body.substr(8, h)` with `DTheme`
      untouched puts mark byte 4 on screen, and **that test passed** while both
      probe tests failed. It was deleted rather than kept as a third guard, and
      its one genuine contribution — naming the three `DTheme` settings in the
      failure message — is folded into the surviving probe test's message. The
      reasoning is recorded in a comment above it so the shape is not
      reinvented.

      Your scenario arithmetic was correct in both variants and is what made the
      two-sided requirement legible; the correction is only about which side of
      the component a test should read.

- [x] **`spec-writer`** — spec.md:599 vs `openspec/specs/thread-read/spec.md:169`
      — **two live `SHALL`s in direct opposition, and the tests enforce the one
      this change loses.** This change: "***Every* reply in which core reports who
      authored something SHALL carry that author's display name alongside the
      author's address**" — universal, with all seven of its scenarios about the
      feed. `thread-read`: "***No item SHALL carry a display name***", with its own
      scenario "No item carries a derived display name" at :192. A `read_thread`
      item reports an author and is a reply, so both cannot hold.
      `openspec validate --strict` passes because the contradiction spans two
      capability files — the failure mode `docs/OPENSPEC-ARCHIVE.md` warns about.
      **Scenario:** a view author reads this change's requirement, expects a name
      on a thread item, finds none, and cannot tell a bug from the contract.
      **Measured:** the losing side is not merely tolerated but actively pinned. I
      added an `authorLabel` field to the thread item's JSON;
      `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others`
      (`wire.rs:5991`) failed on the exact key set, and
      `the_wire_reports_the_author_as_an_address_and_a_key_and_no_name`
      (`wire.rs:6076`) guards four spellings by name. So the suite firmly enforces
      `thread-read` and therefore firmly violates this change's requirement as
      written. Full suite under that mutation: 917 passed, 1 failed.
      **The same shape appears a second time**, weaker but worth settling in the
      same pass: `openspec/specs/identity-onboarding/spec.md:655` — "A reply of
      this capability SHALL therefore carry no display name and no visual mark."
      Whether a slate candidate "reports who authored something" is arguable, but
      the universal quantifier collides with at least two live capabilities and
      should be narrowed rather than left to be read charitably. Severity: high —
      this is the one finding that blocks on its own.

      **FIXED — this change's requirement narrowed; `thread-read` untouched.**
      The requirement is now "A reply reporting an author SHALL carry either the
      name or the key it derives from": a reply carrying only an address owes the
      name, and a reply carrying the public key **SHALL NOT** also carry it,
      because two derivable identifiers on the wire could disagree and the
      recipient could not tell which was wrong. The old universal quantifier was
      reasoned entirely from the address-only case, which is why the narrow
      reading was the intended one — `thread-read` ships `authorKey` precisely so
      a holder can derive the name. Under the rule as restated, `thread-read` and
      `identity-onboarding` are both **satisfying** it rather than excepted from
      it, so the second instance you flag dissolves without amending that
      capability either. Two scenarios added: "A reply carrying the author's key
      carries no name" and "A reply carrying only an address carries the name".
      The suite that pins the thread item's key set stays correct as written.

- [x] **`tester`** — `names.rs:1336` `a_refused_pair_leaves_both_of_its_words_drawing_freely`
      — **the test cannot fail on the property it names.** Its body is
      `NOUNS.contains(&NOUNS[noun as usize])` and
      `PLACES.contains(&PLACES[place as usize])`: it takes an element out of an
      array and asks the array whether it contains it. That is `true` for every
      in-range index, and in-range is already guaranteed by
      `the_denylist_is_sorted_deduplicated_and_in_range`.
      **Scenario:** a curation pass removes `straton` from the noun list while
      leaving `(straton, lampsakos)` on the denylist — exactly the word-level
      exclusion the requirement forbids. This test still passes, because the
      index now names a different word which the array of course still contains.
      **Measured:** by reading; no mutation can make it fail without producing an
      out-of-range index, which a different test already catches. To assert the
      stated property it must compare against a written-down word, e.g. that
      `NOUNS[11] == "agatharchides"` and `PLACES[436] == "knidos"` for the pairs
      it names. Severity: medium — this is the "two explanations, one answer"
      family in `.claude/agents/README.md`.

      **Note from `dev-writer`, not a tick.** The test is **deleted** in
      `118daa9` with the denylist it iterated.

      Your diagnosis stands on its own and I have applied it elsewhere:
      `NOUNS.contains(&NOUNS[i])` is `true` for every in-range index, so the
      assertion could not fail for the reason its name gave. Where I needed a
      similar claim in the replacement tests I pinned **written-down words** —
      `a_real_figures_canonical_citation_is_returned_like_any_other_draw`
      asserts the rendered string `"pensive zenon of kition"` and the literal
      indices `(5136, 1015, 431)`, both produced by `examples/pin_name.rs`
      off-implementation, rather than asking the arrays about themselves.

      **MOOT BY DELETION, and the diagnosis was applied rather than merely
      noted.** Confirmed in this worktree: `grep -rnw` finds no
      `a_refused_pair_leaves_both_of_its_words_drawing_freely` in any source
      file. No replacement was written — the test named a property of a filter
      that no longer exists.

      The `NOUNS.contains(&NOUNS[i])` shape is the thing here that outlives the
      deletion, and it is worth stating as a rule rather than an incident:
      **asking a collection whether it contains an element taken out of it is
      true for every in-range index**, so the assertion measures index validity
      and nothing else. I swept the surviving `names.rs` tests for it while
      closing the other boxes. `an_unknown_key_still_derives_a_name` carries
      three `ADJECTIVES.contains(&name.adjective)`-style lines, but those are not
      the shape: the word comes from the DERIVATION rather than from the array,
      so the assertion is "what the derivation returned is a list member", which
      a derivation returning an invented string would fail. The rewritten
      `every_index_of_every_list_is_reachable_and_uniformly_so` indexes its lists
      too, but it counts the words the derivation returned and compares the
      totals against `65_536 / len` — a written-down expectation, not the array
      agreeing with itself.

- [x] **`tester`** — spec.md:826, "A different scheme version gives a different
      name for one key" — **no test, and the `tasks.md` claim that it needs the
      API widened for tests alone is wrong.** `NAME_PREFIX` is module-private but
      `mod tests` is inside `names.rs` and reaches it through `use super::*`.
      **Measured:** I wrote the test in the existing test module with no API
      change — copy `NAME_PREFIX`, set byte 12 to `b'2'`, hash `v2 || key`, pass
      the digest to `name_from_digest`, and compare with `display_name(&key)`. It
      compiles and passes; all 39 keys differ between versions. This is the one
      scenario guarding the versioning requirement's whole purpose and it is
      currently unpinned. Severity: medium.

      **FIXED —** `a_different_scheme_version_gives_a_different_name_for_one_key`
      in `names.rs`'s test module. No API widened, exactly as you said: `mod
      tests` reaches `NAME_PREFIX` through `use super::*`. It copies the shipped
      separator, sets byte 12 to `b'2'`, hashes `v2 || key` **in the test body**
      rather than calling `name_digest`, and compares against `display_name`
      across 39 seeds.

      Two guards ahead of the sweep, because the comparison is vacuous if either
      fails silently: the version byte is asserted to be at index 12 (if it
      moves, the test would bump the wrong byte and compare v1 with v1), and the
      two separators are asserted to differ.

      **Proved it can fail, in both halves.** Shipping `NAME_PREFIX` at version
      `2` fails the index guard, naming it — `left: 50, right: 49`. Reaching the
      property assertion itself needs a derivation that ignores its digest, so I
      pinned all three slot indices to `0`: `seed 1 renders identically under
      both scheme versions, so a v1 name is silently reproducible by v2`, with
      both sides printed as `"abandonable acheron of abai"`. Implementation
      restored after each.

- [x] **`tester`** — spec.md:831, "Removing a word renames identities that drew
      past it" — **no test, and also testable without widening the API.** The
      lists are `const` arrays, but the test module can build the shortened list
      as a `Vec` and assert the reindexing by hand.
      **Measured:** I wrote it — filter index 100 out of `PLACES`, then assert
      every index at or after 100 renders a different place and every index below
      is untouched. Compiles and passes. Severity: low — the mechanism is real
      and worth pinning, but `every_wordlist_is_pinned_entry_by_entry_and_in_order`
      already catches the removal itself.

      **FIXED —** `removing_a_word_renames_identities_that_drew_past_it`, built
      the way you describe: `PLACES` filtered into a `Vec` with index 100
      dropped, reindexing asserted by hand. No API widened.

      One strengthening on your sketch. "Assert every index at or after 100
      renders a different place" passes on any reshuffling of the tail, where the
      spec's claim is the specific one that everything shifts down by exactly
      one. So the tail is compared against `PLACES[i + 1]` — a written-down
      relation — and the head against `PLACES[i]`, with both sweeps counted so
      neither can pass by not running.

      **Proved it can fail.** The list-side mutation is the one that matters
      here, since the reindexing arithmetic lives in the test: I made index 101
      duplicate index 100 (`araithyrea` → `araden`), which makes the removal
      invisible at that index. The test fails with `moved: 922, expected 923` and
      a message naming the cause. `places.rs` restored.

      **A note on the index, because the brief warned about exactly this and it
      was live here.** `PLACES`'s literals begin at **line 24**, so index = line −
      24: `araden` is index 100 at line 124 and `araithyrea` is index 101 at line
      125. A grep line number taken as an index selects a different word and pins
      the wrong answer silently. Cross-checked against the spec-test reviewer's
      independent measurement of the same pair, which agrees.

- [x] **`tester`** — spec.md:711, "No method accepts a name where an identity is
      required" — **no test.** `design.md` D10 says "the test asserts the
      refusal", and `a_forbidden_field_is_refused_on_every_operation` is about a
      request carrying an unknown *field name* (`displayName`). The scenario is
      about a name-shaped *value* in a known field — `"target": "measured aporia
      of lampsacus"` — which is a different path entirely (hex parsing, not field
      screening).
      **Measured:** I wrote the probe in `wire.rs`'s test module: a vote whose
      `target` is a rendered name and a reply whose `parent` is one. Both are
      refused, so the behaviour is right and the coverage is missing. Severity:
      medium — the requirement is load-bearing for "the address is the identity".

      **FIXED —** `no_method_accepts_a_display_name_where_an_identity_is_required`
      at `wire.rs:5748`, covering five method/field pairs as a table:
      `publish_vote/target`, `publish_vote/stoa`, `publish_reply/parent`,
      `publish_reply/stoa`, `publish_post/stoa`. The names used are REAL derived
      names for real keys plus the written-down colliding name, and the
      connector-dropped form, rather than name-shaped strings — a handler that
      happened to reject `"not hex"` while resolving something name-shaped would
      pass a weaker fixture.

      **Re-verified against the cut rather than assumed.** The scope cut removed
      `displayName` from every reply, which could have made this test vacuous if
      it had depended on a name being on the wire. It does not — it derives the
      names by calling `display_name` directly — and it still passes on the
      current tree.

      **The third assertion is live, and the first two are not enough.** Its job
      is to distinguish a refusal AT THE PARSE from a later failure, by requiring
      the error message to start with the field name. Proved by mutating
      `required_op_id` to resolve an unparseable value by hashing it into an
      `OpId`: the request is then refused later as *"this peer does not hold the
      target def243e…"*, which **passes** the `error`-present and no-`opId`
      checks and fails only the third. That is the mutation the test's own
      comment claims it catches, now measured on this tree rather than inherited.
      `wire.rs` restored.

- [x] **`tester`** — `names.rs:1068` `a_failure_is_never_reported_as_a_name` —
      **the second half of the test asserts nothing it claims to.** The comment
      says "a 9-byte key and the same bytes zero-padded must not agree", the code
      builds `padded`, calls `display_name_from_bytes(&padded)` and discards the
      result with `let _ =`. No comparison happens. The trailing comment
      ("whether the padded value parses at all is incidental") concedes this, but
      the code then reads as covering the no-padding requirement and does not.
      **Scenario:** an implementation that zero-pads a short key to 32 bytes and
      derives a name from it. The first half catches it only because 9 bytes is
      refused — a padding implementation would return `Ok` there and fail the
      first half, so the property is covered by accident and not by the half
      written for it. Either delete the dead half or assert the refusal reason.
      Severity: low.

      **FIXED, and the fix turned up something that raises this above low.** The
      dead half is replaced with a live assertion, but writing it exposed that
      **the fixture could not have worked as either of us assumed**: zero-padding
      `b"too short"` does not produce a valid public key. Those 32 bytes are not
      a decompressable Edwards point, so `display_name_from_bytes` refuses them
      at the parse. A test built on that fixture — including the one your
      suggestion describes — would have passed on a padding implementation,
      because the padded value fails for an unrelated reason. That is this
      repo's "constant that looks obviously valid and is not" trap, the
      all-`0xFF`-Ed25519 case in a different costume, and it is why the corrected
      test now **searches** for a short prefix whose zero-padded form genuinely
      parses (`short_key_whose_padding_is_a_valid_key`, documented with why the
      intuitive choice fails) instead of writing one down.

      With a live fixture the test asserts three things: the padded value parses
      (so padding is a reachable route to a name), the short form is refused, and
      the short call does not arrive at the padded value's name.

      **Proved it can fail.** Mutating `display_name_from_bytes` to zero-pad any
      short input to 32 bytes before parsing: *"a 2-byte key must be refused
      rather than widened"*. The old `let _ =` half could not fail under that
      mutation, which is the finding. `names.rs` restored.

      So: your severity was right about the release and wrong about the class —
      the half was not merely dead, it was unwritable as sketched, and nobody
      would have discovered that without trying to make it assert something.

- [x] **`tester`** — `feed.rs:292–293` — **a phantom citation over an uncovered
      branch, and the uncovered direction is the spec-forbidden one.** The
      `NO SPEC:` block says "See
      `a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked`"; a
      repo-wide grep finds that name only in this comment. No test exercises a
      feed row whose author's derivation fails, in either direction.
      **Scenario:** the `continue` is replaced by a placeholder name — which
      spec.md:776 forbids by name: "There SHALL be no placeholder name, no name
      for 'unknown' [...] each would render as an ordinary participant, which is
      a name attributable to nobody presented as one attributable to somebody."
      **Measured:** substituted `"unknown participant"` on the `Err` arm; 946/946
      passed. So the requirement's own prohibition is unenforced, and the comment
      pointing at the test that would have enforced it names a test that was
      never written — which is worse than silence, because it closes the
      question for the next reader. Severity: medium-high. (Four other reviewers
      found this; the measurement above is my own run, not a relay.)

      **Note from `dev-writer`, not a tick.** Both the branch and the citation
      are **deleted** in `118daa9`, and the branch is now unrepresentable rather
      than untested: with the denylist gone the derivation cannot fail for a
      well-formed key, so `display_name` returns `DisplayName` rather than
      `Result` and there is no `Err` arm for a placeholder to occupy. The scope
      cut then removed the field, so no feed row derives a name at all.

      What replaces it, in case you want to verify rather than take it: the
      absence is pinned positively by
      `the_feed_reply_is_the_ecosystems_pagination_shape`, which asserts the feed
      row's whole key set — I restored a `displayName` and watched it fail on the
      added key. `FeedRow` is also destructured exhaustively in two tests, so a
      re-added field stops them compiling.

      Your framing of *why* it mattered — "worse than silence, because it closes
      the question for the next reader" — is quoted in `design.md` and is the
      rule I worked to for every cross-reference in this pass.

      **MOOT BY DELETION — verified rather than taken on the note, which is what
      this finding asks of a reader.** In this worktree: `grep -rnw` for
      `a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked` over
      `dialectica/` and `dialectica-ui/` returns nothing, and `grep -n
      "display_name\|displayName"` over `feed.rs` returns five hits, all inside
      its test module — no production path in `feed.rs` derives a name at all.
      There is no `Err` arm for a placeholder to occupy and no row for a
      placeholder to reach.

      **No test was written for the deleted branch, deliberately.** A test over a
      path that should not exist guards the wrong thing; removing the path
      removes the class, which is the stronger fix and the one taken.

      The positive replacement checks out too: `the_feed_reply_is_the_ecosystems_
      pagination_shape` exists at `wire.rs:5683`, and
      `a_row_carries_the_address_and_no_derived_display_name` (`feed.rs:631`)
      pins the struct from the other side. Both cited tests exist — I ran the
      grep rather than trusting the citation, which is the whole point of this
      box.

- [x] **`spec-writer`** — spec.md:658, "A reply with no author carries no name
      field", and spec.md:746, "A name is unchanged by every surrounding state" —
      **neither scenario has a test.** A repo-wide grep finds no test deriving a
      name twice across differing Stoa/moderator/identity state, and none reading
      a reply that reports no author and checking for the absence of a name field.
      Both are testable as written; report so the spec-writer can confirm they are
      wanted before the `tester` writes them. Severity: low.

      **CONFIRMED WANTED — both stay, and both are now load-bearing.** "A name is
      unchanged by every surrounding state" is what makes the determinism claim
      checkable against the *surroundings* rather than only against repetition,
      and it sits under a requirement that now also states the derivation is
      total over well-formed keys with malformed input as its only failure — so
      the `tester` writing it also pins that there is no state-dependent refusal.
      "A reply with no author carries no name field" is now one of four scenarios
      under a requirement whose whole content is which reply shape carries what,
      and it is the only one covering the omit-rather-than-null case the module's
      reply contract turns on. Both to the `tester`.

## Clean, and why

**The pins are the strongest part of this change.** Three independent routes —
`tmp/pin.rs` reading the generator's text files,
`the_pinned_name_is_derivable_by_hand_from_the_pinned_digest` doing the index
arithmetic in the test, and `display_name` — must agree, and mutations 2, 5 and 6
each broke that agreement loudly. The wordlist SHA-256 pin is produced by
`sha256sum` over the source text files rather than by hashing the arrays, which
is the correct shape; mutation 1 confirms it is the *only* thing that sees a
reorder.

**The collision fixture is real.** `COLLIDING_SEED_A`/`_B` derive
*plurative archilochos of kyrrhos* through the shipped scheme, and the tests
assert against that written-down string rather than merely `name_a == name_b`.
Both the `names.rs` and `feed.rs` collision tests failed under mutations 2 and 5,
so the fixture is live and not decorative.

**I agree the attestation requirement is rightly scenario-free.** No assertion
over the lists' own contents can distinguish a fabricated Greek word from a real
one; the evidence is outside the program. The spec says so explicitly and
`denylist.rs`'s header repeats it. Writing a scenario there would produce
something that reads as covered and fails on nothing, which is worse than the
gap.

`the_lists_carry_no_exclusion_of_any_kind` pinning eight words as present is the
right call: a negative asserted by example is weak in general, but the withdrawn
screens were *named* screens and these are their named casualties, so the
examples are the evidence rather than a sample.

**The spec's denylist arithmetic is sound and I did not open a box on it.** It
states the family's size as an order of magnitude and says explicitly that the
size "SHALL NOT be pinned". 199 pairs ship (`grep -c "^    ("` over
`denylist.rs`), which is consistent with "hundreds of pairs"; the refusal rate is
199/1,048,576 ≈ 0.019%, an order of magnitude *under* the spec's "on the order of
a tenth of a percent", and the one-redraw-is-enough argument only gets stronger
as the rate falls. If a ~960 figure appears in `design.md` or PLAN.md that is a
different document's problem and belongs to the design reviewer; the spec text
itself does not carry it.

**PLAN.md shed this change's behaviour correctly.** §5.2.1 on the branch replaces
the old exclusion list with a strikethrough plus "**Now a contract — see the
`generated-names` spec**", keeping the warning that no test can enforce
attestation and pointing at the spec for the rest. That is the right shape, and
it matters here because `origin/main`'s §5.2.1 says the opposite of what this
change ships — it excludes `stoa` from the noun list outright and flags `agora`
first-to-drop, where the spec now requires both present and a test pins them.

## What the gates cannot see

- `cargo fmt --check` at workspace level never reaches `dialectica-core`, so
  none of the ~10,500 new lines under `src/names/` is format-checked.
- `cfg(logos_scaffold)` code is not compiled by `cargo test`.
- The QML suite skips silently without `REQUIRE_QML_TESTS=1`. It ran here, but
  two of this spec's scenarios ("The mark and the abbreviation share no byte",
  "The abbreviation keeps a middle group") live only behind that gate.
- `cargo mutants` mutates functions, not constants. Mutations 3 and 4 — the two
  that survived — are both `const` values, and this change is unusually
  `const`-heavy.
- `the_derivation_reads_no_byte_past_its_bound` is weaker than its name: its
  fixture's first draw is permitted, so the reserve is never consulted and a
  derivation reading bytes 12..18 on the *redraw* path survives it. Mutation 6
  was caught by three other tests, not by this one. Not raised as a finding,
  since the property is pinned — but the name promises more than the body checks.
