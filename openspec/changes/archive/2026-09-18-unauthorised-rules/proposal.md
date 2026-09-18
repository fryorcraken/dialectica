# Remove unauthorised general rules from code comments

## Why

A code comment in `FeedScreen.qml` generalised a local implementation fact into
a repo-wide design rule: that a control wired to nothing "reads as a working
control", which is "worse than absent". The owner's ruling is direct — **"remove
the rule from the repo, I never authorised such a rule."**

Two independent reasons that comment goes, and the second is the one that makes
this more than bookkeeping:

1. **No authority.** Nothing in `openspec/specs/`, no `design.md` Decisions
   entry, and no owner decision ratified it.
2. **It contradicts a ratified spec.** `openspec/specs/composer-view/spec.md`
   requires the opposite. Where a row lacks a field an affordance needs, "the
   view SHALL render the row and SHALL render that affordance **inert** —
   present but offering no action", and the requirement adds that the rule "is
   general and SHALL NOT be read as being about any one field". An inert
   affordance is precisely a control wired to nothing, and the spec mandates it.

A comment asserting the negation of a ratified requirement is worse than merely
unauthorised: the next person to meet an inert affordance has one document
telling them to build it and another telling them it is a defect.

The owner also approved sweeping for others of the same kind. The sweep found
that the failure is **structural, not a single lapse**. This repo has a house
comment template — *"X, which is worse than Y"* — that is genuinely excellent
where it carries a measurement or a spec citation, and that template travels
faster than the authority behind it. It is persuasive prose, so it gets copied;
copying carries the rhetorical force but not the grounding, and the copy reads
exactly as authoritative as the original.

## What changes

Comment and doc-comment prose only, across `dialectica-ui/`. **No behaviour
changes**: no logic edits, no renames, no test assertions altered.

The operation is **demotion, not deletion**, wherever a genuine fact is present.
Each site keeps its local observation, its measurement and its mechanism; what
goes is the clause that makes the observation binding on work beyond its own
site.

Sites, each verified against the specs and archive before editing:

- `FeedScreen.qml` — the primary target. Keeps the local fact (one ordering, so
  nothing to select and no handler to write); drops the generalisation and the
  "worse than absent" framing that contradicts `composer-view`.
- `DComposer.qml` — keeps the measured defect account; drops the rule about how
  comments in this repo may be written.
- `check_qml_members.sh` — keeps the measured `-W 0` trade-off; drops the
  pre-emptive "never to raise the `-W` ceiling", which forbids a future fix
  nobody has evaluated.
- `DIdentityChip.qml` — drops "should not be dropped to save a row"; keeps the
  grounded claim that a mark is not an identifier and **cites its source**
  rather than restating it as "the standing rule wherever an identity is named".
- `DStatusBar.qml`, `DTheme.qml` — demotes a palette prohibition binding every
  future component to a statement of present fact and the reason behind it.
- `tst_stoa_screens.qml` — keeps the mutation-testing finding; drops the
  methodology rule imposed on all future absence assertions.
- The "worse than no gate" doctrine in four gate files — softened to match what
  `docs/PLAN.md` §10 actually says, rather than asserting that "this repo rates"
  it so.

## What is deliberately NOT changed

- **`VoteControl.qml`.** It looks like the same class and is not: it is ratified
  by `composer-view`'s "The vote control displays no score", a SHALL NOT
  carrying the same argument. Changing it would be a spec change and belongs to
  a different piece.
- **`dialectica/rust-lib/`.** Swept and clean — every "worse than" there traces
  to a spec requirement or states a correctness property about untrusted input.
- **Measured facts, traps with a mechanism, security properties, and anything
  citing a reproduction.** These are the repo's most valuable comments. The test
  applied throughout is whether a sentence constrains future work beyond its own
  site without recorded authority — not whether it sounds opinionated.
- **Comments already carrying a `NO SPEC:` self-disclaimer**, which model the
  honest form: they say plainly that nothing ratified them.
