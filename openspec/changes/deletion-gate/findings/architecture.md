# Architecture findings — `deletion-gate`

Reviewed at `8ba7f23` in `.claude/worktrees/review-deletion-gate-read`. The suite
runs green as committed (**34 passed, 0 failed**). The sibling adapter gate that
design.md §8b compares against is on `main` and I read it directly
(`.github/workflows/ci.yml:414-482`) rather than accepting either author's account
of it.

The four structural questions the brief poses are each answered in prose at the
end; only three produced boxes.

## Defects

- [ ] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:4` — the
      script's documented signature is three arguments, but it has a fourth,
      undocumented, mandatory input: the caller's working directory
      **Scenario:** the usage line is
      `check-claimed-deletions.sh <base-ref> <head-ref> <pr-body-file>`. Every
      `git` call in the file runs against the ambient cwd's repository — there is
      no `cd`, no `-C`, and no `--git-dir` anywhere (`grep -n "cd \|dirname\|pwd"`
      over the script returns nothing). So which repository is measured is decided
      by the caller's cwd and is invisible in the call.
      This is load-bearing rather than theoretical: the test harness sets it by
      `cd`-ing into each throwaway clone before every single `run_gate`
      (`test-check-claimed-deletions.sh:50, 82, 124, 158, 201...`), and the
      workflow relies on `actions/checkout` having left the runner in the
      checkout. Neither call site passes it, so neither is checkable against the
      signature. Invoking with correct arguments from the wrong directory gives
      `'origin/feature' does not resolve to a commit in this checkout` —
      **measured**, and a misleading diagnosis, since the ref is fine and the
      directory is not.
      The repo's own rule is *"do not let a function quietly acquire a second
      caller with different needs — pass what it needs; do not have it reach for
      ambient state"*, and this script now has exactly two callers reaching for
      ambient state. The cheapest fix is a fourth argument or a `git -C`; the
      cheapest *honest* fix is one line in the usage comment saying the cwd must be
      the repository under test.
      **Severity:** medium — the guards make every wrong-cwd case loud rather than
      silent (verified: it exits 1, it does not pass having measured nothing), so
      this is a contract-clarity defect and not a safety one. It matters because
      design.md §1's whole argument for extracting a file is that the logic becomes
      *invokable outside GitHub Actions*, and an invocation contract that omits a
      required input undercuts the reuse the extraction was for.

- [ ] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:121-124`
      and `204-205` — the cleanup list is duplicated across two `trap` statements
      that must be kept in sync by hand, which is the fourth-guard-copy shape
      CLAUDE.md names as the signal to reshape
      **Scenario:** three temp files are created at 121-123 and registered in a
      `trap` at 124. A fourth is created 80 lines later at 204 and registered by
      **re-declaring the whole trap** at 205 with all four names restated:
      `trap 'rm -f "$deleted_list" "$claimed_list" "$unclaimed_list" "$uncommented_body"' EXIT`.
      Adding a fifth temp file means editing the second trap and remembering the
      first is now a stale prefix of it; forgetting leaves a leak that nothing
      tests and that no reader would notice, since the first trap still looks
      complete. The correct-today behaviour was verified — I ran both the success
      path and the guard-2 early-exit path and counted `/tmp/tmp.*` before and
      after each: delta 0 in both cases, so nothing leaks now.
      This is the repo's own "put the complexity in the data structure, not the
      logic" applied to a five-line span: one `mktemp -d` holding all four files,
      or one `trap 'rm -rf "$tmpdir"' EXIT` set once before any of them, makes the
      invariant hold by construction and a new temp file inherit it for free.
      **Severity:** low — no defect today, and the hazard is latent. Filed because
      it is a two-line reshape and because the duplication is what a later editor
      will copy.

- [ ] **`spec-writer`** — `openspec/changes/deletion-gate/proposal.md:32-40` —
      the proposal's one instruction to the reader to verify its central claim
      cannot be followed, because the branch it names no longer exists
      **Scenario:** the Why section presents a fenced command pair against
      `origin/piece/op-transport` and says *"Run those two commands before
      believing anything below; they are the whole argument in six lines."* The
      branch has since merged and been pruned. Both commands now exit 128:
      `fatal: ambiguous argument 'origin/piece/op-transport': unknown revision or
      path not in the working tree` — confirmed absent from `git branch -r` after
      `git fetch origin --prune`.
      The structural point rather than the factual one: this is the *second* figure
      in the same section to rot this way, and the section already documents the
      first — *"That instance can no longer be measured... the historical 7,071
      figure is not reproducible from the repository today."* The document
      diagnosed the failure mode, then pinned a replacement branch tip that decayed
      identically. CLAUDE.md's rule is that recorded present state must be
      **self-invalidating**; a command naming a specific piece branch is the
      opposite, since piece branches are the shortest-lived refs in the repo and
      their deletion is silent. Phrasing it against whatever branch is currently
      open — or against the archived merge commit, which is permanent — survives.
      Naming a third live branch would buy a few weeks.
      **Severity:** medium — the argument itself is sound and independently
      re-derivable (I confirmed it on `origin/piece/publish-envelope`: 5,067
      deletions two-dot against 130 three-dot), so nothing downstream is wrong. But
      the proposal's stated evidentiary basis is currently unrunnable, and this is
      the artifact a later reader consults to decide whether the gate earns its
      keep.

