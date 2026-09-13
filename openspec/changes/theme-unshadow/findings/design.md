# Design findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `6a7cf00`, in a worktree of its own
(`review/theme-unshadow/design`). `design.md` and `proposal.md` were read against
the shipped code; every figure in `design.md` was re-derived by running the gate
rather than by reading the document; `docs/PLAN.md` and `docs/UI-BRIEF.md` were
read from **`origin/main`**, which is four commits ahead of this branch point.

The spec-test findings at `6a7cf00` were read first. Their three high boxes are
**not** re-reported here — a `tester` owns them. What is below is the design
question those findings open but do not answer: a *decision* recorded in
`design.md` rests on a premise the spec-test review measured false, and the
document still asserts it.

## The decisions that were recorded were taken, and the arithmetic holds

This is unusually good and worth saying before the boxes, because the piece's
own history is one of withdrawn claims and it would be easy to assume more rot
than is there.

- **The prefix decision is implemented as recorded.** `qmldir` declares
  `singleton DTheme 1.0 DTheme.qml`, no module URI was introduced, and no
  `import Dialectica.*` line appears in any of the 17 QML files. The rejected
  alternative (a module URI) is recorded *with* what ruled it out — it relocates
  the collision rather than removing it — and with its cost, an import line per
  file plus a build-time module definition. That is a complete entry by the
  four-part standard: choice, constraint, alternatives, cost.
- **`design.md:64-73`'s figures are exact, not stale.** I extracted the gate's
  script verbatim and ran it against a detached worktree at the branch point
  (`733544d`, the tree carrying the defect): **exit 1**, arm 1 firing on the
  un-prefixed `qmldir` singleton, and `grep -c "references a bare"` over the
  output gives **116** — of which `grep -c "src/qml/.*references a bare"` gives
  **113** and `grep -c "tests/tst_identicon.qml.*references a bare"` gives **3**.
  113 + 3 = 116. All three numbers the document commits to are right, and the
  paragraph at `:71-73` explaining that an earlier "80-odd" was wrong is itself
  accurate. The command is the gate's own script run against `733544d`.
- **`design.md:145`'s "all 17 QML files" is right**, and the disagreeing figure
  is in `ci.yml`, not here — see the box below. Running the gate against this
  tree prints `ok: 17 QML file(s) checked`.
- **The list-to-convention rewrite is implemented.** `GRANDFATHERED = {"Core"}`
  with the reason inline, and arm 1 is `re.fullmatch(r"D[A-Z][A-Za-z0-9]*", name)`
  — a rule, not an enumeration. `design.md:75-95` records the constraint (the
  `hand-maintained sweep lists go stale silently` trap), the rejected alternative
  (five known host names), and what ruled it out (total over no future
  registration). It also records the cost honestly: the convention is a
  convention, and `Core` is an exemption with an expiry nobody can see.
- **The comment-stripping decision matches the code.** `strip_comments` runs once
  before every arm, `/* */` is replaced by the newlines it spanned so line
  numbers stay honest, and `design.md:97-114`'s stated reason — one place to be
  right about what a comment is — is the shape actually shipped.
- **The withdrawn-premise record is the best thing in the document.** A premise
  held, measured false twice, and written down *as* withdrawn rather than quietly
  dropped, in four places that agree with each other. That is the entry that
  stops the next person spending an afternoon building a reproduction that cannot
  work, and it should survive the edits the boxes below ask for.

Neither `docs/PLAN.md` nor `docs/UI-BRIEF.md` on `origin/main` is contradicted or
invalidated. `git grep -n -i "Theme\.\|qml type\|DTheme\|colour token" origin/main
-- docs/UI-BRIEF.md docs/PLAN.md` returns nothing; the brief names no QML type and
no singleton, because it is written for a designer who cannot read the code. The
two PLAN.md hits for `singleton` are about the core module's Rust singleton and
`createNode`, neither of which this change touches. `docs/IDENTICON.md` **is**
invalidated by the rename and **is** fixed in the same change — six `Theme.` →
`DTheme.` edits, including the frozen-palette obligation at "Where the palette
lives". No reasoning was left stranded in PLAN.md by this change, because this
change acted on nothing PLAN.md explained.

---

