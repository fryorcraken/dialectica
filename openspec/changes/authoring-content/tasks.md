# Tasks

## Stages

Added after the fact: this change predates the stage block, so the rows below are
ticked against what the branch's history shows actually ran rather than against a
list that guided it. The six review rows each have a findings file and a
cherry-picked commit on `piece/authoring`.

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

## 1. Settle the approach before writing it

- [x] Read the spec's fourteen requirements, `op-format`, `op-log`,
      `posting-capability`, `module-wire-contract`, and PLAN.md from
      `origin/main`.
- [x] Establish what already exists. All of it: `op.rs` owns the op shapes, the
      canonical bytes, the signing and the op id; `log/` owns the store and its
      newly-stored-or-already-present answer; `keystore.rs` owns the per-Stoa
      key. Nothing in this change invents a primitive.
- [x] **Decide whether a reply's thread can be derived at all.** It can, and the
      answer is in the format rather than in new state: a root carries
      `thread: None` (its id is the hash of the bytes being signed), a reply
      carries `thread: Some(t)`, so the thread an op belongs to is
      `thread.unwrap_or(own_id)`. Recorded in `design.md` with what it trusts.
- [x] Confirm the boundary with the three parallel capabilities. Identity,
      keystore, Stoa creation and transport are not touched: a signing key and a
      Stoa are **inputs**, and the channel a published op goes to is
      `op-transport`'s to name.

## 2. `design.md`

- [x] The Decisions section carries the eight calls worth recording, each with
      what was rejected: parse-then-act over validate-as-you-go; one shared
      `publish` over three near-copies; delivery as a caller-supplied sink whose
      outcome is discarded; a typed `Refusal` over a `String`; the thread
      derivation and what it trusts; no genesis record on the publish path;
      `Appended` passed through rather than recomputed; forbidden fields refused
      rather than ignored.
- [x] Behaviour stays out of `design.md` and reasoning stays out of the spec.

## 3. `authoring.rs` — the decisions, over typed values

- [x] `post`, `reply`, `vote`, each returning `Published` or `Refusal`.
- [x] One private `publish` that signs, appends and reports. The ordering
      requirement is therefore asserted once, not three times.
- [x] `Published` carries `Appended` **through**, not a recomputed boolean.
- [x] `Refusal` is a typed enum, so `NotHeld` and `TargetIsNotAPost` are told
      apart by variant rather than by substring.
- [x] `thread_of` derives a reply's thread; no caller can supply one.
- [x] Takes `&mut L: OpLog` and `&SecretKey`. It opens no store, reads no
      environment and finds no key — so "a publish creates no key material as a
      side effect" holds by construction.
- [x] `Arrival::unordered()` on append: this op did not arrive, so no transport
      ordered it, and claiming a Lamport value would be this peer inventing
      ordering metadata for its own content.
- [x] `attachments: vec![]` — out of scope, and an empty list is a value that
      `op-format` contracts.

## 4. `wire.rs` — the parse, and the one failure shape

- [x] `publish_post`, `publish_reply`, `publish_vote`, each guarded.
- [x] `{"opId":"…","wasNew":bool}` — the same shape for all three, which is what
      makes the absence of a score on a vote reply structural rather than
      remembered.
- [x] `reject_forbidden_fields` is ONE guard over a union list, called from all
      three. Refused, not ignored — the opposite of `order`, and `design.md`
      says why.
- [x] `required_string` distinguishes wrong-typed from missing, by message.
- [x] `"up"`/`"down"` only; anything else is refused **naming what was
      supplied** and never mapped.
- [x] Every field is parsed before `authoring` is reached, so "a refused publish
      appends nothing and delivery was not invoked" is structural.
- [x] The delivery sink is called only on the success arm.

## 5. `src/lib.rs` — three trait methods and the assembly

- [x] Three methods on `DialecticaModule`, documented with the two things a
      caller most needs and would otherwise infer wrongly: `wasNew`'s purpose,
      and that a success is local only.
- [x] One `publishing` helper assembling keystore, key, store and sink, rather
      than three copies. It opens an existing keystore and never creates one.
- [x] The delivery sink logs and does nothing else, because `op-transport` owns
      channel identity. **A no-op is honest; inventing a channel-naming scheme
      here would be two peers opening channels nobody else is in, silently and
      permanently.**
