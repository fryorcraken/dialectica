# Design review — `e2e-created-stoa-flow`

Scope: `git diff fe093be...HEAD` against `design.md` D1–D9, `proposal.md`, and
`openspec/specs/`. GitHub issues #134 and #152, and PR #181, read fresh.

## What I checked

**D1 (four files) / D2 (prefix asserts nothing).** `create.yaml`, `feed.yaml`,
`thread.yaml`, `moderation.yaml` each carry the eight-step key-and-Stoa prefix
and assert nothing about a screen they only pass through — confirmed against
all four files. `tasks.md` sections 4–7 show each break run reddening exactly
its own job and no other, which is D2's claim measured, not assumed.

**D3 (root handles).** All seven handles on `Main.qml` (`listedStoas`,
`createdStoa`, `feedReadState`, `feedRowCount`, `feedCanPost`,
`threadReadState`, `threadItemCount`) match the table, and
`FeedScreen.visibleRows` is the single guarded read D3 describes — the
Repeater, the empty-state label and `feedRowCount` all read it, and no site
still restates `readState === "ok"` independently.

**D4 (naming).** `openStoaButton`, `<kind>DraftField`/`<kind>SubmitButton`,
`readThreadArea`/`moderateArea` all land exactly where D4 says, on the
`MouseArea`s rather than the labels, each with the reasoning repeated at the
binding site.

**D5 (assertion shape).** Every `expect:`/`wait_for:` split, every
absence-beside-a-presence, and the shape-only address assertion in
`create.yaml` match D5.

**D6 (proof breaks).** No break remains in the net diff — confirmed by reading
the current `Main.qml`/`FeedScreen.qml`/`DStoaListScreen.qml`/
`DModerationScreen.qml`, matching 8.1's own `git diff --stat` claim.

**D7 (pruned prose).** Confirmed in `Main.qml`, `FeedScreen.qml`, `join.yaml`.

**D8, and #152.** The code takes exactly the decision D8 records:
`Main.qml`'s `stoaAddress` binding on the feed reads `feed.stoaGenesis ===
root.chosen.genesis` before releasing the address, and the thread's `threadId`
binding reads both `thread.stoaAddress` and `thread.stoaGenesis` before
releasing the id — the self-referential guard, applied to both screens, not
partially. `tst_navigation.qml`'s new tests check every read sent to the fake
rather than the last, on all three routes onto the feed (list, back from
thread, back from moderation), matching the mutation evidence in design.md and
`tasks.md` 3.2/3.3.

The PR does not overclaim against #152: the body's own words are "This may not
be all of #152 ... so this PR does not close it", there is no `Closes #152` in
the PR body, and `Part of #134` (not `Closes #134`) is the PR's actual title
line — consistent with design.md's Context and the proposal's "Why two pieces
and not one".

**D9 (harness wait).** `feed.yaml` and `thread.yaml` both carry the
`wait_for:` on the submit control immediately before each `click:` (post and
reply); `create.yaml` has no such wait, correctly, since it has no composer
submit. Step counts match exactly: `create.yaml` 12, `feed.yaml` 18,
`thread.yaml` 23, `moderation.yaml` 15 — all four counted directly against the
committed files and matching `tasks.md`'s predicted-and-observed numbers.

**Owner's directions.** `ui-tests.yml`'s matrix diff is the one-line change
design.md's Context says it is; the job still runs `lgs basecamp setup` /
`install` for every Basecamp build step, with no direct Basecamp invocation
outside `lgs`. `git grep -n "python3"` over the touched workflow, spec and
change-folder paths returns nothing; the schema and log-parsing steps use `yq`
and `jq` as the file's own comments describe. No milestone or 0.0.1 claim in
this piece's files contradicts the issue's milestone.

**The seeder / two-piece split.** The proposal's departure from #134's "needs a
seeding peer" wording is argued, not asserted: three checkable reasons (a join
consults nothing but its two inputs, a second peer could not supply content
because ops never leave their author per #176, and sitometres drives one app),
each citing a live requirement or a numbered issue. It explicitly names itself
as a departure the owner can overrule, which is the shape the review brief
asks for rather than a silent scope change.

## No blocking findings

I did not find code contradicting a recorded decision, nor a decision made in
the code that Decisions leaves unrecorded. The Decisions entries I checked each
have what the review's own rubric asks for: what was chosen, the constraint,
the rejected alternatives with what ruled them out, the cost, and — for every
guard (D3, D8) — mutation evidence with the exact test and the exact break.
D9 is the one guard-shaped decision with no mutation evidence, and design.md
says why directly rather than omitting it: the miss is a timing race that
"cannot be forced red on demand," and the evidence offered instead (the actual
failing run's log plus a throwaway component probe measuring the button's
pre/post-layout position) is the honest substitute for a break that cannot be
authored to order.

The two-piece split and seeder reasoning live in `proposal.md` rather than
`design.md`'s Decisions section. I read that as the right placement rather
than a gap: it is a scope decision (what this piece covers and why the fifth
item is deferred), which is `proposal.md`'s job throughout this repo's
convention, and `design.md`'s Context section cross-references it rather than
silently duplicating or omitting it.

- [x] **Gate record** — design review of `e2e-created-stoa-flow`
      (`fe093be...HEAD`) against design.md D1–D9, proposal.md, `openspec/specs/`,
      issues #134/#152 and PR #181. Clean: no contradicted decisions, no
      undocumented decisions found, D9's missing mutation evidence is itself
      accounted for in design.md rather than silently absent.