- [ ] **`dev-writer`** — `design.md:52-62` records a **decision** — that the
      static gate is the instrument, because the component layer is
      "structurally unable to see this defect" — and the second half of that
      premise is false, so the decision was made on a false premise and the
      record must say so
      `design.md:59-62` reads: "a QML test is not merely absent here; it is
      **structurally unable to see this defect**, and writing one would be the
      false green CI is built against. The gate is therefore the static `no QML
      type name collides with the host` step in `ci.yml`." The "therefore" is
      what makes this a decision and not an observation: the impossibility is
      the stated reason the gate took the shape it did, and the reason no third
      instrument was looked for.
      The premise is true of **the host collision** and false of **the undefined
      tokens the collision produces**, which is the class the gate is actually
      trying to catch. The spec-test review disproved it; I re-ran both bounds
      myself rather than relaying, because a persuasive citation here is exactly
      the one to check:
      - `qmllint --unqualified disable --missing-property error -I <src/qml>`
        over all 13 `src/qml/*.qml` on **this unmutated tree** → **exit 0**, no
        output. It does not redden the tree.
      - the same invocation over a scratch copy with `Main.qml:36` changed from
        `color: DTheme.desk` to `color: DTheme.noSuchDesk` → **exit 255**,
        `Error: Main.qml:36:23: Member "noSuchDesk" not found on type "DTheme"
        [missing-property]`.
      - the **current CI invocation** (same flags minus `--missing-property
        error`, `ci.yml:561-562`) over that same mutation → prints the identical
        line as `Warning:` and **exits 0**. The step gates on `rc -ne 0`, so it
        is green.
      So a static instrument that is **already in this repo's CI, already
      pointed at these exact files** sees the defect class today and is
      configured not to fail on it. `design.md` does not mention `qmllint` once
      — `grep -i qmllint design.md` returns nothing — neither as a considered
      alternative nor as a rejected one, while the document spends twenty lines
      arguing that the only remaining option was a name check.
      **What to change.** This is two edits, not a rewrite. First, narrow the
      claim at `:55-62` to what was measured: the component layer cannot see
      *the host collision*, because under `qmltestrunner` the host is absent.
      That sentence is true, equally short, and still justifies the static gate.
      Second, add the alternative the entry is missing — `qmllint`'s
      `missing-property` category, what it covers (every undefined token,
      statically, in every file including ones no spec instantiates), what it
      does **not** cover (the host collision, and a wrong-but-defined value),
      and whether this piece takes it or defers it. Either answer is fine; the
      unrecorded one is not, because "we looked at qmllint and it does not
      reach the collision" and "we never looked" are indistinguishable in the
      document as it stands, and the next person re-litigates from scratch.
      **Severity: high.** This is the design defect this dimension exists to
      find — not a stale number but a recorded justification whose load-bearing
      clause is false, in the entry that decided what the piece's primary
      instrument would be. The same overreach appears in `DTheme.qml:33-37`,
      `ci.yml:595-601` and `CLAUDE.md`; the spec-test review already asks a
      `tester` to correct those three, so this box is the `design.md` copy and
      the decision record behind them. **Verified:** both qmllint bounds run in
      this worktree.

