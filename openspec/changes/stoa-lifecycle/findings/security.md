# Findings — security

`code-reviewer`, **security dimension only**. Correctness, readability,
architecture, spec/test correspondence and design/decision correspondence are
other reviewers' files.

Reviewed at `307bf13` in `.claude/worktrees/rev-stoa-security`. Everything below
was re-derived in that worktree rather than taken from the brief; where a claim
in the brief turned out to be wrong, that is stated. **The tree was mutated
(four scratch integration tests, a `cargo mutants` run) and restored: it is
clean, `git status --untracked-files=all` empty, at the end of this review.**

Scope read in full: `dialectica/rust-lib/dialectica-core/src/membership.rs`,
`src/stoa.rs`, `src/cursor.rs`, the `create_stoa` / `join_stoa` / `list_stoas` /
`get_capabilities` / `genesis_for` / `parse_stoa` / `parse_index` /
`with_membership_store` / `membership_page_json` handlers in `src/wire.rs`, the
identity half of `src/keystore.rs`, `Address`/`PublicKey`/`SecretKey` in
`src/identity.rs`, and the adapter in `dialectica/rust-lib/src/lib.rs`.

---

- [x] **1. `MembershipError::Storage` puts the host's absolute filesystem path into the `{"error":...}` wire reply**

**For:** `dev-writer`

The defect: `storage()` (`membership.rs:660`) wraps every `rusqlite::Error` as
`MembershipError::Storage(e.to_string())`, and rusqlite's own message for an
open failure embeds the path it was handed. `with_membership_store`
(`wire.rs:784`) renders that straight into the module's one failure shape, so
the host-stamped `instance_persistence_path` — which carries the instance id —
reaches a view as a user-facing string.

This is a departure from the sibling module's stated posture, not a generic
hardening wish. `keystore.rs` deliberately never names the path in an error
(`KeystoreError::NotFound`, `PermissionsTooOpen { mode }`,
`DirectoryWritableByOthers { mode }` all carry the *fact* and not the *path*),
and it has a test — `no_error_message_carries_key_material_or_a_passphrase` —
pinning that the messages stay clean. `membership.rs` has
`every_error_renders_without_leaking_rust_syntax`, which checks for `::` and
`{` and would pass any path whatsoever.

**Failure scenario, measured.** Calling the public surface with a path whose
directory does not exist:

```
with_membership_store("list_stoas", Path::new("/nonexistent-dir-abc123/secret-user-name/stoas.sqlite"), …)
  -> {"error":"the Stoa membership store could not be used: unable to open database file: /nonexistent-dir-abc123/secret-user-name/stoas.sqlite"}
```

and with a path that exists and is a directory:

```
with_membership_store("list_stoas", std::env::temp_dir(), …)
  -> {"error":"the Stoa membership store could not be used: unable to open database file: /tmp"}
```

Both reproduced from a scratch integration test against the public API only.

**It is a family, not an instance.** `log/sqlite.rs:716` is the identical
`OpLogError::Storage(e.to_string())`, so `list_threads` leaks the op store's
path the same way and has done since before this change. This change copied the
pattern rather than introducing it — which is the exact shape MEMORY's
"unfixed test patterns get copied" note describes. Fixing only the membership
copy leaves the template in place.

`membership.rs:660`, `membership.rs:181-183`, `wire.rs:784`.

**Severity:** low as an information disclosure (the only caller is the local
sandboxed view), medium as a posture inconsistency — the project has an explicit
rule about this that one module honours and its sibling does not.

**Outcome: DEFERRED, with the leak now proven by a test rather than only reported.**

**Why deferred and not fixed**, which is this entry's own argument turned into a
decision. The entry says it plainly: *"It is a family, not an instance… Fixing only
the membership copy leaves the template in place."* `log/sqlite.rs:716` is the
byte-identical `OpLogError::Storage(e.to_string())`, it predates this change, and
`list_threads` leaks the op store's path the same way. A fix that closed the
membership copy and left the op log's would be the exact shape MEMORY's *"unfixed
test patterns get copied"* note describes — and it would change a sibling module's
inherited error shape as a side effect of a Stoa-lifecycle change, which is a
scope this piece should not quietly widen.

**Where it now lives**, because a finding that leaves without landing somewhere
durable was dropped rather than deferred:

1. **A characterisation test, in the code** —
   `membership.rs::an_unopenable_store_puts_the_host_path_into_the_error_a_view_renders`.
   It opens a store under a directory named `an-instance-id-nobody-should-see` and
   **asserts the path IS in the rendered error**, with a comment saying that a fix
   should invert it to `assert_ne`. So the leak is reproducible on demand, the next
   agent does not re-derive it, and a fix has something concrete to flip. Verified
   passing, which is what makes the defect real rather than argued.
