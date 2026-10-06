## Re-review round 12 `4b2aeca0..e4df0727`

Read: `git show --stat e4df0727` (the design, proposal and tasks moves are pure
renames, no content change), the whole spec promotion diff (op-transport, stoa-membership,
composer-view), the deleted `findings/design-review.md` outcomes, the live
`op-transport` Purpose and the requirements the new text cross-references, the live
`content-authoring` publish requirement, #189's spec tidy (it touched no
op-transport, stoa-membership or content-authoring text), and `gh pr view 190`.

The durable reasoning in the deleted design-review outcomes (the wait-leaves-the-book
entry, the 0.75 allowance, Decision 17's rule, sources and pin test) was already in
the archived `design.md` before the deletion; nothing found only in `findings/`.
The archived `design.md` and `proposal.md` carry no `findings/` reference and no
path under `openspec/changes/delivery-wiring/`; the only repo references to the
change (`CLAUDE.md`, `docs/SCAFFOLD.md`) name it, not a path. The promoted
requirements agree with the design's node-creation, start-after-accept and
channel-open decisions, and the cross-referenced requirement names exist in the
live specs. No promoted text says delivery is unwired. `Closes #176` is the only
closing keyword beside an issue number in the PR body. One defect, in the PR body.

- [ ] **`closer`** — PR #190's body (the "What" section's last paragraph) says
      "`openspec/changes/delivery-wiring/design.md` records each decision". That path
      no longer exists: the archive commit moved the folder to
      `openspec/changes/archive/2026-10-06-delivery-wiring/`. The body is what a
      reader of the merged PR follows to the decisions, so it points at nothing.
      Edit the body to the archived path (no closing keyword is involved).
      **Verified:** `gh pr view 190` at `4ffd8886`; `git grep -n -F
      "changes/delivery-wiring"` over the tree finds nothing, so the body is the
      only place left carrying the old path.
