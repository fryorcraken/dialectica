# Drive a successful join end to end, with a core-level seeder

## Why

#134 stays in 0.0.1 until the end-to-end suite covers a successful join, the
feed and the thread, each run against an `lgs`-installed Basecamp (owner's
comment on #134, 2026-09-25). #164 delivered the join refusal, and #181 the key
and Stoa creation, the feed, the thread and the moderation screen. What remains
is a successful join. It is the only one of the five that needs something from
outside the profile under test: a Stoa reference that verifies and names a Stoa
the profile does not hold.

The archived `e2e-created-stoa-flow` proposal settled what that reference must
be and where it comes from, and left one question to this piece: how a
verifiable reference reaches a sitometres specification, which is a static
file.

## What Changes

### One specification: a successful join, from paste to the joined Stoa's feed

A sitometres specification, run in `ui-tests.yml` the way the existing five run,
against a fresh profile that `lgs basecamp install` populated. It must observe:

1. **The profile holds no Stoa**, and the list read succeeded.
2. **Pasting the seeded reference previews it and joins nothing.** The preview's
   lookup answers a fallback, because this profile holds no ops for the Stoa, so
   the seeded founding title is rendered as the founding title.
3. **Acting on the join is reported as joined**, and nothing is rendered as a
   failure of the join.
4. **The way back renders the list, and the joined Stoa is among its rows**
   without a restart, with a share offered for its row.
5. **Opening that row renders its feed**, and the feed read succeeded and holds
   nothing, as opposed to failing. This is what shows that the record the join
   was given is the one the view kept and hands on.

The standards #181 applied carry over unchanged. The spec must be **seen failing
in CI for the reason it exists** before it is believed: one deliberate break,
pushed alone, its red run read, then reverted, with both runs recorded. Every
step asserting an absence also asserts a presence in the same snapshot. Steps
before the spec's own screen only get there and assert nothing (`e2e-created-
stoa-flow` design.md D2). Whether this is a new file or steps added to
`join.yaml` is for design.md to decide; `e2e-created-stoa-flow` design.md D1
gives the cost on each side.

### The seeder: a verifiable reference, supplied at core level

**Already settled, and not reopened here.** The seeder is a core-level fixture
that supplies a Stoa reference in the encoding `stoa-navigation-view` fixes
(`{"stoa": …, "genesis": …}`). It is not a second Basecamp profile. The
reasons are in the archived `e2e-created-stoa-flow` proposal ("The second
piece"). In short: a join consults nothing but the two inputs, a second peer
could not deliver content anyway (#176), and sitometres drives one app. The
reference must have these properties:

- its record decodes under the current genesis encoding;
- its address is that record's hash;
- it names a Stoa the profile under test does not hold;
- its founding title is recognisable and distinct from anything else the run
  creates.

**The open question, for design.md: committed, or generated per run.** Both are
live. The constraints each must meet:

- **A committed reference** is a literal in the spec, or in a file beside it.
  Its creator key cannot be arbitrary bytes, because `stoa-genesis` refuses a
  creator key that is not a valid public key, or one that can never verify a
  signature. Its record is fixed at the genesis encoding it was written under.
  A change to that encoding would turn the spec red in the expensive workflow,
  and that red is signal as well as cost: `stoa-navigation-view` makes the
  reference encoding a compatibility surface ("The reference encoding is a
  compatibility surface and is fixed here"), so a committed reference going
  stale is a reference in the wild going stale. What catches the drift first,
  and in which workflow, is part of the decision.
- **A generated reference** is produced by the core in the run, which means
  building core code in `ui-tests.yml` and getting the output into a spec
  sitometres reads as a file. The design must say how, at the pinned sitometres
  version, and whether that version can take a value from outside the spec at
  all. Three things read the spec file as committed: the `ui-specs` validator
  (`validate-ui-specs.mjs`), the matrix count check and the adjudicator's step
  count (`e2e-ui-suite` design.md D1 and D8). Whatever the run executes must be
  what those three checked. A generated reference also comes from the same core
  that verifies it, so an encoding change moves both halves together. The spec
  stays green through exactly the change that would strand every shared
  reference.
- **Candidate sources** named by the archived proposal: the `seed_store`
  example, which prints an address and its record but mints a fresh keystore on
  every run, and a creation reply from a throwaway core. Neither may write into
  the profile under test: the reference is the seeder's whole output.

Owner constraints that bind the design: YAML through `yq` and JSON through `jq`,
never Python; `lgs` in CI wherever an `lgs` verb exists; no `npm install` or
`pip install`, locally or as a new CI dependency the suite does not already pin;
and no local sitometres run.

### This piece closes #134

Against #134's list: the harness salvaged from #120 and made to pass, the CI
cost and cadence decision, and the #128 re-check were #164's (`e2e-ui-suite`
design.md D7 is the cadence decision). The feed, the thread, the moderation
screen, and key then Stoa creation were #181's. The successful join is this
piece. Nothing on #134's list, or in the owner's comment on it, is left undone.
Receipt of content from another peer is #102's item 7 and waits on #176; it is
not on #134's list. So this piece's PR carries **`Closes #134`**.

**#134 is already closed, by mistake.** #181's body said, in prose, that the next
piece's "PR closes #134". GitHub read that as a closing keyword and closed the
issue when #181 merged (the issue's events show `closed` at the merge commit
`2bda577`). Reopening it is the owner's decision. This PR's `Closes #134` is
correct either way. A PR body that mentions another PR's closing keyword should
quote it, for example "`Closes #N`", so that GitHub does not act on it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `stoa-navigation-view`: "A join is reported from the core's reply, never
  assumed" gains its positive half. A successful join of a Stoa the peer is not
  in MUST be reported as joined. The requirement says a join is reported
  **only** on a successful reply. It has scenarios for a failed reply, for a
  Stoa already held and for a join after a refused lookup, and none for the
  ordinary case this spec walks: a fallback lookup, then a join of a Stoa not
  held.
