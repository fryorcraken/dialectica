# Design review — `e2e-successful-join`

Checked D1–D5 against the code, and design.md against issue #134 (with its two
owner comments), PR #185's body, and the related pieces (#164, #172, #181).

- **D1 (committed vs. generated reference).** The literal in
  `dialectica-ui/tests/ui/seeded-join.yaml` decodes and matches
  `seeded_reference.rs::seeded_record()`, and the three tests in that file
  (`the_committed_reference_is_the_seeded_record_at_its_own_address`,
  `the_preview_of_the_committed_reference_falls_back_to_the_seeded_title`,
  `the_core_joins_the_committed_reference_as_a_new_membership`) exercise
  exactly `Genesis::decode`, `wire::get_stoa` and `wire::join_stoa` as claimed.
  `tasks.md` 1.2 records the two mutations design.md cites (address digit
  changed, `VERSION_1` set to 2) with their observed red messages.
- **D2 (textual read).** `reference_in_spec()` reads `SPEC` at run time
  (not `include_str!`), takes the substring from the first `{"stoa":"` to
  the next `}`, and asserts exactly one match — matching the spec header's
  instruction to keep the reference the only one in the file.
- **D3 (separate file).** `seeded-join.yaml` is its own file and its own
  `ui-tests.yml` matrix entry (`[join, seeded-join, create, feed, thread,
  moderation]`); `join.yaml`'s "does NOT cover" paragraph now points at it.
- **D5 (seeder vs. seeding peer).** The departure from #134's "needs a
  seeding peer" wording is stated plainly, argued from the archived
  `e2e-created-stoa-flow` proposal, and the seeder is confined to
  `dialectica-core/tests/`, touching no Basecamp profile — consistent with
  the code.
- **Scope vs. #134.** The issue (read fresh) and its two owner comments match
  design.md's account exactly: #164 (join refusal), #172 (review/fixes, no
  new scope item), #181 (key/Stoa creation, feed, thread, moderation), and
  this piece (successful join) as the last item. Nothing on the issue's list
  is left undone by this account.
- **PR #185 body.** `Closes #134` is the only closing-keyword phrase; no
  other `close`/`fix`/`resolve` word precedes a `#N` elsewhere in the body.
- **Owner directions.** No Python added by this diff (the pre-existing
  `python3` calls in `ci.yml` are untouched by this piece). No Basecamp
  config touched. All six new commits (`1102393f`..`661ac18`) are unsigned
  (`%G?` = `N`), consistent with "unsigned commits for now". The suite stays
  under milestone 0.0.1 per the issue.

No contradictions found between the code and the recorded decisions, and no
undocumented decision worth recording turned up.

- [x] **none** — code matches design.md's D1–D5, design.md matches issue
      #134 and PR #185's body, and the owner's standing directions are
      honored.
