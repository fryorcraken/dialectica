# Architecture review — `mvp-scope`

Scope: architecture dimension only (document placement, the load-bearing
boundary claim, the case-2 sweep-list judgement call, and whether a scope
decision is the right instrument). Every citation below was re-read against
this worktree's tree, not relayed from the proposal.

## Clean

- **The load-bearing boundary claim holds.** Both narrowing citations are
  exact: `stoa-navigation-view/spec.md:123` is the requirement heading "Every
  number rendered is one this peer can actually answer", and
  `composer-view/spec.md:581` is "The vote control displays no score" —
  verified by direct read, not grep. PLAN.md §9.2 (lines 3554-3566) states the
  boundary in the same place a case-2 author will be reading, which is the
  right place to hit it.
- **Every case-2 citation checks out against source**, content and line
  numbers: `feed.rs:38-41` and `:96-98` (at
  `dialectica/rust-lib/dialectica-core/src/feed.rs`), `DStoaListScreen.qml:390-399`,
  `wire.rs:1984`, `lib.rs:310-315` and `:600`, `revision.rs:8-13`,
  `FeedScreen.qml:344-350`, and `composer-view/spec.md:638-639`. This is
  unusually well-sourced work for a prose-only change.
- **§6 (moderation) and §7.2 (relevance) scope notes are correctly shaped** —
  block-quoted, struck-and-pointed where superseded, and carry no reasoning
  that duplicates `design.md`.
- **Decision 4's argument is correct and the citation supporting it is
  accurate**: `composer-view/spec.md:638-639` does require inert-not-absent
  rendering, so generalising `FeedScreen.qml`'s local comment into a PLAN.md
  rule would have put PLAN.md in conflict with a merged spec. Right call, not
  made.
- **Decision 3's judgement call is defensible on its own terms.** It does not
  deny the case-2 list shares the sweep-list trap's structural exposure
  (nothing greps for an undocumented case-2 control); it makes a cost
  argument (five members, one consumer) and design.md's Risks section states
  plainly this is "mitigation, not prevention." That is the honest way to
  make this call, not a hidden one.

## Findings

- [ ] **`dev-writer`** — `docs/PLAN.md:3253-3272` and
      `openspec/changes/mvp-scope/design.md:207-220` — the unread-counts
      reasoning is duplicated, not migrated, contradicting Decision 2's own
      stated rule.
      **Scenario:** Decision 2 states the reasoning "stays in PLAN.md rather
      than migrating to this file because it is reasoning about something
      still not built, which is PLAN.md's job." Decision 6c is titled "Why
      unread was left undecided" and claims to be "Migrated from §9.1's
      question 8 rationale only insofar as it explains the ruling." But 6c's
      body (design.md:212-217) restates the same argument PLAN.md already
      carries live and unstruck at lines 3260-3264 — "it needs per-user local
      state that is not an op and never crosses the wire... inventing the
      first instance of that as a feed field is how it gets designed badly"
      versus design.md's "it needs per-user local state that is not an op and
      never crosses the wire... Inventing the first instance of that category
      as a feed field is how the category gets designed badly." These are the
      same sentence in two files, not two different arguments split by
      Decision 2's proposed line (ruling-reasoning vs. not-built-reasoning) —
      the ruling *is* the not-built reasoning here, so the split Decision 2
      draws does not actually separate anything. A future editor fixing a
      wording error in one copy will not know to fix the other, which is
      exactly the "two copies drift and the wrong one gets read" failure this
      change's own `.claude/agents/README.md` citation warns against.
      **Fix shape:** either delete 6c and let PLAN.md's own text stand
      (consistent with how 6a/6b handle passages that are genuinely still
      about unbuilt behaviour elsewhere), or strike the PLAN.md passage and
      point to design.md the way 6a and 6b do for their sites — pick one.