- [x] Exactly one parameterless constructor preserved (`#[derive(Default)]`), so
      the `interface: "universal"` scan is unaffected.

## 6. `op.rs` — a doc comment corrected, because it was actively misleading

- [x] `OpKind::Post::thread` said "the store fills the thread in as its own id
      on ingest". **It does not and it cannot** — the op is signed, so a store
      rewriting a field invalidates the signature. A reader who believed it
      would expect `thread` to be non-`None` on every stored root and would
      derive a reply's thread wrongly. This is a comment-only change; the format
      is untouched.

## 7. Tests, and mutation-verifying them

- [x] Tests in `authoring.rs` over typed values, and in `wire.rs` at the
      boundary a view reads from. The count is what the suite reports; CI's
      test-count gate derives a declared count from `#[test]` attributes and
      asserts `ran == declared`, so the suite is the authority and this file
      does not restate it.
- [x] Expectations hardcoded, never read back from the implementation.
      **Amended:** "the identity assertions derive the expected address
      independently through `derive_stoa_key`" was the claim, and independent of
      *the publish path* is not independent of *the derivation*. An expectation
      computed by calling the same function the fixture's key came from agrees
      with it by construction, so changing `STOA_KEY_SALT` left
      `a_published_op_verifies_against_the_identity_derived_for_its_stoa` green.
      That test is still the right one for "the author is the Stoa's identity and
      not some other key"; what it cannot notice is the derivation itself moving.
      `a_published_ops_author_is_pinned_to_a_known_answer_from_another_file` is
      the other half, pinned to `identity.rs`'s hardcoded seed, and joins the two
      existing known-answer tests in failing when the salt changes. Verified by
      running that mutation: those three fail and the original stays green.
- [x] Unspecified behaviour marked. Two `// NO SPEC:` markers, both in §9.
- [x] Nine mutations, each caught:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | `thread_of` returns the parent's own id (copy rather than derive) | `a_reply_to_a_reply_is_derived_into_the_thread_its_parent_belongs_to` — **and nothing else** |
| 2 | `reject_forbidden_fields` dropped from `publish_vote` only | `a_forbidden_field_is_refused_on_every_operation` — and nothing else |
| 3 | `Published::was_new` returns `true` unconditionally | `the_same_content_published_twice_is_one_op_and_the_caller_is_told`, `a_second_publish_of_one_body_says_it_was_not_new_and_names_the_same_op` |
| 4 | `required_string` reports a wrong-typed field as missing | `a_wrong_typed_field_is_distinguishable_from_a_missing_one` — and nothing else |
| 5 | An unrecognised direction defaults to `Up` | `both_vote_directions_publish_and_an_unrecognised_one_is_refused_naming_it` — and nothing else |
| 6 | `reply` reports a non-post parent as `NotHeld` (the two refusals collapsed) | `a_reply_naming_a_non_post_is_refused_distinguishably_from_an_absent_parent`, `a_publish_refused_for_an_absent_parent_says_which_and_not_that_it_is_the_wrong_kind` |
| 7 | `vote` refuses a target that is not a post (a check the spec forbids) | `an_identity_may_vote_on_its_own_post_and_on_any_kind_the_peer_holds` — and nothing else |
| 8 | `publish_reply` calls the delivery sink before matching, so a refusal reaches it | `delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal` — and, since the tester's work, `a_refused_publish_reaches_neither_the_append_nor_delivery` and `the_append_completes_before_delivery_is_invoked_on_all_three_handlers` |
| 9 | `reply`'s and `vote`'s presence and cross-Stoa checks removed | `a_cross_stoa_reply_is_refused_and_a_same_stoa_one_publishes`, `an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives`, `a_cross_stoa_vote_is_refused`, `a_vote_on_an_absent_target_is_refused_and_appends_nothing`, `every_publish_refusal_is_the_error_shape_and_carries_no_op_id` |

