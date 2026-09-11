# Design

## Context

See `proposal.md` — Why. Two of the crate's modules shipped before the
spec-driven flow existed, so the code that everything else is built on is the
code with no behaviour contract. This change supplies one for each, and changes
no code.

**These documents were written after the code**, which has a specific hazard:
with the implementation sitting there to be paraphrased, it is easy to produce a
spec that narrates functions rather than stating a contract. Three things were
done to resist it, following the `op-model` change's precedent.

The spec is written in terms of what a caller may rely on and what a hostile
peer cannot do, naming no function and no type. Where the code does something
that cannot be stated that way, it is recorded below as unspecifiable rather
than dressed up as a requirement.

Properties were **mutation-verified** where the claim was load-bearing: broken
in the source, the suite run, the failures recorded, the source restored, the
worktree confirmed clean. The table is below.

And every requirement was traced to a test before being written, with the
untraceable ones listed as gaps rather than quietly dropped. That table is in
`tasks.md` and is the main product of this change.

## Goals / Non-Goals

**Goals**

- A behaviour contract for the two unspecified modules on the crate's trust
  path.
- An honest inventory of what those modules do that no test pins.
- A home for the identity claims that `op-format` and `stoa-genesis` currently
  state on identity's behalf.

**Non-Goals**

- **No behaviour change.** One test was added under an explicit scope exception
  (argued in `tasks.md`); every other gap found is recorded, not fixed. The test
  is in its own commit so the documents can be reviewed without it.
- **No keystore.** The specs cover key *types*; persistence, unlock and
  at-rest encryption are a separate capability that does not exist yet.
- **No forum-shaped wire surface.** `module-wire-contract` describes what is
  actually exposed today, which is Phase 0's probe surface plus the delivery
  bridge.
- **No rotation, no claims layer.** Both are named in the spec as deliberate
  absences, which is different from specifying them.

## Decisions

### `cursor.rs` gets no spec

**Declined, and this was the closest call in the change.**

The argument for it is real and was not dismissed. Every decoder's
hostile-input safety rests on that read head; a panic there aborts the module
process; and "a decoder refuses truncation rather than panicking" is observable
behaviour that both `stoa-genesis` and `op-format` already state separately,
which is exactly the duplication that usually signals a missing shared
capability.

It was declined for three reasons, in ascending order of how conclusive they
are.

**A spec is a contract for a caller, and this has no callers.** `lib.rs` makes
the module private with a comment saying why — "a shared decoding primitive, not
part of the module's contract" — and the type is `pub(crate)`. Nothing outside
the crate can depend on it, so there is nobody for a contract to be *with*. A
capability spec here would be a description of an implementation detail, which
is the thing a retroactive spec is most at risk of producing.

**The behaviour that matters is already specified, at the boundary where it is
observable.** What a caller can actually check is that *a decoder* refuses
truncation and trailing bytes distinguishably and never panics. Both specs say
so, and both are pinned by tests against the decoders rather than against the
cursor. Moving those statements down a layer would make them less checkable,
not more: the cursor's own tests are unit tests of a private type, and a
contract stated at a level no external caller can observe is a contract no
external test can pin.

**The duplication is the point rather than an accident.** `stoa-genesis` and
`op-format` each say their decoder refuses truncation because each decoder must,
and because the two are independently replaceable — one could stop using the
shared cursor tomorrow and its requirement would still hold. Extracting the
statement into a `cursor` capability would couple two contracts to one
implementation choice, which is the opposite of what a behaviour contract is
for. The generality here is real at the level of *code*, and the right
response to that was the one already taken: the refactor commit that lifted the
read head out of `stoa.rs` so the op decoder could share it rather than copy it.

**What would change my mind**, stated so the next person does not have to
re-derive it: the cursor becoming reachable from outside the crate. If it is
ever made `pub` — because a third decoder lives in another crate, or because a
caller needs to drive it directly — it acquires callers, and a caller is who a
contract is with. A second trigger is weaker but worth naming: if a third
decoder arrives and the three requirement sets are found to be *identical*
rather than merely similar, that is the demonstrated generality the process rule
asks for, and the extraction should be reconsidered then. Note that the
`op-model` change asked a closely related question about the two *encodings* and
declined it for reasons that partly apply here — its analysis is worth reading
first.

### The capabilities are named `identity` and `module-wire-contract`

**`identity`** over `keys`, `crypto` or `author-identity`. The module's subject
is not the cryptography — the scheme is an implementation choice the spec
deliberately does not name — it is what an identity *is* in this forum: one
per Stoa, permanent, unlinkable across Stoas, derived rather than registered.
Naming it after the primitives would make the per-Stoa scoping look like a
detail of key handling when it is the central requirement.

