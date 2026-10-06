# Re-review (design): reply-caption-no-edit

Read against the tree at `3a60de0f`, with `git diff 5cd2c8af...HEAD`. Issue #177
re-read fresh, with its one comment.

**What holds.** No code contradicts a recorded decision. The caption is `"A reply
is a signed record."` with `objectName: "replyCaption"` inside
`replyComposerOpen`; no editing was built. Decision 1's grep (`revise` over
`lib.rs`, `Core.qml`, `content-authoring`) returns nothing, and the widened
`revis|edit` over the first two files finds only the comments the entry describes
(`Core.qml:143,159-161,462`, `lib.rs:134`). The quoted core wordings exist
(`keystore.rs:621`, `identity_store.rs:137`), and the probe handler's doc at
`wire.rs:182-196` says what Decision 3 says it does. Every helper the entry names
exists in `tst_thread_reply.qml` (31 `function test_`; the script reports 33
passed, 0 failed with `init`/`cleanup`). The matcher's stems match the list
Decision 3 gives, and its "five tests" are the five it names. By reading,
Decision 3's "Measured" red-sets for `return false`, for the `edit` stem removed,
for the old caption, for the caption moved out of the group and for a sibling
Text are consistent with the test bodies.

**What I could not re-run.** I tried to run the cheap mutations (`return false`
matcher) and the auto-mode classifier refused the edit to the test file. I did
not work around it. The red-sets in Decision 3 are therefore checked by reading
the assertions, not by execution this round.

The issue: its two options were "drop the sentence to what is true today" or
build editing; the owner's comment picks the first, and the code honours it. The
findings below are gaps in the recorded reasoning, none a code defect.

- [ ] **`dev-writer`** — `design.md` Decision 4 (and Decision 1) — whose call the
      scope growth was is not recorded (gap). The owner's decision is "drop the
      text; build no editing", about a promise that a reply can be edited. The
      spec now also forbids a *denial*, forbids any statement about reading an
      earlier version, covers the shut gate, and adds a positive requirement that
      the open composer states a reply is signed. Those came from the
      `spec-writer`'s outcomes to the spec-test review (`findings/spec-test.md`,
      the denial, `NO SPEC:` and shut-gate entries), not from the owner, and
      `findings/spec-test.md`'s own "Clean areas" says the version-reading and
      shut-gate clauses "go beyond what the owner decided". `design.md` says none
      of this. Decision 4 ends "It was not a request to replace the text with its
      negation", which reads as the owner backing the denial ban. The owner said
      nothing about denial. `findings/` is deleted before merge, so after this
      change lands the provenance exists nowhere. Add to Decision 4 and Decision 1:
      the owner decided the edit promise goes and nothing is built; the ban on
      denials, the signed-statement requirement and the version and shut-gate
      reach were the `spec-writer`'s call, argued as below, and not confirmed by
      the owner. **Verified:** `git grep -n -i owner openspec/changes/reply-caption-no-edit/design.md`
      returns six lines (45, 47, 62, 65, 334, 344): the quoted comment, the owner
      ruling out editing, the owner choosing between the issue's two options, and
      the owner's "drop the text". None attributes the denial ban, the signed
      requirement or the version and shut-gate reach to anyone.

- [ ] **`dev-writer`** — `design.md` Decisions 1 and 4 — what they cost is not
      recorded (gap). Decision 4 names one alternative (permit an honest denial)
      and its staleness argument, but not what the rule forecloses: the screen can
      never tell a user who looks for an edit control that editing is not
      available, so the user gets no answer at all. The spec-test review's
      scenario was exactly an owner wanting "Replies cannot be edited in this
      version". Decision 1's signed-statement requirement forecloses the other
      reading of "drop the text", removing the caption, which Decision 1 rejects as
      an interpretation of the owner's words. With the requirement in place that
      reading now needs a spec change, not a deletion. Say that, and that the owner
      may overrule either: the first cost is the price of the staleness argument,
      the second the price of making the issue's "describes what a reply is" a
      contract.

