## Context

See `proposal.md` — Why. Four owner rulings about what the MVP contains, all of
them scope rather than design.

The design-level constraint that shapes every edit below: **this change alters
no behaviour, so every edit must be legible as a scope record and not as a
withdrawal.** `docs/PLAN.md` is the only file touched. The capabilities the
rulings touch — `content-authoring`, `composer-view`, `moderation-resolution`,
`feed-view`, `stoa-navigation-view` — stay merged, tested and current, and a
future reader must be able to tell that from PLAN.md alone, without having to
diff the specs to discover nothing moved.

Two properties of the target document shape the approach. **PLAN.md is read in
fragments**: §7.2 is read by whoever builds ranking, §9.2 by whoever builds a
screen, and neither reads the other. And **PLAN.md's own convention is strike
and point** (CLAUDE.md, "Keeping this file true"), so a superseded passage stays
visible with a pointer to what replaced it.

## Goals / Non-Goals

**Goals:**

- Each ruling is findable from the section a reader is already in, not only from
  §9.2. A ruling recorded once, in the staging section, is a ruling the ranking
  author never sees.
- The case-2 placeholder set is a **list**, so the documentation condition in
  ruling 4 is checkable by reading one place rather than by sweeping screens.
- The boundary between a PLAN.md scope note and a merged spec requirement is
  stated in PLAN.md, because that is the thing a later reader is most likely to
  get wrong.

**Non-Goals:**

- **No spec delta, and no relaxation of any requirement.** Where a merged
  requirement forbids a placeholder, it continues to forbid one.
- **No general rule about inert controls.** This change writes neither "inert is
  acceptable" nor "inert is worse than absent" as a repo-wide principle; see
  Decision 4.
- **No renumbering of §9.1's stages** and no edit to §7.2's six rules. Ruling 1
  changes §7.2's MVP membership, not its content.

## Decisions

### 1. Each ruling is recorded at its own section, with §9.2 as the index

