# Architecture — `core-e2e`

Reviewed: `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs` (1,483 lines,
20 tests) and the `.github/workflows/ci.yml` test-count gate. Dimension:
**architecture only** — readability is in `readability.md`, correctness and
security are another instance's.

Judged against the question the brief asks: this file will gate every future
piece, four are in flight, and each will want to add to it. All measurements were
taken in a throwaway worktree and reverted; the tree was clean at commit time.

## Defects

- [x] **`dev-writer`** — `.github/workflows/ci.yml:699-703` — the hand-maintained
      `roots` list is the exact shape the project has already been bitten by, and
      there is a shape that cannot go stale
      `roots` is now three entries. The next integration target that is not
      `dialectica-core/tests` escapes it silently — and the failure it produces is
      the one the new comment itself calls the misleading one: `ran` grows,
      `declared` does not, and the job dies with a message about cfg-gating. The
      outer crate (`dialectica/rust-lib/`) has no `tests/` today, so
      `dialectica/rust-lib/tests/` is the concrete next escape; so is a second
      file under a new `dialectica-core/benches` or a third crate.
      **What becomes hard:** every future piece that adds an integration target
      must remember to edit a Python list in a 1,000-line workflow, and the
      penalty for forgetting is a red build blaming the wrong thing. This is the
      `every_request_taking_method` failure the brief names — a list of five that a
      sixth silently escapes — reproduced one commit after the list grew for the
      first time.
      **The shape that cannot go stale:** there is exactly one `target/` in the
      tree (`find … -maxdepth 2 -name target -type d` →
      `dialectica/rust-lib/target` only), so a single
      `pathlib.Path("dialectica/rust-lib").rglob("*.rs")` with
      `if "target" in p.parts: continue` covers `src/`,
      `dialectica-core/src/`, `dialectica-core/tests/` and every future `tests/`,
      `benches/` or crate, with no list to maintain. The comment's two stated
      objections to an rglob do not survive: walking `target/` is a one-line
      exclusion, and "it would keep passing if the inner crate were moved out" is
      already covered by the `declared == 0` guard plus the `ran != declared`
      mismatch — a moved crate changes `ran` too.
      **Severity: medium.** A gate whose staleness mode is a misleading red is
      better than a silent green, but it is worse than a gate with nothing to keep
      in sync.

      **Fixed** in `5a1719b`, adopting your `rglob` with the `target` exclusion
      essentially as written.

      **Your two rebuttals were the deciding argument, and both hold.** I verified
      the single-`target/` claim (`find -maxdepth 4 -name target -type d` returns
      only `dialectica/rust-lib/target`), and the "moved crate" objection really is
      already covered — moving a crate out of the `-p` set changes `ran`, so the
      mismatch fires regardless. The workflow comment now states both rebuttals,
      so the next person tempted back toward a list meets the argument rather than
      the original objections.

      **Measured before and after, on the merged tree:** rglob 530, three roots
      530, cargo `ran` 530 — behaviour-preserving today, strictly broader tomorrow.

      One point in the finding's favour that arrived after you wrote it: `main`'s
      wire-request piece landed `dialectica-core/src/wire/request.rs`, a **new
      nested directory** carrying 6 tests. The old list happened to cover it, being
      under `dialectica-core/src` — but it is a concrete instance of the tree
      growing a directory nobody updated a list for, one merge after you predicted
      the shape.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:702` — `examples/` is excluded
      by a rule that does not hold, and the exclusion is load-bearing for the
      rglob fix above
      If `roots` becomes an rglob (previous entry), `examples/` comes in with it —
      and then the gate really would fail on a `#[test]` in an example, which the
      comment claims (falsely, today) that it already does. Decide which: either
      `examples/` is in scope and the claim becomes true, or it is out of scope and
      the paragraph must say the gate cannot see it. **This box is separate from
      the readability box on the same lines** because it is a design decision
      (which set of files the gate measures), not a wording fix.
      **Measured:** with `dialectica-core/examples/probe.rs` holding one `#[test]`,
      the CI invocation ran 495 and declared 495 — green, with the example's test
      never executed. Reverted.
      **Severity: low** on its own; it is the decision the fix above forces.

      **Fixed** in `5a1719b`. **Decided: `examples/` is IN scope, so the claim
      becomes true** rather than the paragraph being rewritten to admit the gate
      cannot see it.

      You were right that the rglob forces the decision, and right about which way
      it should go. A `#[test]` in an example is a test that cannot run, which is
      worse than no test — so a gate that fails on it is doing its job, and the
      alternative (documenting the blind spot) leaves a real hole in exchange for
      an honest sentence.

      **Measured on the merged tree with your `probe.rs` restored:** `declared`
      531, cargo `ran` 530, and the example's test in no `Running` line — so
      `ran != declared` fires. The same probe under the old list left both numbers
      at 530, reproducing your green. Both runs recorded in `tasks.md` §3.2.

      This box and the readability box on the same lines were one change; I have
      answered them separately because, as you say, they are a design decision and
      a wording fix.

