# Architecture review — `expose-name`

Dimension: **architecture** only. Correctness, security and readability are held
by other instances.

Verified in a worktree of `piece/expose-name` at `9be0329`. Suite green (940 +
30 tests). `cargo mutants` scoped to the new handler: 5 mutants, **5 caught**,
including all three boundary mutants on the hex bound's `>` comparison.

## Findings

- [ ] **`dev-writer`** — `design.md:69` — "There is no third shape" is refuted by
      `design.md:83` in the same decision, which names one
      **What is wrong:** Decision 1 rules the batch out by claiming its reply
      shape is *forbidden*, enumerating two shapes and asserting "There is no
      third shape." Fourteen lines later the same decision describes a third:
      *"`{"names":[...]}` where each entry is a name **or null**, with the
      refusal reason omitted entirely — a shape that is uniform per item and so
      not a partial success."* That shape is not forbidden, and the design says
      so itself.
      **Why it matters architecturally:** the decision's stated ground is
      "the batch is impossible", when the true ground is the much weaker and
      entirely sufficient "the batch is unnecessary until measured, and the
      per-item cost is the cheapest call on this surface." Someone revisiting
      this reads "forbidden" and stops; the honest record would send them to
      measure round trips instead. The **conclusion is right** — one key per
      call is the correct shape here — but the recorded reasoning overstates
      itself, which is the failure mode CLAUDE.md's "write down why a decision
      went the way it did" exists to prevent.
      **Scenario:** a future author hits a slow feed, reads decision 1, and
      concludes batching is barred by `module-wire-contract`. It is not:
      `openspec/specs/module-wire-contract/spec.md:271` forbids *"A reply SHALL
      NOT carry both an error and a result"* — a statement about the top-level
      reply envelope, which a uniform `name-or-null` list does not violate.
      **Fix:** restate decision 1's ground as "unnecessary and unmeasured", keep
      the two bad shapes as rejected alternatives, and delete "There is no third
      shape". **Severity: medium** (defect in the recorded decision, not in code).

- [ ] **`dev-writer`** — `wire.rs:2738-2740` — the doc claims the bound is
      derived from the key's size; it is two literals and can drift
      **What is wrong:** the doc states *"The bound is derived from the key's own
      size rather than written as `64`, so it cannot drift from the type it is
      bounding."* The code is `const MAX_PUBLIC_KEY_HEX_CHARS: usize = 32 * 2;`
      (`wire.rs:2758`). The `32` is a hardcoded literal, not a reference to
      `PublicKey`. Grepped `identity.rs`: there is **no** exported key-size
      constant — every site writes `[u8; 32]` inline — so nothing ties this
      constant to the type at all.
      **Scenario:** `PublicKey` changes representation. `32 * 2` does not follow,
      the comment still asserts it cannot drift, and a reader trusts the comment
      instead of checking. This is the "comment restating a guarantee the code
      does not provide" case — worse than no comment, because it closes the
      question.
      **Fix (either):** introduce `PublicKey::BYTE_LEN` in `identity.rs` and
      derive the bound from it, making the comment true; **or** keep `32 * 2`
      and correct the comment to say the literal is duplicated deliberately and
      what pins it. **Severity: low-medium** (documentation asserts a structural
      property the code lacks).

- [ ] **`tester`** — `wire.rs:2758` — the bound's *value* is unpinned; only its
      comparison operator is
      **What is wrong:** `cargo mutants` catches all three `>` mutants because
      the comparison is exercised, but mutants does not mutate `const` values —
      and no test pins the constant. So the number itself is free.
      **Measured:** I edited `MAX_PUBLIC_KEY_HEX_CHARS` from `32 * 2` to
      `4096 * 2` — a 128-fold loosening of the allocation bound — and
      **970 of 970 tests passed** (940 unit + 30 integration). Restored; tree
      verified clean.
      **Why it matters:** the bound's entire purpose (design §3) is bounding
      allocation *before* `hex::decode` allocates `len/2` from a caller-chosen
      length, because per PHASE0-FINDINGS §3 an allocation failure in a dispatch
      handler is a module **abort**. A silent loosening restores exactly the
      exposure the constant was added to close, with every gate green.
      **Fix:** one assertion that the bound equals 64 — hardcoded, not
      `assert_eq!(MAX_PUBLIC_KEY_HEX_CHARS, 32 * 2)` read back from the
      expression, which would move with it. `request.rs:339-344` already does
      exactly this for `MAX_REQUEST_BYTES` and records why; follow that pattern.
      **Severity: medium**.

