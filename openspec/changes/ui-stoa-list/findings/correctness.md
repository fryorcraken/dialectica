# Correctness findings — `ui-stoa-list`

Reviewed dimension: **correctness**. (A sibling entry covers security; the
readability and architecture rows are untouched by this reviewer.)

Method: baseline suite green at 95 tests across 5 spec files before any probe.
Every scenario below was reproduced by driving the shipped components through a
recording fake bridge in a scratch `TestCase` under `tmp/`, which mutated no
shipped file; the scratch files were deleted before committing. Where a number
appears it was measured, not estimated.

---

- [x] **`dev-writer`** — `JoinScreen.qml:92` / `Main.qml:99` — a second preview
      inherits the first preview's `joinState`, so a Stoa nobody joined renders
      under the "Joined." panel
      **Scenario:** with one `JoinScreen` instance reused for every reference
      (`Main.qml:88`), paste reference A, press Join (core answers success),
      then — **without pressing Cancel** — paste reference B in the still-mounted
      list field and press "Look at it first". `Main.qml:80` reassigns
      `root.previewing`, so `screen.stoaAddress` becomes B, but nothing resets
      `joinState`. Measured on the shipped components: `join.stoaAddress` =
      `bbbbbbbb2222…`, `joinState` = `joined`, the `joinedPanel` is on screen,
      the body reads `This machine has started collecting this Stoa's records`,
      and `join_stoa` was called **once** — for A, never for B. The user is told
      they joined the attacker's Stoa when no join call was made for it.
      **Severity:** high. It is the direct negation of the spec's "A join is
      reported from the core's reply, never assumed" (spec.md:375), reachable
      from an ordinary two-paste sequence with no Cancel and no error.
      **Measured:** all 95 tests pass while this holds; every existing join-state
      test constructs a fresh `JoinScreen`, so none exercises reuse.

      **Fixed** in the commit carrying this file. Reproduced first: the new test
      failed with `Actual: joined / Expected: previewing` before any change, so
      the defect is confirmed independently of the report.

      **The fix is structural rather than a reset**, which the finding's own
      framing invites and which I think would have been the wrong answer. A
      `reset()` on navigation has to be *remembered* at every present and future
      entry point, and the one place it is forgotten is a screen claiming
      something about the wrong Stoa. Instead `JoinScreen` now stores the outcome
      **together with the reference it describes** — `{stoa, genesis, state,
      failure, foundingTitle}` — and `joinState`, `failure` and `foundingTitle`
      are `readonly` properties derived through `currentOutcome`, which yields
      `null` unless the stored pair equals the pair on screen. A stale outcome is
      not unlikely, it is unrepresentable: there is no longer a variable that can
      hold one Stoa's verdict while another is displayed. `join()` also captures
      the pair before calling and never re-reads `stoaAddress` afterwards, since
      that property is a binding that may have moved.

      **Test that fails without it:**
      `test_a_second_preview_does_not_inherit_the_first_joined_state`. It drives
      `Main.qml` — the reused-instance shape the app ships — and asserts the
      user-visible consequences as well as the state string: no `joinedPanel` on
      screen, the `joinButton` back, and no "started collecting" in the body.

