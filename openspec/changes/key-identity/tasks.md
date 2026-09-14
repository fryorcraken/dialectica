# Tasks — key-identity

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## Implementation

### The derivation reads the key

- [x] `name_from_digest(&[u8; 32])` becomes `name_from_key_bytes(&[u8; 32])`,
      drawing from `key_bytes[name_key_bytes()]` rather than from `digest[..6]`.
      The rename is load-bearing: a function still called `…_from_digest` invites
      a caller to hand it one, and a digest is 32 bytes too, so nothing would
      refuse it.
- [x] The window is **one range**, `name_key_bytes() -> 18..24`, with
      `NAME_BYTE_COUNT` derived from it. A start and a length could disagree;
      one range cannot be half-moved. A `const fn` rather than a `const Range`
      because a `const` of a non-`Copy` type re-materialises at each use and
      `rustc` warns on iterating it.
- [x] `NAME_PREFIX` deleted, and with it `name_digest()` — the latter was `pub`
      only so a test could show the name's digest differs from the address, a
      claim the spec now removes as inverted.
- [x] `sha2` no longer imported by the module; it appears only inside
      `mod tests`, hashing the **wordlists**.
- [x] `examples/pin_name.rs` takes a public key instead of a digest and draws
      from bytes `18..23`. It still does not link the crate — every pin below was
      re-derived through it.

### The allocation, enforced

- [x] Rust half — `names.rs` measures which key bytes actually move the name
      (`measured_name_bytes`, probing rather than reading the constant) and
      checks it against the spec's allocation: `the_name_reads_exactly_the_bytes_the_spec_allocates_to_it`,
      `the_names_bytes_overlap_neither_of_the_other_two_channels`,
      `the_name_reads_no_unallocated_byte`, and
      `the_spec_allocation_this_crate_restates_is_internally_consistent`.
- [x] QML half — `tst_identicon.qml` gains `_nameBytes()`, a third probe on the
      same measure-the-component pattern as `_markBytes` and `_displayedBytes`,
      and the pairwise test
      `test_the_three_channels_read_pairwise_disjoint_bytes` replaces the
      two-channel `test_the_mark_and_the_abbreviation_share_no_byte`.
      Stated **pairwise** with a per-channel non-emptiness precondition, because
      a union count passes when a set measures empty.
- [x] `DKeyNameWindow.qml` added so QML has a third channel to probe. It
      computes the three **draw indices** and deliberately carries no wordlists:
      the question the gate asks is which key bytes reach the name, and an index
      moves exactly when a byte it reads moves.
- [x] The meta-test `test_the_byte_probes_find_the_windows_they_should` extended
      to the third probe, so a probe that silently stopped detecting bytes fails
      there rather than reporting a false all-clear.

### Coverage the spec asks for beyond disjointness

- [x] `test_a_byte_one_channel_reads_moves_only_that_channel` — the property as
      an observable consequence rather than a set comparison.
- [x] `test_no_byte_on_screen_reaches_the_mark_or_the_name` — kept separate from
      the pairwise sweep so a failure says whether a *displayed* byte leaked,
      which the spec calls the severe case.
- [x] `test_no_channel_reads_an_unallocated_byte` (QML) and
      `the_name_reads_no_unallocated_byte` (Rust) — bytes `12..13`, `24..28`.
- [x] `test_the_name_window_agrees_with_cores_pinned_case` — the reduction
      arithmetic now exists in two languages, and this pins QML against the same
      third-party-derived case `names.rs` pins, so the two agree with
      `pin_name.rs` rather than with each other.
- [x] `test_a_malformed_key_still_yields_indices_in_range` — peer-supplied
      strings reach `DKeyNameWindow` too.

### Pins re-derived, never copied from the implementation

- [x] `PINNED_CASES` — two keys with their public key hex and expected name,
      both produced by `pin_name.rs`. **Two cases, chosen so their bytes at 16,
      17, 24 and 25 differ**, which is what a one-byte window slip would read;
      `the_pinned_cases_differ_outside_the_name_window` asserts that
      precondition rather than assuming it.
