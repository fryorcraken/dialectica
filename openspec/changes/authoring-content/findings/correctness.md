# Correctness review — `authoring-content`

Reviewed dimension: **correctness only.** Security, readability and architecture
are other reviewers'. Where a finding below has a security flavour (C1 is a
remote-triggerable self-DoS) it is reported here because the defect is an
encode/decode asymmetry the publish path introduces, which is a correctness
question; the security reviewer may reach it independently.

Baseline: `cargo test -p dialectica -p dialectica-core` = **531 passed**, before
and after every mutation below. The tree was restored after each; confirmed
clean with `git status --short` returning nothing.

---

- [x] **`dev-writer`** — **C1** — A body one byte over `MAX_FIELD_LEN` publishes, then permanently bricks every feed read on the peer
      **Verified on conversion** (2026-09-13): `MAX_FIELD_LEN` is `pub` at
      `op.rs:146`; `authoring::MAX_BODY_LEN` at `authoring.rs:206` is defined as
      it, not a second literal; `the_publish_body_cap_is_the_format_field_cap`
      (`authoring.rs:1472`), `a_body_one_byte_over_the_cap_is_refused_rather_than_signed`
      (`authoring.rs:1482`, covering both `post` and `reply`) and
      `every_op_a_publish_produces_decodes_again` (`authoring.rs:1523`) all exist.
      The `spec-writer` note at the end is separately still open — see the
      unticked box below.

**For:** `dev-writer` (the fix is code), with a `spec-writer` note at the end.

**Severity: HIGH.** Self-inflicted denial of service, reachable from a single
wire request whose content the caller fully controls. No panic, so every
existing guard and sweep reports success.

**The defect.** `MAX_FIELD_LEN` (153,600 bytes) is enforced on **decode** only —
`op.rs:798` `take_length_within_cap`. The encode side does not check it:
`op.rs:754` `put_bytes` writes `bytes.len() as u32` with a comment saying the cast
"cannot truncate meaningfully: `MAX_FIELD_LEN` is checked on the way back in".
That reasoning holds for ops that *arrive*; it does not hold for ops this peer
*creates*, which is exactly what this change added. Nothing in `authoring.rs` or
`wire.rs` caps `body`:

- `wire.rs:739` `required_string(&parsed, "body")` — any length accepted
- `authoring.rs:187` `post` — passes `body` straight into `OpKind::Post`
- `authoring.rs:267` `reply` — same

**Failure scenario, measured end to end against the production store
(`SqliteOpLog`, not `MemoryOpLog`):**

```
publish_post {"stoa":"<valid>","body":"readable"}
  -> {"opId":"66ce5836…","wasNew":true}
  iter_stoa -> Ok(1)                                    # feed works

publish_post {"stoa":"<valid>","body":"x" * 153601}     # MAX_FIELD_LEN + 1
  -> {"opId":"72e9faaa…","wasNew":true}                 # reported as published

  iter_stoa -> Err(CorruptEntry("the stored op did not decode:
                 a field claims 153601 bytes, over the 153600 cap"))
  iter      -> Err(CorruptEntry(  …same… ))
```

The over-cap op is written to SQLite (`sqlite.rs:760` stores
`entry.op.to_bytes()`), and every read decodes (`sqlite.rs:688`
`decode_entry` -> `SignedOp::from_bytes`). `ordered_read` at `sqlite.rs:650`
propagates the first decode failure with `?`, so **one row aborts the whole
read**. Because `iter()` is also poisoned, the damage is not confined to the Stoa
posted into — every Stoa's feed on that peer returns the error shape, permanently,
with no way to remove the row through any API this change or any prior change
exposes.

Second consequence, independent of the store: the op's canonical bytes are
undecodable by *any* peer, so `wasNew: true` reports a publish that no peer can
ever accept. `SignedOp::from_bytes(stored.to_bytes())` returns
`Err(FieldTooLong(153601))` — measured.

**Why nothing caught it.** Three reasons, and the third is the one to fix
structurally:

1. `authoring.rs:1342` `a_maximal_body_publishes_rather_than_panicking` tests
   `150 * 1024` — *at* the cap, which is legal — and not one past it.
