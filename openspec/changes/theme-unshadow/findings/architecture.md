# Architecture findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `76a58d2`, in a worktree of its own. The
gate was run both ways and mutated rather than read.

**Is a static name check the right instrument at all?** Yes, and the piece
argues it honestly. The defect lives in basecamp's C++ type registration; the
two `-import` measurements recorded in `design.md` establish that a file-based
competitor cannot be staged under `qmltestrunner`, so a component test here is
not merely absent but structurally unable to fail. A check that cannot fail is
worth nothing by this repo's own standard, and the piece says so in five places
rather than shipping a reassuring assertion. Given that, a static check on the
*name* is the only instrument available in CI, and the gap — that it proves a
name absent, not that resolution is correct — is stated plainly in `ci.yml`
(595–601), `CLAUDE.md` (320–324), `design.md` and `DTheme.qml`. The honesty is
the strong part of this piece. The instrument is right; what is below is where
it does not yet cover its own subject.

**What is clean**, in prose rather than as boxes:

- **The organising idea is right.** Enforcing the convention the design relies
  on, rather than enumerating host names, is the correct response to the
  security finding — a prefix rule is total over registrations that have not
  happened, where a list is total over none. Moving from a blocklist to an
  invariant is "complexity in the data structure, not the logic" applied to a
  gate.
- **Scope discipline holds.** The piece takes only the shadowing from the
  three-way commit `2237a45`; `ApparatusColumn.qml` and `MarginNote.qml` survive
  with renamed references, which is the mechanical consequence of the rename and
  not a judgement, and `design.md` says exactly that. `git diff 733544d..HEAD
  --stat` shows 23 files, all of them ones a rename plus a gate plus its
  documents would touch, and nothing under `dialectica/`.
- **The `D` prefix is a convention the repo can hold, and it is recorded where a
  singleton author meets it** — `CLAUDE.md`'s "Module contract traps" entry, not
  only in this change's documents. That is the right home: it is the section
  build- and launch-time traps are collected in, and a person adding a singleton
  to `qmldir` has the gate's own message as the second meeting point.

---

