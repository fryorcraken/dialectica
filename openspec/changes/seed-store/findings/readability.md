# Readability findings: the store seeder

Reviewed `dialectica/rust-lib/dialectica-core/examples/seed_store.rs` at `c2bf6f5`,
in a worktree of `piece/seed-store`. Correctness and security were reviewed and
closed separately; nothing below re-reports one of their ten entries.

**The tool runs and the output is largely excellent.** Every claim below was
produced by running the program and reading its real output, not by reading the
source and reasoning about it. The suite is green in this worktree — 733 + 26 =
759 passed, 0 failed — `clippy --all-targets -- -D warnings` is clean for both
our packages, and `rustfmt --edition 2021 --check` on the example is clean.

The question this dimension asks is the one the brief names: **does the output
teach a UI developer the truth?** Three of the four entries below are places
where it teaches something that is not true, and all three were introduced or
left behind by the review-fix commit `c2bf6f5` — the fix that moved `second_root`
to the visitor did not carry its consequences into the prose or the report.

---

- [x] **`dev-writer`** — `seed_store.rs:722` — the report says "every seeded op is
      by \<one address\>", and that is false for the **majority** of the ops it
      just wrote
      **Scenario:** the report prints

      ```
      author addresses, which do not agree — this is the known gap:
        getCapabilities reports  061125a2…
        every seeded op is by    3878e42b…
        record names as creator  5c9aaf74…
      ```

      **Measured on a real seeded store** (`--fresh` into a scratch directory,
      then every entry read back through `OpLog::iter` and grouped by
      `entry.op.op.author.address()`):

      ```
      3878e42b6e46622cac28c4663b7815d89b095d11c678b78c0f8e7cf86e6834d3  4
      c13fccc816a073117196b86546f471c3bf7b01899c87afd962f474d303cb0543  5
      ```

      Four of the nine ops are by the address the report names. **Five are by
      `c13fccc8…`, the visitor — an address the report never prints anywhere.**
      Counted from the source and it agrees: founder signs `first_root` (`:438`),
      `nested` (`:480`) and two votes (`:517`, `:519`) = 4; visitor signs
      `second_root` (`:455`), `reply` (`:470`), `other_reply` (`:490`) and two
      votes (`:516`, `:518`) = 5.

      **This is a regression of `c2bf6f5`, not a pre-existing wart.** At `04da8c8`
      both roots were the founder's, so "every seeded op" was at least true of
      every *feed row*. The fix moved `second_root` to the visitor specifically so
      that "author attribution is visible" — and left the line claiming uniform
      authorship standing directly above it.

      **The harm is the exact one this report block exists to prevent.** A UI
      developer reads three labelled addresses, opens `listThreads`, and sees a
      feed row authored by a fourth address that appears on none of the three
      lines:

      ```
      listThreads -> {"items":[
        {"author":"916a6803…", "body":{"text":"On the difference between moderation and censorship"}…},
        {"author":"13920f73…", "body":{"text":"What does it mean for a forum to be decentralized?"}…}]}
      ```

      (run against the seeded store; `13920f73…` is that run's signing address and
      is on the report, `916a6803…` is the visitor and is not). The docstring at
      `:99-102` promises exactly this reader that "Both are printed, side by side
      and labelled as the known gap" — so the one reader who trusts the report
      most is the one it misleads. That is worse than the pre-fix state, where the
      claim was at least consistent with what the feed showed.
      **Severity: high — this is the piece's single most-read output line, and it
      is wrong about the majority of the store.**

      The fix is to print the visitor too and reword the line to what it now is:
      the founder's ops carry the signing address, the visitor's carry the
      visitor's. Printing a fourth labelled address costs one `println!` and the
      visitor's address is already computed at `:670` for the assertion.

      **Fixed**, and reproduced first: a `--fresh` run before any edit printed the
      three labelled addresses with the visitor's among none of them, exactly as
      you measured.

      The block now prints **four** addresses with a per-author op count against
      each, and no "every" anywhere:

      ```
      author addresses — the first three DISAGREE, which is the known gap:
        getCapabilities reports  7066f1af…
        founder signs ops with   f577f34e…  (4 of 9 ops)
        record names as creator  ed297c91…
        and a second identity, so author attribution is visible rather than uniform:
        visitor's ops are by     0a2d32f0…  (5 of 9 ops)
      ```

      4 and 5, matching your count from the source.

      **The counts are read back from the store**, by a `seeded_ops_by(log, hex)`
      helper over `OpLog::iter`, not restated from the writes above. That is
      deliberate and is the difference between this block and the sentence it
      replaced: a figure read back can be wrong and be caught, where "every seeded
      op is by X" was a claim nothing in the program could contradict.

      **And the reword is now backed by an assertion**, which `spec-test.md` entry
      3 is the box for: the distinct-author set over all nine ops must be exactly
      the founder's signing address and the visitor's. Your own point is why — a
      reword alone would go false the next time an op moves between identities,
      which is precisely how this defect arrived.

      `design.md` carries the general rule under "What the report may claim", so
      this is a consequence of a recorded decision rather than a one-off patch.

