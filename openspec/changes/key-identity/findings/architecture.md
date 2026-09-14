# key-identity — architecture review

Dimension covered: **architecture** only. Correctness, security and readability
are other instances'.

The two-language duplication is the right cut and the pin genuinely works —
both verified by mutation, see *What was measured* below. The findings are about
where the duplication's obligation is **recorded**, one overstated mechanism
claim, and one asymmetry the #80 split leaves in the tree.

## Findings

- [x] **`spec-writer`** — `openspec/changes/key-identity/specs/generated-names/spec.md:378-475` —
      the requirement that forces the QML duplicate exists in no spec
      **What is wrong:** the disjointness requirement is written
      implementation-neutral (correct spec hygiene), so nothing in `openspec/specs/`
      says *the name's window must also be derivable in QML*, nor that the QML
      copy must agree with core. That obligation lives only in `design.md` — which
      `openspec archive` moves to `openspec/changes/archive/<date>-key-identity/`,
      a folder no reader of `DKeyNameWindow.qml` will think to open — and in the
      component's own header.
      **Scenario:** a later change judges `DKeyNameWindow` to be dead code (it has
      **zero production consumers**, measured: `grep -rln DKeyNameWindow
      dialectica-ui/` returns only `tst_identicon.qml` and `qmldir`) and deletes it.
      `tst_identicon.qml`'s three-channel gate collapses to the two-channel gate it
      was before this piece, `openspec validate --strict` passes, and every
      requirement above still reads as satisfied because each is phrased about
      "the set of key bytes the name reads" without saying where that set is
      measured. The piece's central security property silently loses its only
      pairwise gate.
      **Severity:** high — this is the one thing that makes the accepted
      duplication safe rather than merely accepted.
      **Suggested shape:** a scenario under *The three channels read pairwise
      disjoint bytes* stating that all three channels are measured in one place and
      that the name's window is reachable there, so deleting the third probe fails
      a requirement rather than only shrinking a test file.

      **Fixed, in the shape you suggested** — and kept implementation-neutral, so
      the spec hygiene you credited is not traded away to close this. The
      requirement now carries a normative paragraph obliging all three channels to
      be **measurable together**, each **by varying key bytes and observing its
      output rather than by restating its arithmetic**, which names no component,
      no language and no file; and the scenario *All three channels are measurable
      in one place*, whose THEN is that the name's window is reachable there "so
      removing whatever makes it reachable fails this requirement rather than only
      shrinking a test file". It says why it is a requirement and not an
      implementation note: it is the only thing making the accepted duplication
      safe rather than merely accepted.

      **The test that fails without it.**
      `test_all_three_channels_are_reachable_from_this_file` pins the channel
      count at three, over a single shared `_allChannels()` definition the pairwise
      sweep and the unallocated-byte test now both draw from — so the three cannot
      drift into checking different channel sets from each other.

      Your deletion scenario, run: dropping the name channel from `_allChannels()`
      leaves **17 of 18 tests green**, including the pairwise sweep, the
      unallocated-byte test and every probe test. Only the count test fails
      (`Actual: 2, Expected: 3`). That reproduces your finding exactly — the
      collapse is silent to everything that was there before — and is the
      measurement for why the count is asserted separately rather than trusted to
      the sweep. Restored; 18/18 green.

      The count is deliberately **not** a restatement of *which* channels they
      are: `test_the_byte_probes_find_the_windows_they_should` already establishes
      that by measurement, and duplicating it here would be the restatement trap
      one level out. This asserts only that the gate still reaches as many
      channels as the requirement names.

      `design.md` records the same thing, with your point about it archiving as
      the stated reason the obligation was moved into the spec rather than left
      there.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/names.rs:349` (and
      the same claim at `:162-163`) — the doc claims a compile error where the
      mechanism is a runtime panic
      **What is wrong:** "widening a draw past it does not compile" and "moving the
      range moves the read and a draw past it does not compile". Neither is true.
      `word(i)` indexes `drawn` through a closure, so the index is a runtime value;
      `NAME_BYTE_COUNT` is derived from `name_key_bytes()` and therefore follows any
      range change instead of contradicting it.
      **Measured:** changing `word(4)` to `word(5)` **compiles** and panics at
      `names.rs:362` — `index out of bounds: the len is 6 but the index is 6` —
      caught by tests, not by rustc. Separately, widening the range to `18..25`
      also **compiles**, failing 2 of 38 `names::` tests at runtime.
      **Why it is architectural rather than a typo:** the design's case for one
      `Range` constant rests on the invariant being structural ("one range cannot be
      half-moved", "an allocation, not merely an offset"). The structural half is
      real — the two constants genuinely cannot disagree — but the *enforcement* is
      a test-caught runtime panic, and a reader who believes the compiler is holding
      this will not notice if those two tests are ever weakened. State what actually
      holds it.

      **Fixed**, and I re-ran both mutations rather than taking them — this repo's
      `run the claim, don't read it` note cuts both ways, and a review claim is a
      claim. Both of yours reproduce exactly:
      - `word(4)` → `word(5)` **compiles**, and panics at `names.rs:362`:
        `index out of bounds: the len is 6 but the index is 6`. Caught by
        `the_name_scheme_is_pinned_to_known_answers`, not by rustc.
      - `name_key_bytes()` `18..24` → `18..25` **compiles**, and fails 2 of 38
        `names::` tests at runtime —
        `the_spec_allocation_this_crate_restates_is_internally_consistent`
        (`left: 26, right: 25`) and
        `the_restated_channels_are_the_spec_s_byte_sets_and_not_merely_disjoint_ones`
        (`left: [18,…,23,24]`).

      Both sites now separate the two claims rather than dropping the first, which
      is your point about it being architectural rather than a typo: the
      *structural* half is real and is stated as such — `NAME_BYTE_COUNT` is
      derived from the range so the two constants cannot disagree, and the
      derivation reads one slice so there is no second copy of the window — while
      the *enforcement* is named as test-caught, with the two tests listed by name
      and the note that weakening them weakens this.

      One further thing the second mutation turned up, recorded because it is
      counter-intuitive: widening to `18..25` does **not** fail
      `the_name_reads_exactly_the_bytes_the_spec_allocates_to_it`. Three draws of
      two bytes read 18..23 whatever the range says, so byte 24 is inside the
      window and genuinely unread, and the probe correctly reports it so. The
      window and what is read are two different facts, and only the restatement
      tests see the first.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/identity.rs:89-101,
      254-272` — the only file the sweep must change carries no forward pointer
      **What is wrong:** every other file touched by this piece marks the
      intermediate state explicitly — `Identicon.qml:90-101` has a paragraph on why
      the `address` property keeps its name, `names.rs:46-52` names
      `key-identity-sweep` for the feed row. `identity.rs`, which holds
      `PublicKey::address()` and the `Address` doc comment *"A 32-byte address: an
      author's, or a Stoa's"*, has **no `#80` or sweep marker at all** (measured:
      `grep -rn "key-identity-sweep\|#80" dialectica/rust-lib/dialectica-core/src/
      dialectica-ui/src/` returns hits in `names.rs`, `Identicon.qml`,
      `AddressLabel.qml`, `DKeyNameWindow.qml`, `DTheme.qml` — and nothing in
      `identity.rs`).
      **Scenario:** after the sweep deletes the author address, `Address`'s doc
      comment still offers itself for "an author's" address and `address()` is
      either gone or orphaned. Nothing in the type says it became Stoa-only, and the
      brief flags exactly this: nothing will enforce it. A reader arriving at
      `Address` between the two pieces cannot tell that half its documented purpose
      is scheduled for deletion.
      **Severity:** medium — no behaviour is wrong today; it is the one place the
      split's intermediate state is invisible at the code that embodies it.

      **Fixed**, at both sites you named, and **comment-only** — `identity.rs`'s
      executable body is unchanged, so it stays byte-identical to `origin/main`
      except for doc comments. That was deliberate: the address removal belongs to
      the sweep, and `address()` still exists.

      - `Address`'s doc comment ("an author's, or a Stoa's") now says that half of
        that sentence is scheduled for deletion, names `key-identity` as the
        change that states the allocation and `key-identity-sweep` as the one that
        performs the removal, and gives the reason nothing changes here yet —
        renaming here and rewiring call sites there would split one rename across
        two pieces and leave the tree non-compiling in between, which is the
        argument `Identicon.qml`'s `address` property already carries. It closes
        on **Stoa addresses are untouched**, so a reader does not over-read the
        deletion.
      - `PublicKey::address()` now opens with the deletion notice and **"Do not
        add a caller"**, before the existing prose — which is left intact but
        prefaced with "Everything below describes the scheme as it stands and is
        why it was built this way, not an argument for keeping it", since that
        prose reads as a defence of the design otherwise.

      No test covers a doc comment. What is verifiable is the measurement that
      motivated the box: `grep -rn "key-identity-sweep\|#80"` over
      `dialectica-core/src/` and `dialectica-ui/src/` now returns `identity.rs`
      alongside the five files you found, which is what the box asked for.

