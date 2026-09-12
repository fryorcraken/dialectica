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

## 1. `MembershipError::Storage` puts the host's absolute filesystem path into the `{"error":...}` wire reply

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

**Outcome:**

---

## 2. Nothing caps a request before `hex::decode` allocates from its length — measured at 1.80× the request in `join_stoa`

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

**Outcome:**

---

## 3. The creator/poster key pairing is held by two closures behind `cfg(logos_scaffold)`, which no test and no CI gate can reach

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

**Outcome:**

---

## 4. `parse_index` narrows a `u64` with `as`, feeding a store that uses `try_from` everywhere else to avoid exactly that

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

**Outcome:**

---

## 5. `Policy::to_byte` replaced by a constant still survives the whole suite, and a code comment says it does not

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

**Outcome:**

---

## 6. Four spellings of one guard: the `stoa` field is parsed inline three times beside the extractor this change added

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

**Outcome:**

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
