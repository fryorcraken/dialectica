# Design — delete `docs/UI-BRIEF.md`

## 1. What this change is, and the shape of the risk

Deleting a file is trivial. What is not trivial is that **twenty-one live
citations pointed at it**, and six of those were sentences that *delegated a
claim outward* rather than merely naming a source. A citation that names a
source can be dropped; a delegation cannot, because dropping it leaves the
citing document asserting something it no longer supports.

The failure mode this change exists to avoid is a repository that still *reads*
as though the brief exists — a PLAN.md paragraph claiming a policy layer with no
document behind it, a QML comment telling the next author to consult a file that
is gone. That is worse than a dead link, because a dead link is visibly dead and
a dangling claim is not.

So the work is a survey, then a judgement per site, and §2 records the
judgements one by one.

### The survey was wider than the dispatch's

The dispatch listed the sites a prior survey had found. Re-running the grep
found **seven more**, and two of them were load-bearing:

| Site | Why the first survey missed it |
|---|---|
| `ScreenFrame.qml:8` and `:47` | the dispatch's list covered `FeedScreen`, `Core` and the feed tests but not the shell component |
| `FeedScreen.qml:397, :400, :654` | the dispatch named `:9` only |
| `tst_feed_copy.qml:285` | the dispatch named `:12` only |
| `tst_screen_frame_geometry.qml:186` | inside an assertion *message* |

`ScreenFrame.qml:8` is the one that mattered most — see §3.

**The grep to re-run** is `grep -rn "UI-BRIEF\|UI brief\|ui-brief\|UI_BRIEF"`
over `docs/`, `dialectica/`, `dialectica-ui/`, `CLAUDE.md`, `.claude/` and
`openspec/specs/`. It returns nothing on this branch.

`openspec/changes/archive/` is deliberately excluded: 23 citations live there
and all 23 stay. An archive records what was decided *at the time*, and a reader
who finds a reference to a since-deleted document learns something true.

## 2. Decisions

### 2.1 No replacement document, and no promotion of obligations into specs

**Taken: the brief's designer-facing material goes with the brief.**

The owner settled this. What is recorded here is the *check* that made it safe
rather than merely instructed: `grep -rn "UI-BRIEF\|UI brief"` over
`openspec/specs/` returns **nothing**. No live requirement cites the brief,
quotes it, or delegates to it — so no requirement is weakened by its deletion,
and there is nothing to promote that is not already promoted.

**Rejected: writing the obligations into a spec as part of this change.** That
would be adding contract under cover of a deletion, and the obligations a
requirement actually depends on are already *in* requirements — obligation 5 as
`module-wire-contract`'s error shape, obligation 6 as
`view-identity-onboarding`'s "The screen states that a name is not unique and
not an identifier", obligation 9 as `op-transport`'s "A successful publish is a
statement about the local log and nothing more".

### 2.2 Where PLAN.md delegated, the claim is stated rather than dropped

Four of the six PLAN.md sites were delegations. **All four are now stated in
PLAN.md**; none was dropped. The reasoning is the same in each case: PLAN.md
declined to state the claim *because the brief held it*, so the brief's removal
is precisely the condition under which PLAN.md should state it.

| Site | Was | Now |
|---|---|---|
| §5.5 (`:2484`) | "the rendering obligations in `docs/UI-BRIEF.md` are policy" | **stated** — the policy layer is real and is now named by its three concrete instances, each of which lives in a spec |
| §5.5 (`:2446`) | quoted the brief's "never auto-join" rule | **stated** — the quotation becomes PLAN.md's own sentence |
| §5.5 (`:2451`) | "`docs/UI-BRIEF.md` describes only what does" | **restated** — the two interface notes are recorded here because no surface exists for them to be contracted against, which is the real reason |
| §12 (`:4414`) | "the brief carries the half of the rendering obligation that is true today" | **stated** — the prohibition (not sent, not delivered, no in-flight state) is now PLAN.md's own text |

The last is the most consequential. PLAN.md had *deliberately* declined to state
the prohibition, on the grounds that the brief carried it. With the brief gone
there is no other document that states it in prose, so PLAN.md states it — and
keeps the reasoning about *why* it did not wait on the three owed things, which
is the durable part.

**One claim was re-grounded rather than stated or dropped**: the `agora`
wordlist entry (`:1368`). See §2.3.

### 2.3 `agora` stays in the wordlist, on a reason that survives

The entry existed because the brief used "Join *Agora*?" as its worked example
of a forgeable Stoa title. That example is gone, so the *stated* reason died
with it — but the entry's underlying reason did not.