## The comment-stripping comparison — I checked both, and the design is right

The brief asks me to judge §8b's claim rather than take either author's word, so
I read the sibling gate on `main` in full (`ci.yml:414-482`).

**The design's characterisation of the sibling is accurate.** The adapter gate
does strip once, up front: `code = re.sub(r"^\s*//.*$", "", src, flags=re.M)` at
line 454, before any ban is applied, with a comment at 449-453 recording that the
gate previously fired on its own explanatory paragraph — the same scar this piece
carries for fenced examples. The two bans that follow (the `for want in (...)`
membership test and the `re.findall` of banned accessors) both run against that
one cleaned buffer. §8b describes this correctly.

**The structural argument holds and I would reach the same conclusion
independently.** An up-front strip is one place to be right and a new ban inherits
it; a per-arm exclusion must be repeated correctly at every site and its omission
is silent. That is CLAUDE.md's "complexity in the data structure, not the logic"
applied to text, exactly as §8b says. This piece's `awk` filter is the same shape
and the right one.

**Two qualifications, neither of which changes the verdict**, offered because the
brief asked for a judgement rather than a ratification:

*First*, the sibling is not the pure exemplar §8b implies. Twelve lines after the
up-front strip it applies a **per-arm patch** —
`code = code.replace("keystore.stoa_key(&stoa)", "<exempt: publish path>")` at 466
— a named single-call-site exemption layered on top of the clean buffer. It is
well documented and deliberately temporary, but it means the real precedent is
"strip once up front, then bolt one exception onto the buffer", not "strip once
and never touch it again". That is a slightly weaker precedent than "this repo
already has the pattern" suggests, and it is the shape a third author copying §8b
would meet.

*Second*, §8b's own honesty caveat is correctly placed and I want it on the record
as a strength: *"#67 is on a branch not merged here, so that half is the closer's
measurement and not mine."* I verified the #67 gate is indeed absent from `ci.yml`
on this branch (`grep -rn "collides with the host"` returns nothing), so the
author has correctly labelled a second-hand claim as second-hand. The structural
argument is explicitly made to stand without it, and it does.

**Verdict: the comparison is sound, the chosen shape is the better one, and §8b
is the right place to record it.** No box.

## The other three structural questions

**Is a shell script the right shape, given the repo has no other tested CI
script?** Yes, and the reversal recorded in §10 is what makes it so. The argument
for extraction is strong on its own terms: the shallow-clone case — the piece's
most important failure — is unreachable from an inline `run:` block without
deliberately misconfiguring the workflow and pushing it, and design.md §1 rejects
the obvious alternative (inline, with tests driving a *copy* of the logic) on the
grounds that a test against a copy is the defect family this repo already
catalogues. That is the correct rejection.

The convention question is the sharper half, and the piece answers it well. A
tested CI script is a new convention here, and the author originally declined to
set it ("not this piece's to set"), then reversed on the argument that **an
unmeasured property is not a held property** — with receipts: two mutations
survived the whole suite before the tests were wired in. I find the reversal
correct and the receipts real. What makes this a convention the repo can carry
rather than a one-off someone maintains alone is that the cost is bounded and
visible: one `run:` line, no new job, ~1s, no dependencies, POSIX `sh` matching
the conventions design.md §Context names (`ci.yml` and
`dialectica-ui/tests/run-qml-tests.sh`), and the tests build their own throwaway
repos touching nothing in the checkout. The next author adding a CI script has a
worked example and a place to put it. I would call this a convention well set.

The one thing that would make it carry further is a `.github/scripts/README.md`
saying "scripts here are tested by `tests/`, and the tests run in `Lint`" — absent
today, so the convention is discoverable only by noticing the second step. Not
worth a box on a directory with one inhabitant; worth doing when a second arrives.

