# Security review: reply-caption-no-edit

- [x] **`none`** — no security findings; verdict box so the file has one.
      Dimension covered: security only.

      The code diff is one string literal cut in `DThreadScreen.qml` (the reply
      caption loses "It can be edited later.") plus an `objectName:
      "replyCaption"` on that `Text`. Neither touches an inbound path, a length,
      an index, an allocation, a signature check or a moderation action. The
      remaining caption, "A reply is a signed record.", is a true statement of
      `op-format` and is not a claim of authority. `objectName` exposes nothing
      beyond the QML object tree, which is already reachable by the view's own
      tests. The shut-gate text rendered from `capability.reason` (line 768) is
      core's own reason, rendered verbatim, unchanged by this diff, and
      untrusted peer content never reaches it. The new tests only read
      `Text.text` and drive a fake bridge, so they add no runtime surface.

      Clean areas: inbound validation, panic reachability, secret comparison,
      error-message leakage, new dependencies (none), CI gates (no file moved).
      Not exercised: no mutation run, since the diff has no Rust and `cargo
      mutants` cannot see QML.
