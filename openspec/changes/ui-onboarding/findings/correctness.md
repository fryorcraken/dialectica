# Findings — correctness and security

Reviewed: `dialectica-ui/src/qml/OnboardingScreen.qml`, `Core.qml`, `Main.qml`,
and the two new spec files. Dimensions covered: **correctness** and **security**.
Readability and architecture are another instance's.

Baseline before any mutation: 91 QML tests across 6 spec files, all passing
(`dialectica-ui/tests/run-qml-tests.sh`, qmltestrunner 6.10.3).

## Defects

- [x] **`dev-writer`** — `OnboardingScreen.qml:306` — `-1` is both the
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
      **Fixed** in `1d91627`, by the second route you name — refusal at the
      point `candidates` is assigned. `isCandidate()` runs over every entry
      before anything is stored and requires an object carrying a non-negative
      integer `index` and a non-empty string `address`; a slate containing one
      that fails is the failed state. The sentinel is now a named
      `nothingSelected` rather than a literal at six sites, so the two are
      readable as different things wherever they are compared.
      Three tests fail without it. `test_no_row_reads_as_chosen_while_nothing_is_selected`
      asserts your measurement directly — 0 visible SELECTED markers on a normal
      slate, 0 on one carrying `index:-1` — and it counts what an OBSERVER sees
      by walking the tree for a shown `SELECTED`, because a test comparing
      `selectedIndex` to `-1` passed throughout the defect: `selectedIndex` was
      never wrong, the row was.
      `test_the_keep_guard_refuses_at_the_sentinel_even_when_a_row_carries_it`
      records something your write-up implies but does not state, and it is
      worse than cosmetic: with the check removed, a keep request **reaches the
      bridge** while nothing is selected (measured: 1 where 0 is required). So
      this was a path to storing an identity the user did not choose, not only a
      dead button.
      I rejected the range-check-at-the-render-site alternative for the reason
      your severity note implies: it leaves a candidate on screen the user can
      see and cannot choose, which is the same dead end with a narrower blast
      radius. A non-numeric sentinel (`null`/`undefined`) was also rejected —
      `selectedIndex` is an `int` property, so QML coerces and reintroduces `0`
      as the collision, which is strictly worse because `0` is a valid index.
      Recorded in `design.md` under "Selection is a candidate's own index, and
      the sentinel is unaddressable", with both rejected alternatives.

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
      **Note from `dev-writer` — this box is left OPEN and is `tester`'s, but
      the line it names no longer exists.** Fixing the defect above changed the
      shape of this gap, so read this before writing the test:
      Your diagnosis was right and the fixture you suggest is now impossible by
      construction — `isCandidate()` refuses a negative index at the boundary,
      so no candidate can carry the sentinel and `candidateAt(-1)` can never
      succeed. That made the first guard genuinely unreachable rather than
      merely untested, so I did not add a fixture to reach it: **I collapsed the
      two guards into one.** With the sentinel unaddressable, "is something
      selected" and "does the selection name a candidate" are the same question,
      and `candidateAt()` answers it. A guard that can only be true when another
      is is not a guard.
      So the mutation to run is now `if (candidate === null)` →
      `if (false)` at what is currently `OnboardingScreen.qml:237`, and I have
      verified it **fails 2 tests** (`test_keeping_is_refused_by_the_view_while_nothing_is_selected`
      and `test_the_keep_guard_refuses_at_the_sentinel_even_when_a_row_carries_it`)
      where the old two-guard arrangement left the suite green. The gap you
      measured is closed, but **I am not ticking your box** — whether the
      coverage is now adequate is your call, not mine, and you may want a test
      that pins the collapse itself rather than inheriting mine.