- [x] **`dev-writer`** — `seed_store.rs:100` — the module docstring makes the same
      false uniform-authorship claim, one level up
      **Scenario:** `:99-102` reads *"every seeded post's `author` in the feed is
      the *signing* address, and `getCapabilities` reports a *different* one."*
      Measured against the run above, the feed's two rows carry **two different
      authors**, only one of which is the signing address. The sentence was true at
      `04da8c8` and `c2bf6f5` falsified it without editing it.

      Separate box from the entry above rather than bundled, because they are two
      edits in two places and a fixer who corrects the report line will not
      necessarily scroll up 620 lines to the preamble. The docstring is also the
      half that a reader meets *first* — someone opening the file to understand the
      tool reads this paragraph before ever running it, so a wrong claim here
      frames everything after it.
      **Severity: medium — documentation, but it is the paragraph the file's own
      report block points at.**

      **Fixed.** The sentence now reads *"the **founder's** posts carry the
      signing address as their feed `author`"* rather than "every seeded post's".

      You were right to make this a separate box, and it earned the separation: I
      would have missed it working from the report line alone, since it is 620
      lines up and the two share no phrasing.

      A second paragraph follows it, saying that the visitor's ops carry the
      visitor's address and that there are **more of them** — four and five of the
      nine — so a reader meets the asymmetry in the preamble rather than being
      surprised by it in a feed row. It names the regression as the reason the
      paragraph exists, cites this finding, and says the distinct-author set is now
      asserted, which a `contains` check structurally cannot do.

- [x] **`dev-writer`** — `seed_store.rs:551-552` — the `ops == 9` assertion message
      contradicts itself on failure, in the one output a broken run produces
      **Scenario:** the message is
      `"two roots, three replies and four votes is nine ops, not {ops}"`, where
      `{ops}` is the count actually found. Reproduced by changing the expected
      count to 10 and running `--fresh`:

      ```
      thread 'main' panicked at dialectica-core/examples/seed_store.rs:550:5:
      assertion `left == right` failed: two roots, three replies and four votes is nine ops, not 9
        left: 9
       right: 10
      ```

      **"is nine ops, not 9"** — the sentence tells the reader that 9 is the wrong
      answer, when 9 is what the store actually holds and the expectation is what
      moved. `{ops}` is the *found* value, so the "not {ops}" construction is
      backwards: it reads as naming the defect and instead names the fact.

      `assert_eq!` already prints `left`/`right` correctly, so the custom message's
      only job is to explain *why* nine is expected — and in the failure case it
      actively fights the two lines below it. The sibling assertion at `:544-549`
      has the same shape (`"must hold two thread heads, not {}"` with
      `feed.items.len()`), and is wrong in the same way for the same reason.

      Both messages are right if the interpolation is simply dropped: "two roots,
      three replies and four votes is nine ops" is a complete and correct
      explanation, and `assert_eq!` supplies the numbers. The restored file was
      verified clean afterwards.
      **Severity: medium — it is a defect in the diagnostic that a failing run
      exists to produce, and this piece's whole architecture rests on those
      assertions being the only thing that makes a bad run loud.**

      **Fixed** by dropping the interpolation from both, as you proposed. And I
      re-ran your reproduction rather than trusting the reasoning — same mutation,
      expected count raised to 10:

      ```
      assertion `left == right` failed: two roots, three replies and four votes is nine ops
        left: 9
       right: 10
      ```

      The message now explains the expectation and `assert_eq!` supplies the
      numbers, with nothing fighting the two lines below it. Restored to `9` and
      the seeder exits 0.

      The sibling at `:544-549` is fixed in the same way — *"the seeded feed must
      hold two thread heads, one per root"*, which also says **why** two rather
      than just restating the number, since that is the custom message's only
      remaining job.

      A comment above the pair records the rule so the next assertion added here
      is not written in the old shape.

