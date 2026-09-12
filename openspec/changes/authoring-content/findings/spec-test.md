# `authoring-content`: spec-vs-test review

Reviewed from `openspec/changes/authoring-content/specs/content-authoring/spec.md`
and `proposal.md` against the `#[cfg(test)]` modules of
`dialectica/rust-lib/dialectica-core/src/authoring.rs` and
`.../src/wire.rs` **only**. No implementation was read except the seven
narrow slices named under "What implementation I read and why" at the end —
each one a line I mutated or a signature I needed to place a mutation.

Gate: `cargo test … -p dialectica -p dialectica-core`, baseline **531 passed**.
Restored to 531 at the end; `git status --porcelain` on the worktree is empty.

---

## Entries

### 1. `A declined handoff leaves the op published` — the reply *does* report a failure

**For:** `spec-writer` (to decide which half moves) and `dev-writer` (if the
spec wins).
**Requirement:** *Publishing signs, appends, and hands off — in that order.*
**Scenario:** *A declined handoff leaves the op published.*

The requirement says, in two places:

> Where delivery declines the handoff, the op SHALL remain in the log and the
> reply SHALL report the op as published. A publish SHALL NOT be reported as
> having failed on the strength of a delivery outcome …

and the scenario: *THEN the reply reports the op as published **and names its
op id***.

The only test for it is
`wire.rs:2676 a_publish_whose_delivery_panics_still_reports_the_op_as_published`.
Its name claims the requirement; its body asserts only that the output parses
as JSON (`as_json(&out);` — the value is discarded) and that the op is in the
log. It never looks at what the reply says. Its own comment concedes the
point — *"the guard turns it into the error shape — but the requirement is
about the LOG"* — which is a restatement of the requirement, not the
requirement.

**Measured.** I added two assertions to that test — that the reply carries no
`error`, and that its `opId` equals the independently-computed id the test
already builds — and ran the gate. Result: **1 failed, 530 passed**, with the
reply printed as

```
{"error":"panic in publish_post: delivery refused the handoff"}
```

So a declined handoff is reported to the caller as a **failure carrying no op
id**, which is what the requirement forbids in as many words. Reverted; gate
back to 531.

