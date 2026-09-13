# Design: unshadow the theme singleton

## Decisions

### A prefix, not a module URI

The collision is that basecamp registers a type called `Theme` in its own C++
type registration, and that registration is what our `Theme.x` bindings reached.
See "The gate is static" below for what that does **not** mean — the precedence
story is the withdrawn premise, not the mechanism. Two shapes of fix were
available.

Giving our components a module URI (`import Dialectica.Theme 1.0`) scopes the
name, but it only relocates the question: it makes our `Theme` safe from *this*
collision while leaving open what happens when the host registers something in a
namespace we also want. It also costs an `import` line in every file and a
build-time module definition, for a problem one character solves.

**Renaming to `DTheme` is chosen because a name nothing in the host can claim
cannot be shadowed by anything basecamp adds later.** The `D` is for dialectica,
it is two characters, and it needs no import machinery. The cost is that it is a
convention rather than a mechanism — nothing stops the next person adding a
`Theme.qml` — which is what the CI gate below is for.

### The gate is static, and that is not a compromise

**Measured, not assumed.** Before writing the gate I tried to build a component
test that fails on the shadowing, on Qt 6.10.3 with the repo's own
`qmltestrunner`:

1. **Staged a competing `Theme` singleton in a second directory and handed it to
   the runner via `-import`.** Our `ScreenFrame` still resolved the plugin
   directory's own `Theme` and reported `implicitWidth` 1000. No shadowing.
2. **Made that directory a named module on the import path** (`-import` at its
   parent, so it resolves as a module rather than a bare directory). Same
   result: `implicitWidth` 1000, no shadowing.
3. **Substituted a token-less `Theme.qml` into a staged copy of the view**, which
   *does* reproduce the symptom — `implicitWidth` 0, `color #ffffff` instead of
   paper `#efe9dc`. But that is a simulation of the consequence, not of the
   mechanism: it proves what happens once `Theme` carries no tokens, and says
   nothing about whether the host can make that happen.

**(1) and (2) withdrew a premise this change started from.** The brief, and the
prior commit's message, explained the defect as a host registration that
*outranks* a plugin directory's `qmldir` entry — the two competing, the host
winning on precedence. That is the intuitive reading and it is **false**: a
file-based competitor does not enter the contest at all, so there is no
precedence for the host to win on. The premise is recorded here rather than
quietly dropped, because it is what anyone will re-derive from the symptom, and
because acting on it means building a reproduction that cannot work.

What is true is narrower and is the durable part: **the collision lives in
basecamp's C++ type registration, which no test harness in this repo can
reach.** Under `qmltestrunner` there is simply no competitor, so
`verify(DTheme.paper !== undefined)` **cannot fail on the collision** — and by
this project's standard a check that cannot fail on the thing it is written for
is worth nothing.

**State that precisely, because the broader version is false**, and the
spec-test review measured it: the assertion is not a check that cannot fail at
all. A probe spec run against this tree's import path confirms `DTheme.paper`
resolves while an undeclared `Theme.paper` throws, so the assertion *does* fail
if the singleton is renamed, if its `qmldir` entry is dropped, or if its file
goes missing. It is blind to the host collision specifically. The overbroad
wording is the sentence that told the next person not to look, and it is why
qmllint's `missing-property` went unexamined for a change about undefined
tokens — a cost, not a pedantic distinction.

So a QML test is not merely absent here; it is **structurally unable to see the
collision**, because under `qmltestrunner` the host is absent and there is no
competitor to lose to. That is the whole of the impossibility, and it is what
makes the name check static: the gate for *the collision* is therefore the
`no QML type name collides with the host` step, which runs
`dialectica-ui/tests/check_qml_names.py`.

**The narrow wording is the point.** An earlier version of this paragraph said
the component layer was structurally unable to see "this defect", which reads as
the whole class and is false of the **undefined tokens the collision produces**.
That is a different class, it is statically visible, and the overbroad sentence
is the reason nobody looked for an instrument that sees it.

