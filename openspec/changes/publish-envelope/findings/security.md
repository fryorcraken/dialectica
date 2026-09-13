# Security findings — `publish-envelope`

Reviewed at `35fc859`, in a worktree of my own. Dimension: **security**
(a sibling file carries correctness; the two share evidence where a defect is
both, and each box is written for the dimension it is filed under). Suite green
at 736 + 26 before any mutation. Worktree deleted on completion.

- [x] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:511` — the request size
      cap does not bound any allocation on the shipped path
      **Scenario:** the whole security value of routing publishes through
      `Request::parse` is that `request.len() > MAX_REQUEST_BYTES` is checked
      *before* `serde_json::from_str` (`wire/request.rs:205`), because per
      PHASE0-FINDINGS §3 an allocation failure in a dispatch handler aborts the
      module process — a 20-second caller timeout and `MODULE_NOT_LOADED` for
      every later call. The adapter defeats that: `Dialectica::publishing` runs
      its own unguarded `serde_json::from_str(request)` at line 511 and only
      then hands the same string to `core::publish_post` at 573. A caller
      sending a 400 MiB `publishPost` body pays the full ~2N heap in the adapter
      and the cap is evaluated afterwards, on bytes already allocated. The
      refusal is *reported*, but the allocation the refusal exists to prevent
      has already happened.
      **Measured:** the ordering is visible at `lib.rs:511` vs
      `wire/request.rs:205`; `core`'s own
      `an_oversized_request_is_refused_before_it_is_parsed` pins the ordering
      inside the crate and cannot see the adapter, which `cargo test` does not
      compile. **Severity: high** — this is the DoS the piece is named for, and
      it survives the piece. The fix is the same one correctness finding 1 asks
      for: delete the adapter's pre-parse so the envelope is crossed once.

      **FIXED.** The adapter's pre-parse is deleted; it now reads its Stoa
      through `core::stoa_of`, which is `Request::parse` then `parse_stoa`. The
      length check is therefore the first thing that happens to the bytes on the
      shipped path, so a 400 MiB `publishPost` body is refused **before** any
      allocation rather than after one — which is the distinction your finding
      turns on, and the reason "the refusal is reported" was not enough.

      **The test that fails without it**, and it asserts the ordering rather
      than the refusal, because "refused: yes" is true either way. In
      `the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does`
      I feed `stoa_of` a request that is both oversized **and** unparseable and
      assert the refusal says `over the … byte limit` and does **not** say
      `invalid JSON`. Only an implementation that measures length before calling
      `from_str` can answer that way — the same technique
      `an_oversized_request_is_refused_before_it_is_parsed` uses one level down,
      which you correctly noted could not see the adapter.

      **Plus a CI ban, because a test in `core` cannot hold a file `core` does
      not contain.** The adapter-derivation gate now fails on any
      `serde_json::from_str` in `rust-lib/src/lib.rs` — the shape, not the
      instance — so a future unguarded parse fails CI rather than shipping.
      Verified against `35fc859`, this piece's own previous commit: it fires.

      Two parses remain on the path (the adapter's, then the handler's), and
      `design.md` decision 7 says so rather than leaving it implied. Both are
      now behind the cap, so the residual is CPU on a request already proved
      small, not unbounded allocation. Removing the second is decision 8's
      deferred supplier-closure reshape.

- [x] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:536` — an unauthenticated
      malformed request buys a 64 MiB Argon2id derivation
      **Scenario:** `core::keystore::open_from_env` runs at line 536, before the
      handler validates anything but `stoa`. So `{"stoa":"<any valid 32-byte
      hex>","author":"x"}` — a forbidden field, refused unconditionally — or
      `{"stoa":"<valid hex>"}` with no `body` each trigger a full 64 MiB
      memory-hard KDF and are then refused. The Stoa need not exist and the
      caller need not be a member; only hex well-formedness is checked first.
      Repeated at request rate this is a memory-and-CPU amplifier with a
      trivially cheap request.
      **Measured:** by call order at 511→536→573. `design.md` §1 identifies this
      exact ordering as a defect and presents the prologue reshape as what makes
      "validate, then unlock" a one-function change; the change makes the
      reshape and does not make the reordering, so the security property is
      claimed and absent. **Severity: medium-high.** Moving `open_from_env`
      below the handler's validation is not possible while the handler needs the
      key — so the fix is to split the handler's validation from its signing, or
      at minimum to run the forbidden-field guard and the required-field reads
      in the adapter's `core` call before the unlock.

      **DEFERRED**, to its own piece, with `design.md` decision 8 carrying the
      argument and `docs/PLAN.md` §9.2 updated so it outlives this findings file.
      Your sharpest sentence — *"the security property is claimed and absent"* —
      is the part I fixed immediately: `design.md` §1 now states the ordering is
      **unchanged on every path** and names both your findings, rather than
      leaving the claim standing.

      **On your "at minimum" option, which I considered and did not take.**
      Running the forbidden-field guard and the required-field reads in the
      adapter before the unlock would work, and it is cheaper — but it puts a
      second reader of what a publish request must contain into the one file no
      test compiles. That is the same shape as the defect in your first finding
      (an adapter validating what the handler also validates, drifting apart
      unobserved), and it is what `design.md` decision 3 deleted `required_stoa`
      to avoid. Buying a DoS mitigation with a second validation path in an
      untested file trades a bounded cost for an unbounded one.

      So the fix is the one you named first — split validation from signing —
      which means the handlers taking a **fallible key supplier** rather than a
      key, moving their three signatures, the `Handler` type test, and every
      sweep fixture. PLAN.md §9.2 already flags that trade independently
      (`authoring` currently *cannot* create key material; a closure makes it
      "does not, and here is a test"), so it wants judging on its own merits
      rather than being smuggled in as a DoS fix.

      Not a defect this change introduced — one it claimed to fix. The claim was
      the error, and the claim is gone.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7541` — the
      envelope sweep can be evaded, so a future method can reach the wire
      unvalidated with the gate green
      **Scenario (security framing; the correctness file carries the parser
      mechanics):** this sweep is the only thing that asserts a dispatch method
      is inside the request envelope — the size cap, the non-object refusal and
      the three distinct messages. It reads the trait by matching
      `"&mut self, request: String) -> String;"` on a single line. Two ordinary
      ways of writing the *next* method escape it: a signature rustfmt wraps
      across lines once it exceeds 100 columns, and a parameter named anything
      but `request`. I added `publish_moderation` to the trait in each shape and
      the sweep **passed** both times, with the method on the RPC surface and
      absent from `every_request_taking_method`.
      **Measured:** two runs of
      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`,
      green, with an unswept trait method present. `publish_moderation` is not
      hypothetical: it is the moderation operation CLAUDE.md's second standing
      rule governs, and an unvalidated moderation handler is the authorisation
      defect `design.md` §1 cites as the reason this reshape happened at all.
      **Severity: high** — the gate is load-bearing for "never trust an inbound
      message" across the whole future surface, and it fails open.

      **CLOSED — the gate no longer fails open, measured on both of your shapes
      and on a third.** The mechanics and the two mutation transcripts are in
      the correctness file's two boxes; what follows is the part that is a
      *security* answer rather than a parser one, because your framing is the
      one that matters and it deserves more than a cross-reference.

      **Your framing was "it fails open", and that is the property that
      changed.** The old parser was a **filter**, and a filter's failure mode is
      silence: an unrecognised method is simply absent from the result, and an
      absent method is indistinguishable from a method that takes no request.
      The replacement is a **classifier** — every `fn` in the trait lands in
      exactly one of three buckets, and the third bucket is a panic naming the
      method and its parameter list. So an unfamiliar shape is now a red gate
      rather than an omission. That is the difference between failing open and
      failing closed, and it is structural rather than a matter of having
      patched two spellings.

      I verified it fails closed on a shape nobody has filed:
      `fn publish_moderation(&mut self, request: &str) -> String;` lands in the
      unclassified bucket and panics naming it. A `&str` request is a perfectly
      plausible future declaration, and the gate refuses to guess about it.

      **`publish_moderation` was the right method to probe with, and that still
      holds.** Your point that this is not hypothetical — it is the operation
      CLAUDE.md's second standing rule governs, and `design.md` §1 names it as
      the reason the reshape happened — is why I used your exact method name in
      all three mutations rather than a neutral one. An unswept moderation
      handler would be an unvalidated moderation handler, which is the
      authorisation defect the standing rule exists to prevent.

      **What this gate still does not do, stated so the close is not read as
      wider than it is.** It asserts a request-taking method is *in the sweep*.
      The sweep then checks the size cap, the non-object refusal and the three
      distinct messages. It does **not** check that a future method authorises
      its caller — no gate here does, and moderation will need one. That is the
      next piece's, not a gap this box leaves behind.

