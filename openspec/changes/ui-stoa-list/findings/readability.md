# Readability findings — `ui-stoa-list`

Reviewed dimension: **readability**. (A sibling file in this commit covers
architecture; correctness and security were reviewed before me and their entries
are acted on. I re-read both before starting so as not to re-report them.)

Method: baseline suite green at **100 tests across 5 spec files** (13 + 12 + 7 +
9 + 59) before any probe, re-measured after. Every claim below that says
"measured" was produced by driving the shipped components under
`qmltestrunner` in a scratch `TestCase` in my own worktree; the scratch files
were deleted and the tree left clean. `qmlformat -n` parses all four new files;
CI's `Text {` vs `textFormat:` count gate balances on each (`JoinScreen` 13/13,
`StoaListScreen` 14/14, `ClipboardSink` 0/1, `Main` 0/0).

The headline question I was given — *is the limit of the verification claim
visible in the code, or only in the spec?* — comes out **well**. See "What was
clean" below; the entries here are the places where a reader is pointed at
something that is not there, or where a number in a comment is not a number a
command answers.

---

- [x] **`dev-writer`** — `Core.qml:114` — the comment sends a reader to a
      function that does not exist, in a file that does not contain it
      **Scenario:** `joinStoa`'s doc comment ends *"which is why the view's paste
      field and its share affordance are one decision — see
      `JoinScreen.parseReference`."* There is no `parseReference` anywhere in the
      view, and `JoinScreen.qml` contains no parse function at all. The thing
      being pointed at is `StoaReference.parse`, in `StoaReference.qml`. Measured:
      `grep -rn "parseReference" dialectica-ui/` returns this line and nothing
      else.
      This is the single most load-bearing cross-reference in the change — it is
      the pointer from the core call wrapper to the file that owns the round
      trip, and it is the sentence a reader follows when asking "where is the
      one decision this comment says exists?". It sends them to the wrong file
      and then to a name that is not in it, so the reader concludes the pairing
      claim is stale rather than that the comment is.
      **Severity:** medium. A comment that names a symbol is checkable and this
      one is checkably wrong; per CLAUDE.md a comment earns its place by saying
      what a command cannot, and this one says something a command refutes.

      **Fixed.** It now points at `StoaReference.qml` and names both functions —
      "which owns both `parse` and `shareText` in one file so the two ends cannot
      drift" — so the pointer carries the *reason* the file is one file, which is
      the thing the sentence was trying to convey.

      Your diagnosis of the cost is the part I had not seen: a reader who follows
      a wrong pointer concludes the **claim** is stale rather than the comment.
      That is worse than no comment, because it quietly discredits the pairing
      invariant this change exists to establish.

- [x] **`dev-writer`** — `design.md:82,84` — the design names `parseReference`
      and `shareTextFor`, the code has `parse` and `shareText`
      **Scenario:** D1's closing paragraph, the one that carries the whole
      strip-on-the-way-in/never-on-the-way-out invariant, reads
      *"`parseReference` accepts an address written `stoa:ab12…` … `shareTextFor`
      writes bare hex."* Both names are wrong; `StoaReference.qml:79,97` spell
      them `shareText` and `parse`. Measured: `grep -rn
      "parseReference\|shareTextFor"` over `dialectica-ui/` and
      `openspec/changes/ui-stoa-list/` returns exactly three hits — this pair
      plus the `Core.qml` one above, and zero hits in any `.qml` implementation.
      Listed separately from the `Core.qml` entry because it is a different file
      with a different owner and a different fix; a reader who greps the design's
      name finds nothing and cannot tell whether the function was renamed or
      never written.
      **Severity:** low-medium. It is prose rather than code, but D1 is the
      section a later change will read before touching the encoding, and two
      wrong symbol names at the top of it cost that reader the grep.

      **Fixed** — both now `StoaReference.parse` and `StoaReference.shareText`,
      qualified rather than bare so the grep lands in the right file too. I
      re-ran your measurement afterwards: `grep -rn "parseReference\|shareTextFor"`
      over `dialectica-ui/` and the change folder now returns only this findings
      file and the `Core.qml` box above, with zero hits in any implementation,
      design or spec text.

      You were right to file it separately from the `Core.qml` entry. They looked
      like one typo repeated and they are not: that one misnames a *file* as well
      as a symbol, and this one sits in the section a future change reads before
      touching a compatibility surface. Different readers, different costs.