## What was measured

Three mutations, run in an isolated worktree against a green baseline of **962
Rust tests** (932 + 30) and **288 QML tests**, all restored afterwards.

**The cross-language pin works, and it is the sole gate.** Changing
`DKeyNameWindow.qml`'s adjective modulus from `% 8192` to `% 8000` — a drift that
leaves the byte *window* untouched — left **16 of 17** `tst_identicon` tests green.
Every disjointness, probe, window, unallocated-byte and isolation test passed;
only `test_the_name_window_agrees_with_cores_pinned_case` failed, reporting
adjective index 6382 against the pinned 5806. This reproduces the design's own
claim exactly and confirms the pin is load-bearing rather than decorative.

**The pin is a genuine three-way agreement, not two implementations agreeing with
each other.** I re-derived the first pinned case by hand: key bytes 18,19 = `0x76ae`
= 30382, `% 8192` = 5806; bytes 20,21 = `0xbebe` = 48830, `% 1024` = 702; bytes
22,23 = `0x7b92` = 31634, `% 1024` = 914 — matching the QML index pins. Those
indices resolve in the wordlist text files to `quartzous` (line 5807), `paris`
(703) and `sypalettos` (915), which is the name Rust's `PINNED_CASES` pins
independently. Both sides trace to `examples/pin_name.rs`, which links no
derivation. The `0xbe/0xbe` noun coincidence both suites call out is real and is
why the second case is needed.

