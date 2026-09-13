# Readability review — `authoring-content`

Scope: **readability only**. Correctness, security and architecture-vs-design are
other reviewers'. Findings only; nothing was fixed.

Suite run before and after reading: **531 passed, 0 failed** — the stated
baseline. Nothing in the tree was mutated; `git status` is clean apart from the
gitignored SDK symlink and this file.

---

- [x] **`dev-writer`** — **R1** — `wire.rs` still carries the false-premise comment `tasks.md` §10 retracted
      **Verified on conversion** (2026-09-13): the comment now says the ordering IS
      observable, names the test that observes it, explains the `Rc<RefCell<Vec<_>>>`
      mechanism, and records that the old claim was load-bearing because it was the
      stated reason the weaker test was accepted as sufficient.

**For: `dev-writer`.**
`dialectica/rust-lib/dialectica-core/src/wire.rs:2406-2413`, inside
`delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal`.

The comment reads:

```
// What this test **cannot** see, and saying so is the point: that the
// append happened BEFORE the sink was called. The sink cannot read the
// log to check, because the handler holds it mutably for the duration —
// so no test through this API can observe the ordering directly. What
// pins it instead is that `crate::authoring::publish` returns a
// `Published` only after its `append` has returned `Ok`, and the sink
// sits after that call. The structural argument is the evidence; this is
// the observable half.
```

**Evidence the premise is false, from the same file.** `tasks.md` §10 records
this exact claim as wrong and retracted — "**~~That the append precedes
delivery.~~ This was wrong, and the tester proved it**" — and the disproof lives
120 lines below the comment, in the same module:

- `wire.rs:2464-2481`, `JournallingLog`'s own doc: *"`tasks.md` §10 records 'that
  the append precedes delivery' as something no test through this API can see …
  That is true of the *log*, and it is not true of the *ordering*"*;
- `wire.rs:2526`, `the_append_completes_before_delivery_is_invoked_on_all_three_handlers`,
  which asserts a hardcoded `["append", "deliver"]` per handler.

So the file now contains both the claim and its refutation, and the claim is the
one a reader meets first (it sits on the earlier test). This is precisely the
shape `tasks.md` §10 warns about in its own words: *"A note saying a property is
unobservable is a note that stops anyone looking for a way to observe it, so it
has to be right."* The note was load-bearing once — it licensed the weaker test
it sits on — and leaving it in place re-arms that licence for the next reader,
who would read "no test through this API can observe the ordering directly" and
not go looking for the journal.

Severity: **high** for a readability defect. The retraction was written into
`tasks.md` and not into the code the retraction is about.

Outcome: **fixed.** The comment now says the ordering *is* observable, names
`the_append_completes_before_delivery_is_invoked_on_all_three_handlers` as the
test that observes it, and records that the old claim was load-bearing while it
stood. No test asserts on a comment, so nothing fails without this; the
correctness reviewer reached the same finding independently (C4), which is the
corroboration standing in for a test here.

---

- [x] **`dev-writer`** — **R2** — `design.md` names three request structs that do not exist
      **Verified on conversion** (2026-09-13): same edit as architecture A3's
      documentation half. `design.md` now describes what each handler actually reads and
      says the three structs were planned and not built; grepping the names finds only
      that sentence.

**For: `dev-writer`.**
`openspec/changes/authoring-content/design.md:39-40`.

> Each handler parses its request into one of three small structs
> (`PostRequest`, `ReplyRequest`, `VoteRequest`) and only then calls into
> `authoring.rs`.

**Evidence.** `grep -rn "PostRequest\|ReplyRequest\|VoteRequest"` over
`dialectica/` and `openspec/` returns only these two lines of `design.md`. The
handlers parse field by field into local bindings (`wire.rs:735-742`, `775-786`,
`818-829`) and pass those positionally to `authoring::post`/`reply`/`vote`.

Why it matters beyond tidiness: the named structs are the *mechanism* design.md
gives for the parse-then-act decision in the very next sentence — *"under
parse-then-act it is structural, because there is nothing to append until every
field has been read"*. A reader who trusts the paragraph goes looking for a type
that would make that structural, does not find one, and is left unable to tell
whether the property was dropped or the prose is stale. It is the prose: the
property does hold, by the statement order in each handler, and `wire.rs:670-676`
records that correctly. design.md describes a shape that was planned and not
built.

