# Findings — correctness and security

Reviewed: `dialectica-ui/src/qml/OnboardingScreen.qml`, `Core.qml`, `Main.qml`,
and the two new spec files. Dimensions covered: **correctness** and **security**.
Readability and architecture are another instance's.

Baseline before any mutation: 91 QML tests across 6 spec files, all passing
(`dialectica-ui/tests/run-qml-tests.sh`, qmltestrunner 6.10.3).

## Defects

- [ ] **`dev-writer`** — `OnboardingScreen.qml:306` — `-1` is both the
      "nothing selected" sentinel and a value a reply's `index` field can take,
      so a candidate carrying `index:-1` renders as SELECTED while nothing is
      selected
      **Scenario:** a slate reply of
      `{"slate":"s1","count":2,"candidates":[{"index":-1,…},{"index":0,…}]}`.
      After `requestSlate()`, `selectedIndex` is `-1` (correct — nothing
      selected), but `row.chosen` is `screen.selectedIndex === row.modelData.index`,
      which is `-1 === -1` for the first row. That row draws the chosen
      background, the chosen border **and** the visible `SELECTED` word. The user
      sees a candidate presented as already chosen and presses "Keep this
      identity"; `keepSelected()`'s `selectedIndex < 0` guard returns silently,
      so the button is dead and no message appears. Clicking that row calls
      `screen.select(-1)`, which leaves `selectedIndex` at `-1` — the row stays
      marked and the button stays dead, with no way for the user to tell why.
      This is precisely what the spec's "Nothing is selected when a set arrives"
      and "A pre-selected candidate is a choice made for the user" forbid, at the
      level an observer of the screen can actually see.
      **Measured:** with a normal slate the probe counts 0 visible `SELECTED`
      markers; with one `index:-1` candidate it counts 1 while `selectedIndex`
      is `-1`. After `select(0)` picks the *other* row, the count is still 1 —
      the marker moved rather than two showing, so the row that reads as chosen
      is not the row that is.
      **Severity: high.** Core as built emits `index` as a `usize` in
      `0..SLATE_SIZE` (`wire.rs:656`, asserted at `wire.rs:3075`), so this is not
      reachable through core today. It is reachable through the bridge, which is
      where the view's own tests supply replies, and CLAUDE.md's standing rule is
      that the view validates what it is handed rather than trusting the sender.
      The fix is to stop overloading `-1`: hold selection as a separate
      "something is selected" fact, or refuse a candidate whose `index` is not a
      non-negative integer at the point `candidates` is assigned.

- [ ] **`tester`** — `tst_onboarding_states.qml` — the `selectedIndex < 0` guard
      in `keepSelected()` is untested; removing it leaves the whole suite green
      **Scenario:** replace `if (screen.selectedIndex < 0) return` with
      `if (false) return` at `OnboardingScreen.qml:146`. Every candidate in every
      fixture has a non-negative `index`, so `candidateAt(-1)` finds nothing and
      the *second* guard (`candidate === null`) catches the call. The first guard
      — which `design.md` calls "the guard that actually guards" and the only
      load-bearing mechanism, since `FlatButton` emits `clicked()` whatever it
      looks like — is therefore dead in every fixture.
      **Measured: 38 of 38 tests pass under this mutation.** The author's
      mutation list does not include it.
      A test that pins the guard needs a fixture where `candidateAt()` would
      succeed at the sentinel — i.e. a candidate carrying `index:-1` — which is
      the same fixture the defect above needs.
      **Severity: medium** as a test gap; it is what let the defect above ship.

- [ ] **`dev-writer`** — `OnboardingScreen.qml:302-360` — a candidate that is
      not an object throws in the delegate and still reaches the slate phase
      **Scenario:** a slate reply of
      `{"slate":"s1","count":3,"candidates":[null,"str",{"index":0}]}`. The
      `Array.isArray` check at line 104 passes, so `phase` becomes `"slate"` and
      three rows are built. `row.modelData.address` then throws
      `TypeError: Value is null and could not be converted to an object` at
      lines 306, 321 and 337, and `Unable to assign [undefined] to QString` at
      321 and 337. The screen renders three rows, one of them blank with no
      address and no mark, and the user is invited to choose between them.
      **Measured:** the probe reports `phase=slate n=3` with six QWARN lines from
      the delegate. No test covers it.
      **Severity: medium.** The spec requires that "a reply this screen cannot
      read is a failure rather than a slate with nothing in it"; an array whose
      *elements* are unreadable is the same confusion one level down. The
      existing check establishes that `candidates` is a list, not that its
      entries are candidates.

