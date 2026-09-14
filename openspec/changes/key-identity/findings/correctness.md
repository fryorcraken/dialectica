# Correctness review — `key-identity`

Dimension covered: **correctness only**. Security, readability, architecture,
spec-test and design are held by the other five reviewers and are not
re-derived here.

Two gates were run to a green baseline first: `cargo test -p dialectica -p
dialectica-core` at **962 passed, 0 failed**, and `run-qml-tests.sh` at **284
passed, 0 failed across 13 spec files**. Every measurement below is stated
against those baselines.

## Findings

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/names.rs:230-234` —
      `Display for DisplayName` can be replaced by a no-op and nothing fails
      **Scenario:** replace the body of `fmt` with `Ok(())`, so that
      `format!("{name}")` and `name.to_string()` both yield the empty string
      while `render()` keeps working. A caller reaching a name through `{}`
      rather than through `.render()` then displays nothing where a participant's
      name belongs — the "name attributable to nobody" the module's own
      `a_failure_is_never_reported_as_a_name` exists to forbid, arrived at
      through the formatting impl instead of through the error path.
      **Measured:** `cargo mutants --file "**/names.rs"` reports
      `26 mutants tested: 1 missed, 15 caught, 10 unviable`, the single survivor
      being this one. Confirmed by hand: with `fmt` returning `Ok(())`,
      **962 of 962 tests pass**. No test anywhere exercises `DisplayName` through
      its `Display` impl — every assertion calls `.render()` directly.
      Severity: **low-to-moderate**. It is latent rather than live, because the
      derivation currently has no production caller at all (see the prose below);
      it becomes live the moment the `key-identity-sweep` piece wires a renderer
      that uses `{}`. One assertion that `name.to_string() == name.render()`
      closes it.

      **Fixed.** `displaying_a_name_gives_the_same_text_as_rendering_it`, over 29
      seeds, asserting both routes rather than the one you name: `to_string()`
      **and** `format!("{name}")`. They are separate assertions because an impl
      could in principle satisfy one and not the other, and `{}` is the one a
      renderer is likelier to write — which is the call site your severity
      argument is about.

      It also asserts the fixture renders something non-empty first. Without that,
      your exact mutation would be caught, but a hypothetical one that emptied
      *both* `render()` and `fmt` would satisfy `to_string() == render()` with two
      empty strings — the two-explanations-one-answer shape this repo keeps
      finding, and the reason I did not write the single equality on its own.

      **Proven to fail before the fix**, in your mutation rather than a proxy for
      it: with `fmt`'s body replaced by `Ok(())` (and the parameter renamed to
      `_f`), it fails at the first seed —
      `left: "", right: "expandible hymnos of limyra"`. Restored; the suite goes
      962 → **963 passing, 0 failed**.

      Your latency point is recorded in the test's comment rather than only here:
      it is latent because no production caller reaches the derivation at all, it
      goes live with `key-identity-sweep`, and that is exactly the wrong moment to
      discover it. The comment also names `cargo mutants`' result (1 missed of 26)
      as the provenance, so the next reader knows this test exists because a
      mutation found a hole rather than because someone liked the symmetry.

## What was measured and found clean

**The reduction arithmetic agrees between the two languages far beyond the two
pinned cases — this was the open question, and the answer is that it holds.**
I transcribed `DKeyNameWindow.qml`'s index path into Rust *as written*,
including its `/^[a-z]+:/i` prefix strip, its `/[^0-9a-f]/gi` filter, its 65-character
zero-pad and its `.slice(0, 64)`, and differentially compared it against
`name_from_key_bytes` over keys nobody pinned. **200,000 pseudo-random 32-byte
keys: 0 mismatches.** The QML duplicate is a faithful reimplementation, not a
coincidence that happens to agree on two fixtures.

**Boundary keys agree too.** All-zero, all-`0xff`, and constructed keys whose
draws land exactly on a list boundary — `(0,0,0)`, `(8191,1023,1023)`,
`(8192,1024,1024)`, `(65535,65535,65535)`, `(8192,1024,0)`, `(16384,2048,2048)`
— **8 boundary keys, 0 mismatches**. A draw landing exactly on the modulus
wraps identically on both sides.

**The six pinned indices are correct, verified by hand against neither
implementation.** Reading the pinned key hex directly: key A byte 18/19 =
`0x76ae` = 30382, 30382 − 24576 = **5806**; bytes 20/21 = `0xbebe` = 48830,
48830 − 48128 = **702**; bytes 22/23 = `0x7b92` = 31634, 31634 − 30720 = **914**.
Key B: `0xef1a` = 61210 − 57344 = **3866**; `0x06ad` = 1709 − 1024 = **685**;
`0xa66d` = 42605 − 41984 = **621**. All six match both `names.rs`'s `PINNED_CASES`
and `tst_identicon.qml`'s `pinnedCases`. Wordlist sizes confirmed by
`grep -c ""`: 8192 / 1024 / 1024.

**A window slip is caught redundantly, not only by the pin.** Moving the QML
adjective draw from byte 18 to byte 17 fails **four independent tests** —
`test_the_name_window_agrees_with_cores_pinned_case`,
`test_the_byte_probes_find_the_windows_they_should`,
`test_the_three_channels_read_pairwise_disjoint_bytes` and
`test_no_byte_on_screen_reaches_the_mark_or_the_name`. The brief's concern that
the cross-language pin is the *sole* gate holds for modulus drift and byte
order, but not for the window: a slip that moves onto an abbreviation byte is
caught by the disjointness probes as well, because the probe measures rather
than restates.

**Malformed input handling is sound on both sides, and asymmetric on purpose.**
Core refuses: `display_name_from_bytes` is exercised at lengths 0, 1, 31, 33, 64
and 1024, on a non-decompressable point, and on the all-zero point (refused as
`KeyError::WeakPublicKey`), plus a 0..70 length sweep asserting no abort. There
is no padding route to a name — and the fixture proving it was chosen
deliberately, since zero-padding an ASCII string is refused at the parse anyway
and would have proved nothing. QML pads instead of refusing, which is correct
for what it is: an internal gate fixture that computes indices and never renders
a name, whose in-range obligation is asserted by
`test_a_malformed_key_still_yields_indices_in_range`. Both QML channels use a
byte-identical normalisation expression, so the "identical in shape to
Identicon's `_body`" comment is true as written.

**The intermediate state is intact — nothing half-deletes.** A three-dot
diffstat against `origin/main` shows `identity.rs`, `op.rs`, `wire.rs` and
`feed.rs` **byte-identical to main** (empty diffstat for all four).
`identity.rs::address()` survives at line 266, `FeedScreen.qml` still passes
`row.modelData.author` (at line 600 — the line moved from the brief's 323, the
field did not), and `NAME_PREFIX` / `name_digest` survive only in prose,
archived specs and findings files, with no live code reference anywhere.

**No reachable panic on any peer-controlled path.** `name_from_key_bytes` has
one `.expect`, on `key_bytes[name_key_bytes()].try_into()` — a fixed 6-byte
slice of a fixed 32-byte array, unreachable by construction and not
data-dependent. The three reductions cannot overflow: `u16 % u16` over list
lengths that divide 65,536. Indexing is bounded by the modulus. QML's `_draw`
peaks at 65535, well inside JavaScript's exact-integer range.

## What a gate here cannot see

**No production caller reaches the derivation at all.** Every reference to
`display_name` in `feed.rs` (line 694) and `wire.rs` (5762) is inside
`#[cfg(test)]` — `feed.rs`'s `mod tests` opens at line 292. The only
non-test `display_name` hits in the tree are the `metadata.json` module title,
which is an unrelated field. So the whole suite's green on this scheme is green
about code no peer bytes currently reach; that is consistent with the piece's own
statement that the feed path cannot render a name until `key-identity-sweep`
lands, but it means the derivation's real exposure is untested by construction
and arrives with that later piece.

