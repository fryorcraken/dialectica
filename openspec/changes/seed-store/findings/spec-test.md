# Spec-test findings: the store seeder

Reviewed `piece/seed-store` at `c166a99` from a worktree of its own
(`review/seed-store/spec-test`), against the **spec suite and the tests only**.
The implementation was read at exactly two points, both forced: there is no test
file in this piece to read instead (the diff adds none), and the mutation below
necessarily edits code.

Suite in this worktree: **733 + 26 = 759 passed, 0 failed**, identical to the
merge base `733544d`. Measured: `git diff --stat 733544d piece/seed-store`
touches `examples/seed_store.rs` and eight `openspec/` files — **no file under
any `tests/` directory, and no `#[test]` added anywhere.**

**The `skip_specs: true` marker is correct and I verified it rather than
accepting it.** Every symbol the seeder reaches is already-contracted public
API — `authoring::{post,reply,vote}`, `MembershipStore::{open,join}`,
`IdentityStore`, `Keystore::*`, `feed::list_threads`, `OpLog::get`,
`Moderators::of`, `SqliteOpLog::open` — spread across the `content-authoring`,
`stoa-membership`, `keystore`, `identity-onboarding` and `module-wire-contract`
capabilities that already exist in `openspec/specs/`. The example adds no
dispatch handler, no method, no field and no reply shape. There is no scenario
to cover because there is no requirement, so **part 1 of this review is vacuous
by construction and that is the right outcome**, not a gap.

That makes the whole of this review reduce to part 2 — *can the checks this
piece does ship actually fail?* — and the answer for the flagship one is no.

---

- [ ] **`dev-writer`** — `seed_store.rs:673-678` and `design.md:141-144` — the
      `assert_ne!(posting_address, signing_address, …)` **cannot fail the day the
      gap closes**, which is the one thing `design.md` claims for it. It asserts a
      property of Ed25519 key derivation, not a property of the module.
      **Scenario:** `design.md:141-144` states the assertion *"**fails the day the
      gap closes**, and the failure message says so … That is the self-invalidating
      shape CLAUDE.md asks for: a note about present state that cannot go quietly
      wrong."* The assertion's own message tells a future reader the same thing:
      *"the probe and the publish path have stopped disagreeing — the
      three-derivations gap is closed, so this assertion and the paragraph it
      documents should both go."*

      But the two operands are not the module's two positions. They are two
      *keystore* calls made by the example itself:

      - `posting_address = keystore.stoa_address_at_path(&address, recorded)`
        → `derive_stoa_key_at_path(root, stoa, path)` (`keystore.rs:870`)
      - `signing_address = keystore.stoa_public_key(&address).address()`
        → `derive_stoa_key(root, stoa)` (`keystore.rs:803`)

      Those are **two different derivation functions on the same root**, so they
      differ for the same reason any two HD paths differ. Nothing the module does
      is on either side of the comparison. This is precisely the recorded defect
      family in `.claude/agents/README.md` — *"mutating a byte and asserting a
      hash moved — a property of SHA-256, not of the encoding"* — with
      `derive_stoa_key` in place of SHA-256.

      **Measured, by closing the gap and watching nothing notice.** The gap is
      that `wire::posting_identity` reports `stoa_address_at_path` while the
      publish path signs with `stoa_key`. I closed it at the probe
      (`wire.rs:324`):

      ```rust
      -        Ok(Some(path)) => Ok(keystore.stoa_address_at_path(stoa, path).to_hex()),
      +        Ok(Some(_path)) => Ok(keystore.stoa_public_key(stoa).address().to_hex()),
      ```

      `getCapabilities` now reports the signing address — the gap the whole
      paragraph documents is gone. Ran the seeder on a fresh directory. It
      **exited 0**, every assertion passed, and it printed, unchanged:

      ```
      author addresses, which do not agree — this is the known gap:
        getCapabilities reports  0cc24ef0…
        every seeded op is by    cd867692…
        => MODERATION DOES NOT WORK on a seeded Stoa: …
      ```

      So on the day someone closes this gap, the seeder does not fail, does not
      name what to delete, and goes on printing a paragraph about a gap that no
      longer exists — the exact "quietly wrong" outcome the design section says
      the shape prevents.

      **What makes this worth a box rather than an observation:** the same
      mutation *was* caught, immediately, by a pre-existing test from an earlier
      piece — `wire::tests::the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa`
      (`wire.rs:4848`) failed. So the property is testable and is already tested;
      what is wrong is that this piece's headline assertion claims credit for
      pinning it and does not. A reader who trusts `design.md:141` will believe
      the seeder guards a gap that `wire.rs`'s suite is actually guarding.

      **The fix does not need new API.** Compare the two values the module really
      uses at its two positions — e.g. assert the feed row's `author` against
      `wire::posting_identity(&address, &keystore, &paths)`, which is the probe's
      own function rather than a re-derivation of it. That operand is produced by
      the module, so closing the gap moves it and the assertion fires.
      **Severity: medium — no live defect, but a documented self-invalidating
      guard that does not invalidate, in the file whose entire argument is that
      its assertions are what make a bad run loud.**