### `qmllint --missing-property`, considered and taken

The alternative the paragraph above should have weighed, recorded here because
"considered and rejected" and "never looked" were indistinguishable until it
was: the design review found `qmllint` named nowhere in this document, while
twenty lines above argued a name check was all that remained. An unrecorded
alternative is re-litigated from scratch by the next reader, which is the cost
this section exists to pay off.

**What it covers.** Every member read off one of our own types, statically, in
every file — including the ones no spec instantiates, which is precisely where
the suite's `check_bindings` stops. `Main.qml` is the case that matters: it
paints the screen's ground and no spec constructs it.

**What it does not cover, and this must not be blurred.** Not the collision. CI
passes `-I dialectica-ui/src/qml`, and `qmldir` declares `singleton DTheme 1.0
DTheme.qml` in that very directory — so qmllint resolves `DTheme` to our own
file, where every member genuinely exists. **It checks a different resolution
than the app performs**, and a green from it is evidence about members and about
nothing else. Nor does it see a wrong-but-defined value.

**This piece takes it rather than deferring it**, because the instrument was
already in CI, already pointed at these files, and already printing the defect —
as a `Warning:` into a green log, since the step gated on the exit code alone.
The escalation is `dialectica-ui/tests/check_qml_members.sh`, run from the `qml`
job with `tst_check_qml_members.sh` beside it pinning both directions. See "What
actually checks the rename" below for the three-way division it completes.

**It was proven to fail before it was trusted.** Run against `main` — the tree
carrying the defect — it exits 1 reporting the `qmldir` declaration as
un-prefixed and **116 lines** carrying a bare `Theme` reference (113 under
`src/qml/`, 3 in `tests/tst_identicon.qml`). Run against this tree, it passes.
That is a gate with a demonstrated failing case, not a step that has only ever
been seen green.

The figure is the gate's own output, counted, and it is unchanged by the two
widenings below — re-measured against `main` after both: 117 error lines, being
the `qmldir` line plus the same 116. An earlier draft of this document said
"80-odd", which no reading of the command produces; the correctness review
measured 113 for the `src/qml/` scope and that half is confirmed here exactly.

### What the gate checks, and why it stopped enumerating

The first version banned a **list** of five names basecamp was known to occupy.
The security review rejected that as the `hand-maintained sweep lists go stale
silently` pattern, and it is right: the list is correct only until basecamp
registers a sixth name, and nothing in CI can notice that it has. The verified
launch log for this change shows the host registering `LogosButton.qml`, so the
namespace demonstrably holds names beyond any five someone thought of.

**The gate now enforces the prefix convention**, which is what the design
actually relies on — "a name nothing in the host can claim cannot be shadowed by
anything basecamp adds later". A rule that every declared type is `D`-prefixed
is total over host registrations that have not happened yet, where a list is
total over none of them.

### What the host namespace actually contains, measured

The arguments above were made without anyone having looked at the host's
registrations. A real Basecamp launch log settles it — read from
`.scaffold/basecamp/profiles/alice/xdg-data/Logos/LogosBasecampDev/logs/basecamp_<timestamp>.log`
(**not** `basecamp.log`; the wrong filename in `CLAUDE.md` is part of why nobody
had read one). **29 distinct types under `qrc:/qt/qml/Logos/`:**

- `Controls/` — 23 types, **every one `Logos`-prefixed** (`LogosButton`,
  `LogosText`, `LogosTable`, `LogosDialog`, …)
- `Icons/` — `LogosIcons`
- `Theme/` — `ColorPalette`, `DarkTheme`, `Spacing`, **`Theme`**, `Typography`

The set reproduces identically across two independent launches of two different
branches, so it is a property of the host build rather than of one run.

**This supports the prefix rule rather than replacing it with the list.** The
host's own convention is a `Logos` prefix, and its five unprefixed names are all
inside `Theme/` — so a `D`-prefixed name cannot collide with what basecamp ships
today, and cannot collide with what it ships later either unless the host
abandons its own convention. Enumerating those 29 names in the gate would be
correct only until the 30th, which is exactly the `hand-maintained sweep lists
go stale silently` trap the security review already rejected. A measurement of
the list is evidence *for* the invariant; it is not a reason to hardcode it.