2. **`design.md`, under Risks / Trade-offs**, as the family: both modules, why one
   copy cannot be fixed alone, and that the keystore's
   `no_error_message_carries_key_material_or_a_passphrase` is the shape both lack.

The entry's supporting observation is confirmed and recorded in that test's comment:
`every_error_renders_without_leaking_rust_syntax` cannot see this, because its
fixture is the hand-written string `"disk on fire"` — it would pass any path
whatsoever.

---

- [x] **2. Nothing caps a request before `hex::decode` allocates from its length — measured at 1.80× the request in `join_stoa`**

**For:** `dev-writer`

This is item 2 of the brief, re-derived. The brief's framing is accurate about
the ordering and understates nothing, but the *magnitude* was not measured
anywhere, and the exposure turns out to be a bounded constant factor rather than
an amplification.

The defect: `genesis_for` (`wire.rs:315`) calls `hex::decode(hex_str)` on the
caller's `genesis` field with no length check first, allocating `len/2` bytes
before `Genesis::decode` sees anything. `Address::from_hex`
(`identity.rs:122`) does the same for the `stoa` field — `hex::decode` first,
`try_into::<[u8;32]>` second — so both fields on a `join_stoa` request allocate
from an attacker-chosen length. `Genesis::decode` is correct once reached: the
title cap is checked *before* `cursor.take(len)` (`stoa.rs:296`), so the decoder
itself allocates nothing from a length prefix.

**Measurement.** A counting `GlobalAlloc` around the public `join_stoa`, with a
20 MiB hex `genesis` field (20,971,608-byte request):

```
peak live-heap delta during the call = 37,750,303 bytes = 1.80x the request
reply = {"error":"genesis: unknown genesis record version 0"}
```

and the same instrumentation around `create_stoa` with a 20 MiB `title`:

```
peak live-heap delta = 41,944,543 bytes = 2.00x the request
reply = {"error":"title: title is 20971520 bytes, the maximum is 1024"}
```

The 2.00× on `create_stoa` is `serde_json`'s parsed `String` plus the
`s.clone()` at `wire.rs:617`, which copies the whole title before any bound is
consulted. Both are constant factors of an input the process already holds, so
this is a 2× memory multiplier on an uncapped request, not an unbounded
allocation from a small one.

**Is there a cap upstream?** No, and I could not find one anywhere in the
stack. Grepping core for `MAX_REQUEST` / `request.len()` / any size constant
returns nothing; `docs/PHASE0-FINDINGS.md` records no IPC payload bound; the
150 KiB figure in `PLAN.md:580` is SDS's network message cap, which is a
different boundary and does not govern a local IPC request. So the module
allocates 2× whatever the host hands it.

`stoa.rs:47-57` already argues where the cap belongs — *"the right home for that
check is the transport boundary, where the SDS frame is actually visible"* — and
that reasoning holds: this decoder cannot know whether its bytes arrived in one
message. The finding is not that the cap is missing from the decoder, it is
that **nothing at any layer has it yet, and no document records that as an open
gap for the stoa surface.** The `op.rs` version of the same gap is written down;
this one is not.

`wire.rs:315`, `wire.rs:617`, `identity.rs:122`.

**Severity:** low. The reachable caller is the local sandboxed QML view, so an
attacker needs the IPC socket or the view itself first. Raise it the moment a
peer-facing decode path reaches `genesis_for`.

**Outcome: DEFERRED to `design.md`, which is what this entry asks for.** The entry is
explicit that the cap does not belong in the decoder — *"this decoder cannot know
whether its bytes arrived in one message"* — and that its finding is narrower:
*"nothing at any layer has it yet, and no document records that as an open gap for
the stoa surface. The `op.rs` version of the same gap is written down; this one is
not."*

So the deliverable is the record, and it now exists: `design.md`'s Risks /
Trade-offs carries the gap with this entry's measurements (1.80× on `join_stoa`,
2.00× on `create_stoa`, and that the 2.00× is `serde_json`'s parsed `String` plus the
`s.clone()` at the title read), the conclusion that this is a bounded constant factor
rather than an amplification, and the negative result that matters most — **no cap
exists anywhere in the stack**, with the three places checked, and that `PLAN.md`'s
150 KiB figure is SDS's network message cap and does not govern a local IPC request.

Not implemented here, deliberately. A cap at the decoder would be the wrong layer by
this entry's own argument; a cap at the transport boundary is a transport change. The
severity assessment is carried over verbatim, including the trigger for raising it:
the moment a peer-facing decode path reaches `genesis_for`.

One thing this change did do that touches the entry's follow-up: `parse_stoa` is now
the single place the `stoa` field is parsed (entry 6), so the length pre-check this
entry contemplates would be **one** edit rather than the four it would have been.