- `view-navigation`: "Every state a user can enter has a specified way out"
  gains the joined Stoa reaching the list without a restart. It already
  contracts that for a created Stoa. For a joined one, "The return is still
  available after a join succeeds" requires that the list renders, and nothing
  requires the joined Stoa to be on it. "A Stoa joined in this session can be
  shared" in `stoa-navigation-view` assumes the row exists without requiring
  it.

The view already does both. `DJoinScreen` reports `joined` on a successful
reply, and `Main.qml`'s `onJoined` records the pasted record and re-reads the
listing. So neither delta changes code. The `tester` pins each scenario that is
not already pinned.

Every other observation above is already a requirement:

- the empty listing read as a success: `stoa-navigation-view`, "Holding no Stoas
  and failing to read membership are different screens";
- the preview before any join, the fallback title rendered as the founding
  title, and the lookup made without the user acting: `stoa-navigation-view`;
- a share offered for a joined Stoa's row: `stoa-navigation-view`, "A Stoa joined
  in this session can be shared";
- the way back after a join, and the feed opened from a row with its record:
  `view-navigation`;
- the feed read succeeding and holding nothing, as opposed to failing:
  `feed-view`.

**The seeder adds no requirement.** It is an input to the test harness, not
behaviour of the forum. The archived `e2e-ui-suite` change's `.openspec.yaml`
gives the reason the harness's obligations stay out of every capability, and it
applies here. Whichever reference design.md chooses, its properties above are
checked by the join succeeding, and by whatever check design.md adds to catch
drift early.

## Impact

- **New or modified:** a sitometres specification under
  `dialectica-ui/tests/ui/`; `.github/workflows/ui-tests.yml`, whose matrix and
  count check follow a new file if there is one; the seeder, wherever design.md
  puts it; and `join.yaml`'s "does NOT cover" paragraph, which names the
  successful join as missing and will then be false.
- **View:** read-only root handles on `Main.qml` or `objectName`s only where the
  spec needs one that does not exist, each pinned in `tst_e2e_handles.qml` (the
  `e2e-ui-suite` change's design.md D6). No behaviour change.
- **Spec:** the two deltas above.
- **CI cost:** one more `ui-tests.yml` job if the spec is its own file, about 3
  minutes warm (`e2e-ui-suite` design.md D7). A generated reference adds its own
  build to that job.
- **Not affected:** the wire contract, and every other requirement in
  `openspec/specs/`. Core code changes only if design.md puts the seeder there.
