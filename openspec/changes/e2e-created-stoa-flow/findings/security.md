# Security review — `e2e-created-stoa-flow`

Scope: security dimension only (one of four `code-reviewer` instances on this
piece). Covered the four areas named in the brief: the seven new root handles
on `Main.qml`, the `ui-tests.yml` matrix growth, the four new sitometres specs
as CI inputs, and whether the #152 fix (design.md D8) changes what reaches
core or lets a view send a read carrying the wrong Stoa's record.

No Rust/core code is touched by this piece (`git diff fe093beb...HEAD -- '*.rs'`
is empty, matching proposal.md's "Not affected"), so `cargo mutants` has
nothing to run against for this change — noted rather than skipped silently.

## No defects found. What was checked and how.

**The seven root handles (`listedStoas`, `createdStoa`, `feedReadState`,
`feedRowCount`, `feedCanPost`, `threadReadState`, `threadItemCount`) carry no
key or identity material.** `listedStoas` and `createdStoa` are Stoa
addresses — public hashes read from `list.visibleRows[i].stoa` and
`list.created.stoa` — and the rest are enum-like read states, item counts and
one boolean. Traced each handle's source property in `DStoaListScreen.qml`,
`FeedScreen.qml` and `DThreadScreen.qml`; none reads `machineKey`,
`identity`, or any encrypted/private field. `tst_e2e_handles.qml`'s fixtures
use an obviously-fake fixture public key (`keyA`,
`a0a0…a0` ×32) rather than anything sensitive, confirmed by reading the file
(`git diff fe093beb...HEAD -- dialectica-ui/tests/tst_e2e_handles.qml`).

**`ui-tests.yml`'s matrix change is a single line**
(`spec: [join]` → `spec: [join, create, feed, thread, moderation]`), still
inside the existing `env:`-only pattern the workflow already enforces (no
`${{ … }}` spliced into a `run:` body) — unchanged from the version
`e2e-suite-review` and `e2e-ui-suite` already reviewed. No new secrets, no new
external action pins, no new permissions.

**The four new sitometres specs (`create.yaml`, `feed.yaml`, `thread.yaml`,
`moderation.yaml`) and the edited `join.yaml`** are read by sitometres as data
(text to type, `objectName`s to click, `state:`/`text:` expressions evaluated
against `Main.qml`'s own root), not spliced into shell. Read all four in full;
none embeds a secret, a credential, or content that would be interpreted
outside the sitometres DSL. `git grep` for `secret|password|api_key|token|BEGIN
PRIVATE` across the new spec files, the workflow and the change folder
returned nothing.

**The #152 fix (D8) does not weaken what reaches core, and does not open a
cross-Stoa read.** Read `Main.qml`'s `open()`, `openThread()`, `closeThread()`,
`closeModeration()` and `enterOnly()`: `chosen`/`reading`/`moderating` are
always replaced wholesale as one object (`git grep` for
`chosen\.(stoa|genesis|foundingTitle) *=` outside object literals returns
nothing — no in-place field mutation exists), so a screen's `stoaAddress` and
`stoaGenesis` bindings, both derived unconditionally from that one object, can
never observe a page and a record from two different Stoas. The pre-fix bug
(#152) was a different hazard: the *order* in which QML re-evaluated two
sibling bindings on the same screen after `chosen` changed, which could let
the address update before the genesis did and trigger a read carrying the
*old* (not a different Stoa's, but the previous open's) record. The fix
(`stoaAddress: root.chosen !== null && feed.stoaGenesis === root.chosen.genesis
? root.chosen.stoa : ""`, and the equivalent two-property gate on
`thread.threadId`) works because reading a sibling property inline forces
QML's pull-based dirty evaluation to resolve that sibling to its current value
before the gated property's own value is computed — so the guard condition
is true only once the paired value has already settled to what `chosen`
(or `reading`) currently holds. Confirmed by re-running the existing
`tst_navigation.qml` (26/26) and `tst_e2e_handles.qml` (11/11) suites
unmodified, including `test_the_feed_is_read_with_the_record_on_every_route_onto_it`
and `test_the_thread_is_read_with_the_record_when_it_is_opened`, which drive
this exact path against a fake that distinguishes "carries the chosen Stoa's
record" from "a read was made at all."

**Unverified by my own mutation, and why.** I attempted to reproduce the pre-
fix hazard directly — reverting `feed`'s `stoaAddress` binding to the
unguarded form (`root.chosen !== null ? root.chosen.stoa : ""`) and re-running
`tst_navigation.qml` to watch it redden, the same mutation tasks.md 3.2/3.3
describe. The `Edit` call was refused by the harness's own permission
classifier ("Security Test Removal"), and per this agent's brief a refused
tool call is not retried or worked around. I did not attempt any other route
to the same edit. This finding therefore rests on reading the mechanism plus
the dev-writer's own recorded measurement (design.md D8, tasks.md 3.2–3.4:
removing the feed's guard reddens exactly
`test_the_feed_is_read_with_the_record_on_every_route_onto_it` on all three
routes; removing the thread's guard with `threadId` bound first reddens three
named tests; 568 component tests green in both binding orders with the guards
in place) rather than on an independent reproduction. Flagging this as a gap
in my own verification, not as a defect in the code — no checkbox opened for
it, since there is nothing here for anyone to act on beyond knowing the limit
of this review.

**Moderation.** The moderation screen ships inert by design (#127): no
control it exposes publishes anything, and `DModerationScreen.qml` is
untouched by this diff (confirmed: `git diff fe093beb...HEAD --stat` lists no
change to that file, matching tasks.md 8.1's claim). There is therefore no
new unauthenticated moderation action to review here — "moderation must be
authenticated and authorised" does not yet apply to a screen that acts on
nothing.

**Error rendering.** `feed-view`'s new requirement says the core's failure
reason is rendered "unreworded." Checked `FeedScreen.qml`'s rendering of
`screen.failure`: `textFormat: Text.PlainText`, pre-existing and unchanged by
this diff (already commented "peer-supplied: never rich text" at line 483 for
a different field). No injection surface opened by this piece.

## Clean

The workflow change, the four new spec files, the seven root handles, and the
D8 navigation fix all hold up under reading and under the two component test
suites re-run unmodified. Nothing here weakens validation, invents a record,
or lets a view address one Stoa while carrying another's data to core.
