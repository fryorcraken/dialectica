# Correctness findings — `ui-stoa-list`

Reviewed dimension: **correctness**. (A sibling entry covers security; the
readability and architecture rows are untouched by this reviewer.)

Method: baseline suite green at 95 tests across 5 spec files before any probe.
Every scenario below was reproduced by driving the shipped components through a
recording fake bridge in a scratch `TestCase` under `tmp/`, which mutated no
shipped file; the scratch files were deleted before committing. Where a number
appears it was measured, not estimated.

---

- [ ] **`dev-writer`** — `JoinScreen.qml:92` / `Main.qml:99` — a second preview
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

- [ ] **`dev-writer`** — `JoinScreen.qml:26` — the founding title carries over
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

- [ ] **`dev-writer`** — `JoinScreen.qml:54` — a verification refusal for one
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

- [ ] **`dev-writer`** — `StoaReference.qml:45` — `stripPrefix` strips one
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
