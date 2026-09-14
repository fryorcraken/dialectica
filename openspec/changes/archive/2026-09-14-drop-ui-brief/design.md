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

**The grep to re-run** is `git grep -n "UI-BRIEF\|UI brief\|ui-brief\|UI_BRIEF"`
over `docs/`, `dialectica/`, `dialectica-ui/`, `CLAUDE.md` and
`openspec/specs/`. (`git grep` rather than `grep -rn`: the latter descends into
`rust-lib/target/`, where build artefacts produce hundreds of spurious hits.)

It returns **one** hit on this branch, deliberately: the `NO SPEC:` marker at
`tst_feed_extent_claim.qml:21`, which records that the extent-locality rule was
carried only by the deleted document and is now held up by those tests alone.
That is the one place naming the file is the point — see §5.1.

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
| §5.5 (`:2446`) | quoted the brief's "never auto-join" rule | **re-grounded** — cites `stoa-navigation-view`'s "Joining shows what is being joined, and joins nothing until the user acts" |
| §5.5 (`:2451`) | "`docs/UI-BRIEF.md` describes only what does" | **restated** — the two interface notes are recorded here because no surface exists for them to be contracted against, which is the real reason |
| §12 (`:4414`) | "the brief carries the half of the rendering obligation that is true today" | **re-grounded** — the view half is contracted by `composer-view`'s "A successful publish claims local storage and never delivery"; PLAN.md keeps only the *ordering* decision |

The last is the most consequential, and an earlier draft got it wrong in an
instructive way: it **restated** the prohibition in PLAN.md, on the stated
premise that "no other document states it in prose". That premise was false.
`openspec/specs/composer-view/spec.md:294` had contracted it throughout, in
prose, with five scenarios — covering every clause the restatement offered plus
a stronger obligation the brief never had (`:309`, the view must *positively
deny* delivery knowledge rather than merely stay silent). Specs were untouched
by this piece, so that requirement was live the whole time.

The rule was right and the reason was invented. That is the same shape as §2.3,
reached from the other direction: rather than fabricating support, it asserted
that no support existed. **Re-grounded on the spec**, which is what PLAN.md's own
convention does at `:2904` and `:3530` for this very capability. What stays in
PLAN.md is the part a spec does not carry — why the view half did not wait on
the three owed things.

### 2.3 A fabricated decision record, withdrawn

An earlier draft of this section recorded a decision to keep `agora` in a
**"wordlist exclusion table"** with a **"first-to-drop flag"**, re-grounded on
test fixtures, citing `docs/PLAN.md:1368`. **Every load-bearing part of that was
false**, and it is recorded here rather than silently deleted because the way it
came to be written is the thing worth carrying.

- **No such table exists.** The only exclusion in `names.rs` is a two-line
  literal-substring check (`:1345-1351`): no `NOUNS` entry may contain the
  connector `" of "` as a word, because `zenon of kition` would render *pensive
  zenon of kition of lampsakos*. Its own comment insists it "is a rule about ONE
  LITERAL SUBSTRING and not a semantic screen".
- **There is no first-to-drop flag**, anywhere.
- **`stoa` is not excluded.** The test at `:1354` asserts `stoa` and `agora` are
  **present**, so "the same failure `stoa` itself is excluded for" inverted the
  contract. (That test's *name* says the lists carry no exclusion of any kind,
  which is broader than what it checks — it pins that nothing is kept out for
  what it says, connotes or whom it names, and is silent on the compound-noun
  rule twenty lines above it. Do not cite the name for the stronger claim.)
- **`docs/PLAN.md:1368` is about credential expiry windows**, not wordlists.
- **Nothing in this change touches a wordlist at all** — no wordlist, `names/`
  or spec file appears in the diff.

**The mechanism, which is this change's own subject seen from the inside.** The
brief used "Join *Agora*?" as a worked example. When the brief went, the entry
lost its only support — and a rewrite that must preserve a conclusion while
discarding its only support has to invent new support. Invented support is
indistinguishable from real support to every reader downstream, which is exactly
how an exclusion screen that does not exist got written down as a decision.

The correct move was the one the rest of this change makes: where a claim's only
support was the brief, delete the claim. There was no wordlist decision to
record, because no wordlist was touched.

**A second fabrication, removed in the same pass.** `names.rs` carried a record
of four "withdrawn screens" — familiarity, a rebadged "legibility", a single-word
rule, a tone-and-authority apparatus, with percentages attached. On the owner's
ruling that is **not** a record of options weighed and declined: those rules were
invented by an agent, and the record of them was deleted rather than reworded.

An earlier draft of this very section argued for keeping it, on the reasoning
that a record of withdrawals is what stops a fifth being proposed. That
reasoning is the trap in miniature: it treats an agent's confident write-up as
evidence of a decision, and would have preserved invented history on the
strength of how useful it sounded.

**What actually constrains the lists** is the one rule that survives owner
review: `names.rs:1345-1351`, no `NOUNS` entry may contain `" of "` as a word,
which is a literal substring check and explicitly not a semantic screen. The
lists are otherwise screened only mechanically — ASCII, deduplicated, attested.

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

