## Why

**The screen cannot render, and one identifier is why.**

`dialectica-ui/src/qml/qmldir` declared `singleton Theme 1.0 Theme.qml`.
Basecamp registers a QML type of the same name in its own C++ type
registration, and basecamp's won. So every `Theme.x` in every one of our QML
files resolved to basecamp's object rather than to ours, every token came back
`undefined`, and QML fell back to its defaults: white ground, black system text,
no spacing, no borders. The reported symptom was 89 errors on a screen with none
of the supplied design on it.

**The collision lives in that C++ registration**, which is the fact the fix and
its gate both rest on. A premise held earlier and withdrawn — that a host
registration *outranks* a plugin directory's `qmldir` entry, the two competing
on precedence — was measured and is false; `design.md` records the two
measurements that disproved it, because it is the intuitive wrong answer and
someone will otherwise re-derive it.

The scale is out of proportion to the cause, and that is the point worth
recording: this is not a theme that renders badly, it is **the entire visual
system removed by one name collision**. Every component in the directory reads
its colours, fonts, spacing and metrics from that singleton, so a single
shadowed identifier takes all of them at once.

`Core`, the other singleton in the same directory, resolved correctly in the
same files under the same imports — purely because basecamp has no `Core`. "Our
other singleton works" was therefore never evidence that a name was safe, and
this change is the reason to stop treating it as such.

**This fix has been written once already and was lost.** It was commit
`2237a45` on branch `ui/layout-fix`, which went out with PR #28 and was closed
for reasons unrelated to the shadowing. Three UI PRs have merged since (#23,
#24, #26) and the screen still cannot render, because the one-line cause went
out of the tree with a PR that was about something else. That commit bundled
three independent fixes; **only the shadowing one is re-landed here**, because a
diff doing three things cannot be reviewed for any of them.

## What Changes

- **The singleton is renamed `Theme` → `DTheme`**, in `qmldir` and in the
  filename, and every reference in the view is updated to match. A prefix rather
  than a module URI: a name nothing in the host can claim cannot be shadowed by
  anything basecamp registers later, whereas a URI only moves the collision to
  whichever namespace the host next occupies.
- **No token changes name, value or meaning.** `DTheme.qml` is `Theme.qml` with
  an added header comment explaining the collision. The identicon's seven-ink
  palette — wire-visible frozen constants — is byte-identical, so no identity's
  mark changes appearance.
- **A CI gate is added**, `no QML type name collides with the host`, which fails
  when a `qmldir` singleton is not `D`-prefixed, when a declared singleton's file
  is missing, or when any QML file in the module still references a bare `Theme`.
  It enforces the **prefix convention** rather than a list of known host names,
  because a list is correct only until the host registers a name nobody put on
  it; it reads every `.qml` under `dialectica-ui/`, tests included; and it strips
  comments once up front so a file's own prose about the collision cannot trip
  it.
- **No behaviour changes anywhere else.** Nothing in `dialectica/` is touched,
  no core method moves, no QML file gains or loses an element.

## Capabilities

### New Capabilities

None. Renaming an identifier adds no requirement: the tokens' names, values and
meanings are unchanged, and nothing a view may claim or must refuse to claim
moves. `.openspec.yaml` sets `skip_specs: true` with that reason.

### Modified Capabilities

None.

## Impact

- **Renamed:** `dialectica-ui/src/qml/Theme.qml` → `DTheme.qml`.
- **Modified:** `dialectica-ui/src/qml/qmldir`, eleven QML components under
  `dialectica-ui/src/qml/`, and `dialectica-ui/tests/tst_identicon.qml`, which
  reads the palette to pin the identicon's ink indexing.
- **Modified:** `.github/workflows/ci.yml` — one new step in the `qml` job.
- **Modified:** `CLAUDE.md` — the trap, under "Module contract traps", since it
  is a build/launch-time failure of exactly the kind that section collects.
- **Expected to collide with UI work in flight.** Four UI pieces
  (`ui-onboarding`, `ui-composer`, `ui-stoa-list`, and a docs sweep) are being
  written against `Theme.`, so each will need the same rename applied to whatever
  it adds. That is a mechanical conflict with a mechanical resolution, and it is
  cheaper now than after four more screens are written against a name that
  cannot work.
- **`tmp/ui-design/handoff/qml/` is deliberately not touched.** It is an
  untracked copy of the design handoff, not a build input.