**And `Theme` is in there, one of only five unprefixed names** — which is the
defect, confirmed in production conditions rather than argued: 209 resolutions
to `qrc:/qt/qml/Logos/Theme/Theme.qml`, **zero** ever reaching the plugin's own
`Theme.qml`, and 295 `Unable to assign [undefined]`.

### `Core` does not collide — measured, not assumed

`qmldir` declares `singleton Core 1.0 Core.qml`, a name as generic as `Theme`,
and the question of whether it collides the same way had never been asked. It
does not. The same log shows **27 resolutions into `dialectica_ui/qml/Core.qml`
and zero into any `qrc:/qt/qml/Logos/Core*`** — the exact inverse of `Theme`.
`Core` is absent from the 29, consistent with `Core` having resolved correctly
throughout the outage.

That makes `Core`'s grandfathering a fact with evidence rather than an untested
hope — **and does not make it safe to depend on.** "Basecamp has no `Core`
today" is a statement with no expiry date attached, which is the shape the
prefix rule exists to stop depending on. `DCore` remains the right end state.

A related symptom was checked and is **not** this defect. A dogfood launch
showed *"The core module returned something that is not a reply."* That string is
`Core.qml:71`, reached only when `JSON.parse` **succeeds** and yields a
non-object. A missing bridge — what a `Core` collision would cause — returns
`Core.qml:51`, *"The core module is not reachable from this view."* The code
path distinguishes the two, so the observed message rules out a `Core` collision
on its own, independently of the log. It points at core returning a bare JSON
scalar, and belongs to whoever owns that call.

### The rule covers every `qmldir` entry, not only the singletons

**The first version of the prefix rule read only `singleton` lines, and that was
a hole the size of the piece.** The architecture review measured it: adding
`Theme 1.0 Identicon.qml` — a plain component entry declaring the exact name
this change exists to remove — left the gate at `ok: 17 QML file(s) checked`,
exit 0. Re-measured here against the pre-fix gate before widening it, to confirm
the finding rather than inherit it: **exit 0, green, over a `qmldir` declaring a
type called `Theme`.**

The step's own evidence pointed at this the whole time. It cites the host
registering `LogosButton.qml` as proof the namespace holds more than any five
names — and `LogosButton` is a **component, not a singleton**. A component entry
registers its name in the same directory namespace and is shadowable in exactly
the same way. The gate was applying the design's central defence to 2 of the 13
names the module declares.

Two shapes of fix were available. Renaming the eleven components to `DIdenticon`,
`DFlatButton` and so on makes the invariant true by construction, which is the
shape this repo prefers — but it is eleven renames across every view file, which
is a piece of its own and not this one's scope. **The rule is instead widened to
every `qmldir` entry, with the eleven existing components named explicitly in
`GRANDFATHERED`.** That trades one silent gap for eleven visible ones: nothing
new is protected today, but the exemption list is now the honest statement of
what is unprotected, and the twelfth entry cannot be added without either a `D`
or a deliberate edit someone has to justify.

`Core` is grandfathered for a stronger reason than the components, and the
distinction is worth keeping: it predates the convention, this piece deliberately
did not rename it, and it is itself the evidence that the host does not claim
every plausible name. That is also why these are grandfather clauses and not a
precedent — "basecamp has no `Core` today" is exactly the kind of fact the prefix
rule exists to stop depending on. `DCore` and the eleven are the right end state
and belong to a piece of their own.

**That deferral is recorded in `CLAUDE.md`, beside the gate's description, and
not only here.** This document is archived when the change closes, and the
`GRANDFATHERED` set that survives states what is unprotected without stating
that anyone intended it — so after archive a reader could not tell deliberate
from overlooked. The `Core` measurement that makes the exemption evidence rather
than hope lives there too, for the same reason.

