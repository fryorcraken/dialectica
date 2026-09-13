# Design findings: the store seeder

Reviewed `design.md` against `dialectica/rust-lib/dialectica-core/examples/seed_store.rs`
at `c166a99`, and against `docs/PLAN.md` read from **`origin/main`** (`53f08b3`) rather
than the branch's copy. The branch changes no file under `docs/`, so the two PLAN
copies are identical — but the branch's merge base is `733544d`, three commits behind
`origin/main`, and that gap is where the first finding lives.

The suite is green in this worktree: `cargo test --manifest-path … -p dialectica -p
dialectica-core` gives 733 + 26 passed, 0 failed. The outer package does not resolve in
a fresh worktree without linking `dialectica/logos-rust-sdk-src` from the main checkout
— which is `design.md`'s own deciding reason for the crate choice, reproduced
accidentally.

**The Decisions section is unusually strong on the things it covers.** Every one of the
twelve entries names what was chosen, the constraint, and at least one rejected
alternative; the self-invalidating assertions are the best-executed part of the piece.
Nothing below contradicts a recorded decision in the "the code does the opposite" sense.
The findings are one decision recorded against a world that has since changed, two
decisions taken in code and not recorded at all, one figure that is wrong, and one
document this change was obliged to fix and did not.

Nothing here re-reports the ten closed entries in `correctness.md`/`security.md` or the
six open ones in `readability.md`/`architecture.md`. In particular the high-severity
"every seeded op is by \<one address\>" line is `readability.md` entry 1's, and the
design question it raises — does `design.md` record what the report may claim — is
answered in entry 3 below from the opposite direction.

---