- [x] **`dev-writer`** — `JoinScreen.qml:26` — the founding title carries over
      from the previous preview, captioning one Stoa with another's title
      **Scenario:** same two-paste sequence. `join()` writes
      `screen.foundingTitle` from A's reply (`JoinScreen.qml:115`) and nothing
      clears it when `stoaAddress` changes. Measured: previewing B renders
      `FOUNDING TITLE — FIXED FOREVER` above `Nym Research`, which is A's title;
      B's record was never decoded. An attacker who gets a user to preview their
      reference right after a legitimate join has their address decorated with a
      title the user already trusts — the exact lookalike harm the spec's
      title-is-not-an-identifier requirement (spec.md:409) exists to surface.
      **Severity:** high. Distinct from the entry above: that one is the joined
      *claim*, this one is the wrong *title*, and fixing one does not fix the
      other.
      **Measured:** 95 of 95 tests pass with this behaviour present.

      **Fixed** in the same commit and by the same shape — `foundingTitle` is now
      a derived `readonly` property reading `currentOutcome.foundingTitle`, so it
      can only ever be the title the core returned for the reference on screen.
      The reviewer's "distinct from the entry above" is right and the separation
      was worth making: the derivation had to be written for the title as well as
      for the state, not merely inherited from it.

      One consequence worth recording, which the finding implies but does not
      state: `lookalikes` compares against `foundingTitle`, so a carried-over
      title could previously **suppress the lookalike comparison entirely** by
      making both sides of the equality the same string from the same source. The
      requirement built to expose impersonation was blind to the one the view
      itself produced. That is now impossible for the same structural reason.

      **Test that fails without it:**
      `test_a_second_preview_does_not_inherit_the_first_founding_title` —
      confirmed failing with `Actual: Nym Research / Expected: ""` before the fix.

- [x] **`dev-writer`** — `JoinScreen.qml:54` — a verification refusal for one
      Stoa stays on screen while a different Stoa is previewed
      **Scenario:** preview A, press Join, core answers
      `{"error":"the genesis record does not hash to this address"}`, then
      preview B. Measured: `joinState` = `failed`, `joinFailurePanel` visible,
      and the rendered refusal is A's — attached to B's address, which was never
      submitted to the core. A user reads "the record you were sent is wrong"
      about a reference the core has not seen.
      **Severity:** medium-high. Same root cause as the two above (no reset on
      `stoaAddress` change) but a separately observable wrong claim, so it needs
      its own regression test rather than being assumed fixed.

      **Fixed**, and the reviewer was right to insist on its own test rather than
      letting it ride on the other two: `failure` needed its own derivation, and
      a fix that cleared only `joinState` would have left the refusal text in
      place with nothing rendering it — a latent wrong claim waiting for the next
      binding change.

      **Test that fails without it:**
      `test_a_second_preview_does_not_inherit_the_first_refusal` — confirmed
      failing with `Actual: failed / Expected: previewing`. It asserts the
      absence of the `joinFailurePanel` on screen, not only the empty string.

- [x] **`dev-writer`** — `StoaReference.qml:45` — `stripPrefix` strips one
      prefix, so a doubled `stoa:` reaches `join_stoa` and surfaces as a
      *verification* failure
      **Scenario:** paste `{"stoa":"stoa:stoa:b02d5e77…","genesis":"00ff"}`.
      `stripPrefix` removes one occurrence and returns `stoa:b02d5e77…`;
      `parse()` answers `ok`. Measured end-to-end: the request sent is
      `{"stoa":"stoa:b02d5e77…","genesis":"00ff"}` and the screen renders **"the
      genesis record does not hash to this address"**. `design.md:85` and
      `StoaReference.qml:37-42` both state this must not happen — a prefix
      reaching `join_stoa` "would be a hash verifying against nothing, surfacing
      as a verification failure rather than as the malformed paste it actually
      is". The invariant the comment asserts is not the one the code enforces.
      A user who double-pasted is told to blame their sender.
      **Severity:** medium. It also means `JoinScreen`'s `previewAddress`
      renders `stoa:b02d5e77…` as "the address in full", so the address shown is
      not the address.
      **Measured:** `test_a_display_prefix_is_stripped_before_anything_is_sent`
      passes, because it only ever supplies a single prefix.

      **Fixed.** `stripPrefix` now loops until no prefix remains, trimming
      whitespace between passes so `stoa: stoa:<hex>` is handled too. A loop
      rather than a second `if`: the defect was a *fixed number of passes*, so a
      fix with a different fixed number is the same defect one step further out.

      The reviewer's second observation — that `previewAddress` was rendering
      `stoa:<hex>` as "the address in full", so the address shown was not the
      address — is the part I had not seen, and it is arguably worse than the
      misdirected refusal: the screen whose entire job is showing the address
      exactly was showing something else.

      **Test that fails without it:**
      `test_a_repeated_display_prefix_is_stripped_rather_than_forwarded` —
      confirmed failing with `Actual: stoa:b02d5e77… / Expected: b02d5e77…`. It
      covers two and three prefixes, interleaved whitespace, and asserts
      end-to-end that nothing carrying a prefix reaches `join_stoa`.