### The stale-reference check derives its names from `qmldir`

The reference arm searched for the literal word `Theme`, which meant it detected
**the collision that already happened and no other**. Once `Core` becomes
`DCore`, a surviving bare `Core.` reference would have been invisible and the
piece would have repeated itself. Measured, to make the cost concrete rather
than hypothetical: renaming `Core` to `DCore` in `qmldir` leaves 33 stale
`Core.` references across `FeedScreen.qml` and three spec files, and the pre-fix
gate reports **exit 0** on all of them.

**The banned names are now derived from `qmldir` itself**: for each type declared
as `DX`, a bare `X` in any `.qml` body is a stale reference. That makes the arm
total over the module's own declarations instead of over the one name someone
remembered, and it removes a literal that the next rename would strand.

The set is seeded from two directions, and the second was found by running the
gate rather than reasoning about it. Deriving only from `D`-prefixed names left
the gate **silent about the 116 bare references on `main`**, because `main`
declares no `D`-prefixed type at all — it reported the one `qmldir` line and
stopped. So a name arm 1 has just *rejected* also seeds the set: a type declared
without the `D` is a name we are asking to be renamed, and every reference to it
must change with it. With that, the gate against `main` reports 117 errors — the
`qmldir` line plus the same **116** bare references (113 under `src/qml/`, 3 in
`tests/tst_identicon.qml`) this document has recorded throughout.

### The gate is a script with its own tests, not a heredoc

It was ~50 lines of Python inside a `ci.yml` heredoc, and **a heredoc cannot be
tested**: the only way to exercise it was to push a branch and read a CI log.
Both of the defects above shipped through review for that reason — they are the
kind a single run would have caught.

It is now `dialectica-ui/tests/check_qml_names.py`, taking a module root as an
argument, with `tst_check_qml_names.py` beside it. That is the standard this repo
already holds `check_bindings` to, and for the same stated reason: a gate is part
of the measurement, and one narrowed until it matches nothing passes exactly as
quietly as a correct one.

**The test pins both bounds, and pins the corpus-builders directly.** Rejecting
each bad tree is the easy half — satisfied by `sys.exit(1)` — so the suite also
requires the gate to *accept* a good module, the real shipped `dialectica-ui`,
and a `qmldir` carrying `module`/`depends`/`import` command lines (the regression
someone would introduce by narrowing the walk back to `singleton`). Every
rejection case additionally asserts on the message text, so "rejected" cannot
pass for the wrong reason.

That last point is what makes the suite evidence rather than decoration, and it
was checked by mutation rather than assumed. Stubbing `qmldir_entries` to return
nothing — the exact defect this repo has shipped before, a filter pinned from
both sides whose corpus-builder was then emptied with every test still green —
fails **twelve of the sixteen cases here**, and seven of those twelve fail
*because of the message assertion*: without it they would have counted as
correctly-rejected. Narrowing the file walk back to `src/qml/*.qml` fails one;
narrowing the entry walk back to `singleton` fails three; hardcoding the
reference arm back to `Theme` fails two; and adding `Theme` to `GRANDFATHERED` —
the move someone makes to turn a red green — fails three.

### The gate moved to the `lint` job

It is `pathlib` and `re` over the checkout: no Qt binary, no `qmllint`, no
`qmltestrunner`. It sat in the `qml` job behind a full Qt6 `apt-get install`,
so the cheapest check in the repo waited longest to report, and it was separated
from the three static QML checks a maintainer would look for it beside.

It now sits in `lint` next to `no QML component shadows a Qt built-in`, which is
the same question asked of a different namespace — that check covers names
QtQuick claims, this one covers names basecamp claims. Putting them together is
where someone adding a third will look.

### Two comment-strippers, and the claim narrowed to match

The sibling gate at the top of `ci.yml` strips comments with
`re.sub(r"^\s*//.*$", ...)` — leading `//` only, because it reads Rust. This
gate's `strip_comments` handles `/* */` across lines and trailing `//`, and
preserves line numbers by substituting newlines. Both are correct for their own
language; neither knows the other exists.

