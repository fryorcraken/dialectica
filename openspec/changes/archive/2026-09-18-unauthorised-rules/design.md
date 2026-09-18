# Design — removing unauthorised general rules from comments

## The shape of the problem

The owner's ruling was about one comment: `FeedScreen.qml` had turned a local
implementation fact into a repo-wide design rule, and *"remove the rule from the
repo, I never authorised such a rule."* The approved sweep found seven more of
the same kind, which makes the interesting question not "which sentences go" but
**why a codebase with this much care about evidence produced eight of them.**

### The structural observation: a template that travels faster than its warrant

This repo has a house comment template — **"X, which is worse than Y"**. It is
one of the codebase's best habits *where it is earned*: it names the alternative
that was rejected and the cost of choosing it, which is exactly what a future
reader needs and cannot reconstruct.

That is also why it spreads. The sweep measured 19 instances of "worse than" in
`dialectica-ui/` alone. The template is **rhetorically complete without being
evidentially complete**: it reads as a settled verdict whether or not anything
settled it, and a copy of the form carries all of the authority-signalling and
none of the authority. Three consequences visible in this tree:

- **Grounded and ungrounded instances are typographically identical.**
  `VoteControl.qml:20` ("worse than printing nothing") is ratified by a spec
  `SHALL NOT`; `FeedScreen.qml:348` ("worse than absent") contradicted one. From
  the comment alone, a reader cannot tell which is which — both are confident,
  both give a reason, neither cites.
- **The copy drifts harder than the original.** `PLAN.md` §10 attributes the
  anti-false-green observation to *Radicle's* CI and instructs "copy the habits".
  Two gate-test files had become **"which this repo rates as worse than no
  gate"** — first person, no attribution, promoted from an inherited habit to a
  house doctrine. Neither file changed what it checks; only the standing of the
  claim moved.
- **A missing source survives being cited.** `docs/IDENTICON.md:21` grounds the
  address-on-screen rule in "`SPEC.md` requires exactly that" — and `SPEC.md`
  does not exist anywhere in the repo (it was a design bundle outside the tree).
  `DIdentityChip.qml` then cited that as "the standing rule", so the chain ran
  comment → note → nothing, with each link reading as sourced.

**This is why the fix is prose and not a gate.** No check can read English for
whether a sentence claims authority it lacks. What the sweep can leave behind is
this account, so the next reader recognises the pattern rather than re-deriving
it from eight instances.

## Decisions

### D1 — Demote, do not delete, wherever a real fact is present

**Taken:** at each site, keep the local observation, the measurement and the
mechanism; remove only the clause that binds work beyond that site.

The tempting alternative was deleting whole comment blocks, which is faster and
obviously discharges the ruling. Rejected because the overreaching clause is
usually welded to something genuinely valuable that nothing else records:

