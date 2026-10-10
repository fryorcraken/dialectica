# Design re-review: draft-belongs-to-target

Range: `fb554d3d..HEAD` for `design.md`, `dialectica-ui/src` and `specs/`. Round
one's four design outcomes that say `design.md` gained something (Decision 2
evidence, Decision 5, Decision 6, the Risks entry for `heldDrafts`) are all true
of `design.md`. The code changes (`targetKeyOf` inlined, `publishedKey` rename,
`applyReply`'s default key) match what `design.md` now says; the default key is
recorded with its reason and its test exists
(`tst_composer.qml:445`). Two things are wrong or missing.

- [ ] **`dev-writer`** — `design.md` carries nothing of the failed-read
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

- [ ] **`dev-writer`** — `design.md` Decision 2, the `":"` bullet, is stale
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
