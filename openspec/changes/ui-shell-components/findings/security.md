# ui-shell-components — security review

Dimension: **security** only. The other three dimensions (correctness,
readability, architecture) are held by other instances and are untouched here.

Measured on Qt 6.10.3, in a scratch worktree, against `piece/ui-shell-components`
at `d9fe78f`. Baseline before any mutation: **18 spec files, 0 failures**.

---

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DStatusBar.qml:120` —
      `ToolTip.text: lamp.explanation` renders peer-influenced text as **markup**.
      A `ToolTip`'s content item is a `Text` whose `textFormat` defaults to
      `Text.StyledText` (2), **not** `PlainText` — so the project's standing rule
      ("never bind raw peer text to a Text element with textFormat StyledText or
      RichText") is violated at the one sink in this piece that nothing covers.
      `deliveryText` / `storageText` / `zoneText` are free strings the component
      declares it does not trust (see its own comment at `:29-31`, "it does not
      trust its input"), and they are the sentence core supplies to explain a
      degraded lamp — an error string is exactly where attacker-influenced
      fragments (a peer address, a relay-supplied reason) end up.
      **Scenario:** `DStatusBar { deliveryText: "<b>OWNED</b> <img src=x>" }`.
      Hovering the DELIVERY lamp renders the tags as markup rather than showing
      them. The `<b>` is consumed and the `<img>` becomes an image request.
      **Measured:** a probe read the live tooltip's content item —
      `contentItem=QQuickText hasTextFormat=true textFormat=2`. And the markup is
      genuinely *interpreted*, not merely flagged: the same string
      `"<b>OWNED</b> x"` paints at `contentWidth` **102** in a `Text.PlainText`
      element and **57** in the ToolTip — the tags are consumed, not drawn. The
      ToolTip's own `text` property still returns the raw source
      (`<b>OWNED</b> x`), which is why a `text`-based assertion cannot see this.
      **Severity: high** — it is the one unsanitised markup path this piece adds,
      and it is the exact class the project already built `SanitisedText.qml` and
      a CI gate to close.
      **Fix shape** (do not apply from this file — noted only so the box is
      actionable): give the lamp's `ToolTip` an explicit content item with
      `textFormat: Text.PlainText`, the same way every `Text` in this piece does.

- [ ] **`tester`** — `dialectica-ui/tests/tst_status_bar.qml:227-250` — the
      `nonPlainTextElements` walker **cannot reach a ToolTip** and so reports the
      bar clean while the defect above is live. The walker descends
      `node.children`; a `ToolTip` is a popup whose content item is not a child of
      the component, and is created lazily on show.
      **Scenario:** instantiate `DStatusBar { deliveryText: "<b>OWNED</b> <img src=x>" }`
      and run the file's own walker over it.
      **Measured:** the walker returns `[]` — zero findings — against that exact
      string. `test_every_text_the_bar_renders_is_plain_text` therefore passes
      over a bar that renders markup. Neither `tst_status_bar.qml` nor
      `tst_vouch_stamp.qml` contains the substring `ToolTip` at all, so there is
      no tooltip assertion of any kind in this piece.
      **Severity: high** — this is the project's recorded defect family (a
      fixture where two explanations give the same answer: "no markup rendered"
      and "the walker never looked" are indistinguishable here). A test must
      assert the tooltip's content item's `textFormat`, driven by actually showing
      the tooltip.

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:306-318` — the
      `every QML Text declares an explicit textFormat` gate is structurally blind
      to `ToolTip.text`, and reports green over the defect above. It counts
      `\bText \{` openings against `textFormat:` assignments per file; a
      `ToolTip.text` binding creates a `Text` that matches neither pattern.
      **Measured:** on `DStatusBar.qml` the gate counts `opens=1` and
      `formats=1`, so `opens > formats` is false and the step passes — while the
      file renders markup through a second, uncounted Text element.
      **Severity: medium** — this is the "a gate the defect satisfies" shape: the
      check closes the question without measuring it. The gate should also
      require that a file containing `ToolTip.text` declares a `textFormat` for
      it (or that the repo forbids bare `ToolTip.text` in favour of a
      `DToolTip` wrapper that pins PlainText once).

---

## What was clean

**`textFormat` on every `Text` element in this piece.** Verified by reading, not
by trusting the gate: `DIdentityChip.qml:56,68,85`, `DVouchStamp.qml:67`,
`DStatusBar.qml:112` and `FlatButton.qml`'s label all set
`textFormat: Text.PlainText` explicitly. `AddressLabel.qml:33` (unchanged by this
piece, but reached from the new chip) sets it on its own root. The only gap is
the ToolTip path above.