- [x] **`dev-writer`** — `Main.qml:38-40` — the comment says "a two-state
      navigator" directly above a three-state one, and does not say which state
      wins
      **Scenario:** the comment reads *"A two-state navigator needs no
      StackView. Its entire state is which of **these three** is non-null"* — it
      contradicts itself inside two sentences, having been written for the
      two-screen version and not updated when the join screen arrived. The
      substantive half of the problem is what it therefore fails to say:
      `screenShown` (lines 41-44) is an ordered ternary, so `chosen` **beats**
      `previewing`, and that precedence is a real behavioural decision nothing
      records. Measured by driving `Main.qml`: with `chosen` set and
      `previewing` then also set, `screenShown` stays `"feed"` — the preview is
      silently swallowed. A reader deciding whether a new caller may set
      `previewing` has no way to learn that from the file.
      **Severity:** medium. This is the "a comment restating the code is noise;
      an absent comment where a reader would ask why is a finding" case in both
      halves at once — the sentence that is present is wrong, and the sentence
      that would be useful (why `chosen` outranks `previewing`) is absent.
      The architecture consequence of that precedence is filed separately in
      `architecture.md`; this box is only about the comment.

      **Fixed**, and the two halves you named were fixed differently, which is
      the part worth recording.

      The wrong sentence is now right: "A three-screen navigator, and it still
      needs no StackView", with D5's argument intact and cited. It was written
      for the two-screen version and not updated when the join screen arrived.

      **The absent sentence was not written — the thing it would have explained
      was removed instead.** You asked for the missing "why does `chosen` outrank
      `previewing`", and the honest answer turned out to be that it should not:
      the precedence was an accident of ternary ordering, not a decision. So the
      architecture box's fix makes the two mutually exclusive, and the comment
      now says the ternary *renders* the state rather than resolving a clash —
      which is a claim a reader can check, where "chosen wins, because…" would
      have documented an arbitrary choice as though it were reasoned.

      That is the better outcome of the two your box allowed, and I would not
      have got there from the architecture box alone: it framed this as latent
      and unreachable, while your framing — a reader cannot learn the rule from
      the file — is what made "there is no rule worth learning" the answer.

