# Correctness review — `generated-names`

Reviewed on `review/generated-names/correctness`, cut from `piece/generated-names`
at `24faad7`. **This reviewer covered CORRECTNESS only**; security, readability and
architecture are held by other instances and their rows remain unticked by me.

Baseline reproduced in this worktree: **918 lib + 28 end_to_end = 946, 0 failed.**
(The worktree needed `nix build github:logos-co/logos-module-builder/9f420c29…#rust-sdk-src
-o dialectica/logos-rust-sdk-src` first — the gitignored SDK symlink does not travel
with a new worktree and `cargo` fails at manifest resolution without it.)

## Findings

- [ ] **`dev-writer`** — `feed.rs:293` — the name-derivation failure path serves no
      name, but nothing stops a future edit serving a placeholder, and the comment
      at `feed.rs:292` cites a test that does not exist
      **Scenario:** replace the `let Ok(display_name) = … else { continue }` with
      `.unwrap_or(DisplayName { adjective: "unknown", noun: "unknown", place: "unknown" })`
      — the exact shape the spec forbids ("no placeholder name, no name for
      'unknown', and no name derived from truncated or padded input") — and the
      whole suite stays green. The branch is unguarded in both directions: neither
      "the row is dropped" nor "no placeholder is served" is asserted anywhere.
      **Measured:** 946 of 946 tests pass under this mutation (918 lib + 28
      end_to_end, 0 failed). `grep -rn
      "a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked"` over
      `dialectica/` and `openspec/` returns exactly one hit: the comment itself.
      Severity: **high** — a fabricated citation on the one branch the spec
      explicitly legislates, and the branch is measurably uncovered.

- [ ] **`dev-writer`** — `names.rs:358-362` — `is_refused`'s arithmetic argues from
      premises the shipped denylist disproves, by a factor of about five
      **Scenario:** the comment states "roughly 800 of the 1,024 nouns are named
      Greeks, each with about 1.2 canonically associated places, so about 960 pairs
      against `1024 * 1024` = 1,048,576 is about 0.092% of draws — roughly 4.6
      identities in every 5,000". The shipped list holds **199** pairs over **167**
      distinct nouns. The real rate is 199/1,048,576 = **0.019%**, about **0.95**
      identities in every 5,000 — not 4.6. The "1.2 places per figure" half is
      sound (199/167 = 1.19); the "800 named Greeks → 960 pairs" half is invented.
      **Measured:** `grep -c "^    (" names/denylist.rs` = 199;
      `grep -oP '^\s+\(\d+(?=,)' … | sort -u | grep -c .` = 167;
      `grep -c . tmp/attributions.txt` = 199, so the generator is faithful and the
      comment, not the list, is what is wrong. Severity: **medium** — a wrong
      premise in the doc that justifies the whole family's existence.

- [ ] **`dev-writer`** — `names.rs:196-197` — `NameError::ReserveExhausted`'s stated
      arrival rate follows from the fabricated figure above and is off by ~23x
      **Scenario:** the doc says "Arrives about once in 1.2 million identities: a
      first draw is refused about once in 1,090, and this needs two consecutive
      refusals." 1/1,090 = 0.092%, which is the `is_refused` figure disproved above.
      With the shipped 199 pairs a first draw is refused once in **5,269**, so two
      consecutive refusals arrive once in **27.7 million**, not 1.2 million. The
      spec's own reasoning ("one redraw is sufficient … a second consecutive
      refusal … arriving about once per million identities") is likewise stated
      against the wrong rate — the conclusion survives comfortably, but the number
      a reader would check does not. Severity: **medium** — the number is the
      justification for the reserve being one draw rather than two.

- [ ] **`spec-writer`** — `wire.rs:1724-1740` — the thread reply reports an author
      and carries no display name, and a test asserts the name's *absence*
      **Scenario:** the spec requires "Every reply in which core reports who
      authored something SHALL carry that author's display name alongside the
      author's address". `thread_page_json` emits `author` and `authorKey` for every
      item and no `displayName`, and `the_wire_reports_the_author_as_an_address_and_a_key_and_no_name`
      (`wire.rs:6076`) asserts `root.get("displayName").is_none()` — so the suite
      actively pins the non-conformance. The test's own comment argues the name is
      derivable by the view "because the item carries the key", which is true for
      *this* reply and is a different reasoning from the feed's; whether the spec
      intends the obligation to be discharged by `authorKey` rather than by a name
      is a question for the spec, not a defect I can settle. Either the spec should
      say a reply carrying the *key* discharges it, or this reply needs the field.
      Severity: **medium** — a requirement and a test that contradict each other,
      whichever way it resolves.

- [ ] **`dev-writer`** — `names.rs:122,299` — `NAME_DIGEST_BOUND` constrains nothing
      in a release build; the real bound is the two literal offsets
      **Scenario:** the constant's doc says "Beyond this the derivation fails rather
      than reading on", but the only non-test reference is a `debug_assert_eq!`
      which compiles out under `--release`. The actual bound is the literals `0` and
      `6` passed to `draw_at` plus the `[u8; 32]` parameter type. Change `draw_at(digest, 6)`
      to `draw_at(digest, 14)` and the derivation silently reads bytes 14..20 —
      past the documented bound, with `NAME_DIGEST_BOUND` still reading 12 and the
      `debug_assert` still passing. **Measured:** three tests do catch that
      particular mutation (`the_redraw_path_is_pinned_to_a_written_down_name`,
      `a_refused_pair_redraws_every_slot_from_the_reserve`,
      `exhausting_the_reserve_fails_rather_than_reading_on`), so the scheme is not
      currently broken — but the constant a reader would trust to enforce the bound
      is decorative, and `the_derivation_reads_no_byte_past_its_bound`, the test
      named for the property, passes unchanged under it. Severity: **low** — a
      documented mechanism that does not exist; the pins are what actually hold.

- [ ] **`tester`** — `names.rs:833-860` —
      `every_index_of_every_list_is_reachable_and_uniformly_so` tests the modulo
      arithmetic, not `draw_at`, so it cannot see the derivation stop being uniform
      **Scenario:** the test computes `draw as u16 % len as u16` in its own body and
      never calls `draw_at`, `name_from_digest` or `display_name`. Replace
      `draw_at`'s adjective reduction with `(word(0) % (ADJECTIVES.len() as u16 - 1)) + 1`
      — index 0 unreachable, and 8,191 does not divide 65,536, so the reduction is
      biased, which is exactly what the spec's two scenarios forbid — and this test
      still passes. **Measured:** 30 of 34 `names::` tests pass under that mutation;
      the four that fail are all name pins, none of them this one. The property is
      covered in practice by the pins; the test named for it is not what covers it.
      Severity: **low** — a test whose name overstates what it checks.

## What was clean

The parts this change most rests on hold up under direct attack.

**The wordlist pins are the strongest thing here and they verify independently.**
`sha256sum` over `tmp/adj-final-sorted.txt`, `tmp/nouns-normalized.txt` and
`tmp/places-final-sorted.txt` reproduces `PINNED_ADJECTIVES_SHA256`,
`PINNED_NOUNS_SHA256` and `PINNED_PLACES_SHA256` byte for byte. Re-running those
three commands is a real, cheap check on a reorder — which is the consensus change
the name pins alone cannot see, as the test's own comment says and as I confirmed.

**The list sizes are what the arithmetic assumes.** Counted by command rather than
from a comment: `grep -c '^    "'` gives 8192 adjectives, 1024 nouns, 1024 places.
So 65,536/8,192 = 8 and 65,536/1,024 = 64 both hold, and the exact-uniformity claim
is sound as shipped.

**The denylist is sorted, deduplicated and in range**, verified by extracting all
199 pairs and reading them rather than trusting
`the_denylist_is_sorted_deduplicated_and_in_range`. Maximum noun index 1015 < 1024,
maximum place index 997 < 1024, strictly ascending throughout, so `binary_search`'s
precondition genuinely holds. I also spot-checked eleven pair comments against the
two lists by index arithmetic (index N = file line 27+N for nouns, 23+N for places):
`(11,436) agatharchides of knidos`, `(26,437) ainesidemos of knossos`,
`(49,847) alkman of sardis`, `(69,505) anaximenes of lampsakos`,
`(206,2) demokritos of abdera`, `(221,17) diodoros of agyrion`,
`(253,38) empedokles of akragas`, `(299,48) eukleides of alexandria`,
`(866,67) semonides of amorgos`, `(910,505) straton of lampsakos` and
`(1015,229) zenon of elea` all name the entries they claim. The generation from
`tmp/attributions.txt` is 1:1 (199 lines → 199 pairs).

**Endianness is well pinned.** Flipping `u16::from_be_bytes` to `from_le_bytes`
fails five tests including all three name pins and the collision fixture — the
hand-computed vector in `the_pinned_name_is_derivable_by_hand_from_the_pinned_digest`
does its job.

**No reachable panic in the production paths.** Every `unwrap`/`expect` in
`names.rs` is inside `#[cfg(test)]`. `draw_at` indexes `digest[offset + i]` with
`offset` only ever 0 or 6 against a `[u8; 32]` parameter, and all three list indexes
are bounded by the `%`, so neither the slice nor the array access can go out of
range for any input. `display_name_from_bytes` parses before deriving, and the
malformed-input sweep (lengths 0..70, plus a non-point and the low-order point)
returns rather than aborting. The feed's `min`/`saturating_*` pagination cannot
panic on a page past the end.

**The redraw path is genuinely covered.** Moving the reserve offset off 6 fails
three tests, and the whole-name-redraw property (not merely "the refused pair is not
returned") is asserted against a digest constructed so a single-slot substitution
would fail.

## What a gate cannot see here

- `cargo fmt --check` at workspace level does not follow the path dependency into
  `dialectica-core`, so none of the files this change adds is format-checked by that
  gate.
- Anything behind `cfg(logos_scaffold)` is not compiled by `cargo test`.
- The denylist's **completeness** is not checkable by any test — `denylist.rs`'s own
  header says so, and it is right: a pair missing from the list is a claim about the
  world. I did not attempt to audit all 199 against sources. Worth a human pass:
  `platon` (noun 772) and `sokrates` (noun 879) are on no pair at all, which may be
  correct (neither is conventionally cited as "of Athens") but is the kind of gap
  only a person can rule on.
- `cargo mutants` was not run: the crate's test suite takes ~18s per run and the
  file set here is large enough that a full scoped run exceeds the couple of minutes
  the brief allows. The targeted mutations above were done by hand instead, and each
  is reported with its measured result.