- [ ] **`dev-writer`** — `design.md` Goals and Risks, third bullet — the
      version-reading ban and the shut-gate coverage have no Decision (gap). Both
      go beyond the issue, which names only "can be edited later". The version
      ban's reason lives in `proposal.md` (the bundle's caption carried it) and in
      one Risks bullet; the issue's evidence paragraph does name that clause,
      which supports it. The shut-gate coverage has no reason in `design.md` or
      `proposal.md` at all: the gate renders core's reason and a sentence about
      the absent composer, and nothing records why a requirement about a reply
      caption reaches it. Add a Decision, or fold one sentence each into Decision
      4, naming the reason and the alternative (leave both out, as the owner's
      words alone would).

- [ ] **`dev-writer`** — `design.md` Decision 3 and Risks, second bullet — the
      guard is wider than the requirement it enforces, and that is not said (gap).
      The spec binds text "the thread screen authors and renders **with the reply
      composer**" and, behind a shut gate, "in the composer's place". The
      beside-the-rows walk (`textsBesideTheRows`) fails on a topic word in *any*
      text in the screen's content column outside the rows: the header, the
      notices, anything. It also walks text authored by `DComposer` and
      `DPublishOutcome`, shared components that build their copy from `kind`, so
      the post composer's copy is in the same strings. Decision 3 calls a sibling
      Text "rendered with the composer in the requirement's sense", which is
      generous. The Risks bullet that admits a permitted sentence could trip the
      matcher speaks only of "this group". Record that the screen-wide walk is
      deliberately stricter than the spec, who it constrains (the next author of
      any copy on this screen, and of the shared composer's strings) and how to
      narrow it, or narrow the walk to the composer's place.

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_thread_reply.qml:472` — a
      pointer into the change's own folder, the shape Decision 2 argues against
      (suggestion). "design.md Decision 3 covers the alternatives, and what breaks
      without each guard below." After archive that file is
      `openspec/changes/archive/<date>-reply-caption-no-edit/design.md`, and the
      comment names no change. Decision 2 records why the caption's comment cites
      the capability rather than this change; the test header was left citing the
      change's document, and the Decision 3 guard list it points at is the
      mutation evidence the next toucher of this file needs. Either name the
      change in the comment so it greps, or restate in the header the one thing
      the pointer carries (a guard list lives in the archive). Other files in the
      tree cite a bare `design.md`, so this is the repository's habit, but this
      change has already decided against it once.

      **Outcome (tester): fixed.** The comment now names the change and the
      archive path it will have: "The alternatives, and what breaks without
      each guard below, are Decision 3 of this change's design.md, which lands
      at `openspec/changes/archive/<date>-reply-caption-no-edit/design.md`."
      It names the change (so it greps) and says where the guard list lives
      once archived. Took the entry although it names the dev-writer, as the
      brief directs: it is an edit to a test-file comment.

- [ ] **`dev-writer`** — `design.md` Decision 3, paragraph beginning "The suite
      excludes them" — a quoted fixture is not the quoted wording (suggestion).
      The entry says the test "drives the two keystore and identity-store wordings
      above". The keystore fixture matches core's text up to the first clause.
      The identity fixture is "the identity record declares layout version 2, which
      this build cannot read", while core's is "the identity record declares layout
      version {found} and this build understands {expected}; upgrade dialectica"
      (`identity_store.rs:137-138`). It is a paraphrase carrying "layout version",
      and the matcher flags either. Say "a paraphrase of" so the entry does not
      claim a verbatim core wording the test does not carry. The entry also says
      core's own error texts can reach "a refused publish's message" and gives the
      keystore and identity strings as the evidence. I confirmed the strings exist
      and that the probe returns a keystore failure as a reason. I did not trace
      either to a `publish_reply` refusal, and the entry cites no path from one to
      the other. Cite the path, or say the refusal fixture is hypothetical.