- [x] **`tester`** — `end_to_end.rs:1225-1233` and `1278-1287` — the over-cap test
      hand-rolls `TempDir::store()` and `TempDir::reopen()` because the helpers
      hardcode one filename, and that is the fourth-guard signal
      `TempDir::store()` (line 175) and `TempDir::reopen()` (line 183) both
      hardcode `"ops.sqlite"`, so a test needing **two** stores in one directory
      cannot use either. `an_over_cap_body…` therefore writes
      `SqliteOpLog::open(&dir.file("at-cap.sqlite"))` three times and
      `drop(store)` inline — a second, divergent copy of the restart primitive the
      file's own doc calls out as load-bearing (*"this is what 'a restart' means
      here… not a flag, not a method on the store"*, line 180).
      **What becomes hard:** the moment a piece needs two stores — a publish path
      writing to one store and reading from another, or a membership test with two
      peers' stores — it will copy the over-cap test's inline form rather than the
      helper, because the over-cap test is the only precedent for it. The project's
      own memory says the unfixed pattern is the template the next test is written
      against, and `log/mod.rs:520-528` records this exact cost once already
      ("leaving the weaker copies here made them the template the next test would
      be written against").
      **The reshape is small and is the CLAUDE.md-preferred one** (complexity in
      the data structure, not a fourth branch): make the filename a parameter —
      `fn store_at(&self, name: &str) -> SqliteOpLog` and
      `fn reopen_at(&self, store: SqliteOpLog, name: &str)`, with `store()` /
      `reopen()` as one-line wrappers passing `"ops.sqlite"`. Every existing call
      site is unchanged and the over-cap test loses its private copy.
      **Severity: medium** — it is the one place in the file where a test that
      needed a slightly different fixture could not get one and copied instead,
      which is precisely the question the brief asks.

      **Fixed** in `1342aa9`, adopting your shape with one addition:
      `store_at(name)` and `reopen_at(store, name)`, with `store()` / `reopen()` as
      one-line wrappers — plus a `CONVENTIONAL_STORE` constant rather than the
      literal `"ops.sqlite"` repeated in both wrappers, since the wrappers'
      only job is to supply that one value and it should exist once. Every existing
      call site is unchanged, as you predicted, and the over-cap test has lost its
      private copy of the restart primitive.

      **Verified the reshape did not weaken what the inline form proved:** the
      `:memory:` mutation still kills the over-cap test, so the at-cap round trip
      and the poisoned-read claim are still claims about files rather than about
      process memory.

      Your `log/mod.rs:520-528` citation is the part that made this a fix rather
      than a judgement call — the cost of leaving a weaker copy in place is
      recorded in this crate rather than hypothetical. Carried into `design.md`,
      along with the two-store cases you named (a publish path writing to one store
      and reading another, a two-peer membership test), so the reason the parameter
      exists outlives this tracker.

      The fixture's doc comment now explains why the filename is a parameter, using
      the over-cap pair as the worked example, so the next author meets the reason
      at the helper rather than having to infer it.

- [x] **`tester`** — `end_to_end.rs:1096-1108` — the shared-prefix fixture uses a
      31-byte prefix, contradicting a recorded, argued decision that says 16
      `log/fixtures.rs:119-135` defines `SHARED_PREFIX_BYTES = 16` with a
      documented rationale for that exact number, and it explicitly rejects the
      value this file chose: *"16 rather than 31: it is comfortably past any
      plausible accidental truncation … A fixture agreeing on 31 of 32 bytes would
      test the same property and read as a puzzle."* The new test agrees on 31 of
      32 bytes.
      The duplication itself is **forced and not a finding** — `log::fixtures` is
      `pub(crate)` (`log/mod.rs:501`) and an integration test genuinely cannot
      reach it, which the file's header correctly argues is the point. What is a
      finding is diverging from the recorded reasoning without naming it: a reader
      comparing the two now sees two prefix lengths, two rationales, and no way to
      tell which is current.
      **What becomes hard:** the next prefix-confusion fixture — and there will be
      one, `iter_target` has the same shape — has two conflicting precedents to
      copy from. A cross-reference costs one sentence:
      *"31 rather than `log::fixtures::SHARED_PREFIX_BYTES`'s 16, because an
      integration test cannot import it and an inline literal is clearer than an
      unexplained 16"* — or just use 16 and say where the number comes from.
      **Severity: low** — genuine divergence, no behavioural cost.

      **Fixed** in `1342aa9`, taking your second option — 16, with a sentence
      saying where the number comes from. Following the recorded decision beats
      documenting a divergence from it when the recorded decision is right, and it
      specifically rejects 31 as "reading as a puzzle", which this fixture was.

      The fixture is now a local `SHARED_PREFIX_BYTES = 16` with the divergence at
      exactly byte 16, mirroring `log/fixtures.rs`'s "agrees on N, differs at N"
      guarantee rather than only its value — including the second fixture guard,
      which the 31-byte version did not have.

      **Verified the weaker-looking fixture is not weaker.** Mutating `iter_stoa` to
      compare `substr(stoa, 1, 8)` — the truncation the recorded rationale names as
      the thing 16 is chosen to catch — still leaks, killing the test with
      `["right","left"]` vs `["left"]`, the same signature the 31-byte fixture
      produced. So the change costs nothing in strength and removes the puzzle.

      Your "forced duplication is not the finding, the disagreeing rationale is" is
      the distinction that made this actionable, and it is the one recorded in
      `design.md`: duplicating a value across a `pub(crate)` boundary is fine;
      duplicating it while inventing a second rationale leaves a reader with two
      precedents and no way to tell which is current. You also called `iter_target`
      as the next fixture with this shape — noted there too.

- [x] **`dev-writer`** — `end_to_end.rs:1-10` — the file is titled "The read path,
      end to end" and stops one layer below the module's actual public surface,
      without saying so
      `wire.rs` **is** on `origin/main` (`git log -1 origin/main --
      dialectica/rust-lib/dialectica-core/src/wire.rs` → `0538c0d`), it is a
      `pub mod` (`lib.rs:29`), and `wire::list_threads_from_request`
      (`wire.rs:414`) is the JSON in / JSON out boundary CLAUDE.md calls *the
      deliverable* — *"the core module's API is the part of this project to be most
      deliberate about"*. Every test in this file stops at
      `feed::list_threads`, one layer below it.
      That layer is not empty of cross-layer behaviour. `wire.rs:435-442` maps a
      failed store *open* to `{"error":…}` rather than to an empty feed, citing
      §11.1 obligation 5 — the same obligation
      `a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed`
      cites at line 577, and asserts one layer too early: it proves
      `feed::list_threads` returns `Err`, not that the JSON a view receives is the
      error shape. The seam between `Err` and `{"error":…}` is exactly the
      encode/decode-asymmetry class this file exists to catch.
      **What becomes hard:** the publish path and `list_stoas` will arrive as
      `wire` handlers (PLAN.md:3485 shows `listStoas()` in the JSON contract), and
      whoever adds them has no precedent here for crossing the JSON boundary — so
      they will either add it below `wire` like these twenty, or invent the shape
      alone.
      **Two actions, and the second is the cheap one:** add one test that reaches
      `wire::list_threads_from_request` over a real file so the JSON boundary has a
      precedent; and, regardless, **give the header an explicit "what this target
      does not cover" section** naming `wire`, the publish path, `list_stoas` and
      membership. The absent ones are legitimately absent — `grep -rn
      "list_stoas\|listStoas"` over `dialectica/` finds nothing but a PLAN.md line
      — but the file currently neither covers them nor says it does not, and "end
      to end" in the title reads as though it does.
      **Severity: medium.**

      **Fixed** in `1342aa9`, doing **both** actions rather than choosing — and the
      first turned out to be the one that mattered, not the cheap second.

      **Two tests now cross `wire::list_threads_from_request`**, in a new section
      whose heading names the boundary: one happy path (a JSON request string in, a
      JSON reply carrying a body that appears nowhere in the request) and one
      failure. Both over a real file, since `wire.rs`'s own unit tests already cover
      the handler against an in-memory log; what they cannot do is prove the JSON a
      view receives reflects a store on disk.

      **Your claim that the seam is real, measured.** Mutating the handler to return
      `{"items":[],"page":0,"hasMore":false}` instead of `error_json` on a failed
      store open kills the new test — **and leaves
      `a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed`
      green**, which is the test you identified as asserting one layer too early.
      So the gap was not theoretical: §11.1 obligation 5 is an obligation about
      what a *reader* sees, and it could not be discharged at the layer the file
      stopped at. That measurement is why extending beat renaming, and it is
      recorded as the decision in `design.md`.

      The second action is done too: the header opens with an explicit **"What this
      target covers, and what it does not"** naming the publish path (every test
      here is a read), `list_stoas`, membership, transport and the view — with your
      grep result quoted, since "absent because it does not exist" and "absent
      because nobody covered it" are different facts and the reader should not have
      to infer which.

      One correction to the brief that sent me here, worth recording since it went
      the other way twice: an earlier brief told the author the wire envelope was
      out of reach. Your `git log -1 origin/main -- wire.rs` is right, and it is
      more true now than when you wrote it — the wire-request piece has since
      merged, so `wire/request.rs` and a promoted `module-wire-contract` spec are
      both on `main`.