- [ ] **`dev-writer`** — `OnboardingScreen.qml:164-166` — a non-string `reason`
      renders as `[object Object]` on the refused screen
      **Scenario:** `keep_identity` answers `{"kept":false,"reason":{"code":7}}`.
      `reply.value.reason !== undefined` is true, so `String(reply.value.reason)`
      yields the literal text `[object Object]`, which is what the screen shows
      the user in place of a reason. The spec says a refusal's "reason SHALL be
      shown as the module wrote it, since a module's reasons are written to name
      a fix" — `[object Object]` names no fix and is strictly worse than the
      screen's own fallback string two lines below, which exists for exactly the
      case where no usable reason arrived.
      **Measured:** probe reports `phase=refused refusal=[object Object]`.
      **Severity: low-medium.** The same `String()` coercion at line 114
      (`slateId`) and in `Main.qml:110` (`identityReason`) has the same shape;
      the fix is to treat a non-string as absent rather than to stringify it.

- [ ] **`dev-writer`** — `OnboardingScreen.qml:113-118` — `requestSlate()`'s
      success path clears `refusal` and `failure` but not `keptIdentity`
      **Scenario:** keep a candidate (`phase` becomes `"kept"`, `keptIdentity`
      holds an address), then press "Try again" or otherwise call
      `requestSlate()`. `phase` returns to `"slate"` while `keptIdentity` still
      holds the kept identity. `enterFailed()` at line 129 clears it; the success
      path does not, so the two exits from a state disagree about what they
      clean up.
      **Measured:** probe reports `phase=slate keptIdentity=HELD` after a
      successful keep followed by `requestSlate()`.
      **Severity: low.** Not visible today, because every render site is gated on
      `phase === "kept"` and `Main.qml` branches on `phase` rather than on
      `keptIdentity`. It is a latent inconsistency in the state machine the spec
      asks to be single-valued, and the next reader of `keptIdentity` inherits a
      stale value for free.

- [ ] **`spec-writer`** — `OnboardingScreen.qml:627` and
      `tst_onboarding_states.qml:647` — the shipped copy asserts "the same four
      words" and a test pins that count, but the owner has since settled
      **three**
      **Scenario:** the margin note reads "Someone else in this Stoa may hold the
      same four words", and `test_the_uniqueness_note_says_four_words_not_three`
      asserts `"same four words"` is present and `"three words"` is absent. The
      owner's decision on `docs/name-shape-sweep` (`d3e7579`, 2026-09-13) reads:
      *"the generated name is three words in the shape adjective + noun + 'of' +
      place … The merged four-word design is superseded."* So the screen will
      ship a wrong count, and the test actively **forbids** the correct one —
      when the name change lands, that assertion fails and a fixer reading it
      will see a pin that says the right answer is wrong.
      **Note the history:** this is the third position on the word count (three,
      then four, now three on a different basis), which is exactly why the copy
      should not assert a count at all. The sentence's obligation — that names
      are not unique, are not identifiers, and the address is what distinguishes
      — survives without one. The spec's own scenario
      ("The uniqueness note is shown with the candidates") says only "the same
      words", not a number; the count came in through the bundle.
      **Severity: medium.** It is user-facing copy that will be wrong on the day
      names ship, and a test written to keep it wrong. The durable fix is to
      state no count in either the copy or the assertion — a pin on a number
      fails on reword rather than on misinformation.

## What was clean

The four traps the author identified are, with the one exception above, actually
handled in the code rather than only in the comments.

**The refused-keep third state is correct.** `keepSelected()` branches on
`reply.value.kept !== true` after `reply.ok`, and a refusal sets `phase =
"refused"` while leaving `candidates` and `slateId` untouched. I re-ran the
author's first mutation (`if (reply.value.kept !== true)` → `if (false)`): **4 of
38 tests fail**, matching the claim. `who_am_i`'s `{"hasIdentity":false}` is
handled the same way in `Main.qml:95-114`, where anything that is not an explicit
`true` routes to onboarding — the safe direction.

**Nothing is pre-selected, and the guard is in the handler.** I re-ran the
author's second mutation (`selectedIndex = -1` → `= 0` in `requestSlate`):
**3 of 38 tests fail**, matching the claim. The `opacity`/`kind` bindings on the
keep button are appearance only; `keepSelected()` returns before touching the
bridge. That guard is under-tested (above) but it is in the right place.

**Addresses are full and no elision is hand-rolled.** Both `AddressLabel` uses on
this screen pass `full: true`, and `AddressLabel.qml` is the only place
`abbreviate()` exists. No `substr`, `slice` or ellipsis appears in
`OnboardingScreen.qml`.

**Both absent cases route to onboarding with the reason unparsed.** `Main.qml`
contains no `indexOf`, no substring test and no classification of core's wording;
`identityReason` is carried verbatim. `tst_launch_branch.qml:108` proves the two
stay distinguishable.

