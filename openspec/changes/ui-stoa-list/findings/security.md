# Security findings — `ui-stoa-list`

Reviewed dimension: **security**. (A sibling entry covers correctness; the
readability and architecture rows are untouched by this reviewer.)

This piece's stated substance is that the *view* is the security boundary: the
core refuses a record that does not match its address, and what the core cannot
do is stop a screen from claiming more than the check establishes. So the review
below is aimed at what a screen can be made to claim, not at what the core
computes. Everything was reproduced against the shipped components through a
recording fake bridge; no shipped file was mutated.

---

- [ ] **`dev-writer`** — `JoinScreen.qml:54` / `Main.qml:80` — **a hostile
      reference can be made to render under a "Joined." panel it never earned**
      **Scenario:** the attacker's goal is a screen that reads as settled for
      their address. Paste any legitimate reference and join it; then paste the
      attacker's reference. `Main.qml:80` rebinds `stoaAddress` to the attacker's
      address while `joinState` stays `joined` from the previous Stoa. Measured
      on the shipped code: the attacker's address `bbbbbbbb2222…` is on screen,
      the `joinedPanel` is visible, the body reads "This machine has started
      collecting this Stoa's records. Nobody was notified…", the join button is
      **gone** (`visible: screen.joinState !== "joined"`, line 429), and
      `join_stoa` was called exactly once — for the other Stoa. The user is shown
      a completed, irreversible-looking outcome for a Stoa the core never saw.
      **Severity:** high, and in my judgement the most serious thing in this
      change. The spec's residual-risk analysis assumes the worst case is a
      reader who verified the attacker's record against the attacker's address;
      this is strictly worse, because no verification happened at all and the
      screen still reads as done.
      **Measured:** 95 of 95 tests pass. Every join-state test builds a fresh
      `JoinScreen`; none reuses one, which is the only configuration `Main.qml`
      actually ships.

- [ ] **`dev-writer`** — `JoinScreen.qml:26,115` — the carried-over founding
      title lends a trusted name to an untrusted address
      **Scenario:** in the same sequence, the panel captioned `FOUNDING TITLE —
      FIXED FOREVER` renders the *previous* Stoa's title above the attacker's
      address, because `foundingTitle` is written from the reply and never
      cleared. This is the impersonation the lookalike requirement
      (spec.md:409) is built to expose, delivered by the view itself rather than
      by the attacker's record — and the lookalike panel cannot fire, because the
      titles being compared are the same string from the same source.
      **Severity:** high, and separable from the entry above: clearing
      `joinState` alone still leaves the wrong title on screen.

- [ ] **`tester`** — `tst_stoa_screens.qml:1375` — the address-note assertion
      admits a note that claims the Stoa itself is confirmed
      **Scenario:** the test's four checks are satisfied by copy that overreaches
      in exactly the way the spec forbids. Measured against the candidate note
      "The record shown is the one this address names, which confirms this is the
      Stoa you were sent. Nothing more is needed. No registry was consulted
      because none is needed; only the founding title is unverified.":
      `c1=true c2=true c3=true c4a=true c4b=true` — it passes every check. It
      carries `this address names` (c1), the word `unverified` (c2, scoped to the
      title only), `no registry` (c3), and `unverified` again neutralises the
      whole-screen `verified` scan (c4a), while avoiding the three literal
      phrases c4b blocks. Yet it tells the reader the address is the one they
      were sent — the single claim spec.md:289-312 says the copy must never make.
      The shipped string is correct; the *test* does not hold it there. Assert
      that the note contains an explicit negation of provenance (that nothing
      establishes this is the address the reader was meant to receive), rather
      than assembling more substrings.
      **Severity:** medium-high (test strength). This is the same defect shape
      the prompt records a tester finding once already — three required
      substrings present while the sentence around them says the opposite.

- [ ] **`dev-writer`** — `StoaReference.qml:45` — a single-pass `stripPrefix`
      lets a `stoa:` prefix reach `join_stoa`, converting a malformed paste into
      a verification accusation
      **Scenario:** `{"stoa":"stoa:stoa:<hex>","genesis":"00ff"}` parses `ok` and
      sends `{"stoa":"stoa:<hex>",…}`; the screen renders "the genesis record
      does not hash to this address". The security consequence is the one
      `design.md:85` names: the user is pointed at their sender rather than at
      their paste, and the *distinctness* of the three paste outcomes — the
      property this screen spends its design on — is defeated by input the
      attacker does not even need to control. Also listed under correctness; it
      is repeated here because the harm is a misdirected trust judgement, not
      just a wrong string.
      **Severity:** medium.

