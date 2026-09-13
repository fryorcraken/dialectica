#!/usr/bin/env python3
"""No QML type of ours may be named something the HOST also registers.

Run: dialectica-ui/tests/check_qml_names.py [module-root]
     (module-root defaults to the `dialectica-ui` beside this file)

Exits 0 with a one-line summary, or 1 having printed a `::error::` line per
problem. The summary names the file count and the entry count it checked, so a
walker that silently stops matching is visible rather than silent.

WHY THIS IS A FILE RATHER THAN A HEREDOC IN ci.yml. It was a heredoc, and a
heredoc cannot be tested: the only way to exercise it was to push a branch and
read a CI log. `tst_check_qml_names.py` beside this file now runs it against
built trees and pins both directions — which is the standard this repo already
holds `check_bindings` to, for the same reason. A gate is part of the
measurement, and a gate narrowed until it matches nothing passes exactly as
quietly as a correct one.

WHERE THE COLLISION LIVES, which is the load-bearing fact: in the HOST's C++
type registration, which no file-based import path can reach. The full
mechanism — including the precedence premise this project held and withdrew,
and why no component test can see the defect — is in CLAUDE.md's "Module
contract traps". It is not repeated here: it was in four files, they must
change together, and CLAUDE.md is where a person adding a singleton meets it.

WHAT A RED FROM THIS GATE MEANS, which is the part a reader of a failure needs:

  * a `qmldir` entry whose declared type name is not `D`-prefixed — add the
    `D`, do not add an exemption (see GRANDFATHERED below);
  * a `qmldir` entry naming a file that is not there — a load failure basecamp
    makes hard to see, caught here instead;
  * a `.qml` body referencing a name we declare as `DX` by its bare `X` — a
    stale reference left by a rename, which resolves to the host's object (or
    to nothing) and reads `undefined` at runtime.

WHAT IT CANNOT SEE, stated because a gate that measured nothing is worse than
none: it proves a NAME absent, not that resolution is correct. Whether
basecamp actually registers any given name is invisible from the checkout —
that half is a real basecamp launch, and the `D` prefix is what makes the
question not need asking.
"""

import pathlib
import re
import sys

# WHAT THE HOST NAMESPACE ACTUALLY CONTAINS, measured from a real Basecamp
# launch log rather than guessed, because it settles why this gate is a prefix
# rule and not a list of names. Enumerated from
# `.scaffold/basecamp/profiles/alice/xdg-data/Logos/LogosBasecampDev/logs/
# basecamp_<timestamp>.log` (NOT `basecamp.log`) on a build carrying the old
# name — 29 distinct types under `qrc:/qt/qml/Logos/`:
#
#   Controls/  — 23 types, EVERY ONE `Logos`-prefixed (LogosButton, LogosText,
#                LogosTable, LogosDialog, …)
#   Icons/     — LogosIcons
#   Theme/     — ColorPalette, DarkTheme, Spacing, THEME, Typography
#
# Two things follow, and both are load-bearing:
#
# 1. The host's own convention is a `Logos` prefix, and the five unprefixed
#    names are all inside `Theme/`. So a `D` prefix on our side cannot collide
#    with anything the host ships TODAY — and, more usefully, cannot collide
#    with anything it ships later either, unless the host abandons its own
#    convention. That is why this is a prefix rule: a list of those 29 names
#    would be correct only until the 30th, which is the
#    `hand-maintained sweep lists go stale silently` trap this repo has paid
#    for. The measurement supports the prefix rule; it is not an argument for
#    replacing it with the list.
#
# 2. `Core` was CHECKED, not assumed. The same log shows 27 resolutions into
#    `dialectica_ui/qml/Core.qml` and ZERO into `qrc:/qt/qml/Logos/Core*` —
#    against 209 host resolutions for `Theme` and zero into the plugin's own.
#    `Core` does not collide. Its grandfathering is therefore a live fact with
#    evidence, not an untested hope; see the note below on why it is still the
#    wrong shape to depend on.
#
# The 29-type enumeration reproduces across two independent launches, of two
# different branches, on the same machine — identical set both times. So it is a
# property of the host build rather than of one run. That is what makes claim 1
# worth anything: a namespace measured once could be a coincidence of timing.