**Security: nothing sensitive is rendered or logged.** There is no `console.log`
anywhere in `dialectica-ui/src/qml/`. `keptIdentity` holds `publicKey` and
`path`, but neither reaches a `Text` — only `address` is rendered, through
`AddressLabel`. `path` is never shown (which is also what keeps the no-name
requirement true). No path produces a placeholder identity: `keptIdentity` is
`null` until a `kept:true` reply carrying a non-empty string address arrives,
and `enterFailed()` nulls it. A `kept:true` with no address is the failed state,
not an empty one — verified by the shipped test and by reading line 176.

**Copy: neither false claim ships.** The unlinkability clause is absent and
nothing replaces it; the sweep test blocks five phrasings of it. (The word-count
claim is the separate finding above.) The word "username" appears nowhere.

**Every `Text` is explicitly `PlainText`** — 15 `Text {` opens, 15 `textFormat:`
assignments, so the CI gate at `.github/workflows/ci.yml:266` passes. The
tree-walking sweep in the tests covers elements added later without editing the
test, which avoids the hand-maintained-list failure mode.

**The singleton-bridge defect is NOT present in this piece.** `Core` is a
singleton and the hazard is real — I built two screens before driving either and
both ran against the last installed bridge (`tags=BB`). But every shipped test
drives each component immediately after installing its bridge, and `Main`'s
`who_am_i` fires during `createObject`, so the two-`Main` test in
`tst_launch_branch.qml:108` correctly shows `tags=AAABBB` with each object's
calls going to its own bridge. The one onboarding test holding two screens alive
(`test_the_two_encryption_replies_produce_different_text`) drives each before
building the next and reports `enc=true plain=false`. No test reports two
distinct outcomes as indistinguishable.

**CI would pass.** `qmlformat` parses `OnboardingScreen.qml` cleanly (exit 0).
`qmllint` emits two new `[unqualified]` warnings at lines 306 and 358 (the
delegate reading the outer `screen` id), but the CI job passes `--unqualified
disable` and gates on exit code, which is 0. The `Text`/`textFormat` count gate
balances. No file was moved or renamed, so no gate is measuring a directory that
no longer holds tests.

## On the author's least-confident test

`test_no_row_presents_a_derivation_path_or_an_index_as_a_name` is a blocklist
over `"7"`, `"0"`, `"1"`, `"#1"` and the public key, and the author is right to
be unsure of it — it cannot catch a name-shaped value nobody anticipated. A
structural assertion is available and is strictly stronger, and I have written it
as a finding for the `tester` rather than leaving it as a remark:

- [ ] **`tester`** — `tst_onboarding_states.qml:719` — replace the blocklist with
      an assertion on the row's structure
      **Scenario:** the current test enumerates forbidden strings, so a row that
      began showing, say, the candidate's `path` formatted as `m/44'/0'/7'`, or a
      truncated address, or a position rendered as `"one"`, passes it untouched.
      The property the spec actually states is structural: *"The row SHALL leave
      the name unshown rather than substituted"*, and the row is built so that
      the only text it contains is the address. So assert that — walk the
      delegate and require that the **set** of non-empty strings in a row is
      exactly `{the candidate's full address, and "SELECTED" when chosen}`. Any
      value added in a name's position then fails, whether or not anyone
      anticipated its spelling; and the test keeps working when the generated
      name lands, because at that point the expected set gains one member
      deliberately rather than the blocklist silently admitting it.
      **Measured:** I added, above the `AddressLabel` in the row's
      `ColumnLayout`, a `Text` whose content is
      `"Key " + String.fromCharCode(65 + (row.modelData.path % 26))` — a
      name-shaped label derived from the derivation path, rendering as `"Key H"`
      for the test's `path:7` fixture. **38 of 38 tests pass under this
      mutation**, including
      `test_no_row_presents_a_derivation_path_or_an_index_as_a_name` itself:
      `"Key H"` is not `"7"`, `"0"`, `"1"`, `"#1"` or the public key. A
      set-equality assertion on the row's strings fails it.
      For contrast, the blunter mutation `Text { text: String(row.modelData.path) }`
      **is** caught (1 of 38 fails), because the fixture's `path` is `7` and
      `"7"` happens to be on the list — which is the blocklist working by
      coincidence of the fixture rather than by covering the property.
      **Severity: medium** as a test-strength finding; no defect in the shipped
      row today.

## What I could not check

The tests assert on properties and the object tree, never pixels, so nothing here
verifies that the mark is not *presented* as a badge, that the phases are
visually distinct, or that the layout holds — including whether the row leaves
room for a name to arrive above the address without moving, which `design.md` and
`UI-BRIEF.md` both promise. Those remain unverified by any gate.

I did not run the module end to end against real core; every reply in this review
came from a fake bridge, which is the same surface the shipped tests use.
