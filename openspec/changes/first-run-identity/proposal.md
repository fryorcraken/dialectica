## Why

**A fresh profile cannot create a Stoa, and cannot obtain the key that would let
it.** Measured against this tree: on a fresh profile with no Stoas, pressing
"Create it" answers core's own *"no keystore found; create one before posting"*,
and no interface anywhere can create one.

It is a deadlock rather than a missing button, and both halves are contracted
deliberately:

- **Creating a Stoa requires a pre-existing key and mints none.**
  `dialectica/rust-lib/dialectica-core/src/wire.rs`'s `create_stoa` says so in as
  many words — *"Creation fails without a key rather than inventing one — There
  is no path from here to `Keystore::generate()`."*
- **The only line that writes the keystore file is `keystore.create(...)` inside
  `keep_selection`.** Its two callers, `generate_identity_slate` and
  `keep_identity`, both call `parse_stoa` first, which refuses a request naming
  no Stoa.

So creating a Stoa needs a key, minting a key needs a Stoa, and a fresh install
reaches neither. Every trait method that touches identity takes a Stoa; none can
mint a key without one.

**The specs are silent on it, which is why nothing caught it.**
`identity-onboarding` assumes a Stoa in hand throughout — every one of its
scenarios begins from one — and `stoa-genesis` requires that a genesis record
carry its creator's public key without saying where a peer's first key comes
from. Neither is wrong; between them there is a state no requirement describes,
and that state is every user's first thirty seconds.

## What Changes

- **A capability to mint this peer's master key, taking no Stoa.** That it takes
  no Stoa is the whole of what makes it the way out: a Stoa parameter is what
  every other method's deadlock is made of.
- **It is idempotent and never destructive.** Where a master key exists it is
  reported and never replaced. `identity-onboarding` already states the stake —
  *"A master key exists in exactly one place. Replacing it silently discards
  every identity derived from it"* — and this must not become a second way to do
  that. A repeat is a success rather than a refusal, because having a key is the
  expected state on every run after the first, and a step that errors on a repeat
  is a step every caller must guard.
- **It reports whether the key is protected at rest.** With no passphrase set,
  core stores the key in the clear; a user should learn that from the interface
  rather than from the file.
- **A first-run step in the view, before "Create it".** The owner's decision: the
  mint is an explicit step on the empty Stoa-list screen rather than something
  folded into `create_stoa`, which keeps `wire.rs`'s stated invariant intact
  rather than overturning it.

## What this is not

**It is not a change to per-Stoa onboarding.** The mint writes the master key
only and records no chosen path, so `generateIdentitySlate` and `keepIdentity`
are untouched and remain the only route to a per-Stoa choice.

That is a decision rather than a scoping convenience, and the alternative is worth
recording because it is the tempting one. Identity here is two layers: the
**master key** (`identity.key`, one per install, stoa-independent) and the
**chosen path** (`chosen_paths`, strictly per-Stoa). Creating a Stoa needs layer
1 alone; posting needs layer 2. So minting under a placeholder Stoa — which would
have reused `keepIdentity` and added no capability — would record a path for a
Stoa that does not exist, creation would then succeed, and **posting into the
real Stoa would still be refused**. That is a worse failure than the deadlock,
because it looks fixed.

**It is not a passphrase flow.** Reporting that a key is unencrypted is a
strictly smaller claim than offering a control that would protect it, and the
smaller claim is the one that is true today.

**It is not a replacement or recovery operation.** Replacing a master key
deliberately remains undefined by any capability, exactly as
`identity-onboarding` leaves it.

## Capabilities

### Modified Capabilities

- `identity-onboarding`: one requirement added, for the state the capability
  currently assumes away — a peer with no master key and no Stoa. Nothing
  existing changes: the per-Stoa slate, the keep, and the refusal to replace a
  stored identity are all untouched, and the new requirement explicitly does not
  record a per-Stoa choice, so the "keeping does not replace an existing one"
  requirement keeps its scope.

## Impact

- `dialectica/rust-lib/dialectica-core/src/wire.rs` — the handler and its
  decision function.
- `dialectica/rust-lib/src/lib.rs` — one trait method and its adapter forward.
  **A trait change has build-surface consequences**: the builder derives the
  `.lidl` from this trait, so the dispatch table must still derive.
- `dialectica-ui/src/qml/Core.qml` — one wrapper.
- `dialectica-ui/src/qml/DStoaListScreen.qml` — the first-run step.
- **The view's copy for this step may not use the word "identity"**, which is a
  constraint discovered rather than chosen: `tst_stoa_screens.qml` bans it on this
  screen because one key signs in every Stoa in this release, so raising identity
  here would offer an unlinkability property the software does not have. The
  honest word is "this machine's key", which is also what the thing actually is.