**`module-wire-contract`** over `wire`, `api` or `module-api`. "Wire" alone is
ambiguous in this crate, which has two other things that are emphatically wire
formats — the op encoding and the genesis encoding — and neither has anything to
do with this. The name says which wire: the module boundary, where JSON crosses
between a view and the core. The longer name is worth the two extra words for
not colliding with `op-format` and `stoa-genesis` in a reader's head.

### Identity is one capability, not split into `identity` and `identity-addressing`

Considered, because the addressing half is genuinely separable: address
derivation, the display form and its strict parser would make a coherent
capability, and addresses appear in contexts identity does not (a Stoa address is
not an identity at all).

Declined, because the README warns against pre-generalising and this would be
exactly that. **This is the first instance.** The rule is to reorganise when a
second instance shows that requirements written for one capability are really
about a general one, and to do it when the generality is demonstrated rather
than predicted — so writing the split into the first spec is deciding the
question before the evidence exists.

The concrete cost of splitting now is also higher than it looks. The two halves
are entangled at the requirement level, not merely adjacent: *An address is
derived from a record, never from a bare key* exists to keep a key log possible,
which is a statement about rotation; and *Verification binds the key to the
claimed author* is a requirement about addressing whose entire purpose is
authenticity. Split them and each spec carries a requirement whose reason lives
in the other.

**What would demonstrate the generality**: a second addressed thing that is not
an identity and not a Stoa — a thread address, say, or a claim address —
carrying the same derivation, display and strict-parse requirements. At that
point the addressing requirements would be about addresses rather than about
identities, and the extraction would be the `REMOVED`-plus-`ADDED` move the
process describes.

### Pagination is deliberately absent from `module-wire-contract`

**The convention has no implementation.** CLAUDE.md and PLAN.md §2.5 both state
that paginated calls take `(page, perPage)` and return
`{"items":[...],"page":N,"hasMore":bool}`, and it was tempting to write that
into the spec since it is clearly intended to be part of the contract.

It is not in the spec, and the reason is the rule against scenarios that cannot
be tested. **No method in the module is paginated**, and no code anywhere in the
crate constructs or consumes that shape — verified by reading every handler in
`wire.rs` and every method of the contract trait in the module crate's `lib.rs`.
`grep -rn "hasMore\|perPage"` over the crate is the check, and it is empty.
There is nothing to write a WHEN/THEN against, and a requirement whose every
scenario is hypothetical is a requirement no gate can see.

So it is **a convention awaiting a caller**, and it stays in PLAN.md §2.5 where
conventions-not-yet-built belong. The change that adds the first paginated
method should add this requirement in the same change, with the scenarios that
method makes checkable — and that is the correct ordering rather than a
deferral, because the shape's details (is `page` zero- or one-based? what does
`hasMore` mean on an empty final page?) are decisions that change nothing until
something implements them, and inventing answers now would fix them by accident.

### The spec does not name Ed25519, SHA-256 or HKDF

The scheme was chosen on recorded criteria and the reasoning is extensive, but a
behaviour contract is the wrong place for it. What a caller may rely on is that
a key of the wrong length is refused rather than fatal, that a key which can
never verify is refused at the parse, that derivation is deterministic and
one-way. None of those is a statement about a curve.

Stating the algorithm in the spec would also make a future scheme change look
like a contract change when it is an implementation change with a contract that
survives it — which is precisely the property the version-in-the-prefix
construction was built to have.

**The exception is the pinning requirement**, which does mention that constants
are pinned to independently derived answers. That is a statement about how the
contract is defended rather than about the algorithm, and it is checkable
without naming one.

### Requirements that already live in another spec were left there

`op-format` states *Verification answers authenticity, not authority*, and its
scenarios describe behaviour implemented in `identity.rs`. `stoa-genesis`
states *A creator key that can never verify a signature is refused*, likewise.

Both restate a property the `identity` spec now owns, and the tidy move would be
to `REMOVED` them there and `ADDED` them here. **That was not done**, for two
reasons.

The properties are genuinely load-bearing *in both places*, and they are not the
same statement. `op-format`'s version is about what a caller of op verification
may conclude; `identity`'s is about what the verification primitive answers.
`stoa-genesis`'s version is about what makes a record describe a Stoa at all;
`identity`'s is about what makes a key worth holding. Each spec's requirement is
the one its own reader needs.

