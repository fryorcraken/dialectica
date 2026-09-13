# Correctness findings: the store seeder

Reviewed `dialectica/rust-lib/dialectica-core/examples/seed_store.rs` at
`04da8c8`, against a worktree of `piece/seed-store`. Every claim below was
produced by running the program, not by reading it.

**The tool works.** `cargo run --example seed_store -- <dir>` writes the four
files, and the printed request was fed to `wire::list_threads_from_request`
against the seeded directory and returned the two-row feed — the `design.md`
claim reproduced independently rather than taken on trust. The refusal path, the
`--fresh` path, `--fresh` placed *after* the directory argument, and a partial
store holding only `ops.sqlite` were each exercised and each behaved as
documented. Both hardcoded assertions were measured discriminating: dropping one
vote fails `ops == 9` (`left: 8, right: 9`), and pointing the author check at
`posting_address` fails it, which is the author's own measurement reproduced.

`cargo test -p dialectica -p dialectica-core` is green in this worktree (733 + 26
= 759 passed, 0 failed), `cargo clippy --all-targets -- -D warnings` is clean for
both our packages, and `rustfmt --check` on the example itself is clean.

---

- [ ] **`dev-writer`** — `seed_store.rs:486-489` — the moderator assertion is
      tautological, and the property it claims to check is FALSE for every store
      this tool writes
      **Scenario:** the assertion is
      `moderators.contains(&genesis.creator)`. `Moderators::of` is
      `moderation.rs:171-176`, which sets `creator: genesis.creator.clone()`, and
      `contains` is `moderation.rs:190-192`, `&self.creator == key`. So the
      assertion reduces to `genesis.creator == genesis.creator` — it holds for
      *any* value whatsoever in that field, including a key from a keystore that
      was never minted. It is the "asks the implementation what it did and
      agrees" variant of this repo's recorded defect family.
      The comment above it claims it "catches the derivation trap head-on … if
      those two ever have to be one key, this fails here rather than as a hide
      button that silently does nothing." **The hide button silently does
      nothing today, and this assertion passes.** Measured against a seeded
      store: the adapter signs every publish with `keystore.stoa_key(&stoa)`
      (`dialectica/rust-lib/src/lib.rs:539`), and `Moderators::authorises`
      (`moderation.rs:206-215`) gates on `self.contains(&entry.op.op.author)` —
      the *signing* author. For the store seeded at
      `dd51b292…`:
      ```
      genesis creator (pubkey addr) 7813a72060fd6564bdd70c8cdf46e4ed91e91df13e30cc9711fd60ae73d970c4
      moderators.contains(creator)  true
      adapter signing pubkey        7b2e29e6420c5435a9e850f9e4ddc68a528ca97112c570e5b41f162c046f2c05
      moderators.contains(signer)   false
      ```
      A moderation op published through the module against a seeded Stoa is
      therefore refused, which is exactly the outcome `design.md`'s "It writes a
      real keystore" section says the real keystore was adopted to prevent. The
      assertion that was supposed to notice cannot, because it compares the
      record to itself.
      **The fix is one line** — assert `moderators.contains(&founder.public_key())`
      (the key ops are actually signed with) instead of, or as well as,
      `&genesis.creator`. That assertion FAILS today, which is the point: it
      surfaces a real gap the tool currently hides. **Severity: high** — this is
      the one assertion whose stated job is to catch a moderation defect, and it
      is structurally unable to.

- [ ] **`dev-writer`** — `seed_store.rs:471-476` — `feed.items.len() == 2` and
      `ops == 9` cannot distinguish a correct thread tree from a broken one
      **Scenario:** the file comment at `:392-396` argues two levels of nesting
      exist so that "a correct `thread` field and a `thread: parent` bug give the
      same answer" is avoided. But nothing asserts on the nesting. The two
      assertions are a count of thread heads (2) and a count of ops (9), and both
      are satisfied by a store in which `nested` was attached to the wrong
      parent, or in which all three replies hang off `first_root`. Verified by
      reading the feed back: `list_threads` returns only thread *heads*, so the
      reply structure is never observed by any assertion. The five op ids are
      printed in an indented tree, which is presentation, not a check.
      The tool's stated purpose is to produce a store whose *nesting* a UI can
      render. **Add an assertion over
      `dialectica_core::feed`'s thread read** (the function `listThread` calls)
      confirming `reply.id`'s parent is `first_root.id` and `nested.id`'s parent
      is `reply.id`. **Severity: medium** — the structure the tool exists to
      produce is the one thing no assertion covers.