- [x] `a_pinned_name_is_reproducible_from_its_key_by_hand` — the index
      arithmetic done in the test, at offsets written out rather than taken from
      `name_key_bytes()`.
- [x] The colliding pair re-searched. The old one stopped colliding because the
      derivation changed, not because anything was papered over; both new keys
      verified through `pin_name.rs` to reach indices `(2628, 768, 971)`.
- [x] `key_bytes_drawing` writes its draws at **18**, not 0. At 0 every test
      using it would have been asserting about three zero draws — passing, and
      about nothing.

### The four falsified retractions corrected

- [x] `openspec/changes/key-identity/specs/generated-names/spec.md` — the delta
      `REMOVED`s *The derivation is domain-separated from every other use of the
      key* and *The mark and the abbreviated address read disjoint address
      bytes*, replacing both with the three-channel requirement. (The live spec
      at `openspec/specs/` keeps the old text until archive applies the delta.)
- [x] `docs/IDENTICON.md` — the byte-layout section carries the allocation
      table; the retraction passage now records **three** versions of the
      argument and which design each was right about.
- [x] `dialectica-ui/src/qml/Identicon.qml` — the header's "THE MECHANISM IT
      DESCRIBED DOES NOT EXIST" replaced by the allocation and why disjointness
      is now load-bearing.
- [x] `dialectica/rust-lib/dialectica-core/src/names.rs` — the module header's
      byte-allocation section, and `name_key_bytes()`'s doc comment.

### Left to `key-identity-sweep`, deliberately untouched

- [x] `identity.rs::address()`, `feed.rs`'s `author` field, `wire.rs:5743`'s
      assertion, `FeedScreen.qml:323`, and the `identity` spec's *An address is
      derived from a record, never from a bare key*. Verified not deleted.
- [x] `Identicon`'s property is still named `address`; renaming it is the same
      change as rewiring the call sites, and splitting that across two pieces
      leaves the tree not compiling in between. A comment says so at the
      property.
- [x] `OP_SIGNING_PREFIX` and Stoa addresses untouched — verified by grep.

### Gates

- [x] `cargo test -p dialectica -p dialectica-core` — 930 + 30 green.
- [x] `cargo clippy --all-targets -- -D warnings` on `dialectica-core` — clean.
- [x] `rustfmt --check` on both changed Rust files — clean.
- [x] QML suite — every spec green; `tst_identicon.qml` at 16.
- [x] `check_qml_names.py` and `check_qml_members.sh` — clean, `DKeyNameWindow`
      satisfies the `D`-prefix rule rather than taking an exemption.
- [x] `qmlformat` parses every changed QML file; `qmllint` clean on them.
- [x] `openspec validate --strict key-identity`.

### Each new gate watched failing

Not a claim that they are green — a claim that they can go red, each under a
mutation naming the defect it is for.

- [x] Name window → `16..22` (onto the abbreviation's middle group): Rust
      reports *key byte 16 is read by the name and DISPLAYED by the
      abbreviation*. 10 of 36 fail.
- [x] Name window → `6..12` (onto the mark): *key byte 6 is read by both the
      name and the mark*.
- [x] Name window → `23..29` (onto the unallocated run): *key byte 24 is
      unallocated but the name reads it*.
- [x] `DKeyNameWindow` adjective slot → byte 10 (the mark): QML pairwise test
      reports the overlap, and the probe reports the **measured** window
      `10,11,20,21,22,23` — following the mutated arithmetic, which is what
      distinguishes it from the deleted computed version.
- [x] → byte 16 (the abbreviation's middle group): the displayed-byte test fires
      as well as the pairwise one.
- [x] → byte 4: `test_a_byte_one_channel_reads_moves_only_that_channel` reports
      *key byte 4 belongs to mark but moved name as well*.
- [x] `DTheme.middleChars` → 20: four tests fail, including the unallocated-byte
      one catching byte 12. Restored.
- [x] `DKeyNameWindow` noun modulus → 1000: only
      `test_the_name_window_agrees_with_cores_pinned_case` fails — a drift no
      disjointness test can see, which is why that pin exists.
