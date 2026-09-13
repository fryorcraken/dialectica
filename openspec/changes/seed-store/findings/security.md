# Security findings: the store seeder

Reviewed `dialectica/rust-lib/dialectica-core/examples/seed_store.rs` at
`04da8c8`. This tool mints a real Ed25519 root secret and writes it to a
directory the user names on the command line, so the review concentrated on what
happens to that secret and on what `--fresh` is capable of destroying.

Everything below was reproduced by running the program against scratch
directories under the worktree's gitignored `tmp/`.

---

- [x] **`dev-writer`** — `seed_store.rs:255` — the seeder writes a root secret
      into a world-writable directory without complaint, producing a keystore the
      module itself then refuses to open
      **Scenario:** `std::fs::create_dir_all(&dir)` creates the directory under
      the process umask and nothing checks its mode; `Keystore::create`
      (`keystore.rs:934-939`) checks only that the *file* does not already exist
      and delegates to `write_to`, which does not check the containing directory
      either. The directory guard lives only on the read path, in `read_checked`
      (`keystore.rs:1235-1247` calling `check_directory_mode` at `:1322-1324`,
      refusing `mode & 0o022 != 0`).
      Reproduced end to end:
      ```
      mkdir -m 777 tmp/openperm
      cargo run --example seed_store -- tmp/openperm      # succeeds, prints a full report
      keystore::open_in("tmp/openperm")
        -> "the keystore's directory is writable by others (mode 0777);
            restrict it to owner-only (chmod 700) — the key can be replaced
            there whatever its own permissions say"
      ```
      Two distinct harms. First, a **real root secret sits in a directory any
      local user can write**, and the keystore's own doc comment
      (`keystore.rs:1300-1304`) states the threat precisely: "a *writable*
      directory lets someone replace the keystore, delete it, or plant something
      at a name a write will touch, none of which the file's own 0600 prevents."
      The seeder set the file to 0600 (verified: `-rw-------`) and the directory
      defeats it. Second, **every assertion in the program passes** and the report
      prints in full, so the tool reports success over a store the module will
      refuse — which is precisely the "a caller would trust a store that does not
      work, and would go looking for the bug in the UI" failure the read-back
      section at `:454-459` exists to prevent.
      **The fix is to ask the keystore rather than to re-implement the check:**
      after writing, call `keystore::open_in(&dir)` and fail on its error. That
      reuses the guard already written and tested, costs one call, and converts a
      silent bad store into a named refusal. **Severity: high** — a root secret
      written to a location the codebase's own guard classifies as unsafe, with
      no diagnostic.

      **FIXED**, taking the suggested fix unchanged: `keystore::open_in(&dir)`
      immediately after `create`, failing on its error.

      Reproduced before and after. `mkdir -m 777 tmp/openperm` then seeding it
      previously printed a full success report; it now exits 1 with

      ```
      Error: "the keystore this wrote is not one the module can open: the
      keystore's directory is writable by others (mode 0777); restrict it to
      owner-only (chmod 700) — the key can be replaced there whatever its own
      permissions say"
      ```

      **Asking the keystore rather than re-implementing the check is the whole
      point of the fix and is now stated at the call site.** A mode test written
      in the example would be a second copy of a guard that already exists and is
      already tested, and the copy no test covers is the one that drifts — the
      two would eventually disagree about which bits matter. `open_in` is
      literally the call the adapter makes, so what the seeder accepts is what the
      module accepts, by construction rather than by agreement.

      It also closes the second harm you name, which was the more insidious one:
      the tool can no longer report success over a store the module will refuse.
      That was the exact failure the read-back section exists to prevent, reached
      by a route that section did not cover — it checked the *content* of the
      store and not whether the store was *openable*.

