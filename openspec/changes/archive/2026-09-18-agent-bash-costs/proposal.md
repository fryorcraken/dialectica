# Give every role file the forbidden Bash shapes and their replacements

## Why

In one session, four dispatched agents cost the user manual approval prompts by
using shapes `CLAUDE.md` forbids. The user complained three times. The shapes
were:

- `QT_QPA_PLATFORM=offscreen qmltestrunner …` — an `env VAR=value` prefix plus a
  bare runner, where `sh dialectica-ui/tests/run-qml-tests.sh <spec>` exists and
  sets that variable itself;
- unpacking an npm package into the session scratchpad and grepping it — reads
  outside the working directories, refused on every call;
- a `for` loop with `$(…)` and a `>` redirect, building a corpus file the `Grep`
  tool would have produced directly;
- a `;`-chained pair of absolute-path greps, one carrying a **typo'd username**
  in the path — which is what turned it into a blocked read.

**Measured root cause**, re-run in this worktree rather than taken on trust:

```
grep -c "never chain\|one plain command\|costs a click\|approval click" \
  .claude/agents/{design-reviewer,code-reviewer,dev-writer,spec-writer,tester,spec-test-reviewer,closer}.md
```

returns **0 for five of seven files**. Only `tester.md` and
`spec-test-reviewer.md` score 1, and each of those carries a single narrow
sentence about its own suite rather than the list.

`README.md` has a section headed "Every agent pays CLAUDE.md's Bash costs", and
it names the chaining rule and the pipe rule. But `README.md` is not what a
dispatched agent necessarily reads: the brief names the piece and the findings
directory, and the role file is the document the agent is *made of*. An agent
reaching for a loop has no local rule stopping it.

The property that was missing, and that this change restores:

> **An agent that reads only its own role file must come away knowing the
> forbidden shapes and their replacements.**

## What changes

- A new canonical list, `.claude/agents/BASH-COSTS.md`, carrying every forbidden
  shape with the replacement to reach for instead — because `CLAUDE.md` is
  emphatic that a ban without a named replacement merely redirects the habit.
- Each of the seven role files gains a short, uniform pointer section carrying
  the one-line summary and the link. No file carries a second copy of the list.
- `README.md`'s existing section becomes a pointer to the same file, so the two
  cannot drift.
- A check, `.claude/agents/tests/check_agent_bash_costs.sh`, failing when a role
  file lacks the pointer, with `tst_check_agent_bash_costs.sh` beside it pinning
  both directions.

## Capabilities

None. This change alters agent instructions and adds a check over them; it adds
no requirement to `openspec/specs/` and declares `skip_specs: true` alongside a
`schema:` key in `.openspec.yaml`.

It is still reviewed. A change with no source diff is not exempt — agent
instruction files are reviewable material, and `README.md` says so.
