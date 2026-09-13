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

- [x] **`dev-writer`** — `seed_store.rs:486-489` — the moderator assertion is
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

      **FIXED**, and the finding's reading of the mechanism is confirmed exactly.
      Reproduced first: substituting `moderators.contains(&founder.public_key())`
      panics with "the signing key must moderate the Stoa it founded", so the
      property really is false for every store this tool writes.

      The fix asserts **both halves as they actually are** rather than only the
      one that fails, because each rules out a different regression:

      ```rust
      assert!(moderators.contains(&genesis.creator), …);
      assert!(!moderators.contains(&founder.public_key()),
              "the signing key has BECOME a moderator — the three-derivations gap
               is closed, which is good news. Delete this assertion, the
               `moderation` line in the report, and the paragraph in design.md
               that documents the gap.");
      ```

      The first is still the tautology you identified and is kept **only** as a
      companion to the second: on its own it proves nothing, but paired it makes
      the negative assertion legible as "the creator moderates and the signer does
      not" rather than as a bare `!contains`. It is no longer carrying the comment
      that claimed it caught the trap; that comment is replaced by one naming the
      tautology and citing this entry.

      The second is the real check, in the self-invalidating shape. It cannot be
      satisfied by the tool changing — only by the module's derivations converging
      — and it names what to delete when they do.

      **The tool cannot close the gap itself**, and that is now stated at the
      assertion rather than implied: which key a publish signs with is the spec
      question `ci.yml` exempts, and a seeder that signed with the creator would
      disagree with the adapter, which is the one thing that would stop a seeded
      store being evidence of anything. What the tool can do is stop hiding it, so
      the report now prints all three addresses and the consequence in words:
      "=> MODERATION DOES NOT WORK on a seeded Stoa: a hide published through the
      module is refused". A UI developer whose hide button does nothing reads a
      line instead of debugging their own screen. `design.md` gains a section.

- [x] **`dev-writer`** — `seed_store.rs:471-476` — `feed.items.len() == 2` and
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

      **FIXED**, with one correction to the suggested route. There is **no thread
      read in `dialectica-core`** — `feed.rs` exposes `list_threads` and
      `clamp_per_page` and nothing else, and the thread read is `piece/thread-read`,
      still in flight. So the assertion goes through `OpLog::get`, which is public
      API the wire layer uses, reading each reply back out of the store rather than
      trusting the `Published` values the publish calls returned.

      All three replies are checked for `parent` **and** `thread`. The `nested` row
      is the discriminating one and the entry is right about why: at two levels
      "the parent's id" and "the parent's thread" are the same value, so a
      `thread: parent` bug is invisible; three levels separate them.

      **Measured discriminating**: pointing `nested`'s expected parent at
      `first_root.id` fails with `assertion left == right failed: nested must name
      its parent`. Restored afterwards.

      A comment at the assertion records that this is the assertion to move onto
      the thread read when `piece/thread-read` lands.

- [x] **`dev-writer`** — `seed_store.rs:506-510` — `feed.items[0].author` is the
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

      **FIXED** by the first of the two routes offered: `second_root` is now
      published by `visitor`. That is the smaller change and it makes the docstring
      claim true where the docstring says it should be — in the feed — rather than
      deferring it to a thread read the crate does not have (see the entry above).

      The assertion is rewritten to check **both** rows by lookup rather than by
      index:

      ```rust
      let authors: Vec<&str> = feed.items.iter().map(|r| r.author.as_str()).collect();
      assert!(authors.contains(&signing_address.to_hex().as_str()), …);
      assert!(authors.contains(&visitor.public_key().address().to_hex().as_str()), …);
      ```

      Two consequences worth naming. It now compares two genuinely different
      values, so neither assertion is satisfied by the other's subject; and it no
      longer bakes in the feed's ordering, which `items[0]` did implicitly and
      which is not a property this tool should be pinning — `cmp_ops` order is
      `feed.rs`'s business and could change without this tool being wrong.

      The op count is unchanged at nine, so the `ops == 9` assertion still holds
      and the nesting assertions above are unaffected (`other_reply` still replies
      to `second_root`; only its root's author moved).

- [x] **`dev-writer`** — `seed_store.rs:255` + `:262-288` — `--fresh` destroys
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

      **FIXED** as suggested — the help text now carries it, in the finding's own
      terms rather than a softened paraphrase:

      > `--fresh` DELETES FIRST AND CHECKS AFTERWARDS. The assertions that make a
      > bad run fail run at the end, over ops already written, so a run that fails
      > one leaves a half-seeded directory and the old store is gone. The
      > no-`--fresh` path promises "nothing was written"; this one cannot.

      **Not fixed by reordering**, and the reason is worth recording rather than
      leaving as an omission. Moving the assertions before the deletion is
      impossible: they are assertions *about the store this run writes*, and there
      is nothing to assert over until the ops exist. The alternatives are seeding
      to a temporary directory and moving it into place — which is a real design
      and is what a tool that must never lose a store would do — or accepting the
      window and documenting it. The second is right here: `--fresh` is opt-in,
      the security entry now makes it refuse a directory that is not a dialectica
      store, and building a staging-and-rename path would grow the tool well past
      what this piece is for.

- [x] **`dev-writer`** — `proposal.md:37` — names `identities.sqlite`, but the
      file the tool writes is `identity.sqlite`
      **Scenario:** `IdentityStore::default_path_in`
      (`identity_store.rs:211-213`) is `dir.join("identity.sqlite")`, and a run
      produces `identity.sqlite`. `proposal.md` line 37 writes
      "an identity record (`identities.sqlite`)". The example itself is correct —
      it derives the name through the `core` function rather than spelling it —
      so this is a documentation defect only. It matters because a reader
      checking the seeded directory against the proposal will not find the file
      the proposal names. **Severity: low.**

      **FIXED** — `proposal.md` now reads `identity.sqlite`. Confirmed against a
      real run: the seeded directory holds `identity.key`, `identity.sqlite`,
      `stoas.sqlite`, `ops.sqlite`, and the `--fresh` output names the same four.

      The diagnosis is exactly right and worth keeping visible: the example was
      never wrong because it derives the name through
      `IdentityStore::default_path_in` rather than spelling it. The prose was wrong
      precisely *because* it spelled a name by hand — the same reason the one
      hand-copied path in the example (`ops.sqlite`, which has no `core` accessor)
      carries a comment flagging it.

- [x] **`dev-writer`** — `seed_store.rs:493-494` — an `expect` on a path this
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

      **FIXED** as described — `.expect(…)` is now
      `.ok_or_else(|| "reading the chosen path back: the path just recorded is
      absent".to_string())?`, so every fallible step in `main` reports the same
      way. The "sits directly after a `--fresh` deletion" point is the one that
      decided it: a backtrace is the worst output to hand someone who has just
      lost their store, and it was the only line in the file that produced one.

      No `expect` or `unwrap` remains in the file (`grep -n "panic!\|expect(\|unwrap()"`
      returns one line). That line is a `panic!` in the nesting loop this round
      added — the non-`Post` arm of a match over `OpKind` — and it is deliberate
      for the reason `authoring.rs` gives about exhaustive matches: it is
      unreachable for ops this program just published as posts, and a wildcard
      returning something plausible would let a future op kind pass silently. It
      sits beside the `assert!`s, which abort on purpose.

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
