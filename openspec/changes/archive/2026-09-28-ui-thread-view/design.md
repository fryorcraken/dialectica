# Design — the thread screen

## What this builds

`DThreadScreen.qml`, a `readThread` binding in `Core.qml`, and the route into and
out of the screen in `Main.qml`. The screen renders one page of a thread read:
the root, its replies nested by the parent chain, a wired reply composer and one
inert affordance.

The view half only. `thread-read` decides what comes back and `composer-view`
decides what the composer does once a reply is submitted; neither is
re-implemented here.

## The wire contract this is written against

Measured from `wire.rs`, not taken from a document. A wrong field name renders
blank silently, so the names are recorded here with their citations.

The request `read_thread` takes (`wire.rs:1683-1728`):

```
{ stoa, genesis, thread, page, perPage, includeHidden }
```

The reply envelope (`wire.rs:1916-1920`): `{ items, page, hasMore }`.

Per item (`wire.rs:1835-1911`):

| Field | Note |
|---|---|
| `thread`, `id`, `currentVersion` | `id` is stable across revisions; `currentVersion` moves |
| `author` | the public key's hex — never an address, never a name |
| `isRevised` | the `edited` label's input |
| `position` | **an opaque ordering token, a string** |
| `moderation` | `{ state, decidedBy }` — `state` is `unmoderated` / `hidden` / `unhidden` |
| `parent` | **omitted on the root**, present on every other item |
| `body` | **omitted where withheld**, `{text,removed,marked}` otherwise |
| `attachments` | omitted by the same rule as `body` |
| `assertedTime` | omitted where the op predates the clock fields |

Three of those are absent rather than null, deliberately (`wire.rs:1816-1828`),
and the code below reads absence as the fact it is.

## Decisions

### D1 — An unresolvable parent is rendered as answering a post not shown, never re-parented to the root

The requirement this change defends hardest, and the one where the obvious
implementation is wrong.

The tempting shape is `depth = parentDepth(item) + 1`, defaulting a parent the
page does not hold to the root's depth — which renders the item as a direct reply
to the root. That undoes, at the render step, the property `thread.rs` exists to
establish. Its module header states the attack (`thread.rs:16-23`): a peer
publishes an authentically signed post — its own key, its own bytes, verifying
perfectly — whose `thread` field names somebody else's thread. Core answers by
deriving membership from the parent chain and never reading the claimed field,
and by returning a post whose chain cannot be completed **under no thread rather
than placed by its claim** (`thread.rs:44-50`).

A view that re-parents such an item to the root gives the attacker the placement
core denied, with a signature and a verification behind it and nothing on screen
marking it out of place.

So `resolveDepth` returns `-1` for an item whose reported parent is not among the
items held, and `-1` is rendered as a distinct state: outermost indent, plus a
line saying the item answers a post not shown. It is never folded into depth 0.

**What breaks without this**: `test_an_item_whose_parent_is_absent_is_not_a_reply_to_the_root`
and `test_an_unresolvable_parent_is_marked_as_answering_something_not_shown` both
go red. Deleting the `-1` branch and defaulting to 0 turns both, and nothing else
in the suite notices — which is why the guard is named here rather than left to
read as a stylistic choice about indentation.

The root is **not** a missing-parent case. The root is the item reporting no
parent at all, and `wire.rs:1864` omits `parent` for exactly it. The two are
distinguished by `parent` absent (the root) versus `parent` present but unmatched
(unresolvable), which is why the wire's omit-rather-than-null rule is
load-bearing here rather than a detail.

**This paragraph described the intent, and the first implementation did not honour
it** — `parentOf` collapsed a present-but-unusable `parent` to the same `""` the
root reports, so the distinction it names existed only in the prose. D13 records
the second route that opened, and the reshaping that closed it.

### D2 — Depth is computed by walking the parent chain with a visited set, bounded for rendering only

`resolveDepth` walks `parent` links through a map built from the page's items,
carrying a visited set. The set is what makes the walk terminate over
peer-supplied data: a self-parenting item and a cycle of any length both reach an
op already visited. That mirrors `thread.rs:30-42`, which chose a visited set
over a depth limit for the same reason and records that a module which stops
answering has to be restarted.