And moving requirements between merged capabilities is a behaviour-neutral but
contract-visible edit to two specs that are already merged and already have
reviewers' attention. Doing it inside a change whose stated scope is "add specs
for unspecified modules" would widen the diff into the two things this change
said it would not touch. **If the duplication proves to drift, that is its own
change** — and the drift would be visible, because both sets of scenarios are
pinned by tests.

## Risks / Trade-offs

- **A retroactive spec can freeze an accident as a requirement.** → The
  mitigation is that every requirement was traced to a test or listed as
  untested, so a reader can see which claims rest on a deliberate decision and
  which rest on nothing. The four gaps in `tasks.md` are the honest answer to
  "which of these did nobody decide?".
- **`module-wire-contract` specifies a surface that is mostly Phase 0
  apparatus.** → Accepted. `panic_probe` is explicitly apparatus to be removed,
  and `delivery_channel_exists` exists to prove a bridge rather than to serve a
  forum need. But the *conventions* they demonstrate — one failure shape, a
  guard on every handler, no partial success — are the durable part, and those
  are what the requirements are about. The spec deliberately states no
  requirement naming an individual method.
- **Specifying "identity does not rotate" makes an absence a contract.** →
  Deliberate, and it is the point of stating it. An unstated absence gets
  quietly filled in by whoever next needs it; a stated one has to be removed on
  purpose, which is the deliberateness the design wants given that rotation
  waits on the claims layer rather than on someone getting round to it.
- **The weak-key requirement is stated as load-bearing on evidence from one
  test.** → Named as a gap rather than smoothed over; see the finding below.

## Mutation verification

Each property was broken in the source, the suite run, the failures recorded,
and the source restored. `git status` confirmed clean afterwards.

| Mutation | Tests that failed | Verdict |
|---|---|---|
| Disable the weak-key refusal in `PublicKey::from_bytes` | `a_low_order_public_key_is_refused_at_the_parse` **only** | Pinned, but by a single test at the primitive. See the finding below — the genesis boundary did not notice. |
| Disable the address binding in `verify_authored_op` | `a_validly_signed_op_under_the_wrong_key_is_still_rejected` **only** | Pinned, by exactly one test. `op.rs`'s own verification tests all stayed green. |
| Drop `String` payload handling from the panic guard's downcast | `guard_carries_a_formatted_panic_payload` | Pinned, and specifically — it is the only test distinguishing the two payload types. |

## Findings

These are reported, not fixed. A retroactive spec is a good way to find
defects, and these are what it found.

### The genesis decoder's weak-creator-key refusal is pinned by nothing

**This is the most significant finding, because it is a gap under a requirement
that is already merged and already claimed.**

`openspec/specs/stoa-genesis/spec.md` carries the scenario:

> **Scenario: A creator key that can never verify a signature is refused**
> — **WHEN** a record carries a well-formed creator key under which no signature
> can ever verify — **THEN** decoding fails — **AND** the failure is
> distinguishable from a malformed key

The genesis decoder has exactly one creator-key test,
`stoa::tests::an_invalid_creator_key_is_refused`, and it constructs `[0x02; 32]`
— not a valid Edwards point, so it asserts `InvalidCreator(NotAValidPublicKey)`.
**No test anywhere puts a low-order key into a genesis record.**

Measured, not inferred: with the weak-key check disabled in
`PublicKey::from_bytes`, that test stayed green and so did every other test in
`stoa.rs`. Only the `identity.rs` primitive test failed.

So the merged scenario's "AND the failure is distinguishable from a malformed
key" half is asserted by nothing at all at the genesis boundary, and the "THEN
decoding fails" half is inherited transitively from a primitive whose own guard
has one test. The consequence is the one `identity.rs` documents in its own
comments: a Stoa whose sole moderator can never authorise anything, decoding and
self-authenticating like any other.

This is also a textbook instance of the fixture trap this project keeps paying
for — the test exercises the creator-key path where *one* of the two rules is
silent, so it cannot tell the two refusals apart.

Note how close the machinery already was:
`every_error_renders_without_leaking_rust_syntax` constructs
`InvalidCreator(KeyError::WeakPublicKey)` explicitly, so the variant was
*rendered* by a test while being *reached* by none. A variant that only ever
appears in a Display test is a variant no decode path is known to produce.

**FIXED in this change**, under a deliberate scope exception granted because no
other open branch touches `stoa.rs` — see `tasks.md`. The test is
`a_creator_key_that_can_never_verify_is_refused_distinguishably`, in its own
commit, watched failing before it passed. Its failure output with the guard
disabled is the defect stated plainly:

```
left:  Ok(Genesis { creator: PublicKey(0000…0000), policy: Open, title: "Agora" })
right: Err(InvalidCreator(WeakPublicKey))
```

