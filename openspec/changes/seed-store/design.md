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

Three assertions hold this in place:

- each feed row's `author` is looked up in the set rather than indexed, and both
  the founder's signing address and the visitor's must appear — measured
  discriminating: publishing the second root from the founder instead of the
  visitor makes the program fail, which was run.
- the **distinct-author set over all nine ops**, read back through `OpLog::iter`,
  must be exactly those two. See "What the report may claim" below for why an
  existential check was not enough.
- `posting_address != signing_address` — which **fails the day the gap closes**,
  and the failure message says so and says to delete both this assertion and the
  paragraph it documents.

**The third assertion's left operand is `wire::posting_identity`'s own answer, and
it was not always.** It used to be `keystore.stoa_address_at_path(&address,
recorded)` — a hand re-derivation of that function's body — against
`keystore.stoa_public_key(&address).address()`. Both operands were then keystore
calls this example made itself, two HD derivations off one root, so the assertion
asserted that two derivation paths differ: a property of Ed25519 key derivation,
not of dialectica. **Measured by the spec-test review**: closing the real gap at
`wire.rs`'s probe made `getCapabilities` report the signing address, and the
seeder exited 0 with every assertion green, still printing `MODERATION DOES NOT
WORK` about a gap that no longer existed. A pre-existing `wire.rs` test caught
that mutation, so the property is testable and is tested — just not by this piece,
which had claimed the credit.

Calling `wire::posting_identity` fixes both halves at once. It puts a value the
*module* produces on the left, so closing the gap moves it and the assertion
fires — re-run with the same mutation, and it now does, naming what to delete. And
it removes a third hand copy of a derivation from a file no test compiles the
logic of and no gate runs. `keystore.rs` records why that matters: *"two call
sites that agree is not the same thing as one derivation"*, after exactly such a
pair re-diverged with every gate green.

The alternative was to keep the hand derivation and record the reason — the only
one available being that it makes the seeder's dependency on the identity record
explicit. Rejected: `posting_identity` reads the recorded path itself, so it makes
that dependency explicit too, and it makes "matches the module at each position"
literally true rather than coincidentally true.

### What the report may claim: only values an assertion pinned

**Every claim this tool makes is pinned by an `assert!` that runs — except the
report's prose, and that is where the one false claim got in.** The counts, the
three `parent`/`thread` pairs, the feed authors, the inequality and the moderator
pair are all assertions. The report block was the twelfth claim and had no such
pin, and it is the only output most readers ever see.

The line was *"every seeded op is by ⟨one address⟩"*. It was true while both roots
were the founder's and went false the moment the second root moved to the visitor
— five of the nine ops are now the visitor's, an address the report did not print
at all. **No assertion could have caught it**, and the reason is structural rather
than an oversight: the two checks beside it are `authors.contains(...)`, which is
existential where the claim is universal. A `contains` check holds whether the
store has one author or two, so the false sentence and the green assertions were
consistent by construction — this repo's recorded "two explanations give the same
answer" family.

**The rule, recorded here so the next report line is not a one-off fix: the report
prints only values the program has asserted.** What that forecloses is summary
prose about authorship. A line beginning "every" cannot be written unless an
assertion establishes it, so what the report now carries is a per-author
breakdown — each address with its op count, counted from the store by
`seeded_ops_by` rather than restated from the code above it — plus one assertion
over the distinct-author *set*.

The alternative — free prose in the report, checked by review — is what was in
place, and it is what shipped a high-severity false claim through five reviewers
with every gate green. A set assertion fails today against the old sentence, and
keeps failing if a later change moves an op between identities, adds a third
author, or collapses the two into one, whether or not anybody updates the prose.
All three of those were run as mutations; the third-author and collapse cases both
panic.

### `Result<(), String>`, because almost no error type in the crate is `Error`

`Box<dyn std::error::Error>` — the obvious signature, and the #31 original's —
does not compile. **Every public error type in `dialectica-core` except
`OpLogError` and `MembershipError` lacks the impl**, so `?` cannot convert any of
the ones this program touches: `KeystoreError`, `GenesisError`,
`IdentityStoreError`, `Refusal`, `RandomnessUnavailable`, `OnboardingError`.

**Stated as a relation rather than a count, on purpose.** This entry previously
said "six of the crate's eight error types", and both figures were wrong — it is
ten of twelve, the four unlisted being `AddressError`, `KeyError`, `OpIdError` and
`OpError`, each with a `Display` impl and no `Error`. That matters beyond
arithmetic because the entry's job is to **size the deferral below**: whoever
proposes the impls would have scoped six and found ten, which is the direction
that gets a deferred change mis-planned rather than merely mis-stated. CLAUDE.md's
rule applies to `design.md` too — do not write down what a command can answer:

```
grep -rn "pub enum .*Error\|pub struct .*Error" dialectica-core/src/
grep -rn "impl std::error::Error" dialectica-core/src/
```

The relation stays true as types are added; a count rots silently.

**That is a real gap in the public API and it is deliberately not fixed here.**
Adding the trait impls widens the crate's public surface, which this project
treats as a decision taken on purpose rather than a side effect of one caller
wanting `?` — and this piece changes no behaviour, so an API widening inside it is
exactly what the change flow exists to catch. Every one of them implements
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

Adding a third step would duplicate two that already work.