- [x] **`dev-writer`** — `end_to_end.rs:448`, `638`, `746`, `997`, `1201` — the
      by-boundary section scheme has no home for the pieces in flight, and one
      section is already ambiguous
      The five sections are *the whole chain*, *empty versus unreadable*, *what a
      reader refuses*, *moderation across the store boundary*, *persistence
      properties about a FILE*, *hostile input from the public API*. Organising by
      boundary crossed rather than by capability is the right call and I am not
      asking for it to change. But three of the six are defined by a *property*
      (persistence, hostile input, refusal) and three by a *subsystem*
      (moderation, the chain, open-time failures), so a new test can belong to two
      at once — and one already does: `a_forged_hide_does_not_displace_the_genuine_one…`
      sits under "Moderation across the store boundary" while being just as much
      an "input a reader refuses" test, and `an_over_cap_body…` sits under "hostile
      input" while being the file's strongest persistence claim.
      **What becomes hard:** `list_stoas` (a read over genesis records, no store
      boundary), membership (authority, like moderation) and the publish path (a
      write, which no section covers — every section here is a read) each have no
      obvious home, and the publish path has none at all. With four pieces in
      flight adding to one file, "where does this go?" answered differently four
      times is how a 1,483-line file becomes 3,000 lines nobody can navigate.
      **The cheap fix is a paragraph, not a reorganisation:** state the sectioning
      rule in the header — one sentence saying sections are *the boundary the test
      crosses*, that a test belonging to two goes under the one it would fail at
      first, and that a write path gets its own section when one arrives. A rule a
      later author can apply beats five headings they have to reverse-engineer.
      **Severity: low** — a structural risk, not a present defect.

      **Fixed** in `1342aa9`, taking the cheap fix you recommended: a stated rule in
      the header, no reorganisation. Three clauses rather than one sentence, because
      the third earns its place independently:

      1. Sections are the **boundary crossed**, not the capability — with the reason,
         that this crate's defects live in seams and organising by capability hides
         seams.
      2. A test belonging to two sections goes under the one it would **fail at
         first**, which is your tie-break and resolves both ambiguities you named.
      3. A heading **states the boundary and never a count** — the readability
         finding about "three boundaries" over four, generalised so it cannot recur
         in a new section.

      **Not adopted as written: "a write path gets its own section when one
      arrives."** The rule is the more general "a test crossing a boundary no section
      names gets a new section", with the write path named as the known instance.
      Same outcome for the publish path, but it also answers `list_stoas` — a read
      over genesis records with no store boundary — which the more specific phrasing
      would have left homeless in exactly the way this finding describes.

      The wire section added for the finding above is the first test of the rule, and
      it took the rule rather than an argument: it crosses a boundary no existing
      section named, so it got one.

