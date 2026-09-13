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
`verify(DTheme.paper !== undefined)` passes whatever the singleton is called —
it is a check that cannot fail, and by this project's standard that is worth
nothing.

So a QML test is not merely absent here; it is **structurally unable to see this
defect**, and writing one would be the false green CI is built against. The gate
is therefore the static `no QML type name collides with the host` step in
`ci.yml`.

**It was proven to fail before it was trusted.** Run against `main` — the tree
carrying the defect — all three of its arms fire: the `Theme.qml` filename, the
`qmldir` declaration, and 80-odd bare `Theme.` references. Run against this
tree, it passes. That is a gate with a demonstrated failing case, not a step
that has only ever been seen green.

### Why the gate excludes comment lines

`DTheme.qml`'s own header explains the collision at length and quotes `Theme.x`
while doing so. Without the `grep -v` on a leading `//`, the gate would be
permanently red on a file that is correct — which is the fastest way to get a
gate deleted, and would make this whole step worse than nothing.

That exclusion is exercised rather than dead: `grep -n "[^A-Za-z]Theme\."` over
`DTheme.qml` returns two comment lines today, so the arm is doing work on every
run.

**It has a known false positive, documented rather than fixed.** Only a *leading*
`//` is excluded, so a bare `Theme.` inside a `/* */` block comment, or in a
trailing comment after code on the same line, fails the arm while being
perfectly correct. Narrowing the exclusion to real bindings needs a QML parser,
which is the elaborate thing this gate deliberately is not — the whole step is a
grep whose failure names a file small enough to read. The cost of the limitation
is therefore a few seconds of diagnosis, and the step's comment says so
explicitly so that a red is checked against the cited line before anyone goes
looking for a collision that is not there.

### The rename's completeness is checked by the existing suite

A missed reference would leave a bare `Theme` in a file, which under
`qmltestrunner` resolves to nothing at all — so the component tests fail with
`Theme is not defined` rather than passing with a wrong value. The four existing
spec files still pass after the rename, `tst_identicon.qml` among them, and that
file reads six palette constants through the singleton. Combined with the CI
gate's reference arm returning empty, the rename is covered from both directions.

This is worth stating plainly because it is the *only* thing the test layer
checks here. **What the tests cover:** that every reference resolves, and that
the identicon's ink indexing still lands on the same seven constants.
**What they cannot see:** the collision itself, for the reason measured above.

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
