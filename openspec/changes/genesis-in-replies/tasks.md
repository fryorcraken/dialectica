# Tasks

## Stages

This block was added late, by the `dev-writer` acting on review findings, and
that is a defect in how this piece ran rather than a property of the piece: the
change folder was created directly instead of by dispatching a `spec-writer`, so
no roster ever existed. Four reviewers each reported having no row to tick and
correctly declined to invent one — recorded in `findings/correctness.md` and
`findings/spec-test.md`. The rows below are the roster from
`.claude/agents/spec-writer.md`.

**The four review rows are ticked on work that ran before this block existed.**
Each is ticked against its findings file, which is the evidence the review
happened; none is ticked on anybody's recollection.

- [x] spec — `spec-writer` (ran as part of the change folder's creation; the
  delta is `specs/stoa-membership/spec.md`)
- [x] design + code — `dev-writer` (`design.md` was written on the findings pass,
  not the first pass — see `findings/design-review.md`)
- [ ] ~~tests — `tester`~~ — struck: no separate `tester` was dispatched. The
  tests were written by the `dev-writer` alongside the code, three of them proved
  to fail first, and `spec-test-reviewer` reviewed them independently and found
  the vacuous guard this pass repaired. Struck rather than ticked because the
  stage genuinely did not run as its own dispatch.
- [x] review: correctness — `code-reviewer` (`findings/correctness.md`)
- [x] review: security — `code-reviewer` (`findings/security.md`)
- [x] review: readability — `code-reviewer` (`findings/readability-architecture.md`)
- [x] review: architecture — `code-reviewer` (`findings/readability-architecture.md`)
- [x] review: spec-test — `spec-test-reviewer` (`findings/spec-test.md`)
- [x] review: design — `design-reviewer` (`findings/design-review.md`)
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Core: the regression tests, written first

- [x] 1.1 `genesis_of`, a helper that decodes a reply's record and asserts its
  address equals the address the SAME reply names. The relation rather than a
  pinned literal: a reply carrying a well-formed record of some other Stoa
  satisfies "the field decodes" and is useless to the caller.
- [x] 1.2 A creation reports the record its address is the hash of. **Proved to
  fail first** — `the reply must carry a 'genesis' record…: {"foundingTitle":
  "Agora","policy":"open","stoa":"80329cf0…"}`.
- [x] 1.3 A listed Stoa carries it too. Proved to fail first, same message
  against the listing's item shape.
- [x] 1.4 The record a listing reports round-trips through the call that takes
  one. This is what pins both sides to ONE encoding — a reply carrying base64
  would pass 1.2 and 1.3 and still be refused by the handler that reads it.
  Proved to fail first (`Option::unwrap()` on a `None`).

## 2. Core: the widening

- [x] 2.1 `GENESIS` constant, documented with why an address cannot substitute
  for a record and why the encoding must match what `join_stoa` reads.
- [x] 2.2 `stoa_reply` carries it — this serves BOTH create and join, which is
  why the constant's docstring states the reason is a property of neither call.
- [x] 2.3 `membership_page_json` carries it per item. Rewritten from a `map`
  closure to a loop so an encode failure can return the error shape; a closure
  could only have swallowed it or panicked.
- [x] 2.4 An encode failure is the error shape, never an empty field. An empty
  record is exactly the input that fails to decode as "ended mid-field", so
  defaulting to one would reintroduce the defect as a success reply.
- [x] 2.5 The two handler docstrings' reply shapes updated to match.
- [x] 2.6 A Stoa is openable from the listing **after a restart** — a real file
  and a genuinely reopened store, so the record is re-decoded from
  `genesis_bytes` rather than being the one this process built. The in-memory
  fixtures cannot reach this scenario, and it is the one the owner lives in.
- [x] 2.7 Full suite green: 1052 tests, 0 failed. `cargo fmt --check` clean on
  both crates — with `-p dialectica -p dialectica-core`, which is what reaches
  `dialectica-core` at all.

## 3. The view

- [x] 3.1 `rememberGenesis(stoa, genesis)`, which records a non-empty string and
  otherwise leaves the map untouched. Writing `""` in would make `canShare` true
  for a Stoa whose share text cannot be built and would send that same `""` back
  to `read_feed`.
- [x] 3.2 The map is REASSIGNED rather than mutated in place. A QML `var`
  property does not notify on an in-place key write, so bindings on `canShare`
  would not re-evaluate and a share button would stay hidden.
- [x] 3.3 `reload()` records every item's record, additively, before the rows
  render — so paging away from a Stoa does not drop its record.
- [x] 3.4 `create()` records the creation reply's own record rather than leaving
  it to the reload: a new Stoa need not land on page 0.