**Failure scenario.** A view submits a post. Delivery declines (a sink that
panics is the shape the test itself chose as "the most violent decline
available"). The view receives `{"error":…}`, shows "posting failed", and the
user retypes and resubmits — while the first op is already in the log and will
be handed to delivery again on the next attempt. The user is told the opposite
of the truth, which is the exact failure the requirement's second paragraph
was written to prevent.

Note this is a *test* defect and *possibly* a code defect; which one depends on
a decision only the spec-writer can make. Either the requirement is too strong
(a panicking sink is not a "decline" and the spec should say what a decline is,
since nothing in this API distinguishes the two) or the handler must catch the
sink's failure and still answer with the op id. The test as written cannot tell
the reader which, because it asserts neither.

**Outcome:**

---

### 2. `A publish returns while delivery is still outstanding` — untestable as written

**For:** `spec-writer`.
**Requirement:** *Publishing signs, appends, and hands off — in that order.*
**Scenario:** *A publish returns while delivery is still outstanding.*

> **WHEN** a publish is called against a delivery that accepts the handoff and
> reports no outcome
> **THEN** … the result is the same as when delivery reports an outcome promptly

The delivery sink's type in this layer is `&mut dyn FnMut(&OpId)` — it returns
`()`. There is no outcome channel, so "reports no outcome" and "reports an
outcome promptly" are not two states the API can be in.

`wire.rs:2716 a_delivery_that_reports_nothing_and_one_that_reports_promptly_give_one_reply`
is the test, and it compares `|_| {}` against `|_| reported = true`. **Second
explanation that also passes it:** every possible implementation, because the
reply cannot depend on a value the sink never produces. The test's own
comment is honest about this (*"the sink returns nothing and there is no outcome
to wait for"*), which means the scenario is a spec defect — something no test
can check — and not a coverage gap.

If the scenario is meant to pin "the handler does not block on delivery", that
needs a sink that *can* be slow or pending, which this signature cannot express.
The requirement's real, testable content — "the append completes before delivery
is invoked, exactly once" — is already pinned properly by
`the_append_completes_before_delivery_is_invoked_on_all_three_handlers`
(see Properly pinned, below). Recommend either dropping this scenario or
restating it in terms the API can exhibit.

**Outcome:**

---

### 3. A body over the format's field cap publishes — unspecified, and untested either way

**For:** `spec-writer` (a gap to decide), then `tester`.
**Requirement:** *A post names a Stoa and carries a body* / *A publish answers
in the wire contract's shapes and never aborts.*

The spec enumerates exactly one boundary on the body — **empty is accepted**,
argued at length because `op-format` contracts an empty variable-length field
as a value. It says nothing about the other end. The hostile-input scenario
mentions *"maximal field lengths"* but only asks that they not panic.

The tests pin the same asymmetry. `authoring.rs:1342
a_maximal_body_publishes_rather_than_panicking` publishes a body of exactly
`150 * 1024` and asserts it succeeds. `wire.rs:2942` feeds the hostile sweep
`150 * 1024` and `150 * 1024 + 1`, and asserts only "an object, and not the
guard's panic marker". **Nothing asserts the outcome for an over-cap body.**

**Measured.** A temporary probe test published bodies of `150 KiB`,
`150 KiB + 1` and `10 MiB` through `publish_post`:

```
PROBE len=153600    error=None  opId_present=true
PROBE len=153601    error=None  opId_present=true
PROBE len=10485760  error=None  opId_present=true
```

All three succeed. `op.rs:132` sets `MAX_FIELD_LEN = 150 * 1024` and
`op.rs:1703` pins that a field of `MAX_FIELD_LEN + 1` **is refused on decode**
(`OpError::FieldTooLong`). So the publish path stores a signed op whose
canonical bytes this peer's own decoder will refuse. That op cannot be sent, and
cannot be read back across a restart by anything that decodes rather than
holding the in-memory struct.

Failure scenario: a paste of a 200 KB log into a compose box returns
`{"opId":…,"wasNew":true}`. The user sees a published post. No peer ever
receives it, and after a restart the local log entry does not decode. There is
no error anywhere.

This is a **spec gap first**: whether an over-cap body is refused at publish
(with `op-format`'s cap named in the message) or silently accepted is a decision
nobody made. It is worth a requirement either way, because the empty end got
one. Once decided, it needs a test — and note that the tests hardcode
`150 * 1024` rather than referring to `MAX_FIELD_LEN` (which is private to
`op.rs`), so they will not move with the cap.

**Outcome:**

---

### 4. `A published op is verified on read like any other` — the tamper half is a property of Ed25519

**For:** `tester`.
**Requirement:** *A publish checks only what it can decide, and never in place
of a reader.*
**Scenario:** *A published op is verified on read like any other* — "**AND** the
result does not depend on this peer having been the publisher".

`authoring.rs:1256 a_published_op_is_verified_on_read_like_any_other` publishes
an op, asserts it verifies, then rebuilds it with a different body and the old
signature and asserts `!verify()`.

The second assertion is the defect family's second entry, restated: mutating the
signed bytes and asserting the signature stops matching is a property of
**Ed25519**, not of this capability. **Second explanation that also passes it:**
any `verify()` that checks a signature over any encoding at all — including one
that ignored the publish path entirely. Nothing in the test contrasts an op this
peer published with one it received, which is the scenario's actual second
clause.

That clause *is* reached, jointly, by `authoring.rs:1188
an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives`, which
hand-builds two ops the publish path refuses, appends them, and asserts each
`verify()`s — "the log does not mark it as ours". Coverage is many-to-many, so
the requirement is not uncovered. What is wrong is that the *named* test's
sharp-looking assertion is not sharp, and a reader will take it as the evidence.
Low severity: recommend the tamper assertion either be dropped (it duplicates
`op.rs`'s own signature tests) or replaced by a direct comparison — publish one
op, receive an identical one built by hand, and assert the two `verify()` calls
agree.

**Outcome:**

---

### 5. `A voted target reads exactly as it did before` — the feed half cannot fail

**For:** `tester` (low severity; the requirement is covered by its other half).
**Requirement:** *A published vote is stored and readable, and no ordering
consumes it.*

`authoring.rs:1118 a_voted_post_reads_back_byte_identical_and_its_feed_row_is_unchanged`
and `wire.rs:2369 a_vote_leaves_the_feed_row_of_its_target_identical` both
assert `before_row == after_row` across two votes.

The byte-identical half is genuine and discriminating. The feed half is not:
`feed::list_threads` resolves `Post` ops, so a `Vote` op cannot appear in a feed
row under **any** implementation of this capability. **Second explanation:** the
feed never read votes in the first place, which is the thing being "proved".
Both tests do guard against the empty-feed trap (`items.len() == 1`), which is
the right instinct — it just does not make the comparison discriminating.

This is worth recording rather than fixing now, because it becomes a *real*
test the moment `relevance-ordering` lands and a scorer starts reading votes —
which the proposal's own closing note flags. Keeping it, with a comment saying
it is currently vacuous and why it is kept, is a defensible answer.

**Outcome:**

---

### 6. `The signing identity is the one the probe reports` — the probe's lookup is injected, not exercised

**For:** `tester` (low severity).
**Requirement:** *The author is derived from the Stoa, never supplied.*

`authoring.rs:502 the_signing_identity_is_the_one_the_capability_probe_reports`
calls `wire::capability_for(&stoa, |s| Ok(derive_stoa_key(&A_ROOT, s)…))` — the
closure *is* the test, hardcoding the derivation — and compares the result
against a publish signed with `a_key(A_ROOT, &stoa)`. Both sides call
`derive_stoa_key`, so the test cannot see the two sides diverging; the comment
claiming *"both sides are computed here from the same root through the two real
functions"* overstates it, since the probe's real lookup is not called.

The tie is in fact structural and holds: `keystore.rs:641`'s `stoa_address`
delegates to `crate::identity::derive_stoa_key`, so there is one derivation, not
two agreeing copies. So the requirement is satisfied — but by a fact the test
does not check, and the comment should say that rather than imply otherwise.
Recording it so nobody later "strengthens" the keystore's lookup into a second
implementation and finds this test green.

**Outcome:**

---

### 7. `Refusal::NoIdentity`'s "no key was created" wording is pinned without a requirement

**For:** `spec-writer`.
**Requirement:** *A publish requires a usable identity and says so when there is
none*, scenario *A refused publish creates no key material*.

The scenario is a **structural** claim: "no keystore and no key exists that did
not exist before". Two tests instead pin a **message**:
`authoring.rs:1179` and `wire.rs:2840` both assert the refusal text contains the
literal `"no key was created"`.

Asserting that a refusal *says* no key was created is not the same as asserting
none was, and nothing in the spec asks for that sentence. It is a good sentence
and a reasonable choice — which is exactly the case the project's `NO SPEC:`
rule exists for, and neither site carries the marker. Either the spec should
require the refusal to state it (then it is covered), or the tests should carry
`// NO SPEC: the refusal states that no key material was created`.

Flagging this as the **third** item in the family the task said was already
routed — the other two being that the refusal is unreachable from any `cargo
test` (no keystore in core) and that "creates no key material" is satisfied
structurally rather than covered. This one is different: it is not a coverage
gap, it is an *unmarked* choice, and it is testable and tested.

**Outcome:**

---

## Leads I was asked to verify — all three confirmed

### Lead 1: `SHALL NOT panic` — the fix works

**Mutation M1.** `wire.rs:727`, inside `guarded("publish_post", …)`:
inserted `if true { panic!("MUTATION"); }` as the first statement, so
`publish_post` panics for every input.

Result: **20 failed, 511 passed** — and
`wire::tests::hostile_publish_input_is_never_a_panic` is among the failures.
The assertion that does it is `wire.rs:3026`:

```rust
!v["error"].as_str().unwrap_or_default().starts_with("panic in ")
```

`guarded` (`wire.rs:63`) formats a caught unwind as
`{"error":"panic in <method>: <detail>"}`, so the marker is present exactly when
the handler panicked and absent when it refused cleanly. **The assertion does
distinguish "refused cleanly" from "panicked and was caught."** The old
`is_object()`-only sweep would have stayed green under M1; this one does not.

One residual: the marker is a *string prefix* the guard happens to write, so a
handler that refused with a message of its own beginning `"panic in "` would be
misread as a panic (a false failure, which is the safe direction), and a change
to the guard's wording silently disarms the check. A `#[should_panic]`-shaped
companion test on `guarded` would bolt it down. Not a finding — the guard's
wording is itself pinned by `guard_names_the_method_that_panicked` (`wire.rs:967`),
which is the right arrangement.

### Lead 2a: "the same body in two Stoas" — the replacement discriminates

**Mutation M2.** `authoring.rs:192`, in `post`, before the `publish(…)` call:
shadowed the `stoa` parameter so the op always names the author's own address as
its Stoa — i.e. `post` ignores its `stoa` argument entirely.

Result: **24 failed, 507 passed**. Relevant rows:

| test | under M2 |
|---|---|
| `the_same_body_in_two_stoas_is_two_ops_with_the_identity_held_fixed` (new) | **FAILED** |
| `content_differing_in_any_way_publishes_a_second_op` (old, Stoa leg) | **passed** |

So the tester's diagnosis holds exactly. The old test's Stoa leg signs each op
with `a_key(A_ROOT, &that_stoa)`, so Stoa and key covary and the ids differ
whichever field `post` actually wrote — a `post` that ignores `stoa` passes it.
The new test at `authoring.rs:673` holds `one_key` fixed across both calls,
leaving the Stoa as the only variable, and it fails under M2. It also asserts
`stored(…).op.stoa == agora` / `== lyceum`, which is the property the differing
ids are evidence *for*. Discriminating.

### Lead 2b: "two replies to different parents" — the replacement discriminates

**Mutation M3.** `authoring.rs:303`, in `reply`'s `OpKind::Post`:
`parent: Some(parent)` → `parent: Some(thread)` — the parent dropped, the
derived thread duplicated into its slot.

Result: **3 failed, 528 passed**:

| test | under M3 |
|---|---|
| `two_replies_to_two_siblings_in_one_thread_are_two_ops` (new) | **FAILED** |
| `a_reply_to_a_reply_is_derived_into_the_thread_its_parent_belongs_to` | **FAILED** |
| `a_published_reply_is_derived_into_its_parents_thread_through_the_wire` | **FAILED** |
| `two_replies_with_one_body_to_different_parents_are_two_ops` (old) | **passed** |

Confirmed exactly as claimed. The old test at `authoring.rs:748` uses two
**roots** as parents; for a reply to a root, `thread == parent`, so the two
explanations ("the ids differ because the parents differ" and "…because the
threads differ") give the same answer. The new test at `authoring.rs:708` makes
both parents replies inside **one** thread, so the two ops agree in Stoa,
author, body *and* thread and differ only in `parent`. It asserts both threads
equal `root.id` before asserting the ids differ, which is what removes the
thread as an explanation.

### Lead 2c: the author expectation is hardcoded

`authoring.rs:533 a_published_ops_author_is_pinned_to_a_known_answer_from_another_file`
derives its expectation from a hardcoded 32-byte hex seed
(`b62b6b59…8026cf`) rather than by calling `derive_stoa_key`, and additionally
pins the fixture's Stoa address to a hardcoded hex string — which is the right
second half, because an author derived correctly from the *wrong* Stoa would
otherwise satisfy the first assertion by coincidence. Nothing in the publish
path contributes to either expectation. It also carries a do-not-update-this
instruction, which is the correct treatment for a consensus-critical constant.
Confirmed repaired.

It failed under M2 as well (M2 changes which Stoa the op names), which is a
bonus rather than the property it is named for.

---

## Requirements checked and found properly pinned

Fourteen requirements. Nine are pinned without reservation; the five carrying
an entry above are named with the entry number.

1. **Publishing signs, appends, and hands off — in that order** — *partly*.
   The ordering itself is pinned **well**:
   `wire.rs:2526 the_append_completes_before_delivery_is_invoked_on_all_three_handlers`
   shares one `Rc<RefCell<Vec<&str>>>` between a wrapping `OpLog` and the sink,
   records `"append"` *after* the inner append returns, and compares against a
   **hardcoded** `["append", "deliver"]` for each of the three handlers
   separately. A handler that delivered first yields `["deliver", "append"]` and
   fails on the comparison; one that delivered twice fails on length. This is the
   right shape, and the header comment correctly overrules `tasks.md`'s claim
   that the ordering is unobservable. `wire.rs:2594
   a_refused_publish_reaches_neither_the_append_nor_delivery` is its mirror,
   asserted as an **empty** journal across four refusals at three different
   depths — which distinguishes "nothing happened" from "an append was rolled
   back", as counting the log afterwards could not. Scenario *The op is in the
   log when the reply is returned* is pinned by `authoring.rs:426` reading the op
   back **through the log** by the reply's id. See **entry 1** (declined handoff)
   and **entry 2** (outstanding delivery) for the two scenarios that are not.
2. **The reply names the op that was published** — pinned. `wire.rs:1922`
   parses the reply's `opId`, re-reads the log by it, and asserts
   `entry.id() == id` plus the body and `parent: None`; `wire.rs:3098` asserts
   `opId` is a string for all three handlers through one function-pointer type.
3. **The author is derived from the Stoa, never supplied** — pinned, and by two
   complementary tests rather than one: `authoring.rs:477` derives the expected
   address independently *and* asserts a different Stoa's address is **not** it
   (without which the first assertion would hold for any key), and
   `authoring.rs:533` pins the same answer to a hardcoded seed. The
   refuse-a-named-author half is `wire.rs:2078
   a_forbidden_field_is_refused_on_every_operation`, which sweeps
   `author`/`identity`/`key`/`address`/`thread` across **all three** handlers —
   the "is the guard called everywhere?" shape, and the one arrangement that
   catches a guard wired into one handler and forgotten in the other two. Third
   scenario: see **entry 6**.
4. **A publish requires a usable identity and says so when there is none** —
   known and routed (structurally unreachable from `cargo test`; the refusal
   lives behind `cfg(logos_scaffold)`). `wire.rs:2802
   the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text`
   does the one thing a `cargo test` can: it asserts the wire reply is **the
   `Refusal::NoIdentity` variant's own `Display` output**, not a second string
   that happens to agree, so drift between the two copies becomes a failure. It
   also asserts the supplied reason survives, which a `no_identity` discarding
   its argument would pass the equality check without. Good work on an
   awkward boundary. See **entry 7** for the unmarked wording choice.
5. **A post names a Stoa and carries a body** — pinned. Empty body accepted at
   both layers (`authoring.rs:464`, `wire.rs:2241`); each of the six required
   fields absent, with the message required to name **which** field
   (`wire.rs:2126`); and `wire.rs:2187` asserts missing and wrong-typed are two
   **different** messages, each naming its own mistake, with
   `!wrong_msg.contains("missing")` closing the obvious cheat. See **entry 3**
   for the unspecified upper bound.
6. **A reply names its parent, and the thread is derived** — pinned, and this is
   the strongest cluster in the change. `authoring.rs:795` and `wire.rs:1988`
   both go **three levels deep**, which is what separates deriving from copying —
   at two levels "the parent's id" and "the parent's thread" are the same value
   and the two implementations agree. `authoring.rs:822` varies body *and*
   identity while holding the parent fixed and asserts one thread.
   `wire.rs:2040` refuses a supplied `thread` and requires the message to name
   the field. Scenario *Two replies with the same body to different parents*:
   see lead 2b — now covered by the new sibling-parents test.
7. **A reply to a parent the peer does not hold is refused** — pinned.
   `authoring.rs:841` asserts the exact `Refusal::NotHeld { what: "parent", id }`
   and an empty log; `authoring.rs:874` asserts `TargetIsNotAPost` **and**
   `assert_ne!` against `NotHeld` for the same id, which is the distinguishability
   the requirement asks for, stated as a difference rather than as two errors.
   `wire.rs:2749` does the same at the wire as two differing message strings with
   `!kind_msg.contains("does not hold")`. `authoring.rs:906` builds the parent in
   a *separate* log so its id is known before the reply is attempted, then
   appends it and republishes — the recovery scenario, pinned without the fixture
   ever holding the parent early.
8. **A reply and its parent belong to one Stoa** — pinned, both directions in one
   test (`authoring.rs:932`), which is the right shape: an implementation that
   always refused and one that never did each pass half of it.
9. **A vote names a target and a direction, and both directions publish** —
   pinned. `authoring.rs:969` reads the stored direction back and matches it
   against a hardcoded `VoteDirection`, and asserts the two ids differ.
   `wire.rs:2261` adds nine near-miss directions — `"UP"`, `"Up"`, `"upvote"`,
   `"raise"`, `"+1"`, `""`, `"u p"`, `"1"`, `"down "` — requires each refusal to
   **name the string supplied**, and asserts the log length is unchanged, which
   is the "not mapped onto a recognised one" half.
10. **A published vote is stored and readable, and no ordering consumes it** —
    pinned. `authoring.rs:1096` asserts the target-restricted read returns
    **exactly** `vec![published.id]`. `wire.rs:2325` checks the reply's key set
    as an **exhaustive** `["opId","wasNew"]` rather than spot-checking names, so
    a new field cannot slip in, and then also spot-checks eight forbidden names.
    See **entry 5** for the feed half.
11. **Publishing a vote is permitted on any op the peer holds** — pinned, and
    the not-refused-on-kind clause is checked against a hand-appended
    `Moderate` op (`authoring.rs:1016`), not merely against another post. Absent
    target and cross-Stoa target both assert the exact `Refusal` variant and an
    unchanged log.
12. **A vote and its target belong to one Stoa** — pinned (`authoring.rs:1064`).
13. **Publishing the same content twice publishes one op** — pinned.
    `authoring.rs:588` asserts one id, `Appended::Stored` then
    `Appended::AlreadyPresent`, and `log.len() == 1`; `wire.rs:1954` asserts
    `wasNew` `true` then `false` with no `error`, which is the "passes it on
    rather than discarding it" clause. `authoring.rs:612` compares
    `to_bytes()` before and after — byte-identical, not merely field-equal,
    which is the assertion that catches a second publish re-signing the entry.
    The *differing*-content side has one weak leg and one strong replacement:
    see lead 2a.
14. **A publish checks only what it can decide** — pinned by
    `authoring.rs:1188`, which hand-builds both shapes this module refuses,
    **asserts each really is refused by `reply`** before appending them (without
    which the test proves nothing), then asserts the log stored and verifies
    both. See **entry 4** for the second scenario's weak half.
15. **A publish answers in the wire contract's shapes and never aborts** —
    pinned, and this is where the change is strongest.
    `wire.rs:2846 every_publish_refusal_is_the_error_shape_and_carries_no_op_id`
    sweeps twelve refusals across three handlers and asserts **no `opId` and no
    `wasNew`** alongside the error — never a partial success.
    `hostile_publish_input_is_never_a_panic` is now a real test of the
    requirement (lead 1), and its op-id-parser cases are notable: the comment
    records that every earlier case malformed the Stoa too, which is parsed
    first, so the parser was reached only with well-formed hex — *"an `expect` on
    `OpId::from_hex` left the whole suite green, this test included, until these
    cases existed"*. That is the defect family caught by the person who wrote it.
    `a_body_is_published_exactly_as_supplied` / `…through_the_wire` cover
    bidi controls, zero-width, NUL, padding and case at both layers, and the
    NFC-vs-NFD pair (`authoring.rs:1317`, `wire.rs:3073`) is the sharp case for
    "no normalisation" — two strings that render identically and differ in bytes,
    so a normalising path collapses them and fails.

Also clean, and worth saying because it is where this family of defect usually
hides: I found **no** test in either module that derives an expected value by
calling the function under test, **no** hash-moved-after-a-byte-changed
assertion, and **no** `assert_eq!(x[0], CONST)` position-without-value pin. The
one place a constant is pinned (`authoring.rs:533`) pins the **value**, from a
hardcoded seed, in two independent places, with an instruction not to update it.

## `NO SPEC:` markers — both are correct

- `wire.rs:2079`, on `a_forbidden_field_is_refused_on_every_operation`: the spec
  requires `author`/`identity`/`key` refused on any publish and `thread` on a
  **reply**; refusing every name on all three operations is the dev's choice.
  Correctly marked, and mirrored on the implementation constant at `wire.rs:584`,
  which names the test. **For `spec-writer`:** worth capturing — refusing
  `thread` on a post and a vote is a better rule than the spec's, and it should
  be the spec's. (`address` needs no marker: the requirement names it.)
- `wire.rs:1215`, on the probe test, predates this change and belongs to
  `posting-capability`.

Unmarked choices found: **entry 7** (the "no key was created" wording) and
**entry 3** (the over-cap body, where the choice is not even pinned by a test).

## Out of my scope, stated so nobody assumes I checked it

The spec's three named boundaries (`op-format`, `op-log`, `posting-capability`)
are referenced correctly and not restated as requirements here, which is the
right arrangement. I did not verify the *other* capabilities' specs still say
what this one attributes to them — that is a cross-spec check nobody was
assigned, and it is how a `REMOVED`-here/`ADDED`-there pair goes missing. This
change is `ADDED`-only with no `MODIFIED` capabilities, so there is no moved
requirement to lose. The proposal's argument for declining `createdAt` is a
design question and belongs to the `design-reviewer`.

## What implementation I read and why

Staying blind to the implementation was the point of the role, so here is the
complete list of non-test lines I read, each forced by a mutation:

1. `wire.rs:63` — the `"panic in {method}: {detail}"` format, to judge whether
   the hostile sweep's marker assertion can tell a caught panic from a refusal
   (lead 1). The test's own comment already quoted it.
2. `wire.rs:721-730` — `publish_post`'s signature and first statement, to place
   mutation M1.
3. `wire.rs:584-588` — the `NO SPEC:` doc comment on `FORBIDDEN_FIELDS`, found by
   the `grep NO SPEC` the task required, plus the five field names.
4. `wire.rs:567`, `721`, `761`, `804` — four `pub fn` signature lines from grep,
   to locate handlers.
5. `authoring.rs:187-196` — `post`'s signature and the head of its `publish(…)`
   call, to place mutation M2.
6. `authoring.rs:267-309` — `reply`'s body, to place mutation M3
   (`parent: Some(thread)`). This is the largest slice and the one I would most
   like not to have read; the mutation the tester claimed could not be placed
   without seeing where `parent` is written. It is 40 lines and I did not read
   past `reply`'s closing brace except for the doc comment on `vote` that
   immediately follows.
7. `op.rs:132` and `op.rs:1703-1706` — `MAX_FIELD_LEN`'s value and the existing
   decode test proving `MAX_FIELD_LEN + 1` is refused, to establish that entry
   3's 10 MB op cannot be decoded. `op.rs` is another capability's file and was
   not off-limits.
8. `keystore.rs:641` (one line, via grep) — that `stoa_address` delegates to
   `derive_stoa_key`, for entry 6.

I did not read `authoring.rs:1-186` or `:310-360`, `wire.rs:1-506` or
`:508-583`, `:589-720`, `:731-934`, `dialectica/rust-lib/src/lib.rs`,
`design.md` or `tasks.md`.

Mutations M1, M2, M3 and two temporary probe tests were all reverted; the gate
is back to **531 passed** and `git status --porcelain` on the worktree is empty.