- [x] **`dev-writer`** — `seed_store.rs:297-302` — the keystore is written
      unencrypted by default and the run says nothing about it
      **Scenario:** `keystore::protection_from_env()`
      (`keystore.rs:365-370`) returns `Unlock::Unencrypted` whenever
      `DIALECTICA_PASSPHRASE` is unset or empty, so the default run stores a
      32-byte Ed25519 root secret in the clear. Verified: `identity.key` is 35
      bytes — a 3-byte header (`MAGIC`, `VERSION_1`, `Protection::None`) plus the
      raw root, per `to_file_bytes` at `keystore.rs:953-959`.
      **The choice of default is correct and is not the finding** — it is
      `protection_from_env`'s own documented contract, deliberately inherited
      rather than reinvented here, and `keystore.rs:350-355` explains why a
      fallback constant would be worse. The finding is that **the run output never
      says which of the two happened.** `keepIdentity`'s reply reports protection
      so that "an unencrypted keystore is a state a view can name rather than a
      silent default" (`lib.rs:731-736`); this tool prints eight labelled values
      and omits that one. A developer seeding a laptop cannot tell from the
      scrollback whether a plaintext root secret was just written.
      One line in the report — `protection  unencrypted (set DIALECTICA_PASSPHRASE
      to encrypt)` — closes it. **Severity: medium.**

      **FIXED**, essentially as worded. The report now carries:

      ```
      protection UNENCRYPTED — the root secret is in the clear (set DIALECTICA_PASSPHRASE to encrypt)
      ```

      and `encrypted with DIALECTICA_PASSPHRASE` in the other case.

      One implementation detail worth recording, because the obvious version is
      subtly wrong: the line is produced by matching on the `Unlock` value that was
      **actually used to write the file**, which is now bound to a variable and
      passed to `create`. Re-reading the environment at print time would be a
      second `var_os` call that could disagree with the first — a narrow window,
      but the failure would be a report claiming encryption over a plaintext file,
      which is the one direction that must not happen.

      Your framing that the default is correct and is not the finding was right and
      is preserved: `protection_from_env` remains the source of the decision, and
      nothing here second-guesses it.

- [x] **`dev-writer`** — `seed_store.rs:262-288` — `--fresh` deletes by name
      without confirming the directory is a dialectica store, so a mistyped path
      destroys four arbitrary files
      **Scenario:** the deletion loop removes any existing file at the four
      derived paths. Three of the four names are generic enough to collide with
      unrelated data: `identity.key`, `identity.sqlite`, `stoas.sqlite`,
      `ops.sqlite`. Reproduced: a directory containing only a hand-made
      `ops.sqlite` (not a dialectica op log, just a touched file) was deleted
      without a word beyond `--fresh: deleting op log …`, and the seeder then
      wrote a fresh store over it. Nothing checks that a file at a derived path is
      the thing the name claims — e.g. that `identity.key` starts with the
      keystore `MAGIC` byte.
      The preamble's defence is that deletion is opt-in and each file is named as
      it goes, which is genuine mitigation and is why this is not rated higher.
      But "a person who passed `--fresh` by mistake can see in the scrollback
      exactly what they lost" (`:282-283`) is after-the-fact: the lines print as
      the deletions happen, not before. **Checking the keystore magic before
      deleting `identity.key`** — the one file whose loss is unrecoverable —
      would cost a four-byte read and would refuse the case that actually hurts.
      **Severity: medium.**

      **FIXED**, with one deliberate substitution in the mechanism.

      A guard now runs **before the first deletion**, and only over `identity.key`
      — your reasoning that it is the one file whose loss is unrecoverable is the
      reason the other three are not checked; they hold rebuildable content.

      **The check is `Keystore::is_encrypted`, not a magic-byte comparison.**
      `MAGIC` is `const`, private to `keystore.rs` (`:81`), so a byte check here
      would mean writing `0xD4` in the example — a second copy of a format constant
      that no test covers, which is this repo's recorded defect family and exactly
      the shape the `ops.sqlite` comment in this file already apologises for.
      `is_encrypted` is public, parses the real header through `parse_header`, and
      returns `NotAKeystore` for anything else. Same protection, nothing copied.

      Reproduced both ways. A directory holding a hand-written `identity.key` at
      0600:

      ```
      Error: "refusing --fresh: identity.key is not a keystore, so this is
      probably not a dialectica store and nothing was deleted: that file is not a
      dialectica keystore; check the path"
      ```

      — and the file was read back afterwards, intact. A real store still deletes
      and reseeds normally.

      One limitation, stated rather than left to be discovered: because
      `is_encrypted` goes through `read_checked`, it checks permissions *before*
      parsing, so a non-keystore file with loose permissions is refused with a
      permissions message rather than a "not a keystore" one. The refusal is
      correct and nothing is deleted either way; only the wording is less direct in
      that case.

