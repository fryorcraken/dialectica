# `expose-name` — security review

Reviewed dimension: **security only**. Correctness, readability and architecture
are held by other instances; an unticked row for those still means nobody has
done them.

Scope: `wire::display_name` and the allocation bound beside it
(`dialectica-core/src/wire.rs`), the adapter forward
(`rust-lib/src/lib.rs`), and the derivation it reaches
(`names::display_name_from_bytes`).

Everything below was measured in a throwaway worktree, which has since been
removed. The branch under review was left untouched.

---

- [ ] **`tester`** — `dialectica-core/src/wire.rs:2828-2839` — the hex bound's
      **ordering** is unpinned, so the one mutation that reintroduces the DoS
      survives the whole suite
      **Scenario:** swap the `if hex_str.len() > MAX_PUBLIC_KEY_HEX_CHARS`
      block with the `hex::decode` block below it — exactly the reversal
      `MAX_PUBLIC_KEY_HEX_CHARS`' own doc says the bound exists to prevent
      ("`hex::decode` allocates `len / 2` bytes from a length the caller
      chose"). A caller then sends
      `{"publicKey":"<4,194,272 chars of valid hex>"}`, under
      `MAX_REQUEST_BYTES`, and the handler builds a **2,097,136-byte `Vec`**
      before refusing it for being over 64 characters.
      **Measured:** **940 of 940 tests pass** under the reversal
      (`cargo test -p dialectica-core --lib`, the branch's full count with
      nothing added). Release-build cost of that one
      request: **39.56 ms reversed vs 2.32 ms correct — 17x**, plus the 2 MiB
      transient allocation, repeatable per call with no reply signal that
      anything was spent. Per PHASE0-FINDINGS §3 an allocation failure in a
      dispatch handler is a module **abort**, not an error reply.
      **Why the existing test does not see it:**
      `the_hex_bound_refuses_an_oversized_key_without_deciding_validity`
      (`wire.rs:13154`) feeds `"ab".repeat(100_000)` — **all valid hex**, so
      `hex::decode` succeeds and the size refusal is still the message that
      comes back after the decode. The test asserts the *message*, which the
      ordering does not change.
      **What would catch it** is the shape `request.rs` already uses one layer
      up: `an_oversized_request_is_refused_before_it_is_parsed` (`request.rs:311`)
      feeds input that is **both oversized and unparseable** and asserts *which*
      refusal comes back, because that is the only thing a return value can
      show about ordering. The analogue here is an oversized `publicKey` that is
      **not valid hex** — e.g. `"zz".repeat(100_000)` — asserting the reply says
      `"over the"` and does **not** say `"not valid hex"`. Measured: that
      assertion is red under the reversal and green on the branch as written.
      **Severity: high** — it is the only defect found, it is the exact property
      the code was written to have, and the code is correct today; what is
      missing is the test that keeps it correct.
      *(Genuine gap, not a style preference.)*

- [ ] **`tester`** — `dialectica-core/src/wire.rs:12857` — the wrong-length
      sweep's `64` entry is refused by the allocation bound, not by the identity
      layer, so the comment above it is false for that entry
      **Scenario:** `key_material_of_the_wrong_length_is_refused` sweeps
      `[0, 1, 31, 33, 64]` bytes, and its comment says "in hex so the decode
      succeeds and the **LENGTH is what the identity layer objects to**". 64
      bytes is 128 hex characters, which is over `MAX_PUBLIC_KEY_HEX_CHARS`, so
      that entry never reaches `PublicKey::from_bytes` at all — it is refused by
      the size bound. **Measured:**
      `display_name(r#"{"publicKey":"<128 chars>"}"#)` returns
      `{"error":"publicKey is 128 hex characters, over the 64 a public key holds"}`,
      never `"cannot derive a display name"`.
      The assertion still passes, so nothing is broken today; what is wrong is
      that the sweep believes it is exercising a path it is not, which is the
      "two explanations give the same answer" fixture family this project has
      recorded. Either drop `64` from this sweep (it is covered by the bound's
      own test) or keep it and correct the comment to say which layer refuses it.
      **Severity: low** — a test that proves less than it claims, not a defect
      in the shipped code.
      *(Genuine gap in what a test measures, not a style preference.)*

- [ ] **`dev-writer`** — `dialectica-core/src/wire.rs:2831-2834` — the size
      refusal reports a **byte** count while calling them "hex characters"
      **Scenario:** `{"publicKey":"<64 × 'é'>"}` — 64 characters, 128 bytes —
      answers `{"error":"publicKey is 128 hex characters, over the 64 a public
      key holds"}`. **Measured**, verbatim. `str::len()` is bytes; the message
      says characters. No security consequence — the refusal is correct, the
      bound is still a sound allocation bound (bytes ≥ characters, so it never
      under-bounds), and nothing attacker-useful leaks, since the caller already
      knows what it sent. But this is contract surface a view renders, and a
      message that states a different quantity from the one it measured is the
      kind of thing someone later "fixes" by relaxing the bound to count
      characters — which would make it a *character* bound on a *byte*
      allocation and reopen the finding above at 3x. Either say "bytes", or say
      "the request's `publicKey` is over the N bytes a public key's hex holds".
      **Severity: low.**
      *(Genuine inaccuracy in contract surface, not a style preference.)*

---

## What was probed and found clean

**No input reaches a panic.** I swept the handler with embedded nulls, invalid
and multibyte UTF-8 (`é` at 32, 63 and 64 repetitions, straddling the
byte/character boundary of the bound), non-hex, odd-length hex, `0x` prefixes,
leading whitespace and newlines, empty strings, wrong-typed `publicKey` values
(number, `null`, array, object, boolean), a 100-deep nested object, duplicate
`publicKey` keys, `\u` escapes forming ASCII hex, every length from 0 to 70
bytes at four fill values, and a `publicKey` of 4,194,272 characters inside a
request 16 bytes under `MAX_REQUEST_BYTES`. **Every one returned a JSON object
with exactly one of `error` or `name`, and none panicked, hung or allocated
beyond the bound.** The maximal request costs 2.32 ms release-build, dominated
by the envelope's own `serde_json` parse rather than by anything this handler
does.

**The panic guard is in the right place.** `wire::display_name` opens with
`guarded("display_name", …)`, and the adapter
(`rust-lib/src/lib.rs:919`) is a bare forward into it — one guard frame, no
unguarded path, and no second `catch_unwind` to nest. The derivation below it
cannot panic either: `name_from_digest` slices at `NAME_DIGEST_BOUND` rather
than indexing past it, and all three word draws are `%` against power-of-two
list lengths, so every index is in range by construction.

**Validation is at the boundary and in the right order.** `Request::parse`
refuses over `MAX_REQUEST_BYTES` *before* `serde_json::from_str`, and refuses
non-objects, so the handler never sees an array masquerading as a fieldless
request. The handler's own bound then runs before `hex::decode`. Nothing
downstream re-decides validity: `PublicKey::from_bytes` is the sole authority,
reached via `&[u8; 32]` (a wrong-length slice is a `TryFrom` failure, not a
panic — unlike the `k256` alternative `Cargo.toml` records rejecting), and it
refuses low-order points, which I confirmed the all-zero point hits.

**No oracle, and no timing side channel.** The derivation reads no keystore, no
op log, no membership store and no session — verified by the adapter's one-line
forward and by `obtaining_a_name_changes_nothing_that_a_later_call_answers`.
A key belonging to no identity this peer has seen names identically to one it
holds, so the method cannot be used to probe what a peer knows. Refusal
messages distinguish only *caller* mistakes (absent field, wrong type, bad hex,
oversize, unparseable key material) — categories the caller already knows,
carrying nothing about local state. There is no secret to compare, so
constant-time comparison does not arise.

**No error message leaks internal detail.** Every refusal goes through
`error_json`, which escapes via `serde_json::json!`; the messages name only the
field, the caller's own length, and the identity layer's verdict. No paths, no
addresses, no key bytes. `guarded`'s panic message would carry a panic string,
but no input I found reaches it.

**The name does not travel, and this method does not weaken that.**
`grep displayName` across `rust-lib/` finds it only in comments recording that
an earlier pass added one and the owner reversed it; the feed row's key set is
asserted *exactly* (`wire.rs:5856`), so a restored `displayName` fails rather
than passing quietly. `no_method_accepts_a_display_name_where_an_identity_is_required`
is **not breached**: `display_name` takes key material and returns a name — the
opposite direction — and it accepts nothing where an identity is required.
The name is also not reversible to a key: it is
`SHA256(NAME_PREFIX || public_key)` truncated to 6 bytes over a 2^33 space, so
the map is massively many-to-one by design, and the piece adds no way to ask
"which key has this name".

**No injection surface in the reply.** All 10,240 wordlist entries are pure
lowercase ASCII and spaces (`grep -n "[^a-z ]"` across `wordlists/` matches
nothing), pinned by `every_entry_of_every_list_is_ascii_lowercase_and_well_formed`,
and the reply is built with `serde_json::json!` rather than string
concatenation.

**No new dependencies.** The diff adds none to either `Cargo.toml`; the licence
and maintenance questions do not arise.

**One surface-wide property, noted but not a box for this change.** Duplicate
JSON keys are last-wins (`{"publicKey":"<zeros>","publicKey":"<real key>"}` is
served with the real key's name — measured). That is `serde_json`'s behaviour
at `Request::parse`, shared by every request-taking method and documented
nowhere on the surface. It predates this piece and is not made worse by it, so
it belongs to the envelope rather than to `expose-name`.

**Also noted:** uppercase hex is accepted and reaches the same name as its
lowercase spelling (measured). Correct — `hex::decode` is case-insensitive, the
method takes a key rather than an identifier, and the reply is the canonical
name either way. Not a finding.

## CI

The new method is registered in both hand-maintained sweep lists —
`every_request_taking_method` (`wire.rs:9323`) and the served-request table
(`wire.rs:9919`) — so the envelope sweeps cover it rather than passing green
around it. No file was moved or renamed, so no CI gate is left measuring a
directory that no longer holds tests.

## Worktree

Work was done in `.claude/worktrees/review-expose-security`, which was removed
with `git worktree remove --force` after this file was committed and
cherry-picked. The branch under review carries none of the mutations above.
