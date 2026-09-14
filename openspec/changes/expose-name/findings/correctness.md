# Correctness review — expose-name

Dimension: **correctness** only. Security, readability and architecture are held
by other instances.

Baseline measured in this worktree: **970 tests pass** (940 `dialectica-core`
lib + 30 `end_to_end`), clean tree, `origin/main` at `6eec84f`,
`piece/expose-name` at `9be0329`.

## Findings

- [ ] **`dev-writer`** — `dialectica-core/src/wire.rs:2823` — the handler reads
      `publicKey` but nothing stops a sibling field being read as key material,
      and no test can see it. The spec's requirement *"An author address SHALL
      NOT be required, in addition to or in place of the key"* is asserted only
      against an address placed **in the `publicKey` field**, never against a
      separate field.
      **Scenario:** change the field read to
      `parsed.get("publicKey").or_else(|| parsed.get("authorAddress"))`. A caller
      sending `{"authorAddress":"<64 hex>"}` and no `publicKey` is then answered
      with a name derived from the address bytes — the exact "a caller holding
      only an address arrives at a name" outcome the requirement forbids, and the
      name is attributable to nobody.
      **Measured:** **970 of 970 tests pass under this mutation**, including
      `a_public_key_alone_is_enough_with_no_address_supplied`, the test written
      for this requirement. Severity: **moderate** — not reachable in the shipped
      code, but the requirement is currently unpinned, so the next edit to the
      field read is unguarded.

- [ ] **`dev-writer`** — `dialectica-core/src/wire.rs:2830` —
      `MAX_PUBLIC_KEY_HEX_CHARS` is set to the key's **exact** hex length (64),
      so it stops being an allocation bound and starts deciding messages for
      inputs that are not oversized. `design.md` §3 claims it "bounds the hex
      string before allocating and decides nothing about validity"; at an exact
      bound, 65 characters is the *ordinary near-miss*, not an attack.
      **Scenario:** a caller forwards a key with one stray character — a trailing
      space, a leading space, or a `0x` prefix, all of which a view can produce by
      concatenation. Measured replies:
      `" 91a2…4b3a"` → `{"error":"publicKey is 65 hex characters, over the 64 a
      public key holds"}`; `"0x91a2…4b3a"` → `"…is 66 hex characters, over the
      64…"`. The caller is told its key is **too long** when its key is the right
      length and merely dirty. Contrast the cited precedent `genesis_for`
      (`wire.rs:1252`), whose bound is `MAX_CANONICAL_BYTES * 2` — a genuine
      maximum with headroom, so a near-miss there still reaches `not valid hex`.
      Severity: **minor** — a misleading refusal, not a wrong name.

- [ ] **`tester`** — `dialectica-core/src/wire.rs:12770-12790` — the second half
      of `a_public_key_alone_is_enough_with_no_address_supplied` is **vacuous for
      its fixture**, confirming the author's own doubt. It feeds
      `key.address().to_hex()` where a key goes and asserts
      `assert_ne!(from_address["name"], Some(PINNED_NAME))`.
      **Scenario:** for `PINNED_KEY_HEX`, the address is
      `36af7f2124409149a2e0d483308e8c5560835dea3bd1f093058e9f7ef7c59614`, which is
      **not a valid curve point** — measured reply
      `{"error":"cannot derive a display name: not a valid public key"}`. So
      `from_address["name"].as_str()` is `None`, and the assertion reduces to
      `None != Some(name)`, which holds for any implementation that refuses
      anything at all. The branch the comment says it is aiming at — an address
      that *parses* and names something wrong — is never taken.
      **Measured:** this is not a rare fixture. Over seeds 1..=200,
      **111 of 200 author addresses parse as valid public keys** (55%). So the
      case is common and the test simply drew one of the 45% that refuse. Pick a
      seed whose address parses, and the assertion becomes a real comparison of
      two names. Severity: **moderate** — the test names a requirement it does
      not exercise.

## What was clean

The author's two flagged claims both **hold under measurement**.

*The refusal genuinely defers to the identity layer.* The chain is
`wire::display_name` → `names::display_name_from_bytes` (`names.rs:259`) →
`PublicKey::from_bytes` (`identity.rs:236`), with `NameError`'s `Display`
(`names.rs:221`) interpolating `KeyError`'s verbatim. No list of shapes exists in
`wire.rs` that could drift. I fed **all eight** low-order points, not just the
all-zero one the test uses — `0000…00`, `0100…00`, `0000…80`, `0100…80`,
`ecff…7f`, `ecff…ff`, `26e8…05`, `c717…fa` — and every one returned
`{"error":"cannot derive a display name: low-order public key, which can never
verify a signature"}`, distinct from the `not a valid public key` wording. The
"admits exactly what the identity layer admits" claim is true by construction,
including for the low-order case.

*No further `cfg(logos_scaffold)` compile gap of the flagged class.* I
enumerated every `core::…` reference inside the gated adapter (35 paths) and
checked each against `dialectica-core`'s root re-export list (`lib.rs:59-67`) and
its module set. All resolve. The only bare `core::display_name` in the file is at
line 913, **inside a doc comment**, not code — the real call at line 920 is
correctly spelled `core::wire::display_name`. I could not run the gated compile
directly (the cfg is driven by `generated/provider_gen.rs`, which no local
checkout stages, and `RUSTFLAGS=` is a blocked command shape), so this is a
static enumeration rather than a compile; `nix build .#lgx` remains the only
authority.

*Mutation coverage on the handler is complete.* `cargo mutants` on
`dialectica-core/src/wire.rs` generates 5 mutants for `display_name` — two
return-value replacements at `2813` and three comparison-operator swaps
(`>` → `==`, `<`, `>=`) at `2830`. **All 5 caught**, in two runs of ~2 minutes
each. The `>=` catch in particular means the hex bound is pinned from both sides.

*The sweep trip-wire works and names the method.* Removing
`("display_name", display_name_m)` from `every_request_taking_method()` makes
`the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares` fail
with `these methods are on the dispatch surface and are NOT swept for the request
envelope: ["display_name"]` — verified, not assumed.
`every_method_with_a_required_field()` needs no edit because it filters the first
list, which `tasks.md:52` records correctly (the proposal's "three lists must
gain the new method" overstates it; two is right, and the code is right).

*Decoder edges behave.* Odd-length hex (63 chars) → `not valid hex`; 64 chars of
non-hex → `not valid hex`; uppercase hex names the same key as lowercase, which
is correct since hex is case-insensitive and `hex::decode` accepts both.
Determinism, independence between calls, the no-known-identity case and the
arbitrary-bytes panic sweep all behave as specified.
