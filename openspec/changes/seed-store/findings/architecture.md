# Architecture findings: the store seeder

Reviewed `dialectica/rust-lib/dialectica-core/examples/seed_store.rs` at `c2bf6f5`,
in a worktree of `piece/seed-store`. Correctness and security were reviewed and
closed separately; nothing below re-reports one of their ten entries.

**Three of the four architectural questions the brief raises resolve in the
author's favour, and I verified each rather than accepting the argument.** The
crate placement, the "calls core rather than reimplementing it" property, and the
declined `--fresh` reordering are all judged correctly and are recorded in prose
below rather than as boxes. The CI-boundary claim is *almost* honest and has one
gate missing from it — that is entry 2. Entry 1 is a genuine shape defect.

Suite green in this worktree: 733 + 26 = 759 passed, 0 failed. Clippy clean for
both our packages with `--all-targets -- -D warnings`.

---

- [ ] **`dev-writer`** — `seed_store.rs:200` + `:431` — `store_files` exists to be
      the one place the four names live, and the op log is then spelled a second
      time outside it
      **Scenario:** `store_files` (`:192-202`) is introduced with a docstring
      arguing the point directly: *"One function rather than four `dir.join(...)`
      calls scattered about, because the names are not this program's to choose."*
      Three of the four entries honour that — they call
      `keystore::default_path_in`, `IdentityStore::default_path_in` and
      `membership::membership_path_in`. The fourth is `dir.join("ops.sqlite")`,
      correctly flagged at `:197-199` as having no `core` accessor and as *"copied
      rather than derived, and flagged as such — if that ever moves, **this** moves
      by hand."*

      **Singular "this" is wrong: it moves in two places.** The op log is opened at
      `:431` with a second, independent `SqliteOpLog::open(&dir.join("ops.sqlite"))`
      that does not consult `store_files` at all. So the function whose stated
      purpose is to be the single site for names is bypassed by the one name that
      actually needed a single site — the only one with no upstream function to
      protect it.

      **The failure this enables is the exact silent one the file's own preamble is
      about.** Change `store_files`' spelling and not `:431`, and `--fresh` deletes
      a file the program does not write while the seeder writes a store `--fresh`
      will not clean and the adapter will not open; every assertion still passes,
      because the assertions read back through the same `log` handle. Change `:431`
      and not `store_files`, and the refusal check stops seeing an existing op log,
      so the "a half-seeded directory is not a state this program can produce"
      guarantee at `:52` silently stops holding.

      **This is CLAUDE.md's "complexity in the data structure, not the logic",
      applied to the one case that needs it.** The repo already knows: `wire.rs:2149-2156`
      records a prior architecture finding that *"Three stores had three conventions
      in three layers; two of them now agree, and the third — the op log's
      `dir.join("ops.sqlite")`, inline in the adapter — is named in `design.md` as
      the remaining one."* Measured across the tree, `grep -n "ops.sqlite"` over
      `src/` and `dialectica-core/src/` shows the adapter spelling it twice
      (`lib.rs:539`, `:666`); this example adds two more. Four hand-copies of a name
      that has no owner.

      **The fix is small and is not widening**: bind the path once from
      `store_files`, or add a `const OPS: &str` in the example and use it at both
      sites. Either keeps the change inside `examples/` and makes the flagged
      hand-copy a single line, which is what its own comment already claims it is.
      **Severity: medium — a latent silent-failure shape in the file whose whole
      argument is that names are derived and not chosen.**