- `DComposer.qml` carries a **measured defect** — `DComposer { kind: "post";
  parentOp: "deadbeef" }` was accepted and silently dropped the parent, with no
  warning, refusal or test. That is a reproduction, and it stays. Only the rule
  it was generalised into ("a sentence that reads as a constraint … is worse
  than no sentence") goes.
- `check_qml_members.sh` carries a **measured trade** — `-W 0` fails on any
  qmllint category, the tree is measured clean of the others, and the named
  alternative is a hand-maintained sweep list. All of that stays. Only the
  pre-emptive *"never to raise the `-W` ceiling"* goes, because it forbids a
  future fix nobody has evaluated. The cost of raising it is still stated, which
  is what a future engineer actually needs.
- `tst_stoa_screens.qml` carries a **mutation-testing finding** — an absence
  assertion scanned a fresh preview whose body said nothing about joining, so
  planting the forbidden text elsewhere left it passing. That stays, along with
  the test that pins the corpus. Only the methodology rule imposed on every
  future absence assertion goes.

**What breaks without this decision:** deleting these blocks would lose three
reproductions that cost real effort to find, and the archive is the only other
place they exist. A finding removed as collateral to a prose sweep is a finding
nobody knows to re-derive.

### D2 — The primary target is removed for contradicting the spec, not only for lacking authority

**Taken:** `FeedScreen.qml` keeps "one ordering, nothing to select, `reload()`
does not read `ordering`" and loses "worse than absent: it reads as a working
control".

The second reason is the load-bearing one and is worth recording, because the
first reason alone would invite someone to re-add the sentence once an owner
approved it. `openspec/specs/composer-view/spec.md` **requires** inert
affordances: where a row lacks a field an affordance needs, "the view SHALL
render the row and SHALL render that affordance **inert** — present but offering
no action", and that requirement adds "This rule is general and SHALL NOT be
read as being about any one field."

An inert affordance *is* a control wired to nothing. The deleted sentence
therefore did not merely lack authority — it told a future reader that the thing
the contract mandates is a defect. The spec also argues *why*, in terms the
comment inverted: rendering inert confines the damage of one malformed row,
where the alternatives hide peer content or let one row blank a feed.

**What breaks without this decision:** a reader implementing the inert-affordance
requirement meets a comment in the view telling them not to, and has no way to
tell which document outranks the other.

### D3 — `VoteControl.qml` is left untouched, and the reason is recorded here

**Taken:** no edit, deliberately.

It matches the pattern by eye — same template, same "worse than" construction,
same subject area — and it is **ratified**. `composer-view`'s "The vote control
displays no score" is a `SHALL NOT` carrying the same argument the comment
makes: a control rendering its default zero "would be worse than one rendering
nothing, because zero is a number and reads as a tally".

This is recorded rather than left silent because the next sweep will find it
again. Editing it would be a spec change wearing a comment change's clothes, and
belongs to a different piece.

**What breaks without this decision:** a later sweep "finishes the job" by
removing a comment that a ratified requirement depends on, with every gate green.

### D4 — Where a claim is partly grounded, split it rather than choosing a side

**Taken:** `DIdentityChip.qml` now states, in two labelled halves, which part of
its claim is grounded and which is a local judgement.

This site was the hardest, and the brief's expectation — cite the source and keep
the grounded part — did not survive checking the source. Measured:

- **Grounded.** The mark is not an identifier. `docs/IDENTICON.md` argues it by
  pigeonhole, and `generated-names` contracts the same of a name: "A display
  name SHALL NOT be treated as unique, SHALL NOT be accepted anywhere an
  identity is named".
- **Not grounded.** Any requirement to place an address beside a mark. No spec
  pairs the two. `IDENTICON.md`'s "must be on screen, not one click away" cites
  a `SPEC.md` that is not in the repo. And `op-format/spec.md`'s "show the Stoa
  address alongside any name" is scoped to Stoas — the spec says outright that
  the sentence was scoped "because it is the one a later reader would cite out
  of context as authority for an author address".

So the comment's "the standing rule wherever an identity is named" was a widened
paraphrase of a note whose own citation is dangling, in a repo whose spec
pre-emptively refused that exact reading. Naming a source would have been a
second wrong citation; deleting the claim would have lost a correct one.

**What breaks without this decision:** the honest form is what stops the cycle.
A confident unsourced sentence is what produced this whole piece, and replacing
one with another — even a better-researched one — reproduces the failure.

### D5 — The anti-false-green reasoning is kept and re-attributed, not softened away

**Taken:** the four gate sites keep the argument in full and cite `PLAN.md` §10
accurately, including that the observation comes from Radicle's CI.

Two things were wrong and only one was the brief's:

- **The standing.** "Which this repo rates as worse than no gate" asserts a house
  verdict where PLAN.md reports an inherited observation and says "copy the
  habits".
- **The section title.** §10 is titled **"CI"**; "Anti-false-green" is a bolded
  paragraph lead inside it. My own first pass introduced `§10
  ("Anti-false-green")` as the citation and it was corrected — recorded because
  it is precisely the failure this piece is about, committed while fixing it.

`run-qml-tests.sh:19` was **checked and left alone**: "which PLAN.md §10 calls
worse than no gate at all" is accurate, since PLAN.md does call it that.

**What breaks without this decision:** the reasoning is sound and load-bearing —
an empty corpus must fail, or the gate reports clean having measured nothing.
Weakening the prose to discharge the ruling would trade a real defence for a
cosmetic fix. The clause that had to go was about *whose verdict it is*, not
about whether it is right.

### D6 — Green and orange become a present fact, not a prohibition

**Taken:** `DStatusBar.qml` and `DTheme.qml` now say these are *currently* the
only interface green and orange, and why that exclusivity carries meaning.

The archive already settled this and the comment had not caught up:
`2026-09-17-ui-shell-components/design.md` records that **nothing executable
reserves green and orange** — the claim's cited source is the same missing
`SPEC.md`, no gate exists, and the tree is correct only as measured on that day.
A sentence binding every future component was therefore resting on a measurement
of the present.

Phrased as a present fact it is self-invalidating in CLAUDE.md's sense: a reader
can check it, and it fails visibly if a fourth colour lands. Phrased as a
prohibition it could only fail silently.

## What a gate cannot see here

Stated plainly, because every gate passed both before and after these edits and
a reader could mistake that for verification.

**No gate in this repo can check the property this change is about.** The edits
are English prose; the gates compile QML, resolve members and names, and run
specs. Their green proves the edits broke nothing — the expected outcome for a
comment-only change, and not evidence the sweep is correct.

What the gate runs *did* verify is narrow and real: four of the edited comments
live inside a shell script and a Python module, where a mangled comment can break
the file containing it. `tst_check_qml_members.sh` and `tst_check_qml_names.py`
both pass, including the empty-corpus cases whose comments were edited.

The review that can see this change is a reader comparing the prose against
`openspec/specs/`.

## Reasoning migrated from PLAN.md

None. This change removes unratified sentences from comments; it implements no
PLAN.md behaviour and closes no PLAN.md section, so there is no passage to move.
The `SPEC.md` and green/orange findings this piece relies on were already
recorded in the archive, and are cited above rather than copied — two copies
drift.
