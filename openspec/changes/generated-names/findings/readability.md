# Readability findings — `generated-names`

Reviewed dimension: **readability only**. Correctness, security and architecture
are held by other instances; nothing below is a claim about any of them.

Method: every quantity, citation and worked example in the new comments was
checked against the shipped data with `grep -c` / `grep -n` and hand arithmetic,
in the worktree `review-gn-readability` at `24faad7`. `cargo test` could not be
run here — `dialectica/logos-rust-sdk-src` is a gitignored symlink absent from a
fresh worktree and creating it was denied — so no finding below rests on a test
run. Every one rests on a file read in this tree.

---

## Comments that argue from numbers the shipped data disproves

- [ ] **`dev-writer`** — `names.rs:358-362` — `is_refused`'s doc comment states
      the denylist arithmetic that `PLAN.md` struck through and `proposal.md`
      explicitly withdrew, and every figure in it is wrong for the list as
      shipped.
      **Scenario:** the comment reads *"roughly 800 of the 1,024 nouns are named
      Greeks, each with about 1.2 canonically associated places, so about 960
      pairs against `1024 * 1024` = 1,048,576 is about 0.092% of draws — roughly
      4.6 identities in every 5,000."* `docs/PLAN.md:1598-1600` carries that exact
      sentence inside `~~strikethrough~~` and follows it with **"The premise is
      withdrawn."**; `proposal.md:169-176` says *"The denylist figures are not
      re-derived here, because their premise is gone. They rested on 'roughly 800
      of the 1,024 nouns are named Greeks'"*. A reader of the code meets the
      withdrawn premise asserted as current fact, with no sign it was retracted.
      **Measured:** the denylist holds **199** pairs, not 960 —
      `grep -c "^    ([0-9]" src/names/denylist.rs` = 199, and `tasks.md:7` records
      the same number. 199 / 1,048,576 = **0.019%**, not 0.092%; that is
      **0.95 identities per 5,000**, not 4.6. Every figure in the sentence is
      wrong by a factor of ~4.8. **Severity: high** — this is the single most
      persuasive paragraph in the module and it is the one that was retracted
      upstream.

- [ ] **`dev-writer`** — `names.rs:196-197` — `NameError::ReserveExhausted`'s doc
      comment quotes two derived rates that inherit the withdrawn 960 premise.
      **Scenario:** it reads *"Arrives about once in 1.2 million identities: a
      first draw is refused about once in 1,090."* 1,048,576 / 960 ≈ 1,092, so
      the "1,090" is the withdrawn 960 restated as an odds figure, and
      1,090² ≈ 1.19 million is the "1.2 million". Both numbers are that premise
      wearing a different notation.
      **Measured:** with 199 pairs the first-draw refusal rate is
      199 / 1,048,576 ≈ **1 in 5,269**, not 1 in 1,090; two consecutive refusals
      is (199/1,048,576)² ≈ 3.60e-8 ≈ **1 in 27.8 million**, not 1 in 1.2 million.
      The comment understates the reserve's headroom by ~23x.
      **Note the contract is better than the code here**: `spec.md:366-370`
      deliberately states this as an order of magnitude and says it *"holds across
      any denylist of that order rather than depending on its exact size"* — the
      code replaced a claim that stays true with two that went stale on the first
      curation pass. **Severity: high.**

- [ ] **`dev-writer`** — `feed.rs:281-282` — the same withdrawn "1.2 million"
      figure is repeated in `list_threads`, so correcting `names.rs` alone leaves
      a second copy.
      **Scenario:** *"A derivation failure here is the denylist reserve being
      exhausted, at roughly one key in 1.2 million."* Same derivation, same error,
      and it is the number a reader uses to judge whether the silent `continue`
      below it is acceptable. **Measured:** ~1 in 27.8 million, as above.
      **Severity: medium** — flagged separately because it is a second file and a
      fix to `names.rs` will not touch it.

## A citation to a test that does not exist

- [ ] **`dev-writer`** — `feed.rs:292` — the `NO SPEC:` block's closing "See"
      names a test that exists nowhere in the repository.
      **Scenario:** the comment ends *"See
      `a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked`."* A
      reader wanting to know whether the silent `continue` at `feed.rs:293-295` is
      exercised follows the pointer and finds nothing, and the pointer is the only
      evidence offered that the dropped-row behaviour was tested at all.
      **Measured:** `grep -rn` for that identifier across `dialectica/` and
      `openspec/` returns exactly one hit — the comment itself. The `tester`
      already recorded this as an open gap in `tasks.md:19`; it is repeated here
      because a comment asserting a test exists is a readability defect
      independent of the coverage gap, and the fix is to the comment either way.
      **Severity: high.** The rest of the marker reads true: `spec.md:775-778`
      does forbid a placeholder and says nothing about a feed's handling of an
      underivable row, so the `NO SPEC:` classification itself is correct.