- [x] **`dev-writer`** — `design.md:247-249` and `seed_store.rs:568-570` — "core has no
      thread read, `piece/thread-read` is still in flight" is **false on `origin/main`**,
      and it is the stated reason the weakest assertion in the file was written the way
      it was
      **Scenario:** the Decisions entry "The nesting is asserted, because the counts
      cannot see it" closes with:

      > **Not through a thread read, because core has none.** `feed.rs` exposes
      > `list_threads` and nothing else; the thread read is `piece/thread-read`, still in
      > flight. When it lands, this is the assertion to move onto it.

      The code carries the same sentence at `:568-570`, and `tasks.md:96-98` records it
      as a correction *to* the correctness reviewer, who had suggested exactly that
      route.

      **It landed.** `piece/thread-read` merged as #61 at `53f08b3`, which is
      `origin/main`'s tip and the commit immediately after this branch's merge base
      `733544d`:

      ```
      $ git log --oneline -1 origin/main -- dialectica/rust-lib/dialectica-core/src/thread.rs
      53f08b3 Contract the thread read: a root, its replies, and who is trusted to place them (#61)
      $ git merge-base piece/seed-store origin/main
      733544df4c89a41dcdcc3216f4bcd321598e654d
      ```

      `dialectica-core/src/thread.rs` on `origin/main` exposes
      `pub fn read_thread(log, moderators, stoa, root, page, per_page, include_hidden)
      -> Result<Result<ThreadPage, NotAThread>, OpLogError>` at `:448`, plus
      `pub fn thread_of` at `:316`; `wire.rs` exposes `read_thread` at `:1547` and
      `read_thread_from_request` at `:1649`. Every argument `read_thread` needs, the
      seeder already has in scope at the assertion site — `log`, `moderators`, `address`,
      and each root's `OpId`.

      **Why this is a design finding and not a rebase chore.** This is the failure mode
      the design dimension exists to catch, named in this role's own brief: *a workaround
      designed against a superseded section*. The `OpLog::get` loop is not the assertion
      the reviewer asked for; it is a **substitute chosen because the real one was
      believed not to exist**, and the belief is now wrong. The difference is not
      cosmetic — `OpLog::get` reads the raw op and checks two fields this program itself
      wrote, whereas `read_thread` is the function `listThread` calls, so it would check
      that the **reader** reconstructs the tree, which is the property the tool exists to
      produce. `thread.rs`'s own docstring says membership is *derived by following
      parents to a root* and *"the claimed field decides nothing"* — so a store whose
      `parent` chain is right passes the current loop and still says nothing about what a
      UI will render.

      Two things must happen, and they are one edit each: **delete the "core has none"
      justification** from `design.md:247-249` and `seed_store.rs:568-570` (it is now a
      false claim in the document whose job is to record why), and **move the assertion
      onto `thread::read_thread`**, which is what both this entry and `tasks.md` already
      commit to doing "when it lands". If the rebase is deferred, the sentence still has
      to go — a recorded reason that is false is worse than no reason, because the next
      reader stops looking.
      **Severity: high — a recorded decision whose justification no longer holds, on the
      assertion covering the one property this tool exists to produce.**

      **Fixed in part, deferred in part** — and the split is forced by where the
      branches are, not chosen.

      **Fixed:** the false sentence is gone from both places. `design.md`'s entry
      now says `read_thread` merged as #61 (`53f08b3`), immediately after this
      branch's merge base `733544d`, and says plainly that the earlier claim was
      true when written and is false now. The code comment at the assertion site
      says the same. Verified before writing it: `git merge-base piece/seed-store
      origin/main` is `733544d`, and `git log --oneline -3 origin/main` shows
      `53f08b3` and `f1b3a5e` above it.

      Both places also now carry the part the finding's own argument rests on and
      neither document stated: **`OpLog::get` checks the store, not the reader.**
      It hands back the raw op, so the two fields compared are two fields this
      program wrote, and a claim-trusting reader and a parent-walking reader give
      the same answer against a store whose chain is correct — so passing says
      nothing about what a UI renders.

      **Deferred, and here is where it lives:** the assertion is not moved onto
      `read_thread`, because **`read_thread` does not exist on this branch.**
      Verified rather than assumed — `dialectica-core/src/thread.rs` is absent in
      this worktree and `lib.rs` declares no `thread` module; `git show
      origin/main:.../thread.rs` is where I read `read_thread`'s signature at
      `:448`. Moving the assertion requires the two branches to meet, and this
      piece's brief is explicit that nothing may be pulled, merged or rebased in.

      So it is recorded as the follow-up in two durable places rather than left in
      this tracker, which is deleted at archive: `design.md`'s entry names the
      edit and lists every argument `read_thread` takes as already in scope at the
      assertion site, and the code comment says the same to a reader who never
      opens `design.md`. The remaining work after the branches meet is one loop
      rewritten.

