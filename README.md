# Δ Dialectica

A decentralized forum, built as a [Logos](https://logos.co) module.

> **Status: Phase 0.** The module path is proven end to end — both modules load
> in Basecamp, the view renders, and the core reaches `delivery_module` across
> a real cross-process call. There is no forum yet.
>
> [Milestone 0.0.1](https://github.com/fryorcraken/dialectica/milestone/1)
> and the rest of [GitHub Issues and Milestones](https://github.com/fryorcraken/dialectica/milestones)
> are the sole source of truth for scope and roadmap; `openspec/specs/` and
> `openspec/changes/archive/` carry the design reasoning behind what has
> shipped, and [`docs/PHASE0-FINDINGS.md`](docs/PHASE0-FINDINGS.md) is what
> building the module path actually taught us, including the parts that
> contradicted the original plan.

## Stoas

The organising concept is the **Stoa** — a sub-forum anyone can create and
moderate. Two properties held in deliberate tension:

- **Permissionless creation.** Creating a Stoa needs no approval and no
  registration with a central service, because there is no global registry to
  register with. A Stoa is a genesis record its creator publishes.
- **Real moderation within a Stoa.** A Stoa's moderators shape it. A forum
  where nothing can be removed is not a forum, it is a firehose.

Dialectica is peer-to-peer software: it ships no servers and operates no
service. Each Stoa is created and moderated by its own participants, who are
responsible for what they publish and for how they moderate it.

## How it works

Two modules in one repository:

- **`dialectica/`** — the core, authored in Rust as a native Logos module. All
  network, storage, cryptography and state.
- **`dialectica-ui/`** — a thin QML view.

The split is forced, not stylistic: Basecamp sandboxes the QML engine, so a view
cannot reach the network or the filesystem itself.

Underneath, one **SDS reliability channel** per Stoa carries signed operations,
and each peer keeps a local SQLite store it can rebuild by replay. Posts are
signed by their author; moderation actions are valid only when signed by a
current moderator, and every peer verifies independently — so moderation binds
without anyone being able to forge it.

Attachments go to Logos Storage by CID, keeping large blobs out of the message
path.

## Building

Through [`logos-scaffold`](https://github.com/logos-co/scaffold):

```
lgs basecamp build --variant all
lgs basecamp modules
lgs basecamp install
lgs basecamp launch alice
```

`lgs basecamp modules` is not optional: `install` builds the project modules
but never reads the `dependencies` array in `metadata.json`, so without it
`delivery_module` is missing at runtime and the plugin fails to load — visible
only as a launcher tile that does nothing.

Unit tests run outside Nix, once the Logos Rust SDK is staged beside the crate.
The crate depends on it by a path that only a Nix build fills in, so without
this step `cargo test` fails with `failed to load manifest for dependency
logos-rust-sdk`. Run it from the repository root, and again whenever the
builder pin in `dialectica/flake.lock` moves:

```
nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src
```

Then:

```
cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core
```

The `-p` flags are load-bearing. Nearly every test lives in `dialectica-core`,
the pure inner crate; without them cargo tests only the outer package and
reports `ok` having run almost nothing.

## Working with the agent flow

Changes go through a spec-driven agent flow that runs on the **specflow**
Claude Code plugin, `specflow@agent-spec-flow`, which `.claude/settings.json`
enables. Until `agent-spec-flow` has a release, this repository does not name
its marketplace, so Claude Code cannot find it on its own. Once, before
starting a session here, three steps. `~/src/agent-spec-flow` below is only an
example; use the same directory in the first two.

1. Clone the plugin's repository into a directory of your choice:

   ```
   git clone https://github.com/fryorcraken/agent-spec-flow ~/src/agent-spec-flow
   ```

2. Register that clone as a marketplace, giving the directory you cloned into:

   ```
   claude plugin marketplace add ~/src/agent-spec-flow
   ```

3. Install the plugin for this project, from the root of a checkout whose
   `.claude/settings.json` already enables it (any checkout of `main` that
   has this section):

   ```
   claude plugin install specflow@agent-spec-flow --scope project
   ```

   Registering the marketplace makes the plugin installable, not installed.
   `--scope project` records it in `.claude/settings.json`, so in such a
   checkout `git diff .claude/settings.json` should show nothing afterwards.
   From a checkout without the entry it would add it to that tracked file.

The first two steps write nothing in this repository and can run any time. A
session started after the third lists the `specflow:*` agents and skills.

These steps go away once `agent-spec-flow` has a release and its marketplace
entry moves into `.claude/settings.json` as a pinned `github` source.

## Where it lives

Developed on [Radicle](https://radicle.xyz) at
`rad:z2ZEqSUxm9c3TWwxJfeeBcL9UAv4m`, with GitHub carrying CI and releases — the
Logos module catalogue is published from GitHub Releases, so releases have to
originate there.

Published to the catalogue at
[fryorcraken/logos-modules](https://github.com/fryorcraken/logos-modules).

## Licence

Dual MIT / Apache-2.0, at your option.

---

Dialectica is an independent community project. It is not built for, on behalf
of, or as part of the work of Logos or the Institute of Free Technology, and has
not been reviewed, audited, approved or endorsed by either.
