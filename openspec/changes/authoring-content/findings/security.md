# `authoring-content` — security review

Scope: **security only**. Correctness, readability and architecture belong to the
other reviewers and are not covered here, except where a defect is both.

Reviewed: `dialectica/rust-lib/dialectica-core/src/authoring.rs`,
`dialectica/rust-lib/dialectica-core/src/wire.rs`,
`dialectica/rust-lib/dialectica-core/src/op.rs`,
`dialectica/rust-lib/src/lib.rs` (the `cfg(logos_scaffold)` adapter), plus the
`keystore.rs` and `log/sqlite.rs` paths the publish handlers reach into.

Baseline `cargo test -p dialectica -p dialectica-core`: **531 passed**, confirmed
before any mutation. Every mutation described below was reverted; `git status`
is clean at the time of writing and the only file added is this one.

---

## S1 — A 153,601-byte body publishes successfully and permanently bricks every feed read on the store

**For:** `dev-writer` (the fix), `spec-writer` (the missing requirement),
`tester` (the boundary tested on one side only)

**Severity: HIGH.** Persistent, self-inflicted denial of service reachable from
one ordinary wire call with no special privilege, surviving a restart, with no
API in this change able to undo it.

**Where:** the encode path has no length check —
`dialectica/rust-lib/dialectica-core/src/op.rs:754` (`put_bytes`) writes any
length at all, while the decode path refuses over the cap at
`dialectica/rust-lib/dialectica-core/src/op.rs:798` (`take_length_within_cap`,
`MAX_FIELD_LEN = 150 * 1024` at `op.rs:132`). Nothing between
`wire.rs:739` (`required_string(&parsed, "body")`) and
`authoring.rs:187` (`post`) bounds the body, and `authoring.rs:183-186`
documents only the *lower* bound ("an empty body is accepted").

**Failure scenario, measured.** Against a real `SqliteOpLog`:

```
PROBE normal publish:    {"opId":"328e3c93…","wasNew":true}
PROBE feed before:       Ok(1)
PROBE oversized publish: {"opId":"72e9faaa…","wasNew":true}       <-- SUCCESS
PROBE feed after:        Err(CorruptEntry("the stored op did not decode: \
                             a field claims 153601 bytes, over the 153600 cap"))
PROBE iter after:        Err(CorruptEntry(…))
PROBE feed after reopen: Err(CorruptEntry(…))                     <-- PERSISTENT
```

Reproduce: `publish_post` with
`{"stoa":"<valid>","body":"x".repeat(150*1024 + 1)}`. It returns
`{"opId":…,"wasNew":true}` — no error. The op is signed, appended, and handed to
the delivery sink. Then:

1. `SignedOp::from_bytes(&stored.to_bytes())` on the op's *own* encoding returns
   `Err(FieldTooLong(153601))`. The encoded op is 153,741 bytes.
2. `log/sqlite.rs:688` (`decode_entry`) turns that into
   `OpLogError::CorruptEntry`, and `log/sqlite.rs:650` propagates it with `?`
   out of `ordered_read`. All three of `iter`, `iter_stoa` and `iter_target`
   route through `ordered_read`, so **every list read on that store fails from
   then on**, not just a read of the bad op.
3. `wire::list_threads` turns that into the error shape. The Stoa's feed — and
   the whole log's — is dead.
4. It is on disk. A reopen reproduces it. Only single-id `get`
   (`log/sqlite.rs:804`) still works, because it decodes one row.

The op is also unreadable by **every conforming peer**, so a user is told their
post was published when no peer can ever render it — the "broken call
indistinguishable from a successful answer" shape §2.5 exists to forbid.

**Why the gates miss it.** Three independent reasons, each worth fixing:

- `authoring.rs:1342` `a_maximal_body_publishes_rather_than_panicking` tests
  `"x".repeat(150 * 1024)` — exactly *at* the cap, which round-trips fine — and
  never one byte over. This is precisely the one-sided-boundary defect
  `keystore.rs:126-140` warns about in its own words ("the boundary was tested
  and the product of boundaries was not").
- `wire.rs:2942` puts both `150*1024` and `150*1024 + 1` into the hostile sweep,
  but the sweep asserts only "no panic" (`wire.rs:3012-3032`) and has **no
  log-length assertion at the end**, unlike
  `every_publish_refusal_is_the_error_shape_and_carries_no_op_id`
  (`wire.rs:2900`). So the oversized case silently *succeeds* and the test is
  blind to it.
- The spec never bounds a body. `specs/content-authoring/spec.md:141` requires an
  empty body be accepted and says nothing about an upper limit, so there is no
  requirement for a test to fail against.

**Note on where the fix belongs.** `op.rs:105-127` argues at length that the
*total* size check belongs at the transport boundary and not in `op.rs`. That
argument is about a **total** across fields and is not a reason to leave the
**per-field** cap unenforced on encode: the decoder already owns that number, so
enforcing it symmetrically on the encode path is making one existing rule total
rather than inventing a second bound. A `Refusal::BodyTooLong { len, cap }` at
the publish path would also let the UI say why.

Outcome:

---

## S2 — Lead 1 re-measured: the sweep reaches every parser, but the tasks.md table's conclusion does not hold, and `required_direction`'s refusal path is unexercised

**For:** `tester` (the claim), `dev-writer` (no code change needed)

**Severity: MEDIUM** — a coverage claim used to license a conclusion it does not
support, in exactly the place the change's own notes say to be careful.

**Where:** `openspec/changes/authoring-content/tasks.md:256-267`.

The table claims, for all three of `publish_vote`'s `required_direction`,
`publish_reply`'s `required_string("body")` and `publish_post`'s
`required_string("body")`, that the sweep "catches it — yes", verified "by making
each handler's *last* parser panic".

**Both halves are true and the conclusion drawn from them is wrong.** I
reproduced the stated experiment two ways:

| Mutation in `required_direction` (`wire.rs:659`) | Sweep result |
|---|---|
| `panic!()` unconditionally, before `required_string` | **FAILS** — the parser is indeed reached |
| panic only on the `Err` arm of `required_string(parsed, "direction")` | **PASSES** |

The second mutation is the one that matters and it survives. The reason is that
the hostile-text block (`wire.rs:2933-2940`) supplies `direction: text` where
every `text` is a JSON **string** (`"\u{202E}\u{202C}\u{200B}"`, `"\0\0\0"`,
`"🏛🏛🏛"`, `"Ἀγορά"`, `"\"}]"`). `required_string` returns `Ok` for all of them,
so the parser is entered only on its success path. No fixture in the sweep pairs
a **valid Stoa and valid target** with a **missing or wrong-typed** `direction`:

- `wire.rs:2928-2931` do carry `direction` as null/object/array/number — but they
  also malform `stoa`, so all three handlers refuse at parser 1 and never reach
  it. This is the *same* ordering hazard the change already identified and fixed
  for `OpId::from_hex`, still present one parser further along.
- `wire.rs:2956` and `wire.rs:2980` pin `direction: "up"`.

So the table's "yes" is measuring reachability, not refusal-path coverage, and
the ordering hazard the change corrected for the op-id parser was not re-checked
for the direction parser. `required_string`'s `Err` arm for `body` on
`publish_reply` is likewise unreached by the sweep (verified: the sweep passes
with a panic on that arm), though it is covered by
`a_publish_requests_missing_field_is_named_and_is_not_defaulted`
(`wire.rs:2126`) — so that one is a sweep gap rather than a coverage gap.
`required_direction`'s `Err` arm is reached by
`every_publish_refusal_is_the_error_shape_and_carries_no_op_id`
(`wire.rs:2879`, `"direction":true`), but that test asserts only that an `error`
key is present, which a caught panic satisfies — so **no test in the repo
distinguishes a refusal from a panic on the direction parser.**

**The lead itself was right to insist on this.** The op-id parser IS genuinely
covered — I verified an `expect` on `OpId::from_hex` in `required_op_id`
(`wire.rs:646`) makes the sweep FAIL, so the fix at `wire.rs:2980` works. The
residue is one parser the fix did not extend to, and a table that reads as
though it did.

Suggested repair: add to the sweep a block pairing a valid `stoa` and a valid
`target`/`parent` with `direction`/`body` absent and of each wrong type, and
re-word the table to say which *arm* of each parser the sweep exercises.

Outcome:

---

## S3 — Every publish runs a full Argon2id keystore unlock and opens SQLite before the request is validated, including requests it has already decided to refuse

**For:** `dev-writer`

**Severity: MEDIUM** — unauthenticated CPU and memory cost amplification on the
module wire, and it contradicts the change's own stated ordering rationale.

**Where:** `dialectica/rust-lib/src/lib.rs:296-332`. Order in `publishing`:

1. `serde_json::from_str` + read `stoa` only (`lib.rs:296-307`)
2. `core::keystore::open_from_env(&keystore_path)` — **Argon2id** (`lib.rs:321`)
3. `keystore.stoa_key(&stoa)` — HKDF (`lib.rs:325`)
4. `core::log::SqliteOpLog::open(…)` (`lib.rs:327`)
5. **only then** `handler(…)`, which is where `reject_forbidden_fields`
   (`wire.rs:732`), `required_string("body")`, `required_op_id` and
   `required_direction` live.

**Failure scenario.** A caller sends `{"stoa":"<64 valid hex>","author":"00ff"}`
to `publish_post`. The spec at `specs/content-authoring/spec.md:86-93` requires
this be *refused*. It is — but only after the module has already read and
decrypted the keystore. The shipped parameters are RFC 9106 option 2 —
`ARGON2_M_COST_KIB = 65_536`, `t = 3`, `p = 4` (`keystore.rs:111-113`) — so each
such request costs a 64 MiB allocation plus the matching derivation, and
`MAX_WORK_FACTOR = 2` (`keystore.rs:179`) permits a *recorded-parameter* file to
cost up to ~5s per unlock by the file's own measured table (`keystore.rs:166-170`).
Repeat the call and you have a loop that does the full KDF every time and
refuses every time. The same holds for a missing `body`, a wrong-typed `body`, a
non-hex `parent`, and an unrecognised `direction` — every refusal in the change
pays for a keystore unlock first.

The SQLite open at step 4 is the same shape one step cheaper.

**Why this is a defect and not a design choice.** `wire.rs:670-676`
(`parsed_object`'s doc comment) states the principle explicitly — *"Parse the
whole request first, then act. The ordering is the requirement. […] there is
nothing to append until the last field has parsed"* — and the adapter inverts it:
the most expensive side effect in the whole path happens before the first
validation. `lib.rs:254-262` argues the Stoa is deliberately read twice so the
handler owns the real parse; that argument is sound and is not what this finding
is about. The cheap fix is to move `reject_forbidden_fields` (and ideally the
whole field parse) ahead of the keystore open, which means giving the handlers a
"validate only" entry point or hoisting the guard into `publishing` — a shape
question for `dev-writer`.

**Caveat on reachability.** The module wire is reached by the local UI, not
directly by a remote peer, so this is not remote-unauthenticated in the internet
sense. It is still attacker-influenced: the security posture in `CLAUDE.md`
treats everything crossing the module wire as untrusted, and a compromised or
buggy view can drive this at whatever rate it likes against a module whose caller
gives up after 20 seconds (`keystore.rs:132`).

Outcome:

---

## S4 — `Refusal::WrongStoa` discloses the actual Stoa of an op the caller only named by id

**For:** `spec-writer` (decide whether this is intended), `dev-writer` if not

**Severity: LOW / informational.** Recorded for a decision, not asserted as a
defect.

**Where:** `authoring.rs:105-109` and `authoring.rs:133-143`
(`Refusal::WrongStoa`'s `actual` field and its `Display`), raised at
`authoring.rs:287` and `authoring.rs:344`.

A caller that guesses or harvests an op id, and names any Stoa at all, is told in
the refusal which Stoa that op **actually** belongs to
(`"the {what} belongs to Stoa {actual} and the request names Stoa {requested}"`).
Combined with `Refusal::NotHeld` versus `Refusal::TargetIsNotAPost`, the three
refusals are a usable oracle over the local store: *do you hold op X*, *is it a
post*, *which Stoa is it in*.

Within this architecture the caller is the local view, which is entitled to read
the store anyway, so this crosses no trust boundary today and the distinction
between the refusals is a genuine spec requirement
(`specs/content-authoring/spec.md:236`). It is worth recording because the
moment any of these handlers becomes reachable by something less privileged than
the local view — a remote RPC, a multi-user host — `actual` becomes a
cross-Stoa read for a caller that could not otherwise perform one, and
`moderation.rs`'s care about cross-Stoa leaks (`log/sqlite.rs:812-814`,
*"a prefix-matching restricted read is a cross-Stoa leak in a
censorship-resistant forum"*) shows the project already treats that as a real
category. A one-line note in the spec saying the disclosure is deliberate and
scoped to a same-privilege caller would settle it.

Outcome:

---

## Areas checked and found clean

Stated with the measurement, not as padding.

- **Lead 2 — authorship cannot be supplied by a caller. Verified and clean.**
  I enumerated every field actually read from the inbound JSON rather than
  trusting the refused list: `publish_post` reads `stoa` and `body` only
  (`wire.rs:735-742`); `publish_reply` reads `stoa`, `parent`, `body`
  (`wire.rs:775-786`); `publish_vote` reads `stoa`, `target`, `direction`
  (`wire.rs:818-829`). There is no other `parsed.get` on any publish path. The
  author is set at `authoring.rs:198`, `authoring.rs:300` and `authoring.rs:354`
  as `key.public_key()` from the `&SecretKey` **function argument**, which no
  JSON field reaches — `authoring.rs:163-165` is right that this is
  un-parameterisable at the wire. `reject_forbidden_fields` (`wire.rs:617`) is
  called from all three handlers (`wire.rs:732`, `wire.rs:772`, `wire.rs:815`),
  and `a_forbidden_field_is_refused_on_every_operation` (`wire.rs:2078`) checks
  all three rather than one. The `FORBIDDEN_FIELDS` list is defence-in-depth over
  a property that already holds structurally, which is the right order of those
  two.
  The caller does control which Stoa, and therefore which derived key
  (`keystore.rs:640`); that is the per-Stoa identity design of PLAN.md §5.2 and
  not a finding.

- **Hex and length parsing is strict and total.** `Address::from_hex`
  (`identity.rs:119`) and `OpId::from_hex` (`op.rs:168`) both `hex::decode` then
  `try_into` a `[u8; 32]`, returning `NotHex` or `WrongLength` — no slicing, no
  indexing, no truncation. I fed the full set through the handlers: empty, 1, 63,
  64, 65, 128 chars, non-hex `z`/`g`, embedded spaces, a `0x` prefix, zero-width
  characters, non-ASCII. All refuse. `OpIdError::WrongLength(n)` reports a byte
  count from `bytes.len()` after a successful decode, so it cannot overflow.

- **No reachable panic found in `authoring.rs`.** `thread_of` (`authoring.rs:239`)
  uses `unwrap_or(id)` and its `_ => id` arm is an answer rather than a
  `unreachable!()`, which is the right call and the comment at
  `authoring.rs:242-245` gives the reason. No indexing, no slicing, no
  arithmetic anywhere in the file. Every `unwrap`/`expect`/`panic!` in
  `authoring.rs` is inside `#[cfg(test)]`.

- **No reachable panic found on the publish path in `wire.rs`.** The one
  `unwrap_or_default()` (`wire.rs:3029`) is in a test. `error_json`
  (`wire.rs:17`) goes through `serde_json::json!`, so an attacker-influenced
  message cannot produce malformed JSON — pinned by
  `guard_output_survives_a_panic_payload_containing_json_metacharacters`
  (`wire.rs:981`).

- **Lead 3 — I read the adapter as unguarded by every gate, and the two things
  outside `core::guarded` cannot panic.** `lib.rs:285-291` (the
  `persistence_path` check and `PathBuf::from`) sits outside the guard; both are
  infallible. Everything that can panic — the JSON parse, the keystore open, the
  SQLite open, the handler, the sink — is inside `core::guarded(method, …)` at
  `lib.rs:293`. The nested guard inside each handler is harmless. `S3` above is
  the finding that came out of reading this file; there is no unguarded panic in
  it.

- **No path traversal from caller input into the keystore.** The keystore
  filename is a fixed `"identity.key"` (`keystore.rs:381`) joined onto the
  host-supplied persistence path; no request field reaches it. `read_checked`
  (`keystore.rs:984`) checks mode on the **handle** not the path, checks the
  parent directory, and caps the file at `MAX_KEYSTORE_LEN` before reading — so
  it will not ingest a large file or block on a FIFO. This path is well hardened.

- **Signatures are verified where verification is claimed, and a publish buys an
  op nothing.** `a_published_op_is_verified_on_read_like_any_other`
  (`authoring.rs:1256`) tampers with a published op and shows it stops verifying;
  `an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives`
  (`authoring.rs:1188`) pins that the log stores what the publish path refuses.
  The module doc at `authoring.rs:25-42` is correct that these checks are
  outbound-only and establish nothing about an inbound op — that framing is the
  right one and is unusually well defended.

- **Nothing is appended or delivered before it is validated, within `core`.**
  `deliver` sits on the success arm of the `Result` at `wire.rs:746`,
  `wire.rs:790` and `wire.rs:834`, so no refusal path reaches it; the append
  completes inside `authoring::publish` (`authoring.rs:173`) before that. The
  return type `()` on the sink makes a delivery outcome structurally unwaitable.
  (The adapter-level ordering problem is S3, which is about cost, not about
  appending unvalidated data.)

- **No secret comparison on this change's paths.** Nothing in `authoring.rs`,
  `wire.rs`'s publish section or the adapter compares key material. `Address`
  and `OpId` equality is over public identifiers.

- **No new dependencies.** `git show --stat` on the change touches no
  `Cargo.toml`; `hex`, `serde_json`, `sha2`, `argon2` were all already present.

- **`cargo mutants` was not run.** Out of scope for a security-only pass and the
  other reviewers own mutation coverage; S1 and S2 were established by targeted
  hand mutations instead, each named above with its result.

---

## Tree state

Every mutation I made was to
`dialectica/rust-lib/dialectica-core/src/wire.rs` and was reverted with
`git checkout --` on that path. `git status --short` in
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-security`
reports nothing but this findings file. The gitignored
`dialectica/logos-rust-sdk-src` symlink was recreated as instructed and is not
committed.