2. `wire.rs:2942` does sweep `150 * 1024 + 1`, but
   `hostile_publish_input_is_never_a_panic` asserts only "is a JSON object and
   not a caught panic". An over-cap publish satisfies both by succeeding.
3. **The publish path is never tested against `SqliteOpLog`.** `grep -c Sqlite`
   is **0** in `authoring.rs` and **0** in `wire.rs`. `MemoryOpLog` stores the
   live `SignedOp` and never round-trips through bytes, so it is *structurally
   unable* to observe any encode/decode asymmetry the publish path introduces.
   This blind spot is not in `tasks.md` §10.

`cargo mutants` could not have found this either: `MAX_FIELD_LEN` is a `const`,
and mutants mutates functions, not constants.

**Note for the fixer.** `MAX_FIELD_LEN` is **private** to `op.rs`
(`const`, not `pub const`, `op.rs:132`), so the publish path cannot currently
check it. The fix therefore touches `op.rs` as well — either export the cap, or
(better, since it makes the invariant hold by construction rather than by a
fourth guard) make the encode side fallible so `Op::canonical_bytes`/`sign`
cannot produce bytes that `decode` would refuse.

**Regression test that must fail before the fix:** publish a body of
`MAX_FIELD_LEN + 1` through `publish_post` against a `SqliteOpLog`, assert the
reply is the error shape, and assert `iter_stoa` still returns `Ok`.

**Outcome: fixed.** Found independently by three reviewers — you, `security.md` S1
and `spec-test.md` entry 3 while blind to the implementation — which is why it went
in first and with the most care. `findings/security.md` S1 carries the full account
of the fix; the parts that answer *your* framing specifically:

- **Your note for the fixer was the load-bearing one.** `MAX_FIELD_LEN` was private,
  so the publish path could not check it. It is now `pub`, with a doc comment saying
  why a writer needs the number the decoder enforces — and
  `the_publish_body_cap_is_the_format_field_cap` pins `authoring::MAX_BODY_LEN` to it
  so they cannot become two constants that agree today. That is your "export the
  cap" option; I did not take the fallible-encoder option, because making
  `canonical_bytes`/`sign` return a `Result` would put a failure case on every op
  construction in the crate to catch one caller's input, and the boundary that can
  *tell* the caller is the publish path.
- **The regression test is the shape you specified**, minus the `SqliteOpLog`:
  `a_body_one_byte_over_the_cap_is_refused_rather_than_signed`, proven to fail before
  the fix with `Ok(Published { id: 72e9faaa…, appended: Stored })` — the same op id
  you and the security reviewer both measured, which is a pleasing three-way
  agreement. It covers `post` and `reply` and asserts the log is untouched.
- **Your reason 3 is the one I acted on structurally**, because it is the class and
  not the instance: the publish path never round-tripped bytes at all.
  `every_op_a_publish_produces_decodes_again` now goes through
  `to_bytes`/`from_bytes` explicitly, and its comment is honest that it does *not*
  reproduce this bug (its longest body is at the cap, so it passes with the guard
  disabled) — it guards the invariant for the next field. The blind spot itself is
  now written into `tasks.md` §10, where you correctly noted it was missing.
- The sweep also gains the closing assertion `security.md` S2 asked for, so a
  *wrongful success* is now visible to it and not only a panic. Proven to fail before
  the fix: `FieldTooLong(153601)`.

Not done: a `SqliteOpLog` fixture. Every publish test uses `MemoryOpLog`, and adding
the production store to this layer is a test-suite decision for the `tester` — the
byte round-trip closes the same class without it, and your measurement against the
real store is recorded here as the evidence rather than re-created.

**Your `cargo mutants` note is worth preserving**: 14 mutants, 0 missed, and it still
could not have found this, because the cap is a `const` and mutants mutates
functions. That is the second time this repo has recorded mutants being blind to a
`const`.