- [x] 3.5 The stale comment naming this as unfixable in the view, and the one
  saying a creation reply carries no record, replaced with what is now true.

## 4. The view's tests

- [x] 4.1 A listed Stoa is openable from the reply alone. **Nothing is seeded** —
  the older tests in this file write `genesisByStoa` directly, which is exactly
  why the defect survived them: seeding the map assumes the missing thing.
- [x] 4.2 A created Stoa is openable and shareable from the creation reply alone.
- [x] 4.3 A record survives paging away from the Stoa that carried it.
- [x] 4.4 An item short of its record is not recorded as an empty one, and the
  listing is still a successful read. The guard direction.
- [x] 4.5 **4.1, 4.2 and 4.3 proved to fail** with the view change set aside —
  each on the empty string, e.g. `Actual (): ` vs `Expected (): 01cccc…74657374`.
  ~~4.4 holds in both directions by design; it pins that nothing writes `""`.~~
  **That claim was wrong, and three reviewers disproved it independently.** 4.4
  asserted only through `genesisFor`, which collapses "no key" and "key holding
  `\"\"`" to the same `""` — so removing the `|| genesis === ""` half of the
  guard left 81/81 green. Repaired on the findings pass: 4.4 now asserts
  `!genesisByStoa.hasOwnProperty(...)` for both rows, and the same mutation now
  fails it by name (80/81, red on "an empty field must leave no key either").
  See `design.md`, "The guard against an empty record is asserted against the
  map's keys".
- [x] 4.6 Full QML suite green: 411 passed, 0 failed across 20 spec files. The
  three static gates green — names, members, reachability.

## 5. Proof by launching

- [x] 5.1 `lgs basecamp build --variant lgx` — both `.lgx` artefacts built, so
  the `interface: "universal"` dispatch table still derives from the changed
  wire shape. Then `setup` (this worktree had no `.scaffold/`), `install` into
  both profiles, and `launch alice`.
- [x] 5.2 `git diff scaffold.toml` after every verb. `build` and `install` left
  it untouched; **`setup` stripped the 3-line header comment** and changed no
  value — restored with `git checkout scaffold.toml`.
- [x] 5.3 The launch is healthy and the shipped artefacts carry the fix: the
  installed plugin's own `DStoaListScreen.qml` holds all three `rememberGenesis`
  call sites, and the log shows **zero** `MODULE_NOT_LOADED`, `ReferenceError`,
  `Unable to assign` or `is not a type` lines.
- [ ] 5.4 **NOT PROVEN: the click.** Basecamp is running against this worktree
  with only the host's own modules loaded — `dialectica_ui/qml` appears **0**
  times in the launch log, because loading the plugin needs a human to click the
  dialectica tile and an agent cannot. So "create a Stoa, press Open, see a feed"
  is **unverified**, and this row is deliberately left unticked rather than
  ticked on the strength of the suite.

  What IS established, and where it stops: the reply now carries the record
  (1052 Rust tests, including one against a real reopened store), the view fills
  its map from the reply (411 QML tests, three of them proved to fail without the
  change), and the adapter and the QML bridge both forward the reply verbatim —
  read and confirmed, so no narrowing sits between core and the view. The
  remaining gap is the rendering itself, which needs the click.

## 6. Acting on the review findings

- [x] 6.1 The vacuous guard repaired. 4.4 now asserts against
  `genesisByStoa.hasOwnProperty(...)` rather than through `genesisFor`, whose
  lossy projection is what made the test pass either way. **Proved by mutation**:
  removing the `|| genesis === ""` half of `rememberGenesis`'s guard takes the
  suite to 80/81 with this test red by name, where it was 81/81 green before.
  Mutation reverted.
- [x] 6.2 The uncovered spec scenario covered — "A record that cannot be encoded
  is a failure, not an empty field" now has
  `a_record_that_cannot_be_encoded_is_a_failure_rather_than_an_empty_field`.
  **Proved to fail first** against `unwrap_or_default()` in `stoa_reply`, whose
  output is the forbidden shape verbatim: `{"foundingTitle":"xxx…","genesis":"",
  "policy":"open","stoa":"0000…"}`. It asserts the failure shape AND the absence
  of a success shape, so a partial success cannot satisfy it.
- [x] 6.3 The spec's positional cross-reference replaced with the requirement's
  name ("A joined Stoa's genesis record is retained, not only its address"),
  since archive splice position is not fixed for an existing capability.
- [x] 6.4 `design.md` written, carrying the three implementation decisions and
  the **refused codec-bug diagnosis**, which existed only in the PR body and
  would not have survived into `changes/archive/`.
- [x] 6.5 The stage block added, and 4.5's disproved "holds in both directions"
  claim struck through rather than quietly corrected.
