# Specs for the two capabilities that shipped before the spec flow existed

## Why

`openspec/specs/` holds two capabilities, `stoa-genesis` and `op-format`. The
crate holds six modules. The gap is not a judgement about which code deserves a
contract — it is an accident of ordering: `identity.rs`, `wire.rs` and
`cursor.rs` landed in PR #5, and the spec-driven flow arrived in PR #8.

So the two modules with no behaviour contract are the two the rest of the crate
is built on:

- **`identity.rs`** is what `op-format` means by "a signature", "an author" and
  "a valid public key". `op-format`'s requirement *Verification answers
  authenticity, not authority* is a statement about a function in `identity.rs`,
  written into a spec that does not own it. `stoa-genesis`'s scenario *A creator
  key that can never verify a signature is refused* is likewise a claim about
  `PublicKey::from_bytes`, stated where the behaviour is consumed rather than
  where it is defined.
- **`wire.rs`** is the module's entire public surface. CLAUDE.md calls the core
  API "the deliverable" and PLAN.md §2.5 fixes its conventions, but no spec
  states them, so the one contract a view actually programs against is the one
  with no requirements.

Both are also the places where an unspecified behaviour is most expensive.
Identity is where authenticity is decided; the wire boundary is where a panic
stops being a bug and becomes a dead module process (PHASE0-FINDINGS §3).

**This change is retroactive**, with the hazard that implies: it is easy to
write a spec that narrates the implementation instead of stating a contract. See
`design.md` for what was done about that, and for the requirement-to-test audit
that is the main product of writing a spec after the fact.

## What Changes

Documents only. No code, no tests, no behaviour. The suite was green before this
change and is green after it, over identical code.

- An `identity` capability spec: what a key, an address and a signature
  guarantee; what is refused at the parse and why; and — stated as a
  requirement rather than left to be inferred — what identity deliberately does
  not do.
- A `module-wire-contract` capability spec: the JSON-in/JSON-out convention, the
  single failure shape, and the panic guard that keeps the module process alive.
- A `design.md` carrying the reasoning, the capability-naming argument, the
  `cursor.rs` decision, and the defects the exercise turned up.
- A `tasks.md` whose centre is a requirement-to-test table naming every
  requirement no test pins.

The code being specified, for reference:

- `dialectica-core/src/identity.rs` — `Address`, `PublicKey`, `SecretKey`,
  `Signature`, `derive_stoa_key`, `sign_op_bytes`, `verify_op_bytes`,
  `verify_authored_op`, `stoa_address`.
- `dialectica-core/src/wire.rs` — `guarded`, `error_json`, and the handler
  bodies behind them.

## Capabilities

**New Capabilities**

- `identity` — keys, per-Stoa derivation, addresses and their display form, what
  a signature proves, and the separation between authenticity and authority.
- `module-wire-contract` — the JSON contract at the module boundary: the reply
  shape, the single failure shape, and the panic guard.

**Modified Capabilities**

None. `op-format` and `stoa-genesis` both make claims that now have a home in
`identity` — see `design.md`, "Requirements that already live in another spec",
for why they were left where they are rather than moved in this change.

**Considered and declined**

- `cursor` — argued and declined in `design.md`, with the condition that would
  reopen it.
- Splitting `identity` into `identity` and `identity-addressing` — declined in
  `design.md`.

## Impact

- No source file changes. `openspec/changes/spec-backfill/` is the whole diff.
- No wire-contract change: `module-wire-contract` describes the surface that
  exists today, which is Phase 0's probe surface plus the delivery bridge.
  Nothing forum-shaped is exposed yet.
- **Unblocks nothing and blocks nothing.** What it changes is that the next
  change to either module has a contract to modify rather than a blank page —
  and that four behaviours nobody had noticed were untested are now written
  down as untested.