**Is the `Deletes:` boundary drawn where the design says?** Yes, and this is the
piece's best-governed decision. The mechanism is unreviewed attacker-controlled
text asserting something about a diff, and the author never claims otherwise. The
limit is stated in four places that agree with each other and with the code: the
script header (*"This converts a silent deletion into a stated one, never into a
reviewed one — closer.md step 2 is still the reader"*), `proposal.md` ("What it
does not catch"), design.md Non-Goals, and design.md Risks. I checked the code
matches: nothing validates that a claim is *correct*, and the failure text says
which paths need claiming without asserting that claiming them is right.

Drawing it here is the right call. The alternative — a gate that judges whether a
deletion *should* happen — is not automatable, and a gate that pretended to would
be the "trusted past its limits" failure the header explicitly guards against. The
honest framing also keeps `closer.md` step 2 alive rather than letting a green
check retire it, which is the real risk with a gate in this position.

**Does the scope match what it claims — seven of eight?** Yes, stated plainly and
without inflation. `proposal.md` names the exception (the duplicate `storage_dir`
on `piece/ui-stoa-list`), explains why a deletion gate is structurally blind to it
(it was an *addition* from a textual merge), and names the only job that caught it
(Build LGX, because both copies are `cfg(logos_scaffold)` and so compiled by
neither `cargo test`, clippy nor `cargo fmt`). The script header repeats the limit
in its own words. I confirmed the duplicate is gone from the tree today —
`dialectica/rust-lib/src/lib.rs` has one `fn storage_dir` at line 412.

Nothing in the piece's framing implies more protection than it gives. The
proposal's *"Naming this is the point rather than an apology for it"* is the right
instinct, and the third limit — that the gate never inspects `main`, so a green
`main` run is not evidence it ran — is stated in the proposal, the script header
and the `ci.yml` comment, which is the limit most likely to be forgotten.

**Where does it belong — `Lint`, or a new job?** `Lint` is right, and the name
objection does not survive contact with the job. I listed every step in the
workflow: `Lint` already holds eleven, and **none** of the pre-existing ones is a
lint in the compiler sense. They are structural repo-hygiene gates — "metadata.json
is valid and self-consistent", "the UI icon is a 256x256 PNG", "scaffold.toml kept
its comments", "the adapter derives the creator and the poster in one place". The
deletion gate is the same kind of thing: a cheap textual assertion about the
repository that needs no build. It fits the job's actual contents rather than
overloading them; if anything the job's *name* was already the loose part, and
that predates this piece.

The stated reason for choosing it — `Lint` is already a required status check, so
a step there is enforced without a branch-protection change, where a new job would
need one plus a queue slot for a single `git diff` — is sound and verifiable, and
the proposal cites the command
(`gh api repos/<owner>/<repo>/branches/main/protection`) rather than asserting the
protection state. Correct by this repo's own rules. No box.

## What else was clean

**The `fetch-depth: 0` change is correctly scoped.** It is applied to the `lint`
checkout only, not to all five jobs, which is the minimum that makes the
merge-base range work. §3's rejection of a finite depth is right for the stated
reason: a number chosen to be "usually enough" makes the gate fire on branch age,
which is a false positive, which is what gets a gate disabled.

**The `if:` belongs on the step, not inside the script, and §6 gives the right
reason** — a skipped step is visible as skipped in the GitHub UI, where an early
`exit 0` inside the script would render as a check that ran and passed. That is
the correct place for the decision, and it keeps the script a pure function of its
arguments (modulo the cwd defect filed above) rather than of the CI event.

**The gate-on-the-gate runs on every event while the gate itself runs only on
`pull_request`**, and the asymmetry is deliberate and correctly reasoned: the
script's behaviour is a property of the script, not of the event, so a push to
`main` that breaks it should go red immediately rather than on someone's next PR.

**Both scripts are committed executable** (`100755` in the index for both), so the
bare-path invocations in `ci.yml` will work. This is the kind of thing that fails
only in CI, and it is right.

**No new dependencies.** POSIX `sh`, `git`, `sed`, `awk`, `grep`, `mktemp` — all
already required by the workflow. Nothing to assess for licence compatibility.

**No CI gate was left measuring a directory that moved.** I checked the
source-layout-derived gates the brief warns about: the `#[test]`-counting gate
(`ci.yml:887-909`) scopes to the Rust tree via `metadata.json`'s crate name, so the
new shell tests neither inflate nor deflate its `declared` count, and the panic-guard
gate names its own file. Adding two `.github/scripts/` files disturbs neither.

## What I could not check

Whether `$RUNNER_TEMP` is writable at that point in the `Lint` job and whether
`git fetch --no-tags origin "$BASE_SHA"` succeeds against GitHub's server — both
need a live Actions run, and both were already recorded as unchecked by the
correctness reviewer. They are architecture-relevant only insofar as the step
would fail loudly under `set -eu` rather than pass; I confirmed the shape of that
locally but not on a runner.

Whether the tested-CI-script convention actually gets followed by the next author.
That is unknowable today and is the only real risk to the "convention, not one-off"
judgement above.
