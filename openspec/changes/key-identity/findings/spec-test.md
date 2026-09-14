# spec-test review — `key-identity`

Read: the delta spec, the live capability, `names.rs`'s `mod tests`, and
`tst_identicon.qml`. The implementation bodies were read only where a mutation
required editing them.

Both mutations the `tester` stage reported as closed were re-run and are now
genuinely caught — see *Mutations run* at the foot. A third of the same family
survives, in both languages at once, and is the first box below.

- [x] **`tester`** — `names.rs:842` (`PROBES`) and `tst_identicon.qml:225`
      (`probeValues`) — **the byte-allocation gate is blind to a byte the
      derivation reads only for values outside a fixed 7-value probe set.**
      Every "which bytes does this channel read" assertion in both languages
      routes through one hardcoded probe list. A byte that participates
      conditionally is reported as unread, and `the_name_reads_no_unallocated_byte`
      then certifies the allocation clean. This is the repo's named defect
      family — the measurement and the test agree because they share one blind
      spot — sitting under the property the whole piece exists to establish.
      **Scenario:** the derivation additionally reads unallocated key byte 12,
      but only when that byte equals `0x42`. The name is then a function of seven
      key bytes, not six; grinding for a lookalike name partly grinds against a
      byte the allocation says no channel touches, and the disjointness argument
      that makes the three costs multiply no longer holds.
      **Measured:** with
      `let adjective_index = (word(0) ^ if key_bytes[12] == 0x42 { 1 } else { 0 }) % ADJECTIVES.len() as u16;`
      **all 70 `dialectica-core` lib tests passed**, `the_name_reads_no_unallocated_byte`
      and `the_name_reads_exactly_the_bytes_the_spec_allocates_to_it` among them.
      The identical mutation in `DKeyNameWindow.adjectiveIndex()`
      (`(_draw(18) ^ (_byte(12) === 0x42 ? 1 : 0)) % 8192`) left **all 16
      `Identicon` QML tests passing and the whole QML run exiting 0**.
      Appending `0x42` to `PROBES` makes both Rust tests fail immediately
      (measured: `left: [12, 18, 19, 20, 21, 22, 23]`), which confirms the gap is
      the probe set and nothing else. **Severity: high** — it is the one gate
      standing behind the piece's central security property, and a fixed literal
      list is precisely the shape `hand-maintained sweep lists go stale silently`
      warns about. A fix that merely adds `0x42` reproduces the defect one value
      out; the probe needs to be exhaustive over the 256 values (32 bytes x 256 is
      8,192 derivations, trivial in both languages) or driven from a seeded sweep
      wide enough that a single-value carve-out cannot hide.

      **Fixed.** Both probe sets are now the byte's **entire domain**, `0..=255`,
      and the value list is gone from both languages: `PROBES` is deleted and the
      Rust loop is `for probe in 0..=u8::MAX`; `probeValues` is replaced by
      `probeCount: 256` with a `_probeHex(v)` helper, used by all four probe sites
      in `tst_identicon.qml`.

      **Not a longer list, deliberately** — your warning that appending `0x42`
      reproduces the defect one value out is the reason. A probe enumerating
      values is always one value short of something; the whole domain is the one
      list that cannot be extended and cannot go stale. The seeded-sweep
      alternative was considered and rejected: to be confident of catching a
      one-value carve-out it needs samples on the order of the domain anyway, and
      it buys a seed to record and a flake mode in exchange for trading a
      certainty for a probability. Recorded in `design.md` under *The byte probes
      try every value of the byte, not a list of good ones*, with the residue the
      sweep still cannot see stated explicitly (a read gated on two bytes at once)
      and why that is answered structurally rather than by a wider sweep.

      **The tests that fail without it**, each run in both directions:
      - your exact mutation, `^ if key_bytes[12] == 0x42 {1} else {0}` on the
        adjective reduction — was 38/38 green under `PROBES`, now fails
        `the_name_reads_exactly_the_bytes_the_spec_allocates_to_it`
        (`left: [12, 18, 19, 20, 21, 22, 23]`) and
        `the_name_reads_no_unallocated_byte` ("key byte 12 is unallocated but the
        name reads it").
      - a *different* byte and value, `key_bytes[27] == 0xa7`, chosen because no
        hand-picked list would contain either — also caught, naming byte 27. This
        is the measurement that distinguishes the fix from appending `0x42`.
      - the QML twin, `(_draw(18) ^ (_byte(12) === 0x42 ? 1 : 0)) % 8192` — was
        17/17 green, now fails `test_no_channel_reads_an_unallocated_byte` ("key
        byte 12 is unallocated but the generated name reads it") and
        `test_the_byte_probes_find_the_windows_they_should`
        (`Actual: 12,18,19,20,21,22,23`).

      Both mutations reverted; baselines restored green (Rust `names::` 38/38 in
      0.06s, `tst_identicon.qml` 17/17). **Cost measured:** Rust unchanged at
      0.06s; the QML spec goes 68ms → ~1.1s.

      **One further defect found while fixing this**, reported rather than
      quietly folded in: `test_a_byte_one_channel_reads_moves_only_that_channel`
      looped `for (... && !moved; ...)`, stopping at the first value that moved
      the target channel — which made its own comment ("the other two must be
      untouched for EVERY probe value, not merely for the one that moved the
      target") false for every value after the first, so a cross-channel leak
      gated on a later value passed unseen. The loop now runs to the end of the
      domain and the comment says why it must.

- [ ] **`spec-writer`** — delta spec, *A name is not a credential* /
      live spec, *Malformed key material is refused rather than crashed on* —
      **the two channels contradict each other on malformed key material, and a
      test pins the contradiction.** The live requirement, which the delta does
      not touch and which therefore still governs, says there SHALL be "no name
      derived from truncated or padded input". `DKeyNameWindow._body` zero-pads
      any short or malformed key to 64 hex characters and returns indices for it,
      and `test_a_malformed_key_still_yields_indices_in_range`
      (`tst_identicon.qml:529`) asserts exactly that as required behaviour, using
      `""`, `"k:"`, `"k:0"` and `"not-a-key"` as its cases. Core refuses the same
      inputs. No requirement says which of the two is right for a component that
      computes indices rather than a name, and no `NO SPEC:` marker records that
      anyone chose.
      **Scenario:** `DKeyNameWindow` is reached by a view — nothing prevents it;
      it is exported from `qmldir` and the only thing keeping it test-only today
      is that no file instantiates it. A peer-supplied malformed key then yields
      three in-range indices, which is a name attributable to nobody rendered as
      one attributable to somebody — the failure the live requirement names in as
      many words.
      **Measured:** the behaviour is pinned green; nothing in either suite
      compares the QML padding path against core's refusal. **Severity: medium** —
      the spec must say whether the index-only component is exempt from the
      no-padding rule, and say why, or the component must refuse.

- [ ] **`spec-writer`** — delta spec, *The three channels read pairwise disjoint
      bytes*, scenario "An overlap is detected rather than passed over" —
      **this scenario asserts a property of the test suite, not of the system,
      and no test can hold it.** Its THEN is "the disjointness check fails and
      names the overlapping byte", which is a statement about what a check does
      under a mutation. A suite cannot assert that one of its own tests fails;
      the only way to establish it is to run the mutation by hand, which leaves
      no artefact and decays the moment the probe changes — as the box above
      demonstrates it already has for a whole class of overlap. The same shape
      recurs in "A shifted byte window is visible" and "Changing which bytes a
      slot reads renames every identity", neither of which has a test either.
      **Scenario:** the probe set narrows or a channel gains a conditional read;
      the scenario still reads as covered because the suite is green, and nobody
      re-runs the mutation it actually describes.
      **Measured:** the overlap scenario does hold today for an *unconditional*
      overlap — `middleChars: 16` in `DTheme.qml` puts the abbreviation on name
      bytes 18 and 19, and five QML tests fail naming byte 18 exactly. It does
      not hold for a conditional one. **Severity: medium** — either state these
      as obligations on the *change process* (a mutation to run at review time,
      recorded where the next reviewer finds it) rather than as scenarios, or
      give the suite a self-test in the shape of
      `dialectica-ui/tests/tst_check_qml_names.py`, which pins a gate in both
      directions and is the precedent this repo already has for exactly this.

- [ ] **`spec-writer`** — delta spec, *The scheme and its wordlists are frozen*,
      scenario "A different scheme version gives a different name for one key" —
      **the scenario's WHEN is inert: nothing in either THEN depends on it.**
      The WHEN changes a wordlist entry and re-derives; the two THENs assert that
      the derivation takes no version input and that names carry no scheme marker.
      Both are true without changing any wordlist, and both tests
      (`a_names_whole_input_is_six_key_bytes_with_no_version_alongside_them`,
      `a_name_carries_no_marker_of_which_scheme_produced_it`) correctly ignore the
      WHEN entirely. A reader checking coverage sees a scenario about wordlist
      changes and two tests about neither.
      **Scenario:** a later change adds a scheme marker that appears *only* when
      two wordlists are in play. The scenario reads as covering it; no test does.
      **Measured:** both tests pass on an unmodified tree with no wordlist
      touched, which is the whole of what the WHEN asks to be varied.
      **Severity: low** — drop the WHEN's wordlist clause, or move the scenario's
      real content into the requirement prose, where the spec already says
      plainly that this is a one-way door rather than a property under test.

## What was clean

**The cross-language pin is real and does compare the two implementations**,
which was the question asked. It does so through a third party rather than by
comparing the two directly: `dialectica-core/examples/pin_name.rs` reads the
wordlists from the text files and links neither implementation, and both sides
pin against its output — `names.rs`'s `PINNED_CASES` as rendered names, the
QML `pinnedCases` as the three draw indices. I verified by hand that the two
forms agree for both keys: key 1's bytes `18..23` are `76 ae be be 7b 92`, giving
`0x76ae % 8192 = 5806`, `0xbebe % 1024 = 702`, `0x7b92 % 1024 = 914`, and lines
5807, 703 and 915 of the three wordlist files are `quartzous`, `paris` and
`sypalettos` — the pinned name. So the two written-down tables are not merely
each self-consistent. What is *not* compared is the byte-allocation table itself:
`names.rs`'s `SPEC_MARK_BYTES` / `SPEC_ABBREVIATION_BYTES`, the QML meta-test's
literals, and `docs/IDENTICON.md`'s table are three independent transcriptions,
each pinned to the spec's figures rather than to each other. All three agree
today, and each would fail alone on a drift, so this is adequate rather than a
finding.

**The name pins are independent of the implementation** in the way the spec
demands, and the byte-order precondition the `tester` added is load-bearing
rather than decorative — measured below.

**Requirement migration is intact.** Both `REMOVED` requirements' obligations
appear in the `ADDED` one: pairwise disjointness subsumes "the mark SHALL read
only bytes the abbreviation does not show", the middle-group obligation is
carried verbatim as both prose and a scenario, and the security argument
transfers unchanged. The one sentence dropped without replacement — the mark's
"effective contribution is only the bytes it reads that the abbreviation hides" —
is explanatory rather than an obligation. Every moved requirement still has a
test.

**The delta is self-consistent.** The allocation table's counts add correctly
(4 + 8 + 4 + 6 + 3 = 25) and its complement is the seven bytes named unallocated
(12, 13, 24–28), totalling 32. `openspec validate key-identity --strict` passes,
and reading the whole file surfaced no requirement contradicting another.

**`docs/PLAN.md` on `origin/main` carries nothing stale** for this change: it
holds no copy of the byte allocation, no separator or scheme-version claim, and
the two places where it asserted "the address is the identity" were given the
strikethrough-plus-authority shape rather than being silently rewritten.

**No `NO SPEC:` markers appear in `names.rs` or `tst_identicon.qml`.** The one
unmarked chosen behaviour I found is the QML padding path, which is the second
box above.

## Mutations run

Each was applied, measured, and reverted; `git status` was clean of tracked
modifications before committing.

| mutation | file | result |
|---|---|---|
| noun slot reduced little-endian | `DKeyNameWindow.qml` | **caught** — `test_the_name_window_agrees_with_cores_pinned_case` fails on the second pinned case only (`inpardonable paidagogos of myriandros: noun index`). The first case stays green, confirming the `tester`'s diagnosis that one case was insufficient. |
| `SPEC_MARK_BYTES` moved `4..12` → `6..14` | `names.rs` | **caught** — `the_restated_channels_are_the_spec_s_byte_sets_and_not_merely_disjoint_ones` fails (`left: [6,7,8,9,10,11,12,13]`). |
| whole derivation little-endian | `names.rs` | **caught** — 5 tests fail, including the seed-7 pin. |
| noun and place slots swapped | `DKeyNameWindow.qml` | **caught** — the cross-language pin fails on the first case's noun index. Byte sets are unchanged, so every disjointness probe stays green; only the pin sees it. |
| `middleChars: 8 → 16` | `DTheme.qml` | **caught** — 5 QML tests fail, naming key byte 18 as read by both the abbreviation and the name. |
| conditional read of unallocated byte 12 | `names.rs` | **SURVIVED** — 70/70 pass. First box above. |
| conditional read of unallocated byte 12 | `DKeyNameWindow.qml` | **SURVIVED** — 16/16 `Identicon` tests pass, whole QML run exits 0. First box above. |