**Mutation 8 was killed through one handler only, and that is the shape to check
rather than the instance.** It was run against `publish_reply`; the equivalent in
`publish_vote` — hoisting the sink above the direction parse — was caught by
**nothing** in the original suite except by accident, and the accident is worth
naming: `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`
counts sink calls and saw four instead of three. That is a count, not an
ordering, and it would not have noticed a hoist that replaced the later call
rather than adding to it. Re-run after the tester's work, in both remaining
handlers:

| Mutation | Tests that failed |
|---|---|
| Sink hoisted above the direction parse in `publish_vote` | `a_refused_publish_reaches_neither_the_append_nor_delivery`, `the_append_completes_before_delivery_is_invoked_on_all_three_handlers`, `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over` |
| Sink hoisted above the append in `publish_post`, with a correctly **precomputed** op id and the success-arm call removed | `the_append_completes_before_delivery_is_invoked_on_all_three_handlers`, `a_publish_whose_delivery_panics_leaves_the_op_in_the_log` |

The second is the one the original suite could not see at all: a precomputed id
satisfies "the sink was handed the op that was published", so only a test that
observes the *sequence* fails.

**Mutation 1 is the one worth dwelling on, because it is the family this project
keeps hitting.** Copy-the-parent's-id and derive-the-parent's-thread give the
**same answer** for a reply to a root — which is the fixture anyone writes first.
Only a three-level fixture separates them, and the first draft of
`a_reply_names_its_parent_and_is_derived_into_the_parents_thread` was two levels
and could not fail for the reason it named. The three-level test was added
before the mutation was run, on exactly that suspicion, and the run confirmed it:
one test failed, and it was that one.

Re-measured after the tester's work: `thread_of` returning `id` unconditionally
now fails **three** tests —
`a_reply_to_a_reply_is_derived_into_the_thread_its_parent_belongs_to`,
`two_replies_to_two_siblings_in_one_thread_are_two_ops` (the tester's
replacement for the parent/thread-covarying test) and
`a_published_reply_is_derived_into_its_parents_thread_through_the_wire`. The
derivation therefore has a third guard, and it arrived as a side effect of
fixing an unrelated defect family rather than by design — which is the argument
for the fix, not against it: the two surviving guards were one test away from
being one.

**Mutation 2's shape is the "is the guard called everywhere?" question.** One
handler forgetting the guard is invisible without checking all three, which is
why `a_forbidden_field_is_refused_on_every_operation` loops over the union of
field names crossed with all three operations rather than spot-checking `author`
on a post.

## 8. A compile error no gate here can see, found by a test

`publish_*` took `deliver: impl FnOnce(&OpId)` in the first draft, because
"called at most once" is the honest bound on a sink. **It does not survive the
test that pins all three handlers as one function-pointer type**, and the failure
is invisible to every gate that can be run in this repo.

**Say precisely whose constraint that is, because an earlier version of this
section named the wrong one** (`findings/architecture.md` A1). It claimed the
requirement came from the *adapter*. It does not: `Dialectica::publishing` is
generic over the handler (`F: FnOnce(…)`), so it monomorphises per call site and
would accept a generic sink perfectly well.

The pin belongs to
`the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`, via its
`type Handler = fn(…)`. A generic `impl FnOnce` monomorphises per call site, so
the three are three types; coercing them fails on a higher-ranked lifetime,
because a `&mut dyn FnMut(&OpId)` argument is not the
`for<'d> fn(…, &'d mut dyn …)` pointer that one pointer type needs.

That the constraint is a test's rather than the adapter's is what makes
**recovering `FnOnce` a live option**: it costs changing that test's `Handler`
type, not a redesign. The note matters because a wrong impossibility claim is
what stops the next reader from retrying.

- [x] Changed to `&mut dyn FnMut(&OpId)`. **Nothing a requirement rests on is
      lost**: the return type `()` is what makes a delivery outcome unwaitable,
      and `FnOnce` only added at-most-once, which no requirement asks for.
- [x] Added `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`,
      which is what caught it — and which is also the thing imposing the
      constraint, not merely detecting it. Pinning the three signatures as
      interchangeable *in this crate* is worth its cost because the adapter is
      behind `cfg(logos_scaffold)`, which no `cargo test` sets: a signature that
      drifted would otherwise surface only in the builder's build, the one that
      runs last and reports worst.

## 9. Unspecified behaviour, marked

Two `// NO SPEC:` markers, and both are the same decision seen twice:

- `wire.rs`, on `FORBIDDEN_FIELDS`, and again on
  `a_forbidden_field_is_refused_on_every_operation`. **The spec requires a
  `thread` field to be refused on a reply** and says nothing about one on a post
  or a vote. Refusing it everywhere is chosen, because a caller who sent one has
  the same wrong model whichever operation it reached — and the alternative
  (ignore it on two of three) is the kind of inconsistency a caller discovers by
  having it work once.

`author`, `identity` and `key` are not NO SPEC: the spec requires "a field that
names an author, an identity or a key" to be refused on any publish. `address`
is the fourth name in the same family and is refused on the same grounds; it is
covered by the marker above.

## 10. What the green gate structurally cannot see

- **That an op ever reaches a peer.** Delivery is a sink that logs. The three
  requirements about the handoff are satisfied by a sink's *shape*, and no test
  here observes a real `channelSend`. `op-transport` is where that becomes
  testable, and until then this change's "hand it to delivery" is a seam rather
  than a wire.
- **~~That the append precedes delivery.~~ This was wrong, and the tester
  proved it.** The claim was that the sink cannot read the log because the
  handler holds it mutably for the call's duration, so no test through this API
  can observe the ordering. **That is true of the log and false of the
  ordering.** An `Rc<RefCell<Vec<_>>>` held by both a wrapping `OpLog` and the
  sink is two clones of one handle: neither borrows the other, each pushes its
  own name when it runs, and the resulting sequence is evidence rather than an
  argument.
  `the_append_completes_before_delivery_is_invoked_on_all_three_handlers`
  asserts a hardcoded `["append", "deliver"]` per handler, and
  `a_refused_publish_reaches_neither_the_append_nor_delivery` asserts an EMPTY
  journal across four refusals at three depths — which distinguishes "nothing
  happened" from "an append was rolled back" in a way that counting the log
  afterwards cannot.

  **Why recording this matters more than the test does.** "No test can see
  this" was load-bearing: it was the reason a weaker test was accepted, and it
  licensed
  `delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal` to
  observe only that the sink got the right id. Hoisting `deliver` above
  `authoring::post` with a correctly **precomputed** op id leaves that test
  green and fails the new one. A note saying a property is unobservable is a
  note that stops anyone looking for a way to observe it, so it has to be right
  — and here the reachable-state argument was applied to the wrong quantity.
- **That the hostile-input sweep reaches every parser — it does now, and the
  hazard is ordering, not input variety.** `hostile_publish_input_is_never_a_panic`
  asserted only `is_object()`, which `guarded` satisfies for a *caught* panic:
  the reply is `{"error":"panic in <method>: …"}`, a perfectly well-formed
  object, so a handler panicking on every input passed. Rejecting that marker
  then exposed a second gap the sweep's size hid: **`stoa` is parsed first in
  all three handlers, and every wrong-length case malformed the Stoa too**, so
  each handler refused before `parent` or `target` was read and the op-id parser
  saw nothing but well-formed hex. An `expect` on `OpId::from_hex` left the
  whole suite green.

  **The shape to look for is a fixture that never reaches parser N because
  parser N-1 always refuses first**, and it is invisible from the sweep's input
  list — that list looks exhaustive either way. Verified per parser rather than
  argued, by making each handler's *last* parser panic and checking the sweep
  fails:

  | Parser made to panic | Sweep catches it |
  |---|---|
  | `required_direction`, `publish_vote`'s third | yes — the hostile-`direction` case with a valid Stoa and a valid target |
  | `required_string("body")`, `publish_reply`'s third | yes — same case, valid Stoa, valid parent |
  | `required_string("body")`, `publish_post`'s second | yes — same case, valid Stoa |

  **What that table measures is reachability, and not refusal-path coverage** —
  a distinction review had to draw for us (`findings/security.md` S2), because
  this section's earlier wording read as though "yes" meant the parser was
  fully exercised. It does not. Every hostile-*text* fixture supplies
  `direction` and `body` as well-formed JSON **strings**, so those parsers are
  entered only on their **success** arms; the wrong-*typed* fixtures
  (`direction` as null, object, array, number) also malform `stoa`, so all
  three handlers refuse at parser one and never reach them.

  Measured: a `panic!` placed unconditionally in `required_direction` fails the
  sweep, and a `panic!` on only its `Err` arm **passes**. So the ordering hazard
  this change fixed for `OpId::from_hex` is still present one parser along. No
  fixture pairs a valid Stoa and a valid target with a missing or wrong-typed
  `direction`.

  Worse, and for the `tester`:
  `every_publish_refusal_is_the_error_shape_and_carries_no_op_id` *does* reach
  that `Err` arm, but asserts only that an `error` key is present — which a
  caught panic satisfies too. So **no test here distinguishes a refusal from a
  panic on the direction parser.** The sweep's `starts_with("panic in ")`
  assertion is the thing that would, and it is missing from the one test that
  gets there.

  What reaches those last parsers is the hostile-*text* block, which pairs a
  valid Stoa and a valid op id with adversarial `body` and `direction`. The
  wrong-length blocks reach only the first parser in each handler, by
  construction; they are kept because "64 characters" and "64 *hex* characters"
  are different acceptances, and the valid-Stoa variants are what test the
  second.