That is a genesis record decoding cleanly, self-authenticating, and naming a
sole moderator who can never authorise anything.

### Two security properties each rest on exactly one test

The address binding in `verify_authored_op` and the weak-key refusal in
`PublicKey::from_bytes` are each pinned by a single test, measured above. Both
are the kind of check whose removal leaves everything working — signatures still
verify, records still decode — so the single test is the only thing between the
property and a silent regression.

Not a defect today, and both tests are well-constructed (the address-binding one
in particular asserts that the signature *does* verify under its own key, so it
cannot pass because both halves are broken). Recorded because a single point of
failure on an authenticity check is worth knowing about, especially given that
`op.rs` has its own verification tests which one might reasonably assume covered
this and which measurably do not.

### `verify_strict` versus `verify` is pinned by no test, and the code says so

`identity.rs` requires `verify_strict` and argues at length that this is a
correctness requirement rather than belt-and-braces: every peer must run the
same predicate or peers disagree about whether an op is validly signed, which
partitions the network in the one place the design cannot tolerate it.

**No test distinguishes them.** The code is candid about this — the comment on
`verification_refuses_a_low_order_key_obtained_around_the_parse` says the
swap was measured to leave that test green, and that the guard is the doc
comment plus the deliberately-absent `Verifier` import rather than a test.

This is honest and well-documented, and it is the reason the `identity` spec has
**no requirement about strictness**. Writing one would mean writing a scenario
nobody can check, since separating the two predicates needs a crafted
small-order forgery. It is recorded here so the absence is visibly a decision.

### Cross-Stoa unlinkability is stated more strongly than any test can show

The spec requires that no public derivation exists — the property cross-Stoa
unlinkability actually rests on. That is an absence, and an absence of an
algorithm cannot be tested; what the tests show is the much weaker fact that two
derived keys differ.

The requirement is kept because it is the real contract and because it
constrains future change (adding a public-derivation function would violate it,
and a reviewer with the requirement in front of them can say so). But the
scenarios under it deliberately claim only what is checkable, and `tasks.md`
records the gap rather than letting the scenarios imply coverage they do not
have.

## What could not be stated as a requirement

Recorded rather than invented, per the rule against untestable scenarios.

- **`SecretKey::generate` panics if the OS random source fails.** Deliberate and
  well-argued in the code — there is no safe fallback, and continuing with a
  predictable key forges every signature that identity ever makes. It is not in
  the spec because it cannot be triggered: there is no way to make the OS random
  source fail from a test, and a scenario nobody can run is one the rule
  forbids. The reachability argument (key generation happens at keystore setup,
  never while serving an inbound op) is also a claim about a keystore that does
  not exist yet.
- **The secret key type's denial of `Clone`, `Debug` and `Serialize`.** Stated
  in the spec as a requirement, but its scenarios cover only the round trip,
  because the denials are compile-time facts. A test that a type does *not*
  implement a trait is not expressible in the normal suite; it would need a
  compile-fail harness the project does not have. The requirement is worth
  stating anyway — it tells a future author that adding `#[derive(Clone)]` is a
  contract change — but nothing pins it, and `tasks.md` says so.
- **"The module keeps serving after a panic."** The guard requirement's last
  scenario says the module answers subsequent calls. In-process this is
  untestable in the way that matters: the unit tests prove the guard converts a
  panic into the error shape, but the property they cannot show is the one
  PHASE0-FINDINGS §3 measured against a *running host* — that without the guard
  the process aborts. The tests would not fail in that case; they would abort
  the test binary. The scenario is kept because it is the actual contract and it
  was verified experimentally, but the verification is a Phase 0 finding rather
  than a suite gate, and that is recorded in `tasks.md`.

## Open Questions

- **Should the one-sided `verify()` tests be given their negative half?** The
  `op-model` change identified three and left them, reasonably, because changing
  them is a behaviour change to the suite. **#11 has since added a fourth** —
  `a_metadata_op_by_a_non_moderator_is_authentic` — which is what an unfixed
  pattern does: it becomes the model the next test is written against. The
  property they pin is also now load-bearing for `moderation.rs`, which decides
  authority on read. Both facts argue the same way, and the fix is cheap: have
  each also assert that a deliberately broken version of the same op does not
  verify. Recorded in `tasks.md` where a reader of the `identity` spec will meet
  it.
- **Does `module-wire-contract` survive the removal of `panic_probe`?** The
  method is marked for deletion once the panic question stops being live, and
  the guard requirement's "exercised rather than merely asserted" scenario is
  written against it. The requirement should outlive the method — but whoever
  removes it needs to replace the exercise with something, or the guard goes
  back to being a claim no gate can see.