Outcome: **fixed.** The entry now describes what each handler actually reads, and
says explicitly that the three named structs were planned and not built, what
holding one would have bought (evidence every field parsed), and that the
requirement consequently holds three times over rather than by construction. The
architecture reviewer independently reached the same place from the other
direction (A3), and the reshape is now recorded as a live option there rather
than as done.

---

- [x] **`dev-writer`** — **R3** — `design.md` rests a requirement on a `let _` and a `FnOnce` the code does not have
      **Verified on conversion** (2026-09-13): the bogus `let _` reasoning is gone
      (grep finds no `let _` in `design.md`), the three requirements are re-assigned to
      the mechanisms that actually carry them, and the `FnOnce` dead end is recorded in
      Decisions including the A1 correction.
      **One update since that outcome was written:** the snippet no longer shows a bare
      `deliver(&published.id);`, because the panicking-sink fix moved the handoff into
      `delivered_and_published`. The snippet was updated in the same change, so it still
      shows the real code — which is the property this finding was about.

**For: `dev-writer`.**
`openspec/changes/authoring-content/design.md:64-74`.

The code block reads `let _ = deliver(&published.id);   // outcome discarded`,
and the prose then assigns requirements to those exact tokens:

> "A declined handoff leaves the op published" is the `let _`. "A publish returns
> while delivery is still outstanding" is that `deliver` is a `FnOnce(&OpId)`
> returning `()`.

**Evidence both halves are false.** The sink is `&mut dyn FnMut(&OpId)`
(`wire.rs:725`, `765`, `806`) and is called bare, not bound: `deliver(&published.id);`
(`wire.rs:746`, `790`, `834`). `tasks.md` §8 records the `FnOnce` → `&mut dyn FnMut`
change and *why* — a higher-ranked-lifetime coercion failure in the adapter — and
explicitly states *"Nothing a requirement rests on is lost: the return type `()`
is what makes a delivery outcome unwaitable, and `FnOnce` only added
at-most-once."* design.md was not updated to match, so it still hangs a
requirement on the discarded half.

The `let _` half is worse than stale, because it is a reason that was never
right: a `let _` on a `()`-returning call discards nothing (it is a no-op, and
clippy-adjacent noise). What actually satisfies "a declined handoff leaves the op
published" is the sink's `()` return plus the append having already completed —
which `wire.rs:683-696` states correctly. A reader who acts on design.md would
"restore" a `let _` believing a requirement depends on it.

Outcome: **fixed.** The snippet now shows the real `match` with a bare
`deliver(&published.id);` and the comment "returns (), nothing to discard", and
the three requirements are re-assigned to the mechanisms that actually carry
them: statement order inside the `Ok` arm, `deliver` being named only on the `Ok`
arm, and the sink's `()` return. The `FnOnce` → `&mut dyn FnMut` dead end is now
recorded in Decisions beside it, which is where design-review F1 said it belonged
— including the correction that the adapter does *not* force the erasure.

---