- [ ] **`spec-writer`** — `StoaReference.qml:80` — the reference encoding accepts
      arbitrary non-hex content in both halves, and no requirement says whether
      it should
      **Scenario:** `{"stoa":"zzzz not hex at all","genesis":"!!!"}` parses `ok`
      and is forwarded to `join_stoa`; so does a Cyrillic homoglyph address
      (`{"stoa":"аabb",…}`, U+0430) and a half containing `<img src=x>`. The view
      is right not to re-derive the hash — that is the core's one check, and a
      second implementation is what the design refuses. But "carries two halves"
      and "carries two halves that could conceivably be an address" are different
      gates, and the spec (spec.md:326-350) only requires the former, so the
      implementation is compliant while the question is genuinely open. A
      homoglyph address is also the one input where the 8-8-6 abbreviation and
      the full form can disagree for a reader. My judgement on the encoding: the
      JSON choice is sound — self-describing, so a truncated paste fails as
      malformed rather than as a verification mismatch, which is the distinction
      the whole screen rests on — and the parse is correctly strict on *types*.
      What is undecided is whether the halves need a character-class gate before
      the core sees them. That is a spec question, not a bug.
      **Severity:** low, and deliberately not addressed to `dev-writer`: tightening
      this without a requirement risks refusing a future address encoding.

---

## What was clean

**No reachable panic or throw was found on any peer-derived path.** `parse()`
handles truncated JSON, trailing bytes, arrays, bare scalars, `null` halves,
boolean halves, a 200 KB half and non-UTF-8-looking content without throwing.
`lookalikes` survived six malformed `heldStoas` shapes (non-array, null entry,
missing `stoa`, numeric `stoa`, object `foundingTitle`) returning 0 each time.
`Core.call()` guards the absent bridge, a throwing bridge, non-JSON, and JSON
that is not an object, and each reaches the screen as a failure rather than a
value — confirmed by the existing `tst_core_call.qml` suite, which I re-ran.

**Markup containment is real, not asserted.** Every peer-supplied string reaches
a `Text` pinned to `Text.PlainText`, the `ClipboardSink`'s `TextEdit` is pinned
likewise (`ClipboardSink.qml:56`), and the markup test correctly asserts
`textFormat` rather than `text` — the fix the file's header records. CI's
Text-vs-`textFormat` count gate (`ci.yml:262`) backs it structurally.

**The view performs no verification of its own**, as required: `parse()` checks
only that two string halves are present, and `joinState` moves to `joined` solely
on `reply.ok`. The share affordance is correctly absent where no record is held,
and `shareText` returns `""` rather than an address-only string — so nothing
unjoinable can be produced. No secret material is compared here, and no error
message leaks anything beyond what the core chose to say.

**The clipboard boundary is drawn honestly.** `ClipboardSink.qml:22-32` states
plainly that under `QT_QPA_PLATFORM=offscreen` no test can observe the system
clipboard, and that `lastCopied` records only what the sink was *asked* to copy;
`tst_stoa_screens.qml:489-492` repeats the limitation at the assertion rather
than implying coverage it does not have. That is the right shape.

**The `stoa:` prefix is never added on the way out** — `shareText` strips both
halves and emits bare hex, confirmed by round-tripping a doubly-prefixed input.
The failure is only on the way *in*, and only for a repeated prefix.

## What I could not check

- **What actually reaches the system clipboard.** Offscreen, `TextEdit.copy()`
  writes nowhere readable, so "the share string reached the clipboard" is
  unverified here and in CI. The code and the tests both say so.
- **The core's side of any of this.** Every reply was a fake; whether
  `join_stoa` in fact refuses a prefixed address, and with which message, is the
  core's own test surface, not this piece's.
- **`cargo mutants`** does not apply — this change is QML only, with no Rust in
  the diff.
- **Rendering fidelity.** Assertions run against the object tree, so a defect
  visible only in paint (contrast, occlusion, z-order) would not be caught by
  anything I ran.