**`spec-writer` note.** No requirement in
`specs/content-authoring/spec.md` says an over-long body is refused. "An empty
body SHALL be accepted" contracts the lower bound and nothing contracts the
upper. The spec should say which of the two it wants — refuse at the boundary, or
contract that `op-format`'s cap is the publish path's cap — because at present
"Hostile input is never a panic" is the only requirement in the area and it is
satisfied by the defective behaviour.

- [x] **`spec-writer`** — **C1's spec note** — no requirement bounds a body from above
      **Still open, verified on conversion** (2026-09-13): grepped
      `specs/content-authoring/spec.md` for a cap, length or upper bound — there is
      none. The code refuses at `op-format`'s cap, so the behaviour a caller can
      already depend on is unspecified. Same gap as `findings/spec-test.md` entry 3.

      **Fixed** (`spec-writer`). You offered two options and I took the second — "contract
      that `op-format`'s cap is the publish path's cap" — because it is the one the code
      implements, and a spec pass exists to ratify behaviour rather than to invent a
      second number. The requirement "A post names a Stoa and carries a body" now
      carries, beside the empty-body paragraph whose asymmetry caused this:

      > **A body SHALL be bounded from above by the same cap `op-format` enforces on a
      > variable-length field**, and the two SHALL be one value rather than two that
      > agree. A body over that cap SHALL be refused before the op is signed or
      > appended, and the refusal SHALL name both the length supplied and the cap.

      Four scenarios, each checkable through the API: a body at the cap publishes and
      its canonical bytes decode again; a body one byte over is refused, appends
      nothing and does not invoke delivery; a reply's body is bounded by the same cap;
      and the publish cap and the format's field cap are one number. The last is the
      scenario for `the_publish_body_cap_is_the_format_field_cap`, so the constant
      pairing you asked for is now contracted and not only tested.

      **Your closing sentence is the one I acted on.** *"Hostile input is never a panic"
      is the only requirement in the area and it is satisfied by the defective
      behaviour* — that was true, and leaving it true would have meant the new bound
      could still be read as satisfied by a non-panicking success. So that scenario now
      also asserts that an over-cap body among the hostile inputs is **refused rather
      than published**, "so that not-a-panic is not read as a licence to accept it".
      Naming the requirement's own weakness in the requirement is the only part of this
      that goes beyond ratifying the code.

      No behaviour change requested: the code's choice was right, and `MAX_BODY_LEN`
      being defined *as* `MAX_FIELD_LEN` is what the fourth scenario contracts.