- [x] **`dev-writer`** — `seed_store.rs:361` — `stoa_key` is documented as
      called by no handler, so "matching the module" pins a derivation the MVP
      does not use
      **Scenario:** the seeder signs with `keystore.stoa_key(&address)`, which
      matches `lib.rs:539` exactly — the claim in `design.md` is true as written.
      But `Keystore::stoa_key`'s own doc comment
      (`keystore.rs:793-797`) says it is "**Built and, in the MVP, not called by
      any handler** — see `Keystore::identity_key`, which is the key the module
      actually signs and creates with today", and `identity_key`'s comment
      (`keystore.rs:813-826`) says "**there is exactly one derivation position, so
      a creator and a poster cannot be two keys.**"
      These two statements contradict the adapter, which does call `stoa_key`.
      The seeder has faithfully copied the adapter, so it is not wrong — but it
      has thereby **frozen the contradiction into a tool whose output developers
      will treat as ground truth**, and its own inequality assertion at `:511-516`
      now guarantees the program fails if anyone resolves it in the direction
      `keystore.rs` says is already true. Whoever fixes the three-derivations gap
      will hit this assertion and must delete it; that is by design and is handled
      well. What is missing is a pointer **from** this file to the two docstrings
      that disagree with the adapter, so the next reader does not conclude the
      seeder invented the divergence. **Severity: low — documentation**, but it
      bears directly on the moderation defect in `correctness.md`.

      **FIXED** as asked — the module docstring now carries the pointer, naming
      both docstrings, quoting what each claims, stating plainly that the adapter
      does call `stoa_key` so both sentences are false of the code as it stands,
      and saying why this file follows the adapter anyway. It ends by directing
      whoever resolves the gap to fix those two docstrings in the same change, and
      cites this entry.

      **The two docstrings are NOT edited here, and that is a deliberate
      boundary.** They are in `keystore.rs`, a file this piece otherwise does not
      touch; correcting them is a behaviour-free edit to a core module that would
      turn a tool piece into a change that also edits the crate's documentation,
      and the coordinator's instruction on this piece is not to widen it. The
      entry's own severity — low, documentation — supports leaving it as a
      pointer.

      Your reading of the interaction with the inequality assertion is right and is
      why this is not merely cosmetic: that assertion guarantees the program fails
      if anyone resolves the derivations in the direction `keystore.rs` already
      claims is true. Someone hitting it needs to know, at that moment, that the
      docstrings and the adapter disagreed *before* this tool existed. The pointer
      is what tells them.

## What was clean

**No secret reaches stdout.** The report prints addresses and public keys only —
`address.to_hex()`, `genesis_hex`, `posting_address`, `signing_address`, five op
ids and the Stoa title. The `visitor` secret is generated, used and dropped;
neither it nor the root is formatted anywhere. The `why` helper wraps only
`Display` of this crate's error types, and `KeystoreError`'s no-secret-material
property is itself covered by a test at `keystore.rs:2193-2200`.

**File permissions on the keystore are correct**: `identity.key` came out
`-rw-------` (0600), which `check_mode` (`keystore.rs:1292-1294`) accepts. The
three SQLite files are `-rw-r--r--`, which matches what the module's own stores
produce and holds no secret material.

**`--fresh` cannot escape the named files.** It iterates a fixed four-element
list derived from `core` path functions, calls `remove_file` (never
`remove_dir_all`), and takes no glob or recursion. Pointed at a directory where
`identity.key` was a symlink into another directory, it unlinked the symlink and
left the target file byte-identical — checked by reading the target back
afterwards. `design.md`'s "`--fresh` does not remove the directory" claim is
accurate.

**The refusal path is ordered correctly** and cannot be raced into a partial
write by the program itself: the existence check at `:262-265` completes before
`Keystore::generate`, and `Keystore::create` independently refuses an existing
file (`keystore.rs:935-937`), so the "half-seeded directory is not a state this
program can produce" claim holds for the non-`--fresh` case. A TOCTOU window
exists between the check and the write, but the attacker who could exploit it is
a local user who can already write the directory, which is the finding above.

**No peer input is involved.** The tool reads no network data and parses no
untrusted bytes; every value it writes it computed. The standing "never trust an
inbound message" rule has no surface here, and I found no indexing, slicing or
arithmetic reachable from anything an attacker supplies. The one `expect`
(`:494`) is on a value this program wrote a few lines earlier.

**No new dependency**, so no licence or supply-chain question: `hex` is already
`dialectica-core/Cargo.toml:110`, and the argument parser is hand-rolled
specifically to avoid adding a flag crate.