- [ ] **`dev-writer`** — `design.md:169-186` — the "what CI cannot see" section is
      the right section and is missing a gate: `cargo fmt` never reaches this file
      **Scenario:** the section names two gates that do compile the example and is
      candid that nothing *runs* it. Both halves are true and I reproduced the
      compile claim. But it presents `cargo test` and `cargo clippy` as the Rust
      gates, and CI has a third — `cargo fmt --manifest-path
      dialectica/rust-lib/Cargo.toml --check` at `ci.yml:696` — which does **not**
      see this file at all.

      Measured, with the exact CI invocation plus `-v` so the file list is visible:

      ```
      $ cargo fmt --manifest-path .../dialectica/rust-lib/Cargo.toml --check -v
      [custom-build (2021)] ".../dialectica/rust-lib/build.rs"
      [staticlib (2021)]    ".../dialectica/rust-lib/src/lib.rs"
      rustfmt --edition 2021 --check .../build.rs .../src/lib.rs
      ```

      Two files. `cargo fmt` does not follow the path dependency into
      `dialectica-core`, so **no file in the crate holding this example — or any of
      its logic — is format-checked by CI.** The example itself is clean today
      (`rustfmt --edition 2021 --check` on it directly passes, which I ran), so
      this is not a red-CI problem; it is that the section claiming to state
      honestly what the gates do and do not cover overstates the coverage by one
      gate.

      **The underlying gap is pre-existing and repo-wide, and is explicitly not
      this piece's to fix** — `findings/correctness.md` closes on the same
      observation and calls it a pre-existing repo gap, which is the right call.
      What belongs to this piece is one sentence in the section whose stated
      purpose is that *"a green gate that measured nothing is worse than none"*:
      of CI's three Rust gates, two compile this file and the third cannot see it.
      A reader who takes the current section at face value will believe a `fmt`
      regression here turns CI red, and it will not.

      Recorded as a `dev-writer` box rather than left in prose because the section
      is a claim about measurement and the correction is a one-line edit to
      `design.md`, not a code change.
      **Severity: low — documentation of a gate boundary, in the document whose job
      is to draw that boundary honestly.**

## What was clean, and the judgements the brief asked for

### Where it lives is right, and the deciding reason is real

`dialectica-core/examples/` rather than the outer crate. The stated deciding
reason — the outer package cannot build in a plain checkout — is **true, and I hit
it myself rather than reading it**. `dialectica/rust-lib/Cargo.toml:37` is
`logos-rust-sdk = { path = "../logos-rust-sdk-src" }`; `.gitignore:41` ignores that
path; `ci.yml:692` stages it with `nix build "github:$rev#rust-sdk-src" -o
dialectica/logos-rust-sdk-src`. My fresh worktree had no such directory, and
`cargo test` on the outer manifest could not resolve until I linked it to the main
checkout's `/nix/store` target — while `cargo run --example seed_store` against
`dialectica-core/Cargo.toml` compiled and ran with no staging at all. A tool whose
first instruction is "stage an SDK from a flake lock" is a tool nobody runs, and
that is not a rhetorical point here; it is the reviewer's own first five minutes.

The two supporting reasons also hold: every function the seeder calls is a
`dialectica-core` module, and the outer crate's only non-forwarding content is
behind `cfg(logos_scaffold)`, which an example does not set. This is the only
`examples/` target in the tree, so it sets the precedent — and it sets it in the
crate that can actually be run.

### It calls core; it does not duplicate it

This is the property that would matter most if it were violated, since a tool that
reimplemented derivation would drift from the adapter silently and stop being
evidence of anything. **It holds at all three derivation positions, each checked
against the adapter:**

- creator — seeder uses `keystore.identity_public_key()` (`:373`); adapter uses
  `core::keystore::creator_key_in` (`lib.rs:684`), which is
  `creator_and_poster_in(dir)?.0`, which is `ks.identity_public_key()`
  (`keystore.rs:479-484`). Same value.
- signing — seeder `keystore.stoa_key(&address)` (`:426`); adapter
  `keystore.stoa_key(&stoa)` (`lib.rs:539`). Identical call.
- probe — seeder `keystore.stoa_address_at_path(&address, recorded)` (`:648`);
  `wire::posting_identity` `keystore.stoa_address_at_path(stoa, path)`
  (`wire.rs:324`). Identical call.

I also checked for a **fourth** position the "three derivations" framing might have
missed: `wire.rs:1090` (`whoami`) uses `stoa_public_key_at_path`, which is
`stoa_address_at_path`'s own inner call (`keystore.rs:864-872`), so it collapses
into the probe's rather than being a fourth. The framing is complete.

Every write goes through `authoring::*`, `MembershipStore::join` or a `Keystore`
method, and the reads go through `feed::list_threads` and `OpLog::get` — the same
functions the wire layer calls. Nothing appends an `Op` directly. The "public API
only" claim is structural, not aspirational, and it is what lets the seeded store
be evidence.

**Verified end to end rather than by inspection**, which is the strongest form of
this claim: I fed the program's own emitted request to
`wire::list_threads_from_request` against the seeded directory and called
`wire::get_capabilities_from_stores` for the same Stoa. Both answered — the feed
returned its two rows and the probe returned
`{"canPost":true,"identity":"60a244de…"}`, matching the address the report prints
on its `getCapabilities reports` line. The store the tool writes is one the module
reads.