## Clean

- **No new dependency.** The test reaches `rusqlite::Connection` directly
  (`end_to_end.rs:628`), and `rusqlite` is already a normal (not dev) dependency
  of the crate at `dialectica-core/Cargo.toml:170`. Nothing was added to either
  manifest, so there is no licence or maintenance question to answer.
- **No implementation file is touched**, which the diff confirms: `git diff
  --stat origin/main...HEAD` is `ci.yml | 18 +-` and
  `tests/end_to_end.rs | 1483 ++++`. A test-only piece is the right shape for
  what this is.
- **Not reaching `log::fixtures` is correct and correctly argued.** The header's
  §1 reasoning — that a unit test borrowing a private fixture proves nothing about
  the public surface — holds, and `fixtures` being `pub(crate)` makes the
  separation structural rather than a convention.
- **The restart primitive is the right abstraction.** `TempDir::reopen` being a
  dropped connection and a reopened path, rather than a flag on the store, is the
  one thing that makes the persistence claims about a file; I verified it is
  load-bearing by mutating `SqliteOpLog::open` to `Connection::open_in_memory()`
  and watching 18 of 20 fail.
- **`TempDir` cleaning up in `Drop` and naming per test** is right, and the
  per-test naming really does prevent the shared-store race it claims to.
- **CI's other gates are unaffected.** The panic-guard step
  (`ci.yml:745-759`) reads `dialectica-core/src/wire.rs` by path, which this change
  does not move; no gate measures a directory that stopped holding tests.