A cycle yields `-1` — the same state as an absent parent, and honestly so: an
item in a parent cycle has no establishable depth, which is exactly what `-1`
means. Rejected: treating a cycle as depth 0, which would re-create D1's defect
by a second route.

The **indent** is bounded separately, at `maxIndentDepth = 6`. The bound clamps
the pixels only: it never changes the computed depth, never changes which item is
reported as a parent, never omits an item and never touches the sequence. Two
different numbers doing two different jobs, rather than one limit doing both —
a truncating walk would make a deep thread stop being readable at a line nobody
could derive, which is the failure `thread.rs` names.

### D3 — The sequence is the model, and nesting is a rendered property of each row

`Repeater { model: screen.items }` renders the returned array in order.
Nesting is an `indent` on each delegate, computed from the parent chain, and
changes no element's place. There is no tree construction, no grouping, no sort.

This is what makes "the view computes no order of its own" structural rather
than a rule someone has to keep: there is no code path that could reorder,
because the model is the array core returned and nothing wraps it.

`position` is therefore read by nothing. It is an opaque token
(`thread.rs:200-206`) and the view has no use for it — the sequence already
carries the order. Not parsing it is the whole of the requirement, and the way to
not parse a value is to not read it.

### D4 — A withheld body and an empty body are two different renderings, distinguished by absence

`body` is **omitted** from the item where it is withheld (`wire.rs:1867-1869`),
and present as `{"text":""}` where the author cleared it. `thread.rs:166-174`
records that this is why `ThreadItem::body` is an `Option`: "withheld and empty
are different facts, and this makes them different values rather than one value a
caller has to disambiguate correctly at every call site."

The view preserves the distinction by testing for the field's presence:

- `body === undefined` → a withheld notice, rendered as a moderation outcome.
- `body` present → `SanitisedText`, which renders `{text,removed,marked}` and
  renders an empty text as empty.

The failure this prevents is the distinction surviving core, the wire and the
projection, and being destroyed in the last component that touches it. A
`body.text || ""` would do exactly that — one falsy test collapsing both facts to
a blank — which is why the presence test is written against the field rather than
against its contents.

### D5 — One composer at the thread's foot, not one per post

The bundle's screen 06 places a single composer under the thread, behind a
`REPLYING AS` identity line, and offers no per-post reply control. The screen is
built that way.

It follows that the composer's parent is the **root**, and the spec's requirement
that a reply names "the post whose reply affordance was acted on" is satisfied
because the only affordance offered is the thread's own. The requirement's
prohibition — that a reply to a non-root must not be sent naming the root — is
satisfied by construction: there is no affordance on a non-root post, so the
request that would misname a parent cannot be constructed.

**This is satisfied-by-construction rather than tested-by-assertion**, and the
distinction matters for whoever reads the task list: no test can exercise "a
reply to a reply names that reply", because the interface offers no way to make
one. A per-post affordance is the piece that makes that requirement testable, and
it is not this one.

`parentOp` is guarded anyway. Where the root carries no usable `id`, the composer
is not rendered at all — the spec's "no reply call is made for a post the view
holds no op id for", held where the value is produced rather than at the call.

### D6 — The earlier-versions affordance is inert, the composer is wired

Two affordances, two answers, and the difference is the contract rather than
effort.

`publish_reply` exists (`lib.rs`), so the composer calls it. An unwired composer
would be a defect under the owner's merged ruling.

No contract method reads a prior version — `revision.rs` keeps superseded
versions in the op log and nothing exposes them — so "read the earlier versions"
is rendered present and offering no action, under `composer-view`'s existing
inertness convention. `docs/PLAN.md` §9.2 case 2 entry 4 already documents this
placeholder, which is what makes it acceptable rather than a dead control.

It is rendered as static text rather than as a `FlatButton` with an empty
handler. A button that depresses and does nothing reads as broken; text that
states the position reads as a statement. **It reaches no call**, which is the
requirement, and there is no handler to reach one from.

### D7 — The moderation shape diverges between the feed and the thread read, and this view pays for it

*Migrated from `docs/PLAN.md` §9.1 "What a thread view is", where it was live
prose. It is a live gap, not a settled one.*

