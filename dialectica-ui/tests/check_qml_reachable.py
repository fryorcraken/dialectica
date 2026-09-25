#!/usr/bin/env python3
"""Every registered QML type is reachable from the view's root, or says why not.

Run: dialectica-ui/tests/check_qml_reachable.py [module-root]
     (module-root defaults to the `dialectica-ui` beside this file)

Exits 0 with a one-line summary, or 1 having printed a `::error::` line per
problem.

WHY THIS GATE EXISTS. Five registered types were instantiated nowhere, and 92
test functions passed over four of them. The worst was `DOnboardingScreen` — the
view's ONLY route to acquiring an identity — so a fresh install could not obtain
a key at all while a green suite reported the screen works. It did work. It was
simply unreachable.

**No component-level test can catch this, and that is the whole point.** A
component suite instantiates the component under test itself, so it supplies
exactly the reachability whose absence is the defect: the component is reachable
FROM THE TEST no matter what the application does. So this reads the
registrations and the view's sources together instead of asserting against
anything it constructed.

WHY REACHABILITY IS COMPUTED TRANSITIVELY, which is the load-bearing decision.

The cheap version of this check is "does this type's name appear in any .qml?".
It is one grep and it is WRONG HERE, measurably: `DTip` is instantiated by
`DStatusBar.qml` and by `DVouchStamp.qml`, and before the change that added this
gate BOTH of those were themselves unmounted. A mention-based check reports
`DTip` reachable on the strength of two callers no user could reach either — it
passes the entire unreachable island, which is the exact defect it was built to
find. A gate whose input the defect satisfies reports clean, and that is worse
than measuring nothing, because it closes the question.

So: start at the root, find what it instantiates, follow those into their own
sources, repeat to a fixed point. `tst_check_qml_reachable.py` pins this with a
three-file tree where B instantiates C and nothing instantiates B; under a
mention-based check C reads as reachable, so that case is what distinguishes the
two implementations.

WHY THE RECORD LIVES IN `qmldir` AND NOT IN THIS FILE. A list of exempt names
inside the checker is a place a new type can be added quietly — this repo's
`hand-maintained sweep lists go stale silently` trap, which `check_qml_names.py`
carries a warning about in its own header for the same reason. A comment beside
the registration is edited by the same hand that adds the registration, in the
same file, in the same diff. The reason travels with the thing it excuses, and a
type added to `qmldir` with no record is reported without anyone maintaining
anything.

WHAT IT CANNOT SEE, stated because a gate that measured nothing is worse than
none: it proves a type is INSTANTIATED somewhere the root reaches, not that a
user can actually get to it at runtime. A screen mounted behind a condition that
is never true is reachable by this gate and dead to a user. That half is a real
launch — the sitometres specs in `tests/ui/`, run by `ui-tests.yml`, which
cover only the screens their specs drive. This is the static half.
"""

import pathlib
import re
import sys

# The view's root — where reachability starts. Not configurable: there is one
# root, basecamp loads it by name, and a flag here would be a second place to
# get it wrong.
ROOT_QML = "Main.qml"

# A record marking a registration as deliberately uninstantiated. The text after
# the colon is the reason, and it must be non-empty: "intended" without a reason
# is indistinguishable from an oversight someone waved through.
UNINSTANTIATED = re.compile(r"#\s*UNINSTANTIATED:\s*(\S.*)$")

# A type instantiated in a QML body: `TypeName {`, possibly with an id or
# properties after the brace. Anchored so `foo.TypeName {` and `DTypeName` do
# not match a shorter name.
#
# This deliberately does NOT try to parse QML. It over-approximates — a name
# followed by a brace inside a string literal would count — and over-approximation
# is the safe direction here: it can only make an unreachable type look reachable
# if someone writes its exact name followed by `{` in a comment or string, and
# comments are stripped before the scan.
INSTANTIATION = re.compile(r"(?<![A-Za-z0-9_.])([A-Z][A-Za-z0-9_]*)\s*\{")

# A singleton is never instantiated — it is USED, as `DTheme.paper`. Asking
# whether anything writes `DTheme { }` is a category error that reports every
# singleton in the module as dead code, which is what the first version of this
# gate did: it flagged `DTheme`, `Core` and `DStoaReference`, all three of them
# reached constantly.
#
# So a singleton is reachable when something the root reaches READS a member off
# it. That is the honest analogue of instantiation for a type that cannot be
# instantiated, and it fails in the right direction: a singleton nothing reads
# is genuinely dead, and one read only by an unreachable type is still caught,
# because the walk that finds the read is the same transitive walk.
SINGLETON_USE = re.compile(r"(?<![A-Za-z0-9_.])([A-Z][A-Za-z0-9_]*)\s*\.")


def strip_comments(src):
    """`//` to end of line, and `/* */` however many lines it spans.

    The same shape `check_qml_names.py` uses, and for the same reason: a type
    named in a comment is not an instantiation, and a gate that counted one
    would report a type reachable because somebody mentioned it in prose. That
    is not hypothetical — `Core.qml` names `DOnboardingScreen` in a comment
    while instantiating nothing.
    """
    src = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), src, flags=re.S)
    return re.sub(r"//.*$", "", src, flags=re.M)