- **An encode/decode asymmetry, because the publish path is never run against
  `SqliteOpLog`.** `grep -c Sqlite` is **0** in both `authoring.rs` and
  `wire.rs`: every publish test uses `MemoryOpLog`, which stores the live
  `SignedOp` and never round-trips it through bytes. So no test here can see a
  publish that produces bytes the decoder would refuse — the store that would
  notice is the one the tests do not use.

  This is not hypothetical; it is how the over-cap body defect survived three
  gates (see `findings/security.md` S1 and `findings/correctness.md` C1). A test
  against the production store is what closes the class, not another fixture
  against the in-memory one.
- **`cargo fmt` does not reach `dialectica-core`.** The manifest is
  `[workspace]` with no members, so the CI gate's `cargo fmt --check` over
  `rust-lib/Cargo.toml` format-checks `rust-lib/src` only — and every line of
  this change's logic is in `dialectica-core`. Formatted by running `cargo fmt`
  against the core crate's own manifest by hand, which also revealed drift in
  ten pre-existing files. **That drift is deliberately NOT in this change**: a
  diff that reformats ten files and adds a publish path cannot be reviewed for
  either. The gap is worth a change of its own.
- **That the adapter compiles.** `cfg(logos_scaffold)` is set by `build.rs` only
  when the builder's `generated/provider_gen.rs` is present, so `src/lib.rs`'s
  `publishing` helper and its three forwards are type-checked by no gate in this
  repo. §8's test covers the one thing about it that could plausibly fail; the
  rest is unchecked until a module build runs.

  Still unchecked, and named rather than left to be discovered: the Stoa
  double-read, the keystore open, the store open, the delivery `eprintln!`. **Do
  not add a test that pretends to cover them** — the constraint is `build.rs`
  observing a file the builder generates, and `build.rs` says in as many words
  that stubbing it would produce "a green gate that cannot see the thing it
  claims to check".

  One piece moved OUT of this list. The no-identity refusal's *wording* used to
  live here as a hand-written `format!`; it is now `wire::no_identity` going
  through `Refusal::NoIdentity`'s `Display`, which `cargo test` compiles and
  `the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text`
  asserts on. Raising the refusal is still the adapter's — only the adapter can
  open a keystore — but that is a call site, not a message.
- **Whether the thread derivation is right about an inbound parent.** It
  propagates the parent's own `thread` claim without auditing it, which is the
  correct division (`design.md` argues it) and means a peer that received a reply
  naming a thread its parent does not belong to will publish further replies into
  that same wrong thread. Auditing the graph is the read side's, and the read
  side does not exist.
- **That a Stoa's posting policy is enforced.** It is not, and `Policy` has one
  variant, so there is nothing to enforce yet. Named in `design.md` rather than
  left to be discovered.

## 11. Two requirements this change does not discharge, and one it now does

These are **not** ticked as tested, because ticking them would be the failure
`.claude/agents/README.md` names: a scenario that cannot be tested, satisfied on
paper.

The two are different in kind, and the difference is the point. One is a gap this
change's **design** creates and could close (the no-identity trigger). One is
satisfied **structurally**, which is stronger than a test and would be weakened by
recording it as coverage (no key material).