- [ ] **`tester`** — the piece ships **nine assertions that no gate ever
      executes**, and the `tests` row is correctly unticked; this is the box that
      records what the missing stage must cover.
      **Scenario:** `design.md:182-186` is candid that *"nothing **runs** the
      program, so a seeder that compiles and produces a useless store would
      pass"*, and `tasks.md:23-35` sketches the closure. Both are right, and I
      measured the consequence: deleting nothing and changing nothing, the suite
      is 759 tests at both `733544d` and `c166a99` — **this piece moved the test
      count by zero.** Every property it asserts (nine ops, two thread heads,
      three `parent`/`thread` pairs, two author lookups, the moderator pair, the
      inequality) lives behind "a person typed `cargo run --example`".

      That is a structural blind spot on top of the two this repo already
      records, and it is worth stating in the same breath: `cargo test` does not
      compile `cfg(logos_scaffold)` code, and `cargo fmt --check` does not follow
      path dependencies so it never reaches `dialectica-core` at all (confirmed —
      `findings/architecture.md` entry 2 measures it). The seeder adds a third:
      **a target CI compiles but never executes.**

      The concrete shape worth building, and the reason it is not merely
      duplicating the example: an integration test over a `tempdir` in
      `tests/end_to_end.rs` that mints a keystore, founds a Stoa, records a path,
      publishes and reads back would catch the mutation in the box above *and*
      the two false-report claims in `findings/readability.md`, because a test can
      assert the set of distinct authors in the store is exactly two where a
      `println!` cannot. **Do not add a `#[test]` to `examples/seed_store.rs`** —
      the file and `tasks.md` are both right that CI's count gate would fail on it.
      **Severity: medium — the piece's own design document names this gap; the box
      exists so the unticked row has something to point at.**

- [ ] **`tester`** — `seed_store.rs:711-726` — the false "every seeded op is by
      \<one address\>" report line, already filed as high-severity by
      `findings/readability.md`, **had no check that could have caught it, and the
      brief's question is why.**
      **Scenario:** this box does not re-report the defect — `readability.md`
      entry 1 owns it, with the 4-vs-5 op split measured. The spec-test question
      is the one asked of me: *was there an assertion that should have caught a
      false claim in the report, and why did it not fire?*

      The answer is that **the two assertions nearest that line are both
      existential where the claim is universal.** `:665-672` asserts
      `authors.contains(signing_address)` and `authors.contains(visitor_address)`
      — each says "at least one row has this author". The report line says "**every**
      seeded op is by" one address. No `contains` check can ever contradict an
      "every" claim, so the assertions and the false sentence are mutually
      consistent by construction: both are green whether the store has one author
      or two. This is the repo's recorded family exactly — a fixture where two
      explanations give the same answer.

      **What closes it is one assertion, not a reword.** Collect the distinct
      authors across all nine ops via `OpLog::iter` and assert the set has size
      two, naming both. That assertion fails today against the standing report
      line, and it keeps failing if a later change moves an op between identities
      without updating the prose — which is precisely how `c2bf6f5` introduced
      this defect while every existing assertion stayed green.
      **Severity: medium — the check is cheap and the defect it would have caught
      shipped at high severity.**

