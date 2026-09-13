# Design: the store seeder

## What this is

One file, `dialectica/rust-lib/dialectica-core/examples/seed_store.rs`, run as
`cargo run --example seed_store -- [--fresh] <directory>`. It builds a whole peer
state — keystore, identity record, membership store, op log — holding one Stoa,
two threads, three replies and four votes, then prints the two values a caller
cannot derive and a request they can paste.

Nothing outside `examples/` changes.

## Decisions

### It goes in `dialectica-core`, not `dialectica`

Three reasons, and the third is the one that settles it even if the others did
not.

- **Every function it calls lives there.** `authoring`, `keystore`,
  `membership`, `identity_store`, `log`, `feed`, `moderation`, `stoa` are all
  `dialectica-core` modules. An example in the outer crate would reach them
  through a re-export and buy nothing.
- **The outer crate does not build in a plain checkout.** `dialectica` depends on
  `logos-rust-sdk` through `path = "../logos-rust-sdk-src"`, a directory the
  builder stages and nobody commits. `cargo run --example` there fails before
  compiling anything, with an error naming neither Nix nor the builder — the
  same trap `ci.yml` carries a whole step to work around. A tool whose first
  instruction is "first stage an SDK from a flake lock" is a tool nobody runs.
- **There is nothing in the outer crate to reach.** Its only content beyond the
  forwarding adapter is behind `cfg(logos_scaffold)`, which `cargo test` does not
  set and an example does not either. An example there could call the same `core`
  functions and no more.

The `cfg(logos_scaffold)` constraint is therefore real but not binding here: the
seeder needs no path that only exists under it, because everything the adapter
does with a host-supplied directory — derive four file paths, open four stores —
is a `core` function over a `&Path`, and the directory is an ordinary argument.

### It writes a real keystore, where the #31 original used throwaway keys

This is the substantive change from the original and it is not a refinement; the
original's approach has become wrong.

PR #31 predates `identity-onboarding` and the keystore write path, so it called
`SecretKey::generate()` three times and said so honestly: *"the keys are
discarded when this exits, so the seeded posts cannot be replied to as their
original authors afterwards."* That was the only option then. Now:

- **A Stoa's creator is its sole moderator, and the creator is fixed inside the
  address preimage.** A Stoa founded with a key the running module does not hold
  is a Stoa its own user can never moderate, and nothing can repair it — the
  address would have to change. `keystore.rs::creator_and_poster_in` exists
  because this exact class of mistake shipped once already.
- **`getCapabilities` would report `CannotPost`.** The probe reads the identity
  record, so a seeded Stoa with no recorded path renders a disabled compose box in
  whichever UI piece is being examined — which reads as a bug in that piece.

So the seeder mints a `Keystore`, writes it with
`keystore::protection_from_env()` (the same environment contract `keepIdentity`
follows, rather than a policy invented here), names `identity_public_key` as the
creator, signs with `stoa_key`, and records a chosen path.

**The second author is still a generated key, and that asymmetry is deliberate.**
A peer holds exactly one root secret, so two local identities is not a state the
module can be in. What a real store holds is one identity of its own plus ops that
arrived from peers — and an op from a peer is, to the log, an op signed by a key
this peer does not hold. The generated visitor reproduces that rather than
pretending to be a second local user.

### It refuses an existing store; `--fresh` is the explicit opposite

Three behaviours were available: overwrite, merge, or refuse.

**Overwrite is disqualified by one file.** `identity.key` holds the root secret in
exactly one place, so replacing it destroys every identity the user has —
including their authorship of every op already published to peers — and no error
anywhere says so. `Keystore::create` refuses to overwrite for precisely that
reason; a seeder that deleted the file first would be routing around a guard the
keystore put there on purpose.

**Merge was rejected** because it is not one behaviour but four, one per store,
and the interesting one is undecidable: a keystore that already exists means the
seeded Stoa's creator would be an identity the seeder did not mint, which is fine,
while an op log that already exists means the counts this program asserts on are
not about what it wrote. A tool whose assertions stop meaning anything on a second
run is worse than one that declines the second run.

**So it refuses, listing what it found, and writes nothing.** The check runs
*before the first open*, because `SqliteOpLog::open` and every sibling creates its
file — a check placed after would be checking a file this program had just made.
`--fresh` deletes the four by name, printing each as it goes so a person who
passed the flag by mistake can see in the scrollback exactly what they lost. It is
the only path here that removes anything.

### Both author addresses are printed, and their disagreement is asserted

The module derives a posting identity at one position (`stoa_address_at_path`, the
probe) and signs at another (`stoa_key`, the publish path). `ci.yml` carries a
named exemption for this and calls it what it is: *"which key a publish signs with
is a spec question this gate cannot answer."*

