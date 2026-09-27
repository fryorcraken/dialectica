## Context

See `proposal.md` for why: the README's `cargo test` cannot resolve its
manifest in a fresh clone, and the staging step it needs was written down
three ways, none of them in the README.

**The path dependency is the builder's design, not dialectica's.** The pinned
builder's `lib/mkLogosModule.nix` lays the crate out under `rust-lib/` and
copies its own SDK to `logos-rust-sdk-src` beside it, so
`logos-rust-sdk = { path = "../logos-rust-sdk-src" }` resolves to "the SAME rev
the generator came from". The generated `provider_gen.rs` is written against
that SDK. So the SDK outside a Nix build has to be *that* SDK, and every
decision below is about getting it there with no second pin.

The builder exports `packages.<system>.rust-sdk-src` for this purpose. Its
`flake.nix` says the output exists "so a codegen.rust module can stage it as
`../logos-rust-sdk-src` to generate its Cargo.lock against the SAME SDK the
builder links", and its own doctest stages the SDK this way before running
`cargo generate-lockfile`.

## Goals / Non-Goals

**Goals:**

- One staging command, run from the repository root, that resolves the builder
  through `dialectica/flake.lock` and nothing else.
- The README is the one place a reader is told how to stage. CI runs the same
  command byte for byte, which makes CI the README's witness.

**Non-Goals:**

- Changing `dialectica/flake.nix`, `flake.lock` or `rust-lib/Cargo.toml`.
- A drift gate asserting the README and `ci.yml` carry the same command. See
  Risks.

## Decisions

### D1. Resolve the builder with `--inputs-from ./dialectica`

```
nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src
```

`--inputs-from ./dialectica` makes each input of `dialectica/flake.lock` a
registry entry, so the bare name `logos-module-builder` resolves to the locked
builder. The staged SDK moves with the pin, and no rev is written anywhere.

**Measured on 2026-09-25, from a worktree root, with the relative form above:**

| Run | Result |
|---|---|
| the command above | link → `/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src` |
| `github:logos-co/logos-module-builder/<rev in dialectica/flake.lock>#rust-sdk-src` | same store path |
| `logos-module-builder#rust-sdk-src` with **no** `--inputs-from` | `error: cannot find flake 'flake:logos-module-builder' in the flake registries` |
| a registry mapping `logos-module-builder` to `github:NixOS/patchelf`, no `--inputs-from` | `does not provide attribute … rust-sdk-src` (the registry is consulted) |
| the same registry **with** `--inputs-from ./dialectica` | `lsdgw…` again (the lock wins) |

**`--inputs-from` is the guard, and what breaks without it is measured:** drop
it and the command fails outright, because no registry names the builder. It
does not silently fall back to an unpinned builder. The last two rows show the
lock also beats a same-named entry in a registry passed with
`--option flake-registry`. That is the global-registry slot. Precedence over
the *user* registry (`~/.config/nix/registry.json`) was not measured, because
doing so means editing the owner's configuration.

Alternatives, and what ruled each out:

- **Read the rev out of `flake.lock` with Python and build
  `github:$rev#rust-sdk-src`** (CI until now). It reaches the same store path,
  but it is a hand-written lock parser with its own guards (input type must be
  `github`, `rev` must exist) that Nix already performs, it adds `python3` to
  the job, and it is not a command a README reader can type.
