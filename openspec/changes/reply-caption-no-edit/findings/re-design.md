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

- [x] **`dev-writer`** — `design.md` Decision 4 (and Decision 1) — whose call the
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

      **Outcome (dev-writer): fixed** in "Record whose call each rule beyond
      the owner's decision is, and what Decisions 1 and 4 cost (#177)". New
      Decision 5 lists the four rules as the `spec-writer`'s calls, none
      confirmed by the owner, each trimmable by a spec change. Decision 1 now
      says that is all the owner decided and points there. Decision 4's "It
      was not a request to replace the text with its negation" is replaced:
      the owner's words say nothing about a denial, and forbidding one is the
      spec-writer's call. One correction to this entry, from the spec's
      history (`git log -- openspec/changes/reply-caption-no-edit/specs/thread-view/spec.md`,
      three commits): not all four came from the spec-test review. The
      shut-gate reach was in the spec's first draft. The earlier-version ban
      was the spec-writer's second pass, before any review, after design.md's
      first draft called the bundle's clause uncontracted. Only the denial ban
      and the signed requirement answered the review. Decision 5 records it
      that way. The record does not show the spec wrong. It shows four rules
      the owner has not confirmed, and ruling on them is the owner's. Prose
      only, so no test.

- [x] **`dev-writer`** — `design.md` Decisions 1 and 4 — what they cost is not
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

      **Outcome (dev-writer): fixed**, same commit. Decision 1's
      remove-the-caption alternative ends with "What that costs": that reading
      now needs a spec change, not a deletion, as the price of making "describes
      what a reply is" a contract, and the owner may drop the requirement.
      Decision 4 has "What it costs": the screen can never answer a user
      looking for an edit control, and "Replies cannot be edited in this
      version" is forbidden. That is the price of the staleness argument. The
      owner may overrule it, and the spec then needs a permitted-denial
      scenario and the test needs a denial-aware matcher.

- [x] **`dev-writer`** — `design.md` Goals and Risks, third bullet — the
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

      **Outcome (dev-writer): fixed**, same commit, as two bullets of the new
      Decision 5, each with its reason and the leave-it-out alternative. The
      version ban: the bundle carried the clause beside the edit promise, the
      issue's evidence names it as dropped for promising an absent facility,
      and nothing reads a prior version. Left out, the bundle's clause
      restored verbatim would pass every test. The shut gate: no reason was
      written down when the first draft added it, and the bullet says so. The
      reason it gives is mine. The shut group fills the composer's slot, and
      the two groups are one expression against its complement, so a rule on
      one branch lets the promise move to the other with every test green.
      Covering the gate costs nothing today. The Risks third bullet is
      removed: its reason is now in Decision 5, and the pin on the bundle's
      clause is in Decision 3's guards. Goals cite Decisions 4 and 5.

- [x] **`dev-writer`** — `design.md` Decision 3 and Risks, second bullet — the
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

      **Outcome (dev-writer): fixed by recording it**, same commit. The walk
      is not narrowed, because that is a test edit and the brief forbids one.
      Decision 3 has a new paragraph, "What it does reach is wider than the
      requirement, deliberately". It names what the column walk binds that
      the spec does not: the header, the failed-read state, the no-replies and
      more-replies notices, and later additions. It also says both walks reach
      `DComposer`'s and `DPublishOutcome`'s `kind`-built copy, which the post
      composer shares. It names who that constrains, why the walk is wide, and
      how to narrow it: skip the column's children before the composer's
      double rule as well as the rows. "Rendered with the composer in the
      requirement's sense" is gone from design.md. The Risks bullet on future
      permitted sentences no longer speaks only of "this group". Established
      by reading, not by mutation: `textsBesideTheRows` skips only the
      Repeater and its delegates, and the column's other children are
      `DThreadScreen.qml`'s header `RowLayout` (:311), failed-read `Rectangle`
      (:372), two notices (:625, :639), double rule (:653) and the two groups
      (:668, :753). Not mine to fix: the test comment at
      `tst_thread_reply.qml:697-699` still says a sibling Text "is rendered
      with the reply composer in the requirement's sense". It is the tester's
      file, so this is reported to the runner.

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

- [x] **`dev-writer`** — `design.md` Decision 3, paragraph beginning "The suite
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

      **Outcome (dev-writer): fixed, both halves**, same commit, from tracing
      the paths. The keystore wording reaches both places. It is the probe's
      reason verbatim (`wire.rs`, `get_capabilities_from_stores`:
      `open_keystore().map_err(|e| e.to_string())`). It is also embedded in a
      refused publish: the adapter's `publishing` (`lib.rs:853-855`) answers a
      keystore that will not open with `core::no_identity(&e.to_string())`,
      which is `Refusal::NoIdentity`'s "no identity is available to sign with:
      {why}; ..." (`authoring.rs:150-153`). Decision 3 now cites that path.
      The identity-store wording reaches no thread-screen path. Neither
      `publishing` nor `get_capabilities` opens the identity record (the
      comments at `lib.rs:865-870` and `:980-981` say so), and the adapter's
      only opener of it is called from `keep_identity` (`lib.rs:1135`).
      Decision 3 now says the test drives the keystore wording verbatim as the
      shut gate's reason, and that its refusal is a hypothetical paraphrase of
      the identity store's error that stands for any core message saying
      "version". Not mine to fix: the test comment at
      `tst_thread_reply.qml:796-798` says "both are real core wordings", the
      same overclaim. It is the tester's file, so this is reported to the
      runner.
