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

- [ ] **`dev-writer`** — `Core.qml:114` — the comment sends a reader to a
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

- [ ] **`dev-writer`** — `design.md:82,84` — the design names `parseReference`
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

- [ ] **`dev-writer`** — `Main.qml:38-40` — the comment says "a two-state
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

- [ ] **`tester`** — `tst_stoa_screens.qml:50` and `:207` — the same measurement
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

- [ ] **`tester`** — `tst_stoa_screens.qml:1433,1463,1487` — "all 47 prior tests
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

- [ ] **`tester`** — `tst_stoa_screens.qml:1045` — a superseded test is left in
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