# `Core` is grandfathered, and every component entry below it is grandfathered
# for the same structural reason with a weaker excuse. The exemptions are
# written here with their reasons rather than left as bare names in a list.
#
# `Core` predates the convention and `piece/theme-unshadow` deliberately did
# not rename it: that piece's scope was the one collision that stopped the
# screen rendering, and a diff doing two things cannot be reviewed for either.
# `Core` is also the evidence that the host does NOT claim every plausible
# name — it resolved correctly throughout the outage, in the same files, under
# the same imports, purely because basecamp has no `Core`.
#
# The ELEVEN COMPONENT NAMES are here because the architecture review found
# them silently exempt: the rule read only `singleton` lines, so a plain
# component re-registering a host name passed green. They are QML type names in
# the same directory namespace as the singletons and are shadowable in exactly
# the same way — the host's verified launch log registers `LogosButton.qml`,
# which is a COMPONENT, not a singleton. Naming them here trades one silent gap
# for eleven visible ones: nothing new is protected today, but the twelfth
# entry cannot be added without either a `D` or a deliberate edit to this set.
#
# That is why this is a grandfather clause and not a precedent: "basecamp has
# no `FlatButton` TODAY" is a fact with no expiry date attached, and the whole
# point of the prefix rule is to stop depending on such facts. Renaming these
# is the right end state and belongs to a piece of its own.
#
# DO NOT ADD A NAME HERE — add the `D` instead.
#
# SCOPE OF THE EXEMPTION, because a reader who hits a red from the reference
# check may otherwise reach for this set and find it does nothing: this set
# exempts a name from the PREFIX rule only. It does not exempt a missing file,
# and it does not exempt a bare reference — a `.qml` body writing `Theme.x`
# fails no matter what is in here.
GRANDFATHERED = {
    "Core",
    "Identicon",
    "AddressLabel",
    "VoteControl",
    "PostHeader",
    "MarginNote",
    "ApparatusColumn",
    "FlatButton",
    "ScreenFrame",
    "SanitisedText",
    "FeedScreen",
}

# A qmldir type name that is ours-by-convention: `D` then an upper-case letter.
PREFIXED = re.compile(r"D[A-Z][A-Za-z0-9]*\Z")


def strip_comments(src):
    """`//` to end of line, and `/* */` however many lines it spans.

    Stripping once up front rather than filtering per-check means there is one
    place to be right about what a comment is, and the next check added here
    inherits it instead of re-deriving it. (The Rust-side gate at the top of
    ci.yml strips leading `//` only, which is correct for Rust; the two are
    deliberately separate and `design.md` says why.)

    Block comments are replaced by the newlines they spanned, so every reported
    line number still points at the line a human will find in the file.
    """
    src = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), src, flags=re.S)
    return re.sub(r"//.*$", "", src, flags=re.M)


def qmldir_entries(qmldir_text):
    """Every type a `qmldir` declares, as (line number, name, file-or-None).

    EVERY entry, not only `singleton` ones. The previous form read
    `parts[0] == "singleton"` and skipped the rest, so `Theme 1.0 Identicon.qml`
    — a plain component declaring the exact name this piece exists to remove —
    passed the gate green. Measured by the architecture review.

    Command lines (`module`, `depends`, `import`, `optional`, `prefer`) are not
    type declarations and are skipped; `internal Foo Foo.qml` declares a type
    whose name is still registered in the directory namespace, so it is not.
    """
    commands = {"module", "depends", "import", "optional", "prefer",
                "typeinfo", "classname", "plugin", "designersupported"}
    for n, line in enumerate(qmldir_text.splitlines(), 1):
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split()
        if parts[0] in commands:
            continue
        if parts[0] == "singleton" or parts[0] == "internal":
            parts = parts[1:]
        if not parts:
            continue
        name = parts[0]
        target = parts[2] if len(parts) >= 3 else None
        yield n, name, target