## What was clean

**`skip_specs: true` is the honest answer, not a workaround, and the reasoning in
`.openspec.yaml:8-19` survives scrutiny.** I checked the alternative it declines
— inventing a requirement so validation passes — and agree it would be worse:
it would put an example's stdout under contract, which is the one part of this
change that should stay free to change. The marker carries `schema: spec-driven`,
without which `skip_specs` is silently ignored; that trap is documented in the
file itself and the file gets it right.

**The stage block is correctly shaped for a piece with no spec.** The spec row is
struck through with its reason rather than omitted, which keeps "does not apply"
and "nobody did this" distinguishable — the property `.claude/agents/README.md`
says catches a missing reviewer. The `tests` row is left **unticked rather than
struck**, with a note explaining what a `tester` could genuinely do. That is the
right call and it is the distinction that made this review's second box
writable.

**No `NO SPEC:` markers, and none are missing.** `grep -rn "NO SPEC"` over
`examples/` returns nothing, which is correct here rather than an omission: the
marker records a dev choosing where a spec was silent, and this piece adds no
behaviour to the contract for a spec to have been silent about. The choices it
does make — `SEEDED_PATH`'s value, the two titles, nine ops — are an example's
fixture data, not unspecified system behaviour.

**PLAN.md on `origin/main` sheds nothing here, correctly.** I checked for a
section this piece answers and there is none — the seeder is a developer tool,
so there is no open question it closes and no future intent it now specifies.
Part 6 of this review is empty by construction.

**No requirements moved between capabilities** (part 4): the diff adds no
`specs/` delta at all, so there is no `REMOVED`/`ADDED` pair to verify.

**The structural assertions that are not the flagship one do discriminate.** The
`parent`/`thread` triples at `:588-598` read back through `OpLog::get` rather
than reusing the `Published` values, and the two author lookups compare against
addresses derived independently of the feed — so unlike the inequality, these
assert against something the read path produced. The count assertions were
measured discriminating by the correctness reviewer and I reproduced the `ops`
one indirectly (the mutation runs above all kept `ops 9`).

## Mutations run, and what each showed

| Mutation | Result |
|---|---|
| `wire.rs:324` `stoa_address_at_path` → `stoa_public_key(stoa).address()`, i.e. **the three-derivations gap closed at the probe** | **Seeder survived — exited 0, all assertions passed, still printed "which do not agree" and "MODERATION DOES NOT WORK".** The pre-existing `wire::tests::the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` **failed**, so the property is testable and is tested — just not by this piece. Box 1. |
| Baseline runs of the seeder on two fresh directories, no mutation | Exited 0 both times; independently reproduced `findings/readability.md` entry 1 — the report's three addresses omit the visitor, who authors the majority of ops and one of the two feed rows. |

**Restored.** `git status --porcelain` in this worktree is empty after the
mutation was reverted, and `cargo test -p dialectica-core --lib` is back to
**733 passed, 0 failed**. The worktree is deleted rather than restored, per the
reviewer protocol.

## What I could not check

**Nothing behind `cfg(logos_scaffold)`.** `cargo test` does not compile it, so
the adapter's own path derivations (`lib.rs`) are outside anything I could
execute. Where I needed the adapter's behaviour — which key it signs with — I
read `wire.rs`, which is compiled, rather than inferring from the scaffold layer.

**Whether the seeded store opens under Basecamp.** That needs `lgs basecamp
launch` and a human click, and no gate in this repo covers it. The architecture
reviewer verified the nearest checkable thing — feeding the emitted request to
`wire::list_threads_from_request` — which is the strongest available substitute
but is not the same claim.