- [x] **`dev-writer`** — `OnboardingScreen.qml:302-360` — a candidate that is
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
      **Fixed** in `1d91627`, by the same `isCandidate()` sweep as the defect
      above — your framing that this is "the same confusion one level down" is
      why it is one mechanism rather than two.
      `test_a_candidate_that_is_not_an_object_is_a_failure_not_a_blank_row`
      fails without it, on your exact `[null,"str",{...}]` reply, and the five
      QWARN lines you measured are gone from the run.
      I added a third clause you did not ask for and should know about: a
      candidate whose `address` is missing or not a string is also refused. The
      spec calls the address "the only unforgeable way to tell two candidates
      apart", so a candidate without one is not something a user can choose
      between — it renders as a row with a blank where the deciding value goes,
      which is the blank-row outcome you describe arriving by a different route.
      `test_a_candidate_without_a_usable_address_is_a_failure` covers both
      spellings.
      Refusing the whole slate rather than dropping the bad entry is deliberate:
      dropping would silently show four of five and make "the count comes from
      the reply" false.

- [x] **`dev-writer`** — `OnboardingScreen.qml:164-166` — a non-string `reason`
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
      **Fixed** in `1d91627`, at all three sites you name, by the policy you
      name: a non-string is absent, never stringified.
      `test_a_non_string_reason_does_not_render_as_object_Object` fails without
      it on your `{"code":7}` reply, and asserts both halves — that
      `[object Object]` is absent AND that the screen still says something,
      since a blank would be its own defect.
      The two sibling sites resolve differently, which is a decision rather than
      an inconsistency and is recorded in `design.md`: `identityReason` becomes
      `""`, honestly "the module gave no reason this view could read" — a
      placeholder identical for every unreadable reason would distinguish
      nothing, and distinguishing the two absent cases is that field's whole
      job. But `slateId` is a **failure**, not a fallback, because it is the
      value a later keep sends back to core to say which set the selection was
      made against; there is no honest default for that, and a stringified one
      would be sent to core as if it were a set identifier.
      `test_a_non_string_slate_identifier_is_not_stringified_into_a_request`
      covers it.

- [x] **`dev-writer`** — `OnboardingScreen.qml:113-118` — `requestSlate()`'s
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
      **Fixed** in `1d91627`: `requestSlate()`'s success path nulls
      `keptIdentity`, so both exits from a state now clear the same things.
      `test_a_new_slate_clears_a_previously_kept_identity` fails without it —
      keep, then `requestSlate()`, then assert `keptIdentity` is null while
      `phase` is `"slate"`.
      Agreed on the reasoning rather than only the symptom: the spec asks this
      machine to be single-valued, and a second stale answer sitting beside the
      one value is exactly how the next reader is made wrong for free. Recorded
      in `design.md` alongside the `phase` decision so the invariant is stated
      where the machine is described, not only where it was violated.

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
      **Fixed** in `1d91627` by `dev-writer`, taking your durable fix exactly:
      no count in the copy and none in the assertion. The margin note now reads
      "may hold the same name", and
      `test_the_uniqueness_note_states_the_obligation_without_a_word_count`
      asserts the three obligations (not unique / not identifiers / the address
      distinguishes) and then sweeps for eight count spellings — "two words"
      through "five words" and their numeral forms — so it fails on the
      reintroduction of ANY number rather than pinning one. That is the
      difference your last sentence asks for: it catches misinformation instead
      of catching a reword.
      **Box addressing:** this box names `spec-writer`, and the coordinator
      dispatched it to me as a `dev-writer` item — the two disagree. I acted on
      it because the fix is entirely in code I own (`OnboardingScreen.qml` and
      `tst_onboarding_states.qml`, the two files your scenario cites) and
      because leaving a known-wrong count in shipping copy to await a handoff
      was the worse outcome. **Nothing here touches the spec**, so if
      `spec-writer` was wanted for something in `specs/` — the scenario's
      wording, say — that part is untouched and still open. Flagged to the
      coordinator in my report rather than silently absorbed.
      Recorded in `design.md`: the count's three moves, and why the obligation
      does not depend on it.

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
