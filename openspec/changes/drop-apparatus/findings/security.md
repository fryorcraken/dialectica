# Findings — security

Reviewed `piece/drop-apparatus` at `927dd9f` against CLAUDE.md's two standing
rules (never trust an inbound message; moderation must be authenticated and
authorised) and the usual boundary checks.

## Scope, measured first

`git diff 468e716 927dd9f --stat` over `dialectica-ui/` and `dialectica/` touches
six QML files and nothing else. **No core change, no Rust, no wire handler, no
storage, no key material, no moderation path.** Nothing in this diff parses,
validates, indexes, slices or allocates from peer bytes, so the panic-as-DoS
surface PHASE0-FINDINGS §3 measures is not reachable from anything here.

## What was checked and is clean

**The one string this change adds is a literal, and it is explicitly plain text.**
`FeedScreen.qml:225-233` binds a hardcoded constant with `textFormat:
Text.PlainText`. It cannot carry peer input, and even if a later change made it
dynamic it is opted out of QML's `AutoText` markup sniffing. This is the trap
CI's `textFormat` gate exists for, and the gate still balances after the change
(13 `\bText \{` against 13 `textFormat:` in `FeedScreen.qml`, re-measured).

**The deletions remove rendering surface rather than validation.** `MarginNote`
bound a `body` property that was correctly defended (`textFormat: Text.PlainText`
with a comment saying why). Deleting it removes one place peer-supplied text
*could* have been rendered; it does not remove a check that anything relied on.
No guard, no sanitiser and no authorisation test was deleted or bypassed —
`SanitisedText` is untouched and is still what every peer-supplied body goes
through (`FeedScreen.qml:386-400`).

**Nothing about identity presentation was weakened, and this is the one place
this change could plausibly have hurt security.** The deleted `ON THE MARK` note
concerned identicon-versus-address, which is a real impersonation surface: a
generated mark is a second forgeable channel and must never stand in for the
address. Verified structurally rather than taken on trust — `PostHeader.qml:35-38`
renders `AddressLabel` with **no `visible:` binding and no empty-string
collapse**, while the `Identicon` at `:21-26` is the element that *can* hide
(`visible: root.markSize >= Theme.markMinDraw`). The asymmetry runs the safe way:
an address can appear without a mark, a mark cannot appear without an address.
Same at the Stoa header (`FeedScreen.qml:123-143`). `AddressLabel.qml`'s
head/middle/tail abbreviation — the anti-vanity-address measure — is unchanged.

**Error text.** The failure path still renders core's message verbatim
(`FeedScreen.qml:280-287`) and this change does not widen it. Nothing new leaks a
path, a key, or an internal identifier.

## Findings

- [ ] **`dev-writer`** — `ScreenFrame.qml:28-37` — a shared-shell layout change
      whose failure mode is an invisible element, on the shell every future screen
      uses
      **Scenario:** a child declaring `Layout.fillHeight: true` inside a
      `ScreenFrame` now lays out at `height=0` (measured: 544 on `origin/main`,
      0 here, same markup, Qt 6.10.3). It renders nothing, logs nothing, and lints
      clean.
      **Why this is in the security file and not only the correctness one:** the
      screens queued on this shell are the onboarding screen (#60), the join and
      Stoa-list screens (#63) and the composer (#62), and the content those
      screens must not silently drop includes a seed-phrase permanence warning, a
      closed-gate reason, and a publish outcome that must not claim delivery. An
      interface obligation discharged by an element that happens to be
      zero-height is an obligation not discharged, and the repo's own rule is that
      a warning the user cannot see is not a warning. This is a *latent* exposure
      — no such element exists on this branch today — which is why it is filed as
      one box and not as a blocker on the shipped behaviour.
      **Severity: medium (latent).** Full measurement and the mechanism are in
      `findings/correctness.md`; this box and that one are the same defect and
      only one fix is needed — tick both when it lands.

## Not findings, recorded so the absence is legible

**A stale `apparatus:` on a sibling branch fails loudly, not silently, and I
measured both halves rather than reasoning about them.**

- **Assigning** the removed property is a hard failure. Instantiating
  `ScreenFrame { apparatus: [ … ] }` against this branch's `ScreenFrame` raises
  `Cannot assign to non-existent property "apparatus"` and the object is not
  created (`created=false`). Five branches carry such an assignment —
  `piece/ui-composer`, `piece/ui-onboarding`, `piece/ui-stoa-list`,
  `piece/thread-read`, `piece/theme-unshadow` and others at
  `FeedScreen.qml:446`. Each will fail at merge, visibly, which is the outcome to
  want. Note the CI "QML parses" gate cannot catch it — `apparatus: [...]` is
  syntactically valid, so `qmlformat` accepts it; qmllint's `missing-property`
  check (default severity `warning`, and the gate keys on exit code) is what
  fires.
- **Reading** it is silent: `screen.apparatus` evaluates to `undefined` with no
  error. That matters for `piece/ui-stoa-list`'s `tst_stoa_screens.qml:219-245`,
  whose `apparatusText()` reads exactly that property and whose `bodyText()` is
  defined by subtracting it. Post-merge `apparatusText()` returns `""`,
  `bodyText()` widens to the whole screen, and
  `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
  (`:1002`) short-circuits on its own `if (app !== "")` guard at `:1034` rather
  than failing. **That branch's author anticipated this** — the comment at
  `:1025-1033` says so explicitly — so the corpus guard degrades to inert rather
  than to wrong. It is the sibling piece's to resolve, not this one's, and it is
  recorded here only so the next reader does not rediscover it as an alarm.

**The `compose.apparatus` spec requirement on `piece/ui-composer` is genuinely
undischarged-but-harmless from this change's side.** I confirmed the string has no
implementation in any QML file on that branch and that `composer-view` is not
among the merged capabilities, so nothing this change does breaks a contract that
is in force. `design.md` §7 hands it off correctly.

## Conclusion

No security defect in the shipped behaviour of this change. The single box above
is a latent layout hazard on a shared shell, filed because the screens about to be
built on it carry text that must not silently disappear. There is no
peer-input, cryptographic, authorisation or moderation surface in this diff at
all.
