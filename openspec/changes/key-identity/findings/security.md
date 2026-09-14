# Security review — `key-identity`

Dimension: **security only**. Correctness, readability, architecture, spec-test
and design are held by other reviewers; nothing below covers them.

Reviewed at `piece/key-identity` = `c9de1c4`, in a worktree of its own. Every
claim below was produced by running or breaking the code, not by reading it.

## Findings

**None.** No box is opened below, and that is a conclusion rather than an
omission: I attacked the piece's central property five ways and the gates caught
all five, with the precise byte named in each failure. The evidence is set out
below so the next reader can check the claim rather than take it.

One candidate finding was investigated and **withdrawn** — it is recorded at the
end under *A finding I withdrew*, because a reviewer who re-derives it should
meet the disproof rather than repeat the work.

## Verdict on the piece's central property

**Byte-disjointness is enforced by measurement, in both languages. It is not
merely asserted.** Five independent mutations, each caught:

| mutation | result |
|---|---|
| `name_key_bytes()` `18..24` → `6..12` (name onto the mark) | **11 of 70** name tests fail; `the_names_bytes_overlap_neither_of_the_other_two_channels` reports "key byte 6 is read by both the name and the mark" |
| `Identicon._weave()` `_byte(11)` → `_byte(18)` (mark onto the name) | **3 of 17** identicon tests fail; the pairwise sweep reports "key byte 18 is read by BOTH the mark and the generated name" |
| `DTheme.middleChars` 8 → 14 (abbreviation onto the name) | **5 of 17** fail, including `test_no_byte_on_screen_reaches_the_mark_or_the_name`: "key byte 18 is DISPLAYED by the abbreviation and also read by the generated name" |
| `DKeyNameWindow.adjectiveIndex()` `% 8192` → `% 4096` (cross-language modulus drift) | `test_the_name_window_agrees_with_cores_pinned_case` fails: actual 1710, expected 5806 |
| Rust `SPEC_ABBREVIATION_BYTES` tail `29..32` → `28..31` (restatement drift) | `the_restated_channels_are_the_spec_s_byte_sets_and_not_merely_disjoint_ones` fails alone, 1 of 38 |

Two properties make these gates real rather than decorative, and both were
exercised:

- **They probe the components, not restate their arithmetic.** The mark, the
  abbreviation and the name window are each measured by varying one key byte and
  watching the output move. That is why the `DTheme` widening is caught: a test
  that recomputed `abbreviate`'s arithmetic would have followed the constant.
- **Non-emptiness is a precondition on both sides.** `names.rs:880` and
  `tst_identicon.qml:356` each refuse a measurement that found nothing, closing
  the "an empty set is disjoint from everything" hole this repo has been bitten
  by before.

The multi-value probe set matters and is correctly chosen: seven spread values
rather than one, because `_weave()` is `_byte(11) % 3` and `0x00 % 3 == 0xff % 3`,
so a single-`ff` probe reports byte 11 unread. That defect is recorded in the
test file as having actually shipped once.

## `OP_SIGNING_PREFIX` and Stoa addresses: intact, provably

`git diff --stat origin/main...piece/key-identity` over `identity.rs`, `op.rs`,
`wire.rs` and `feed.rs` returns **empty output** — all four are byte-identical to
`origin/main`. `OP_SIGNING_PREFIX`, `STOA_ADDRESS_PREFIX` and
`AUTHOR_ADDRESS_PREFIX` still stand at `identity.rs:61`, `:53` and `:49`. All 38
`identity::tests` pass, including `signing_is_domain_separated_from_a_bare_digest`
and `the_wire_constants_are_pinned_to_known_answers`.

"No prefixes" reached only the name's deleted `NAME_PREFIX`. It did not touch the
signing path, which is the highest-severity defect this piece could have had and
does not.

`AddressLabel.qml`'s executable body is unchanged — its diff is comment-only — so
the shared author/Stoa abbreviation path is untouched, as the spec requires. The
Rust suite keeps a Stoa address as a live fixture in
`no_other_32_byte_value_travelling_beside_a_key_reaches_its_name`, with a
precondition asserting the fixture differs from the key inside the name's window
so the case cannot pass for the wrong reason.

## What else was clean