**`cargo mutants` cannot see a changed constant.** It mutates function bodies,
so the moduli `8192`/`1024`, the wordlist contents, `CONNECTOR`, and
`NAME_KEY_BYTES`-equivalent literals are all invisible to it. Its clean report
on 15 caught mutants says nothing about those; what covers them is
`every_wordlist_is_pinned_entry_by_entry_and_in_order` (SHA-256 over each list)
and the written-down name pins, both of which are independently produced by
`examples/pin_name.rs`.

**The known modulus-drift result is not re-derived here.** The runner reports
`% 8192 → % 8000` in `DKeyNameWindow.qml` was already run twice: caught by the
cross-language pin alone, with every disjointness test green. My window-slip
measurement above is the complementary case and is not the same mutation.

**The shared 7-value probe set remains a real blind spot**, already filed by the
spec-test reviewer; I confirmed the two probe sets are the same seven values
(`PROBES` in `names.rs:842`, `probeValues` at `tst_identicon.qml:225`) but did
not re-derive that finding, as it belongs to that dimension.

**`cargo fmt --check` does not follow path dependencies into `dialectica-core`**,
so the crate holding this entire scheme is not format-checked by that gate; and
`cargo test` compiles nothing behind `#[cfg(logos_scaffold)]`.

## Not reached

`cargo mutants` was run on `names.rs` only. The QML side has no mutation
harness, so `DKeyNameWindow.qml` was probed by hand mutation (two cases) rather
than exhaustively.
