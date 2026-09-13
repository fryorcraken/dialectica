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

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:693` — the prefix rule covers
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

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:731` — arm 3 hardcodes the
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

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:637` — the gate is placed in
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

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:661` vs `:365` — the two gates
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