- [x] **`dev-writer`** — `seed_store.rs:177-183` — `SEEDED_PATH = 0`'s docstring gives a
      reason that is **false**, and the real decision is not in `design.md` at all
      **Scenario:** the constant is documented as:

      > Zero is the first candidate of a slate, which is what a user pressing through
      > onboarding without deliberating would land on.

      A slate's candidate paths are **not** `0..5`. `onboarding::derive_path`
      (`onboarding.rs:175`) is `SHA256(SLATE_PATH_PREFIX || nonce || index)`, first four
      bytes big-endian with the top bit masked — so the first candidate's path is a
      pseudorandom value below 2³¹, and index 0 of a slate is path 0 only with
      probability ~2⁻³¹.

      **Measured**, by calling `derive_path` over 2000 distinct nonces:

      ```
      nonce#0: index-0 path = 427553217, slate paths = [427553217, 1588889694, 899522834, 1097285982, 1016223951]
      nonce#1: index-0 path = 358622328, slate paths = [358622328, 1105834749, 1385818511, 455176866, 911074720]
      nonce#2: index-0 path = 1893493476, slate paths = [1893493476, 279406633, 157675469, 153861641, 2115427235]
      nonces out of 2000 whose index-0 candidate has path 0: 0
      ```

      So a seeded store records a path that **no onboarding run would ever produce**.
      The docstring says the opposite: that it is the ordinary path.

      **The choice itself is defensible and is not the finding — the missing record is.**
      `0` is a perfectly good seeder path: it is in range (`identity_store`'s guard is
      `PATH_LIMIT`, the same constant the mask uses), the seeder and the probe agree
      because both read the recorded row, and the value is reproducible across runs,
      which matters for a tool whose output a person pastes. There is even an argument
      it is the *better* choice precisely because it is unreachable — a seeded store is
      then distinguishable from a real one. **None of that is written down**, and a
      reader who checks the stated reason finds it false and has nothing to fall back on.

      This is `design.md`'s "a constant whose value matters" case: the real alternatives
      were "derive a slate and keep candidate 0, as onboarding does" (heavier, needs a
      nonce, but produces a store indistinguishable from a real one) versus "pick a fixed
      path" (what was done). **Add a Decisions entry** naming the constraint (the seeder
      and the probe must agree, and the value must be reproducible), the alternatives,
      and the cost — that a seeded store's recorded path is one onboarding cannot
      produce, so a test that assumed "any recorded path came from a slate" would be
      wrong about seeded stores. And fix the docstring, which currently asserts the
      thing the code is not.
      **Severity: medium — a false justification on a constant, plus an unrecorded
      decision with a real rejected alternative.**

      **Fixed.** Both halves.

      **The docstring** now says the opposite of what it said: zero is *not* a
      path onboarding produces, with the mechanism (`SHA256(prefix || nonce ||
      index)`, first four bytes big-endian, top bit masked) and the ~2⁻³¹ figure.

      **I re-ran the measurement rather than quoting yours**, per this repo's "run
      the claim, don't read it" rule — a throwaway example over 2000 generated
      nonces, deleted afterwards. Zero hits, and three sample slates:
      `[1336077579, 1318252717, 831137178, 537227485, 647957063]`,
      `[766249687, 2014285701, …]`, `[855138262, 91996491, …]`. Different nonces
      from yours, same conclusion.

      **The Decisions entry** is new, under "`SEEDED_PATH = 0`, a path onboarding
      cannot produce". It names the constraint (seeder and probe must agree; the
      value must be reproducible because a person pastes the output), both
      alternatives with what ruled each out, and the cost in your own terms — that
      a test assuming "any recorded path came from a slate" would be wrong about
      seeded stores. Your "unreachability is arguably the better property" reading
      is recorded as an argument that exists but was *not* the deciding reason,
      rather than promoted into one it never was.

- [x] **`dev-writer`** — `design.md` has no entry for **what the report is allowed to
      claim**, which is the gap the high-severity readability finding fell through
      **Scenario:** the review brief asks specifically whether `design.md` records a
      decision about what the printed report may assert. It does not — and that absence
      is the structural cause of `readability.md` entry 1, rather than a separate bug.

      Every other claim this tool makes is pinned by an `assert!` that runs: the two
      counts, the three `parent`/`thread` pairs, the two feed authors, the
      `posting_address != signing_address` gap, the negative moderator check. The report
      block at `:680-757` is the **one** set of claims with no such pin, and it is the
      only output most readers ever see. `design.md`'s "Both author addresses are
      printed" entry decides *which* addresses to print and asserts their disagreement —
      but says nothing about the **prose around them**, so `:722`'s "every seeded op is
      by \<one address\>" was free to go false when `c2bf6f5` moved `second_root` to the
      visitor, with every assertion still green.

      Confirmed by running the seeder: the report prints three labelled addresses and the
      visitor's is not among them, while five of the nine ops are the visitor's. The
      assertions at `:664-672` already compute the visitor's address for a different
      purpose and pass, so **nothing in the program's own checking apparatus could have
      noticed**.

      This is CLAUDE.md's "put the complexity in the data structure, not the logic",
      one level up: the file's invariant is *a claim in the report is a claim an
      assertion pins*, held at eleven sites and missed at the twelfth — the point the
      guard should have become a rule. **Record the decision**: state that the report
      prints only values the program has asserted, name what that forecloses (a line like
      "every seeded op is by X" cannot be written unless an assertion establishes it, so
      summary prose about authorship has to become either a per-author breakdown or
      nothing), and say why the alternative — free prose in the report, checked by
      review — was rejected. Once it is recorded, `readability.md` entry 1 is a
      consequence of it rather than a one-off.
      **Severity: medium — a gap where no decision was made, in the one output no
      assertion covers. Not a contradiction; a missing entry.**

      **Fixed.** New Decisions entry, "What the report may claim: only values an
      assertion pinned", recording the rule in the terms you set it: eleven claims
      pinned by a running assertion and the report's prose the twelfth; the rule
      that the report prints only asserted values; what that forecloses —
      authorship summaries beginning "every" cannot be written, so the line becomes
      a per-author breakdown or nothing; and the rejected alternative, free prose
      checked by review, which is what was in place and what shipped a
      high-severity false claim through five reviewers with every gate green.

      The entry also carries the structural half you identified from the other
      direction, because it is the reason the rule is needed rather than a
      preference: `contains` is existential where the claim was universal, so no
      `contains` check could ever have contradicted it.

      `readability.md` entry 1 is now the consequence of this entry rather than a
      one-off, which is what you asked for. The report's per-author counts are
      **read back from the store** by a `seeded_ops_by` helper rather than
      restated from the writes above, so a figure it prints is a reading that can
      be wrong and be caught.

