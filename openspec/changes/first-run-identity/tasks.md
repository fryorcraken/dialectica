# Tasks

## 1. Core: the mint

- [x] 1.1 `Minted` reply type and `to_json`, one success shape rather than two
  exclusive ones — this operation cannot answer no and still succeed.
- [x] 1.2 `mint_master_key`, with the `exists()` branch FIRST so nothing is
  generated when a key is already on disk.
- [x] 1.3 `create_identity` handler: the panic guard, `Request::parse`, the
  decision, the reply.
- [x] 1.4 Re-export from `dialectica-core/src/lib.rs`.

## 2. Core: the tests

- [x] 2.1 A first run writes a key and reports it as new, asserted against the
  file rather than against the reply alone.
- [x] 2.2 **The regression test for the destructive case**: a second call
  replaces nothing, reports `wasNew:false`, and leaves the file's BYTES
  unchanged. Proved to fail with the `exists()` guard removed — the reply becomes
  `{"error":"a keystore already exists at that path…"}`.
- [x] 2.3 Protection is reported in both directions, and for an existing key is
  read off the file. Proved to fail with the file read replaced by the `unlock`
  argument.
- [x] 2.4 No per-Stoa choice is recorded.
- [x] 2.5 The deadlock's exit, end to end: mint, then `creator_key_in` resolves,
  then `create_stoa` succeeds naming that key — paired with a test that creation
  is still refused BEFORE a mint, so the pair distinguishes the fix from a
  `create_stoa` that never needed a key.
- [x] 2.6 Registered in `every_request_taking_method` so the envelope sweeps
  cover it.
- [x] 2.7 `NO_REQUIRED_FIELD` reshaped from a one-name filter into a named set
  with reasons, plus a sweep proving `{}` is SERVED for each member — the old
  shape asserted that half on `list_stoas` by name, so a second exempt method
  would have inherited no coverage.

## 3. The contract

- [x] 3.1 Trait method on `DialecticaModule`, documented with what it takes, what
  it never does, and why it takes no Stoa.
- [x] 3.2 Adapter forward, supplying the two host-derived values the way
  `keep_identity` does.
- [x] 3.3 `lgs basecamp build --variant lgx` — prove the dispatch table still
  derives from the changed trait. Both `.lgx` artefacts built.

## 4. The view

- [x] 4.1 `Core.createIdentity()` wrapper.
- [x] 4.2 The first-run step on `DStoaListScreen`, above "Create a Stoa",
  rendered in EVERY read state — a peer whose membership failed to read may still
  have no key.
- [x] 4.3 The unencrypted warning, shown only when the reply reports it.
- [x] 4.4 Copy avoids the word "identity" on this screen, per the existing ban.

## 5. The view's tests

- [x] 5.1 The step is offered before any Stoa exists, and also when the listing
  failed.
- [x] 5.2 Nothing is probed or minted before the user presses.
- [x] 5.3 New and already-present both render, and are distinguishable.
- [x] 5.4 The unencrypted warning appears and — the other direction — does not
  appear for a protected key.
- [x] 5.5 A refusal renders core's reason and claims no key.
- [x] 5.6 A success naming no key is treated as a failure.
- [x] 5.7 The request names no Stoa.

## 6. Proof by launching

- [x] 6.1 Rebuilt, `lgs basecamp install`, `lgs basecamp launch alice` on a
  profile verified to hold no `identity.key` and no store beforehand.
- [x] 6.2 **The deadlock is closed, proved against the profile rather than
  against a reply.** `module_data/dialectica/40c5423a292f/` now holds
  `identity.key` (35 bytes, mode 0600, protection byte `00` — unencrypted, which
  is what a first run with no passphrase yields) and `stoas.sqlite` with **one
  row**. A Stoa exists on a profile that could not previously make one.

  **The two keys are the same key**, which is the claim that matters and is
  asserted as a relation rather than against a literal: the genesis record's
  creator is `8fb955ab…a42339`, and opening the minted keystore and taking
  `identity_public_key()` yields `8fb955ab…a42339`. So the key the mint wrote is
  the key `create_stoa` named as creator.

  **`chosen_paths` holds 0 rows**, confirming under a real launch what the unit
  test asserts: the mint writes layer 1 and records no per-Stoa choice, so the
  placeholder-Stoa shortcut's failure mode was avoided rather than merely
  argued.

  The log shows the plugin healthy: `DStoaListScreen.qml` resolved 65 times and
  `DTheme.qml` 72 times into the plugin's own directory, with **zero**
  `ReferenceError`, `Unable to assign`, `is not a type` or `MODULE_NOT_LOADED`
  lines, and basecamp exited 0.