## Not a finding, but my read on the deferred defect

The brief says whether to pin the over-cap defect as-it-stands rather than fix it
is the runner's call. Pinning it is right, and the test is written the way a
deferred defect should be: the assertion names the current outcome, the panic
message names what should replace it, and the at-cap half means the fencepost is
already covered when someone does fix it. The one thing I would want before merge
is a pointer from the test to wherever the defect is *tracked* — a test is a good
place to reproduce a known bug and a bad place to be the only record that it
exists.

**Answered (not a box, so nothing to tick).** The runner confirmed pinning it
as-it-stands is their call and they are keeping it. Your one request could not be
satisfied, so the file now says why instead: **the defect is tracked nowhere.** I
checked both places it could be — `gh issue list --state all` on the repo returns
nothing (it has no issues at all), and `docs/PLAN.md` does not mention the
asymmetry. Rather than leave that implicit, the header now states it in as many
words, says a test being the sole record is the wrong place for one, and names what
should replace the paragraph once somebody files it, with `Op::canonical_bytes` and
`MAX_FIELD_LEN` as the search terms in the meantime.

Reported to the runner, who said they would file it if nowhere did.

## Paperwork

There is no `openspec/changes/core-e2e/` on the branch — no `proposal.md`, no
`specs/` delta, no `tasks.md`. I created the `findings/` directory for these two
files. **Consequence for the runner: there is no stage block, so there are no
`code-reviewer` rows to tick** — I could not tick mine, and an unticked row
cannot be the signal that a dimension is unreviewed for this piece. Whoever owns
the piece needs to decide whether a test-only change gets a `tasks.md` stage block
or whether the review record lives only in these two files.

**Answered, and the decision went your way.** `.claude/agents/README.md` now says a
piece with no behaviour change still gets a change folder and a stage block, with
the spec row **struck through carrying its reason** rather than omitted — because
"does not apply" and "nobody did this" are different states and the block exists to
tell them apart. Your report is cited as the first instance: a reviewer with no row
to tick, saying so.

Created in this piece: `openspec/changes/core-e2e/` with `proposal.md`,
`design.md`, `tasks.md` and `.openspec.yaml`. The stage block has your two rows
ticked (this file and `readability.md` are the evidence, committed in `df91185`) and
the correctness/security/spec-test/design rows unticked, which is now doing the job
you said was missing — one of those reviewers is still running, and the rest have
not happened.

`openspec validate core-e2e --strict` passes. One trap for the next test-only piece,
recorded in `design.md`: `skip_specs: true` needs a `schema: spec-driven` key beside
it, or it is **silently not honoured** and the validator reports two errors for one
problem.

I have not ticked the `tester` row. This piece's code is tests, but they were
written by `dev-writer` alongside the target, and the `tester` stage asks a
different question from the spec rather than the code — ticking it would be the
false statement the row exists to prevent.