- [x] **`dev-writer`** — `design.md:146-149` and `seed_store.rs:146-150` — "**six** of the
      crate's **eight** error types" is wrong in both figures; it is **ten of twelve**
      **Scenario:** the `Result<(), String>` entry opens:

      > **Six of the crate's eight error types do not implement `std::error::Error`.**
      > Only `OpLogError` and `MembershipError` do, so the obvious
      > `Box<dyn std::error::Error>` signature does not compile against `KeystoreError`,
      > `GenesisError`, `IdentityStoreError`, `Refusal`, `RandomnessUnavailable` or
      > `OnboardingError`.

      Counted over `dialectica-core/src/`, every one of them in a `pub mod` and therefore
      public:

      ```
      $ grep -rn "pub enum .*Error|pub struct .*Error|pub enum Refusal|pub struct RandomnessUnavailable" dialectica-core/src/
      stoa.rs:204            GenesisError
      identity.rs:182        AddressError          <- not listed
      identity.rs:496        RandomnessUnavailable
      identity.rs:512        KeyError              <- not listed
      authoring.rs:84        Refusal
      onboarding.rs:363      OnboardingError
      membership.rs:162      MembershipError       (has Error)
      identity_store.rs:66   IdentityStoreError
      keystore.rs:501        KeystoreError
      log/mod.rs:195         OpLogError            (has Error)
      op.rs:193              OpIdError             <- not listed
      op.rs:478              OpError               <- not listed
      ```

      Twelve types; `impl std::error::Error` appears exactly twice in the crate
      (`membership.rs:274`, `log/mod.rs:262`). So **ten** lack it, not six, and the
      denominator is twelve, not eight. `AddressError`, `KeyError`, `OpIdError` and
      `OpError` are missing from the list — all four have a `Display` impl and none has
      `Error`.

      **This matters beyond arithmetic, because the entry's whole job is to size a
      deferred change.** It ends by recording a dead end *"for whoever wants `?` next:
      propose the impls as their own change"* — and that person will scope six impls and
      find ten. The understatement makes the deferral look cheaper than it is, which is
      the direction that gets a deferred change mis-planned rather than merely
      mis-stated.

      It is also CLAUDE.md's *"do not write down anything a command can answer"* applied
      to `design.md`. **The fix is to name the command rather than the number** — the two
      greps above, or simply "every error type in the crate except `OpLogError` and
      `MembershipError`", which is a relation that stays true as types are added rather
      than a count that silently rots. The argument the entry makes does not depend on
      the figure and survives the edit unchanged.
      **Severity: medium — a wrong figure in the entry that sizes a follow-up change, and
      a count where a command belongs.**

      **Fixed, and I re-counted rather than taking your figure.** Both greps run
      against this worktree: twelve `pub` error types, and `impl std::error::Error`
      exactly twice (`membership.rs:274`, `log/mod.rs:262`). Ten of twelve, and
      your four unlisted — `AddressError`, `KeyError`, `OpIdError`, `OpError` —
      are all present. Your count is right.

      **Both `design.md` and the module docstring now state the relation and name
      the commands**, rather than carrying a corrected number that would rot the
      same way: "every public error type in `dialectica-core` except `OpLogError`
      and `MembershipError`", with the two greps beside it. The types this program
      actually touches are still listed, because that list is what makes the `?`
      argument concrete.

      The `design.md` entry keeps the wrong figure visible as the reason for the
      change — it records that the previous "six of eight" understated the
      deferral's scope by four types, which is the direction that gets a deferred
      change mis-planned. The argument itself needed no edit, as you said.