---

- [x] **3. The creator/poster key pairing is held by two closures behind `cfg(logos_scaffold)`, which no test and no CI gate can reach**

**For:** `tester`

This is item 3 of the brief. **The property holds today — verified — but the
thing that makes it hold is unguarded, and it is the same unguarded seam that
produced the bug `4313cf6` fixed.**

What I verified. `dialectica/rust-lib/src/lib.rs:346` supplies
`ks.identity_address().to_hex()` to `core::get_capabilities`, and
`lib.rs:385` supplies `ks.identity_public_key()` to `core::create_stoa`. Both
come from the same `Keystore` root via `identity_key()`
(`keystore.rs:679`), which takes no address argument at all, and
`identity_address()` is defined as `identity_public_key().address()`
(`keystore.rs:691`). So creator and poster are the same key by construction, and
`the_creator_of_a_stoa_this_keystore_made_can_moderate_it`
(`keystore.rs:3022`) asserts `Moderators::of(genesis).contains(identity_public_key())`
plus `identity_address() == genesis.creator.address()`. I found **no remaining
path where a creator and a poster can be two different keys.**
`Keystore::stoa_key` / `stoa_public_key` / `stoa_address` still exist and still
take a caller-chosen `Address`, but grepping `wire.rs` and `lib.rs` shows no
handler calls any of them.

The defect is that the *pairing* — the fact that those two closures name the
same key — lives entirely in `cfg(logos_scaffold)` code. The test at
`keystore.rs:3048-3055` says so itself: *"This is the pair the adapter wires up,
checked here because `cfg(logos_scaffold)` is not built by tests."* It checks
that `identity_address() == identity_public_key().address()`, which is a fact
about `Keystore` and is true whatever the adapter does. An edit that changed
`lib.rs:346` back to `ks.stoa_address(stoa)` while leaving `lib.rs:385` alone
would restore the original bug exactly, and:

- `cargo test` does not compile `lib.rs` at all (`lib.rs:191-206` explains why),
- CI's Rust job has no gate over the adapter (`.github/workflows/ci.yml` — the
  Lint job's greps cover `metadata.json`, the trait's location, the UI icon and
  QML; none reads the closure bodies),
- and `cargo mutants` structurally cannot see it: run over
  `keystore.rs` filtered to `identity_key|identity_public_key|identity_address|stoa_key|stoa_public_key|stoa_address`,
  **all 6 mutants came back unviable** (no `Default` for the key types), so
  mutation testing kills nothing on these accessors either.

**Failure scenario (the mutation that survives the suite).** Change
`lib.rs:346` from `ks.identity_address().to_hex()` to
`ks.stoa_address(_stoa).to_hex()` (and un-underscore the binding). Result:
`getCapabilities` reports a per-Stoa address, `createStoa` records the root
identity as creator, and `Moderators::of(genesis).contains(posting_key)` is
false for a Stoa's own creator — the original defect, permanently baked into
every address minted after the edit. **Every gate stays green**: the suite
passes, clippy passes, fmt passes, `cargo mutants` has no viable mutant here,
and no Lint-job grep looks.

What would close it: either a gate that reads both closure bodies and asserts
they name the same accessor (the Lint job already does this shape of
source-grep check five times), or moving the pairing into `core` as a single
function both handlers call, so one derivation position is a type-level fact
rather than two comments agreeing.

`dialectica/rust-lib/src/lib.rs:336-347`, `dialectica/rust-lib/src/lib.rs:375-389`,
`keystore.rs:3048-3055`.

**Severity:** medium. The code is right now; the regression it already suffered
once has no gate.

**Outcome: FIXED**, taking the second of the two options this entry offers —
moving the pairing into `core` — and then adding the first as well, because the
two close different halves.

Addressed here rather than by `tester` (whom it is addressed to) because the fix
is a code reshape, not a test: the reason no test could pin this is that there was
nothing in `core` to point a test at. Same finding as `architecture.md` entry 1,
answered once in both files.

`core::keystore::creator_and_poster_in(dir) -> (PublicKey, Address)` now derives
both halves in one expression from one `identity_key()` root, with
`creator_key_in` / `poster_address_in` as the wrappers the adapter calls. Two call
sites that had to agree became one derivation, so divergence is no longer
representable.

**The test that fails without it:**
`wire.rs::the_creator_a_creation_names_is_the_identity_the_probe_reports`. It
drives *both* wire handlers through those `core` functions against a real on-disk
keystore. Running this entry's own named mutation — pointing the poster half at
`ks.stoa_address(...)` — gives **550 passed, 1 failed**, and the one failure is
that test. Before this change the same mutation left the suite at 550/550.