## What I verified and found clean

**Moderation-grade authenticity of the signing key is now correct, with no
bypass.** This is the change's real security win and it holds. All three publish
trait methods route through one `Dialectica::publishing` (`lib.rs:790,794,798`)
with a single `let key`, so there is no alternate signing path to audit.
`publishing_key` (`wire.rs:153`) is a structural mirror of the probe's
`posting_identity` — identical `paths.path_for(stoa)` lookup, identical three
arms — and `stoa_address_at_path` is *defined* as
`stoa_key_at_path(..).public_key().address()` (`keystore.rs:859-872`), so the op
author and the probe's reported identity agree by construction rather than by a
coincidence a test happens to pin. I confirmed the test is not self-agreeing:
mutating `publishing_key` back to `keystore.stoa_key(stoa)` fails
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports`, naming
`656c6003a040…` signed against probe `fb045664ae4d…`. The mutation landed and
was caught.

**A missing choice is a refusal, not a fallback**, and that is the right call —
`publishing_key` returns `Err(NO_CHOICE_FOR_THIS_STOA)` rather than deriving
something, so there is no state in which a publish succeeds under a key the
probe would not name.

**No secret material is leaked or compared unsafely.** `publishing_key` returns
a `SecretKey`, which deliberately has no `Debug` (the test at `wire.rs` uses
`.err()` rather than `.expect_err()` for exactly that reason), so the key cannot
reach a log through a format string. The refusal path returns the shared
`NO_CHOICE_FOR_THIS_STOA` constant, identical to what `getCapabilities` and
`whoAmI` already return, so a caller learns nothing from the publish refusal it
could not already learn from the probe — no new oracle. No comparison of secret
material is introduced.

**The deleted CI exemption is safe and its removal was genuinely load-bearing.**
`stoa_key` remains in `lib.rs` only at lines 544 and 665, both inside `//`
comments the gate strips before its ban regex runs; the only live Keystore
accessor call is `core::keystore::creator_key_in` at 711, which the gate
requires rather than bans. All three required `want` strings — including the
newly added `core::wire::publishing_key` — are present. I checked every
occurrence in the file rather than only the paths the author tested.

**No new dependency** is introduced by either commit.

**No new panic path.** `publishing_key` has no index, slice, `unwrap` or
arithmetic; both new tests' `unwrap`s are in test code. The three handlers
remain inside `guarded`.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is `cfg(logos_scaffold)` and compiled by
nothing runnable here, so the first two findings rest on reading the call order
at 511 / 536 / 573 rather than on executing it. Build LGX compiles that file but
asserts nothing about the order of operations inside it, so both defects pass
every gate on this PR. I did not exercise the module against a live basecamp
host.