**Outcome of the `spec-writer` note: routed, still open.** The code now refuses at
`op-format`'s cap, so the spec has a behaviour to ratify rather than a blank — but the
choice between your two options ("refuse at the boundary" versus "contract that
`op-format`'s cap is the publish path's cap") is unmade, and the second is the one the
code implements. Until the spec says so, this is unspecified behaviour a caller can
already depend on.

Your last sentence is the one for the spec-writer to read first: *"Hostile input is
never a panic" is the only requirement in the area and it is satisfied by the defective
behaviour.* The over-cap publish was never a panic. That is why no requirement was
violated while a store was being corrupted, and it is a good argument that the
requirement is the wrong shape.

The same gap is raised from the spec side by `findings/spec-test.md` entry 3, which
found it from the spec's asymmetry alone — the empty end of the body is argued at
length and the other end is silent.

---

- [x] **`tester`** — **C2** — `a_refused_publish_reaches_neither_the_append_nor_delivery` does not reach the cross-Stoa refusal it says it covers
      **Verified on conversion** (2026-09-13): the fourth case at `wire.rs:2667`
      seeds a post into `stoa`, clears the journal so the seed's append is not
      counted, then votes on that seed naming `elsewhere` — the prescription
      exactly. The per-case message assertion is present, keyed on the four
      fragments named. The stale "three refusals" comment now reads four at three
      depths.

**For:** `tester`.

**Severity: MEDIUM.** A stated coverage claim that is false, of the family
`MEMORY.md` records as this project's recurring defect: two explanations give the
same answer on the fixture chosen.

**The defect.** `wire.rs:2600-2604` says the cases "sit at three different
depths: one fails in the parse …, one in `authoring`'s own presence check, and
one in its cross-Stoa check". The fourth case (`wire.rs:2636-2641`) names
`elsewhere` as the Stoa and `absent` as the target against an **empty log**, so
`log.get(&absent)` returns `None` and it refuses at `NotHeld` — the *same* code
path and the *same* depth as case 3. Measured, by printing each refusal:

```
a forbidden field:                   "author is not accepted: …"
an unrecognised direction:           "direction must be \"up\" or \"down\", …"
an absent parent:                    "this peer does not hold the parent 9b9b…"
an absent target in another Stoa:    "this peer does not hold the target 9b9b…"
                                      ^^^ NotHeld, not WrongStoa
```

So **no case in this test reaches `Refusal::WrongStoa`**, and the requirement
"a publish that is refused … delivery was not invoked" is unasserted for the
cross-Stoa refusal. A handler that called the sink on the cross-Stoa path only
would leave this test green. The `authoring.rs` cross-Stoa tests
(`a_cross_stoa_reply_is_refused_and_a_same_stoa_one_publishes`,
`a_cross_stoa_vote_is_refused`) assert `log.len()` and so cover the append, but
they take no sink and cannot cover delivery.

**Fix:** the fourth case needs a target that **is** held, in a different Stoa —
seed a post into `stoa`, then vote naming `elsewhere` with that seed's id. Then
the comment's "three depths" becomes true.

**Outcome: fixed, exactly as prescribed, plus a guard against it recurring.**

The fourth case now seeds a post into `stoa`, clears the journal so the seed's own
append is not counted as the refusal's, and votes on that seed's id naming
`elsewhere`. The comment is corrected to "four refusals at three depths — two in
the parse", which is what the fixtures actually deliver.

**The part worth more than the fix.** Every case now also asserts the refusal
*message* rather than only that some refusal happened, keyed to the reason the case
exists: `"author"`, `"direction"`, `"does not hold"`, `"belongs to Stoa"`. Without
that, this fixture can silently degrade back into a duplicate of the case above it
— which is precisely what had happened — and nothing would notice.

Proven: restoring the old absent-target fixture now fails with
`expected a refusal mentioning "belongs to Stoa", got {"error":"this peer does not
hold the target 9b9b…"}`. Before this change that substitution was invisible.

This is the project's recurring defect family in its purest form — two explanations
giving the same answer on the fixture chosen — and it is the second instance found
in this one test file. Worth noting for `MEMORY.md`: the discriminator here is not a
better fixture but an assertion on *which* refusal, and that pattern generalises to
any table-driven test whose rows are meant to exercise different paths.

---

- [x] **`tester`** — **C3** — The over-cap boundary literal is a second copy of `MAX_FIELD_LEN`
      **Verified on conversion** (2026-09-13):
      `a_maximal_body_publishes_rather_than_panicking` (`authoring.rs:1453`) builds
      its body from `MAX_BODY_LEN` rather than `150 * 1024`, and both new boundary
      tests use the constant. `the_field_cap_is_pinned_to_a_known_answer`
      (`op.rs:1657`) still pins the literal, which is where a hardcoded expectation
      belongs.

**For:** `tester`.

**Severity: LOW.** Contributes to C1 rather than being independent.

`authoring.rs:1351` writes `"x".repeat(150 * 1024)` and its comment names
`MAX_FIELD_LEN`, but the value is a literal. `op.rs:1662` pins
`MAX_FIELD_LEN == 150 * 1024`, so a drifted cap is caught *there* — but this
test would then silently be testing a body that is no longer at the boundary,
which is precisely what a boundary test exists to avoid. `op.rs:1689` and
`op.rs:1703` reference the constant; this test cannot, because the constant is
private. If C1's fix exports the cap, this literal should become a reference to
it.

**Outcome: fixed**, and your conditional is what made it free — C1's fix exports the
cap, so the literal became a reference in the same commit.
`a_maximal_body_publishes_rather_than_panicking` now builds its body from
`MAX_BODY_LEN`, and its comment records why: as a literal, a drifted cap would have
silently moved this test off the boundary it exists to sit on, while
`the_field_cap_is_pinned_to_a_known_answer` kept passing over in `op.rs`.

The two new boundary tests use the constant for the same reason, so the "at the cap"
and "one byte over" pair move together by construction rather than by someone
remembering to edit both.

---

- [x] **`dev-writer`** — **C4** — A stale comment asserts the ordering is unobservable, in the same file as the test that observes it
      **Verified on conversion** (2026-09-13): the comment in
      `delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal` now
      states the ordering IS observable, names
      `the_append_completes_before_delivery_is_invoked_on_all_three_handlers` as the
      test that observes it, and records that the old claim was load-bearing while
      it stood. The retracted sentence is gone.

**For:** `dev-writer`.

**Severity: LOW** (documentation correctness, not behaviour). Reported because
`tasks.md` §10 makes the point itself that a note saying a property is
unobservable "stops anyone looking for a way to observe it", and this is the last
copy of that note still standing in the code.

`wire.rs:2406-2413`, inside
`delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal`:

> "so no test through this API can observe the ordering directly. … The
> structural argument is the evidence; this is the observable half."

`the_append_completes_before_delivery_is_invoked_on_all_three_handlers` sits 120
lines below it and observes exactly that, which `tasks.md` §10 now records as the
correction. The commit message corrects `tasks.md`; this comment was not
corrected with it. A reader who reaches this comment first draws the conclusion
the commit was written to retract.

**Outcome: fixed** in `4324364`. Found independently by the readability reviewer
(entry 1), which is why it went in the first commit. The comment now states the
ordering *is* observable, names the test that observes it, and records that the old
claim was load-bearing while it stood — so the retraction is legible rather than
merely absent.

Your framing is the one I kept: `tasks.md` §10 itself says a note claiming
unobservability "stops anyone looking for a way to observe it", and this was the last
copy of that note still standing in the code. Two reviewers reaching it separately is
a reasonable proxy for the test a comment cannot have.

---

## Leads: verified, with measurements

### Lead 1 — the sign→append→hand-off ordering is genuinely pinned, not merely counted. **GOOD.**

`wire.rs:2482` `JournallingLog` holds `Rc<RefCell<Vec<&'static str>>>` cloned
into the sink; `append` pushes `"append"` *after* the inner append returns
(`wire.rs:2496`, so the entry means "stored", not "attempted"), the sink pushes
`"deliver"`. `wire.rs:2585` asserts a **hardcoded** `["append", "deliver"]`,
looped over all three handlers with the journal cleared after the seed publish.
The expected sequence is not read back from anything the handlers produced.
The mechanism is sound: two clones of one `Rc` borrow nothing from each other.

### Lead 2 — the "count, not ordering" worry is real about the count test and is **fully covered** by the new ordering test. **GOOD, lead superseded.**

The tester's concern was that `the_three_handlers_share_one_signature…`
(`wire.rs:3150`) asserts `delivered.len() == 3`, so a hoist that *replaced* the
later sink call rather than adding to it would keep the count at 3 and survive.
That is correct about that test. I built exactly that mutation in `publish_vote`
— precompute the op id from `Op{stoa, author, kind: Vote{target, direction}}`,
call `deliver(&precomputed)` **before** `authoring::vote`, and delete the
success-arm `deliver` — so the count stays at 3. Result:

```
2 failed:
  the_append_completes_before_delivery_is_invoked_on_all_three_handlers
    left:  ["deliver", "append"]
    right: ["append", "deliver"]
  a_refused_publish_reaches_neither_the_append_nor_delivery
    "an absent target in another Stoa: … and it reached [\"deliver\"]"
```

So the replacing form is killed twice over. The count test's weakness is
accurately described in `tasks.md` §7 and no longer load-bearing. Mutation
reverted; suite back to 531.

### Lead 3 — `thread_of` returning `id` unconditionally fails exactly **three** tests. **GOOD, count confirmed.**

Mutation applied at `authoring.rs:241`
(`OpKind::Post { thread, .. } => thread.unwrap_or(id)` → `=> id`):

```
528 passed; 3 failed
  authoring::tests::a_reply_to_a_reply_is_derived_into_the_thread_its_parent_belongs_to
  authoring::tests::two_replies_to_two_siblings_in_one_thread_are_two_ops
  wire::tests::a_published_reply_is_derived_into_its_parents_thread_through_the_wire
```

Exactly the three `tasks.md` §7 names, in the order it names them. Reverted.

### `cargo mutants` on `authoring.rs` — **no surviving mutants.**

```
cargo mutants --package dialectica-core --file dialectica-core/src/authoring.rs
Found 14 mutants to test
14 mutants tested in 6m: 8 caught, 6 unviable
```

(The 6 unviable are `Default::default()` substitutions for types with no
`Default` — `Published`, `Refusal`, `OpId`.) Note the standing limitation, and it
is exactly what hid C1: mutants mutates functions, not `const` values, so
`MAX_FIELD_LEN` was never in scope.

---

## Areas examined and found clean

Stated rather than padded — each of these I tried to break and could not.

- **Thread derivation.** `thread_of` is total, has no panic path (the non-`Post`
  arm returns an answer rather than unreachable-panicking), and is right in both
  directions: a reply to a root lands in the root's thread, a reply to a reply in
  the thread its parent names. The three-level fixture separates derive from
  copy, which a two-level one cannot. Consistent with `feed.rs`, which keys a
  thread head on `parent == None` and sets `thread: id.to_hex()` rather than
  reading the op's own `thread` field.
- **`wasNew` semantics.** `Published::appended` is carried through from
  `OpLog::append` unchanged rather than recomputed, so the two cannot disagree.
  `MemoryOpLog::append` (`log/mod.rs:470`) uses `entry().or_insert()` so
  first-wins is structural; `SqliteOpLog` uses `INSERT OR IGNORE`. Both report
  `Stored`/`AlreadyPresent` consistently. Duplicate publish returns the same id,
  log holds one op, and `a_repeated_publish_does_not_disturb_the_stored_op`
  compares `to_bytes()` rather than field equality.
- **Duplicate-op behaviour.** Same body twice, same body two Stoas (with the key
  held fixed, which is the leg that isolates the variable), same body two
  identities, same reply body two parents (with the thread held fixed, which is
  the leg `two_replies_to_two_siblings_in_one_thread_are_two_ops` adds) — all
  four covered with one explanation each. NFC/NFD is covered at both layers.
- **Refusal paths and their distinguishability.** `NotHeld` vs
  `TargetIsNotAPost` vs `WrongStoa` vs `Storage` are a typed enum, asserted on
  the variant rather than on message substrings. The `BrokenLog` fixture proves
  a store failure is never reported as an absent parent — which is the mistake
  that would send someone waiting forever.
- **Hex parsing at the boundary.** `Address::from_hex` and `OpId::from_hex` are
  strict on both length and hex validity (`identity.rs:119`); no odd-length,
  `0x`-prefixed, whitespace-padded or non-hex input is accepted. The sweep's
  valid-Stoa block genuinely reaches the op-id parser, and `tasks.md` §10's
  per-parser verification table (made each handler's *last* parser panic) is the
  right way to have established that — reading the input list would not have.
- **Refusal ordering within a handler.** Parse-everything-then-act means "a
  refused publish appends nothing" is structural rather than an artefact of
  statement order. No refusal path reaches `deliver`, which sits on the success
  arm of the `Result`.
- **Panic surface.** No indexing, slicing, `unwrap` or `expect` on any
  peer-reachable path in `authoring.rs` or the three handlers. `guarded` wraps
  each. The over-cap case in C1 is emphatically *not* a panic — that is what
  makes it slip past every sweep.
- **Arithmetic.** No arithmetic in the new code that can overflow. The one cast
  that could (`put_bytes`'s `as u32`) is pre-existing and needs a 4 GiB body to
  truncate, which JSON parsing would not survive first; C1's over-cap case is
  reachable at 150 KB and is the real bug there.

**Tree state:** clean. Every mutation listed above was reverted and the suite
re-run at 531 passed. `git status --short` returns nothing.
