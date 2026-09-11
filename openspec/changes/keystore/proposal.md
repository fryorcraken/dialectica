# The keystore, and the capability probe that gates posting on it

## Why

`identity.rs` can mint, derive, sign and verify — and has nowhere to put a key.
Its own module doc says so: *"No keystore. §5.6 specifies one... this module
defines the key types that a keystore will hand back."* Nothing today persists a
root secret, so every restart is a new identity, which is exactly the failure
§5.4 rejects the chat module for (*"restarting mints a fresh identity"*).

Two things make this the change to be most careful in.

**It is the only code in the project that writes secret material to disk.** The
reference §5.6 names is deliberate about the bar: *"LEZ's own keystore is
plaintext JSON at 0644 containing every secret, with `// TODO: Use password for
storage encryption`. Match the crypto, not the key handling."*

**The module can never ask the user anything.** A Logos module is a library
inside a sandboxed host with no terminal and no stdin. A keystore designed
around a passphrase prompt does not degrade gracefully here — it hangs, and
PHASE0-FINDINGS §2 measured what a hang costs the caller: a 20-second timeout
and a message pointing at a slow provider. Every unlock path must therefore be
non-interactive, and a locked keystore must be a *returned error naming the
fix*, not a wait.

The probe is the second half of the same problem, and §5.6 states the cost of
getting it wrong: *"A compose box that cannot be submitted loses whatever the
user typed."* A view must be able to ask "can I post?" and get a truthful answer
**before** it renders an affordance.

## What Changes

- A `keystore` capability: a root secret encrypted at rest at a fixed path,
  derived from a passphrase, with an authenticated cipher so a tampered file is
  refused rather than decrypted to garbage.
- A **canonical keystore file format** with a version discriminant and strict
  decoding — same discipline as the genesis record, applied to a file that
  arrives from the filesystem rather than from a peer.
- Two non-interactive unlock paths: an **unencrypted** keystore (the empty
  passphrase, which is a recorded choice rather than an accident) and a
  **passphrase supplied by environment variable**. An agent path is deliberately
  deferred; see design.md.
- **Refusal on over-permissive file modes.** A keystore readable by anyone else
  on the machine is refused, not warned about.
- Atomic writes, so an interrupted save cannot destroy an existing key.
- `getCapabilities()` on the wire surface, returning
  `{"canPost":true,"identity":"…"}` or `{"canPost":false,"reason":"…"}` — where
  the reason **names the fix**.

## Capabilities

**New Capabilities**

- `keystore` — how a root secret is stored, what unlocks it, what is refused,
  and what a caller may learn about the result.
- `posting-capability` — the probe a view gates every posting affordance on:
  what it answers in each state, and the guarantees its answer carries.

**Modified Capabilities**

None. No existing spec describes key storage or the wire surface.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/keystore.rs`, registered in
  `lib.rs`.
- `wire.rs` gains `get_capabilities`, and `lib.rs` re-exports it — the first
  widening of the wire contract since Phase 0, and deliberate per §2.5.
- `identity.rs` is not edited. The keystore consumes `SecretKey::to_bytes` and
  `SecretKey::from_bytes`, which already exist for exactly this.
- Two new dependencies (`chacha20poly1305`, `argon2`), justified in design.md.
  `zeroize` and `subtle` are already in the tree transitively and become direct.
- No SDK types. The keystore takes its path as an argument; nothing reads the
  environment or the filesystem at module-init time.
- No store, no op log, no new op kind, no UI change.