- [ ] **`dev-writer`** — `design.md:145` says 17 and `.github/workflows/ci.yml:660`
      says sixteen, of the same set of files, so one of the two is wrong and a
      reader cannot tell which
      `design.md:145-147` — "checks the rename's completeness statically, across
      all 17 QML files in the module". `ci.yml:660` — "did not read four of the
      module's **sixteen** QML files".
      Measured: the gate's own output on this tree is `ok: **17** QML file(s)
      checked`, and `find <worktree>/dialectica-ui -name '*.qml'` returns 17 (13
      under `src/qml/`, 4 under `tests/`). `design.md` is right; the `ci.yml`
      comment is the stale one. It is describing the pre-fix tree, where
      `Theme.qml` had not yet become `DTheme.qml` and the count was 16 — but it
      is written in the present tense about the current module, so it reads as a
      claim about today.
      The reason this is a design finding and not a typo: `ci.yml:660` is the
      comment justifying **why the walker was widened from `src/qml/*.qml` to
      `rglob("*.qml")`**, i.e. it is the recorded rationale for a decision. A
      rationale that miscounts its own evidence is the shape that gets trusted
      and then re-derived wrongly. Fix the comment to 17, or better, drop the
      absolute figure and say "four of the module's QML files sat outside the
      gate entirely" — the count is a thing the gate prints on every run, so
      per CLAUDE.md's "do not write down anything a command can answer" it does
      not belong hardcoded in a comment at all. **Verified:** gate run in this
      worktree prints 17; `find` agrees.
      **Severity: low**, and it is a one-word edit — but it is the only
      arithmetic disagreement in the change and leaving it costs more in
      re-derivation than fixing it.

- [ ] **`dev-writer`** — `design.md:90-95` defers renaming `Core` to `DCore` to
      "a piece of its own", and nothing outside this change records that, so the
      deferral is deleted when `findings/` and the change are archived
      `design.md:95`: "`DCore` is the right end state and belongs to a piece of
      its own." `ci.yml:686-700` repeats it. Both of those are correct calls —
      the piece was right not to bundle a second rename, and the grandfather
      clause is honestly written as a clause with an expiry rather than a
      precedent.
      But `git grep -n "DCore" origin/main -- docs/PLAN.md` returns **nothing**,
      and `grep -rn "DCore"` across `docs/` and `CLAUDE.md` in this tree returns
      nothing either. The only durable trace after archive is the `GRANDFATHERED
      = {"Core"}` set in a workflow comment, which is where the fact lives but
      not where a reader looks for planned work.
      CLAUDE.md's own rule is that PLAN.md is left carrying **what is not built
      yet**. An identified, deliberately-deferred rename with a stated end state
      is exactly that. One line in PLAN.md — "the QML `Core` singleton is not
      yet `D`-prefixed; the gate grandfathers it" — is self-invalidating in the
      way that section asks for, because the moment someone renames it the line
      is visibly about a thing they can check.
      **Severity: medium.** Not a contradiction, a gap: the decision is recorded
      in a document designed to be thrown away. This is also the one finding
      here that gets *harder* to fix later, since after archive nobody will know
      the deferral was deliberate rather than overlooked.

- [ ] **`dev-writer`** — `design.md:136-140` records "fail on any QWARN" as the
      rejected alternative for the runner check, but not `QT_FATAL_WARNINGS`,
      which is the alternative a reader actually reaches for and which
      `run-qml-tests.sh:100-105` already explains
      `design.md:136-140` records the choice (two pinned patterns), the
      constraint (a `ReferenceError`-only check passes `DTheme.noSuchToken`), and
      one rejected alternative (failing on every QWARN, "too blunt"). That is
      three of the four parts.
      The missing alternative is the obvious one-line answer: `QT_FATAL_WARNINGS`.
      `run-qml-tests.sh:100-105` rejects it properly and with a measured reason —
      it aborts on the first warning of any kind, so the run crashes instead of
      diagnosing and the remaining specs never execute, and `qmltestrunner` has
      no flag that escalates a warning to a failure. `CLAUDE.md` carries it too.
      `design.md` carries neither.
      That inverts the intended direction of travel: the durable reasoning sits
      in a shell comment and a repo-wide file, while the change's own Decisions
      section — the document a reviewer of *this* change reads — has the thinner
      entry. Two lines in `design.md` naming `QT_FATAL_WARNINGS` and what ruled
      it out completes the entry.
      **Severity: low.** A thin entry rather than a wrong one, and the reasoning
      does exist in the tree. Worth closing because this is the entry most likely
      to be re-litigated — "why not just set the env var" is the first question
      anyone asks of `check_bindings`.

## What I could not check

- **The collision itself.** I did not launch basecamp, so "`DTheme` resolves to
  our file under the real host" is unverified here, as it was for the spec-test
  reviewer. `design.md:155-157` is explicit that no gate covers this and that a
  real launch is required; I am recording the gap as still open, not disputing
  the claim.
- **Whether the gate's shipped heredoc and the copy I ran diverge on some third
  input.** I confirmed identical behaviour on this tree (exit 0, 17 files) and on
  `733544d` (exit 1, 116 lines, 113/3 split), which is both bounds the document
  claims, but a divergence visible only on a third input would not have
  surfaced. The spec-test review's second box asks for the gate to become a
  callable script for exactly this reason, and I endorse it on design grounds
  too: a rationale that cannot be executed by the thing it documents is a
  rationale nobody can re-check.
- **Whether the `dev-writer` currently working in `.claude/worktrees/piece-theme-unshadow`
  has already addressed any of the above.** My branch point is `6a7cf00` and may
  be behind that worktree. Nothing in what I read looked half-finished.
