# Design review — `ui-thread-view`

Checked: did the code take the decisions design.md records, and were the
decisions worth recording recorded. No blocking findings — the entries below
are the checks performed and what they turned up, kept because the brief asked
for this stage's reasoning to be inspectable, not because anything is broken.

## D13 — the mutation-proof generalisation, and the rejected sentinel

**Verified against the code**, not just the prose. `parentOf`
(`DThreadScreen.qml:148-156`) returns `{kind, id}` over exactly the three kinds
D13 states (`parentRoot`, `parentNamed`, `parentUnusable`), and `resolveDepth`
(`:188-231`) branches on kind rather than on a shared `""` value. The old
single-value collapse D13 describes as the bug is gone; there is no line left
where the root case and the malformed-parent case could be read as each other.

D13's generalisation — "a mutation test proves the guard it mutates is reached
and load-bearing on the paths the fixtures take; it says nothing about a path
that returns before the guard" — is stated correctly and narrowly. It does not
overclaim ("mutation testing is unreliable") or underclaim ("this specific
bug was subtle"); it names the exact scope of what mutating `resolveDepth`'s
`-1` branches can and cannot see, which is the generalisation a future reader
would need before trusting a green mutation result on a guard with an earlier
return path. The entry also records, correctly, that `wire.rs:7041` and the
`read_thread` builder at `:2027` never send a malformed `parent` today, and
explicitly instructs against deleting the guard as dead code because "core is
honest today" is a different claim from "the view cannot be handed this
shape" — so the reachability disclosure does not become an invitation to prune
the defence later.

**The rejected sentinel argument holds up.** A distinguishable string (e.g.
`" unusable"`) would still let a caller written as `parentOf(x) === ""` compile
and silently take the root branch — the same failure shape one value later,
since nothing about a second string value stops a caller from comparing
against the wrong one. The `{kind, id}` shape forces a caller to name which
kind it means, which a string constant does not. The argument is sound and is
recorded as a rejected alternative rather than only as "here is what we built"
— it names what was rejected and why, which is the part that would otherwise
have to be re-derived by the next person who reaches for a sentinel.

**`itemId`'s asymmetry is checked and correct.** Both its call sites
(`DThreadScreen.qml:165`, `:205`) treat `itemId(x) === ""` as a refusal
(skip keying / skip seeding `visited`), never as an affirmative fact the way
`resolveDepth` read `parentOf(x) === ""` as "this is the root" pre-fix. There
is genuinely no second meaning for `itemId`'s `""` to collide with, so leaving
it unreshaped is the right call, not an inconsistency.

**The null/undefined-item reclassification is disclosed at the right width.**
Checked `tst_thread_nesting.qml`'s sixteen tests: none constructs a
null/undefined *item* (as opposed to a `parent` field) and feeds it through
`resolveDepth` or `parentOf` directly. D13's "deliberately left alone... flagged
as untested" framing for the fixer's honest-gap discipline is accurate — this
is a real, disclosed gap, not an understated one.

## D1's edited paragraph

Read the diff (`2b8d083`) against the pre-fix text. The original closing
paragraph read `distinguished by parent === undefined (the root) versus parent
present but unmatched`, which was already inaccurate to what the pre-fix
`parentOf` did (collapse *any* non-string parent, including `undefined`, to the
same `""` as an absent field) — the tester's finding demonstrates this exactly.
The correction — "this paragraph described the intent, and the first
implementation did not honour it" — is not retrospective tidying: it is a
dated, verifiable statement pointing at D13 for the mechanism, and it matches
what the tester's finding and the `0444726`/`2b8d083` diffs show. Editing a
prior decision is scrutinised hardest here per the brief, and it survives that
scrutiny.

## D9's ownership table and the reconciliation record

Cross-checked the table in *What the merge took from each side* against the
code-reviewer's findings file. The `openThread()` `rootOp === ""` guard —
flagged by `code-reviewer` as a cross-piece edit missing from D9's table — now
has its own row, attributed to this piece and into navigation's owned
function, plus D11 recording what it defends, that it is currently
unreachable, and what breaks without it (nothing, stated plainly rather than
invented). D9's opening paragraph, which the code-reviewer separately flagged
as describing this piece's proposed (and superseded) mechanism as though it
shipped, is now marked as superseded with the actual shipped behaviour stated
in the same breath, and kept rather than deleted — for the reason it gives (a
later change reaching for the same proposal should find it was tried and what
displaced it). Spot-checked `FeedScreen.qml:418-424`'s `threadTarget()` against
D10's "neither piece had it right, correct field is `thread`" claim: it reads
`rowData.thread`, matching.

No further crossing edits found beyond what the code-reviewer already
surfaced and what D9/D11 now record.

## Decided-but-unrecorded candidates

- `while (cursor !== "") → while (true)`: recorded, in D13's closing
  paragraph, with the correct reason (the string test was never the
  terminating condition; the visited set is).
- The null-item reclassification: recorded, in D13, as above.
- `Core` vs. a JS module for the extracted probe helpers (D12): recorded, with
  the rejection reasoned ("`Core` is already imported by both screens and is
  already the boundary the reply crosses, so a second mechanism would add a
  file without adding an invariant") rather than merely asserted.

No further undocumented decisions found in `DThreadScreen.qml`, `Main.qml`'s
crossing edit, or `FeedScreen.qml`'s `threadTarget`/`voteTarget` split.

## PLAN.md — reasoning migration and honest-gap discipline

Read `docs/PLAN.md` from `origin/main`: it still carries, in full prose, both
the moderation-shape-divergence paragraph and the bidi-obligation-is-wider
paragraph that D7 and D8 claim to have migrated. That is expected and correct
— `origin/main` predates this change; the branch's own `docs/PLAN.md` has
already pruned both to one-line pointers at `design.md` D7/D8 (verified via
`git diff origin/main -- docs/PLAN.md`), which is the shedding CLAUDE.md and
`.claude/agents/README.md` describe. No duplication survives on the branch.

D7 and D8's own honest-gap language was checked against the sibling-piece
failure mode named in the brief (a gap entry claiming a change "would very
likely stay green" with nothing witnessing the mechanism). Neither entry makes
that shape of claim: D7 states the divergence is "a live gap, not a settled
one" and names the two citations (`wire.rs:1575`, `wire.rs:1849`) that make it
measured rather than asserted; D8 states plainly "the view applies no second
transformation" and that a second sanitiser would be undetectable drift, which
is a claim about what the code does, not a prediction about what will keep
holding under future change. D11's "what breaks without it: nothing in the
suite today" is the same honest form. No entry found reading as overclaiming
its own coverage.

## What this review did not find

No instance of code contradicting a recorded decision, no decision applied at
some call sites and missed at others, and no unrecorded choice a reader would
plausibly have made differently. The Decisions section is in good shape:
every entry names what was chosen, the constraint, the alternatives and why
each was rejected, and — where the decision is a guard — the mutation evidence
naming which test goes red and why. `design.md:377-467` (D13) is the strongest
entry in the file by that standard, and it is also the newest.
