# Δ Dialectica

A decentralized forum, built as a [Logos](https://logos.co) module.

> **Status: planning.** No code yet. [`docs/PLAN.md`](docs/PLAN.md) is the
> design; everything below summarises it.

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

Nothing to build yet. When there is, it will be through
[`logos-scaffold`](https://github.com/logos-co/scaffold):

```
lgs basecamp build --variant all
lgs basecamp install
lgs basecamp launch alice
```

## Where it lives

Developed on [Radicle](https://radicle.xyz), with GitHub carrying CI and
releases — the Logos module catalogue is published from GitHub Releases, so
releases have to originate there.

Published to the catalogue at
[fryorcraken/logos-modules](https://github.com/fryorcraken/logos-modules).

## Licence

Dual MIT / Apache-2.0, at your option.

---

Dialectica is an independent community project. It is not built for, on behalf
of, or as part of the work of Logos or the Institute of Free Technology, and has
not been reviewed, audited, approved or endorsed by either.
