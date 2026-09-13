# Correctness findings — `publish-envelope`

Reviewed at `35fc859`, in a worktree of my own. Dimension: **correctness**
(a sibling file carries security). Suite run green at 736 + 26 before any
mutation; every mutation below was confirmed to land before its result was
believed, and the tree was mutated only in my own worktree, which is deleted.

- [ ] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:511` — the adapter's
      own pre-parse shadows all three envelope fixes, so none of them reaches
      the shipped module
      **Scenario:** `publishing()` still opens with a bare
      `serde_json::from_str(request)` at line 511 and its own `parsed.get("stoa")`
      ladder at 515-522, *before* `handler(request, …)` at 573 hands the same
      bytes to `core::publish_post`. So on the real wire:
      (a) `[]` is answered `{"error":"missing field: stoa"}` at line 521 and never
      reaches `REQUEST_NOT_AN_OBJECT` — the exact defect the proposal says is
      fixed; (b) an N-byte request is fully parsed at ~2N transient heap at line
      511 before `Request::parse`'s `request.len() > MAX_REQUEST_BYTES` check is
      ever evaluated, so the cap bounds only a *second* parse of bytes already
      paid for, and the PHASE0-FINDINGS §3 abort it exists to prevent is
      unchanged; (c) `invalid JSON` is still worded by the adapter at 513.
      **Measured:** `dialectica-core` is fully correct in isolation — reverting
      `PublishRequest::parse` to the pre-fix parse turns 3 of 736 red
      (`a_request_that_is_not_an_object_is_refused_for_its_shape`,
      `every_request_taking_method_refuses_an_oversized_request`,
      `the_three_refusals_a_caller_can_earn_are_three_different_messages`), each
      naming `publish_post` and the exact message pair. But no test in this repo
      calls `Dialectica::publishing`, because `lib.rs` is `cfg(logos_scaffold)`,
      so all three go green while the shipped behaviour is the old behaviour.
      **Severity: high.** The piece's headline claim is true of the crate and
      false of the module. The fix is to delete lines 510-522 and have the
      adapter read the Stoa through `core` (a `core::wire::stoa_of(request)`
      returning `Result<Address, String>` built on `Request::parse`), so the
      envelope is crossed once.

- [ ] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:536` — "validate, then
      unlock" is stated as the reshape's purpose and does not hold on any path
      **Scenario:** `design.md` §1 and the refactor commit message both say the
      64 MiB Argon2id unlock running before validation is the defect the
      prologue type exists to make fixable, and the review brief asks that the
      new order hold on every path. It holds on none. `open_from_env` is still
      called at line 536, before `handler` at 573, so
      `{"stoa":"<valid hex>","author":"x"}` (forbidden field),
      `{"stoa":"<valid hex>"}` (missing `body`), and a 5 MiB oversized request
      each pay a full key derivation before the guard that refuses them runs.
      Only a malformed or missing `stoa` is refused early, which was already
      true before this change.
      **Measured:** by reading the call order at lines 511→536→573; no gate can
      execute it. **Severity: medium** (a DoS amplifier rather than a wrong
      answer — an attacker who can reach `publishPost` gets a 64 MiB Argon2id
      per malformed request). Note this is *not* a defect the change introduced;
      it is one the change says it fixes and does not. If it is deferred, say so
      in `design.md` rather than leaving §1 claiming otherwise.

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
      the sweep parser is evaded by a wrapped signature, silently
      **Scenario:** `the_dispatch_traits_request_taking_methods` matches
      `signature.starts_with("&mut self, request: String) -> String;")` on a
      single `line`. Declaring the fourth publish operation as rustfmt would
      emit it once the signature passes 100 columns —
      ```
      fn publish_moderation(
          &mut self,
          request: String,
      ) -> String;
      ```
      — makes the method invisible to the parser. I added exactly that to the
      trait and ran the gate.
      **Measured:** `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      **passed** with `publish_moderation` declared on the dispatch surface and
      absent from `every_request_taking_method` — the precise failure this test
      was written to make impossible, for the precise method `design.md` §1
      names as the reason the reshape happened. The `!found.is_empty()`
      backstop does not help: the other fourteen still parse, so `found` is
      non-empty and the assertion is silent.
      **Severity: high** — a gate whose parser can be evaded without anyone
      noticing is worse than the hand-maintained list it replaced, because the
      list's doc at least said "Nothing checks it". Fix: normalise whitespace
      across the whole trait body before matching (collapse the body to one
      line per `fn … ;` by splitting on `;` rather than on `\n`).

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
      the sweep parser is also evaded by renaming the parameter
      **Scenario:** the match is on the literal text `request: String`. A method
      declared `fn publish_moderation(&mut self, req: String) -> String;` is a
      byte-for-byte identical dispatch surface — the parameter name has no
      compiler or codegen consequence — and is not in `found`.
      **Measured:** I made that one-word edit and the sweep test **passed**,
      with the method unswept. This is a separate box from the wrapped
      signature because a fix for one (splitting on `;`) does not fix the other;
      this one needs the type matched rather than the name, e.g. a check for
      `String) -> String` with the parameter name unconstrained.
      **Severity: medium** — less likely than a rustfmt wrap, but it is the
      cheaper of the two to fix and the same class.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
      the parser's own failure mode is undetectable from its assertion text
      **Scenario:** the three evasions above all leave `found` non-empty, so the
      `!found.is_empty()` guard — whose message says "the signature shape this
      test reads by has changed, so it is now measuring nothing rather than
      failing" — never fires. It only catches a *total* change of shape, not a
      partial one, which is the shape that actually happens (one method written
      differently from the other fourteen).
      **Measured:** both evasions above ran with the guard in place and green.
      **Severity: low** as a defect, but it is the reason the two above are
      silent rather than loud, so it wants a comment correction at minimum: the
      guard does not protect against what its message claims.

## What I verified and found clean

**The refactor commit `4c30aaa` is genuinely behaviour-preserving.** I read it
statement by statement against the three handlers it replaced. The order inside
`PublishRequest::parse` (parse → `reject_forbidden_fields` → `required_stoa`) is
the order all three handlers had; `refused()` is textually
`error_json(&refusal.to_string())`, the same expression the three `Err` arms
carried; `publishing` wraps `guarded(method, …)` at the same scope, and
`delivered_and_published` is reached on the same `Ok` arm. Per-field read order
inside each closure is preserved (`parent` then `body`; `target` then
`direction`), which matters because the first failing read is the message the
caller gets. Corroborating measurement: reverting only the *fix* commit's
envelope line leaves **733** of 736 passing — exactly the count the refactor
commit claims it was green at, so the three tests the fix adds are the only ones
whose outcome the two commits differ on.

**The signing-key fix is right, not merely test-passing.** `publishing_key` is a
structural mirror of `posting_identity` — same `paths.path_for(stoa)`, same three
arms, same `NO_CHOICE_FOR_THIS_STOA` — and `stoa_address_at_path` is defined as
`stoa_key_at_path(..).public_key().address()` (`keystore.rs:859-872`), so the two
agree by construction rather than by coincidence. There is no second path: all
three trait methods route through the single `Dialectica::publishing`
(`lib.rs:790,794,798`), which has one `let key = …`. Mutating `publishing_key`
back to `keystore.stoa_key(stoa)` turns
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports` red naming
`656c6003a040…` against probe `fb045664ae4d…` — the mutation landed and was
caught, so the regression test is real.

**The deleted CI exemption is safe.** `stoa_key` survives in `lib.rs` only at
lines 544 and 665, both inside `//` comments that the gate strips with
`re.sub(r"^\s*//.*$", …)` before the ban regex runs; the only live accessor call
is `core::keystore::creator_key_in` at 711, which is required rather than
banned. All three `want` strings the gate requires are present. The gate passes,
and it passes for the right reason.

**The "null tests passed on the unfixed code" claim is true and honest.** With
the envelope fix reverted, `one_field_has_one_null_reading` stayed green — so
they are coverage, as `design.md` says, not regression tests. They are not
vacuous either: adding a `Some(Value::Null) => missing field` arm to
`required_string` turns the test red naming `body`, so the property they assert
is one the code could fail.

**`cargo mutants`** was not run — the changed surface is three closures and one
four-line function whose mutants I exercised by hand above, and the run would
have cost more than it returned on a 736-test suite.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is behind `cfg(logos_scaffold)` and is compiled
by nothing I can run, so findings 1 and 2 are from reading the call order rather
than from executing it. Only Build LGX compiles that half, and Build LGX checks
that it compiles, not what order it runs in — which is why both defects are
invisible to every green gate on this PR.
