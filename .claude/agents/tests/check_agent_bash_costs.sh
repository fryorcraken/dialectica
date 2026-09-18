#!/bin/sh
# Every role file in .claude/agents/ must point at BASH-COSTS.md.
#
# The property: an agent that reads ONLY its own role file must come away
# knowing the forbidden Bash shapes and their replacements. Before this gate,
# five of seven role files carried no such rule, and four dispatched agents cost
# the user approval prompts in a single session.
#
# This catches the eighth role file, added later with no pointer. That is the
# whole point: a hand-maintained list of which files have been done goes stale
# silently, so the gate enumerates the directory instead.
#
# Usage: sh .claude/agents/check_agent_bash_costs.sh [<agents-dir>]
set -eu

DIR="${1:-.claude/agents}"

# Files in the directory that are NOT role files. A role file is one with YAML
# frontmatter declaring `name:` — that is what makes it dispatchable, so the
# set is derived rather than listed.
CANON="BASH-COSTS.md"

if [ ! -f "$DIR/$CANON" ]; then
  echo "FAIL: canonical list $DIR/$CANON is missing" >&2
  exit 1
fi

failed=0
checked=0

for f in "$DIR"/*.md; do
  base=$(basename "$f")
  [ "$base" = "$CANON" ] && continue

  # Role files declare `name:` in frontmatter on the first few lines.
  # README.md and RUNNER.md are prose and are exempt by that test, not by name.
  if [ "$(head -n 6 "$f" | grep -c '^name: ')" -eq 0 ]; then
    continue
  fi

  checked=$((checked + 1))

  if [ "$(grep -c "$CANON" "$f")" -eq 0 ]; then
    echo "FAIL: $base does not point at $CANON" >&2
    failed=$((failed + 1))
    continue
  fi

  # The pointer is not enough on its own: the file must also carry the shapes,
  # so an agent that reads only this file and never follows the link still
  # knows what not to do. Check a representative few rather than all of them —
  # pinning every row would make the gate fail on a reworded list.
  missing=""
  for token in 'one plain command per call' 'stop and report it' './tmp/'; do
    if [ "$(grep -c -- "$token" "$f")" -eq 0 ]; then
      missing="$missing\n    missing: $token"
    fi
  done

  if [ -n "$missing" ]; then
    # shellcheck disable=SC2059
    printf "FAIL: %s points at %s but omits the summary:$missing\n" \
      "$base" "$CANON" >&2
    failed=$((failed + 1))
  fi
done

if [ "$checked" -eq 0 ]; then
  echo "FAIL: no role files found in $DIR — the gate measured nothing" >&2
  exit 1
fi

if [ "$failed" -ne 0 ]; then
  echo "" >&2
  echo "$failed of $checked role file(s) failed." >&2
  echo "Add the 'What Bash costs here' section; copy it from any passing role file." >&2
  exit 1
fi

echo "OK: all $checked role file(s) point at $CANON and carry the summary."