### The declined `--fresh` reorder is the right call, and the argument is sound

The author declined to move the assertions before the deletion, on the grounds
that they are assertions *about the store this run writes* and there is nothing to
assert until the ops exist. **That is correct and not merely convenient.** I
reproduced the window — changed `ops == 9` to `10`, ran `--fresh` over a populated
directory, and watched all four deletions print before the panic, leaving a
half-seeded directory with the old keystore gone. So the window is real, and it is
exactly what the help text now says it is.

The named alternative — seed to a temporary directory and rename into place — is
correctly identified as the design a tool that must never lose a store would use,
and correctly declined as widening. Two things make declining right rather than
lazy: `--fresh` is opt-in and now refuses a directory whose `identity.key` is not a
keystore (the security review's fix), so the case that actually destroys something
irreplaceable is already guarded; and the window is now documented in the help
text in the finding's own words rather than a softened paraphrase. Naming a real
alternative, saying why it is not taken, and documenting the residual cost is the
right shape for a decision like this.

The residual cost *is* real and is worth someone knowing: a failed `--fresh` run
leaves a directory that the next non-`--fresh` run will refuse, so recovery needs a
second `--fresh`. That is a consequence of the documented window rather than a
separate defect, so it gets no box.

### The CI boundary, judged

**The boundary is drawn honestly and the assertions do carry the weight now placed
on them** — with the one omission in entry 2 above.

What the section gets right is the part that is easiest to get wrong: it states
plainly that nothing runs the program, that a seeder which compiles and writes a
useless store passes every gate, and that "a run is a person typing the command".
That is a green gate correctly described as not measuring the thing that matters.

And the weight the assertions carry has grown appropriately with this round. Before
the review fixes there were two count assertions that a broken tree could satisfy;
there are now nine checks across four properties — two counts, three
`parent`/`thread` pairs read back through `OpLog::get`, two author lookups, and the
two self-invalidating inequalities. Crucially they are no longer all of one kind:
the counts, the structure and the identity relations fail independently, so the
single-explanation fixture defect this repo records is much harder to reach. Each
was measured discriminating by the correctness reviewer, and I reproduced the
`ops` one myself.

Two things I want on the record as limits of that judgement rather than as boxes.
First, the assertions only fire for someone who runs the program, and the one
person guaranteed to run it is the author — so their protection against *rot* is
weaker than their protection against a bad change made deliberately. The
`tasks.md` note to the `tester` already identifies the right closure for this (an
integration test over a `tempdir` in `tests/end_to_end.rs`, not a `#[test]` in
`examples/`), and that row is correctly left unticked rather than struck. Second,
`assert!` in an example is `debug_assertions`-dependent in principle; in practice
`cargo run --example` is a dev-profile build and `assert!`/`assert_eq!` are
unconditional in Rust regardless, so this is not a live hazard — noted only so the
next reader does not have to re-derive it.

### Smaller things that were right

**No new dependency**, and the hand-rolled two-argument parser is the correct call
with the reason stated at the call site (`:239-241`): a flag crate for one optional
flag is the unexamined widening the crate's dependency posture exists to prevent.
`hex` is already a normal dependency at `dialectica-core/Cargo.toml:110`. No
licence or supply-chain question arises.

**`main` has one job per section and no function has two.** The file is one
long `main` plus three small helpers (`store_files`, `usage`, `why`), and the
length is sequence rather than complexity — eight phases that each do one thing,
in the order a peer state has to be built. Splitting it into eight functions
threading a dozen bindings would be worse, and the banners already give a reader
the structure. No `handle`/`process`/`And` anywhere.

**The `Result<(), String>` decision is correctly scoped and correctly recorded.**
Six of eight error types not implementing `std::error::Error` is a real gap in the
crate's public API; adding the impls would widen the public surface inside a
tool piece, which is the shape the change flow exists to catch. Declining it,
documenting it in both `design.md` and the module docstring, and naming the
argument for whoever proposes it next ("a public error type a caller cannot box is
a public error type a caller cannot compose") is exactly right.

**`SEEDED_PATH` is a named constant with its reasoning attached** (`:177-183`), and
the reasoning is the non-obvious half: the value does not matter, only that the
seeder and the probe agree, and they agree because both read the record rather
than assuming a number. The code then honours it — `:644-648` reads the path back
out of the store rather than reusing the constant.