- [x] **`dev-writer`** — **R4** — `Dialectica::publishing` does two jobs, and its own doc heading says so
      **Rejected, with the argument; verified on conversion** (2026-09-13). A fixture's
      job *is* the assembly, and splitting it into five would spread key/store ordering
      across the one file no gate compiles — the code's own comment gives that reason,
      and it holds. The reader cost the finding identified was not dropped: it is
      recorded as a decision in `design.md` ("The adapter reads `stoa` twice, and the
      second parser is the authority"), which names the cost that a malformed `stoa` may
      be reported by either parser.

**For: `dev-writer`.**
`dialectica/rust-lib/src/lib.rs:251-252`, `276`.

The doc comment is *"Assemble the keystore, the key, the store and the delivery
sink, then run one publish handler"* — an `And` in all but the conjunction, and
the tell CLAUDE.md names. The body then does five distinguishable things before
the handler: the persistence-path guard (285-290), a second JSON parse to recover
the Stoa (296-307), the keystore open and no-identity refusal (320-324), the
store open (327-330), and the sink (332-341).

Reported as readability rather than architecture because the reader cost is
specific: the function's name says "publishing", so a reader tracing *why a
publish reported "missing field: stoa"* has to discover that the message can come
from either of two parsers — this one at `lib.rs:306` or `required_string` at
`wire.rs:635` — and the doc's own §"The Stoa is read twice" is the only thing
that says so. That section is sound and earns its place (it argues why the
double-read is not a redundancy to remove); the problem is that the function
holding both jobs is the one no gate compiles, so the ambiguity is unchecked.

Note this is also the file the task brief flags as prose no gate reads. I checked
the whole file: the `cfg(logos_scaffold)` reasoning (176-204), the
`RustModuleContext` stand-in (29-48) and the `no_identity` note (314-319) are all
sound against the code and against `build.rs`'s stated constraint.

Severity: **medium**, stylistic-to-structural boundary. Flagged, not asserted as
a must-fix.

Outcome: **rejected**, and the reviewer's own framing is why. "Assemble X, then
run one handler" is the shape of a fixture, and a fixture's job *is* the
assembly — the alternative is three copies of it, which the comment at
`lib.rs:246-248` rejects for the reason that three copies are three places to
forget the key or open the store in the wrong order. Splitting the assembly into
five functions would spread the key/store ordering across more of the one file no
gate compiles, which trades a reading cost for a correctness risk in the worst
possible place.

The genuine reader cost the finding identifies — that "missing field: stoa" can
come from either of two parsers — is real and is not fixed by splitting the
function; it is a consequence of the deliberate double-read, which the finding
itself agrees earns its place. Recorded instead as design-review F4(d), where the
double-read is now a named decision rather than only a code comment.

Flagged rather than asserted, and declined on that basis. If the adapter grows a
sixth assembly step this should be revisited.

---

- [x] **`dev-writer`** — **R5** — `thread_of`'s fallback comment contradicts what the line returns
      **Verified on conversion** (2026-09-13): the "it belongs to no thread" clause is
      gone. The comment now says answering with `id` treats the op as its own thread
      root, which is the same answer a root post gets, and keeps the unreachability half
      — which still checks out, since `reply` refuses a non-post parent before
      `thread_of` is reached.

**For: `dev-writer`.**
`dialectica/rust-lib/dialectica-core/src/authoring.rs:242-245`.

```rust
// Not a post, so it belongs to no thread. Unreachable from `reply`,
// which refuses a non-post parent before it gets here — but an answer
// rather than a panic, because a panic aborts the module process.
_ => id,
```

"It belongs to no thread" and `=> id` say opposite things: the arm returns the
op's *own id as its thread*, which is what a thread **root** looks like, not what
"no thread" looks like. A reader reconciling the two has to decide whether the
arm is a deliberate "treat it as its own root" or a placeholder, and the comment
pushes toward the second.

The rest of the comment is **sound** and I verified it: `reply` refuses a
non-post parent at `authoring.rs:283` before reaching `thread_of` at `294`, and
`grep` confirms `thread_of` has exactly one caller — so the arm genuinely is
unreachable, and the reason for answering rather than panicking is the right one.
Only the first clause misdescribes the value.

Severity: **low**. One clause.

Outcome: **fixed.** The clause "it belongs to no thread" is gone; the comment now
says answering with `id` treats the op as its own thread root, which is the same
answer a root post gets and so introduces no shape a caller has not already seen.
The unreachability half, which the reviewer verified as sound, is kept unchanged.

---

- [x] **`tester`** — **R6** — A test comment claims two functions were called that were not
      **Verified on conversion** (2026-09-13): the overstated "through the two real
      functions" sentence is gone. The comment now states what the test reaches (the
      publish path agreeing with `stoa_address`'s composition from the same root) and
      what it does not (`Keystore::stoa_address` itself being changed to compose
      differently), and says why closing that would mean reaching a real `Keystore` this
      layer deliberately does not take. Comment-only, as the outcome claims: the test
      body is unchanged.

**For: `tester`.**
`dialectica/rust-lib/dialectica-core/src/authoring.rs:507-508`, in
`the_signing_identity_is_the_one_the_capability_probe_reports`.

> The probe's own lookup is the keystore's `stoa_address`, so both sides are
> computed here from the same root through the two real functions.

**Evidence.** The test hands `capability_for` a closure that computes
`derive_stoa_key(&A_ROOT, s).public_key().address().to_hex()`
(`authoring.rs:511`). `Keystore::stoa_address` is never called. The *value* is
identical — `keystore.rs:651-653` defines `stoa_address` as
`stoa_public_key(stoa).address()`, and `640-642` defines `stoa_key` as
`derive_stoa_key(&self.root, stoa)`, so the closure reproduces the composition —
but "through the two real functions" is a stronger claim than the test makes, and
it is the claim that matters here: a reader checking whether the probe and the
publish path can drift would conclude the keystore's own accessor is under test
on this path. It is not; it is re-implemented in the fixture. If
`Keystore::stoa_address` were changed to compose differently, this test stays
green.

The surrounding reasoning is sound — the probe/publish disagreement it describes
is real and worth a test, and the test does catch a publish path that signed with
a different key. Only the "two real functions" sentence overstates.

Severity: **low-medium** — a claim about coverage, in a comment, that the
assertions do not reach.

Outcome: **fixed** (comment only; the test is the `tester`'s and is unchanged).
The comment now states what the test does reach — that the publish path agrees
with `stoa_address`'s composition from the same root, which would catch a publish
signing with a different key — and what it does not: `Keystore::stoa_address`
itself being changed to compose differently, since this layer deliberately takes
no `Keystore`. Whether to close that gap is the `tester`'s call and is not
claimed here either way.

---

- [x] **`tester`** — **R7** — `an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives` says "both shapes"; the module refuses three
      **Verified on conversion** (2026-09-13): the comment now says "two of the three
      shapes this module refuses", names which two are built (the orphan and the
      cross-Stoa reply), names the one omitted (a non-post parent), and says the point is
      the direction rather than an enumeration. The third case was deliberately not
      added, which the finding left to the `tester` as optional rather than required —
      so this is the word fixed, not a coverage gap left open.

**For: `tester`.**
`dialectica/rust-lib/dialectica-core/src/authoring.rs:1195-1196`.

> Both shapes this module refuses are built by hand, appended, and must be
> readable

The module refuses **three** shapes, each its own `Refusal` variant with its own
`Display`: `NotHeld` (`authoring.rs:275`, `339`), `TargetIsNotAPost`
(`authoring.rs:284`) and `WrongStoa` (`authoring.rs:287`, `343`). The test builds
two of them — the orphan (absent parent) and the cross-Stoa reply — and omits
the non-post parent.

"Both" reads as an exhaustiveness claim in a test whose whole subject is *"an op
a publish would refuse is stored anyway"*. A reader checking whether the
outbound/inbound asymmetry has been demonstrated for every refusal counts two,
matches the word "both", and stops. Whether the third shape is worth adding is
the tester's call; the word is the finding either way — say "two of the three" or
add the third.

Outcome: **fixed** the word, as the reviewer's first option. The comment now says
"two of the three shapes this module refuses", names which two (the orphan and the
cross-Stoa reply) and which is omitted (a non-post parent), and says the point is
the direction rather than an enumeration. Whether to add the third case is left to
the `tester`, unclaimed.

Severity: **low-medium**. A word, with an exhaustiveness implication.

---

## Comments I checked and found sound

A cleared claim is a result. Each of these was verified against the code, not
grepped:

- **`op.rs:300-315`, the corrected `thread` doc.** The replacement text — nothing
  fills `thread` in later and nothing can, because the op is signed — is
  correct: `put_option_id` (`op.rs:774-782`) writes `thread` into the signed
  preimage, and no store path rewrites it. The correction is the right one and it
  is recorded in the same change, as `tasks.md` §6 claims.
- **`authoring.rs:232-238`, the same correction restated in `thread_of`'s doc.**
  Consistent with `op.rs`, and it names the concrete harm (a reader expecting
  non-`None` on every stored root).
- **`authoring.rs:34-39`, "the log stores exactly such a reply, deliberately".**
  `a_reply_whose_parent_is_absent_is_stored` exists at
  `log/contract.rs:1238`, run against both backends (`1258`, `1263`). The
  citation is real and the test does what the comment says.
- **`authoring.rs:533-554`, the pinned-seed test's reasoning.** Both hardcoded
  values match `identity.rs::the_wire_constants_are_pinned_to_known_answers`
  exactly: the seed `b62b6b59…26cf` is that test's pinned
  `derive_stoa_key([7;32], stoa_address(b"a genesis record"))`
  (`identity.rs:653-656`), and the Stoa address `6b1f1c28…9cd8` is its pinned
  `stoa_address(b"a genesis record")` (`identity.rs:642-644`). The comment's
  account of *why* the sibling test cannot see the derivation moving is correct,
  and the "do NOT update the expected value" instruction is warranted.
- **`authoring.rs:328-330` and `wire.rs:529-533`, "nothing reads a `Vote` op —
  `feed.rs` says so in as many words".** `feed.rs:73-75` reads *"**No vote
  score.** … Nothing reads `Vote` ops, so no row carries a score."* The citation
  is verbatim-accurate, which is the failure mode this project has hit before.
- **`authoring.rs:169-172`, the `Arrival::unordered()` note.** The reason given
  (this op did not arrive, so claiming a Lamport value would be self-asserted
  ordering) matches the call and matches CLAUDE.md's SDS section.
- **`wire.rs:548-566`, `no_identity`'s reason for existing.** Verified by grep
  over the whole of `dialectica/`: `Refusal::NoIdentity` is constructed in
  exactly two non-doc places — `wire.rs:568` and the assertion at `2826` — and
  `lib.rs:323` calls `core::no_identity` rather than formatting its own text. The
  "one home for the wording" claim holds, and
  `the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text`
  (`wire.rs:2802`) does assert the two are one string rather than two that agree.
- **`wire.rs:698-720`, the `&mut dyn FnMut` vs `impl FnOnce` note.** The
  signature it describes is the one in the code (`725`, `765`, `806`), and
  `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`
  (`3098`) uses a concrete `fn` pointer type, which is what would fail under
  `impl FnOnce`. Note this is the note design.md contradicts — finding 3 — so the
  code comment is the correct copy.
- **`wire.rs:670-676`, "parse the whole request first, then act".** Each handler
  does read every field before `authoring` is reached; the statement order backs
  the claim.
- **`wire.rs:3016-3025`, "`is_object()` ALONE cannot see a panic".** Correct:
  `guarded` (`wire.rs:50-66`) returns `{"error":"panic in <method>: …"}`, a
  well-formed object, so the `starts_with("panic in ")` assertion is genuinely
  the only thing distinguishing a caught panic from a refusal. The comment
  earns its place by saying what the assertion cannot.
- **`wire.rs:2968-2979`, the valid-Stoa op-id cases and why they were needed.**
  The reasoning — every earlier case malformed the Stoa, which is parsed first in
  all three handlers, so the op-id parser saw only well-formed hex — matches the
  handlers' statement order (`stoa` before `parent`/`target` in all three).
- **`authoring.rs:1343-1346`, `a_maximal_body_publishes_rather_than_panicking`.**
  The at-cap value is right (`MAX_FIELD_LEN = 150 * 1024`, `op.rs:132`), and the
  hardcoding is forced rather than sloppy: the constant is private to `op.rs`.
- **`tasks.md` §10's own retraction and §11's two undischarged requirements.**
  Accurate as written; §11's account of why the no-identity *trigger* is
  untestable here matches `build.rs`'s constraint and the `cfg` gate.
- **Test names against assertions.** I read every `#[test]` in `authoring.rs` and
  every publish-path test in `wire.rs`. Names match what is asserted, including
  the two whose names make a precise distinction the body honours —
  `the_same_body_in_two_stoas_is_two_ops_with_the_identity_held_fixed`
  (`authoring.rs:673`, holds the key fixed, which is the point of its existing
  alongside the covarying leg) and
  `two_replies_to_two_siblings_in_one_thread_are_two_ops` (`authoring.rs:708`,
  holds the thread fixed). No misnamed test found.
- **Dead or vestigial code.** None found in the new material. `thread_of`'s
  `_ => id` arm is unreachable but required (it is a non-exhaustive match on
  `OpKind`, and a panic there would abort the module process) — finding 5 is
  about its comment, not the arm.

## Areas clean

- Names in `authoring.rs`: `post`, `reply`, `vote`, `publish`, `thread_of`,
  `Published`, `Refusal`, `was_new`. Each says what it does and no more. `publish`
  is the one private function and its "sign, append, report" doc matches its three
  statements.
- The `Refusal` variants and their `Display` arms: each message names the
  identifier the refusal is about, and `a_refusal_names_the_id_or_stoa_it_is_about`
  (`authoring.rs:1153`) pins that as a shape once so no other test pins wording.
- `FORBIDDEN_FIELDS` and `reject_forbidden_fields` (`wire.rs:588-624`): the
  "one guard over a list" doc is accurate, the `NO SPEC:` marker is where
  `tasks.md` §9 says it is, and the guard is called from all three handlers.
- The spec (`specs/content-authoring/spec.md`): no unobservability or
  impossibility claim a reader could inherit. I grepped and read the thread and
  cross-Stoa requirements; the prose matches the implemented behaviour.