The feed reports moderation as a boolean and the thread read reports the
three-valued object. Both are built from the same resolver, so a view must
currently branch on which call produced an item — which §2.5's "JSON shapes are
source-independent" forbids. The thread read's shape is the correct one; the
feed's is the older. Until the feed is brought to it, `restored` is a state the
feed cannot express at all.

Measured on this tree: `wire.rs:1575` emits `"isHidden": row.is_hidden` for a
feed row, and `wire.rs:1849` emits `"moderation": { "state": state }` for a
thread item. So `FeedScreen.qml` reads `modelData.isHidden === true` and this
screen reads `moderation.state === "hidden"`, and the two cannot share a
component — which is the divergence costing something, at the first screen that
had to render the other shape.

This view does not paper over it. It reads the three-valued state as the contract
gives it, and renders `unhidden` as distinct from `unmoderated` where the feed
could not. Closing the gap means bringing the feed to the thread's shape, and
that is a core change this piece does not make.

### D8 — The view renders what core sanitised and re-sanitises nothing

*Migrated from `docs/PLAN.md` §9.1, the bidi paragraph insofar as it reaches post
bodies.*

The bidi obligation is wider than §11.1 states it. §11.1 frames Unicode and bidi
rendering around *metadata titles*, because that is where it was found: the
metadata op deliberately does not sanitise, since normalising would break op-id
agreement between peers. The same reasoning applies unchanged to every
attacker-supplied string a thread screen renders — post bodies above all, which
are the largest and least constrained of them, and also the abbreviated public
key rendered beside an author. `op.rs` preserves display text exactly and never
normalises it, by design and with a test pinning that; so the obligation follows
the text everywhere it goes, not only to the field where it was first noticed.

What that means for this screen: every peer-supplied string goes through
`SanitisedText`, which core already sanitised, and which renders `textFormat:
Text.PlainText` with no property to change it. The view applies no second
transformation. A second sanitiser here would be a second implementation of a
judgement already made, and the two would disagree with nothing observing it.

`removed` and `marked` counts are rendered as chips rather than applied — a
marked character stays in the text, because replacing it would turn a deceptive
string into a plausible one.

### D9 — The screen is a fourth nullable property on the navigator, not a StackView

`Main.qml`'s invariant is that the navigator's whole state is which property is
non-null. A fourth screen holds to it: `reading` joins `chosen` and `previewing`,
and `screenShown` gains one arm.

`reading` is set only through `openThread()`.

**This piece proposed that `openThread()` leave `chosen` set** — the feed staying
live underneath, so leaving the thread returned to the feed the user opened it
from with its page and its scroll intact, on the reasoning that `preview` and
`open` are alternatives to each other where a thread is a screen *above* a feed.
**That mechanism was superseded by the merge and is not what ships**, along with
the `closeThread()` that followed from it. Navigation's version clears `chosen`
and rebuilds the feed from what `reading` carries, and it is the one in the tree;
see *What the merge took from each side* below for why it won. The paragraph is
kept rather than deleted because the rejected alternative is the part worth
reading — a later change reaching for "keep the feed alive underneath" should
know it was proposed, and what displaced it.

What ships, and what the rest of this section is written against:
`openThread()` clears `chosen` and carries the whole feed context on `reading`;
`closeThread()` rebuilds `chosen` from it. One state, one property — the
invariant holds because no two of the navigator's properties are ever both set,
which is what makes `screenShown`'s ternary a rendering of the state rather than
a resolution of a conflict (`Main.qml:64-101`).

**The transition is `piece/ui-navigation`'s, and this piece owns the
destination.** `view-navigation`'s *A thread is opened from a feed row and can be
left* contracts what travels across the transition; this change implements a
route so the screen is reachable at all. Where the two pieces disagree on the
mechanism, the navigation piece's spec governs — recorded here because both
touch `Main.qml` concurrently and the collision is foreseeable rather than
discovered.

**That collision arrived, and this is how it was settled.** `piece/ui-navigation`
merged to `main` as `cc37f2e` carrying its own `DThreadScreen.qml`, written
independently against a relaxed scope rather than against `thread-view`. The
rebase produced 27 conflict regions across `DThreadScreen.qml`, `Main.qml` and
`FeedScreen.qml`, resolved by the split D9 states: **the transition is
navigation's, the destination is this piece's.** See *What the merge took from
each side* below for the file-by-file record, including the two places where
navigation's version was kept over this piece's.