def check(module_root):
    status = 0
    module_root = pathlib.Path(module_root)

    # EVERY .qml in the module, not just `src/qml/*.qml`. The previous form
    # globbed `src/qml/*.qml`, which left `dialectica-ui/tests/` outside the
    # gate entirely. rglob also reaches a subdirectory under `src/qml/`, should
    # one ever be added; the old glob could not.
    qml_files = sorted(module_root.rglob("*.qml"))
    if not qml_files:
        print("::error::no QML files found — this gate measured nothing")
        return 1

    qmldir = module_root / "src/qml/qmldir"
    if not qmldir.exists():
        print(f"::error::{qmldir} not found — this gate measured nothing")
        return 1

    entries = list(qmldir_entries(qmldir.read_text()))
    if not entries:
        print(f"::error file={qmldir}::declares no types — this gate measured nothing")
        return 1

    # ---- 1. every type we declare is D-prefixed -------------------------
    for n, name, _target in entries:
        if name in GRANDFATHERED:
            continue
        if not PREFIXED.match(name):
            print(f"::error file={qmldir},line={n}::qmldir declares '{name}', "
                  "which is not D-prefixed. Basecamp registers its own QML "
                  "types in the host's C++ type registration, and a name it "
                  "also uses resolves to ITS type — every property then reads "
                  "undefined at runtime. The `D` is what makes the name "
                  f"unclaimable. Rename to 'D{name}'.")
            status = 1

    # ---- 2. the file backing each declaration exists ---------------------
    # A qmldir naming a file that is not there fails at load, not here, and the
    # load failure is the one basecamp makes hard to see.
    for n, name, target in entries:
        if target is None:
            continue
        if not (qmldir.parent / target).exists():
            print(f"::error file={qmldir},line={n}::declares '{target}', "
                  "which does not exist")
            status = 1

    # ---- 3. no binding reaches a declared type by its unprefixed name ----
    #
    # DERIVED FROM qmldir, not a hardcoded literal. The previous form searched
    # for the single word `Theme`, so it detected the collision that already
    # happened and no other: once `Core` becomes `DCore`, a surviving bare
    # `Core.` reference would have been invisible, and the piece would have
    # repeated itself. For each type declared as `DX`, a bare `X` in any `.qml`
    # body is a stale reference from the rename.
    stale = {name[1:]: name for _n, name, _t in entries if PREFIXED.match(name)}

    # AND the names arm 1 just rejected. A type declared WITHOUT the `D` is a
    # name we are asking to be renamed, so every reference to it is a reference
    # this gate should name — otherwise a tree mid-rename (or `main`, which
    # still declares `Theme`) reports the one qmldir line and stays silent
    # about the 116 bodies that must change with it. Measured: without this,
    # the gate run against `main` printed the qmldir error and nothing else.
    for _n, name, _t in entries:
        if name not in GRANDFATHERED and not PREFIXED.match(name):
            stale.setdefault(name, "D" + name)

    if not stale:
        print(f"::error file={qmldir}::no D-prefixed type declared, so the "
              "stale-reference check has nothing to look for — either the "
              "convention was abandoned or this gate stopped reading qmldir")
        return 1

    pattern = re.compile(
        # ANCHORED WITHOUT REQUIRING A PRECEDING CHARACTER, which the previous
        # regex was not. It used `[^A-Za-z]Theme\.`, so a binding split across
        # two lines with `Theme.note` at column 0 was invisible to it — a
        # genuinely broken binding that passed this gate, qmllint AND the QML
        # suite, all green. `\b` on the trailing side catches `Theme` split
        # from `.ink`, the other line-break form that escaped the old regex.
        r"(?<![A-Za-z])(" + "|".join(sorted(stale, key=len, reverse=True)) + r")\b"
    )
    for path in qml_files:
        code = strip_comments(path.read_text())
        for n, line in enumerate(code.splitlines(), 1):
            m = pattern.search(line)
            if m:
                bare = m.group(1)
                print(f"::error file={path},line={n}::references a bare "
                      f"'{bare}' — dialectica's type is '{stale[bare]}'. A bare "
                      f"{bare} resolves to basecamp's registration, so every "
                      "property read from it is undefined at runtime.")
                status = 1

    if status == 0:
        print(f"ok: {len(qml_files)} QML file(s) and {len(entries)} qmldir "
              f"entr{'y' if len(entries) == 1 else 'ies'} checked, every "
              "declared type D-prefixed or named as grandfathered, no bare "
              "reference to a D-prefixed type")
    return status


if __name__ == "__main__":
    if len(sys.argv) > 2:
        print("usage: check_qml_names.py [module-root]", file=sys.stderr)
        sys.exit(2)
    root = sys.argv[1] if len(sys.argv) == 2 else \
        str(pathlib.Path(__file__).resolve().parent.parent)
    sys.exit(check(root))