- [ ] **`dev-writer`** — `seed_store.rs:506-510` — `feed.items[0].author` is the
      same value as `feed.items[1].author`, so the author assertion is weaker
      than it reads
      **Scenario:** both roots are published by `founder`
      (`:373-390`), so every thread head in the feed carries one author. The
      assertion indexes `items[0]` and the comment at `:498-500` says the row
      "comes back out of the store", implying the choice of index is meaningful.
      It is not: `items[1]` would assert the identical thing. Measured — the
      feed read returns
      `"author":"30b73893…"` for both rows.
      The `visitor` key signs only replies and votes, none of which appear in
      `list_threads` output, so **the two-identity property the module docstring
      advertises at `:12-14` ("from two identities so that author attribution is
      visible rather than uniform") is not visible in the feed at all** — only in
      a thread read, which nothing here performs. Either publish one root as the
      visitor, or assert over a thread read where both authors appear.
      **Severity: medium** — a fixture where two explanations give the same
      answer, and a docstring claim the output does not support.

- [ ] **`dev-writer`** — `seed_store.rs:255` + `:262-288` — `--fresh` destroys
      the store BEFORE any assertion can fail, so a failed run leaves nothing
      **Scenario:** `create_dir_all` and the deletion loop run at `:255` and
      `:281-286`; every `assert!` is at `:471-516`, after all nine ops are
      written. Reproduced by dropping one vote and running
      `--fresh` against a populated directory: the four deletions print, the
      program then panics on `ops == 9`, and the directory is left holding a
      half-seeded store with no keystore the user had before. The refusal path's
      own error text promises "Nothing was written" for the non-`--fresh` case;
      the `--fresh` case offers no equivalent guarantee and the docs do not say
      so. **Severity: low** — `--fresh` is opt-in and the preamble is candid
      about deletion, but "the assertions make a bad run fail loudly" (`design.md`,
      "No CI change") is true only in the sense that it fails loudly *after*
      deleting. Worth one sentence in the `--fresh` help text.

- [ ] **`dev-writer`** — `proposal.md:37` — names `identities.sqlite`, but the
      file the tool writes is `identity.sqlite`
      **Scenario:** `IdentityStore::default_path_in`
      (`identity_store.rs:211-213`) is `dir.join("identity.sqlite")`, and a run
      produces `identity.sqlite`. `proposal.md` line 37 writes
      "an identity record (`identities.sqlite`)". The example itself is correct —
      it derives the name through the `core` function rather than spelling it —
      so this is a documentation defect only. It matters because a reader
      checking the seeded directory against the proposal will not find the file
      the proposal names. **Severity: low.**

- [ ] **`dev-writer`** — `seed_store.rs:493-494` — an `expect` on a path this
      program has just written, where the surrounding code returns `Result`
      **Scenario:** `paths.path_for(&address)` is unwrapped with
      `.expect("the path just recorded must read back")`. Every other fallible
      call in `main` goes through `why(...)` and returns a `String` error. The
      `expect` is almost certainly unreachable, which is the argument for it —
      but it is the one line that aborts with a backtrace rather than a one-line
      message, and it sits directly after a `--fresh` deletion. Converting it to
      `why("reading the chosen path back", …)?` plus an explicit
      `ok_or_else(|| "…".to_string())?` costs two lines and makes the failure
      mode uniform. **Severity: low — stylistic**, noted because the file's own
      `why` helper exists precisely to avoid this shape.

## What was clean

The argument parser handles `--fresh` before and after the path, rejects a second
directory, rejects an unknown option, and exits 2 with usage when the required
directory is absent — each exercised. `--fresh` deletes exactly the four files it
names and nothing else; pointed at a directory where `identity.key` is a symlink
to a file elsewhere, `std::fs::remove_file` unlinked the symlink and the target
survived intact (checked by reading it back). The refusal path runs before the
first store is opened, so the "a check after the first open would be checking a
file this program just wrote" reasoning holds as stated.

The three derivation positions genuinely match the module: the seeder's creator
is `identity_public_key` and `create_stoa` uses `creator_key_in`
(`lib.rs:684`), which is `creator_and_poster_in().0` = `identity_public_key`;
the seeder signs with `stoa_key(&address)` and the adapter signs with
`keystore.stoa_key(&stoa)` (`lib.rs:539`); the seeder reports
`stoa_address_at_path` and `wire::posting_identity` (`wire.rs:324`) reports
`keystore.stoa_address_at_path(stoa, path)`. That claim is verified, and it is
the substantive part of the change.

No new dependency: `hex` is already `dialectica-core/Cargo.toml:110`. Zero
`#[test]` in the example — `grep -c` returns 2, both of which are the words
`#[test]` inside prose in the module docstring at `:124-131`, not attributes;
CI's gate counts `^\s*#\[test\]\s*$` on its own line, which matches neither.
The CI compile claim holds: `cargo test` and `cargo clippy --all-targets` both
build the example.

`cargo fmt --manifest-path … --check` — the exact CI invocation — passes,
because it does not follow the path dependency into `dialectica-core`. That is a
pre-existing repo-wide gap, not this piece's defect, and the example is
`rustfmt --check` clean when checked directly.