The seeder cannot settle it and does not try. What it does is **match the module
at each position**, so a seeded store behaves exactly as one the module built —
and then print both addresses side by side, labelled as the known gap. A UI
developer who saw only the probe's would read the feed as attributing their own
posts to a stranger and go looking for the bug in their own screen.

Two assertions hold this in place:

- `feed.items[0].author == signing_address` — measured discriminating, not
  vacuous: pointing it at `posting_address` instead makes the program fail, which
  was run.
- `posting_address != signing_address` — which **fails the day the gap closes**,
  and the failure message says so and says to delete both this assertion and the
  paragraph it documents. That is the self-invalidating shape CLAUDE.md asks for:
  a note about present state that cannot go quietly wrong.

### `Result<(), String>`, because six error types are not `Error`

`Box<dyn std::error::Error>` — the obvious signature, and the #31 original's —
does not compile. Only `OpLogError` and `MembershipError` implement
`std::error::Error`; `KeystoreError`, `GenesisError`, `IdentityStoreError`,
`Refusal`, `RandomnessUnavailable` and `OnboardingError` do not, so `?` cannot
convert any of them.

**That is a real gap in the public API and it is deliberately not fixed here.**
Adding six trait impls widens the crate's public surface, which this project
treats as a decision taken on purpose rather than a side effect of one caller
wanting `?` — and this piece changes no behaviour, so an API widening inside it is
exactly what the change flow exists to catch. Every one of the six implements
`Display`, so a one-line `why(context, result)` helper costs nothing.

The helper adds a context prefix rather than passing the message through, because
four of the stores here are SQLite files that fail with the same `rusqlite`
wording: "unable to open database file" names none of the four on its own.

**Recorded as a dead end for whoever wants `?` next:** propose the impls as their
own change, arguing that a public error type a caller cannot box is a public error
type a caller cannot compose.

### No CI change, and that was measured rather than assumed

The worry the brief raises is the right one — an example that stops compiling as
the API moves is a tool lost again silently, which is how the #31 original went.
The question is whether CI already sees it, and it does. A deliberate type error
was planted in the file and both Rust gates failed on it:

- `cargo test … -p dialectica -p dialectica-core` — cargo builds a package's
  examples as part of `cargo test`, so the `Tests` step compiles this file before
  running a test;
- `cargo clippy … --all-targets -- -D warnings` — `--all-targets` includes
  `--examples`, so `Clippy` compiles and lints it too.

Adding a third step would duplicate two that already work. **What CI cannot see**,
stated because a green gate that measured nothing is worse than none: nothing
*runs* the program, so a seeder that compiles and produces a useless store would
pass. The inline `assert!`s are what make a run fail loudly, and a run is a person
typing the command.

### No `#[test]` in this file, ever

CI's test-count gate counts `#[test]` attributes across the whole Rust tree with
`examples/` in scope, and cargo does not run an example's tests under
`cargo test`. One declared here makes `declared` exceed `ran` and fails the job —
correctly, because a test that cannot run is worse than no test. `core-e2e`
measured exactly this with a probe. The file says so at the top; verified zero
here.

### Counts are asserted, not merely printed

A seeder that printed an address without checking the store serves it is the one
failure mode that makes the output worse than nothing: a caller would trust a
store that does not work and go looking for the bug in the UI. So the feed is read
back through `feed::list_threads` — the same function the wire handler calls — and
two hardcoded numbers are asserted against it: two thread heads, nine ops. Both
come from the writes above rather than from the call being checked.

**The printed request was verified end to end by execution**, not by inspection:
the exact `{"stoa":…,"genesis":…}` line the program emits was fed to
`wire::list_threads_from_request` against the seeded directory, and it returned
the two-row feed rather than an error.

## What was NOT done

- **`--fresh` does not remove the directory**, only the four files it knows. A
  seeder that `rm -rf`'d a path the user typed is a different and much worse
  program.
- **No attachment is seeded.** The #31 original put a placeholder CID on one
  reply. `authoring::reply` takes no attachments parameter today — the field is
  `vec![]` inside `authoring.rs`, out of that change's scope — so there is no
  public way to publish one, and the original's call no longer exists.
- **No moderation op is seeded**, so nothing renders as hidden. The publish path
  has no `moderate` function; building one by hand would mean constructing an `Op`
  and appending it directly, which is exactly the "public API only" property that
  makes this program evidence of anything.
- **Nothing is delivered.** There is no transport (`op-transport` owns it), so a
  seeded store is local and says nothing about what a peer would see.
