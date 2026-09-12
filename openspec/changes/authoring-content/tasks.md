# Tasks

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

- [x] 50 tests: 30 in `authoring.rs` over typed values, 20 in `wire.rs` at the
      boundary a view reads from. Suite 475 → 525, and `#[test]` attributes
      475 + 50 = 525, so CI's test-count gate agrees.
- [x] Expectations hardcoded, never read back from the implementation. The
      identity assertions derive the expected address **independently** through
      `derive_stoa_key` rather than comparing the op against itself.
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
| 8 | `publish_reply` calls the delivery sink before matching, so a refusal reaches it | `delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal` — and nothing else |
| 9 | `reply`'s and `vote`'s presence and cross-Stoa checks removed | `a_cross_stoa_reply_is_refused_and_a_same_stoa_one_publishes`, `an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives`, `a_cross_stoa_vote_is_refused`, `a_vote_on_an_absent_target_is_refused_and_appends_nothing`, `every_publish_refusal_is_the_error_shape_and_carries_no_op_id` |

**Mutation 1 is the one worth dwelling on, because it is the family this project
keeps hitting.** Copy-the-parent's-id and derive-the-parent's-thread give the
**same answer** for a reply to a root — which is the fixture anyone writes first.
Only a three-level fixture separates them, and the first draft of
`a_reply_names_its_parent_and_is_derived_into_the_parents_thread` was two levels
and could not fail for the reason it named. The three-level test was added
before the mutation was run, on exactly that suspicion, and the run confirmed it:
one test failed, and it was that one.

**Mutation 2's shape is the "is the guard called everywhere?" question.** One
handler forgetting the guard is invisible without checking all three, which is
why `a_forbidden_field_is_refused_on_every_operation` loops over the union of
field names crossed with all three operations rather than spot-checking `author`
on a post.

## 8. A compile error no gate here can see, found by a test

`publish_*` took `deliver: impl FnOnce(&OpId)` in the first draft, because
"called at most once" is the honest bound on a sink. **It does not survive the
adapter**, and the failure is invisible to every gate that can be run in this
repo.

The adapter assembles the keystore, key, store and sink once and dispatches over
the three handlers through one function-pointer type. A generic `impl FnOnce`
monomorphises per call site, so the three are three types; coercing them fails
on a higher-ranked lifetime, because a `&mut dyn FnMut(&OpId)` argument is not
the `for<'d> fn(…, &'d mut dyn …)` pointer the dispatch needs.

- [x] Changed to `&mut dyn FnMut(&OpId)`. **Nothing a requirement rests on is
      lost**: the return type `()` is what makes a delivery outcome unwaitable,
      and `FnOnce` only added at-most-once, which no requirement asks for.
- [x] Added `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`,
      which is what caught it. The adapter is behind `cfg(logos_scaffold)`, which
      no `cargo test` sets, so without that test the error would have surfaced in
      the builder's build — the one that runs last and reports worst.

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
- **That the append precedes delivery.** The sink cannot read the log to check,
  because the handler holds it mutably for the call's duration — so no test
  through this API can observe the ordering directly. What pins it is that
  `publish` returns a `Published` only after `append` returned `Ok`, and the
  sink sits after that call.
  `delivery_is_handed_the_published_op_and_is_not_reached_by_a_refusal` says so
  in a comment rather than claiming a discriminating power it does not have.
- **`cargo fmt` does not reach `dialectica-core`.** The manifest is
  `[workspace]` with no members, so the CI gate's `cargo fmt --check` over
  `rust-lib/Cargo.toml` format-checks `rust-lib/src` only — and every line of
  this change's logic is in `dialectica-core`. Formatted by running `cargo fmt`
  against the core crate's own manifest by hand, which also revealed drift in
  ten pre-existing files. **That drift is deliberately NOT in this change**: a
  diff that reformats ten files and adds a publish path cannot be reviewed for
  either. The gap is worth a change of its own.
- **That the adapter compiles.** `cfg(logos_scaffold)` is set by the builder and
  by nothing else, so `src/lib.rs`'s `publishing` helper and its three forwards
  are type-checked by no gate in this repo. §8's test covers the one thing about
  it that could plausibly fail; the rest is unchecked until a module build runs.
- **Whether the thread derivation is right about an inbound parent.** It
  propagates the parent's own `thread` claim without auditing it, which is the
  correct division (`design.md` argues it) and means a peer that received a reply
  naming a thread its parent does not belong to will publish further replies into
  that same wrong thread. Auditing the graph is the read side's, and the read
  side does not exist.
- **That a Stoa's posting policy is enforced.** It is not, and `Policy` has one
  variant, so there is nothing to enforce yet. Named in `design.md` rather than
  left to be discovered.