- [ ] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:339` — `display_name` is
      a noun phrase among fifteen verb phrases, and collides with
      `metadata.json`'s existing `display_name`
      **What is wrong:** every other method on the dispatch trait is an
      imperative verb phrase or a question — `ping`, `get_capabilities`,
      `list_threads`, `read_thread`, `create_stoa`, `join_stoa`, `list_stoas`,
      `generate_identity_slate`, `keep_identity`, `who_am_i`, `publish_post`,
      `publish_reply`, `publish_vote`, `panic_probe`,
      `delivery_channel_exists`. `display_name` names the *return value*, not
      the action, and read as a verb it says "display the name" — which the
      method does not do; it derives one and displays nothing.
      **Second, concrete cost:** `dialectica/metadata.json:3` already binds the
      key `display_name` to the **module's** human label (`"Dialectica"`). Two
      different `display_name` keys now exist in one module's surface meaning
      unrelated things. Different namespaces, so no build collision — but a
      reader grepping `display_name` across the module gets both.
      **Scenario:** a UI author calls `modules().display_name(...)` expecting
      the module's label — the meaning `metadata.json` already established —
      and gets a key-derivation endpoint.
      **Note:** design §6 spends a decision on the Rust-path ambiguity between
      `wire::display_name` and `names::display_name`, resolving it with a longer
      import path. That ambiguity is a **symptom of this name**, and §6's own
      rejected alternative ("renaming the wire method would make the JSON method
      name and the Rust function name disagree") assumes the JSON name is
      fixed — but this change is what fixes it, so it is still free to choose.
      A verb-first name such as `derive_display_name` or `name_for_key` would
      dissolve the §6 problem entirely and let the handler be re-exported at the
      crate root like every other one.
      **Severity: medium** — cheap now, permanent once shipped; the core API is
      the deliverable and outlives any UI.

- [ ] **`dev-writer`** — `design.md:207-212` — the per-call cost is mitigated
      against an unstated bound; the number is 100
      **What is wrong:** the risk entry answers §2.4 with "a
      `clamp_per_page`-bounded row count" without saying what the bound is.
      Measured: `feed.rs:96` and `thread.rs:95` both set `MAX_PER_PAGE = 100`
      (`DEFAULT_PER_PAGE = 20`). So a worst-case screen is **100 IPC round trips
      for names alone**, and a typical one 20.
      **Why it matters:** PLAN.md is more pointed than the design's paraphrase
      allows. `docs/PLAN.md:2679-2681` says *"Every cross-module call is IPC
      (§2.4), and a feed is a loop. A method that answers one post per call
      turns a page of thirty into thirty IPC round trips. The paginated shape is
      not politeness; it is the only shape that works."* — framed on **round-trip
      count**, not per-call cost. The design's counter (§2.4 warns about
      *expensive* calls; this one is cheap) is a fair distinction and I think it
      holds, but it is a reframing of PLAN.md rather than a reading of it, and
      the design does not acknowledge that.
      **Fix:** state the 100/20 figures and note the reframing explicitly, so a
      future reader weighing a batch has the number and knows PLAN.md's wording
      points the other way. **Severity: low** (the decision stands; the record is
      thin at the point someone would revisit it).

## What was clean

**The API widening is shaped right**, and the questions I was asked to check on
it all came back positive:

- **Field spelling matches the emitters exactly.** `publicKey` in hex is what
  `Kept::to_json` (`wire.rs:759`), `who_am_i` (`wire.rs:1035`) and the slate
  candidates (`wire.rs:715`, `c.public_key.to_hex()`) already emit. A caller
  forwards the value it was handed, unmodified — verified, not taken on trust.
- **The envelope is inherited structurally, not reimplemented.** The handler's
  only route to a parsed request is `Request::parse`, so the non-object refusal,
  the 4 MiB cap (`request.rs:139`), the null readings and the panic guard come
  for free. No part of the envelope is restated.
- **The stateless signature is real.** `display_name(request: &str) -> String`
  is structurally identical to `ping`'s and takes no store, log or keystore —
  unlike `list_stoas(request, store)` or `who_am_i(...)`. The one-line adapter
  forward and the "no fixture needed" sweep entry both rest on a property the
  type signature enforces rather than one the prose asserts.
- **The sweep-list trap is genuinely closed.** `every_method_with_a_required_field()`
  (`wire.rs:9091`) *derives* from `every_request_taking_method()` by named
  exclusion rather than being a second hand-written list, and
  `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
  (`wire.rs:9627`) checks that list against the **dispatch trait source**. This
  is CLAUDE.md's "complexity in the data structure, not the logic" applied
  correctly, and the new method inherits it rather than adding a fourth guard.
- **`words` is load-bearing, not redundancy.** Verified rather than accepted:
  `wordlists/places.txt` contains **10 multi-word entries**, so
  `name.split(' ')` genuinely truncates a two-word toponym. `DisplayName`
  (`names.rs:154`) is three `&'static str` fields with the connector existing
  only in `render()`, so shipping both fields exposes the real structure.
- **Not re-exporting at the crate root is defensible and the precedent is
  real.** The adapter does already call `core::wire::get_capabilities_from_stores`
  (`lib.rs:764`) and `core::wire::publishing_key` (`lib.rs:647`). One nit, not
  worth a box: the comment at `dialectica-core/src/lib.rs:55-56` offers
  `get_capabilities_from_stores` as precedent for the long path, but that
  function *is* in the re-export list — so the adapter's long path there is a
  choice, not a necessity. The precedent holds via `publishing_key`, which is
  genuinely not re-exported.
- **`interface` is `cdylib`, not `universal`** (`metadata.json:6`), so
  CLAUDE.md's public-constructor dispatch-table trap does not apply here.
- **Design §6's build-gate claim is true.** I confirmed a local
  `cargo check --cfg logos_scaffold` cannot compile the adapter — it needs
  `generated/provider_gen.rs`, which only the scaffold build produces. So
  `nix build .#lgx` really is the only gate that sees the adapter, and that
  warning belongs where it is. **`dialectica-core` itself compiles and tests
  clean.**

**Scope discipline is good.** The change adds exactly one method and touches no
existing reply shape. *The name SHALL NOT travel* is preserved structurally —
the only route to a name is asking with a key in hand — and the design correctly
declines to specify a batch method it cannot yet justify, which is the right
call under "do not refactor speculatively".

**`docs/UI-BRIEF.md` is not a finding.** The proposal (line 122) obliges the
dev-writer to fix Obligation 6 in this change and the diff does not; `tasks.md`
lines 80-88 record the owner's mid-task override ruling the file misleading and
not to be edited or cited. Correctly handled and correctly documented.
