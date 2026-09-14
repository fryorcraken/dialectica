# key-identity — architecture review

Dimension covered: **architecture** only. Correctness, security and readability
are other instances'.

The two-language duplication is the right cut and the pin genuinely works —
both verified by mutation, see *What was measured* below. The findings are about
where the duplication's obligation is **recorded**, one overstated mechanism
claim, and one asymmetry the #80 split leaves in the tree.

## Findings

- [ ] **`spec-writer`** — `openspec/changes/key-identity/specs/generated-names/spec.md:378-475` —
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/names.rs:349` (and
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/identity.rs:89-101,
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