**The third was a contradiction and is now discharged.** "A declined handoff
leaves the op published" was a live disagreement between the code and the
scenario; the owner settled it, the code was changed to match, and it is tested.
Its section is kept below because the reasoning — in particular what ruled the two
alternative routes out — is what a later reader needs, and because the measurement
that proved the defect is the only evidence the fix was necessary.

### "A publish requires a usable identity and says so when there is none" — not discharged here, and the reason is this change's design rather than the spec

Both scenarios. **An earlier version of this section said the requirement was
"untestable as specified" because `dialectica-core` structurally cannot reach a
keystore. That reason is wrong**, and it is the same shape as the §10 claim this
change already had to retract — a statement that something is impossible, which
stops anyone looking.

The counter-example is in the same file, fifteen hundred lines up:
`get_capabilities` takes `lookup: impl Fn(&Address) -> Result<String,
KeystoreError>`, and `capability_for` is tested directly against a lookup that
*fails*. Core does not reach a keystore there either — the adapter passes a
closure that does. So a capability can be handed a fallible key-supplier and
tested on its failure path, in core, today.

The honest statement is narrower: **because `authoring` takes an already-resolved
`&SecretKey`, the keystore open happens above core, in the adapter no
`cargo test` compiles — so the refusal's trigger is unreachable from any test
here.** That is a consequence of a design choice, and the choice has a real
benefit: taking a resolved key is what makes "a publish creates no key material
as a side effect" structural rather than asserted. The cost is this gap, and
`design.md` records the benefit without it.

The alternative not taken: give the handlers the same `lookup`-closure shape
`get_capabilities` uses, which would move the keystore open into core's testable
surface and discharge both scenarios — at the price of the structural
no-key-material property. Worth deciding deliberately rather than inheriting.

What the work on Finding 1 did get is narrower and worth stating precisely: the
refusal's **wording and wire shape** are now testable and tested; its
**trigger** is not. A keystore that cannot be opened still produces the error
shape only when the builder's build has compiled the adapter.

**For the spec-writer.** Three routes, and they are not equivalent now that the
"impossible" framing is gone:

1. **Keep the requirement and close the gap in code** — the handlers take a
   fallible key-supplier, as `get_capabilities` already does, and both scenarios
   become testable in core. This is the only route that discharges the
   requirement as written, and it trades away the structural no-key-material
   property. A `dev-writer` call as much as a spec one.
2. Scope the requirement to what the wire contract can be held to — "a publish
   that cannot obtain a signing identity SHALL answer with the error shape
   naming what is missing" — and let the module build check the trigger. Testable
   here, says less.
3. Move the requirement to whichever capability owns the keystore, since "what
   happens when there is no identity" is a fact about identity availability
   rather than about publishing. `keystore` and `posting-capability` both have a
   claim; `posting-capability`'s probe already answers "can this Stoa be posted
   to", which is the same question asked earlier.

What the spec should **not** be told is that it asks for something unobservable.
It does not; this change chose a shape that cannot observe it.

### "A declined handoff leaves the op published" — settled, and the code now honours the scenario

Found by review, and by two reviewers independently (`findings/design-review.md`
F6, `findings/spec-test.md` entry 1). It was a live contradiction rather than a
coverage gap, and **the owner has now settled it.**

The defect as measured: `deliver` was called *inside* `guarded` in all three
handlers, so a sink that panics was caught and the reply became
`{"error":"panic in publish_post: …"}` with **no `opId`** — for an op that is in
the log. The requirement says a publish "SHALL NOT be reported as having failed
on the strength of a delivery outcome" and the scenario's condition is "delivery
**refuses or errors** on the handoff", which a panic is the most violent form of.

Measured before the fix: adding "no `error` key" and "`opId` equals the expected
id" to the test failed, 530 passed / 1 failed, with the reply printed as the
error shape.

The user-visible cost, which is what made it worth settling rather than
documenting: a view is told the post failed, shows "posting failed", and the user
retypes and resubmits — while the first op is already in the log and will be
handed to delivery again. They are told the opposite of the truth, which is what
the requirement's second paragraph exists to prevent.

