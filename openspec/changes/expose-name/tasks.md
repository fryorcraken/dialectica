# Tasks — expose-name

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## Implementation

The ordering is: the handler first, because everything else forwards to it; the
sweep-list entries next, because two of them are trip-wires that go red the
moment the trait declares the method; the trait and adapter last, since the
trait declaration is what the trip-wire reads.

### The handler

- [x] `display_name` in `dialectica-core/src/wire.rs`: read `publicKey`, bound
      the hex string before decoding, decode, hand the bytes to
      `names::display_name_from_bytes`, render.
- [x] Four distinguishable refusals — missing field, wrong type, bad hex, and
      the identity layer's own words for key material it refuses (design §4).
      The last defers to `NameError`'s `Display` rather than restating it, so
      the low-order refusal and the not-a-key refusal read differently without
      this file enumerating either.
- [x] Reply carries `name` and `words` (design §5). No gloss field.

### The dispatch surface

- [x] `DialecticaModule::display_name` declared in `dialectica/rust-lib/src/lib.rs`.
- [x] Forwarded from the `#[cfg(logos_scaffold)]` impl — a one-line body, since
      the handler needs no state.

### The three hand-maintained sweep lists

Each is an obligation rather than a courtesy; two have trip-wires that fail
naming the method, and meeting them deliberately rather than being told is the
point.

- [x] `every_request_taking_method()` — gains `("display_name", display_name_m)`.
      Without it `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      fails naming the method.
- [x] `every_method_with_a_required_field()` — inherited automatically, since it
      filters `every_request_taking_method()` and `publicKey` is required. No
      edit needed; recorded here so the omission is visibly deliberate rather
      than forgotten.
- [x] `a_served_request()` — gains a `display_name` arm supplying a valid key,
      or its catch-all panics naming the method.

### Tests

- [x] A name is obtainable for a supplied key, and it is **the derivation's**
      name for that key — asserted against `names::display_name`, so the entry
      point provably reaches this derivation rather than a second one.
- [x] Pinned against a **hardcoded** name, not against what the code just
      produced: the wire method's answer for a written-down key is a written-down
      string.
- [x] Determinism across calls, and independence between calls.
- [x] A key belonging to no known identity still names.
- [x] Wrong-length key material is refused.
- [x] **A low-order point is refused rather than named** — the case a length
      check misses.
- [x] The entry point's verdict matches `PublicKey::from_bytes` over a corpus
      spanning valid keys, wrong lengths, non-points and the low-order point.
- [x] Absent and malformed key material carry **different** messages.
- [x] No refusal carries a name or a placeholder.
- [x] Arbitrary byte strings do not abort the process, and a later well-formed
      call still answers.
- [x] Many distinct well-formed keys all succeed — no second failure condition.

### Not done, by owner instruction

- **`docs/UI-BRIEF.md` is deliberately untouched.** The original brief asked for
  obligation 6 to be corrected in this change, per CLAUDE.md's same-change rule.
  The owner overrode that mid-task: the file's content is ruled misleading, it is
  being deleted by another piece, and it is not to be edited, cited, or treated
  as a requirement. No edit was made to it, and the citations that had been
  written into `design.md` were removed and re-grounded in
  `openspec/specs/generated-names/spec.md`, which is the authoritative contract.