An earlier version of this document presented "one place to be right about what
a comment is" as the reason for the change, which was **broader than the code**:
there are two places, in two languages. The claim is narrowed here rather than
the code unified. Unifying would mean a shared helper importable from two jobs
for two different comment syntaxes, which buys less than it costs while there are
two. The principle that survives is the per-step one: strip once up front so
every check *within a step* inherits one answer, rather than bolting a filter
onto the one arm that needs it. If a third gate needs stripping, that is the
point to reconsider.

What the stripping bought, which is the half worth keeping: the previous version
bolted a `grep -v ':[0-9]*: *//'` onto the one arm that needed it, and documented
the resulting false positive rather than fixing it — a bare `Theme.` in a `/* */`
block comment, or trailing a line of code, failed the arm while being correct.
Stripping `//` to end of line and `/* */` however many lines it spans, once up
front, removes both. A gate that fires on its own explanation is a gate someone
weakens rather than obeys, and `DTheme.qml`'s header quotes `Theme.x` on purpose.
Both forms are pinned in `tst_check_qml_names.py` from the accepting side. Block
comments are replaced by the newlines they spanned, so every reported line number
still points at the line a human will find in the file.

### What actually checks the rename, and what does not

**The suite is not a second line of defence, and an earlier draft of this
document said it was.** That claim — that the CI arm and the component tests
cover the rename "from both directions" — is false, and the correctness review
measured why: `qmltestrunner` reports a `ReferenceError` raised inside an
instantiated component as a **QWARN, not a failure**. Reproduced here: the
column-0 mutation in `MarginNote.qml` printed `ReferenceError: Theme is not
defined` dozens of times across the `FeedStates` spec, and the runner still
reported `12 passed, 0 failed`, whole suite exit 0.

So the two did not cover each other; they shared a blind spot. Before the fix
below, the suite caught a stale reference **only** when it sat inside a
`compare()` — as `tst_identicon.qml`'s palette assertions do, which is why that
one spec failed on a stale reference while the rest stayed green.

**That blind spot is now closed at the runner**, in `check_bindings` in
`run-qml-tests.sh` (`tasks.md` §6). The runner captures each spec's output and
fails the run when it carries a diagnostic meaning "a binding evaluated to
`undefined`", whatever the spec reported. Failing on *any* QWARN was rejected as
too blunt — an unrelated Qt warning would fail the suite for a reason unrelated
to the code — so two specific families are matched, and the second is not
optional: a missing token on a correctly-named singleton (`DTheme.noSuchToken`)
raises **no `ReferenceError` at all**, reporting `Unable to assign [undefined]`
instead, so a `ReferenceError`-only check would have passed it.

**`QT_FATAL_WARNINGS` was the other rejected alternative**, and it is the one a
reader reaches for first — "why not just set the env var" is the first question
anyone asks of `check_bindings`. It aborts the process on the **first** warning
of any kind, so the run dies with a crash rather than a diagnosis and takes the
remaining specs with it; it is blunt in exactly the way failing on any QWARN was
rejected for being. `qmltestrunner` has no flag that escalates a warning to a
failure — `-help` lists none — so reading the runner's output is the mechanism
actually available. `run-qml-tests.sh` carries the same reasoning at the point
of use.

That leaves the honest division, which is now genuinely two directions rather
than one:

- **The CI gate** checks the rename's completeness statically, across every QML
  file in the module — `tests/` included, which is the widening that mattered;
  the count is printed on every run rather than recorded here. It reads source,
  so it catches the old name wherever it appears, including in files no spec
  touches.