**The decision: catch the panic at the handoff and report the publish as
successful.** The op is in the log, the requirement says so, and delivery belongs
to the transport. Of the three routes review offered, this is (3) in outcome but
not in reasoning — it is *not* "an infrastructure fault reported as one", because
the reply carries no fault at all.

**What ruled route (1) out**, and it is the measurement that matters: moving
`deliver` outside `guarded` leaves the panic unguarded, and PHASE0-FINDINGS §3
measured what that costs — the module process aborts (`failed to initiate panic,
error 5`, SIGABRT), the caller waits out a 20-second timeout, and every later
call reports `MODULE_NOT_LOADED`. A dead module is a worse answer than an
unreported delivery failure.

**What ruled route (2) out** is the delivery contract, and it inverted the
premise the whole question rested on. `delivery_module.lidl` carries
`channelMessageSent`, `channelMessageError` and `messagePropagated` — the outcome
arrives **asynchronously, after the publish call has returned**. A return value
could not carry it even if we wanted it to. So the synchronous reply was never
the place to learn about delivery: accepting an op is `channelMessageSent` and
says nothing about whether a peer received it. Scoping the requirement to
"returns rather than unwinds" would have contracted a distinction that carries no
information.

Implemented as `wire::delivered_and_published`, one function called from all
three handlers, wrapping only the sink call in its own `catch_unwind`. The outer
`guarded` stays. A caught panic goes to stderr, because a transport defect no
operator can see is the cost this decision incurs.

Pinned by `a_panicking_delivery_sink_still_reports_the_op_as_published_on_all_three_handlers`,
which fails before the fix with exactly the error shape quoted above, and covers
all three handlers because the sink is called from three places.

**The obligation this hands on, recorded in `docs/PLAN.md` §9.2:** `op-transport`
must make an op that reaches `channelMessageError`, or that never reaches
`messagePropagated` within some bound, visible somewhere. Without that, this
decision converts a loud failure into a silent one.

The sibling test is named `a_publish_whose_delivery_panics_leaves_the_op_in_the_log`,
which is what it actually pins — the log, not the reply. Both are kept: an
implementation that reported success while rolling the op back would satisfy the
reply assertion alone.

### "A refused publish creates no key material" — satisfied structurally, not covered

There is nothing to assert. Nothing in `dialectica-core` can create key
material: `authoring` takes a `&SecretKey` it did not derive and an
`&mut impl OpLog` it did not open, and the crate depends on no filesystem path.
A test asserting "no keystore appeared" would be asserting that a crate with no
way to write a file did not write one.

This is a **stronger** guarantee than a test, and recording it as coverage would
make it weaker — a test can be deleted; a crate that cannot reach a filesystem
cannot quietly start. What would weaken it is the adapter growing a `generate`
or `create` call, which is a review question about `src/lib.rs` and not
something core can be made to prove.

### Both `NO SPEC:` markers stay

`wire.rs` on `FORBIDDEN_FIELDS`, and on
`a_forbidden_field_is_refused_on_every_operation`. **One choice, not two.**
Refusing `thread` on a post and a vote is the unspecified one — the spec requires
it on a *reply* only — and is for the spec-writer to ratify or overturn; §9
records what was chosen and why.

`address` is **not** unspecified, and an earlier version of this section said it
was. The requirement text is explicit: "No publish operation SHALL accept an
author, an identity, a key, **or an address** as a parameter" (`spec.md:86`). The
scenario one screen down names only the first three, which is what the wrong
claim was read off — but the requirement is the contract, and it names four.

**A third marker, added by review** (`findings/spec-test.md` entry 7).
`a_refusal_names_the_id_or_stoa_it_is_about` in `authoring.rs` and
`the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text` in
`wire.rs` both assert the refusal text contains `"no key was created"`. The spec's
scenario "A refused publish creates no key material" is a **structural** claim —
that no keystore or key exists which did not before — and does not ask the refusal
to *say* so. Saying it is a choice, and a good one, so both sites now carry the
marker.

Worth keeping the distinction the reviewer drew: this is not a coverage gap like the
other two in §11. It is an *unmarked choice*, and it is both testable and tested —
asserting that a refusal says no key was created is simply a different claim from
asserting none was.