- [x] **`dev-writer`** — `seed_store.rs:220` — `no---fresh` in the help text reads
      as a typo, and the paragraph it sits in is the one review asked for
      **Scenario:** `--help` prints, verbatim:

      ```
      no---fresh path promises "nothing was written"; this one cannot.
      ```

      The source is `The\n no---fresh path`, intended as "no-`--fresh`". Rendered
      as a triple hyphen it reads as a formatting accident, in the sentence added
      to answer `findings/correctness.md` entry 4 — so the one line a reader is
      meant to take seriously about losing their store is the one that looks
      unproofed. "the path without `--fresh`" says it with no ambiguity.

      The same `usage()` block has a second rendering defect worth the same edit:
      the `--fresh deletes the four files this tool writes ({})` line expands at
      runtime to 121 characters (`48` + `56` of filenames + `17`), breaking a block
      that is otherwise hard-wrapped at ~70. The source line looks short because
      the placeholder hides the expansion; only running `--help` shows it.
      **Severity: low — cosmetic, but both are in the help text, which is the one
      thing a user reads before deciding whether to delete their store.**

      **Both fixed**, and verified by running `--help` rather than by reading the
      source, which is the only way either was visible in the first place.

      `no---fresh` is now *"The path without `--fresh` promises …"*, your wording.

      The long line is fixed by **breaking the list onto its own indented lines**
      rather than by shortening it, since the expansion is four filenames and no
      phrasing makes them fit:

      ```
      --fresh deletes the four files this tool writes, before seeding:
        identity.key
        identity.sqlite
        stoas.sqlite
        ops.sqlite
      Without it, an existing store is REFUSED and nothing is written.
      ```

      Every line of `--help` is now inside the block's ~70-column wrap, and the
      four names are scannable rather than comma-run-on — which matters more here
      than the wrap, since this is the list a person checks before deleting. The
      call site carries a comment saying the length is only visible by running
      `--help`, because the source line looks short and the placeholder hides it.

## What was clean

**The self-invalidating assertion is the best-judged thing in the piece, and its
intent is legible without the findings file.** `assert_ne!(posting_address,
signing_address, …)` at `:673-678` carries its own reason in the message — *"the
probe and the publish path have stopped disagreeing — the three-derivations gap
is closed, so this assertion and the paragraph it documents should both go"* —
and the negative moderator assertion at `:622-627` does the same, naming three
specific things to delete. A reader who hits either failure learns, from the
panic message alone and without opening any other file: that the failure is good
news, that it means the module changed rather than the tool, and exactly what to
remove. That is the shape CLAUDE.md's "make it self-invalidating" section asks
for, executed better than the guidance describes. Nothing to do here.

**The comment above the negative assertion earns its place under the "say what a
command cannot" test.** `:605-617` explains that the line it replaced was
`assert!(moderators.contains(&genesis.creator))`, *why* that reduced to
`creator == creator`, and that the property it appeared to claim is false today.
None of that is recoverable from the code, it is the reasoning most likely to be
undone by a later reader "simplifying" the pair back to one assertion, and it
cites the finding that produced it. The companion positive assertion kept beside
it does read as redundant in isolation, but the comment pre-empts exactly that
reading and says why it is kept — which is the right resolution.

**The `keystore.rs` pointer at `:88-97` is enough for a reader who lands in the
seeder first, and I checked it rather than taking it on trust.** It names both
docstrings, quotes what each claims, and states plainly that the adapter calls
`stoa_key` so both sentences are false. Verified: `Keystore::stoa_key`'s comment
at `keystore.rs:793-797` does say "built and, in the MVP, not called by any
handler"; `Keystore::identity_key`'s at `:822-826` does say "there is exactly one
derivation position, so a creator and a poster cannot be two keys"; and
`dialectica/rust-lib/src/lib.rs:539` is `keystore.stoa_key(&stoa)`. The quotes are
accurate and the conclusion follows. A reader arriving here, then reading
`keystore.rs` and finding it contradictory, has been warned in advance and told
who should fix it. Leaving `keystore.rs` unedited is the right call for a piece
told not to widen, and the pointer carries the cost of that decision honestly.

**The three `Main.qml` properties are correct**, checked against the file rather
than the claim: `stoaAddress`, `stoaTitle` and `stoaGenesis` are declared at
`dialectica-ui/src/qml/Main.qml:26`, `:27` and `:32`. Pasting the three printed
lines works.

**The report's ordering is well judged.** The two underivable values come first
and are labelled as the reason the program prints anything; the pasteable request
and the QML properties come last, which is the order a person actually needs them
in. The `=> MODERATION DOES NOT WORK` line does the job the brief asks about — a
developer whose hide button does nothing reads one line and learns it is not
their screen — and it says *why* (the signing key is not the creator the record
names) rather than only *that*.

**The `why(context, result)` helper is the right shape and its comment justifies
itself**: four of the stores are SQLite files that fail with identical `rusqlite`
wording, so the prefix is load-bearing rather than decorative, and the comment
says so in one sentence.

**Section banners and naming.** The `── Section ──` comment banners divide the
`main` body into eight phases that each read as one job, and every binding is
named for what it is (`first_root`, `nested`, `other_reply`, `posting_address`,
`signing_address`) rather than for its type or position. There is no `handle`,
`process` or `And` anywhere in the file.
