#!/bin/sh
# Pins check_agent_bash_costs.sh in BOTH directions: it must pass on a correct
# corpus and fail on each way of getting it wrong. A check that cannot fail is
# this repo's most-documented defect class, and one narrowed to nothing passes
# as quietly as a correct one.
#
# Usage: sh .claude/agents/tests/tst_check_agent_bash_costs.sh
set -eu

HERE=$(dirname "$0")
CHECK="$HERE/check_agent_bash_costs.sh"
WORK="${TMPDIR_OVERRIDE:-./tmp}/tst_check_agent_bash_costs"

rm -rf "$WORK"
mkdir -p "$WORK/agents"

pass=0
fail=0

expect_ok() {
  if sh "$CHECK" "$1" >/dev/null 2>&1; then
    echo "  ok: $2"
    pass=$((pass + 1))
  else
    echo "  FAILED: $2 — expected exit 0, got non-zero" >&2
    fail=$((fail + 1))
  fi
}

expect_fail() {
  if sh "$CHECK" "$1" >/dev/null 2>&1; then
    echo "  FAILED: $2 — expected non-zero, got exit 0" >&2
    fail=$((fail + 1))
  else
    echo "  ok: $2"
    pass=$((pass + 1))
  fi
}

# A minimal role file carrying everything the gate wants.
write_good_role() {
  cat > "$1" <<'ROLE'
---
name: example-role
description: A role file for the test corpus.
---

## What Bash costs here

Read BASH-COSTS.md before your first shell command.
The short version: one plain command per call. Scratch goes in ./tmp/ inside
the worktree. If a task cannot be done within those shapes, stop and report it.
ROLE
}

echo "Building the corpus..."
: > "$WORK/agents/BASH-COSTS.md"
write_good_role "$WORK/agents/example-role.md"

echo "Direction 1 — a correct corpus passes:"
expect_ok "$WORK/agents" "a role file with the pointer and the summary"

echo "Direction 2 — each defect fails:"

# The defect this gate exists for: a new role file added with no pointer.
write_good_role "$WORK/agents/second-role.md"
cat > "$WORK/agents/second-role.md" <<'ROLE'
---
name: second-role
description: An eighth role file added later, with no Bash-costs section.
---

Do the work and report back.
ROLE
expect_fail "$WORK/agents" "a role file with no pointer at all"
rm "$WORK/agents/second-role.md"

# A pointer with no summary: the agent that never follows the link learns
# nothing. This is the subtle one — the link is present and looks done.
cat > "$WORK/agents/third-role.md" <<'ROLE'
---
name: third-role
description: Points at the list but carries none of it.
---

See BASH-COSTS.md for what Bash costs.
ROLE
expect_fail "$WORK/agents" "a pointer with none of the summary"
rm "$WORK/agents/third-role.md"

# Partial summary: has the pointer and one phrase, missing the others.
cat > "$WORK/agents/fourth-role.md" <<'ROLE'
---
name: fourth-role
description: Half a summary.
---

Read BASH-COSTS.md. The short version: one plain command per call.
ROLE
expect_fail "$WORK/agents" "a summary missing ./tmp/ and the fallback"
rm "$WORK/agents/fourth-role.md"

# The canonical file itself going missing.
mv "$WORK/agents/BASH-COSTS.md" "$WORK/agents/.BASH-COSTS.md.hidden"
expect_fail "$WORK/agents" "the canonical list missing"
mv "$WORK/agents/.BASH-COSTS.md.hidden" "$WORK/agents/BASH-COSTS.md"

echo "Direction 3 — the gate must not pass by measuring nothing:"

# Breaking the corpus builder must make the gate FAIL, not report clean.
# Without this case, a frontmatter-matching bug that skipped every file would
# look identical to a clean run.
mkdir -p "$WORK/empty"
: > "$WORK/empty/BASH-COSTS.md"
expect_fail "$WORK/empty" "a directory with no role files at all"

# Prose files without `name:` frontmatter are exempt by that test, not by a
# hardcoded name list — so a README with no pointer must not fail the gate.
cat > "$WORK/agents/README.md" <<'ROLE'
# Prose, not a role file

No frontmatter, so no pointer is required of it.
ROLE
expect_ok "$WORK/agents" "a prose file with no frontmatter is exempt"

rm -rf "$WORK"

echo ""
echo "$pass passed, $fail failed."
[ "$fail" -eq 0 ] || exit 1