Verified by running both suites — see §6.

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

**This is the one place in the brief whose content was not output**, which is
what makes moving it inward legitimate rather than laundering. Every clause is a
fact about *our* implementation, measured by our own tests — `ScreenFrame.qml`'s
own body carries the numbers and `tst_screen_frame_geometry.qml:183` pins the
zero. The designer's handoff has its own reference `ScreenFrame.qml` and it
contains none of this, because the trap is a property of the code rather than of
the design. So the material is re-grounded on the tests that establish it, not
restated from a document.

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

## 4. The `SPEC.md` citations are correct, and an earlier draft nearly broke them

An earlier draft of this section called `SPEC.md` a **phantom** — "does not
exist and never existed in this repository's history" — on the strength of
`git log --all --diff-filter=D -- "*SPEC.md"` returning nothing, and rewrote two
`sanitise.rs` citations to stop quoting it.

**That was wrong, and the git check is why it was convincing.** `SPEC.md` is the
designer's handoff at `tmp/ui-bundle-new/handoff/SPEC.md`. `tmp/` is gitignored,
so a history search cannot see it — a clean `git log` proved only that the file
was never *tracked*, not that it never existed, and the two were conflated.

Every one of the **seven** citations in the tree resolves against it:

| Citation | `SPEC.md` |
|---|---|
| `sanitise.rs:24` (the quoted line) | "Strip or visibly mark bidirectional overrides… correcting changes what the record says" |
| `sanitise.rs:56` | "never bind raw peer text to a Text element with textFormat StyledText or RichText" |
| `sanitise.rs:96` | the codepoint ranges |
| `SanitisedText.qml:10`, `:39`, `tst_sanitised_text.qml:13` | the same two rules |
| `FeedScreen.qml:330` | "build the row from a model, never from hard-coded items" |
| `IDENTICON.md:129` | "Order matters", of the fill-ink pair |

(Seven, not the six an earlier count claimed — `sanitise.rs` and
`SanitisedText.qml` carry two each, which one-per-file arithmetic missed.)

**So the two rewrites are reverted and the citations restored.** This is the
inverse of §2.3's error and worth holding beside it: there, support was invented
for a claim that had lost it; here, a correct citation to genuine input was
dissolved into prose that argued for itself. Both replace an external referent
with the repository's own voice, and both make the claim *look* better sourced
while making it less so.

**The distinction that decides it: `SPEC.md` is input and `UI-BRIEF.md` was
output.** The handoff comes *from* the designer *to* this codebase, so citing it
points at evidence. The brief went the other way — a summary written *from* the
code *for* an external reader — so citing it pointed at ourselves, one lossy lap
later. That is the whole reason one is deleted and the other is kept.

## 5. One rule left standing on tests alone, and it is marked

### 5.1 The extent-locality rule

*"Where the interface asserts extent, that assertion must be readable as local"*
— the rule behind the feed's pagination control and the whole of
`tst_feed_extent_claim.qml`.

**No live requirement contracts it.** Checked: no spec in `openspec/specs/`
mentions extent, `hasMore` or pagination beyond `module-wire-contract`'s
envelope. `SPEC.md` requires "Pagination only. No infinite scroll, no totals",
which fixes the **control** and says nothing about what the control may be read
as claiming.

Its *substance* is nonetheless derivable from the code rather than from the
deleted document: `feed::list_threads` computes `hasMore` from this peer's log
alone, so "Next" is a fact about this machine and a reader takes it for a fact
about the Stoa. That derivation is now written out at the head of
`tst_feed_extent_claim.qml`, with a **`NO SPEC:`** marker naming the gap and the
deleted document as its former home.

**Neither deleted nor re-grounded on invented support**, because it is enforced
by live tests and by a shipped control. It is flagged instead: the marker is what
a spec-writer should find, and closing the gap is a decision for the owner rather
than something to settle inside a documentation deletion.

## 6. Gates

Run in the worktree, after the change:

- `cargo test -p dialectica -p dialectica-core` — **925 + 30 passed, 0 failed.**
- `run-qml-tests.sh` — **13 spec files, 278 passed, 0 failed.**
- `check_qml_names.py` — ok, 31 files and 17 qmldir entries.
- `check_qml_members.sh` — ok, 18 files.

`cargo fmt --check` on the **workspace** manifest is clean — which is exactly
the known gap, because it does not follow the path dependency into
`dialectica-core`, the crate holding all the logic. Pointing it at
`dialectica-core/Cargo.toml` directly reports diffs in `keystore.rs`,
`moderation.rs`, `op.rs` and `revision.rs`.

**Those are pre-existing**, measured rather than assumed: stashing this change
and re-running produces byte-identical output. **None is in a file this change
touches** — the four files named above appear nowhere in this diff.

### What the gates cannot show

Every gate here is green on a tree with the brief deleted **and** on a tree where
the citations were left dangling — a dead documentation link fails no test. So
the gates prove this change *broke nothing*; they cannot prove the prose repairs
are right. That is a reading judgement, and §2's table is what a reviewer should
check rather than the green run.