**And the grep gate this entry asks for**, for the half no test can reach (that
the adapter still *calls* those functions rather than picking an accessor itself):
a new Lint step, `the adapter derives the creator and the poster in one place`.
Both of its checks were verified to fire — removing `poster_address_in` trips the
name check, and calling `ks.stoa_address(...)` while leaving the name present trips
the banned-accessor check.

Two corrections to this entry, neither affecting its conclusion:

- The entry says `cargo mutants` "structurally cannot see it". Confirmed, and now
  partly moot: the derivation it could not mutate lives in a `core` function a test
  calls directly.
- `keystore.rs:3048-3055`'s comment claiming to check "the pair the adapter wires
  up" is corrected in the same commit rather than the test being deleted — the
  entry is right that the test cannot pin the adapter, and the test is still worth
  having for what it does pin.

Recorded in `design.md` under *"Where that decision lives:
`core::keystore::creator_and_poster_in`, because two agreeing call sites are not
one derivation"*, including what the gate cannot see.

---

- [x] **4. `parse_index` narrows a `u64` with `as`, feeding a store that uses `try_from` everywhere else to avoid exactly that**

**For:** `dev-writer`

`parse_index` (`wire.rs:452-467`) reads `page` and `perPage` through
`serde_json::Number::as_u64()` and then does `Ok(Some(v as usize))`. That is a
silent narrowing cast on the only numeric values in the request path.

It is the one place in the reviewed code that uses `as` for a width-changing
conversion, and `MembershipStore` argues against it in as many words two files
away — `membership.rs:596-599`: *"`try_from` rather than `as` so that a platform
where it could fail says so instead of wrapping."* `list` then goes to
considerable trouble over the `usize`→`i64` boundary (`membership.rs:532-558`,
with a regression test for a bug it already caused), and that care is fed by a
cast that could hand it a value the caller never sent.

**Failure scenario.** On a 32-bit target, `{"page":4294967296}` — a value
`as_u64` accepts — narrows to `0`, and `list_stoas` serves **page 0** to a
caller who asked for a page far past the end, reporting `"page":0` so the caller
cannot tell. That is the same class of wrong answer
`a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page`
(`membership.rs:1456`) exists to prevent one layer down, reintroduced above it.