- [x] **`dev-writer`** — `docs/UI-BRIEF.md:296-299` asserts the property this change
      exists to document as **false**, and the brief is not fixed in this change
      **Scenario:** the brief's "Creating a Stoa" section tells the external designer:

      > **The key recorded as creator is the same key the user posts under**, which is
      > the one identity constraint 2 describes. It matters here because a Stoa's creator
      > is its sole moderator: a creator key the user does not sign with would be a Stoa
      > nobody can moderate, permanently […] so do not design as though the creator and
      > the poster could be different people.

      This change's central discovery is that **they are different keys today**. The
      seeder asserts it (`:622-627`, `!moderators.contains(&founder.public_key())`, which
      passes), `design.md`'s "Moderation does not work on a seeded Stoa" entry states it,
      and the report prints it in capitals:

      ```
      author addresses, which do not agree — this is the known gap:
        getCapabilities reports  03cec99750bcb7ff…
        every seeded op is by    46743d6cbb79b608…
        record names as creator  9cec24182d1afca0…
        => MODERATION DOES NOT WORK on a seeded Stoa: a hide published through
           the module is refused, because the signing key is not the creator
           the record names.
      ```

      (from a real `--fresh` run in this worktree). The adapter signs with
      `keystore.stoa_key(&stoa)` at `dialectica/rust-lib/src/lib.rs:539`, names the
      creator through `creator_key_in` at `:684` — which is `identity_public_key` — and
      `ci.yml:367-377` carries a named exemption calling it *"three derivations for one
      user"*. So the brief's sentence is false of the code, and the *"do not design as
      though the creator and the poster could be different people"* instruction tells a
      designer to design against the one state the system is actually in.

      **The brief is unchanged on this branch** (`git diff origin/main...piece/seed-store
      -- docs/` is empty) and was last touched at `53f08b3`, so the sentence is current.
      The claim has no PLAN.md source — `grep "the same key the user posts under"` over
      `docs/PLAN.md` returns nothing — so the brief is asserting it on its own authority.

      **Why this is this change's finding.** CLAUDE.md is explicit: the brief is *"a live
      document […] designed against"*, and *"if a change makes it wrong, fix it in the
      same change; a stale brief is worse than none"*. This change did not make the brief
      wrong — the gap predates it — but it is **the change that established the fact and
      wrote it down in three places**, and it left the one document an external designer
      reads still saying the opposite. A finding is the right outcome rather than silent
      deferral, because the deferral is what has happened so far.

      The edit is small and does not require resolving the spec question: the brief
      should say that today the recorded creator and the signing key **differ**, that
      moderation therefore does not bind, and that this is a known gap under review —
      keeping the design instruction ("do not design as though they could be different
      *people*", which is still the intent) while dropping the factual claim that they
      are the same *key*. Cite `ci.yml`'s exemption and this change's `design.md` so the
      next person finds the reasoning rather than re-deriving it.
      **Severity: medium — a live document contradicting the change that documented the
      contradiction, in the section a designer reads before drawing the creation screen.**

      **Fixed**, and along the lines you set: the factual claim goes, the design
      instruction stays.

      **Grepped for citers before touching the prose**, because a false claim here
      has been the stated reason for a deferral elsewhere and removing one has
      stranded a deferral before. Nothing outside the brief and this findings file
      cites the sentence — checked across `docs/`, `openspec/` and `dialectica/`.
      Confirmed your PLAN.md point too: it is asserted on the brief's own
      authority, so there is no disagreement to resolve and PLAN.md does not need
      to win one.

      What the section now says, in the order you proposed:

      - **Constraint 2 still holds and is separated from the false part.** One
        person, one identity, one root secret — that was never wrong. What does
        not hold is that creator and poster are the same **key**.
      - **The consequence is named**: a hide against a Stoa this peer created is
        refused, because the signing key is not the creator the record names.
      - **It is a known gap under review, not an intention**, citing `ci.yml`'s
        named exemption ("three derivations for one user") and this change's
        `design.md`, so the next person finds the reasoning rather than
        re-deriving it.
      - **The design instruction survives verbatim in intent**: do not design as
        though the creator and the poster could be different *people*.
      - **One thing added beyond your sketch**, because the brief's own constraint
        2 makes it the sharper obligation: do not build a screen that *asserts*
        the user moderates what they created. The brief already treats "the
        interface must not tell a user they have a property they do not have" as
        the one failure that could actually harm someone, and this is an instance
        of it. Say so if you read that as overreach.

      The neighbouring claim at `:292-294` — the creator's key comes from the
      user's own keystore and cannot be supplied — was checked and left, because it
      is still true: it is a different derivation off the same root, not a
      different person's key.

- [x] **`dev-writer`** — `seed_store.rs:644-649` re-derives the probe's address instead of
      calling `wire::posting_identity`, which is an unrecorded decision against the
      pattern `keystore.rs` exists to forbid
      **Scenario:** `design.md`'s "Both author addresses are printed" entry says the
      seeder **"matches the module at each position"** — and for two of the three
      positions it does so through the named `core` function. The probe's position is the
      exception: it is re-derived by hand.

      ```rust
      let posting_address = keystore.stoa_address_at_path(&address, recorded);
      ```

      `wire::posting_identity(stoa, keystore, paths)` (`wire.rs:318`) is `pub`, takes
      exactly the three values in scope, and is the function `get_capabilities` calls at
      `:365`. **Verified they agree today** — I ran both against a seeded store and got
      the identical address, `f7703b699e6c1627…` from each.

      **The agreement is the problem, not the reassurance.** This is the precise shape
      `keystore.rs:448-453` was written to eliminate:

      > The fix for that named the same accessor in two places. **Two call sites that
      > agree is not the same thing as one derivation**, and the two sites were in
      > `cfg(logos_scaffold)` code that `cargo test` does not compile […] so re-diverging
      > them restored the bug with every gate green.

      The seeder is now a third site, in `examples/`, which no test compiles the *logic*
      of and nothing runs in CI — the same "no gate reads this file" property that let the
      original defect come back. `wire::posting_identity`'s own docstring records that the
      choice was **moved into `core` specifically so a test could reach it**
      (`wire.rs:302-309`); re-spelling it here moves one copy back out.

      Note this differs from the two positions the correctness reviewer verified as clean:
      the creator goes through `identity_public_key`, which `creator_key_in` also returns,
      and the signing key through `stoa_key`, which the adapter also calls — both are
      single accessors with no composite function to bypass. The probe is the one position
      that **has** a named composite function, and it is the one bypassed.

      Two ways to close it, and either wants recording. **Call `wire::posting_identity`**,
      which is one line, keeps the change inside `examples/`, and makes "matches the
      module at each position" literally true rather than coincidentally true. Or, if
      there is a reason to re-derive — the only one I can construct is that it makes the
      seeder's dependency on the identity record explicit — **record it in `design.md`**
      with that reason and its cost, which is that the probe's derivation now has a third
      call site that no gate compares to the other two.
      **Severity: medium — an unrecorded choice against a named repo pattern, in the one
      position where a composite function exists to prevent it.**

      **Fixed** by taking your first option: the seeder calls
      `wire::posting_identity(&address, &keystore, &paths)`. The hand derivation
      and the `recorded` binding it needed are both gone.

      **It closed `spec-test.md` entry 1 as a side effect, which neither finding
      predicted and is the more valuable half.** That entry's `assert_ne!` compared
      two values this example derived itself, so it asserted a property of Ed25519
      derivation rather than of the module. Routing the left operand through
      `posting_identity` puts a value the *module* produces on one side — so the
      same mutation the spec-test reviewer ran now fails the seeder instead of
      passing it. Run, not reasoned: `wire.rs:324` changed to
      `keystore.stoa_public_key(stoa).address().to_hex()`, and the seeder panicked
      at the `assert_ne!` with both operands printed identical and the message
      naming what to delete. `wire.rs` restored; `git diff --stat` shows it
      untouched.

      One correction to your scenario, which does not change the conclusion:
      `posting_identity` returns `Result<String, String>` rather than an address
      type, so `signing_address` is now compared as hex on both sides. The error
      arm is mapped into the program's `why`-style prefix, which preserves the
      property the hand derivation's `ok_or_else` was there for — an unreadable
      path fails here rather than on screen.

      `design.md`'s "Both author addresses are printed" entry now records the
      choice with your `keystore.rs:448-453` quote as the reason, and names the
      rejected alternative — keep the hand derivation and record why — with what
      ruled it out: `posting_identity` reads the recorded path itself, so it makes
      the dependency on the identity record explicit too, and it makes "matches the
      module at each position" literally true rather than coincidentally true.

## What is in good shape

**The crate choice is the best-argued entry and its deciding reason is true.** I hit it
before reading it: `cargo test` on the outer manifest failed in a fresh worktree with
*"failed to read .../dialectica/logos-rust-sdk-src/Cargo.toml"* and only ran after I
symlinked the staged SDK from the main checkout, while `cargo run --example seed_store`
against `dialectica-core` needed no staging at all. Three reasons, ranked, with the
deciding one named as deciding — this is the shape the rest of the section should be
judged against.

**"It refuses an existing store" is complete by the four-part test.** What was chosen
(refuse), the constraint (`identity.key` holds the root secret in one place), both
alternatives with what ruled each out (overwrite destroys an unrecoverable file and
routes around `Keystore::create`'s own guard; merge is four behaviours and makes the
assertions meaningless on a second run), and the cost stated plainly — `--fresh` deletes
before it checks, which `correctness.md` entry 4 forced into the help text rather than
letting it stay implicit.

**The two self-invalidating assertions are recorded as decisions, not just written.**
`design.md` explains for both `posting_address != signing_address` and
`!moderators.contains(&founder.public_key())` that the failure is good news and names
what to delete, and the assertion messages carry the same. A decision that pins itself
to the state that justified it is the rare shape that cannot go quietly stale — which is
exactly why finding 1 stands out: the one entry that *does* depend on external state
(`piece/thread-read`) is the one with no such pin.

**"No CI change, and that was measured rather than assumed" is the right kind of entry**
— a planted type error, both gates failing, and an explicit statement of what the gates
cannot see. `architecture.md` entry 2 correctly adds the third gate it omits; that is a
completion of a good entry, not a defect in it.

**The asymmetry between the founder and the visitor is recorded with its reason** — a
peer holds one root secret, so a second local identity is not a state the module can be
in, and a generated visitor reproduces "an op that arrived from a peer". That is a
choice a reader would plausibly have made differently (two keystores), and the rejected
alternative is named.

**`design.md` carries no test counts, versions, or other command-answerable state**,
which is what CLAUDE.md asks. Finding 4 is the single exception, and it is a count of
types rather than a count of tests.