- [ ] **`dev-writer`** — `docs/PLAN.md:3534` — a first-person RFC-2119 `MUST`
      is asserted in PLAN.md with no spec behind it, which is new to this
      document and blurs the "scope note, not a requirement" framing the
      whole piece rests on.
      **Scenario:** PLAN.md's only prior use of `MUST` (line 509) quotes an
      external protocol document's own requirement ("the sender 'MUST include
      its own globally...'") — it reports somebody else's contract. Ruling 4's
      case 1, "the control MUST be wired," is PLAN.md's first first-person
      normative MUST, asserted nowhere in `openspec/specs/`
      (`grep -rln "MUST be wired" openspec/specs/` returns nothing). The
      proposal argues at length (proposal.md:190-197, `.openspec.yaml`) that
      ruling 4 cannot be spec material because it has no fixed enumerable
      subject — that is an argument against *testability*, not against
      whether an obligation phrased with spec-grade keyword force belongs in
      a document whose own stated job (per `.claude/agents/README.md`) is "a
      short summary of what exists, and what is not built yet," never a
      MUST-bearing rule. A reader who has internalised RFC-2119 conventions
      elsewhere in this repo (§4's SDS quotations) will read line 3534 as
      contractual, and nothing enforces it if they are wrong to.
      **Severity:** moderate — this is the strongest candidate for "should
      this actually be a spec change" the review was asked to test. It is
      arguable that this MUST is fine as informal shorthand rather than a
      genuine RFC-2119 assertion, but the piece does not make that argument;
      it simply uses the keyword. Worth a conscious decision (soften the
      language, or accept that this is a case for a lightweight spec/policy
      requirement instead) rather than leaving it as an accidental first use.

- [ ] **`dev-writer`** — `openspec/changes/mvp-scope/proposal.md:92-108` — the
      case-1 method table is exactly the hand-maintained sweep list the same
      document warns against 90 lines later, with no caveat pointing at the
      authoritative source.
      **Scenario:** `proposal.md:194` argues that a list of trait methods "in
      a document is this repo's `hand-maintained sweep lists go stale
      silently` trap" — and PLAN.md itself (§9.2, lines 3536-3540) acts on
      that lesson: "Read the current surface from the `Dialectica` trait...
      rather than from any list here — a list of methods in a document is
      this repo's `hand-maintained sweep lists go stale silently` trap." But
      `proposal.md` lines 92-108 contain precisely such a table — 13 methods
      with line numbers — stated flatly as fact, with no "read the trait
      instead" caveat. `proposal.md` does archive as a historical snapshot
      rather than staying live, which lowers the practical risk (nobody is
      expected to maintain it going forward), but the document explicitly
      invokes the trap by name elsewhere without noticing its own table
      instantiates it. At minimum this deserves the same one-line caveat
      PLAN.md carries ("read the trait for the current surface"), which the
      table currently lacks.
      **Severity:** low — proposal.md is archived rather than edited forever,
      so staleness here is less costly than in PLAN.md, but the
      inconsistency (warning against the pattern while using it, unflagged,
      in the same file) is worth a one-line fix.

## Not filed as findings, recorded for the record

- Decision 3's rejection of a `// CASE 2:` marker is a judgement call the
  writer flagged as most likely to be argued with. I tested it rather than
  accepting it: the list does share the sweep-list trap's structural blind
  spot (an omission is invisible to any gate), but the piece does not deny
  this — it makes an explicit, bounded cost argument and records the
  residual risk in `design.md`'s Risks section. I would not block on this;
  it is a defensible five-item judgement call, not a hidden one.
- Rulings 1–3 are correctly scope notes: each is a staging fact about a
  behaviour that is already fully contracted elsewhere (`content-authoring`,
  `composer-view`, `moderation-resolution`), and none relaxes or extends a
  requirement. Ruling 4's *case 2* half (placeholder permitted, bounded by
  two named requirements) is likewise correctly scope-shaped. It is
  specifically case 1's MUST phrasing (finding above) that reads as
  requirement-grade language landing outside `openspec/specs/`.
