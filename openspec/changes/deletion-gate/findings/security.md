# Security findings — `deletion-gate`

> **Note from `dev-writer`, appended without editing anything below.** Both
> boxes here are addressed to the `tester` and stay open; neither is mine to
> tick. Both name script-level defects I have since fixed, so the code has
> moved under them:
>
> - **`-F` and the crafted claim** — `-F` was always present; what was missing
>   was a test pinning it. Test 17 now deletes `sub/lib.rs` and `sub/store.rs`
>   and asserts `Deletes: sub/lib?rs` absorbs neither. Reverting `-F` turns it
>   red, and the failure names `sub/lib.rs` specifically.
> - **fenced claim** — fixed in the script rather than only tested: fenced
>   blocks are removed before claims are extracted, so an example in a code
>   block is no longer an assertion. An unclosed fence swallows the rest of the
>   body, which is deliberately the safe direction — claims go missing, so the
>   gate fails a deletion rather than accepting one.
>
> Thank you for the injection audit; it is the part of this piece I had least
> ability to check myself.

Reviewed at `94ef230` in `.claude/worktrees/piece-deletion-gate`. The threat
model taken: anyone who can open a pull request controls `PR_BODY` entirely, and
on a fork PR controls the branch contents too. The questions asked were (a) can
that text execute anything, (b) can it make a genuine deletion look claimed, and
(c) can it make the gate pass having measured nothing.

## Defects

- [ ] **`tester`** — `.github/scripts/check-claimed-deletions.sh:164` — a
      crafted PR body can claim a deletion it does not name, if `-F` is ever
      lost from the lookup
      **Scenario:** the claim lookup is `grep -qxF -- "$path" "$claimed_list"`,
      where the *deleted path* is the pattern and the *body's claims* are the
      haystack. `-F` is the only thing making that a literal comparison, and it
      is unpinned by any test. Drop it and a body reading `Deletes: src/lib?rs`
      is accepted as claiming a deletion of `src/lib.rs`. That is an
      attacker-supplied pattern matching an attacker-chosen path: a fork PR can
      write a claim line naming a path that does not exist in the tree and have
      it acknowledge one that does.
      **Measured:** 23 of 23 tests pass with `-F` removed; the exploit was run
      against a fixture deleting `src/lib.rs` and `src/store.rs`, where only
      `src/lib.rs` was absorbed by the crafted claim. Filed on the correctness
      sheet too, because it is both; one box, here, is enough to act on.
      **Severity:** high — silent pass on a real deletion, and the mechanism is
      attacker-controlled text reaching a pattern position.

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:224`
      (test 8) — a claim placed inside a Markdown code fence is treated as a
      genuine claim
      **Scenario:** a PR body whose three lines are a triple-backtick fence
      open, then `Deletes: sub/doomed.txt`, then a fence close, passes the gate
      for a real deletion of `sub/doomed.txt`. To a human reading the rendered
      PR this is an example, not an assertion; to the gate it is a claim. An
      author wanting a deletion to slip past a reviewer has a form that looks
      like documentation. design.md §4 names exactly this hazard — a body that
      "may quote this very check" — and test 8 covers only the inline-sentence
      form, which the leading anchor rejects for an unrelated reason.
      **Measured:** reproduced against the standard fixture. Blockquote, bold
      `**Deletes:**`, lowercase, HTML comment, numbered list and nested bullet
      all correctly fail, so the fence is a specific gap and not general
      looseness.
      **Severity:** medium — requires a deliberately shaped body, and the gate
      is an honesty aid rather than an access control, but it defeats the one
      structural property (anchoring) the design relies on.

## What was clean

**No shell injection, and the author's claim about it is exact.** `PR_BODY`
reaches the step through `env:` and is written to a file with
`printf '%s' "$PR_BODY"` — quoted, so a body containing `$(id)`, backticks,
`;`, `&&`, newlines or a leading `-` is written verbatim and never evaluated.
The script then reads that file only with `sed -n` and `grep -qxF -- `, both of
which take it as data. `--` guards the `grep` against a path or claim beginning
with a dash. I found no path by which body text becomes a command, an option, or
a filename.

**`${{ }}` interpolation was audited across the whole workflow.** The only
expressions in `ci.yml` are `github.ref` (line 26, in `concurrency.group`, not a
shell context), `github.event.pull_request.body` and
`github.event.pull_request.base.sha` (lines 106-107, both `env:` assignments,
not `run:` bodies), and `runner.os` / `env.LGS_VERSION` (1045, 1052). None is
attacker-controlled text expanded into a script. The author's claim that this is
the only step handling untrusted input in the file is correct.

**`BASE_SHA` cannot be steered.** It comes from `github.event.pull_request.base.sha`,
which GitHub computes; a fork PR author cannot substitute a commit of their
choosing to make the diff empty. It is passed as a single quoted argument to
both `git fetch` and the script, and `git rev-parse --verify "$ref^{commit}"`
rejects anything that is not a resolvable commit before it is used in a range.

**Permissions posture is right for this step.** The workflow sets
`permissions: contents: read` at the top level, so even if the gate step were
compromised it holds no write token. This matters because the step runs on
`pull_request` (not `pull_request_target`), so a fork PR's code runs with no
secrets and no write access — the correct trigger choice, and one worth not
losing in a later edit.

**Temp-file handling is sound.** `mktemp` for all three lists, a
`trap ... EXIT` that removes them, and every failure to create one routed
through `cannot_measure` (exit 1) rather than continuing with an unset variable
— `set -u` would catch that anyway. No predictable filename, no world-writable
directory, nothing appended to a path an attacker names.

**No error message leaks anything the caller should not learn.** The failure
output contains deleted paths, the merge-base SHA and the PR body's own claims
— all of which the PR author already has.

**Wildcard and prefix claims are correctly refused.** `Deletes: docs/` does not
acknowledge `docs/anything`, and `Deletes: src/.*` acknowledged nothing with
`-F` in place; the exact-equality comparison holds, which is what stops a single
claim line from blanket-authorising a tree.

## What I could not check

Live GitHub Actions behaviour: the exact JSON shape of
`github.event.pull_request.body` for a PR with no description (locally both an
empty string and the literal string `null` behave correctly — a deletion still
fails), and whether `$RUNNER_TEMP` is writable at that point. Neither is a
security question on the evidence I have; both abort the step loudly under
`set -eu` if they fail.