def qmldir_records(qmldir_text):
    """Every declared type, as (line, name, reason-or-None, is-singleton).

    The reason is taken from the nearest preceding `# UNINSTANTIATED:` comment
    block. Adjacency is what ties a reason to its registration: a reason
    anywhere in the file would let one comment excuse a type added ten lines
    later by someone who never read it.

    `is-singleton` is carried because a singleton is reached by a member read
    rather than by instantiation — see SINGLETON_USE.
    """
    commands = {"module", "depends", "import", "optional", "prefer",
                "typeinfo", "classname", "plugin", "designersupported"}
    pending = None
    for n, line in enumerate(qmldir_text.splitlines(), 1):
        stripped = line.strip()

        if not stripped:
            # A blank line breaks the association. Without this, a record at the
            # top of the file would attach to whatever type came next after any
            # amount of unrelated text.
            pending = None
            continue

        if stripped.startswith("#"):
            found = UNINSTANTIATED.search(stripped)
            if found:
                pending = found.group(1).strip()
            continue

        parts = stripped.split()
        if parts[0] in commands:
            pending = None
            continue
        singleton = parts[0] == "singleton"
        if parts[0] in ("singleton", "internal"):
            parts = parts[1:]
        if not parts:
            pending = None
            continue

        yield n, parts[0], pending, singleton
        pending = None


def reachable_from(qml_dir, root_name, declared, singletons):
    """Every declared type the root reaches, transitively.

    `declared` maps a type name to the file that backs it, so the walk follows
    only types this module registers — a `Rectangle` or a `ColumnLayout` is not
    ours and leads nowhere we need to follow.

    **A singleton's members are followed too.** `Core.qml` is reached because
    screens write `Core.listThreads(...)`, never `Core { }`, and the types a
    singleton itself uses are reachable through it.
    """
    root_file = qml_dir / root_name
    if not root_file.exists():
        return None

    seen = set()
    # The root itself is reachable by definition; it is what basecamp loads.
    frontier = [root_name]
    while frontier:
        name = frontier.pop()
        if name in seen:
            continue
        seen.add(name)

        target = declared.get(name)
        source = qml_dir / (target if target else name)
        if not source.exists():
            continue

        body = strip_comments(source.read_text())

        # Instantiations reach a component; member reads reach a singleton. Both
        # are followed from the same body, because a screen does both.
        for pattern, wanted_singleton in ((INSTANTIATION, False),
                                          (SINGLETON_USE, True)):
            for found in pattern.finditer(body):
                child = found.group(1)
                # Follow only what this module declares, and only by the form
                # that actually reaches that kind of type: counting `Foo {` for a
                # singleton, or `Foo.` for a component, would let the wrong
                # syntax mark a type reached.
                if child not in declared or child in seen:
                    continue
                if (child in singletons) != wanted_singleton:
                    continue
                frontier.append(child)

    return seen


def check(module_root):
    status = 0
    module_root = pathlib.Path(module_root)
    qml_dir = module_root / "src/qml"
    qmldir = qml_dir / "qmldir"

    if not qmldir.exists():
        print(f"::error::{qmldir} not found — this gate measured nothing")
        return 1

    records = list(qmldir_records(qmldir.read_text()))
    if not records:
        print(f"::error file={qmldir}::declares no types — this gate measured nothing")
        return 1

    declared = {name: None for _n, name, _r, _s in records}
    singletons = {name for _n, name, _r, s in records if s}
    for line in qmldir.read_text().splitlines():
        parts = line.strip().split()
        if not parts or parts[0].startswith("#"):
            continue
        if parts[0] in ("singleton", "internal"):
            parts = parts[1:]
        if len(parts) >= 3:
            declared[parts[0]] = parts[2]

    if not (qml_dir / ROOT_QML).exists():
        print(f"::error file={qmldir}::{ROOT_QML} is missing, so reachability "
              "has no starting point and this gate measured nothing")
        return 1

    reached = reachable_from(qml_dir, ROOT_QML, declared, singletons)
    if reached is None:
        print(f"::error::{qml_dir / ROOT_QML} not found — this gate measured nothing")
        return 1

    # A walk that reached only the root found nothing to follow, which means the
    # instantiation pattern stopped matching rather than that the view is empty.
    # Silent narrowing is the failure mode a gate cannot report on itself.
    if len(reached) <= 1:
        print(f"::error file={qml_dir / ROOT_QML}::the reachability walk reached "
              "nothing beyond the root itself — the instantiation scan matched "
              "no declared type, so this gate measured nothing")
        return 1

    for n, name, reason, _singleton in records:
        if name in reached:
            if reason is not None:
                print(f"::error file={qmldir},line={n}::'{name}' is recorded as "
                      "deliberately uninstantiated, but the view's root DOES "
                      "reach it. Remove the record — a stale one excuses a type "
                      "nobody is holding back, and the next reader cannot tell "
                      "it from a live one.")
                status = 1
            continue

        if reason is None:
            print(f"::error file={qmldir},line={n}::'{name}' is registered and "
                  f"nothing {ROOT_QML} reaches instantiates it. A registered "
                  "type nothing instantiates compiles, renders correctly when a "
                  "test builds it, passes every assertion made about it, and "
                  "cannot be opened by a user. Either instantiate it, or record "
                  f"why not with a '# UNINSTANTIATED: <reason>' comment on the "
                  "line above.")
            status = 1

    if status == 0:
        unreached = [name for _n, name, r, _s in records if r is not None]
        print(f"ok: {len(records)} registered type(s), {len(reached) - 1} "
              f"reached from {ROOT_QML}, {len(unreached)} recorded as "
              f"deliberately uninstantiated ({', '.join(sorted(unreached))})")
    return status


if __name__ == "__main__":
    if len(sys.argv) > 2:
        print("usage: check_qml_reachable.py [module-root]", file=sys.stderr)
        sys.exit(2)
    root = sys.argv[1] if len(sys.argv) == 2 else \
        str(pathlib.Path(__file__).resolve().parent.parent)
    sys.exit(check(root))
