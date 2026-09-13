# Security findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `77edb82`, in a worktree of its own.

**This change has a small security surface and it is genuinely small.** Nothing
in `dialectica/` is touched, so no peer-input path, decoder, signature check or
moderation authorisation moves. The diff is an identifier rename plus comments,
a CI step, and documentation. Neither of this project's two standing rules —
never trust an inbound message, and moderation must be authenticated and
authorised — is engaged by a QML singleton's name.

**What was checked and is clean**, in prose rather than as boxes:

- **No peer-controlled data reaches anything new.** The rename touches no index,
  length, allocation or parse. `DTheme.qml` is `Theme.qml` with 34 comment lines
  prepended and no code change — verified by
  `git diff main:…/Theme.qml piece/theme-unshadow:…/DTheme.qml`.
- **The `textFormat: Text.PlainText` hardening survives the rename intact.**
  Every `Text` element that renders a potentially peer-supplied string still
  pins `Text.PlainText` — `SanitisedText.qml`, `MarginNote.qml` (including its
  comment explaining that `body` is a bound property a caller may one day fill
  with a peer string, so `AutoText` would sniff it), `PostHeader.qml`,
  `FlatButton.qml`, `AddressLabel.qml`. A rename that had disturbed one of these
  would be a rich-text injection surface; none is disturbed.
- **The identicon palette is unchanged**, so the wire-visible frozen constants
  that make an identity's mark recognisable across peers are byte-identical. A
  changed ink would have been a recognisability defect with an impersonation
  flavour to it; `tst_identicon.qml` still pins the ink indexing against the
  named roles and passes.
- **The CI step introduces no secret handling, no network access and no new
  dependency.** It is `grep` and `[ -e ]` in the existing `qml` job, under the
  workflow's `permissions: contents: read` posture, with no `pull_request_target`
  and no checkout of untrusted refs. It cannot be made to execute attacker
  content: every pattern is a literal, and the only interpolation is `$name`
  from a loop over five hardcoded identifiers.
- **No error message leaks anything.** The step's `::error` strings name our own
  file paths and type names, all of which are public in the repository.
- **No comparison of secret material** appears anywhere in the diff.

The one item below is a hardening observation about the gate's durability rather
than an exploitable defect, and it is the only thing I would ask anyone to act
on in this lane.

---

- [x] **`dev-writer`** — `.github/workflows/ci.yml:611` — the host-name list is
      hand-maintained with no mechanism to notice it has gone stale, which is the
      failure mode this repo has already recorded
      **Scenario:** the arm bans exactly `Theme Style Palette Colors Typography`.
      The step's own comment says to "add to it whenever a launch log shows one
      of ours resolving to a `qrc:/qt/qml/Logos/...` path" — i.e. the list is
      only correct until basecamp registers a sixth name, and nothing in CI can
      detect that. Adding a `Button.qml` singleton would pass every gate while
      being exactly the defect this piece fixes: the verified launch log for this
      change reports the host registering `LogosButton.qml` in
      `qrc:/qt/qml/Logos/`, so the namespace demonstrably holds component-shaped
      names beyond the five listed.
      This is the `hand-maintained sweep lists go stale silently` pattern the
      project has been bitten by before, and the honest mitigation is not a
      longer list: it is that the **prefix convention** is the actual defence
      (`design.md` says so — "a name nothing in the host can claim cannot be
      shadowed by anything basecamp adds later"), and the gate should enforce
      *that* rather than enumerate collisions. A check that every `singleton`
      line in `qmldir` declares a `D`-prefixed type is total over future host
      registrations, where the list is total over none of them.
      **Severity: low as shipped** — no current collision, and the piece's own
      fix is correct. It is filed because the gate is presented as the thing
      preventing recurrence, and in its current shape it prevents recurrence only
      of the five names someone already knew about.

      **FIXED, and adopted as the gate's organising idea rather than patched** —
      `tasks.md` 4.7. The five-name list is gone. The gate now requires every
      `singleton` line in `qmldir` to declare a `D`-prefixed type, which is the
      check you argued for: total over host registrations that have not happened,
      where the list was total over none of them.
      *Mutation that survives without it:* declaring `singleton Button 1.0
      Button.qml` — passes the old arm (not among the five) and fails the new one
      with a message naming `DButton`. Your `LogosButton.qml` observation from
      the launch log is what makes that mutation the realistic one rather than a
      contrived one.
      **One thing your entry did not anticipate, and it shaped the fix:** `Core`
      is a singleton this piece deliberately does not rename, so a bare prefix
      rule fails on the tree it ships with. I caught that by running the rule
      before writing the outcome here. Rather than drop the rule, `Core` is
      grandfathered **with its reason in the step**, and the comment says
      explicitly that it is a grandfather clause and not a precedent — that
      "basecamp has no `Core` today" is exactly the kind of fact the prefix rule
      exists to stop depending on, that `DCore` is the right end state, and that
      the fix for a second such name is the `D` rather than a second exemption.
      That keeps one name in a set, which is weaker than zero; but it is a set
      whose growth is called out as the wrong move at the point someone would
      make it, where the old list invited growth as its intended use.
      The prefix rule also subsumes part of the correctness lane's column-0
      finding: a bare `Theme` is by construction not `D`-prefixed.