**No peer-supplied bytes reach the name derivation from the wire.** No module RPC
method derives a name. `wire.rs` pins the *absence* of `displayName` on every
reply shape (`a_feed_row_carries_no_display_name`, plus the key-set assertion at
`wire.rs:5734`), and `no_method_accepts_a_display_name_where_an_identity_is_required`
pins that no method accepts one where an identity is required — tested with real
derived names and the colliding pair, not name-shaped junk. The derivation is
reachable only from a key the peer has already parsed.

**Malformed key material cannot panic the module.** `name_from_key_bytes` takes
`&[u8; 32]` and slices a statically in-bounds `18..24`, so its `expect` is
unreachable by construction — no input makes it fire.
`display_name_from_bytes` parses first and refuses:
`arbitrary_bytes_do_not_abort_the_process` sweeps every length 0..70 against two
byte patterns, and `malformed_key_material_is_refused_rather_than_crashed_on`
covers non-points and the low-order all-zero point. No `unwrap` sits on a peer
path. This matters because PHASE0-FINDINGS §3 makes an unguarded panic a
remotely triggerable DoS.

**`DKeyNameWindow.qml` survives hostile input.** I wrote a throwaway probe spec
feeding it 19 adversarial strings — empty, bare and repeated prefixes (`k:`,
`kkkkkkk:`, `::::`, `k:k:k:`), non-hex, a NUL and a control byte, an unpaired
surrogate, a lone emoji, `k:-1`, an over-long key with trailing junk, and a
10,000-character string. Every case gave indices in range (`0..8192`, `0..1024`,
`0..1024`), no `NaN`, and `_body.length === 64` exactly; `AddressLabel` returned
a string for all of them. The pad-and-slice shape is shared with
`Identicon._body`, which the suite already covers. The probe file was deleted and
is not in the commit.

**Removing the hash costs nothing beyond the accepted item.** The owner's ruling
accepts one cost — no versioning seam for the wordlists — and I looked for others.
The concern that a raw-byte name makes key bytes observable where only digest
bytes were before does not land: a public key is public by construction, no reply
carries a name, and a name never travels without the key it derives from.
Grinding cost is also unchanged in practice — an Ed25519 public key is a
compressed curve point whose bytes cannot be chosen, so each candidate costs one
scalar multiplication whether or not a SHA-256 follows it; deleting the hash saves
the attacker a rounding error.

**`cargo mutants`** on `names.rs`: 26 mutants found, baseline green. Abandoned per
the brief's time limit with 14 resolved — **0 missed**, 0 timed out, 4 caught
(both arithmetic mutations on `NAME_BYTE_COUNT`, the `DisplayName::words`
replacement, and a `name_key_bytes` replacement), 7 unviable, all of the latter
being `cargo mutants` failing to synthesise a `std::ops::Range`. Partial, and
reported as partial.

**CI would run these gates.** `ci.yml:957` runs the QML suite under
`REQUIRE_QML_TESTS=1`, and `:969-979` asserts every `tst_*.qml` on disk actually
ran, so a spec that quietly stopped being discovered fails. The new
`DKeyNameWindow` is declared in `qmldir` and carries the mandatory `D` prefix, so
the host-collision name gate is satisfied.

**Documents carrying security claims are consistent with the code.** `PLAN.md`
keeps the moderation obligation — a moderator's *public key* must be present
rather than one click away — and states that Stoa addresses are untouched.
`IDENTICON.md` and both QML files carry the same allocation table as the spec.

## A finding I withdrew

I drafted a finding that the mark's byte 5 was under-used — that
`_outlineInk()`'s `_byte(5) % 5` contributed under 2.33 bits against the eight
bits the allocation implies, and that shrinking it to `% 2` would pass the suite.
**Both halves are false, and I disproved them by running them rather than by
re-reading.**

- The mutation `_byte(5) % 5` → `_byte(5) % 2` **fails**
  `test_a_fixed_address_selects_fixed_values` ("outline indexing", actual
  `#42744f`, expected `#71321f`). The modulus is pinned.
- The `% 5` is exactly right rather than lossy. `_outlineInk` walks forward from
  ink B by `k + 1` steps skipping ink A, so the reachable set is the 5 inks that
  are neither A nor B — and `k ∈ 0..4` spans precisely those 5. A wider modulus
  would alias; a narrower one would make some ink unreachable.

Recorded because the claim is plausible on a reading and the disproof takes one
command, which is the shape this repo's `run the claim, don't read it` note is
about.
