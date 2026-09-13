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

### Moderation does not work on a seeded Stoa, and the tool says so

**This was found by review and the assertion that should have caught it could
not.** The original assertion was `moderators.contains(&genesis.creator)`, whose
comment claimed it caught the derivation trap head-on. `Moderators::of` sets
`creator: genesis.creator.clone()` and `contains` is `&self.creator == key`, so it
reduced to `genesis.creator == genesis.creator` — true for any value in that
field. It is this repo's recorded "asks the implementation what it did and agrees"
defect, in the one file whose entire job is to be evidence.

**The property it claimed to rule out is true today.** `Moderators::authorises`
gates on `entry.op.op.author` — the *signing* author — and the adapter signs with
`stoa_key` while the record names `identity_public_key`. Measured: pointing the
assertion at `founder.public_key()` fails. So a hide published through the module
against a seeded Stoa is **refused**, which is the outcome the "real keystore"
section below says the real keystore was adopted to prevent. Adopting it fixed the
half where the creator must be a key the peer *holds*; it did not make the creator
the key the peer *signs with*.

The tool cannot close that — which key a publish signs with is the spec question
`ci.yml` exempts, and a seeder that disagreed with the module would stop being
evidence of anything. So both halves are now asserted as they really are: the
record's creator does moderate, and the signing key does **not**. The second
assertion fails the day the gap closes and says what to delete. The report prints
the consequence in words, because a UI developer whose hide button does nothing
needs to read a line rather than debug their own screen.

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

### The keystore is read straight back through `open_in`

**Writing a keystore is not the same as writing one the module will open**, and
review measured the gap: seeding a `chmod 777` directory succeeded, printed a full
report with every assertion passing, and produced a keystore the module then
refused with *"the keystore's directory is writable by others (mode 0777)"*. A real
root secret in a directory any local user can replace, reported as success.

The cause is that `Keystore::create` checks only that the file does not exist,
while the directory-permission guard lives on the **read** path in `read_checked`.
So the fix asks the keystore rather than re-implementing it: `keystore::open_in`
after writing, which is the same call the adapter makes. A mode check written in
the example would be a second copy of a guard that already exists and is already
tested, and the copy no test covers is the one that drifts.

### `--fresh` checks that `identity.key` is a keystore before deleting anything

The four filenames are generic enough to collide with unrelated data, and printing
each deletion as it happens is after the fact. Only `identity.key` is checked,
because it is the only file here whose loss is unrecoverable — the other three hold
rebuildable content.

**`Keystore::is_encrypted` is the check, not a magic-byte comparison.** `MAGIC` is
private to `keystore.rs`, so spelling `0xD4` here would be a second copy of a
format constant no test covers — the defect family this repo records. `is_encrypted`
parses the real header and answers `NotAKeystore` for anything else. Verified: a
directory holding a 0600 non-keystore `identity.key` is refused with *"that file is
not a dialectica keystore; check the path"*, and the file survives.

### The nesting is asserted, because the counts cannot see it

A seeder that printed an address without checking the store serves it is the one
failure mode that makes the output worse than nothing: a caller would trust a
store that does not work and go looking for the bug in the UI. So the feed is read
back through `feed::list_threads` — the same function the wire handler calls — and
two hardcoded numbers are asserted against it: two thread heads, nine ops. Both
come from the writes above rather than from the call being checked.

**Those two counts are blind to the reply structure**, which review named:
`list_threads` returns thread *heads*, so a store where `nested` hung off the wrong
parent satisfies both. The nesting is the one thing this tool exists to produce,
because a UI cannot render a tree that is not there — and the indented tree in the
report is presentation, not a check.

So each reply is read back through `OpLog::get` and its `parent` and `thread`
asserted. **The `nested` row is the one that discriminates**: at two levels "the
parent's id" and "the parent's thread" are the same value, so a `thread: parent`
bug is invisible; three levels separate them. Measured — pointing `nested`'s
expected parent at `first_root` fails.

**Not through a thread read, because core has none.** `feed.rs` exposes
`list_threads` and nothing else; the thread read is `piece/thread-read`, still in
flight. When it lands, this is the assertion to move onto it.

### One root each, so the two-identity claim is visible

Both roots were the founder's, which made two claims false at once. The author
assertion indexed `items[0]` as though the index mattered when `items[1]` asserted
the identical thing; and the visitor signed only replies and votes, neither of
which `list_threads` returns — so the module docstring promising "two identities so
that author attribution is visible" described output where it was not visible at
all.

The second root is now the visitor's. The assertion checks both rows by lookup
rather than by index, so it compares two different values and does not bake in the
feed's ordering.

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