### D10 — A feed row's thread is opened by `thread`, and BOTH pieces got the field wrong

The merge's one genuine defect, found only because two independent
implementations disagreed and the disagreement had to be adjudicated against the
wire rather than against either author's intent.

Each piece wrote its own guard resolving which op id a feed row opens:

| Piece | Field read | Why it is wrong |
|---|---|---|
| `ui-navigation` | `currentVersion` | Present on the row, but it is the VERSION id. `feed.rs:135-140`: "different from `thread` the moment the post has been edited", and it is the field a *moderator* acts on. A route built on it points at an identifier that moves the moment anyone revises the root, and it fails silently — the link simply stops resolving. |
| `ui-thread-view` | `id` | **Not a field of a feed row at all.** `id` is what a THREAD item carries (`wire.rs:1837`). Reading it here yields `undefined` on every row core sends, so the "read the thread" link would be invisible on the entire feed — against live core, the thread screen would be unreachable. |

The correct field is **`thread`**, and it is measured rather than argued.
`wire.rs:6015-6027` pins the feed row's whole key set as a SET — `attachments,
author, body, currentVersion, isHidden, isRevised, thread` — so that an added
field fails the test as well as a removed one. There is no `id` on that row and
never has been. `FeedRow::thread` is documented at `feed.rs:130-134` as "the
thread's id, which is the root post's op id" and "**never changes across
edits**, which is what makes it the thing a reply names as its parent and the
thing a view uses as a stable row key" — exactly the identifier
`view-navigation` requires to travel.

**What breaks without this**: `test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`.
Its fixture was strengthened as part of this fix and the strengthening is the
load-bearing half — see below.

**Why no test caught either version.** The fixture in main's
`tst_navigation.qml` supplied `currentVersion` alone. With one identifier
present, a route built on `currentVersion` and a route built on the root's
stable id are indistinguishable: both read the same string, and the test passes
either way. That is this project's recorded defect family — a fixture where two
explanations give the same answer. The fixture now carries `thread` and
`currentVersion` with **different** values, modelling a revised post, so the
assertion discriminates: opening by the version fails it rather than passing by
coincidence, and a new `verify(args.indexOf(editedVersion) < 0)` names the wrong
answer explicitly rather than only asserting the right one.

Changing a test that arrived with `main` is worth flagging, and the
justification is that the fixture encoded a reply core cannot send: a row short
of `thread` violates the key set `wire.rs` pins. The assertion was not weakened
to accommodate the code — it was strengthened, and the code was changed to meet
it.

### D11 — The empty-`rootOp` guard is held at the transition as well as at the affordance

`openThread()` returns without setting `reading` when `rootOp` is `""`, and this
is deliberately the *second* place that judgement is made: `FeedScreen`'s
`threadTarget()` already withholds the link from a row naming no op
(`visible: target !== ""`).

**It is currently unreachable through the live UI, and it is kept anyway.** That
is the honest description — reachability today is a property of the one caller
that exists, not of the function. `thread-view`'s *No thread is rendered before
one has been chosen* is a requirement about the navigator, and a guard living
only in the caller is one every future caller has to remember. CLAUDE.md's rule
is the one being applied: a guard is a job, and "is it applied everywhere?" stays
a question with an answer only where the answer does not depend on each new call
site.

**What breaks without it**: nothing in the suite today, and saying otherwise
would be the false claim worth avoiding. What it prevents is a second caller —
a deep link, a notification, a restored session — opening the thread screen on
the empty string, which then asks core to read a thread identified by `""`. The
cost of keeping it is one comparison; the cost of removing it is discovered by
whoever adds that caller.

### D12 — One probe normalisation, on `Core`, rather than a copy per screen

`capabilityFrom` and `identityFrom` turn a probe reply into a fixed shape with
every field present. They were byte-for-byte identical on `FeedScreen.qml` and
`DThreadScreen.qml`.

They now live on `Core`, which is where the reply is produced: `call()`
normalises the envelope, and these normalise the two probe answers, beside it.
Both screens delegate.

Rejected: leaving the copies with a comment saying they must agree. The failure
is not that two copies exist, it is that **nothing can observe them diverging** —
each copy is internally consistent, so a fix to the `=== true` strictness applied
to one and not the other leaves the unfixed screen claiming an identity the
machine does not have, with every gate green. A comment is a rule someone has to
remember; the shared function is one the data enforces.

Also rejected: a small imported JS module. `Core` is already imported by both
screens and is already the boundary the reply crosses, so a second mechanism
would add a file without adding an invariant.

**What breaks without this**:
`test_both_screens_normalise_a_probe_the_same_way` in `tst_core_call.qml` — the
one assertion the duplication could not hold — plus the four strictness and
shape tests beside it. The strictness fixtures discriminate rather than decorate:
measured, four of the six (`"true"`, `1`, `{}`, `"yes"`) give a different answer
under a loose `!!v` than under `=== true`.

A consequence worth knowing: `DIdentityChip.qml`'s header named `FeedScreen.qml`
as *the* normalising boundary, which stopped being true the moment a second
screen normalised too. It now names `Core`, and that sentence stays true as
screens are added, which is the property the extraction buys.

### D13 — `parentOf` answers with a KIND, because a guard proved by mutation along one route was bypassed along another

D1 and D2 are the security property of this screen, and both were verified by
mutation: flip `resolveDepth`'s `-1` branches to `depth + 1` and tests go red. Two
agents ran that mutation and both concluded the property held. It did hold — **along
the route they mutated.** The `tester` found a second route that never reaches those
branches at all, and it is worth recording as a finding about *how guards are
proved*, not only as a bug that was fixed.

The original `parentOf` returned a plain string, `""` meaning "no parent". But it
produced that `""` for **any** `parent` that was not a non-empty string — `null`, a
number, an object — not only for an absent field. `resolveDepth`'s first line read
`parent === ""` as "this item IS the root" and returned `0` **unconditionally**,
before the visited set, before the walk, before either `-1`. So an item whose
`parent` was present but unusable rendered at the root's own depth with no "answers
a post not shown" notice — precisely the outcome D1 forbids and `thread.rs` refuses,
reached by a path where mutating D1's guard changes nothing.

**The lesson to carry:** a mutation test proves the guard it mutates is *reached and
load-bearing on the paths the fixtures take*. It says nothing about a path that
returns before the guard. "Two agents mutated it and it held" was a true statement
about an incomplete question.

**The fix is a shape, not a fourth guard.** CLAUDE.md's "complexity in the data
structure, not the logic": the defect existed because two different facts shared one
value, so no amount of care at the call site could keep them apart — the caller had
nothing to branch on. `parentOf` now returns `{ kind, id }` over three kinds:

| `kind` | means | `resolveDepth` |
|---|---|---|
| `parentRoot` | no `parent` field at all | `0` — this is the root |
| `parentNamed` | a usable, non-empty op id | walk the chain |
| `parentUnusable` | present but not a usable string, or no item at all | `-1` |

There is now no value the root case and the malformed case share, so a caller
cannot read one as the other by omission — it must name the kind it means. A future
call site added by someone who has not read this entry gets the distinction for
free, which a `""` sentinel or a fourth `if` at each call site would not give.

Rejected: **a distinguishable sentinel string** (the `tester`'s suggested shape,
e.g. returning `" unusable"`). It fixes this instance, and it keeps the
property that a caller comparing against `""` still compiles and still silently
takes the root branch — the same failure mode one value later. Rejected too: leaving
`parentOf` alone and adding a `typeof` check in `resolveDepth`, which is the fourth
slightly-different guard CLAUDE.md names as the signal to reshape.

Note a null/undefined *item* now classifies as `parentUnusable` rather than
`parentRoot`. That is deliberate and is a behaviour change in its own right: an
absent item is not a root, and the previous `""` return made it one.

**`itemId` does NOT have the same defect, and was deliberately left alone.** It
looks analogous — same `typeof` test, same `""` fallback — but its `""` is
single-valued. Both call sites treat it as a *refusal*: `itemsById` skips the item
rather than keying it, and `resolveDepth` skips seeding `visited`. Neither reads
`itemId(x) === ""` as an affirmative fact about the item the way `resolveDepth` read
`parentOf(x) === ""` as "this is the root". So there is no second meaning to
collide with, and reshaping it would add a kind nobody branches on. Recorded because
"the sibling function has the same shape" is the obvious next question, and the
answer is not symmetric.

**Is this reachable today? No — and it is still boundary defence, not dead code.**
`wire.rs:7041` and the `read_thread` response builder at `wire.rs:2027` only ever
omit `parent` (root) or serialise it as a hex op-id string. The current core cannot
produce a malformed `parent`. This guard defends the view against a shape its own
contract does not currently produce, which is CLAUDE.md's "Never trust an inbound
message... Validate at the boundary" applied where peer-supplied items are not
validated element-by-element anywhere upstream of this screen. **"Core is honest
today" is a different claim from "the view cannot be handed this shape"**, and it
expires the moment core changes, another producer appears, or an item reaches this
screen by a path that did not come through `read_thread`. Do not delete this as
unreachable — the whole point is that nothing in the view can tell.

**What breaks without this**:
`test_an_item_reporting_a_non_string_parent_is_not_rendered_as_the_root` in
`tst_thread_nesting.qml` goes red (`resolveDepth` returns `0`, expected `-1`), and
it is the only one that does — measured by replacing the `parentUnusable` branch's
`-1` with `0`: 15 passed, 1 failed. The two guards are independently pinned, which
is the property that says the routes really are distinct: the *original* mutation
(`-1` → `depth + 1` on the not-on-this-page branch) was re-run after this change and
still turns `test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root`,
`test_items_carrying_no_id_do_not_share_a_parent_slot` and
`test_the_unresolved_parent_notice_is_visible_only_where_it_must_be` red — while
leaving the new test green. So the reshaping extends D1's guard rather than routing
around it.

One incidental simplification: the walk's `while (cursor !== "")` became
`while (true)`. The loop condition was never the terminating one — `parentNamed`
guarantees a non-empty cursor, and termination comes from the visited set over a
finite `itemsById`, exactly as D2 says. Keeping the string test would have implied a
fourth thing `cursor` could be.

## What the merge took from each side

`cc37f2e` and this piece each built a thread screen. The resolution is not
per-file: each side won where its capability owns the question.

| Area | Taken from | Why |
|---|---|---|
| `DThreadScreen.qml` — nesting, states, composer, inert control | **this piece** | `thread-view` contracts all of it. Navigation's screen was built opportunistically and asserts none of these requirements. |
| `Main.qml` — the whole navigator | **navigation** | Five screens, onboarding, the shared status bar and the both-probes routing. `view-navigation` contracts it and `tst_navigation.qml` holds it. |
| `Main.qml` — `openThread()`'s `rootOp === ""` guard | **this piece**, into navigation's function | The one line this piece added to a file it does not otherwise own. `thread-view`'s *No thread is rendered before one has been chosen* requires that no read be made for a thread the user never asked for, and the transition is where that holds for the route. Recorded here because the table is what a maintainer reads to find every place this piece touched `Main.qml`, and a cross-piece edit missing from it is one nobody can find. |
| `FeedScreen.qml` — identity chip, footer, `createIdentityRequested`, the reading-is-free line | **navigation** | Contracted by `view-navigation`; this piece's screen predates all of it. |
| `FeedScreen.qml` — `threadTarget()` | **neither, see D10** | Both were wrong. The field is `thread`. |
| `Core.qml`, `tst_core_call.qml` — the bridge unwrap | **navigation** | The host double-encodes replies; every core call depends on it. Taken wholesale. |

**Two places navigation's version was kept over this piece's spec'd one**, which
is the part worth reading twice because it is where the spec did not win:

- **The navigator's thread transition.** This piece's `openThread()` left
  `chosen` set so the feed stayed live "underneath" the thread; navigation's
  clears it and rebuilds the feed from what `reading` carries. D9 assigns the
  transition to navigation, and `view-navigation` constrains only the outcome —
  the user reaches the feed again without the view restarting — which both
  mechanisms satisfy. Navigation's is the merged and contracted one, so it
  stands. Three assertions in `tst_thread_navigation.qml` were pinning this
  piece's mechanism; they are **rewritten, not deleted**, each against the
  outcome the spec requires rather than the implementation that produced it, with
  the supersession recorded in that file's header.
- **The `REPLYING AS` line.** Navigation's screen rendered the identicon and the
  address beside it, where this piece's rendered the label alone. `thread-view`
  does not require the mark and does not forbid it, and naming the key a reply
  will be signed by is the standing rule wherever an identity is named — so
  navigation's richer version was kept and carried over, along with the
  `who_am_i` probe that feeds it. That probe is a **second** call beside
  `get_capabilities`, not a replacement: `lib.rs:258` records that the two can
  honestly disagree, and only the identity report answers *who*.

**One defect the auto-merge introduced silently**, recorded because it is the
shape that would have shipped under a faster resolution: both branches added a
`readThread()` to `Core.qml`, git's three-way merge kept **both**, and QML
refused the file with `Duplicate method name`. That took every one of the 24
spec files from passing to failing-to-compile at once — which is loud, and is the
good case. The dangerous version of this is two near-identical additions that
*do* compile. Merged into one wrapper carrying both comments' reasoning.

## What the design bundle settled

The spec was written while the bundle was unreadable, so it pins no bundle
wording and lists several things as unknown. The bundle became readable partway
through this change (`tmp/ui-bundle-new/handoff/`), and **the owner's instruction
is that the visual design is the deliverable** — layout, structure, spacing,
indentation, borders, typography and colour, taken from
`reference/Dialectica App.dc.html`'s `06 Thread` block rather than invented.

What that block actually shows, measured from it rather than guessed:

| Question the spec left open | What screen 06 shows | Built |
|---|---|---|
| collapse/expand | no such control | none |
| a depth limit | two levels drawn, no limit stated | indent bounded at 6 |
| "load more" within a thread | no such control | none |
| indent treatment | `padding-left` 34px per level, `border-left:1px solid #d8d0bf` | `indentStep: 34` and a hairline rule per indented block |
| the composer's place | one composer at the thread's foot, under a `REPLYING AS` line and a `3px double` rule | as shown (D5) |
| the vote column | 40px, arrows, a numeral | 40px, arrows, **no numeral** — see below |
| revised posts | `edited` plus `read the earlier versions` | as shown, the second inert (D6) |