## The running example cannot be produced by the code it illustrates

- [ ] **`dev-writer`** — `names.rs:9`, `names.rs:153`, `names.rs:725`,
      `feed.rs:133` — the canonical example name *measured aporia of lampsacus*
      is unreachable: neither `measured` nor `lampsacus` is in the shipped lists.
      **Scenario:** a reader learning the scheme from `names.rs`'s module header
      takes *measured aporia of lampsacus* as a specimen output, then greps for
      `measured` in `ADJECTIVES` to see the slot it came from and finds nothing.
      Worse at `names.rs:725`, where the comment computes token counts off the
      example (*"`rendered.split(' ').count()` is 4"*) as though it were a real
      draw.
      **Measured:** `grep -n "\"meas" src/names/adjectives.rs` yields only
      `measurable` and `measureless`; `grep` for `lampsacus` in `places.rs` yields
      nothing — the list holds `lampsakos` (line 528). `aporia` is real
      (`nouns.rs:129`). So two of the three words are fabrications.
      **Severity: medium** — the scheme is explained correctly, but every worked
      example in the module is a name the module cannot emit, which is exactly the
      shape of claim a reader cannot check without doing what I just did.

- [ ] **`dev-writer`** — `names.rs:344-347` and `names.rs:353` — the
      true-attribution argument is worked entirely in a transliteration the lists
      do not use, so the one example that would let a reader verify the denylist
      does not match any entry in it.
      **Scenario:** the comment says *"straton of abdera and measured aporia of
      lampsacus both draw normally while straton of lampsacus does not"*, and
      *"straton of lampsacus is how Straton of Lampsacus is actually referred
      to"*. A reader checks the denylist for the refused pair and finds
      `(910, 505), // straton of lampsakos` — a different spelling — so cannot
      tell whether the comment describes this denylist or an earlier one.
      **Measured:** `PLACES[505]` is `lampsakos`; `lampsacus` is absent from
      `places.rs`. `denylist.rs:11` gets the same sentence right
      (*"straton of lampsakos is how Straton of Lampsacus is actually referred
      to"*, keeping the Latinised form only for the English prose), so the two
      files disagree about the same example. **Severity: medium.**

- [ ] **`dev-writer`** — `names/adjectives.rs:9` — the module header names three
      adjectives as examples of the open slot and two of them are not in the file.
      **Scenario:** *"`brittle`, `luminous` and `damp` draw alongside
      `measured`."* A reader sampling the list to check the "no register screen"
      claim looks up `brittle` and `damp`, finds neither, and is left unsure
      whether the screen was actually dropped.
      **Measured:** `grep -n "\"brittle\|\"damp" src/names/adjectives.rs` returns
      nothing; `luminous` is at line 4338; `measured` is absent. Three of the four
      cited words are not entries. **Severity: medium** — the claim being
      illustrated (no register screen) is true, which is what makes the false
      examples costly: they are the only evidence offered for a true claim.

- [ ] **`dev-writer`** — `names/places.rs:16` — the "an entry may hold an internal
      space" claim names three examples, of which only one is in the list.
      **Scenario:** *"An entry may hold an internal space — `alexandria troas`,
      `herakleia pontike`, `lokroi epizephyrioi`."* A reader checking that the
      withdrawn single-word screen really is gone greps the first two and finds
      neither.
      **Measured:** `lokroi epizephyrioi` is at `places.rs:567`; `alexandria`
      appears bare at line 71 with no `troas`; `herakleia pontike` is absent
      entirely. The list does hold 10 multi-word entries
      (`grep -c "^    \"[a-z]* [a-z]"` = 10), so the claim is true and only its
      examples are wrong. **Severity: medium.** Note the neighbouring
      cut-entry examples at `places.rs:9-14` all check out (`gortyn` and `gortys`
      kept, `gortynia`/`piraeus`/`lelantine` cut), which is the standard the space
      example should meet.

- [ ] **`dev-writer`** — `names.rs:722-726` and `names.rs:1168-1172` — two test
      comments illustrate multi-word-place behaviour with `alexandria troas` and
      `heraclea pontica`, neither of which is in `PLACES`.
      **Scenario:** `a_name_is_three_drawn_words_and_a_fixed_connector` explains
      why it counts by slot rather than by token using *"5 for `measured aporia of
      alexandria troas`"*; `every_entry_of_every_list_is_ascii_lowercase_and_well_formed`
      says the withdrawn single-word screen *"discards `alexandria troas` and
      `heraclea pontica`"*. Both are the right argument attached to entries that
      are not there, and the second is the load-bearing justification for why an
      internal space is permitted. **Measured:** both absent from `places.rs`; the
      real multi-word entries are listed above. **Severity: low** — same defect
      family as the two above, split out because it is in test comments a fixer
      would otherwise not open.

- [x] **`spec-writer`** — `docs/UI-BRIEF.md:124` — the brief hands a designer two
      specimen names the core cannot produce.
      **Scenario:** it offers *measured aporia of lampsacus*, *brittle kairos of
      abdera* and *luminous stasis of delos* as what an identity looks like. The
      brief is explicitly designed against by someone who cannot read the code
      (CLAUDE.md), so a mockup will carry `measured` and `lampsacus` as
      representative strings and nobody downstream can check them.
      **Measured:** `luminous stasis of delos` is fully reachable (`luminous`
      4338, `stasis` `nouns.rs:924`, `delos` `places.rs:213`); `kairos`
      (`nouns.rs:465`) and `abdera` (`places.rs:25`) are real but `brittle` is
      not; `measured` and `lampsacus` are both absent. One of the three examples
      is sound. **Severity: medium** — CLAUDE.md requires the brief be fixed in
      the same change that invalidates it, and this change is what shipped the
      lists that invalidate it.

      **FIXED.** Line 124 now reads *pensive aporia of lampsakos*, *quipful
      ismene of korykos*, *luminous stasis of delos* — your verified one kept,
      and the two replacements chosen for provenance rather than plausibility:
      `quipful ismene of korykos` is `PINNED_NAME_FOR_KEY_4` in `names.rs`, a
      name the shipped scheme actually derives, and `pensive` (adjectives 5155) /
      `aporia` (nouns 129) / `lampsakos` (places 528) are each verified by index.
      Also fixed at line 19 (the same specimen in the header block) and line 143
      (*pensive aporia lampsakos* for the connector-dropped form). Line 19's
      `§5.2.1` citation was repointed at the spec in the same pass, since the
      shed moves that heading.

      **Two additions beyond what you raised**, both consequences of this pass:
      the brief now describes the per-word gloss affordance (fetched on request,
      noun and place only, adjectives never), and the header block now names the
      spec rather than PLAN.md as the authority on the name's shape.

- [x] **`spec-writer`** — `spec.md:163`, `228`, `327`, `357`, `537-538`, `575` —
      the contract carries the same unreachable examples, so a fixer correcting
      the code has no authoritative spelling to correct them *to*.
      **Scenario:** the spec was written before the wordlists existed, so its
      examples were aspirational rather than wrong at the time. They are now the
      reference a `dev-writer` would reach for while fixing the six findings
      above, and they would propagate the same fabrications back into the code.
      **Measured:** `measured`, `brittle`, `damp` and `lampsacus` all absent from
      the shipped lists, as above. **Severity: low as a defect, high as a
      blocker** — fix this one first or the code fixes will re-copy it.

      **FIXED, and fixed first, for the reason you give.** Every example in the
      spec is now a word verified present by index in the shipped arrays. The
      authoritative spellings a `dev-writer` should copy:

      | Slot | Word | Index |
      |---|---|---|
      | adjective | `pensive` | `adjectives.rs:5155` |
      | adjective | `luminous` | `adjectives.rs:4338` |
      | adjective | `restless` | `adjectives.rs:6210` |
      | noun | `aporia` | `nouns.rs:129` |
      | noun | `stasis` | `nouns.rs:924` |
      | noun | `zenon` | `nouns.rs:1042` |
      | place | `lampsakos` | `places.rs:528` |
      | place | `delos` | `places.rs:213` |
      | place | `kition` | `places.rs:454` |

      Full names used: *pensive aporia of lampsakos* (the workhorse, replacing
      *measured aporia of lampsacus* throughout), *luminous stasis of delos*, and
      *pensive zenon of kition of lampsakos* for the two-places hazard — which is
      now a **better** illustration than *measured zeno of citium of lampsacus*,
      because `zenon` and `kition` are both real entries under the kappa rule, so
      the example shows a source's qualified form colliding with real list
      contents rather than with invented ones.

      **Two further fabrications found in the same sweep and fixed**, neither in
      your list: `alexandria troas` and `heraclea pontica` appear at spec lines
      500 and 571 as the multi-word-place examples and are **both absent** from
      `places.rs` — they survive only in that file's header comment. Real
      multi-word entries are `lokroi epizephyrioi` (567), `antiocheia maiandros`
      (113), `arsinoe kyprou` (139), `euxeinos pontos` (292) and five others; the
      spec now uses the first two. Worth flagging to the `dev-writer`: the header
      comment at `places.rs:16` illustrates the internal-space rule with the two
      absent ones, which is the same defect one layer down.

## A named constant that documents an invariant it does not enforce

- [ ] **`dev-writer`** — `names.rs:109-122` and `names.rs:299` —
      `NAME_DIGEST_BOUND` carries a 14-line doc comment describing the bound as
      the thing that keeps the derivation finite, but nothing in the derivation
      reads it, and the one line that mentions it is an assertion that cannot
      fail.
      **Scenario:** a reader arrives at *"The first byte past the name's slice of
      the digest… **Beyond this the derivation fails rather than reading on**"*
      and reasonably looks for where `name_from_digest` bounds itself by the
      constant. It does not: `draw_at(digest, 0)` and `draw_at(digest, 6)` are
      called with literals, and `draw_at` indexes `digest[offset + i]` with no
      reference to the bound. The only non-test occurrence is
      `debug_assert_eq!(NAME_DIGEST_BOUND, 12, "the budget is two draws of six")`
      — a constant compared to its own literal value, which is a tautology, is
      compiled out in release, and cannot fail under any edit to `draw_at`.
      Changing the literal offsets from `(0, 6)` to `(0, 8)` would silently move
      the real bound to 14 while the constant, its doc comment and the assert all
      continued to say 12.
      **Measured:** `grep -n NAME_DIGEST_BOUND src/names.rs` returns 4 hits —
      the definition (122), the tautological assert (299), and two test uses
      (780, 784). Zero uses bound anything. **Severity: medium** — a reader is
      told the invariant is enforced by a constant and it is enforced by two
      literals elsewhere.

## The replaced tests: the prose names a term the assertion omits

- [ ] **`dev-writer`** — `names.rs:1259-1272` — the comment explaining the
      three deleted tests names five terms as what they asserted absent, then the
      loop pins only four of them present, and the missing one is `strategos`.
      **Scenario:** the comment says *"Those asserted that `stoa`, `platon`,
      `sokrates`, `archon` and `strategos` were ABSENT"*, and justifies the
      inverted test with *"Pinned as PRESENT rather than merely 'not asserted
      absent', because a curation pass that quietly dropped them would otherwise
      reintroduce the withdrawn screen with every test still green."* The loop
      then iterates `["stoa", "agora", "archon", "tyrannos", "genesis"]` and
      `["platon", "aristoteles", "sokrates"]`. `strategos` is named in the prose,
      is in the list, and is pinned by nothing — so the exact gap the comment
      says it closes stays open for the one term it singled out.
      **Measured:** `grep -n "\"strategos\"" src/names/nouns.rs` = line 936
      (present); it appears in no assertion in `names.rs`. The historical claim
      itself checks out: at `d1e6506` the three tests were
      `no_entry_is_a_term_of_this_projects_own_vocabulary`,
      `no_noun_is_a_figure_whose_invocation_is_an_argument` and
      `no_entry_asserts_authority`, and `strategos` was in the third one's banned
      set. **Severity: medium** — the comment is accurate about history and
      inaccurate about what the code beneath it does.

## A transcription in design.md that would not compile

- [ ] **`dev-writer`** — `design.md:66` — the recorded `NAME_PREFIX` has seven
      trailing nulls where the code has six, in a sentence that asserts the value
      is 32 bytes.
      **Scenario:** the design records the separator as
      `b"/dialectica/1/Name/Display\0\0\0\0\0\0\0"` — 32 bytes, the same padded
      style…*. `/dialectica/1/Name/Display` is 26 characters, so seven nulls is 33
      bytes and would not compile as `&[u8; 32]`. A reader reconstructing the
      preimage from the design — which is the one document that records *why* the
      prefix is shaped this way — gets a value that hashes differently from every
      name the network computes.
      **Measured:** `names.rs:97` has six nulls;
      1 + 10 + 1 + 1 + 1 + 4 + 1 + 7 = 26 characters, + 6 = 32. `grep -rn
      "Name/Display"` finds exactly these two sites and they disagree.
      **Severity: low** — nothing reads design.md at build time, and the code is
      the correct one. Flagged because a consensus-critical constant transcribed
      wrong in the document that explains it is the kind of thing that gets copied
      back.

## A worked example that changes transliteration mid-sentence

- [ ] **`dev-writer`** — `names/nouns.rs:19-22` and `names.rs:1231-1236` — the
      "no noun carries the connector" rationale names its inputs in Greek
      transliteration and its output in Latinised English, so the example does not
      demonstrate the substitution it describes.
      **Scenario:** *"A source supplying named historical Greeks supplies them
      already QUALIFIED — `zenon kitieus`, `straton lampsakenos` — and such an
      entry renders *measured zeno of citium of lampsacus*."* Neither `zenon
      kitieus` nor `straton lampsakenos` renders as `zeno of citium`; a reader
      tracing the example has to work out independently that "zeno of citium" is
      an English gloss of `zenon kitieus` rather than a third form, and the point
      being made is about a literal substring.
      **Measured:** `nouns.rs:1042` holds `zenon`; no entry contains a space
      (the test at `names.rs:1230` pins this). `design.md:242` states the same
      example consistently — *"renders *measured zenon kitieus of lampsakos*"* —
      which is the version that actually demonstrates the problem.
      **Severity: low.** Stylistic-adjacent, but it is the only explanation of why
      a whole class of source entries was cut, so a reader who cannot follow it
      cannot check the decision.

---

## What was clean

The **module header** (`names.rs:1-56`) is the strongest prose in the change:
the four sections each answer a question a reader would actually ask, the
"What a name is NOT" section pre-empts the three misreadings that matter, and
its claim that `feed.rs` carried a doc comment asserting the opposite is true —
I read the removed comment in the diff and it said exactly what the header says
it said. The correction at `feed.rs:120-131`, which quotes the false comment
verbatim before replacing it, is the right shape for this repo: it leaves the
reader able to recognise the error if they meet it in an older checkout.

**Test names say what would break.** `every_wordlist_is_pinned_entry_by_entry_and_in_order`,
`a_refused_pair_redraws_every_slot_from_the_reserve`,
`an_unrefused_draw_never_consults_the_reserve`,
`exhausting_the_reserve_fails_rather_than_reading_on` and
`the_pinned_name_is_derivable_by_hand_from_the_pinned_digest` each name a
failure rather than a subject. No `handle`, `process` or `And` anywhere in the
change.

**The pin tests' "if this fails, do not update it to match" instruction** appears
at `names.rs:391`, `464`, `413` and `626-629` — four sites, consistently worded,
each explaining what a failure *means* rather than what to do about it. That is
the comment earning its place by saying what a command cannot.

**The `// NO SPEC:` marker at `feed.rs:287` reads true** on its substance: I
checked `spec.md:775-778` and the spec does forbid a placeholder and is silent on
a feed's handling of an underivable row. Only its trailing citation is broken
(above).

**`draw_at`'s uniformity argument** (`names.rs:311-323`) is checkable and checks
out: 65,536/8,192 = 8 and 65,536/1,024 = 64 are both whole, so the "exactly
uniform, no modulo bias" claim is sound and the "power-of-two sizes are
load-bearing" conclusion follows from it.

`DisplayName`'s three-field shape with the connector living only in `render()`
does make "the connector is not a slot" structural rather than tested-at-every-
call-site, as `names.rs:124-134` claims. `words()` is a good name for what it
returns.

## What no gate here can see

**`cargo fmt --check` never reaches any code in this change.**
`ci.yml:696` runs `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml
--check`, and `rust-lib/Cargo.toml` declares an **empty `[workspace]` table**
(line 11) with `dialectica-core` as a plain path dependency (line 32) — a path
dependency is not a workspace member, so `cargo fmt` does not follow it. Every
line this change adds lives under `dialectica-core/src/`. This is a
previously-recorded gap rather than a new one, but it is worth restating at this
size: ~11,900 new lines are format-checked by nothing. Clippy *is* scoped
correctly (`-p dialectica-core` is named explicitly at `ci.yml:712`, with a
comment explaining why), so lints do reach this code even though formatting does
not.

**No gate can see a fabricated example.** The ten findings above are all
comments, and no test, lint or CI step in this repo reads a comment. The
`every_wordlist_is_pinned_entry_by_entry_and_in_order` hash pin would catch
someone *editing a list to match a comment*, which is the right direction — but
nothing catches a comment drifting from the list.

**I could not run the suite in this worktree.** `cargo test` fails at manifest
resolution because `dialectica/logos-rust-sdk-src` is a gitignored symlink into
the Nix store that a fresh `git worktree add` does not create, and creating it
was denied. No finding above depends on a test run; all are file reads and hand
arithmetic, with the commands shown.
