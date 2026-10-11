# Design re-review: draft-belongs-to-target

Range: `fb554d3d..HEAD` for `design.md`, `dialectica-ui/src` and `specs/`. Round
one's four design outcomes that say `design.md` gained something (Decision 2
evidence, Decision 5, Decision 6, the Risks entry for `heldDrafts`) are all true
of `design.md`. The code changes (`targetKeyOf` inlined, `publishedKey` rename,
`applyReply`'s default key) match what `design.md` now says; the default key is
recorded with its reason and its test exists
(`tst_composer.qml:445`). Two things are wrong or missing.

- [x] **`dev-writer`** — `design.md` carries nothing of the failed-read
      reasoning the spec was corrected on, and line 69 now states the opposite.
      `findings/spec-test.md` records a measurement: the feed's post composer is
      still rendered through a failed list read (`FeedScreen.qml` gates it on the
      posting probe alone), while the thread screen stops rendering its reply
      composer. The spec therefore says a failed read's effect on rendering "is
      not decided here" and requires only that the text is displayed nowhere but
      in a rendered composer's field. Reasoning never goes in a spec, and
      `design.md` has no entry for why the two screens differ, why the spec
      leaves it open, or why the assertion is "text only in rendered fields".
      Worse, Decision 1's alternative "One composer instance per target"
      (`design.md:69`) still says "a draft behind a shut gate or a failed read
      must outlive its composer not being rendered", and line 87 covers only the
      shut gate. For the feed that premise is false for a failed read, and the
      sentence will be read as a recorded fact. Add to Decision 1 (or a
      decision of its own): the measured asymmetry, that the spec deliberately
      does not decide it, what that costs (a future change that makes the feed
      stop rendering on a failure is allowed, and the draft still must not show),
      and the mutants the tester ran (the failure banner carrying the draft text
      is caught; keeping the thread composer mounted is not, by design). Correct
      line 69 to say a shut gate, and the thread screen's failed read.
      **Verified:** `git grep -n -i "failed" design.md` returns line 69 only.

      **Fixed** (`dev-writer`), in the commit that ticks this box. `design.md`
      gains Decision 7, *Through a failed read the draft's text is bound, and
      whether a composer is rendered is not*: the asymmetry and where each
      screen's gate is, that this change picked neither answer, why the spec
      leaves it open (the owner's answers settle a shut gate only), why the
      text is what is bound, and what leaving it open costs. Decision 1's
      "One composer instance per target" now says a shut gate, or the thread
      screen's failed read, and the paragraph on the owner's remaining answers
      points at Decision 7. `tasks.md` 3.1 said the same stale thing ("its
      screen's gate stops rendering it", with the failed-read test listed
      under it) and is corrected with it.
      **Measured on this tree**, each edit run with
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_draft_targets.qml`
      (baseline 41 passed) and reverted with `git checkout --`:
      - thread failure message `screen.failure + " " + replyComposer.draft`:
        1 failed, `test_a_draft_is_back_when_a_failed_read_recovers`, "reply:
        ... the 0 composer field(s) rendered and nowhere else", actual 1,
        expected 0;
      - the feed's core-failure `Text`, `screen.failure + " " + composer.draft`:
        1 failed, the same test, "post: ... the 1 composer field(s) rendered",
        actual 2, expected 1. These two messages are also where the 1 post
        field and 0 reply fields in Decision 7 come from;
      - reply composer `visible:` without `readState === "ok"`: 41 passed;
      - post composer `visible:` with `readState === "ok"` added: 41 passed.
        The reviewer's entry names only the first of these two; the second is
        the "future change that makes the feed stop rendering" it describes,
        and was run so the design does not state it unmeasured.
      Not measured: the last two edits against any other spec file. Decision 7
      says so.

- [x] **`dev-writer`** — `design.md` Decision 2, the `":"` bullet, is stale
      against the test. It says the test "sweeps a hand-written list of
      separators, so a join on a character outside the list passes it". The
      tester's outcome in `findings/spec-test.md` replaced that list with a
      sweep of every UTF-16 code unit and the empty separator
      (`tst_draft_targets.qml:1078-1091`), and measured that a `\u0001` join and
      a U+FFFD join both fail. The design's stated gap no longer exists, and the
      "what would pin it" tone now misleads a reader into thinking the
      injectivity choice is under-tested. State the sweep, that it costs about
      eight seconds of the file's runtime, and keep the one still-true limit:
      only a join is tested, not another non-injective encoding.

      **Fixed** (`dev-writer`), in the commit that ticks this box. The bullet
      now states the code-unit sweep, its cost and the remaining limit.
      **Measured on this tree**, `targetKey` edited and
      `tst_draft_targets.qml`, `tst_composer.qml` and
      `tst_publish_outcome_visits.qml` run, then `git checkout --`:
      - `.join(":")`: 1 failed, the sweep test, `separator U+3a`; the other
        two files green;
      - `.join("")`, a separator of my choosing and not the reviewer's:
        1 failed, `separator U+e000` (`tst_draft_targets.qml` only);
      - `.join("")`: 1 failed, `no separator` (`tst_draft_targets.qml` only);
      - `.join("😀")`, U+1F600, two code units: **all three files
        green**. That is the still-true limit, and it is narrower than "only a
        join is tested": a join longer than one code unit is not tested
        either. `rereview-spec-test.md` has an open `tester` entry on the same
        thing; `design.md` records the measurement and does not depend on how
        that entry is answered.
      The cost: `tst_draft_targets.qml` totals 13997 ms at baseline and 5842 ms
      when the sweep stops at its first pair (the `""` join), so about 8.2 s
      is the sweep. `design.md` says "about eight of its fourteen seconds".