**Kept, re-grounded on the test fixtures.** `Agora` is this project's standing
worked example of a Stoa *title* throughout the Rust suite, verified rather than
assumed: `grep -rn '"Agora"' dialectica/rust-lib/` returns matches across
`membership.rs`, `feed.rs`, `moderation.rs`, `wire.rs`, `end_to_end.rs` and
`log/fixtures.rs`. A generated *user* name reading as `agora` is therefore a
user who reads like a Stoa, which is the same failure `stoa` itself is excluded
for, one step removed. The first-to-drop flag stays.

**Rejected: dropping the entry.** The exclusion table is an enumeration of
reasoning "not recoverable from the word it excludes" — its own words. Removing
a row because its citation rotted would discard the reasoning and leave the next
curator to rediscover it.

### 2.4 Comments state their rule instead of citing an ordinal

Every source and test comment naming the brief explained *why* the code is as it
is, so none was deleted. Each now **states the rule it depends on** in one
clause, rather than pointing at an ordinal.

That is not only a repair, it is a correction. The brief restarted its numbering
per section and contained a `2b`, so "obligation 5" was never a locator — a
point `end_to_end.rs` had already made about itself and which is now preserved
as the general lesson (*cite a requirement by its heading, never by an ordinal*)
attached to the surviving `module-wire-contract` requirement.

### 2.5 Two assertion *messages* were reworded; no assertion changed

`tst_screen_frame_geometry.qml:186` and `tst_feed_copy.qml` carried the brief's
name inside failure-message prose. Those strings are read by a human diagnosing
a failure, never compared against anything, so rewording them changes no
condition. **No test's assertion, fixture or expected value was touched.**

Verified by running both suites — see §5.

## 3. The one thing that moved rather than being deleted

**`ScreenFrame.qml` delegated its implementer contract to the brief**, at
`:8`: *"What this gives a screen, and what it asks of one, is stated for the
person writing a screen in `docs/UI-BRIEF.md` under What `ScreenFrame` gives
you."*

That section was addressed to **code authors, not designers** — it is the one
part of the brief that was not designer-facing material. It carries the
`Layout.fillHeight` trap: in a content-sized card a `fillHeight` child measures
**zero**, with no error, no binding loop, and every gate green. A blank region
on a screen.

**Taken: consolidate the contract into `ScreenFrame.qml`'s own header comment.**
Four clauses — the card reports its own height, do not give it an explicit
height, there is no slack to fill, design screens that grow downward.

**Rejected: letting it go with the brief.** It is the only material in the
deleted file whose audience is someone editing the code, and losing it would
reintroduce a defect the project has already paid for once.

**Rejected: a new `docs/SCREENFRAME.md`.** That would be the replacement
document the owner ruled out, and it would recreate the exact failure the brief's
own prose diagnosed: *"the apparatus column shipped because an obligation lived
in a document nobody implementing a screen had a reason to open."* The component
file is the place a screen author already has open.

This is the change's one instance of moving material, and it moves it *inward*
— from a document to the code it constrains — which is the direction this repo
prefers.

## 4. A pre-existing defect found and deliberately not fixed

`sanitise.rs` quoted a document it called **`SPEC.md`**, which does not exist
and — checked with `git log --diff-filter=D` — **never existed in this
repository's history**. Six such citations remain across `sanitise.rs`,
`SanitisedText.qml`, `FeedScreen.qml`, `tst_sanitised_text.qml` and
`IDENTICON.md`.

Worse, the text `sanitise.rs` attributed to `SPEC.md` — *"Strip or visibly mark
bidirectional overrides…"* — existed in exactly one file in the repo:
`docs/UI-BRIEF.md`, obligation 1. So the attribution was wrong **and** the
quotation's only real home was the file this change deletes.

**Two of the six were unavoidable and are fixed here**, because they sat inside
the comment block being repaired and one of them quoted the deleted file. Both
now state the rule directly instead of quoting a phantom.

**The other four are left alone and reported.** They are in files this piece has
no other reason to open, and sweeping them in would widen a documentation
deletion into an unrelated citation audit. They are a real defect and deserve
their own piece.

## 5. Gates

Run in the worktree, after the change:

- `cargo test -p dialectica -p dialectica-core` — **889 + 30 passed, 0 failed.**
- `run-qml-tests.sh` — **13 spec files, 274 passed, 0 failed.**
- `check_qml_names.py` — ok, 31 files and 17 qmldir entries.
- `check_qml_members.sh` — ok, 18 files.

`cargo fmt --check` reports diffs in `dialectica-core`, and **they are
pre-existing**: measured by stashing this change and re-running, which produces
byte-identical output. This is the known gap where `cargo fmt` does not follow
path dependencies, so the crate holding the logic has never been
format-checked. None of the diffs is in a file this change touches.

### What the gates cannot show

Every gate here is green on a tree with the brief deleted **and** on a tree where
the citations were left dangling — a dead documentation link fails no test. So
the gates prove this change *broke nothing*; they cannot prove the prose repairs
are right. That is a reading judgement, and §2's table is what a reviewer should
check rather than the green run.
