# Choosing an identity: the onboarding slate, and the metadata it creates

## Why

**The keystore can store a root secret and nothing creates one.** `keystore.rs`
has `generate()` and `create()`; no wire method calls either. So a fresh install
has no identity, `getCapabilities` answers `canPost: false` with a reason naming
a fix the user has no way to perform, and every posting affordance is correctly
but permanently disabled. This is the root of the MVP's dependency tree: nothing
can be signed, no Stoa created and no post authored until it is closed.

`identity` and `keystore` already contract what an identity *is* and how a root
is *stored*. Neither says how a user comes to **have** one, and that gap is this
change.

## What Changes

- A new `identity-onboarding` capability: a slate of candidate identities, a way
  to keep one, and a way to ask who the user currently is.
- **A user chooses among derivation paths over one master key**, not among
  several master keys. The master key is generated locally and is exportable;
  what onboarding selects is which path from it becomes this user's identity.
- **That choice is metadata that must be stored**, because it cannot be
  recomputed. This is the substantive new obligation in this change and the one
  worth reviewing hardest — see Decisions below and the spec's requirement
  "A chosen derivation path is recorded, because it cannot be recomputed".
- `getCapabilities` gains no new shape and **its derivation is unchanged**; it
  begins answering `canPost: true` because a keystore now exists.

## Capabilities

**New Capabilities**

- `identity-onboarding` — how a user acquires an identity: what a slate is, what
  keeping one guarantees, what is recorded, and what the reply may and may not
  carry.

**Modified Capabilities**

- `identity` — one requirement is MODIFIED. Per-Stoa derivation currently takes
  the root and the Stoa address and forbids any third input, in order to keep an
  identity recomputable from those two alone. A user-chosen derivation path is
  exactly such a third input, so the requirement is restated to admit it and to
  move the recomputability claim onto the recorded path. **This is a narrowing of
  a guarantee and is called out rather than absorbed.**

**Not modified, deliberately**

- `keystore` — unchanged. What is stored is still a single root secret from which
  per-Stoa identities are derived, and this change stores no per-Stoa key. The
  path metadata is not keystore content: it is not secret, it is not required to
  be encrypted, and putting it in the keystore file would widen a format whose
  every field is currently accounted for by a test asserting no room for
  anything else.
- `posting-capability` — unchanged. The probe's shape, its reasons and its
  derivation are untouched. A keystore existing is a state it already describes.

## Decisions

### The identity is chosen by derivation path, not by minting several roots

The owner's decision, in their words:

> *"I do want a master key and HD model so that a user can save one
> key/mnemonic, and then derive his identities from there"* — and the identity
> choice is *"about proposing several derivation path (1, 2, 3, etc)"*.

The alternative — mint five independent root secrets and keep one — was what a
first implementation did, and it is worse for a reason that has nothing to do
with the slate: it gives the user five things to back up instead of one, and
four of them are discarded. One master key plus a path index is one secret to
save.

### The chosen path is metadata, and that is a real cost

`derive_stoa_key` today takes `(root, stoa)` and the `identity` spec forbids a
third input, so an identity is recomputable from the root and the Stoa address
alone — nothing needs backing up beyond the root and the list of Stoas joined,
which is needed to read the forum anyway.

**A user-chosen path breaks that**, and the owner named the consequence:

> *"It does mean there is some metadata to take in account (what derivation path
> did the user choose for a given stoa); I think at first we can just have the
> metadata on store; later we wil want to be able to export it as a file or back
> it up on Logos storage so that as long as storage has the data + master key is
> preserved, user can reclaim their identity"*

So: local store now, export and remote backup later. The honest statement of the
interim, which the spec requires the interface to be able to make, is that
**until the backup path exists, losing the local store loses the identities even
with the master key preserved.** That is a deliberate trade for letting the user
choose, not an oversight.

### The derivation salt is versioned, and a third input should bump it

`derive_stoa_key`'s HKDF salt carries a scheme version. Adding a path input
without bumping it would make path 0 silently produce the same key as every
identity derived under the two-input scheme. Making path 0 equivalent may well be
wanted — it would keep existing identities valid — but it must be a decision
somebody took, not a collision. The spec requires the schemes be distinguishable;
which way to resolve it is `design.md`'s.

### Matching LEZ, and what that forecloses

The owner wants the key model to match LEZ so derivation and signing can move to
an external wallet, core holding only public keys and signatures, with an
ephemeral core-held key for delegation. An investigation of the LEZ and Medusa
source establishes three facts that bear on this spec:

- **LEZ already has the API.** `lez/keycard_wallet/src/lib.rs` exposes
  `get_public_key_for_path(path)` at `:173` and
  `sign_message_for_path(path, message)` at `:209` — caller-supplied BIP-32 path
  string, private key never leaving the card.
- **It is an in-process Rust API over PC/SC, not IPC or RPC**, and the C ABI in
  `lez/wallet-ffi` is keyed by account id rather than by path. So a path-based
  request from dialectica needs a transport that does not exist yet.
- **Public derivation from a parent public key is unavailable by design.** LEZ's
  key tree is hardened-only, and `lee/key_protocol/.../keys_public.rs:68-71`
  gives the reason: non-hardened keys would need untweaked public keys and are
  *"not PQ secure"*. Viewing keys are ML-KEM-768.

The third is the one with a spec consequence: **every candidate public key
requires the secret**, so a slate cannot be computed offline or from a
watch-only export. The spec therefore states what a slate must contain without
requiring it be derivable without the master key.

Notably dialectica's own `derive_stoa_key` refuses public derivation too, for an
unrelated reason — cross-Stoa unlinkability. Both schemes decline the same
capability on different grounds, so the wallet direction does not cost a property
dialectica had.

**Nothing in this change implements the wallet path.** It is recorded because a
spec written as though core will always hold the secret would have to be rewritten
rather than extended.

### Four questions that were deliberately not settled by the implementer

These were raised as things to "argue in the PR", which would have made them
decisions taken by whoever wrote the code first. Three are settled here; one is
recorded as unsettled because nobody has decided it.

- **Does the secret leave core?** Settled: **no**. A slate carries public keys
  and addresses. Nothing on the screen needs a secret, the view cannot sign, and
  widening later is additive while a secret that has crossed the boundary cannot
  be recalled. Under the wallet model core may hold no secret at all, which this
  does not obstruct.
- **What happens to unkept candidates?** Settled: **they are discarded and their
  secret material cleared**. Under the derivation-path model there are no unkept
  *secrets* to begin with — one master key, several paths — which is a second
  reason to prefer that model. `keystore` already requires secret material be
  cleared on drop; the spec extends that to slate material rather than restating
  it.
- **Where does the slate live between generating and keeping?** Settled: **in
  memory, and nothing is written until a candidate is kept**. PLAN.md leaves this
  open explicitly; writing only on selection is chosen because it never persists
  a choice the user did not make. Losing a slate costs a click, because nothing
  was published.
- **Is a passphrase required?** **UNSETTLED, and recorded as such.** The keystore
  is Argon2id over a passphrase and the MVP has no passphrase UI. A fixed
  development passphrase is refused outright — it would be shared by every
  install and known to every reader of the source, while the file recorded itself
  as encrypted, which is protection that reads as strong. The spec requires only
  that whichever protection is applied be **recorded in the file and reportable
  to the caller**, so an unencrypted keystore is a state the interface can name
  rather than a silent default. Choosing between "prompt", "environment variable
  only" and "unencrypted with a visible warning" is a product decision and is
  out of scope here.

### A correction this proposal exists partly to record

An instruction given during implementation said that one-identity-per-user meant
**not** calling `derive_stoa_key`. That was wrong, and building on it would have
changed every author address on the peer:

- `dialectica/rust-lib/src/lib.rs:251` — `get_capabilities`, a shipped method —
  calls `open_from_env(&path).map(|ks| ks.stoa_address(stoa).to_hex())`
- `keystore.rs:651` `stoa_address` → `:645` `stoa_public_key` → `:640`
  `stoa_key` → `:641` `derive_stoa_key(&self.root, stoa)`

Two merged specs require it: `keystore` ("a single root secret from which every
per-Stoa identity is derived", "per-Stoa keys are derived from it rather than
stored separately") and `identity` (a per-Stoa key's public key differs from the
root used directly).

**Per-Stoa derivation is on the live path and stays there.** This change adds a
path input to it; it does not bypass it. "One identity per user" is read as one
master key and one keystore with no identity-selection flow beyond this one —
which is what the code already does — and not as signing with the root.

## Impact

- New `openspec/specs/identity-onboarding/spec.md` on archive.
- `openspec/specs/identity/spec.md` — one MODIFIED requirement, replacing the
  whole block including its scenarios.
- New wire methods on the module contract. Naming and shapes are the spec's
  concern only where behaviour depends on them; the method list is `design.md`'s.
- A store for the path metadata. Whether that is the existing SQLite projection
  or a separate file is `design.md`'s; the spec constrains only that it survives
  a restart and can be exported later.
- No change to the keystore file format, to the op format, or to the UI module.
