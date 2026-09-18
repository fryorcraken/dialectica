# The forbidden shapes, and what to reach for instead

**This is the canonical list. Every role file points here and none repeats it**,
because two copies of a rule drift and the wrong one gets read.

This page is the *operational* half of `CLAUDE.md`'s "How to work in this repo,
and what Bash costs" — that section carries the reasoning, this one carries the
table an agent checks a command against. Read `CLAUDE.md`'s section too; it is
where the exceptions live.

## Why this costs the user something

The permission setup blocks commands it cannot **statically analyse**, and each
block costs the user a **manual approval click**. A blocked agent also cannot
answer a status query while it is parked on the prompt, so from the outside it is
indistinguishable from a stalled one.

The goal is not to avoid Bash. It is to avoid the *shapes* that defeat the
analyser. One plain command per call, with an argument the checker can read, runs
silently.

## The shapes, and the replacement for each

Every row on the left costs a click. **Reach for the right-hand column instead** —
a ban with no named replacement just redirects the habit to the next unanalysable
shape, which is how three of the four measured incidents happened.

| Forbidden | Reach for |
|---|---|
| `\|` pipes — including `\| tail`, `\| head`, `\| grep`, `\| wc -l` | the `Grep` tool, with `output_mode: "count"` or `"content"` and `-n: true`. For a long command output, **run it plain and read all of it** |
| `&&`, `;` — any chaining | **one plain command per call.** Two calls cost nothing; one compound call costs a click |
| `$(…)`, backticks, `<(…)` | run the inner command as its own call and read its output |
| `for` / `while` loops | the `Grep` tool across the whole set at once, or `Glob` then one call per hit. A loop that builds a corpus file is a `Grep` that was never run |
| `>` and `>>` redirects | the `Write` tool, or `Edit` for a change to an existing file |
| heredocs (`<<'EOF'`) | the `Write` tool |
| shell globs (`src/*.rs`) | the `Glob` tool, or pass the directory and let the command recurse |
| `env VAR=value <cmd>`, or a bare `VAR=value` prefix | a wrapper that sets it — for QML that is `sh dialectica-ui/tests/run-qml-tests.sh`. For scaffold-gated Rust code, `nix build .#lgx` |
| `cd <dir> && <cmd>` | **the tool's own path flag** (`--manifest-path`, `-C` where it exists), or nothing at all: you are already in the right tree |
| `cat`, `head`, `tail`, `ls` | the `Read` tool, with `offset` / `limit` to slice |
| `sed -i`, any in-place edit | the `Edit` tool, which refuses a string that is missing or non-unique where `sed -i` silently changes every match or none and exits 0 either way |
| `qmltestrunner` invoked directly | `sh dialectica-ui/tests/run-qml-tests.sh <spec>` — see below, this one is a correctness rule too |
| `--jq` on a `gh` call | run `gh` plain and read the JSON. `gh` is free **until you filter it** |
| materialising an old version of a file to diff it | `git diff origin/main -- <path>`, one plain command |
| reads outside the working directories — `/tmp`, the session scratchpad, an npm package unpacked anywhere | `./tmp/` **inside your own worktree**, which is gitignored, visible to the reviewer, and survives where the work is |

## Two things that are easy to get wrong

**Use relative paths inside your own worktree.** You arrive in the right tree, so
`dialectica-ui/tests/run-qml-tests.sh` resolves. A long absolute path into your
own tree is unnecessary, and it is where a typo becomes a *blocked read* rather
than a missing file — one measured incident was exactly that, a mistyped username
in a path that pointed at nothing the checker would allow. Absolute paths remain
the rule for anything reaching **outside** the tree you are standing in.

**A long output is not a reason to pipe.** This is the most common way the rule
gets broken by someone who knows it: appending `| tail -30` to keep the output
manageable turns a call the checker would have approved into a prompt, which is
the opposite of what the pipe was for.

## `qmltestrunner` is a correctness rule, not only a cost rule

**Never invoke `qmltestrunner` directly.** The bare name resolves to Qt5 here and
exits 1 with **no output at all**, which reads exactly like your change having
broken the suite. The script picks the Qt6 binary, sets the import path and the
offscreen platform, and runs `check_bindings`, which is what turns an undefined
binding from a warning into a failure:

```
sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_<name>.qml
```

With no argument it runs the whole suite.

## One shell trap worth not re-learning

`tar tzf … | grep -q` exits **141**: `grep -q` closes the pipe at the first match
and `tar` dies of SIGPIPE. Under a bare `set -eu` that is invisible, and it
becomes a spurious failure the moment anyone adds `-o pipefail`. Inside a script
— where a pipe costs no approval click, because the script is the one analysed
command — use `[ "$(… | grep -c …)" -gt 0 ]`, which consumes all the output.

## The fallback

**If a task cannot be done within these shapes, stop and report it.** Do not
improvise around the block: a blocked agent someone can unblock costs far less
than a stalled session, and the workaround an agent reaches for is reliably
another unanalysable shape.