**Measurement: not reachable on the supported target.** `dialectica` targets
Linux x86_64 (`PLAN.md`'s Basecamp traps are Linux-specific), where
`usize == u64` and the cast is total. I confirmed the 64-bit behaviour is
correct by measurement — `{"page":18446744073709551615,"perPage":1}` returns
`{"hasMore":false,"items":[],"page":18446744073709551615}`, and `-1`, `1.5` and
`1e308` are each refused as `"page must be a non-negative whole number"`. I did
**not** build for a 32-bit target; doing so would require adding a target and is
the measurement this entry lacks.

`wire.rs:460`.

**Severity:** low (latent; unreachable on the supported platform). Filed because
it is a one-character divergence from a rule the code states explicitly, in the
one place a bad number reaches an offset.

**Outcome: FIXED.** `parse_index` now does `usize::try_from(v)` and treats an
unrepresentable index as the refusal it is, sharing the existing message — *"a page
number this platform cannot hold is not a page number, exactly as `-1` is not"*.

The entry is right on every point including the one it declines to claim: it is
latent on the supported target and it **did** measure the 64-bit behaviour rather
than assume it (`{"page":18446744073709551615,"perPage":1}` answering with that page
verbatim, and `-1`, `1.5`, `1e308` each refused). I did not add a 32-bit target
either, so no test distinguishes the two versions on this platform — **stated
plainly rather than papered over**, because `usize == u64` here means the old `as`
and the new `try_from` are behaviourally identical and no fixture can tell them
apart. 550 tests pass before and after, as they must.

What the change buys is therefore not a caught bug but the thing the entry actually
filed: the one width-changing `as` in the request path is gone, so the code no
longer contradicts, in the place a bad number reaches an offset, the rule it states
two files away — *"`try_from` rather than `as` so that a platform where it could fail
says so instead of wrapping."* The comment records the 32-bit failure scenario the
entry constructed, so the reasoning survives even though no gate exercises it.

---

- [x] **5. `Policy::to_byte` replaced by a constant still survives the whole suite, and a code comment says it does not**

**For:** `tester`

`stoa.rs:124-130` documents the fix for a mutation `cargo mutants` found:

> Exists because `cargo mutants` found that replacing [`Policy::to_byte`] with a
> hardcoded `0` survived the suite — true while there is one variant, and
> silently wrong the moment there are two. A test iterating this fails when a new
> variant is added without a discriminant.

Re-run at `307bf13`, against the **whole** `dialectica-core` suite (no test
filter):

```
cargo mutants --file dialectica-core/src/stoa.rs --timeout 200
  19 mutants tested in 5m: 1 missed, 14 caught, 4 unviable
  MISSED  dialectica-core/src/stoa.rs:137:9: replace Policy::to_byte -> u8 with 0
```

So the mutant the comment is about is **still alive**. The comment is not false
about *why* — `Policy::ALL` genuinely makes the test fail when a second variant
lands without a discriminant — but it reads, and was read by me on first pass,
as saying the gap is closed. It is closed *prospectively*. Today
`every_policy_round_trips_through_its_discriminant` (`stoa.rs:897`) iterates one
variant whose discriminant is `0`, so `to_byte() -> 0` and the real `to_byte`
are indistinguishable, and `the_wire_format_is_pinned_to_a_known_answer`
asserts `Policy::OPEN == 0` (a `const`, which mutants cannot mutate) and a hex
blob whose policy byte is also `0`.

**Why it belongs in a security file.** The policy discriminant is inside the
address preimage (`stoa.rs:107-114`, `stoa.rs:844-871`) and
`Policy::from_byte` refuses an unknown discriminant specifically so that *"a
token-gated Stoa [does not] silently become world-postable on an older client"*
(`stoa.rs:145-149`). A `to_byte` that ignores its input is the encode-side
version of that failure, and the moment a second policy variant exists it ships
every Stoa as `Open` — inside the address, so with no error anywhere.

The honest fix is either a second variant (not in scope) or a hardcoded
expectation that does not depend on there being two: assert
`Policy::Open.to_byte() == 0` against the literal rather than round-tripping,
which is `identity.rs::the_wire_constants_are_pinned_to_known_answers`'s own
technique — and amend `stoa.rs:124-130` to say the mutant is live and why,
since a comment claiming a closed gap is worse than no comment.

`stoa.rs:124-130`, `stoa.rs:136-140`, `stoa.rs:896-921`.

**Severity:** low today, high the day a second `Policy` variant lands. The
mis-stating comment is the part that costs something now.

**Outcome: OPEN — addressed to `tester`, and deliberately left for it.** Noted here
by `dev-writer` so the box is not mistaken for an oversight.

Not taken on for two reasons. The fix the entry names is a **test** change — *"assert
`Policy::Open.to_byte() == 0` against the literal rather than round-tripping"* — and
the test suite is `tester`'s to own; writing it here would put two agents in the same
file on one piece. And the second half is a comment at `stoa.rs:124-130` that must
say the mutant is live, which is only true once the test that makes it dead does not
exist — so the comment edit and the test belong in one commit, by whoever writes the
test.

Nothing in this change touches `Policy::to_byte`, `Policy::ALL`, or
`every_policy_round_trips_through_its_discriminant`, so the entry's measurement still
stands as written.

**Outcome (`tester`): FIXED, with two corrections to the entry — the mutant is
unkillable, and the fix the entry names does not kill it either.**

**First, the entry's measurements reproduce exactly.** Both halves verified before
changing anything:

1. **The mutation survives.** `Policy::to_byte` replaced by `{ 0 }`, whole suite,
   no filter: predicted 551 passed / 0 failed, **observed 551 passed, 0 failed**.
   Restored.
2. **`cargo mutants` agrees**, re-run at this tree: **19 mutants, 1 missed, 14
   caught, 4 unviable**, the miss being `replace Policy::to_byte -> u8 with 0` —
   the entry's numbers to the mutant.

So the entry is right that the mutant is live and right that the comment read as
claiming otherwise. Both stand.

**Correction 1 — the entry's proposed fix does not close it.** The entry names the
fix as *"assert `Policy::Open.to_byte() == 0` against the literal rather than
round-tripping"*. I wrote that assertion and ran it under the mutation: predicted
pass, **observed pass**. It cannot fail.

The reason is structural rather than a weakness in the assertion. `Policy` has
exactly one inhabitant and its discriminant *is* `0`, so `to_byte` and the constant
`0` are **the same function on the whole domain**. No fixture — hardcoded literal,
round-trip, hex blob or otherwise — can distinguish them, because there is nothing
to distinguish. This is an **equivalent mutant**, not a coverage gap, and the entry's
own sentence *"either a second variant (not in scope) or a hardcoded expectation"*
presents those as alternatives when only the first works. Confirmed after the fix:
`cargo mutants` still reports **1 missed, 14 caught, 4 unviable**. The mutant is
still alive and will remain so until a second `Policy` variant exists.

**Correction 2 — and this is the part that was a real, closable gap.** While
establishing the above I found that the promise `Policy::ALL`'s doc makes —
*"a test iterating this fails when a new variant is added without a discriminant"* —
**does not hold**, for a reason the entry does not mention. A variant added to the
enum but **not** added to `ALL` leaves `every_policy_round_trips_through_its_discriminant`
green, because it iterates `ALL` and `ALL` still has one entry. Measured: with a
second variant wired correctly through `to_byte`, `from_byte` and `policy_name` but
absent from `ALL`, that test **passes**. So the enum's exhaustiveness bookkeeping —
which every iterating test in the module depends on — was enforced by a comment
asking the next author to remember.

**The test that now fails without it:**
`stoa.rs::policy_all_holds_every_variant_and_each_maps_to_its_pinned_byte`.
It carries a table of `(variant, hardcoded byte)` rows plus an exhaustive `match`
whose only job is to be exhaustive, so the enforcement is a **compile error** rather
than a request. Proven able to fail, in both of its modes:

- **A variant missing from `ALL`** (the gap above): predicted a length disagreement,
  **observed `left: 2, right: 1`** — *"the pinned table and Policy::ALL disagree
  about how many variants exist"*. The pre-existing round-trip test passes in the
  same tree, which is what makes this test worth having.
- **A variant not in the table at all**: predicted a non-exhaustive-match compile
  error naming the test's line, **observed `E0004` at `stoa.rs:963`**. A useful
  extra observation while doing this: `to_byte` itself and `wire.rs:570`'s
  `policy_name` are *also* exhaustive and *also* fail to compile, so a variant
  genuinely cannot be added without choosing a discriminant. The gap was only ever
  `ALL`.

The bytes are hardcoded literals, not `Self::OPEN` — reading the discriminant back
out of the implementation would be the "ask the code what it wrote and agree" shape,
and `Policy::OPEN` is a `const`, which `cargo mutants` cannot mutate, so an assertion
written against it is the one thing no gate can check.

**And the comment, which the entry is right to call the part that costs something
now.** Both overclaiming comments are corrected rather than deleted:

- **`Policy::ALL`'s doc** (`stoa.rs:122-140`) now says the mutant **is still
  reported MISSED**, why it is unkillable, that the literal assertion survives it
  too, and what the named test actually closes.
- **`every_policy_round_trips_through_its_discriminant`'s comment** made the same
  overclaim from the test side and is corrected the same way: iterating `ALL` does
  not make the mutant die today, and does not catch a variant that never reached
  `ALL`.

**What `cargo mutants` cannot see here**, since the brief asked: two things, and the
second is new.

1. As the entry says and the "Checked and clean" section repeats — it mutates
   functions, not `const` values, so `Policy::OPEN` is invisible to it. The real
   exposure on the *discriminant* is therefore a constant, pinned only by
   hand-written assertions (`the_wire_format_is_pinned_to_a_known_answer`'s
   `assert_eq!(Policy::OPEN, 0, …)` and now the table above).
2. **It cannot distinguish an equivalent mutant from a missed one.** A permanently
   MISSED row that no test can ever kill looks identical in its report to a genuine
   coverage gap, which is how this entry came to name a fix that could not work.

**Gates.** Suite **552 passed, 0 failed** (551 baseline + 1 new test). Clippy
`-D warnings` clean. `rustfmt --check --config skip_children=true` on `stoa.rs`
reports **5 hunks, all pre-existing** — verified by stashing this work and
re-running on the untouched baseline, which reports the same 5. Nothing
pre-existing reformatted. `git diff --stat` shows **one file, `stoa.rs`**: the
implementation is untouched apart from the doc comment this entry asks to be
corrected, and the three scratch mutations (`to_byte`, a second `Policy` variant,
`wire.rs`'s `policy_name` arm) are all restored.

---

- [x] **6. Four spellings of one guard: the `stoa` field is parsed inline three times beside the extractor this change added**

**For:** `dev-writer`

`parse_stoa` (`wire.rs:812-819`) is new in this change and is the right shape —
it makes the absent / wrong-typed / not-an-address distinction one job with one
answer. It has three unconverted copies beside it: `wire.rs:210-217`
(`get_capabilities`), `wire.rs:345-352` (`list_threads_inner`) and
`wire.rs:423-430` (`list_threads_from_request`), each an identical inline
`match parsed.get("stoa") { Some(String(s)) => Address::from_hex(s) … }`.

CLAUDE.md: *"When you find yourself writing the fourth slightly-different copy
of a guard, that is the signal to reshape rather than to add a fourth test"*,
and *"A guard is a job. Keep it separate, so 'is it called everywhere?' stays a
question with an answer."* This change wrote the extractor and then left three
copies, so the question still has no answer: a future tightening of the address
parse — a length pre-check ahead of `hex::decode`, per entry 2 — has to be
applied four times and will be applied to one.

The four copies agree today. I read all four and they are byte-identical in
behaviour, so **this is not a live divergence** — it is the shape that produces
one.

`wire.rs:812-819` against `wire.rs:210-217`, `wire.rs:345-352`, `wire.rs:423-430`.

**Severity:** low. Shape, not defect. The architecture reviewer owns the
reshaping argument; it is here because the duplicated thing is a validation
guard on attacker-supplied content.

**Outcome: FIXED.** All three inline copies now call `parse_stoa`.
`grep -c "missing field: stoa" wire.rs` returns **2** — the one inside `parse_stoa`,
and one inside a doc-comment citing this finding.

The entry's framing is what made this worth doing rather than tidy, and it is quoted
in the code: *"a future tightening of the address parse — a length pre-check ahead of
`hex::decode`, per entry 2 — has to be applied four times and will be applied to
one."* That is now one edit, and the doc says so.

This entry is `findings/readability.md` entry 3 from the security side; one change
answers both. The entry's careful note that *"the four copies agree today… this is not
a live divergence"* is confirmed by the result: 550 tests pass unchanged, which is
what unifying four agreeing copies must produce.

---

## Checked and clean

Each of these cost time and found nothing, which is worth as much to the next
agent as a finding. Do not re-derive them.

**The `getCapabilities({"stoa":"<64 zeros>"})` finding is genuinely closed, by
construction, and the test that claims so does establish what its name says.**
Item 1 of the brief, re-derived three ways. (a) The only wired lookup is
`lib.rs:336-347`, whose closure ignores its `_stoa` argument and returns
`ks.identity_address()`; `identity_key()` (`keystore.rs:679`) takes no address,
so there is no context argument for a caller to choose. (b) Grepping `wire.rs`
and `lib.rs` for `stoa_key|stoa_public_key|ks.stoa_address|derive_stoa_key`
finds hits only inside `#[cfg(test)]` in `wire.rs` — no handler reaches a
per-stoa derivation. (c) `CREATOR_KEY_DOMAIN` is gone from the tree entirely
(no grep hit outside the `4313cf6` commit message). Measured: with the zero
address, `get_capabilities` returns the identity address and
`ks.stoa_address(Address::from_bytes([0u8;32]))` is a *different* key —
`assert_ne!` held. `no_stoa_address_a_caller_can_name_reaches_the_identity_key`
(`keystore.rs:2964`) is a fair test of its name: it holds the identity constant
and varies the derivation context over all-zeros, all-ones and a real record
hash, asserting each yields a different key. It would fail if `identity_key`
were re-implemented as `derive_stoa_key(root, some_fixed_address)`, which is the
mutation it is guarding.

**`cargo mutants`, no survivors in the two files that matter.**
`--file dialectica-core/src/membership.rs` → 26 mutants, 18 caught, 8 unviable,
**0 missed**. `--file dialectica-core/src/wire.rs` (full suite, 16 min) → 59
mutants, 56 caught, 3 unviable, **0 missed**. I also ran the six
security-critical wire helpers alone
(`genesis_for|parse_stoa|policy_name|stoa_reply|membership_page_json|membership_path_in`)
→ 8 caught, 2 unviable, 0 missed — including `delete ! in genesis_for`, which is
the mutation that removes the self-authentication check. The one survivor
anywhere in scope is entry 5. Note what mutants cannot see here: it does not
mutate `const`s, so `MEMBERSHIP_LAYOUT_VERSION`, `MAX_TITLE_BYTES`, `VERSION_1`
and `Policy::OPEN` are invisible to it — each is pinned by a hand-written
hardcoded assertion instead, which I checked exists for all four.

**The genesis decoder against hostile bytes.** Truncated, trailing, lying
length prefix (both directions), a claim over the cap, invalid UTF-8, an
invalid creator point and a low-order creator point are each refused
distinguishably, and the cap-before-read ordering (`stoa.rs:296` ahead of
`stoa.rs:302`) is correct so an over-long claim costs no allocation. `Cursor`
(`cursor.rs`) is the right shape: `checked_add` before the bounds check, a
failed `take` does not advance the head, and `take_length` widens rather than
narrows. Measured from the public surface: `genesis` of `"0"`, `"zz"`, `""` and
`"0100"` each return a distinct refusal and **none panics**.

**No reachable panic in the non-test code under review.** `membership.rs`'s
module doc claims *"outside `#[cfg(test)]` there is no `unwrap`, no `expect` and
no indexing that could be out of bounds"* — I grepped and read it, and the
claim holds exactly: zero `unwrap`/`expect`/`panic!`/indexing, one
`saturating_mul`, one `saturating_add(1).min(i64::MAX as usize)`, and a
`usize::try_from` on the count. `wire.rs`'s non-test code has one cast (entry 4)
and no indexing. `keystore.rs:681`'s `expect` is genuinely unreachable — every
32-byte string is a valid Ed25519 seed and the root is 32 bytes by its own
type. I drove `create_stoa`, `join_stoa`, `list_stoas` and `get_capabilities`
with: empty titles, 1024- and 1025-byte titles, a 1200-byte multi-byte-character
title (over the cap in *bytes*, under it in *chars* — correctly refused), NUL
and BOM in a title, `page`/`perPage` at `u64::MAX` and `i64::MAX`, `-1`, `1.5`,
`1e308`, odd-length hex, non-hex, a 40 MiB hex field, and 200,000 nested `[`.
Not one panicked; the nested JSON is refused by serde's own recursion limit
(`{"error":"invalid JSON: recursion limit exceeded at line 1 column 128"}`).

**SQL parameter handling.** Every statement in `membership.rs` is a static
string with `?1`/`?2` bound through `rusqlite::params!`. The only interpolation
is `format!` over `MEMBERSHIP_LAYOUT_VERSION`, an `i32` constant, into
`create_schema`'s `PRAGMA` — no caller-supplied value reaches SQL text.
`STRICT` is on the table and the address is the `PRIMARY KEY`, so `INSERT OR
IGNORE` makes a repeated join structurally idempotent rather than by a branch.

**The record/address verification, and the storage round-trip.** `join` encodes
before it writes, so a refused join leaves nothing behind; `matches` re-derives
and consults only its two arguments (`verification_consults_only_the_two_inputs`
proves the answer does not depend on store contents); `decode_row` takes the
address from the row *key* and the record from the row *bytes* and then checks
them against each other, so a hand-edited file cannot file one Stoa's record
under another's address. `genesis_for` verifies before `store.join` verifies
again, and the second check is not redundant — it is what makes the store's
invariant hold for callers that do not come through the handler.

**The encode/decode asymmetry on the creator key, probed and dismissed.**
`Genesis::decode` refuses a weak (low-order) creator, but `create_stoa` puts
`ks.identity_public_key()` into the record without going through
`PublicKey::from_bytes`, so in principle a peer could create a Stoa whose
genesis record it cannot itself decode — unreadable on the next `listStoas`.
Measured: **0 of 200,000 distinct seeds** produce a public key that
`PublicKey::from_bytes` refuses, which is what Ed25519's scalar clamping
predicts (the clamped scalar is a multiple of 8 in `[2^254, 2^255)`, so `a·B`
cannot be low-order). Unreachable; not filed. Worth knowing that
`create_stoa` never round-trips the record it built, so the module doc's *"the
decoder refuses exactly what the encoder declines to produce"* is exact for the
title bound and approximate for the creator.

**Founding-title sanitisation.** `foundingTitle` ships the title byte-for-byte
including bidi overrides and zero-width characters
(`a_title_carrying_bidi_and_zero_width_characters_is_retained_unchanged`), which
is correct — normalising would change the address and split one Stoa in two —
and `docs/UI-BRIEF.md:250-256` already carries the rendering obligation
explicitly for this field. Not a finding. One thing the brief now overstates,
inherited and not made worse by this change: `UI-BRIEF.md:215` says the core
*"deliberately does not sanitise display text"*, while `feed_page_json`
(`wire.rs:493`) does run post bodies through `sanitise` and returns
`removed`/`marked` counts. That is the feed capability's sentence to fix.

**Keystore file handling.** `read_checked` (`keystore.rs:1024`) is the TOCTOU
fix already made: one `File::open`, `metadata()` on the *handle*, mode checked
before any content is used, `MAX_KEYSTORE_LEN` bound plus a `take()` so a FIFO
cannot stream unbounded. Passphrase verification is the AEAD tag (constant-time)
with the header as AAD and no separate stored verifier. `write_atomically`
stages through a random unguessable name with `create_new`, and the absent
`O_NOFOLLOW` is a recorded decision with the residual gap written down. The one
remaining path-resolution is `check_directory_mode`'s `fs::metadata(dir)`, which
is documented and advisory. Nothing here is a finding and the reasoning is
already in the file; do not re-litigate it.

**What the CI gates cannot see, re-measured.** `cargo fmt --manifest-path
dialectica/rust-lib/Cargo.toml --check` — the exact command in
`.github/workflows/ci.yml:607` — exits **0**, while running fmt directly on
`dialectica-core/Cargo.toml` reports **106 diffs, 24 of them in the four files
under review**. MEMORY's `dialectica-ci-fmt-gap` note holds unchanged: `cargo
fmt` does not follow path dependencies, so the format gate reaches none of this
code. Pre-existing and not this change's doing, recorded so the next reviewer
does not read a green fmt job as coverage.

**The membership store has no permission check**, unlike the keystore beside it
in the same directory. Considered and not filed: it holds only genesis records,
which are public by construction (a Stoa address is published so others can
join), so there is no secret for a mode check to protect. Noted in case a future
column makes that untrue.