- **The suite** now fails on any binding that evaluates to `undefined` at
  runtime, **in the components a spec instantiates** — including defects the
  gate cannot see, since a D-prefixed typo (`DThemeTypo`) and a missing token
  both contain no bare `Theme`. Measured: the gate exits 0 on both, the runner
  exits 1. It also still pins that the identicon's ink indexing lands on the
  same seven constants.

  **The qualifier is load-bearing and was measured, not assumed.** `Main.qml` is
  not a component any spec instantiates. With `color: DTheme.desk` there
  rewritten as `DTheme.noSuchDesk` — a typo in the binding that paints the whole
  screen's ground — the suite exited 0 with 41 passed and zero diagnostics, and
  the name gate exited 0 alongside it.
- **The member gate** (`dialectica-ui/tests/check_qml_members.sh`) closes that.
  It is `qmllint --missing-property error` over every file in `src/qml`, and it
  is the third direction rather than a variation on the other two: it needs no
  spec to instantiate anything, so it reaches `Main.qml` and every other file a
  spec never constructs. Measured both bounds — exit 255 naming the file, line
  and member with the typo in place; exit 0 on the unmutated tree.

  It exists because an impossibility claim was wrong. Four places in this repo
  said the component layer was structurally blind here and only a static *name*
  check was available. That is true of the collision; it was never true of the
  undefined members the collision produces, and qmllint had been printing those
  as warnings into a green log the whole time.
- **None of the three can see the collision itself**, for the reason measured
  above: under `qmltestrunner` the host is absent. The runner check closes the
  runtime half of the blind spot, not the host-precedence half, which remains a
  static check plus a real basecamp launch.

  **The member gate specifically must not be read as covering it.** CI passes
  `-I dialectica-ui/src/qml`, which puts our own singleton on the import path,
  so qmllint resolves `DTheme` to the correct file where every member genuinely
  exists — it is checking a *different resolution* than the app performs.
  Measured from both bounds: silent exit 0 on a real branch, exit 255 with a
  bad member planted. A green from it is evidence about members and about
  nothing else.
- **Neither can see a wrong-but-defined value** — `DTheme.paper` where
  `DTheme.ink` was meant produces no diagnostic anywhere.
- **The gate proves a name absent, not that resolution is correct.** Whether
  basecamp actually registers any given name is invisible from the checkout.
  The `D` prefix is what makes the question not need asking, which is why the
  gate checks the convention rather than trying to enumerate the host.
- **The eleven grandfathered component names are unprotected, by name.** They
  are listed in the gate rather than covered by it, so `Identicon` colliding
  with a future basecamp `Identicon` would pass. That is the honest state after
  widening the rule to every entry: the gap was there before and invisible, and
  it is there now and enumerated. Renaming them closes it and is its own piece.

## Dead ends, recorded so they are not re-walked

- **"A host registration outranks a plugin directory's `qmldir` entry" is the
  withdrawn premise**, and it is the one to know about, because it is what the
  symptom suggests and what this change was briefed with. It frames the defect
  as a precedence contest, which invites exactly the reproduction that cannot
  work. Measured false twice; see "The gate is static" above.
- **`-import` with a competing singleton does not reproduce host shadowing** —
  two variants tried, both resolved our own file. Anyone reaching for "surely a
  test can stage the competitor" should read the three measurements above before
  spending the afternoon again.
- **`Core` resolving correctly is not evidence.** It resolves because basecamp
  has no `Core` — the host never registers the name, so nothing displaces our
  file. The same file, the same imports, and a different outcome purely on
  whether the host happens to occupy the name.

## What this change deliberately does not do

Commit `2237a45`, from which this fix is recovered, bundled three independent
repairs: the shadowing, a `ScreenFrame` with no `implicitHeight`, and a header
`RowLayout` that overflowed instead of eliding. It also deleted
`ApparatusColumn.qml` and `MarginNote.qml`, and added a layout test.

**Only the shadowing is taken.** The other two are real and are someone else's
piece; a diff doing three things cannot be reviewed for any of them, nor
reverted without losing the halves you wanted. `ApparatusColumn.qml` and
`MarginNote.qml` therefore survive here with their references renamed, which is
the mechanical consequence of the rename and not a judgement about whether they
should exist.