**Chosen:** a short block-quoted note at the head of the section a reader is
already in (§7.2 for voting, §9.1's question 8 for unread), plus the entry in
§9.2's list. §6's existing note — *"Out of the MVP, by owner decision — scope,
not a design change"* — is the model, and it is block-quoted at the head of §6
for exactly this reason.

**Rejected: record all four in §9.2 only.** It is the tidier diff and it fails
on the access pattern. §7.2 opens *"This is where dialectica is investing"*,
which is a true statement about the project and a false one about the MVP; a
reader who arrives at §7.2 to build ranking and never scrolls to §9.2 takes that
sentence as a staging instruction. The cost of the duplication is one line per
section pointing at §9.2, which is a pointer rather than a second copy of the
reasoning.

**What breaks without this:** nothing a test can see — which is why it is
recorded here. The failure mode is a future agent building vote weighting first
because §7.2 told it to.

### 2. §9.1's question 8 is rewritten in place, not struck through

Ruling 2 is the one ruling that changes an entry's **kind** rather than its
content: an open question invites the next agent to answer it, and this one is
answered for the MVP. So the entry is rewritten as a decided exclusion, **keeping
its reasoning verbatim and in place** — the peer-local-state argument, which is
not restated here; `docs/PLAN.md` §9.1 question 8 is its only copy, and
Decision 6c records why it was not migrated.

**Rejected: strike the bullet and add a ruling below it.** PLAN.md's strike
convention exists for a claim that became *false*. This one did not become
false; it stopped being a question. Striking it would say the reasoning was
withdrawn, which is the opposite of what happened — the reasoning is precisely
why the answer is "not in the MVP".

The reasoning stays in PLAN.md rather than migrating to this file because it is
reasoning about something **still not built**, which is PLAN.md's job. Only
reasoning attached to the behaviour this change records moves here.

### 3. The case-2 placeholders are a list section in §9.2, not annotations at each site

**Chosen:** one list in §9.2, each entry naming the control, the reason core
cannot serve it, and the citation that establishes the absence.

**Rejected: rely on the comment already at each site.** All five are already
documented at their sites — `feed.rs:96-98`, `VoteControl.qml:17-23`,
`DStoaListScreen.qml:390-399` and the rest — and that is not enough, because
ruling 4's condition is that the set is *findable*. A set documented only at its
members' sites is one you can confirm an instance of and never enumerate. That
is this repo's `hand-maintained sweep lists go stale silently` trap read from
the other end: the list does not replace the site comments, it makes the
absences countable so they can be worked off rather than rediscovered screen by
screen.

**Rejected: a machine-checkable marker.** A grep-able `// CASE 2:` token would
make the condition enforceable, and it is the right shape if this set outlives
the MVP. It is not built now because the set has five members and one consumer,
and a gate whose corpus is five hand-written comments is a gate that measures
the comments rather than the placeholders. Recorded so the next person reaches
for it deliberately rather than re-deriving it.

**The list is seeded with five verified entries**, each re-read against this
tree rather than relayed from the proposal.

### 4. No repo-wide rule about inert controls, in either direction

This is the decision most likely to be re-litigated, so the argument is recorded
rather than the conclusion.

`FeedScreen.qml:344-350` argues that an inert ordering row *"reads as a working
control"* and is *"worse than absent"*. That is a local observation about one
row. Generalised into a repo rule it **contradicts a ratified requirement**:
`composer-view/spec.md:638-639` requires that where a row lacks a field an
affordance needs, the view *"SHALL render that affordance **inert** — present
but offering no action"* rather than omitting the row. A rule saying inert is
worse than absent would put PLAN.md in conflict with a merged spec, which is a
conflict PLAN.md loses and should never have been in.

So PLAN.md records **case 2 as a permission, bounded by what the specs forbid**,
and cites `FeedScreen.qml` nowhere.

**What breaks without this:** nothing red. A generalised rule would sit in
PLAN.md agreeing with itself, and the next screen author following it would
write a view that fails `composer-view`'s requirement — caught, if at all, by a
spec reviewer noticing the contradiction rather than by any gate.

### 5. The two narrowing requirements are named in PLAN.md, not paraphrased

`stoa-navigation-view/spec.md:123` (*"Every number rendered is one this peer can
actually answer"*) and `composer-view/spec.md:581` (*"The vote control displays
no score"*) each forbid a placeholder in a place case 2 would otherwise permit
one.

PLAN.md states both by requirement name and states that a scope note does not
override them. The alternative — leaving case 2 unqualified and trusting a
reader to find the specs — is the failure this change exists to prevent in its
other three rulings: scope that lives only in a conversation is scope the next
agent re-derives from the design documents.

Naming them by requirement rather than restating their substance follows
`.claude/agents/README.md`'s rule in reverse: a spec must not cite a PLAN
section number, because PLAN headings are unstable. PLAN.md citing a
**requirement name** is safe in a way the reverse is not — requirement names are
the specs' stable identifiers, and `openspec` renames them visibly.

### 6. Reasoning migrated out of PLAN.md into this file

Per `.claude/agents/README.md` — reasoning attached to a landed decision moves
to `design.md` and is **removed** from PLAN.md, because two copies drift and the
wrong one gets read. **Two passages moved here** (6a and 6b), each now recorded
once; 6c records the third candidate and why it stayed in PLAN.md instead.

#### 6a. Why item 5 contradicted a §9.1 decision — and how the contradiction ended

§9.2 carried a paragraph naming votes as *"the one item that contradicts a §9.1
decision"*. The tension it described was real and is now resolved, so the
paragraph's reasoning belongs here rather than in a document describing what is
not built.

§9.1 deliberately declined to stage votes. The reasoning: §7.2 rule 2 ships no
score, so a vote button publishes an op that changes nothing a reader sees —
*"a control with no visible effect teaches users the app is broken."* §9.2's MVP
list nonetheless carried `Upvote / downvote` as item 5, and the paragraph
recorded the disagreement rather than reconciling it quietly, naming two honest
options for whoever built the control: a visible per-post tally that is not a
ranking, or a control whose effect the copy does not overstate.

**Ruling 1 resolves it in §9.1's direction**, and the resolution is cleaner than
either option the paragraph offered, because one of them turned out not to
exist. A per-post tally is **not** an available option: no call returns a score,
a tally, or the viewer's earlier votes. `composer-view` resolved the interface
half in the only direction left — the control shows the viewer their own vote
back and no number at all.

So the question §9.2 flagged as open at PLAN.md:3599-3608 is closed twice over:
by the contract, which never offered the tally, and by the ruling, which takes
the control out of the MVP.

#### 6b. Why a rendered zero is worse than a rendered absence

§9.2's closing paragraph pointed at the `composer-view` change's `proposal.md`
for this. It is the load-bearing reason the vote control renders nothing, and it
generalises to every case-2 placeholder, so it is stated here rather than left
one hop away.

A control with a score property and nothing to bind it to renders `0`. **Zero is
a number, so it reads as a tally — and it reads as the tally ZERO, a claim that
this post is known to have received no votes.** Core has never said that, and it
is false the moment any peer votes. An absent number says "not known here",
which is true.

The mechanism that makes this hold is a default rather than a convention:
`VoteControl.qml` gates the numeral behind a property **defaulting to false**,
so a call site that forgets the property shows no number instead of a fabricated
one. Showing a count becomes something a later change must decide to do — which
is the point, since that change is the one that must first find a count to show.

**This is the general shape for a case-2 placeholder, and the reason case 2 is a
permission rather than a default**: an inert control makes no claim, and a
placeholder *value* usually does. Where the placeholder would be a number, it is
almost always forbidden — which is why both narrowing requirements in Decision 5
are about numbers.

#### 6c. Nothing migrated for unread — and why that is not an omission

**Two passages moved here, not three.** Decision 2 keeps §9.1 question 8's
reasoning in PLAN.md, because it is reasoning about something still not built.
An earlier draft of this file also restated that reasoning here, under a heading
claiming to migrate "only insofar as it explains the ruling" — which did not
divide anything, because for unread the ruling-reasoning and the not-built
reasoning are the same sentences. The result was the same argument in two files,
which is the drift Decision 6 exists to prevent; it is removed rather than
reworded, and `docs/PLAN.md` §9.1 question 8 is the single copy.

This heading is kept rather than deleted so the asymmetry is visible: a reader
comparing Decision 2 against 6a/6b would otherwise wonder whether the third
migration was forgotten. It was declined, and Decision 2 is the argument.

### 7. Case 1's `MUST` stays in PLAN.md, and stays a `MUST`

Ruling 4's case 1 — *"core serves it: the control MUST be wired"* — is PLAN.md's
first **first-person** RFC-2119 keyword. Its only prior `MUST` (line 509) quotes
an external protocol document's requirement; this one asserts an obligation of
our own, in a document whose stated job is "what exists, and what is not built
yet", and nothing in `openspec/specs/` carries it.

**Chosen: keep the wording, keep it here.** The phrasing is the owner's ruling
quoted — *"if core exists, then the button must be wired"*, given as a scope
ruling for the MVP phase. This change exists to record scope without altering
it, so rewording the ruling into "should" would be the change editing its own
subject.

**Rejected: soften to "should be wired".** It removes the keyword-force
objection and it removes the ruling's content with it. Case 1's whole point is
that inertness is *not* an acceptable shortcut where the call exists; "should"
is precisely the register that permits the shortcut.

**Rejected: promote case 1 to a requirement in `openspec/specs/`.** Two
arguments, and the second is the load-bearing one. The weaker: case 1 has no
fixed enumerable subject — the set of wired-able controls widens with every
trait method, which is this repo's `hand-maintained sweep lists go stale
silently` trap, so a requirement naming them is stale on the next method. That
is an argument about testability. The stronger: **a spec contracts what the
system does; case 1 contracts what a phase may ship.** It is true of the MVP and
expires with it, and staging has never been spec-level here. A requirement
reading "during the MVP" is one nothing retires, and `openspec/specs/` would
then carry a contract that outlives its own subject.

**What breaks without this:** a reader who has internalised RFC-2119 from §4's
SDS quotations reads line 3534 as contractual and finds no test behind it. That
risk is real and mitigated only by placement — the sentence introducing the four
rulings calls them "scope decisions... none is a design change", and the
two-merged-requirements block below states that a scope note does not override a
spec. Both sit within a screen of the MUST. Recorded because the next reader to
notice the keyword will ask this question again, and the answer is a decision
rather than an oversight.

### 8. No CI gate on "worse than absent"-style phrasing

Decision 4 declines to write a repo-wide rule about inert controls. A mechanism
was proposed to *keep it declined*: a CI gate scanning prose for "worse than
absent"-style phrasing, so that a later change generalising `FeedScreen.qml`'s
local comment into a PLAN.md or spec rule would fail a check rather than pass
review unnoticed.

**Rejected as over-engineering for the MVP phase.** The failure it guards
against is one sentence appearing in one document, with exactly one existing
instance — a code comment that is correct where it sits. A phrase-matching gate
over prose has the two properties this repo has already been bitten by: its
corpus is hand-written English, so it measures the wording rather than the rule
(the same objection Decision 3 makes to a `// CASE 2:` marker), and it produces
false positives on every legitimate *citation* of the comment — of which this
change's own `docs/PLAN.md` case-2 list contains one, so the gate would have
failed on the change that introduced it.

**Rejected alternative, for the same reason it is worth naming:** a narrower
gate keyed to PLAN.md and `openspec/specs/` only, exempting citations. That is
a smaller corpus but the exemption is the hard part — distinguishing "PLAN.md
states this as a rule" from "PLAN.md quotes a file that states it locally" is
the judgement the gate was supposed to replace.

**What breaks without this:** nothing red, and that is the honest statement of
the residual risk. Decision 4's argument is enforced by review and by
`composer-view/spec.md:638-639` contradicting the generalisation — a spec
reviewer noticing the conflict, not a gate. Recorded so the next agent tempted
to build the gate reaches for it deliberately, with the false-positive problem
already known rather than rediscovered.

### 9. `FeedScreen.qml:344-350` is left untouched, and deferred rather than swept

The comment Decision 4 spends the most space on is the one file this change does
not edit. That is a choice, not an omission.

**Chosen: leave it entirely alone**, and let a separate piece deal with it. The
`unauthorised-rules` piece owns that comment and has done the work there, which
is why this change touches no file under `dialectica-ui/`.

**Rejected: annotate the comment in place** — a line pointing at ruling 4, so a
reader hitting `FeedScreen.qml` sees the scope note rather than an unqualified
local claim. It is the more helpful end state and it was still the wrong move
here, for two reasons. It would put a `dialectica-ui/` edit into a change whose
entire reviewable claim is *"`docs/PLAN.md` only, no behaviour"* — the property
that lets a reviewer verify the piece by reading one diff, and the basis of
`.openspec.yaml`'s `skip_specs: true` argument. And two changes editing the same
comment for different reasons is the collision that produces a silent merge
keeping both edits.

**What breaks without this:** nothing, and the deferral is durable rather than
lost — the comment is the other piece's subject, not a note left for whoever
finds it next.

## Risks / Trade-offs

**A scope note is read as a permission to render a number the specs forbid** →
Decision 5 names both requirements in PLAN.md and states that relaxing them is a
spec change. The residual risk is real: PLAN.md cannot enforce this, and the
enforcement lives in the merged specs and their tests.

**The case-2 list goes stale as core grows** → it is dated to the MVP phase and
each entry cites the file that establishes the absence, so an entry whose
citation no longer says what it claims is checkable against the tree. This is
mitigation, not prevention; nothing fails if an entry is worked off and not
removed. Decision 3 records the machine-checkable alternative and why it was not
built at five members.

**Recording a ruling at two sections lets the two drift** → the section-level
notes are pointers carrying the ruling and no reasoning, with §9.2 as the single
place the substance lives. A pointer can go stale in only one way — pointing at
a renamed section — which is visible, unlike two copies of an argument
disagreeing.

**A reader takes "out of the MVP" as "withdrawn"** → every entry states that the
design stays and the specs stay merged, following §6's model, and the Impact
section of `proposal.md` says the same. This is the failure the §6 wording was
already written to prevent.

## Migration Plan

Not applicable — a documentation change to one tracked file, revertible by
reverting the commit. No build artefact, no data format, no deployed behaviour.

## Open Questions

None that can be deferred. **Two mechanisms are deliberately not built rather
than undecided**, each with its rejection recorded where the next person will
look for it: Decision 3's grep-able `// CASE 2:` marker, whose condition is met
today by the list, and Decision 8's CI gate on "worse than absent"-style
phrasing, whose false-positive problem is recorded so it is not rediscovered.

Decision 7 notes one thing that is genuinely open, though not blocking: whether
case 1 should eventually become contract material rather than a PLAN.md scope
rule. It is left to the owner, since it means re-deciding an owner ruling, and
nothing in this change forecloses it.
