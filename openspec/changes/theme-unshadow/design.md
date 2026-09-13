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
carrying the defect — it exits 1 reporting the `qmldir` singleton as
un-prefixed and **116 lines** carrying a bare `Theme` reference (113 under
`src/qml/`, 3 in `tests/tst_identicon.qml`). Run against this tree, it passes.
That is a gate with a demonstrated failing case, not a step that has only ever
been seen green.

The figure is the gate's own output, counted. An earlier draft of this document
said "80-odd", which no reading of the command produces; the correctness review
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
anything basecamp adds later". A rule that every declared singleton is
`D`-prefixed is total over host registrations that have not happened yet, where
a list is total over none of them.

`Core` is grandfathered, with the reason in the step rather than as a bare name:
it predates the convention, this piece deliberately did not rename it, and it is
itself the evidence that the host does not claim every plausible name. That is
also why it is a grandfather clause and not a precedent — "basecamp has no
`Core` today" is exactly the kind of fact the prefix rule exists to stop
depending on. `DCore` is the right end state and belongs to a piece of its own.

### Comments are stripped once, and the false positive is gone

The previous version bolted a `grep -v ':[0-9]*: *//'` onto the one arm that
needed it, and documented the resulting false positive: a bare `Theme.` in a
`/* */` block comment, or trailing a line of code, failed the arm while being
correct.

The gate now strips comments — `//` to end of line, and `/* */` however many
lines it spans — **once, up front**, so every check runs against clean source.
This is `piece/publish-envelope`'s shape, and the reason to prefer it is not
style: there is one place to be right about what a comment is, and the next
check added to this step inherits it instead of re-deriving it.

Measured: both comment forms that used to fail the old regex now pass, and the
`DTheme.qml` header — which quotes `Theme.x` twice while explaining the
collision — is clean without needing an exclusion written for it. Block comments
are replaced by the newlines they spanned, so every reported line number still
points at the line a human will find in the file.

### What actually checks the rename, and what does not

**The suite is not a second line of defence, and an earlier draft of this
document said it was.** That claim — that the CI arm and the component tests
cover the rename "from both directions" — is false, and the correctness review
measured why: `qmltestrunner` reports a `ReferenceError` raised inside an
instantiated component as a **QWARN, not a failure**. Reproduced here: the
column-0 mutation in `MarginNote.qml` printed `ReferenceError: Theme is not
defined` dozens of times across the `FeedStates` spec, and the runner still
reported `12 passed, 0 failed`, whole suite exit 0.

So the two do not cover each other; they shared a blind spot. The suite catches
a stale reference **only** when it sits inside a `compare()` — as
`tst_identicon.qml`'s palette assertions do, which is why that one spec does
fail on a stale reference while the rest stay green.

That leaves the honest division:

- **The CI gate** is what checks the rename's completeness, across all 17 QML
  files in the module. It is the only check that sees a stale reference in a
  component no spec asserts against.
- **The suite** pins that the identicon's ink indexing still lands on the same
  seven constants, and would fail on a stale reference inside its own
  assertions.
- **Neither can see the collision itself**, for the reason measured above.

Making the runner fail on a `ReferenceError` in its output would close this
properly and is `tester`'s box in `findings/correctness.md`, not this one's.

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