**The author's claim that the tests assert `textFormat !== 0` rather than reading
`Text.text` is true**, and the reasoning behind it is correct.
`tst_identity_chip.qml:221-224`, `tst_vouch_stamp.qml:147-150`,
`tst_flat_button.qml:206-209` and `tst_status_bar.qml:232-235` each walk for
`node.textFormat !== 0`. I confirmed independently why this matters: the ToolTip
probe showed `text` returning `<b>OWNED</b> x` verbatim while the element painted
it as markup, so a string-based assertion genuinely cannot fail on a format
mutation. The claim checks out; the walkers' only weakness is reach, not the
predicate.

**Nothing in this piece renders a value outside the sanitisation scope.**
`generatedName` and `identityAddress` are locally derived from key material, which
`docs/PLAN.md` places outside the sanitiser's remit (post/comment content, Stoa
title and description). `DVouchStamp`'s labels are hardcoded literals
(`"VOUCHED"` / `"VOUCH"` at `DVouchStamp.qml:25`), and `DStatusBar`'s lamp labels
are the three fixed strings — `test_a_lamp_given_no_explanation_invents_none`
pins that the bar renders nothing it was not given. The lamp *explanation* is the
one free string, and it is the finding above.

**`DIdentityChip`'s two arms are genuinely mutually exclusive, in both
directions.** `hasIdentity` gates all four identity-present children
(`:47,52,59,71`) and both no-identity children (`:77,88`) on the same single
boolean, so there is no state in which both or neither renders. The tests cover
both directions and the transition: `test_with_no_identity_nothing_claims_one`
(`:146`) deliberately populates `generatedName` and `identityAddress` and asserts
neither reaches the screen and no mark is drawn;
`test_with_an_identity_the_prompt_is_gone` (`:168`) covers the converse; and
`test_losing_an_identity_switches_the_chip_back` (`:179`) proves the binding is
reactive rather than read once at construction. The walker used here is
visibility-aware (`:31-41`, checking `ancestorsVisible && node.visible !== false`),
so an invisible child's text is correctly not counted as on-screen. This is the
"who am I acting as" affordance done right.

**An unrecognised lamp state cannot map to `ok`.** `normalisedState`
(`DStatusBar.qml:49-53`) is an explicit allow-list of the three known values with
`degraded` as the fallback, which is the correct direction — the inverse-shaped
implementation the comment describes (`s === "failed" ? … : "ok"`) would read
every unknown value as green. `test_an_unrecognised_state_degrades_rather_than_reading_as_ok`
(`:49-60`) drives 13 values including `""`, `"OK"`, `"okay"`, `"FAILED"` and
`"undefined"`, asserting both the normalised string and the rendered colour; and
`test_the_three_named_states_are_returned_unchanged` (`:33`) is the necessary
companion that stops a function returning `"degraded"` for everything from
passing. Case sensitivity is pinned separately at `:66`.

**On whether `failed` would be the safer default than `degraded`:** I do not
think so, and I am not opening a box for it. The two are not ordered purely by
severity here — they differ in what they claim. `failed` asserts the subsystem is
known broken, which the UI cannot back from a string it could not parse, and it
is the same overclaiming (in the other direction) that the `ok` fallback was
rejected for. `degraded` is the honest reading of "this machine does not know",
and it is visually distinct from green, so the lamp still withholds the
assurance. Routing every unknown value to a red alarm the user cannot act on also
erodes red's meaning for the genuine `failed` case. The recorded choice is right.

**No secrets, paths or key material in rendered strings or logs.** No `console.log`,
no file path, no key material in any of the new components. `identityAddress` is
public-by-design and is abbreviated through `AddressLabel`'s head/middle/tail
scheme rather than printed raw; no private key or seed reaches any of these
components' properties.

**`Identicon.muted` cannot leak or alter identity.** `Identicon.qml:69` applies
`muted` as an `opacity` on the root, outside `onPaint`, so it cannot reach a
colour or shape selector — the determinism contract holds and two peers render
the same address identically regardless of which list it appears in. No security
consequence.

**`FlatButton`'s unknown-kind fallback is a security improvement, not a
regression.** The previous form rendered an unrecognised `kind` as paper-coloured
text on a transparent ground with no border — an invisible control that still
accepted clicks. Falling back to `secondary` (`FlatButton.qml`) makes a typo'd
kind visible. Worth noting positively since an invisible-but-clickable control is
a genuine clickjacking-adjacent shape.

---

## Method note

Three findings, all one defect and its two missing guards. Probes were run in a
scratch worktree (`review/ui-shell-components/security`) and deleted; the full
suite was re-run to confirm the 18-spec / 0-failure baseline, and the worktree
was verified clean (`git status --porcelain` empty) before this file was written.
No source file on the piece branch was modified by this review.
