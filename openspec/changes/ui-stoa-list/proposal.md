# Stoa list and join screens in the QML view

## Why

Core can now create a Stoa, join one, and list the ones this peer is in — the
`stoa-membership` capability contracts all three. The view cannot reach any of
them. `Core.qml` wraps `listThreads` and `getCapabilities` and nothing else, and
`Main.qml` takes `stoaAddress`, `stoaTitle` and `stoaGenesis` as properties a
developer fills in by hand, with empty defaults. A user running the built module
has no way to reach a Stoa at all: there is no list, no create affordance, and no
way to act on an address somebody sent them.

This change is the view half. It is also the first screen in this project where
the interface, rather than the core, is the security boundary. Joining is the
point where a user acts on a string that arrived from an untrusted channel, and
every property that makes that safe — that the address is the identity, that the
title is decoration, that a hash verifies a record and cannot reconstruct one —
is a property the *rendering* has to carry. Core already refuses a record that
does not match its address; what core cannot do is stop a screen from showing a
title and calling it verified.

## What Changes

- A **Stoa list screen**: the Stoas this peer is in, each row carrying its
  address beside its founding title, with the three read states kept visually
  distinct — holding Stoas, holding none, and unable to read membership at all.
- A **join preview screen**: what is about to be joined, shown before the user
  commits, with the address in full and the founding title labelled as founding.
- A **create affordance**: a title and nothing else, always offered, reporting
  core's own failure when there is no usable key.
- A **share affordance**: what a user copies in order to let somebody else join.
- `Core.qml` gains named wrappers for `create_stoa`, `join_stoa` and
  `list_stoas`, so a core method name is spelled in exactly one place.
- `Main.qml` stops taking a developer-supplied Stoa. The list becomes the view's
  entry point and the feed is reached from it, carrying the address and the
  genesis record the list already holds.

### Three places the mockup implies behaviour core does not have

The design bundle (`tmp/ui-design/handoff/`) is the visual and copy reference and
the owner has directed it be used. Its screens 02 and 03 are this change's. In
three places the mockup depicts something core cannot answer, and this proposal
records which way each was settled rather than leaving the divergence to be
rediscovered while implementing.

**1. A single paste field for a bare address cannot work.** Screen 02 shows one
field labelled `PASTE AN ADDRESS` holding `stoa:b02d5e77…` and a
`Look at it first` button. But `join_stoa` takes an address **and** the genesis
record, and that is a property of the address rather than a gap in the call: the
address is a one-way hash of the record, sufficient to verify a record somebody
hands over and insufficient to reconstruct one. A screen offering a field that
accepts an address alone offers a field whose successful-looking input can never
join anything. So the spec requires that what is pasted carries both halves, and
that what is copied carries both halves — the two are the same decision seen from
each end, and neither works unless the other does. `docs/UI-BRIEF.md` already
states this under *Joining a Stoa* and says the shareable thing's shape is "the
first question to answer for this screen"; this change answers it.

**2. The `CURRENT TITLE — CHOSEN BY A MODERATOR, CHANGEABLE` panel cannot be
filled.** Screen 03 renders the founding and current titles side by side, which
is exactly the right idea and is why the distinction is worth designing around.
Nothing resolves the moderator-signed metadata op that carries a current title —
`stoa-metadata`'s "Current metadata resolves by last-write-wins, falling back to
genesis" says plainly that resolution is not implemented and that the requirement
fixes the rule without claiming the behaviour exists. Core returns
`foundingTitle` and no current title at all. So the spec requires the founding
title to be labelled as founding, and requires that no current title be rendered
until a resolved one is actually supplied — the panel arrives with the value, not
before it. Rendering the founding title under a "current" label would assert
something no peer has checked, which is the one failure this distinction exists
to prevent.

**3. `31 posts received here` on a list row is not available.** Screen 02 puts a
per-row count of held posts in the right margin, and such a count would be
legitimate — it counts what this machine holds, which is the only kind of number
this software may show. But `list_stoas` items carry `stoa` and `foundingTitle`;
there is no count on them, and `list_threads` is paginated and returns
`hasMore` rather than a total, so no call answers "how many posts do I hold for
this Stoa". The spec therefore requires the row to render without one, and
requires that nothing be substituted — a page-length from some other call
rendered in that slot would be a number that looks like a total and is not one.
`nothing received yet` is separately unavailable for the same reason: it is a
claim about a count, and this build cannot distinguish "none" from "unknown".

A fourth, smaller one: screen 03's apparatus note says joining "generates you an
identity for it alone". Per-Stoa identity is built in core and **not switched on
in the first release** — one key signs in every Stoa — so that sentence is a
promise the software does not keep, and the spec forbids it.

### The bundle's copy is a default, not an authority, where it makes a claim

`copy.json` is written to be used verbatim and mostly should be. But it is a
design artifact rather than a contract, and it has been found to assert things
the software does not do — `onboarding.body` promises that a key "cannot be
linked to you anywhere else", which PLAN.md's identity section contradicts in as
many words for the MVP, and `onboarding.uniqueness` says "the same three words"
where the name is four. Neither string is on this change's screens; they are
cited because they establish that the file needs checking rather than copying.

So: where a bundle string makes a **claim about what the software guarantees** —
privacy, verification, permanence, reach, or a count — this change verifies it
before pinning it, and narrows it where it overreaches. Three on these screens
were checked:

- **`join.note`** — *"The address is a hash of the founding record, so pasting it
  is itself the verification. Nothing else here is verified."* The first half is
  accurate about the mechanism and **overreaching about what it buys**, which
  matters more here than anywhere else in the bundle. The check is a hash
  comparison between the two inputs the user supplied and consults nothing else,
  so it proves exactly that the record shown is the record that address names. It
  proves nothing about whether *that address* is the one the user was meant to
  receive. A reader who pasted a hostile address and saw a verified record has
  verified the attacker's record against the attacker's address, successfully.
  The second sentence is the load-bearing one. The spec's requirement "What the
  address proves is stated exactly, and nothing broader" pins the distinction and
  requires the unverified remainder to be named, rather than pinning the string.
- **`join.currentTitle`** — promises a value nothing resolves; see divergence 2
  above. Not pinned.
- **`stoaList.heldCount`** / **`stoaList.nothingHeld`** — the wording is correctly
  scoped ("received here" is about this machine, not a global total), so the
  README's rule 3 is not what rules them out. They are unavailable for the
  duller reason in divergence 3: no call computes the number. Not pinned.

The strings that survive unchanged are the ones making no such claim —
`stoaList.title`, `stoaList.subtitle`, `join.eyebrow`, `join.foundingTitle`,
`join.collision`, `join.notThisOne`, and the three button labels.

## Capabilities

### New Capabilities

- `stoa-navigation-view`: what the QML view must render, and must refuse to
  claim, when a user lists the Stoas this peer is in, previews one before
  joining, creates one, or shares one.

Named for the surface rather than for "the UI", deliberately. Every QML screen so
far — the feed, the identicon, the text sanitiser — landed with no spec at all, so
this is the first view capability in the suite and it sets the precedent for where
the next one goes. A capability called `ui` or `view` would become the drawer
every later screen is filed in, and a spec drawer is unreadable at the size this
one would reach. A later feed-view or thread-view capability gets its own name
beside this one.

**No core capability is modified.** Everything this change renders is already
contracted by `stoa-membership` (creation, joining, listing, and what a listed
title is), `module-wire-contract` (the envelope and the single error shape),
`stoa-metadata` (founding versus current) and `posting-capability` (the reason a
key is unusable). This spec cites those by requirement name and restates none of
them. Where it appears to repeat one it is stating a *rendering* obligation the
core requirement creates and cannot itself discharge.

### Modified Capabilities

None.

### One thing the view needs that the listing does not return

**`list_stoas` items carry `stoa` and `foundingTitle`, and not the genesis
record.** The core *retains* the record for every Stoa the peer is in —
`stoa-membership`'s "A joined Stoa's genesis record is retained, not only its
address" requires it, precisely so that moderation can be resolved later — but
the listing reply does not hand it back.

Two things on these screens need it, and neither can work around its absence,
because deriving a record from an address is exactly what a one-way hash
forbids:

- **Sharing a Stoa from the list.** What is shared has to carry both halves, so
  a row whose record the view does not hold cannot be shared from.
- **Opening a Stoa's feed from the list.** `FeedScreen` takes `stoaGenesis` and
  passes it to `list_threads`, because a reader may not resolve moderation for a
  Stoa whose record it does not hold.

This spec is written to be honest rather than to assume the gap closes: sharing is
required only where the record is held, and the absence of a share affordance is
specified as the correct rendering rather than an error. **That is a deliberately
degraded contract, and the better fix is in the core** — returning the retained
record on each listing item, which costs nothing the peer does not already have
on disk.

**This is reported, not made.** `stoa-membership` and the wire trait are core, and
this change edits neither — the listing shape was verified against merged `main`
(`list_stoas` returns `{"items":[{"stoa","foundingTitle"}],…}`), so this is a gap
in what is shipped rather than a comment on work in flight. Closing it is a core
piece of its own: widen the listing item to carry the retained record. If that
lands, the two requirements above lose their conditional half and the spec gets
simpler; until it does, the conditional is what keeps the spec buildable.

## Impact

- `dialectica-ui/src/qml/Core.qml` — three named wrappers added.
- `dialectica-ui/src/qml/Main.qml` — the developer-supplied Stoa properties go;
  navigation between the list and the feed arrives.
- `dialectica-ui/src/qml/` — new screens. The existing components (`Theme`,
  `Identicon`, `AddressLabel`, `FlatButton`, `ScreenFrame`, `MarginNote`,
  `ApparatusColumn`, `SanitisedText`) are reused as they are; nothing is
  restyled, and no second address abbreviation is written — `AddressLabel` owns
  the 8-8-6 form and the bundle forbids hand-rolling elision anywhere else.
- `dialectica-ui/tests/` — a new QML test target, in the shape
  `tst_feed_states.qml` established: the real screen driven through a fake
  bridge, so what is under test is the screen's state machine rather than a
  re-implementation of it.
- `docs/UI-BRIEF.md` — the *Stoa list* section's per-row material and the
  *Joining a Stoa* section need the resolutions above written back into them, so
  that the brief a designer works from stops describing a row that cannot be
  rendered. The brief is a live document and a stale one is designed against.
- **`AddressLabel.copyRequested()` is a signal nothing connects**, and this
  change is what gives it a receiver. There is no clipboard route anywhere in the
  view today, so copying is behaviour this change introduces rather than wires up.
- No core code changes. `dialectica/rust-lib/src/lib.rs`, `membership.rs`,
  `keystore.rs`, `stoa.rs` and `wire.rs` are untouched.