- [ ] **`tester`** — `tst_stoa_screens.qml:1306` — the malformed-paste assertion
      blocks three exact phrasings and admits equivalent misinformation
      **Scenario:** the `noVerif` check rejects only `does not hash`,
      `did not verify` and `does not match`. Measured against candidate reasons:
      "What was pasted could not be confirmed against its address." and "What was
      pasted failed the check against the address it names." both score
      `names=true noVerif=true` — they pass. Each tells a user who truncated a
      paste that their sender's record is wrong, which is the precise
      misinformation the test's own header says a tester already demonstrated
      once. Assert the *meaning* — that a malformed paste must not describe any
      comparison against the address, however phrased — rather than extending the
      blocklist to five strings.
      **Severity:** medium (test strength, not a live defect: the shipped string
      is correct today).

- [ ] **`tester`** — `tst_stoa_screens.qml:405` — `indexOf("members")`
      over-matches `membership`, so the count assertion is one honest copy edit
      from a false failure
      **Scenario:** the list's empty-state body says "Your membership was read
      without error…" (`StoaListScreen.qml:288`) and the failed-state body says
      "…could not be listed right now". `"membership".indexOf("members")` is `0`.
      The assertion passes today only because its fixture puts a row on screen,
      which hides both panels. Moving that sentence into a row, or adding a row
      to the empty-state fixture, fails the test for a reason that is not a
      defect. Use a word-boundary regex, as
      `test_joining_a_stoa_already_held_is_success_with_no_warning` already does
      for `\bagain\b` (line 876).
      **Severity:** low (fragility). The prompt asked whether this sweep fix
      landed: it did **not** — the bare `indexOf` is still at line 405.

- [ ] **`tester`** — `tst_stoa_screens.qml:717` — `indexOf("identity")`
      over-matches `identical`, the same unlanded fix
      **Scenario:** `"identical".indexOf("identity")` is `-1`, so this one is
      currently safe by luck of spelling — but `JoinScreen.qml:335` already uses
      "identical" in a comment and the lookalike panel is one copy edit from
      saying "the titles are identical" on screen, at which point the assertion
      fails on honest copy. It is listed separately from the `members` entry
      because it needs its own boundary fix, not the same one.
      **Severity:** low (fragility).

---

## What was clean

The `StoaReference.parse` type discipline is genuinely strict where it matters:
`typeof` per half rather than truthiness means a number, an object, an array, a
boolean or `null` in either field is refused rather than coerced, and a bare JSON
string, number or `true` is refused as not-an-object. Trailing bytes after a
well-formed object are refused by `JSON.parse` rather than ignored. A 200 KB half
parses without incident — there is no quadratic path and no fixed buffer.

`JoinScreen.lookalikes` survived every hostile `heldStoas` shape I could
construct — a string in place of an array, a `null` entry, an entry missing
`stoa`, a numeric `stoa`, an object `foundingTitle` — returning 0 without
throwing in each case, and 1 for the genuine same-title-different-address case.
No reachable panic there.

`StoaListScreen.reload()`'s failure path never writes `rows`, and the `Repeater`
model is gated on `readState === "ok"`, so a listing that succeeds and then fails
on reload correctly hides the stale rows: measured, the previous title and
address are both off screen while the failure banner is up. (`screen.rows` itself
retains the old array, which is invisible to the user but is a trap for a future
call site — noted for the architecture reviewer rather than opened as a box.)

The `visibleText` / `visibleNamed` walkers do reach `Repeater`-created delegate
content, so the absence assertions built on them are not vacuous for that reason.
The three-state list distinction, the non-array `items` guard, and the
"unreachable core" path all behave as specified.
