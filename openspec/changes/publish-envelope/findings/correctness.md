# Correctness findings — `publish-envelope`

Reviewed at `35fc859`, in a worktree of my own. Dimension: **correctness**
(a sibling file carries security). Suite run green at 736 + 26 before any
mutation; every mutation below was confirmed to land before its result was
believed, and the tree was mutated only in my own worktree, which is deleted.

- [x] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:511` — the adapter's
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

      **FIXED**, exactly as suggested. `core::stoa_of` (`wire.rs`) is
      `Request::parse` then `parse_stoa`, and the adapter's lines 510-522 are
      gone — it now calls `core::stoa_of(request)` and returns its `Err` arm
      unchanged. It reads the Stoa and nothing else: the forbidden-field guard
      and every required-field read stay the handler's, because an adapter
      validating twice is the two-readers-of-one-field shape that produced this.

      **The test that fails without it:**
      `the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does`.
      Reverting `stoa_of` to the adapter's old bare-parse shape turns it red
      with your exact defect:
      `left: "{\"error\":\"missing field: stoa\"}"` against
      `right: "{\"error\":\"the request must be a JSON object\"}"`. It also pins
      the size ordering the way `an_oversized_request_is_refused_before_it_is_
      parsed` does — an oversized *and* unparseable request must come back with
      the size refusal, which only a length check before `from_str` can answer.

      **And two CI changes, because one test cannot hold a file no test
      compiles.** The adapter-derivation gate now *requires* `core::stoa_of`,
      and — a new ban, for the shape rather than the instance — **fails on any
      `serde_json::from_str` in the adapter at all**. Run against `35fc859`,
      this piece's own previous commit, the gate fails on both counts. Your
      point that Build LGX proves the file compiles rather than what order it
      runs in is exactly why the gate had to grow rather than the test alone.

- [x] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:536` — "validate, then
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

      **DEFERRED**, to its own piece, and §1 is corrected rather than narrowed —
      which is the option you named and the right one.

      The argument is the shape of the fix, not its size. `handler` takes
      `&SecretKey`, so the key must exist before the handler runs and the
      handler is what validates. Reordering therefore means the three handlers
      taking a **fallible key supplier** instead of a key, which moves their
      signatures, the `Handler` type in
      `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`,
      every sweep fixture passing `&publish_key()`, and the adapter. That is a
      second reshape of the same three functions — and PLAN.md §9.2 already
      flags the supplier-closure trade-off as one to *"be judged on its own
      merits rather than as the price of testability"*, because `authoring`
      currently **cannot** create key material and a closure replaces "cannot"
      with "does not, and here is a test".

      **§1 no longer claims it.** It now says the ordering is unchanged on every
      path, names your two findings as having caught it, and points at
      `design.md` decision 8, which carries the deferral with its argument and
      says where it goes. `docs/PLAN.md` §9.2 is updated too — the reshape half
      is struck through as done, the unlock half is written out as outstanding
      with your `{"stoa":"…","author":"x"}` case — because a findings file is
      deleted at merge and this must outlive it.

      Your last sentence is the one I acted on most directly: the defect is not
      one this change introduced, it is one the change claimed to fix. The claim
      was the error.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
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

      **FIXED by the `dev-writer`'s classifier, and now MEASURED rather than
      taken on trust.** Your finding was filed at `35fc859` against the text
      filter; `d055c0c` replaced it with the classifier at
      `wire.rs:7938` (your citation `7541` has drifted — the function moved,
      the finding did not). The `spec-writer` declined to tick this without a
      measurement, which was the right call, so I re-ran your exact evasion
      rather than reading the new code and agreeing with it.

      **The mutation, and the observed failure.** I added your probe verbatim to
      the dispatch trait in `dialectica/rust-lib/src/lib.rs`:

      ```
      fn publish_moderation(
          &mut self,
          request: String,
      ) -> String;
      ```

      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      went **red**, naming the method:

      ```
      these methods are on the dispatch surface and are NOT swept for the
      request envelope: ["publish_moderation"]
      ```

      Predicted and observed agree: the classifier strips comments, joins the
      trait body into one line and splits on `fn ` rather than on `\n`, so the
      wrap is gone before anything is matched. The mutation was restored with
      `git checkout --` and `git status --porcelain` is empty.

      **What else could have produced that red, checked rather than assumed.**
      A test that failed merely because the trait changed would be worthless
      here. It did not: the failure names `publish_moderation` specifically and
      comes from the sweep-coverage assertion, not from the `unclassified` panic
      and not from a compile error. The rustfmt trailing comma — which the
      `dev-writer` records as having put an ordinary wrapped method in the
      unclassified bucket on the first cut — is handled, so this red is the one
      the box is about rather than a parser failing for the wrong reason.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
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

      **FIXED, and MEASURED separately from the box above** — which is your own
      point that one fix does not imply the other, honoured rather than assumed
      away. I declared exactly your probe:

      ```
      fn publish_moderation(&mut self, req: String) -> String;
      ```

      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      went **red** with the identical message:

      ```
      these methods are on the dispatch surface and are NOT swept for the
      request envelope: ["publish_moderation"]
      ```

      The classifier keys on the parameter's **type** and ignores its name
      entirely (`wire.rs:8018-8023`: `Some((_param_name, ty)) if ty.trim() ==
      "String"`), so `req: String` and `request: String` are the same dispatch
      surface to it, which is what they are to the compiler. Restored; tree
      clean.

      **And the third mutation neither of us named — the instrument's own
      input.** Pinning a filter from both sides is no use if what feeds it can
      be emptied, so I mutated the corpus-builder rather than the trait:
      `let normalised = String::new()`, which is the "filter runs over nothing"
      shape that has passed a full suite on a sibling branch here. It fails
      **loudly**, at `wire.rs:8045`:

      ```
      no request-taking method was found in the trait declaration — the
      signature shape this test reads by has changed, so it is now measuring
      nothing rather than failing
      ```

      So the `!found.is_empty()` backstop — which you correctly said could not
      catch a *partial* change of shape — does catch a total loss of the corpus,
      and the classifier's `unclassified` panic covers the partial case the
      backstop cannot. The two together are why all three mutations are red.
      Restored; tree clean.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` —
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

      **FIXED, and this is the box I acted on hardest**, because your diagnosis
      is the general one and the other two are its symptoms. *"It only catches a
      total change of shape, not a partial one"* is exactly right, and it made a
      comment correction the wrong fix: the defect is that **a filter's failure
      mode is silence**, so any patch that kept filtering would have left the
      next unanticipated shape just as quiet.

      So it no longer filters. `the_dispatch_traits_request_taking_methods` now
      **classifies every `fn` in the trait** into exactly one of three buckets —
      takes a request, takes none, takes something else — and the third bucket
      is a **panic naming the method and its parameters**. Both your evasions
      fall out of that one change rather than needing two patches: whitespace is
      normalised across the whole trait body before matching (the wrap), and the
      match is on the parameter's **type** rather than its name (the rename).

      **Measured on a shape neither of us had tried.** I ran a third probe,
      `fn publish_moderation(&mut self, request: &str) -> String;`, and it lands
      in the unclassified bucket and fails loudly:
      `["publish_moderation (parameters \`request: &str\`)"]`. That is the
      property the first version could not have: erring toward red on an
      unfamiliar shape, where the cost is teaching the parser one shape and the
      alternative is a dispatch method reaching the wire unswept.

      One thing your finding surfaced that I would have missed: the first cut of
      the classifier failed on the *wrapped* probe for the wrong reason —
      rustfmt's trailing comma put an ordinary method in the unclassified
      bucket. Fixed and commented, because a gate failing for the wrong reason
      is one an author "fixes" by teaching it a shape it already knew.

      **Preconditions now stated**, per the brief's instruction not to overclaim
      a second time: rustfmt-shaped Rust, a declaration ending in `;`, and a
      request parameter typed `String` by value. Anything else fails loudly
      rather than passing. That is in the function's doc comment and in
      `design.md` decision 6.

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
