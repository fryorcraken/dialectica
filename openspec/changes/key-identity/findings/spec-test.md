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

- [x] **`spec-writer`** — delta spec, *A name is not a credential* /
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

      **Fixed by the spec saying, and saying why** — the first of your two
      options. The behaviour does not change; what changes is that it is now
      required rather than merely unopposed.

      The distinction the spec now draws is that **the two rules apply to
      different things**, which is why this is an exemption with a reason rather
      than a carve-out. *Malformed key material is refused rather than crashed on*
      gives its own reason for forbidding a padded derivation: such a name "would
      render as an ordinary participant, which is a name attributable to nobody
      presented as one attributable to somebody". That hazard exists **only where
      a name is rendered**. So the requirement is scoped by its own stated
      rationale, not narrowed to suit the code.

      The measurement apparatus is then obliged the *opposite* way, and for a
      reason internal to this change rather than for convenience: a measurement
      that returned a non-value for malformed input would report that channel as
      reading no byte, and "disjoint" is satisfied by a measurement that found
      nothing — which is the failure the non-emptiness scenario already exists to
      forbid. Refusing would make the gate weaker, not safer.

      Added to *The three channels read pairwise disjoint bytes of the public
      key* (which is where the apparatus is described):
      - normative text that whatever makes the name's window reachable **SHALL
        NOT render a name and SHALL have no consumer other than the
        measurement**, then the scoping argument above, closing on the rule in
        one line: *"anything that renders a name refuses malformed input; the
        thing that only measures accepts it"*;
      - the explicit transition, which is the part that protects your scenario:
        **a change that gives the apparatus a rendering consumer moves it under
        the first rule and SHALL make it refuse**;
      - two scenarios, *The measurement apparatus renders no name* and *The
        measurement apparatus accepts malformed input*.

      **The test that fails without it**, and this is the one that answers your
      scenario directly rather than the current behaviour:
      `test_the_name_window_exposes_no_name_only_draws` asserts the component's
      surface is three numeric draws and that eleven rendering-shaped members
      (`render`, `name`, `text`, `words`, `adjective`, the three wordlists, …)
      are all `undefined`. Measured: adding a one-line
      `function render() { return "adjective noun of place"; }` to
      `DKeyNameWindow.qml` fails it, naming `render`; everything else in the file
      stays green. Reverted; 19/19.

      That test is deliberate about what it cannot do, and says so in its comment
      rather than letting the name imply coverage: **nothing inside
      `qmltestrunner` can see the rest of the tree**, so the "no consumer other
      than the measurement" half is held by the spec requirement and the
      component's header, not by this test. Measured separately and recorded in
      the comment: `grep -rl DKeyNameWindow dialectica-ui/` returns `qmldir` and
      `tst_identicon.qml`, nothing else.

      `test_a_malformed_key_still_yields_indices_in_range` keeps its behaviour and
      gains the reasoning in its comment, so a reader meeting the two languages
      disagreeing finds the answer at the test that pins the disagreement. The
      `DKeyNameWindow.qml` comment carries the same, opening on **"this diverges
      from core deliberately, and the spec says which way"** — it was drafted as a
      `// NO SPEC:` marker (which is what the design reviewer asked for) and
      became a spec citation once the spec answered; leaving a `NO SPEC:` on
      behaviour a requirement now mandates would have been the more misleading of
      the two.

- [x] **`spec-writer`** — delta spec, *The three channels read pairwise disjoint
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

      **Fixed for the scenario you name; the other two are a partial rejection,
      argued below.**

      **"An overlap is detected rather than passed over" is deleted**, and you are
      right about why: its THEN was *"the disjointness check fails and names the
      overlapping byte"*, a statement about what a **check** does under a
      mutation, and a suite cannot assert that one of its own tests fails. Worse,
      as you say, it read as covered while the class of overlap it described had
      already escaped — which the first box in this file measures.

      Replacing it is *A channel reading an unallocated byte is detected at every
      value*, whose THEN is a statement about **the system**: "the measured byte
      set for that channel includes the unallocated byte", explicitly "including
      when it depends on that byte only for a single one of the byte's 256
      values". That is testable, and it is the obligation the old scenario was
      reaching for — the measurement must be *complete*, which is a property of
      the measurement rather than of the suite's reaction to a mutation. It is
      what makes the first box's fix a requirement rather than a good idea.

      Also added, from the architecture reviewer's box: *All three channels are
      measurable in one place*, which likewise states a property of the system
      (all three sets obtained in one place, the name's window reachable there)
      rather than of a check, and is pinned by
      `test_all_three_channels_are_reachable_from_this_file`.

      **On "A shifted byte window is visible" and "Changing which bytes a slot
      reads renames every identity": I disagree that they share the shape, and I
      checked rather than assumed.** Their THENs are *"at least one pinned case no
      longer matches its written-down name"* and *"the names differ from those the
      same keys had before"* — both statements about **derived output**, not about
      what a check does. Both are testable without asserting a test fails, and
      both have tests:
      - `removing_a_word_renames_identities_that_drew_past_it` (`names.rs:2038`)
        builds the shortened list as a `Vec` and does the reindexing by hand, so
        it *exhibits* the renaming rather than asserting a removal happened. Its
        own comment draws exactly this distinction.
      - The shifted-window scenario is held by its two preconditions, which is the
        subtler case: `the_pinned_cases_differ_outside_the_name_window` checks the
        pinned keys differ at bytes 16, 17, 24 and 25 — the values a one-byte slip
        in either direction would actually read — and
        `some_pinned_case_is_byte_order_sensitive_in_every_slot` closes the
        equal-bytes hole. Together those make "a slip is visible in at least one
        pin" a property of the fixtures, established by construction rather than
        by running a mutation. Your own *Mutations run* table confirms the
        mechanism: the `SPEC_MARK_BYTES 4..12 → 6..14` and little-endian
        mutations were each caught by a named test.

      So I have not reshaped those two. If you think the preconditions do not
      amount to the scenario's THEN, that is worth another look — but it is a
      different argument from the one about the deleted scenario, and conflating
      them would have cost two real tests their spec anchor.

- [x] **`spec-writer`** — delta spec, *The scheme and its wordlists are frozen*,
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

      **Fixed by dropping the clause**, your first option. The WHEN is now simply
      *"a name is derived for one public key"*, and both THENs are unchanged
      because, as you established, neither ever depended on the clause. The two
      tests you name go on passing without modification, which is the evidence
      the clause was inert rather than load-bearing.

      A note is attached saying the omission is deliberate and why — that the
      absence of a version input and the absence of a scheme marker are **standing
      properties of the derivation, not consequences of a change**, so varying a
      wordlist made the WHEN inert and invited a reader to take this scenario as
      covering what a wordlist change does, which is the next scenario's job. That
      also answers your forward-looking scenario: a later change adding a marker
      that appears only when two wordlists are in play would be read against
      *Removing a word renames identities that drew past it*, which does vary a
      list, rather than against this one.

      **The scenario keeps its title**, and that was not a style choice. Renaming
      it to match the new WHEN made `openspec validate --strict` **fail**:
      *"MODIFIED … omits scenario(s) the current spec still has … a MODIFIED
      requirement replaces the whole block, so archive refuses to drop them"*. A
      rename inside a MODIFIED block is a silent scenario drop as far as archive
      is concerned. Worth recording alongside your box, because the natural fix to
      the finding is the one that loses the requirement at archive time — the
      `docs/OPENSPEC-ARCHIVE.md` trap, met live. `openspec validate key-identity
      --strict` passes with the title preserved.

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