**The `D` prefix was taken, not exempted.** `check_qml_names.py` reports *32 QML
files and 18 qmldir entries checked, every declared type D-prefixed or named as
grandfathered*; the `GRANDFATHERED` set is untouched by this diff.
`check_qml_members.sh` is green over 19 files. I also tested whether the public
`qmldir` entry was gratuitous by demoting it to `internal` — it is **not**
gratuitous: the test fails with `DKeyNameWindow is not a type`, because a
directory carrying a `qmldir` resolves as a module and `internal` genuinely does
not export. Hypothesis refuted, not filed.

**No wire-contract widening.** `git diff --stat origin/main...piece/key-identity`
over `dialectica/src`, `identity.rs`, `feed.rs` and `check_qml_names.py` is empty.
This piece is allocation, not API, as claimed.

## Verdict on the two-language duplication

**The cut is correct.** `DKeyNameWindow` carrying indices but no wordlists is the
right boundary: it duplicates ~4 lines of arithmetic instead of 10,240
consensus-critical entries, and the indices are exactly sufficient for the gated
property, since an index moves iff a byte it reads moves. The three rejected
alternatives in `design.md` are each rejected for a reason I could verify — probing
`Core` cannot run under `qmltestrunner`, and a constant-reading probe would follow
the constant, which is the documented failure that got the earlier computed probe
deleted.

**Is there a shape that makes divergence impossible rather than detected?** Not
within Basecamp's sandbox. The three candidates all fail: generating
`DKeyNameWindow.qml` from the Rust constants at build time moves the duplication
into a generator and puts a QML file under a build step the QML gate would then be
testing rather than the shipped artefact; having QML call core is precisely what
`qmltestrunner` cannot do; and a shared data file cannot be read by a sandboxed
view. Detection is the reachable ceiling here, and the piece reaches it — with the
qualification in finding 1, that what is detected is pinned in a document that
archives.

One residual the pin cannot see, noted as context rather than as a finding
because it is unreachable today: `DKeyNameWindow`'s zero-padding of malformed keys
diverges from core, which refuses them outright (`NameError::NotAValidPublicKey`).
Changing the pad constant from `0` to `1` leaves **17 of 17** tests green. This is
harmless only because the component has no production consumer — it becomes a live
divergence the moment one is added, which is a further reason finding 1's record
should say the component is test-only by design.

## Areas that were clean

The `#80` split is **coherent**, and the intermediate tree makes sense. The piece
states the allocation property; the sweep performs the removal. `names.rs:46-52`
explains why the feed path cannot render a name until the sweep lands and names the
spec requirement (*no reply carries a display name*) that forbids the wrong remedy
meanwhile — that is the right way to leave a half-finished migration, because the
constraint is anchored in a spec rather than in a comment. `Identicon.qml`'s
retained `address` property is argued correctly: renaming it here and rewiring call
sites there would split one rename across two pieces and leave the tree
non-compiling in between. The deleted `no_other_32_byte_value_...` fixture being
re-pointed at a Stoa address rather than the author address is exactly right — a
test feeding a value #80 deletes would have gone on passing about nothing.

`NAME_PREFIX` and `name_digest()` being deleted rather than deprecated is the right
call for the reason given: a surviving `name_digest` would be read as evidence the
name still hashes. The four falsified retraction sites now describe the mechanism
that exists, and each keeps its correction trail (`Identicon.qml:51-59`, PLAN.md's
strikethroughs) rather than silently rewriting history — which matters here, since
the retractions were *correct about the design they described*.

The `const fn` over `const Range` decision is sound and its recorded reasoning
(`const_item_mutation`, and the `.clone()` a later reader would delete) is a real
trap correctly avoided. Pairwise-rather-than-union and the non-emptiness
preconditions are right, and the meta-test pinning all three probes against their
windows closes the false-all-clear hole properly.

The worktree was left clean and was removed after this file was committed.
