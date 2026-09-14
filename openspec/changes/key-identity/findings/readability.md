# Readability review — `key-identity`

Dimension: **readability only**. Correctness, security and architecture are held
by other instances; an unticked row for those still means nobody has done them.

Method: every number, byte offset, index, count and range in every new or edited
comment was checked by grepping the wordlist text files, hand-computing the
draws from the pinned key hex, and re-running the three `sha256sum` commands the
doc comment tells a reviewer to re-run. What was checked and found **true** is
recorded in prose at the bottom, so the short list below is not mistaken for the
whole of the verification.

## Findings

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/names.rs:1716-1720` —
      the parenthetical that exists to stop a reader confusing an array index
      with a file line number states both offsets one too low, so following its
      own instruction produces the wrong line.
      **Scenario:** the comment says *"The literals start at line 27 of `nouns.rs`
      and line 23 of `places.rs`, so a grep's line number is the index plus that
      offset."* Applying it to the two indices the same comment gives: zenon
      index 1015 + 27 = line 1042, and kition index 431 + 23 = line 454.
      **Measured:** `grep -n '"zenon"' names/nouns.rs` → **1043**;
      `grep -n '"kition"' names/places.rs` → **455**. Both off by one.
      The real first literal is `"acheron"` at line **28** of `nouns.rs` and
      `"abai"` at line **24** of `places.rs` — line 27 and line 23 are the
      `pub const NOUNS/PLACES = &[` lines, not the first entry.
      The indices themselves (1015, 431) are **correct** — verified against
      `wordlists/nouns.txt:1016` and `wordlists/places.txt:432`. Only the
      offsets are wrong. Severity: low impact, high irony — this is the one
      comment in the change whose entire job is preventing this class of error,
      and a reader who trusts it lands on the neighbouring word.
      **Fix:** 27 → 28 and 23 → 24.

      **Fixed.** Both offsets corrected, and I re-ran your two greps rather than
      taking them: `"zenon"` → `nouns.rs:1043`, `"kition"` → `places.rs:455`, and
      the first literals are `"acheron"` at `nouns.rs:28` and `"abai"` at
      `places.rs:24` with the `pub const … = &[` lines at 27 and 23 exactly as
      you found. The comment now also *shows* the arithmetic it is asking a
      reader to do — `1015 + 28 = 1043`, `431 + 24 = 455` — so the offset and the
      answer are both checkable against one grep, rather than the offset alone
      being asserted. No test covers this (it is a comment); the correction is
      verified by the two greps above.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/names.rs:347` —
      a rustdoc intra-doc link to `[`NAME_KEY_BYTES`]`, an identifier this change
      deleted. The item is now the `const fn name_key_bytes()` declared 170 lines
      above, and `names.rs:172` already refers to the old name in prose where it
      is correct (it is describing the rejected `const` alternative).
      **Scenario:** a reader following the link in generated docs gets nothing —
      rustdoc resolves no item named `NAME_KEY_BYTES`, so the bracketed text
      renders as literal text rather than a link to the range the sentence is
      about. The sentence *"The slice below is taken at [`NAME_KEY_BYTES`] rather
      than indexed relative to it"* names a constant that does not exist in the
      shipped crate.
      **Measured:** `grep -rn "NAME_KEY_BYTES"` over `dialectica/` returns two
      hits in `names.rs` — line 172 (correct, prose about the rejected form) and
      line 347 (this one, a live doc link). CI runs no rustdoc step
      (`grep -n "rustdoc\|broken_intra_doc_links" .github/workflows/ci.yml` →
      no output), so nothing catches it.
      **Fix:** `[`NAME_KEY_BYTES`]` → `[`name_key_bytes`]`.

      **Fixed** as you specify. `grep -n "NAME_KEY_BYTES" names.rs` now returns
      one hit, line 172, which is the prose about the rejected `const` form and
      is correct there — your reading of the two sites was right.

      The same sentence carried a second and worse defect, which the architecture
      reviewer filed separately and which is fixed in the same edit: it ended
      *"widening a draw past it does not compile"*, and that is false. I measured
      both halves rather than trusting either review — `word(4)` → `word(5)`
      **compiles** and panics at `names.rs:362` (`index out of bounds: the len is
      6 but the index is 6`); widening the range to `18..25` **compiles** and
      fails two tests at runtime. The doc now says the invariant is structural
      (the two constants cannot disagree) but the enforcement is test-caught, and
      names the two tests that catch it.

      `design.md` also described the window as a `Range<usize>` **constant**
      called `NAME_KEY_BYTES` and showed `key_bytes[NAME_KEY_BYTES]`, which is
      the same dead name one document further out; corrected there too, with a
      line saying it began as a `const` and why it is not one, so the entry below
      that explains the rename is no longer contradicted by the entry above it.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/AddressLabel.qml:27-28` — the
      worked example names only half of what it demonstrates, and the half it
      omits is the one the surrounding paragraph calls the severe case.
      **Scenario:** the comment says *"The middle group is CENTRED, so widening
      it walks outward in both directions at once — `middleChars: 20` reaches key
      byte 11, which the mark reads."* The arithmetic is `start = floor((64 - mid)
      / 2)`, so at `mid: 20`, `start = floor(44/2) = 22`, and chars 22..41 are key
      bytes **11 through 20**. That reaches byte 11 (the mark) as stated — and
      also bytes **18, 19, 20**, which are the generated name's window. The
      example is the only concrete demonstration of "walks outward in both
      directions", yet it reports only the leftward collision, leaving the
      rightward one — a *displayed* byte reaching the *name*, which lines 24-26
      single out as "the worst kind to share" — unmentioned.
      **Measured:** current `DTheme.middleChars` is 8 (`DTheme.qml:172`), giving
      `start = 28`, chars 28..35, bytes 14..17 — matching the table at line 18
      exactly, so the table itself is correct and only the example is partial.
      Severity: genuine defect, not style — a reader retuning the group would
      check byte 11 against the mark, see one collision, and could conclude
      moving the group leftward fixes it.
      **Fix:** extend the example to say `middleChars: 20` reaches key bytes
      11..20, colliding with the mark at 11 and with the name at 18..20.

      **Fixed**, and I re-derived the arithmetic rather than taking it: `start =
      Math.floor((body.length - mid) / 2)` at `AddressLabel.qml:52`, so at
      `mid: 20` over a 64-character body `start = floor(44/2) = 22`, chars 22..41,
      key bytes **11..20** — your figures exactly. The comment now states the
      full range, names both collisions (the mark at 11, the name at 18, 19, 20),
      shows the `floor` step so the next person retuning the group can redo it,
      and closes on the trap you identified: *"Checking only the leftward
      collision and moving the group left would look like a fix and would not be
      one."*

      No test covers the comment, but the behaviour it describes is covered —
      `test_no_byte_on_screen_reaches_the_mark_or_the_name` is what fails if
      anyone acts on the example and widens the group, and the security review
      measured `middleChars: 8 → 14` failing it with "key byte 18 is DISPLAYED by
      the abbreviation and also read by the generated name". The comment was
      wrong about which collisions exist; the gate was never wrong.

- [x] **`spec-writer`** — `openspec/changes/key-identity/specs/generated-names/spec.md:506-508` —
      a migration pointer cites a requirement by a title this same delta renames,
      so "above" sends a reader to a heading that will not exist once the change
      is applied.
      **Scenario:** the REMOVED block's reason says *"The scheme-version half of
      this requirement is carried by *The scheme and its wordlists are versioned
      together and frozen*, above"*. The `## RENAMED Requirements` block at line
      369-370 of the same file renames that requirement **to** *The scheme and its
      wordlists are frozen, with no version to bump*. After the change applies,
      no heading bears the cited title.
      **Measured:** `grep -rn "versioned together and frozen" openspec/` returns
      the old title only in the archive
      (`changes/archive/2026-09-14-generated-names/`), in the current merged spec
      (`openspec/specs/generated-names/spec.md:713`), and in this delta's own
      `FROM:` line and `proposal.md:102` — never as a live heading in the delta's
      ADDED/MODIFIED sections. `grep -n "^### Requirement"` on the delta confirms
      the live heading is *"The scheme and its wordlists are frozen, with no
      version to bump"* (line 206).
      Severity: low — the rename block sits above and resolves it — but the
      citation is the kind a later reader greps for and does not find.
      **Fix:** cite the post-rename title, or say explicitly "renamed above to".

      **Fixed**, taking both halves of your suggestion rather than choosing: the
      citation is now the post-rename title *The scheme and its wordlists are
      frozen, with no version to bump*, it says explicitly that this change
      renames it from the old one and points at the `## RENAMED Requirements`
      block, and it states why the post-rename title is the one cited — it is the
      heading that exists once the change applies. Keeping the old title visible
      in the sentence means a reader who greps for either finds this.

      Verified: `grep -n "^### Requirement"` on the delta puts the cited title at
      line 206, and `openspec validate key-identity --strict` passes.

## What was checked and is correct

Recorded in prose rather than as boxes, because none of it needs action — and
because the count of numeric claims *verified true* is the useful measure of how
dense this change is.

**Every wordlist index literal in the change is right.** `zenon` 1015, `kition`
431, `pensive` 5136 (`wordlists/nouns.txt:1016`, `places.txt:432`,
`adjectives.txt:5137` — line number minus one). The collision fixture's
`(2628, 768, 971)` at `names.rs:419` resolves to `fearable` / `pindaros` /
`thorai`, matching `COLLIDING_NAME` exactly. The reordering example at
`names.rs:717` — "`araden` and `araithyrea` — places 100 and 101" — is right:
`places.txt:101` and `:102`.

**Both pinned names were reproduced by hand from the key hex, independently of
the crate.** For seed 7, key bytes 18..19 = `0x76ae` = 30382, `% 8192` = 5806;
bytes 20..21 = `0xbebe` = 48830, `% 1024` = 702; bytes 22..23 = `0x7b92` = 31634,
`% 1024` = 914. Those index `adjectives.txt:5807` = `quartzous`,
`nouns.txt:703` = `paris`, `places.txt:915` = `sypalettos` — "quartzous paris of
sypalettos", the pinned value.

**The byte-order coincidence claim at `names.rs:611` is exact.** `PINNED_CASES[0]`
key bytes 20 and 21 are both `0xbe`, so that case cannot fail on a little-endian
noun slot — which is precisely the reason the comment gives for seed 11 earning
its place. The QML counterpart at `tst_identicon.qml:447` claims case 2's slot
pairs are `(0xef/0x1a, 0x06/0xad, 0xa6/0x6d)`; decoding
`66be7e33…810c473a` at bytes 18..23 gives `ef 1a 06 ad a6 6d` — matching, and all
three pairs differ, so the "sensitive in every slot" claim holds.

**The three `sha256sum` commands in the wordlist-pin doc comment
(`names.rs:689-693`) were re-run and all three digests match** the pinned
constants byte for byte. The comment's claim that the paths are tracked and the
commands work on any checkout is true in this worktree.

**Every byte-allocation figure is internally consistent and agrees across all
five sites.** The allocation totals 25 of 32 (4+8+4+6+3), leaving seven
unallocated (12, 13, 24-28) — stated identically in `names.rs`'s module header,
`Identicon.qml`'s table, `AddressLabel.qml`'s table, `IDENTICON.md`'s table and
the spec's table. `IDENTICON.md`'s "8 of the 21 bytes the abbreviation hides and
none of the 11 it displays" checks out: displayed = 4+4+3 = 11, hidden = 21, mark
= 8. The abbreviation table was re-derived from `abbreviate()` itself rather than
taken on trust: at `mid: 8`, `start = floor((64-8)/2) = 28`, chars 28..35 →
bytes 14..17; head 8 chars → bytes 0..3; tail 6 chars → bytes 29..31.

**The `18..23` / `18..24` notation split is not a defect.** Prose and the spec
use inclusive `18..23` throughout; Rust code uses exclusive `18..24`, and
`names.rs:144` labels it *"`18..24`, six of them"*, which disambiguates at the
one place the two conventions meet. `SPEC_MARK_BYTES = 4..12` follows the same
rule. Consistent.

**The four corrected retraction sites are all now true.** `IDENTICON.md`,
`Identicon.qml`'s header, `names.rs`'s module header and the spec delta each
state that the reservation was fictional under the digest design and is real
under this one, with the mechanism named. A sweep for the old wording
(`grep -rn "DOES NOT EXIST\|fictional\|reserved for the generated-name"`) finds
no surviving uncorrected claim in the change's files. `docs/UI-BRIEF.md:227`
still carries a stale "scheme that does not exist", but the owner ruled that file
out of scope.

**`IDENTICON.md`'s three-version structure reads as argument, not archaeology.**
Each version is labelled with its truth status at the time ("wrong when
written" / "right when written and now superseded" / "the one that holds now"),
the conclusion is stated once as surviving all three, and the section closes on a
transferable lesson — *a mechanism can be fictional under one design and
load-bearing under the next*. A reader who needs only the current rule gets it
from the bolded first sentence and can stop. The paired lesson later in the file
("a cost correctly dismissed can come back") is the mirror of the original and
earns its place.

**No UI-BRIEF citation appears anywhere in this change's files.** Every
`grep -rn "UI-BRIEF"` hit is in a file this diff does not touch. The three
re-grounded claims all cite `generated-names` requirement titles that exist
verbatim in the delta: *A name is never unique, never an identifier, and never
numbered* (line 270), *The scheme and its wordlists are frozen, with no version
to bump* (line 206), and *The three channels read pairwise disjoint bytes of the
public key* (line 378). **All three check out** — cited in `docs/PLAN.md` and
`docs/IDENTICON.md`, and `names.rs:971-973` quotes the third correctly.

**`DKeyNameWindow.qml`'s purpose is evident without this brief.** Its header
answers, in order, the three questions a reader arrives with: what it is for (a
third channel for the disjointness gate), why it computes indices and not a name
(the wordlists are consensus-critical and belong in core), and why it is a
component rather than a constant (a probe that reads a constant follows the
constant and can never report an overlap). It also states the duplication as a
cost accepted with the alternative named, which is the shape this repo asks for.
The one-job rule holds: it computes three draw indices and nothing else.

**The comments say why rather than what throughout.** `names.rs`'s doc comments
on `name_key_bytes`, `NameError` and `name_from_key_bytes` each record a rejected
alternative and the reason — the tautological `debug_assert_eq!` that compiled
out, the `ReserveExhausted` arm that no input could produce, the `…_from_digest`
name that would invite a caller to hand it a digest. `examples/pin_name.rs`
states why it does not link the crate. Every function name is a noun phrase or a
single job; no `And`, no `handle`/`process`/`update`.

**Two pre-existing inconsistencies, noted but not opened as boxes** because this
change does not touch either line and fixing them belongs to whoever owns the
mark's arithmetic. `docs/IDENTICON.md:743-745` puts the mark's output at "about
13 bits" and the excess at 2^51; `Identicon.qml:25-27` says "about 16 bits" and
2^48. Both are internally consistent (64 − 13 = 51, 64 − 16 = 48) but they
disagree with each other, and neither matches the product of the shipped
selectors — 11 forms × 7 × 6 × 5 inks × 12 angles × 4 pitches × 3 duties × 3
weaves = 997,920 ≈ 2^19.9. Separately, `Identicon.qml:263` says "Ten pooled
forms" where `_form()` is `% 11` and `_tracePath` has eleven cases (0-9 plus
`default`); `Identicon.qml:151` says "eleven" correctly.

**One stylistic observation, not a defect.** In `tst_identicon.qml`, `_nameBytes`
resets `win.key` to the base after each byte (line 307) where `_displayedBytes`
and `_markBytes` do not. All three are correct — each iteration assigns a
complete 64-character hex string, so no state carries over — but the asymmetry
invites a reader to wonder whether one of them needs it. A one-line note, or
dropping the reset, would settle it.