- [x] **`tester`** — `tst_stoa_screens.qml:50` and `:207` — the same measurement
      is stated twice, in different words, 157 lines apart
      **Scenario:** line 50 says *"It is 747 of the 1371 characters a join screen
      renders — measured, not estimated"*; line 207 says *"Measured, not
      estimated — 747 of 1371 characters on a rendered preview."* One number,
      two copies, and the two describe the corpus differently ("a join screen
      renders" vs "on a rendered preview"). The figure is defensible as a
      measurement of a thing being removed — `design.md` D7 explains why it was
      worth taking — but a duplicated constant is a constant that can go half
      stale, and the apparatus column is scheduled for removal by another piece,
      which is precisely when one copy gets updated and the other does not.
      **Severity:** low. Keep one, at the `bodyText` helper where it justifies
      the subtraction, and delete the other.

      **Fixed**, keeping the copy at `bodyText` exactly as prescribed — that is
      where the number does work, because it is the argument for subtracting the
      column at all. The header now says "most of what a whole-screen scan sees"
      and points at the helper for the figure.

      The reasoning is worth keeping: the apparatus is scheduled for removal by
      another piece, and a duplicated constant is one somebody updates in one
      place. That is the same failure mode as the box below — a claim no command
      can check — and the two arrived together for that reason.

- [x] **`tester`** — `tst_stoa_screens.qml:1433,1463,1487` — "all 47 prior tests
      passed" is stated three times and is already arithmetically impossible
      **Scenario:** three mutation notes each say the mutation *"left all 47
      prior tests passing"*. The file now defines 47 `test_*` functions, so a
      test inside it cannot have 47 *prior* ones; each figure is a snapshot from
      the moment it was written. `:694` carries the same shape with "95 of 95
      passed", against a suite that now runs 100. None of these can fail loudly —
      they are exactly the class CLAUDE.md's "do not write down anything a
      command can answer" section exists to prevent, and the suite is the
      command. Rephrase as the relation ("the suite was green with this defect
      present") and let the runner supply the number.
      **Severity:** low. No test is weakened by it; it is a maintenance claim
      that rots silently.

      **Fixed**, all four, as the relation with the number left to the runner:
      "left the whole suite green with the defect present", and for the reused-
      instance note, "every test in every spec file passed with that defect
      present".

      **The finding understates it — the drift was worse than reported.** The
      file now defines 64 `test_*` functions (`grep -c "function test_"`), so
      "47 prior" was stale by seventeen rather than merely impossible, and the
      reviewer's own "47" was itself a snapshot taken mid-review. That is the
      cleanest possible demonstration of why the number should not have been
      written down: it went stale between the review and the fix.

      What the sentences are *for* survives intact — that the mutation was
      invisible to everything else — and that claim is the one a reader needs
      and the one that does not rot. `CLAUDE.md`'s "name the command instead"
      applies to test comments exactly as it does to prose, which I had not
      carried across.

- [x] **`tester`** — `tst_stoa_screens.qml:1045` — a superseded test is left in
      place with nothing marking it as superseded, so a reader learns the weaker
      rule first
      **Scenario:** `test_a_record_that_does_not_verify_is_a_failure_distinct_from_a_bad_paste`
      (1045-1070) asserts only that the two refusals *differ*. The file's own
      header (lines 64-66) records that distinctness is insufficient — misinforming
      strings are still distinct strings — and
      `test_a_malformed_paste_and_an_unverified_record_say_different_things_to_do`
      (1575-1642) is the replacement, built on the same fixture and the same
      error string. Both remain, 530 lines apart, with no cross-reference in
      either direction. A reader arriving at 1045 reads the rule the file
      elsewhere says is not enough, and has no signal to keep reading.
      **Severity:** low. Not a coverage gap — the strong test is present and
      green. It is a reading-order defect: the repo's own memory note records
      that a known-weak test is the template the next one gets written against,
      and this one is unlabelled.

      **Fixed by removing it**, which is further than the box asked, so the
      argument matters.

      The finding offers labelling. I took deletion because **a label is a
      weaker guard than absence** against the exact failure the box cites: if a
      known-weak test is the template the next one is written against, a comment
      saying "this one is weak" does not stop it being copied — it only means
      whoever copies it was warned. The repo's memory note says to fix the
      family rather than the instance, and the instance here is a whole test.

      **Removal was only available because nothing was lost, and I proved that
      rather than asserting it.** Two mutations, each aimed at one half of what
      the removed test did:
        - *the core's refusal swallowed* — `joinFailureText` bound to
          "Something went wrong. Please try again." instead of `screen.failure`.
          **Two** tests fail where the removed one made one assertion:
          `…say_different_things_to_do` on the same claim it made, and
          `test_a_failed_join_does_not_report_the_stoa_as_joined` independently.
        - *the two refusals made identical* — the malformed reason replaced with
          the core's verification string verbatim, which is precisely the
          "reads alike" case the removed test existed for. Caught, and caught
          **earlier** than it would have been: the failure is on "a malformed
          paste must name what was pasted", a meaning check, rather than on the
          inequality the old test asserted.

      So coverage strictly increased. What replaced the test is a comment at the
      section head recording that it was removed, why distinctness was
      insufficient, and where each of its assertions now lives — which is the
      cross-reference the box asked for, pointing forward from an absence
      instead of sideways from a live weak test.

---

## What was clean

**The verification claim's limit is carried in the code, not only in the spec,
and that is the thing I was asked to check hardest.** `JoinScreen.qml:246-255`
is a nine-line comment that states the narrowing in the reader's terms — *"a
hash comparison between TWO INPUTS THE USER SUPPLIED … a reader who pasted a
hostile address and saw a verified record has verified the attacker's record
against the attacker's address, perfectly successfully"* — and it sits
immediately above the `addressNote` string it governs, so the claim and the copy
cannot drift apart on screen without drifting apart in one visual block. The
shipped copy then says the same thing in user-facing words ("nothing here says
this is the address you were meant to receive"). Someone deleting the caveat has
to delete a paragraph explaining why it is there. That is the right shape.

**Which lines carry the security property is legible.** The three that matter
each announce themselves: `StoaReference.qml:92-96` says the parse checks only
that two halves are present and never whether they match, and says why a second
implementation would be one too many; `JoinScreen.qml:104-107` says `joinState`
is derived and never assigned; `JoinScreen.qml:159-163` says the reference is
captured before the call and never re-read after it. A reader can tell these
from the presentation code around them without being told which is which.

**The `outcome` invariant is legible exactly where someone would be tempted to
add a plain variable.** `JoinScreen.qml:22-50` is positioned at the property
declaration itself, not in a distant design note, and it states the invariant in
the form a future editor needs: not "don't add a variable" but *why* a `reset()`
is the wrong fix — "a reset has to be REMEMBERED, at every present and future
caller, and the one place it is forgotten is a screen making a claim about the
wrong Stoa". `currentOutcome` (57-61) then sits between `outcome` and its three
readers, so the derivation is physically in the way. This is the best-written
block in the change.

**The six join states read as six meanings, not as an enum waiting to be
collapsed.** `JoinScreen.qml:91-107` gives each value a one-line gloss and then
states the argument against booleans concretely — *"a `malformed` that is a
distinct value cannot reach the branch `failed` renders through … the bug would
be a missing `&& !`"*. The list's three states get the same treatment at
`StoaListScreen.qml:17-21`, with the reason the distinction matters more here
than on the feed. Someone proposing to merge two of them has to answer a written
argument.

**`stripPrefix`'s loop carries its reasoning at the loop.**
`StoaReference.qml:44-61` is directly above the `while`, and it names the
generalisation rather than the instance: *"the defect was a fixed number of
passes, so a fix with a different fixed number is the same defect further out."*
That is the sentence that stops the "simplify this to an `if`" edit, and it is in
the only place that edit gets made.

**The absent things are explained where they are absent.** Three positions in
the change render nothing, and each has a comment at the empty position rather
than a note elsewhere: the per-row count (`StoaListScreen.qml:367-376`), the
current-title panel (`JoinScreen.qml:320-328`), and the genesis record a fresh
creation does not carry (`StoaListScreen.qml:146-150`, which explicitly says it
is "said plainly rather than left as an apparent oversight"). That last one is
the right instinct — it is the comment a reader would otherwise file as a bug.

Naming is a genuine strength throughout: no function here has an `And`, and no
`handle`/`process`/`update`. `canShare`, `genesisFor`, `lookalikes`,
`currentOutcome` and `shareText` each say what they answer. Test names state
properties rather than mechanisms, almost without exception.

## What I could not check

- **Anything visual.** QML tests see the object tree, never pixels. Whether the
  failed and empty list states *read* as different screens, and whether the
  address is set large enough to compare by eye, are unverified by anything I
  ran — which the change's own tasks.md already says.
- **`cargo mutants`** does not apply: the implementation diff is QML only. The
  Rust in the diff (`transport.rs`) arrived with the merge of `main`, not with
  this piece.