- **A rev substituted by hand from `flake.nix`** (the `.gitignore` comment until
  now). A second pin that drifts. The failure mode of a stale rev is not an
  error: it is `cargo test` passing against an SDK the module does not link,
  which is a green gate testing the wrong thing (`PHASE0-FINDINGS.md` §2 on why
  an SDK's capabilities are a property of the builder pin).
- **Put the SDK in `Cargo.toml` as a Cargo git dependency.** It would build,
  since the builder allows git fetches. But it is a second pin beside
  `flake.lock` that can drift from the generator. The SDK's README says "You
  don't add this SDK to a `Cargo.toml` by hand", and it is not on crates.io.
- **Re-export the builder's `rust-sdk-src` from `dialectica/flake.nix`.** An
  earlier draft of the proposal took this route. The owner's comment on #168
  ruled it unnecessary if `--inputs-from` works. It does, so the build's own
  flake is not changed to serve developer tooling.
- **The runner links a store path into each agent tree.** What #91 did. It was
  the correct SDK (the owner's comment on #168 withdrew the concern that it was
  not: the runner linked `lsdgw…`, the same path this command gives). But ten
  dispatches waited on it, and one lost its worktree by stopping unchanged.

### D2. `-o dialectica/logos-rust-sdk-src`, an out-link rather than a copy

The out-link is a GC root, so a `nix-collect-garbage` does not delete the SDK
from under a working tree. It is also what the path dependency resolves
through, so nothing else is needed. Re-running the command replaces the link:
measured by pointing the link at an unrelated store path and re-staging, which
put it back on `lsdgw…`. That is what makes "run it again after the pin moves"
a sufficient instruction.

### D3. README and CI carry the command byte for byte

CI's step is the README's line with nothing added. The old step passed `-L`;
it is dropped, because `rust-sdk-src` is a source copy with no build log worth
printing, and a CI command that differs from the README's is a CI command that
is not testing the README.

The `test -f dialectica/logos-rust-sdk-src/Cargo.toml` after it stays. It turns
"the builder's output changed shape" into a failure at the staging step. Without
it, the first symptom is `cargo fmt` failing with
`failed to load manifest for dependency logos-rust-sdk`, which names neither
Nix nor the builder. This guard is CI-only, and no local test can turn it red.

### D4. The README documents it once; `.gitignore` and `CLAUDE.md` point there

Two copies of a command drift. `.gitignore` and `CLAUDE.md` point to the
README's "Building" section, and neither names the command or any other
staging method. CI is the one deliberate second copy, because it has to execute
the command. D3 is why that copy earns its place.

**A shared script called from both was considered and ruled out.**
`sh <relative-path>` costs an approval click in this repository (`CLAUDE.md`,
"what Bash costs"), while `nix build …` runs unprompted. The point of
`CLAUDE.md`'s pointer is that an agent in a fresh worktree stages its own tree
without waiting on anyone, and a script would put a click back in that path to
save one line.

### D5. The `rust` job's Nix-version comment is re-grounded, not dropped

The comment said the job needs no version floor because "it builds one
`github:` flake with no relative path input". After this change it evaluates
`dialectica/flake.lock` through `--inputs-from`, so that reason no longer
describes the step. The conclusion still holds. The floor exists because Nix up
to 2.24 rejects a lock file containing a relative `path` input, and
`dialectica/flake.lock` has none: its nodes are `github` and `git` inputs only
(`jq "[.nodes[] | .locked.type] | unique" dialectica/flake.lock`). The comment
now says that. `install_url` stays matched to the `build` job for the reason
the comment already gave.

## Risks / Trade-offs

- **README and `ci.yml` can drift apart**, and nothing checks that they match.
  If only `ci.yml` changes, CI goes on passing while the README line goes
  untested. → Both are one line, and D3 makes a byte-identical copy the rule. A
  gate is a follow-up if drift is ever observed; it is not built speculatively
  here.
- **A local link goes stale when the builder pin moves.** `cargo test` then runs
  against the old SDK until someone re-stages. → The README says to re-run the
  command when the pin moves (D2 shows re-running replaces the link). CI always
  stages fresh, so the gate itself cannot be stale.
- **A user-registry entry named `logos-module-builder`** could in principle
  compete with `--inputs-from`. Precedence over the global-registry slot is
  measured (D1); precedence over the user registry is not. → `nix registry
  list` on the machine this was measured on has no such entry, and the name is
  specific enough that one appearing by accident is unlikely.
