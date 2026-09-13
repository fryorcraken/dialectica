## Why

**There is no way to look at the UI with real data in it.** Four UI pieces are in
flight — onboarding, the composer, the Stoa list, and a thread read — and every one
of them renders a screen whose interesting states only exist when a store already
holds a Stoa, some threads and some replies. Today the only way to reach those
states is to drive the module by hand through its own compose box, which cannot
produce a thread with two levels of nesting, two authors, or a second page.

**And a populated store is not enough on its own.** `list_threads` takes a
`genesis` alongside the `stoa` address, because `Moderators::of` is the only way to
build a moderator set and it takes a genesis record. So a caller needs two values,
and **neither can be recovered from the store by a person**:

- the **address** is `SHA-256` over the genesis record's canonical bytes, so it
  cannot be guessed;
- the **record cannot be recovered from the address at all** — that is the same
  hash, run backwards.

A seeded store whose address and record were never printed is therefore usable by
nothing but the process that wrote it. Printing both is not a convenience of this
tool; it is the reason it is a program rather than a test fixture.

This tool existed once, on the closed PR #31, and went out with it. It is restored
here rather than reinvented, and written against today's API — which has moved
substantially underneath it: `publish::*` is now `authoring::*`, `SqliteOpLog` no
longer carries a `StoaRegistry`, membership lives in its own store, and a signing
identity now comes from a real keystore instead of `SecretKey::generate()`.

## What Changes

- **A new example**, `dialectica-core/examples/seed_store.rs`, run as
  `cargo run --example seed_store -- <directory>`. The directory is a required
  argument with no default, because Basecamp's persistence directory is
  `module_data/dialectica/<instance-id>` and the instance id is host-assigned.
- **It builds a whole peer state, not just an op log.** A keystore
  (`identity.key`), an identity record (`identity.sqlite`), a membership store
  and an op log (`ops.sqlite`) — the same four files the adapter opens, under the
  same names, through the same `core` functions that derive them.
- **It signs with the keystore's real identity**, so the seeded Stoa's creator is
  a key the running module actually holds. A Stoa founded by a throwaway key is a
  Stoa its own user cannot moderate, permanently, because the creator is fixed
  inside the address preimage.
- **It refuses to touch an existing store.** If any of the four files is already
  present the program prints what it found and exits non-zero, having written
  nothing. Recreating is available as an explicit `--fresh` flag that names the
  files it will delete.
- **It prints the Stoa address, the genesis record, and a pasteable
  `list_threads` request**, then reads the feed back through the public API and
  asserts what it printed.
- **It prints both author addresses and asserts they still disagree.** The module
  derives a posting identity at one position and signs at another, so a seeded
  post's `author` is not the address `getCapabilities` reports. Printed side by
  side rather than hidden, because a UI developer seeing only one would read the
  feed as attributing their posts to a stranger.
- **No file outside `examples/` changes at all**, CI included. A deliberate type
  error was planted in the example and both Rust gates failed on it: `cargo test`
  builds a package's examples, and `cargo clippy --all-targets` includes them. A
  third step would duplicate two that already work.

## Capabilities

### New Capabilities

None. The seeder adds no requirement: it calls only public API that
`stoa-membership`, `content-authoring`, `keystore`, `identity-onboarding` and
`module-wire-contract` already contract, and it adds no method, field or reply
shape to the module surface. `.openspec.yaml` sets `skip_specs: true` with that
reason.

### Modified Capabilities

None.

## Impact

- **New file:** `dialectica/rust-lib/dialectica-core/examples/seed_store.rs`, and
  nothing else. No modified file at all.
- **No CI change.** Measured, not assumed — see above. The worry that an example
  rots silently is the right worry and the existing gates already answer it.
- **No new dependency.** The example reaches `hex` and the `rusqlite`-backed
  stores through the crate's existing normal dependencies.
- **A gap in the public API is documented and not fixed.** Six of the crate's
  eight error types do not implement `std::error::Error`, so
  `Box<dyn std::error::Error>` does not compile against them and the example
  converts through `Display` instead. Adding six trait impls is a widening of the
  public surface and belongs in its own change, not smuggled into a tool piece.
- **Which crate, and why it matters.** It goes in `dialectica-core`, the pure
  inner crate, because that is where every function it calls lives and because
  the outer `dialectica` package depends on `logos-rust-sdk` through a path that
  does not exist in a plain checkout — an example there could not be run without
  staging the SDK first, and nothing behind `cfg(logos_scaffold)` is reachable
  from an example in any case.
- **A `#[test]` must never be added to this file.** CI's test-count gate counts
  `#[test]` attributes across the whole Rust tree, `examples/` included, and
  cargo does not run an example's tests — so one declared here fails the job.
  That is the correct outcome and the file says so.