**CI has a third Rust gate and it does not reach this file at all.** `cargo fmt
--manifest-path dialectica/rust-lib/Cargo.toml --check` does not follow the path
dependency into `dialectica-core`, so no file in the crate holding this example —
or any of its logic — is format-checked. Measured with `-v`, which prints the file
list: two files, `build.rs` and `src/lib.rs`. The example is `rustfmt`-clean today
and was checked directly, so this is not a red-CI problem; it is that a reader
taking "both Rust gates see this file" at face value would believe a `fmt`
regression here turns CI red, and it will not. The underlying gap is pre-existing
and repo-wide and is explicitly **not** this piece's to fix. Of CI's three Rust
gates, two compile this file and the third cannot see it.

**What none of the three can see**, stated because a green gate that measured
nothing is worse than none: nothing *runs* the program, so a seeder that compiles
and produces a useless store would pass. The inline `assert!`s are what make a run
fail loudly, and a run is a person typing the command.

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

**`OpLog::get` checks the store, not the reader, and that is this assertion's
limit.** It hands back the raw op, so the two fields compared are two fields this
program itself wrote. A reader that places posts by the claimed `thread` field and
a reader that derives membership by walking parents give the *same answer* against
a store whose parent chain is correct — and a seeded store's is, by construction.
So passing this says nothing about what a UI renders, which is the property the
tool exists to produce. That is this repo's recorded defect family: a fixture where
two explanations give the same answer.

**`core::thread::read_thread` is the assertion this wants to be, and this branch
cannot call it.** It is what `listThread` calls, it derives membership by walking
parents, and its own docstring says the claimed field decides nothing — so it would
check the *reader*. An earlier version of this entry said core had no thread read
and that `piece/thread-read` was "still in flight". **That was true when written and
is false now**: it merged as #61 (`53f08b3`), which is `origin/main`'s tip and the
commit immediately after this branch's merge base `733544d`. The sentence is
removed rather than softened, because a recorded reason that is false is worse than
no reason — the next reader stops looking.

What replaces it is a statement of where the branches are. `dialectica-core/src/`
on **this branch** has no `thread` module, so `read_thread` is not a symbol the
example can name; moving the assertion onto it requires the two branches to meet,
and this piece was told not to pull `main` in. Every argument `read_thread` takes
(`log`, `moderators`, `address`, each root's `OpId`, page, per-page,
`include_hidden`) is already in scope at the assertion site, so the edit is small
once they do. **That edit is the follow-up**, and the code carries the same note at
the assertion so a reader who never opens this file finds it.

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

### `SEEDED_PATH = 0`, a path onboarding cannot produce

The constraint is narrow: the seeder and the probe must agree on a path, and the
value must be reproducible across runs, because a person pastes this program's
output. They agree because both read the recorded row rather than assuming a
number, so the *value* is free.

Two ways to fill it. **Derive a slate and keep candidate 0, as onboarding does** —
heavier, needs a `SlateNonce`, and produces a store indistinguishable from a real
one. Or **pick a fixed path**, which is what was done. Zero is in range:
`identity_store`'s guard is `PATH_LIMIT`, the same constant
`onboarding::derive_path`'s mask is expressed in.

**The cost, which is the part worth recording: a seeded store's recorded path is
one onboarding would never produce.** `derive_path` is `SHA256(prefix || nonce ||
index)`, first four bytes big-endian with the top bit masked, so a slate's
candidates are pseudorandom values below 2³¹ and index 0 lands on path 0 with
probability ~2⁻³¹. Measured over 2000 generated nonces: zero hits, and three
sample slates printed as `[1336077579, 1318252717, …]` — nothing near 0. So a test
that assumed "any recorded path came from a slate" would be wrong about seeded
stores.

There is an argument that unreachability is the *better* property here, since it
makes a seeded store distinguishable from a real one, but it was not the deciding
reason and is not claimed as one. The constant's docstring previously asserted the
opposite of all this — that zero is "the first candidate of a slate, which is what
a user pressing through onboarding without deliberating would land on" — which was
simply false, and a reader who checked the stated reason found nothing to fall
back on.

### `ops.sqlite` is spelled once, in the one place with no upstream owner

`store_files` exists so the four filenames live in one place, and three of the four
honour it by calling `keystore::default_path_in`, `IdentityStore::default_path_in`
and `membership::membership_path_in` — a rename upstream moves them. The op log has
no `core` accessor; the adapter spells `dir.join("ops.sqlite")` inline, and
`wire.rs` records that as the remaining unowned one of the three store names.

**So the one entry that actually needed a single site was the one bypassing it.**
The op log was opened with a second, independent `dir.join("ops.sqlite")` that did
not consult `store_files` at all, and the failure that enables is silent in both
directions: change `store_files` alone and `--fresh` deletes a file the program
does not write while the seeder writes a store `--fresh` will not clean and the
adapter will not open; change the open alone and the refusal check stops seeing an
existing op log, so "a half-seeded directory is not a state this program can
produce" quietly stops holding. Every assertion passes either way, because they all
read back through the same `log` handle.

A `const OPS_LOG` plus an `ops_log_path(dir)` helper both sites call. Chosen over
binding the path once in `main` and passing it, because `store_files` is called
before `main` has a directory to bind from — `usage()` calls it with an empty path
to print the filenames. Keeping it a function keeps both callers deriving rather
than one deriving and one receiving.

### Assertion messages explain the expectation and interpolate nothing

`assert_eq!` already prints `left` and `right`, so a custom message's only job is
to say *why* the expected value is expected. The two count assertions appended
`", not {found}"` over the value actually found, which reads backwards on failure:
raising the expectation to ten printed *"two roots, three replies and four votes is
nine ops, not 9"* — telling the reader nine is wrong when nine is what the store
holds and the expectation is what moved. It names the fact and presents it as the
defect, in the one output a broken run exists to produce, and this piece's whole
architecture rests on those assertions being what makes a bad run loud. Both
messages are correct and complete with the interpolation dropped.

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