Two deliberate divergences from the bundle, recorded because a divergence on
purpose looks identical to one nobody noticed:

- **The vote column renders no numeral.** Screen 06 draws `12`, `4`, `1`, `0`
  beside the arrows. No call in the contract returns a score for a thread item,
  so a rendered number would be a tally core never reported — and `composer-view`
  forbids a rendered score wherever a vote control appears. The column's
  geometry is the bundle's; the numeral is dropped. The control is also inert,
  per the MVP ruling that puts voting out of scope; both are noted in
  `docs/PLAN.md` §9.2's case-2 list.
- **The bundle's `replyCaveat` is not used verbatim.** It ends "earlier versions
  stay readable", which promises exactly the facility D6 renders inert. The
  clause is dropped and what remains is true: a reply is a signed record that can
  be edited later.

The owner's ruling that **an inert control is acceptable when documented in
PLAN.md** overrides the bundle's README rule 4 ("with no identity the acting
affordances are not drawn at all"), so affordances are drawn and made inert
rather than omitted.

**Empty and error copy is this change's**, the bundle supplying none for this
screen: written to the spec's constraints — core's message verbatim, no
permanence claim, no fault attributed to the user.

## What the tests cannot see

`check_qml_names.py` proves a name absent, not that resolution is correct;
whether basecamp registers a competing name is invisible from the checkout, and
the `D` prefix is what makes the question not need asking.

A component test cannot catch a host name collision at all — under
`qmltestrunner` the host is absent, so there is no competitor for a binding to
lose to. That is the static gate's job, not the suite's.

No test here exercises a real core: the bridge is a fake throughout. What is
under test is the screen's own state machine and its nesting arithmetic, which is
the half this change wrote. That the request actually reaches `read_thread` and
comes back parseable is not proven by this suite, and a launch under basecamp is
what would prove it.