- [x] **`dev-writer`** — `.github/workflows/ci.yml:693` — the prefix rule covers
      only `singleton` lines, so a `qmldir` entry declaring a non-singleton type
      named exactly `Theme` passes the gate green
      **Scenario:** measured in this worktree. Add one line to
      `dialectica-ui/src/qml/qmldir`:
      `Theme 1.0 Identicon.qml`
      — a plain component entry, not a singleton. Run the step's script: **`ok:
      17 QML file(s) checked, every singleton D-prefixed, no bare Theme
      reference`, exit 0.** The gate is green over a `qmldir` that declares a QML
      type called `Theme`, which is the precise thing this piece exists to
      prevent. Arm 1 skips the line because `parts[0] != "singleton"`; arm 3
      reads `.qml` bodies, never the `qmldir` type names.
      **Severity: high.** The step's own evidence points straight at this: it
      cites the host registering **`LogosButton.qml`** as proof the namespace
      holds names beyond any five — and `LogosButton` is a *component*, not a
      singleton. The eleven component entries already in `qmldir` (`Identicon`,
      `FlatButton`, `PostHeader`, `ScreenFrame`, `FeedScreen`, …) are QML type
      names in the same directory namespace and are exempt from the rule the
      piece calls its defence. `design.md` states the defence as "a name nothing
      in the host can claim cannot be shadowed by anything basecamp adds later";
      the gate applies that to 2 of the 13 names the module declares.
      Two shapes of fix, and the second is the one that matches the design: apply
      the `D` rule to every `qmldir` entry (which means renaming eleven
      components and is a piece of its own), or — cheaper and available now —
      have arm 1 read every `qmldir` line rather than only `singleton` ones and
      fail any entry whose declared name is not `D`-prefixed *or* grandfathered,
      with the eleven existing components entering `GRANDFATHERED` explicitly.
      That trades one silent gap for eleven named ones, which is the honest
      state and makes the twelfth visible.

      **Fixed**, taking the second shape you name. Confirmed before fixing
      rather than inherited: I reconstructed the pre-fix arm verbatim, added
      `Theme 1.0 Identicon.qml` to `qmldir`, and reproduced your measurement —
      **exit 0, `ok: 17 QML file(s) checked`** over a `qmldir` declaring a type
      called `Theme`. The widened gate exits 1 at `line=13` on that same tree.

      `qmldir_entries` now yields every declaration — `singleton`, `internal`
      and plain component — skipping only the command lines (`module`,
      `depends`, `import`, …) that declare no type. The eleven component names
      are in `GRANDFATHERED` explicitly, with the comment saying they are the
      enumeration of what is *unprotected* rather than a place to put a twelfth.
      `Core` keeps its stronger reason, stated separately.

      On your "two shapes" framing: the eleven renames are the shape that makes
      the invariant hold by construction, and I did not take it — it is eleven
      renames across every view file, which is a piece of its own. `design.md`
      records that under "The rule covers every `qmldir` entry", including the
      gap that remains by name.

      The gate is also no longer a heredoc, which is why both of your findings
      could ship through review at all: it is
      `dialectica-ui/tests/check_qml_names.py` with `tst_check_qml_names.py`
      beside it, 16 cases, both bounds. Three of them fail if the entry walk is
      narrowed back to `singleton`; one of them is that the gate must still
      ACCEPT a `qmldir` carrying `module`/`depends`/`import` lines, which is the
      regression the narrowing would otherwise be a fix for.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:731` — arm 3 hardcodes the
      single name `Theme`, so the gate detects the collision that already
      happened and no other
      **Scenario:** arm 1 is the general rule (every singleton carries a `D`);
      arm 3 is a literal for one name. The two are doing different jobs under one
      step name. Concretely: rename `Core` to `Palette` in `qmldir` and reference
      `Palette.x` throughout the view — arm 1 fires (not `D`-prefixed), which is
      right. But a *component* named after a host type, or a stale reference to
      any singleton name other than `Theme`, is invisible: arm 3 searches for
      `Theme` and nothing else. Once `Core` becomes `DCore` — which the
      grandfather comment names as the right end state — nothing in this gate
      will notice a surviving bare `Core.` reference, and the piece will have
      repeated itself.
      **Severity: medium.** The general version is available and is the same
      shape as arm 1: derive the banned reference names *from `qmldir`* — for
      each singleton declared as `DX`, a bare `X` in any `.qml` body is a stale
      reference. That makes the gate total over the module's own singletons
      instead of over the one name someone remembered, and it removes the
      hardcoded literal that a future rename will strand.

      **Fixed**, exactly as you specify. `stale` is now built from `qmldir`:
      for each type declared `DX`, a bare `X` in any `.qml` body is reported,
      with the message naming both the bare name found and the declared name it
      should be. Your `Core` scenario measured before the fix, so the cost is
      concrete rather than hypothetical: renaming `Core` to `DCore` leaves **33
      stale `Core.` references** across `FeedScreen.qml` and three spec files,
      and the pre-fix gate reports **exit 0** on all 33.

      One thing the fix surfaced that neither of us predicted, recorded because
      it changes what a red looks like. Deriving *only* from `D`-prefixed names
      made the gate go quiet on `main`: `main` declares no `D`-prefixed type at
      all, so the derived set was empty and the gate reported the one `qmldir`
      line and nothing else — losing the 116 bare references that are this
      change's proof-of-failure figure. So the set is seeded from two
      directions: the `D`-prefixed declarations, and the names arm 1 has just
      *rejected*, since a type declared without the `D` is one we are asking to
      be renamed and every reference to it must change with it. Against `main`
      the gate now reports **117 lines = 1 `qmldir` + the same 116** (113 under
      `src/qml/`, 3 in `tests/tst_identicon.qml`), matching `design.md` exactly.

      Pinned by `a tree mid-rename`, which fails if that seeding is removed —
      and it asserts on the message, so "rejected for the wrong reason" does not
      pass it.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:637` — the gate is placed in
      the `qml` job behind a full Qt6 `apt-get install`, while its three sibling
      static QML checks live in the `lint` job with no Qt at all
      **Scenario:** the step is pure `python3` — `pathlib`, `re`, and reading
      text files. It needs no Qt binary, no `qmllint`, no `qmltestrunner`. Yet it
      sits at `ci.yml:637`, after the `Install Qt` step at 435 which pulls
      `qt6-base-dev qt6-declarative-dev qt6-declarative-dev-tools` and seven
      `qml6-module-*` packages. The three checks it most resembles — `no QML
      component shadows a Qt built-in` (202), `every QML file importing layouts
      declares the import` (221), `every QML Text declares an explicit
      textFormat` (262) — are all in the `lint` job, which installs nothing.
      So the cheapest and fastest-failing check in the repo is the one that waits
      longest to report, and it is separated from the three checks a maintainer
      would look for it beside.
      **Severity: low** (no correctness consequence; a CI-shape and
      discoverability one). Moving it to the `lint` job next to the Qt-built-in
      shadow check puts the two "no QML type of ours may be named X" gates in one
      place, which is where someone adding the third will look.

      **Fixed**, to exactly the position you name — it now sits immediately
      after `no QML component shadows a Qt built-in`, and the comment states the
      pairing: that check covers names QtQuick claims, this one covers names
      basecamp claims. Verified by re-parsing the workflow with `yaml.safe_load`
      and printing both jobs' step lists, so the placement is what the file
      reads as rather than what the diff looks like.

      The move came free with the extraction the first box needed: a step that
      is one `run:` line has nothing job-specific left in it. Both jobs are
      otherwise unchanged — `qml` keeps its five Qt-dependent steps.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:661` vs `:365` — the two gates
      converged in style but each still owns its own definition of a comment, so
      they are not yet maintainable together
      **Scenario:** the prompt asks whether this piece's adoption of
      `piece/publish-envelope`'s shape went far enough. It did not. The sibling
      gate at `ci.yml:365` strips comments with
      `re.sub(r"^\s*//.*$", "", src, flags=re.M)` — leading `//` only, no block
      comments, because it reads Rust. This piece's `strip_comments` (661–665)
      handles `/* */` across lines *and* trailing `//`, and preserves line numbers
      by substituting newlines. Both are correct for their own language; neither
      knows the other exists. The shared idea — "strip once up front so every
      check inherits one answer about what a comment is" — is now stated twice in
      two comment blocks (360–364 and 630–636) and implemented twice with
      different semantics.
      **Severity: low.** This is an observation about direction rather than a
      defect: the piece adopted the right *principle*. But `design.md` presents
      "one place to be right about what a comment is" as the reason for the
      change, and there are now two places. If a third gate needs stripping, the
      decision to make is whether these become one helper the workflow's Python
      steps share, or whether the claim in `design.md` is narrowed to "one place
      per step". Either is fine; leaving the claim broader than the code is not.

      **Fixed by taking the second option: the claim is narrowed, not the code
      unified.** `design.md` has a new section, "Two comment-strippers, and the
      claim narrowed to match", which states plainly that there are two
      implementations for two languages, that neither knows the other exists,
      and that the surviving principle is the per-step one — strip once up front
      so every check *within a step* inherits one answer, rather than bolting a
      filter onto the one arm that needs it.

      The reasoning for choosing that over unifying, since you left it open: the
      two strip different syntaxes because they read different languages, so a
      shared helper would need a language parameter, and it would have to be
      importable from two jobs — which for a workflow means a third file
      existing to be shared by two callers that disagree about what it should
      do. That costs more than it buys at two. `design.md` names the point to
      reconsider: a third gate needing stripping.

      Your framing that this is "an observation about direction rather than a
      defect" is right, and the defect was the sentence rather than the code —
      which is why the fix is in prose.
