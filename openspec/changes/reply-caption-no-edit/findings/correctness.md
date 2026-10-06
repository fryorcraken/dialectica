# Correctness review: reply-caption-no-edit

Dimension reviewed: **correctness** only. Read-only after the arrival check:
the permission classifier refused two attempts to mutate `DThreadScreen.qml`
(a revised caption to see the open-composer tests fail), and then refused the
whole-suite QML run, so **no mutation was measured** and the matcher analysis
below is by reading the regex against hand-written strings. The one run that was
allowed, `tst_thread_reply.qml` on the tree as committed, passed 27 of 27.

- [x] **`dev-writer` / `tester`** — `dialectica-ui/tests/tst_thread_reply.qml:460-461` —
      the matcher's justifying comment is false for the shut gate, and the matcher
      would flag copy the spec permits there.
      **Scenario:** the comment says "Nothing this group renders has any reason to
      say 'version'". The shut gate renders `screen.capability.reason` verbatim, and
      that is core's own `Display` text. Reachable reasons include
      `keystore format version {v} is newer than this build understands; upgrade
      dialectica` (`keystore.rs:619`) and `the identity record declares layout
      version {found} ...` (`identity_store.rs:135`). `claimsEditing` flags both
      through `\bversions?\b`. Neither claims a reply can be edited or an earlier
      version read, so the spec is not violated. The fixture only ever supplies
      `"no keystore"`, so the suite is green. A future shut-gate test that drives a
      real reason string, or a fixture reason reworded to mention a version, fails
      for a reason the requirement does not hold.
      **Severity:** low, a test-fragility defect and not a product one. The shipped
      screen is correct. Fix by narrowing the claim in the comment, or by scoping
      the `version` clause to a claim shape (for example a version of *a reply*),
      and adding a real keystore reason to the matcher's "leaves alone" table.
      **Measured:** not mutated. The regex was read against the two strings above.
      **`dev-writer` half, fixed** in the commit "Record why the reply caption may not
      deny editing either, and drop the caption comment's history (#177)":
      `design.md` Decision 3 no longer says nothing the group renders has a reason to
      say "version". It now says that holds for text the screen authors, names the
      core-supplied text and the draft the walk also collects, quotes both strings
      above (re-found at `keystore.rs:621` and `identity_store.rs:137`), and says the
      fixtures cannot trip on them today because every core string they supply is
      hand-written. The spec has since put core-supplied text outside the requirement
      (`findings/spec-test.md`, third entry), so this finding's "the spec is not
      violated" is now contracted. **Box left open for the `tester`**: the comment at
      `tst_thread_reply.qml:460-461` is in a test file this pass may not edit, and
      whether to exclude core text from the walk or scope the `version` clause, plus
      the keystore row for the "leaves alone" table, are the test half.
      **Outcome (tester, the test half): fixed, by exclusion.** The walks now take
      the strings a test fed the screen as core's reply or as the draft, and
      collect them without flagging them (`mentionsAmong(texts, suppliedText)`).
      The exemption is exact strings the test chose, so text the screen authors
      is never exempt. `test_text_core_supplies_and_the_draft_are_outside_the_requirement`
      drives the two real wordings quoted above (`keystore format version 9 is
      newer than this build understands; upgrade dialectica`, and `the identity
      record declares layout version 2, ...`) as a shut gate's reason and as a
      refused publish's message, plus a draft saying "I will edit this and
      publish a newer version later". It asserts the walk collects each, that the
      matcher flags each when unexempted (so the scope and not a blind matcher
      spares them), and that the exempted walk is clean for the group and beside
      the rows. The false comment ("Nothing this group renders has any reason to
      say 'version'") is rewritten to "Nothing the screen authors in these
      groups". Measured: ignoring `suppliedText` turns that test red on core's
      reason; exempting only the refusal turns it red on the draft; with the
      matcher mutated to `return false` it goes red on "and the matcher does flag
      it". The keystore row is in this test and not in the "leaves alone" table,
      because that table lists what the group's own copy renders and the reason
      is not the group's copy. Predicted each; observed each.

Areas that were clean, in prose:

- **The implementation change** (`DThreadScreen.qml`, one `Text` literal and an
  `objectName`) is correct, and nothing else in the diff changes behaviour. No
  other rendered `text:` or `placeholderText:` in `dialectica-ui/src` or the
  end-to-end specs under `dialectica-ui/tests/ui` carries an edit or version
  claim. The only hits are `PostHeader.qml`'s `edited` marker and the
  earlier-versions label on the thread rows, both of which the spec excludes.
- **No stale reference to the removed sentence** exists outside the change folder
  and the matcher's own hand-written table.
- **Every string `DComposer.qml` and `DPublishOutcome.qml` renders** was read
  against `claimsEditing`. None matches, so the "leaves alone" table is a faithful
  copy of what the group renders and the open-gate and after-publish walks are not
  failing on copy. The grep hits in `DComposer.qml` are all in comments.
- **Walk coverage**: `stringsUnder` reads `.text` through `children`. It reaches
  the composer's `TextEdit` draft and the child `Text`s of the button and the
  outcome. It cannot reach popup or tooltip text, which the file's own comment
  concedes, and `DComposer` and `DFlatButton` contain no `ToolTip`, `Accessible` or
  placeholder.
- **Matcher shape**: the alternation precedence in
  `\b(edit|...)\w*|\bversions?\b` is correct, and every prefix stem is anchored
  at a word boundary, so `exchange` and `unrevised` do not match.
- **Non-vacuity** of the open, after-publish and shut-gate walks is asserted
  before the no-claim assertion, and the whole-screen scope test pins why the
  walk is scoped.
