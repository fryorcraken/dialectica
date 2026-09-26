# Design — a created Stoa, driven end to end

## Context

See proposal.md for why this piece exists and what each specification must
observe. The harness is the archived `e2e-ui-suite` change's, as corrected by
`e2e-suite-review`: `lgs basecamp setup` and `install` build and populate a
profile, sitometres 0.1.2 launches Basecamp against it, and the adjudicator
requires a passing verdict, every step passed, and the report's step count
equal to the spec's. Nothing about that harness changes here except the matrix.

**The owner's decision on #134 is what makes this piece 0.0.1 work.** The
comment on the issue, 2026-09-25: *"the full end-to-end suite stays in 0.0.1.
The owner wants good testing foundations in this release. PR #164 landed the
join-refusal spec, and that is not enough to close this issue. Still required
in 0.0.1: a successful join, which needs a seeding peer; the feed screen; and
the thread screen, each run end to end against an `lgs`-installed Basecamp.
Splitting the remainder out to 0.0.2 was considered and rejected."* The issue
body adds the moderation screen, shipped inert in #127, "once [it is a]
stable target", and #164's hand-back added key creation then Stoa creation as
the cheapest route to the feed. This piece is four of those five; the
successful join, with its seeder, is the next piece, and its PR closes #134.
This one is `Part of #134`.

Three facts about the tools shape the decisions below, each read at the pinned
sitometres tag `v0.1.2` (`6dc23e2`) rather than the fork commit the archived
design cites (`e2e-suite-review` design.md D5):

- **Selectors exclude hidden items** (`src/runner/selector.ts`, `resolveAll`:
  `if (!sel.includeHidden && !node.visible) continue`), and a QQuickItem's
  `visible` reads false under a hidden parent, which `tst_e2e_handles.qml`
  measured. Every screen stays mounted in `Main.qml`, so a control on a hidden
  screen is in the tree and excluded, not absent.
- **A click aimed at a label goes to a handler found from its ANCESTORS**
  (`src/runner/snapshot.ts`, `clickTargetFor`): the nearest clickable ancestor,
  else the first visible `MouseArea` among the descendants of the nearest
  ancestor holding one. A label's own child `MouseArea` is never searched.
- **Every step polls for at least one second** while the default `no_errors`
  check is live (`src/runner/runner.ts`, `pollChecks`: `settleMs` defaults to
  1000, and a clean result is accepted early only when every check is
  monotone-positive, which `no_errors` is not).

## Goals / Non-Goals

**Goals:** the four specifications green in CI on this piece's tip, each also
seen red in CI for the reason it exists, with both runs recorded; every root
handle a spec reads pinned against a fixture that fails its null
implementation.

**Non-Goals:** the successful join and its seeder (proposal, "The second
piece"); any change to core or to the wire contract. The view code adds
read-only projections, `objectName`s and one reshaping that changes nothing
rendered (D3), plus the fix for one defect the feed spec found (D8). That fix
brings the view into line with `view-navigation`, and adds no behaviour the
specs do not already require.

## Decisions

### D1 — Four files, not one chained file

**Chosen:** `create.yaml`, `feed.yaml`, `thread.yaml` and `moderation.yaml`,
four matrix jobs beside `join`. Each starts from a fresh profile, so each after
the first repeats the key-and-Stoa prefix: eight steps from opening the app to
a created Stoa's row.

**Rejected: one file walking the whole route.** sitometres 0.1.2 stops at the
first failed step and records every later step as `inconclusive`
(`e2e-suite-review` design.md D2, measured). In one file, a feed defect would
therefore hide the thread and the moderation screen: the report would say
nothing about them either way. Four files keep a red job's name the screen
that broke, and let each proof break (D6) redden exactly one job.

**The cost, stated:** four more jobs on every PR. They run in parallel, so the
wall-clock cost is one job's, and a warm job is about three minutes (the
`e2e-ui-suite` change's design.md D7 measured 3m16s warm, 9m19s cold); the
repeated prefix adds seconds of steps to minutes of setup. The runner minutes
are four times one job's. Accepted: the jobs share the store cache, and the
alternative spends the same Basecamp build to learn less.

### D2 — The prefix gets there and asserts nothing

Each spec's steps before its own screen only wait for what the next action
needs — `createKeyButton` before clicking it, `openStoaButton` before opening
the row — and assert nothing about the screens they pass through. Every
observation the proposal lists is asserted once, in the one spec it belongs
to: the key-held state's absences in `create.yaml`, the empty-versus-failed
read in `feed.yaml`, and so on.

**Why:** a prefix that re-asserted `feed.yaml`'s read in `moderation.yaml`
would make a feed defect red in two specs, and a proof break aimed at the feed
could no longer show that `feed.yaml` is the spec that sees it. The final
"feed is rendered again" steps in `thread.yaml` and `moderation.yaml` assert
`screenShown` only, for the same reason. D6's feed break is the check: its red
run must be `feed.yaml`'s alone.

### D3 — Root handles, and the feed's guard moved into its data

`state:` expressions are evaluated against `Main.qml`'s root, so the specs
read seven new read-only projections, each of state a screen already owns
(the `e2e-ui-suite` change's design.md D6 is the rule):

| handle | reads | used by |
|---|---|---|
| `listedStoas` | `list.visibleRows`, each row's `stoa` | create |
| `createdStoa` | `list.created.stoa`, the creation reply the screen kept | create |
| `feedReadState` | `feed.readState` | feed |
| `feedRowCount` | `feed.visibleRows.length` | feed |
| `feedCanPost` | `feed.capability.canPost === true` | feed |
| `threadReadState` | `thread.readState` | thread |
| `threadItemCount` | `thread.items.length` | thread |

**`feedRowCount` needed a reshaping first**, committed on its own with no
behaviour change. `FeedScreen.rows` keeps the last good page across a failed
reload, on purpose, so the Repeater and the empty state each restated
`readState === "ok"` before reading it, and a root handle would have been the
third copy. `FeedScreen.visibleRows` now holds that guard once, the shape
`DStoaListScreen.visibleRows` already has, and all three read it.
`threadItemCount` needs no such guard: `DThreadScreen.reload()` empties
`items` on every failure path.

**`createdStoa` and `listedStoas` exist so "holds that one Stoa" is a
comparison, not a count.** `root.listedStoas[0] === root.createdStoa` fails
on a listing of one Stoa that is not the one just made, where `stoaCount === 1`
would pass.

**What breaks without each, measured** against `tst_e2e_handles.qml`, each
mutation applied alone and reverted:

- `feedRowCount` bound to `feed.rows.length`: `test_the_feed_handles_follow_the_feed_screen`
  goes red on "a feed the screen has said it could not read is not counted"
  (`Actual 2, Expected 0`), and nothing else does.
- `createdStoa` bound to the first listed row: `test_the_creation_handles_follow_the_creation_reply`
  goes red on "nothing created yet" (it reads stoaA before any creation).
- `listedStoas` mapped over `lastListing`: `test_a_failed_reload_does_not_count_the_listing_it_kept`
  goes red on "nor are its addresses listed".
- `feedCanPost` fixed at `true`: the feed test goes red on "the gate follows
  the probe".
- `threadItemCount` fixed at 2: the thread test goes red on the failed read's
  count.

**Rejected: a handle for the composer's outcome.** It would need `FeedScreen`
and `DThreadScreen` to widen their surface to expose their composers' state.
The requirement is what the screen *states* — `composer-view`'s "A success
names local storage" — so the specs match the rendered sentence, "Your post
was saved on this machine.", which is the claim itself.

### D4 — What the specs click and type into is named at the handler

- **`openStoaButton`** on the list row's Open. One per row, so the name is
  unambiguous on the one-row list a fresh profile holds after one creation.
- **`postDraftField` / `postSubmitButton` and `replyDraftField` /
  `replySubmitButton`**, derived in `DComposer` from `kind`. The feed and the
  thread each mount one composer, both stay in the tree whichever screen is
  shown, and selectors excluding hidden items (Context) would already pick
  the visible one — but a name that is unique by construction does not rest on
  that. `tst_composer.qml`'s `test_the_field_and_the_submit_are_named_by_kind`
  pins that each name lands on the element doing the job; with the submit's
  name fixed at `"postSubmitButton"`, its reply case goes red, measured.
- **`readThreadArea` and `moderateArea`, on the `MouseArea`s, not on the
  labels.** Per Context, a click aimed at the `moderateLink` label resolves
  to the first `MouseArea` under the feed header's `RowLayout`, and the first
  one there is the "All Stoas" button's: the click would leave the feed for
  the list. `readThreadLink` happens to be safe today, being first in its own
  row, and is named at its handler for the same reason. `tst_navigation.qml`
  already finds both labels by their existing names, which are unchanged.

No component test clicks the two `MouseArea`s by name: a parentless view has
no window for `mouseClick`, and the e2e run fails by name ("No object has
objectName …") if either is missing.

### D5 — What each step asserts, and when it reads

- **`state:` for what the app believes, `text:`/`not_text:` on an `objectName`
  where the claim is about the element tree**, and every absence beside a
  presence in the same snapshot (the `e2e-ui-suite` change's design.md D5).
- **`expect:` where the value follows synchronously from the step before**,
  `wait_for:` where it arrives later. The feed's read runs synchronously in
  `onStoaAddressChanged`, so "the feed was read, and holds nothing" is an
  `expect:` on the state after `screenShown === 'feed'` held: it asserts the
  read's result, not that some later retry came good. That matters for #152,
  whose report is intermittent.
- **The created address is asserted by shape** (`/^[0-9a-f]{64}$/`): it is the
  hash of a record whose creator key is minted fresh each run.
- **Moderation's "acting on an inert control leaves the way out offered" is
  asserted on the click's own step**, and both halves already hold before the
  click. It reads a snapshot taken after the click only because of the
  one-second settle floor (Context). If a session ever ran without log
  evidence, `no_errors` would drop out, every check would be monotone-positive,
  and this step would return at t≈0 and prove nothing. The D6 break is the
  evidence it currently can fail.
- **No `calls:` and no `no_calls:`**, for the `e2e-ui-suite` change's design.md
  D10 reason: every observation here is an effect. `no_calls` on the inert
  click would also hold every step to its full timeout, because a negative
  check cannot settle early.

### D6 — Each spec's proof is one break, pushed alone

The `e2e-suite-review` change's model (its design.md D3, its tasks.md 2 and 3):
each break is its own commit, pushed alone, both workflows waited out, then a
`git revert` pushed and waited out. Both workflows cancel in progress per ref,
so a push before a run finishes records nothing. Each break targets the
observation its spec exists for, and D2 is what lets it redden one job:

| spec | break | where it goes red |
|---|---|---|
| create | the key block's Loader also active in the key-held state, and that state given the empty `refusal` the block reads | "the key is held, so a Stoa is offered and a key is not", on `not_text` |
| feed | a read with no items becomes the failed state | "the feed was read, and holds nothing" |
| thread | `Main.openThread` sends `genesis: ""` | "the thread was read, and holds the root with no replies" |
| moderation | "Mark as moderated" hides the way out | "acting on an inert control leaves the way out offered" |

**The create break carries a second edit, and it is there to keep the red in
one job.** The Loader alone, measured locally, makes the key block read
`machineKey.refusal` in a state that has none: `Unable to assign [undefined]
to QString`, on every key-held showing. That binding would evaluate in every
spec's prefix, not only `create.yaml`'s, and whether sitometres' default
`no_errors` counts it is a question the proof should not also be asking.
Giving the held state `refusal: ""` removes the warning and leaves the break
exactly what it claims to be.

Each break also reddens `ci.yml`'s component tests that pin the same
behaviour. That red is in the other workflow and expected; `tasks.md` records
which tests, predicted by running the suite locally with the break applied.

### D7 — Prose pruned in the files touched

`Main.qml` said a created Stoa "is unshareable immediately" because
`create_stoa` returned no record, and that `chosen.genesis` is "" because "the
listing does not return one". Both stopped being true with
`genesis-in-replies` (#130), and `stoa-navigation-view`'s delta here retires
the requirement that said the same. `FeedScreen.qml` cited `SPEC.md`, which is
not a file this repository tracks, for the empty-versus-unreadable rule; it
now cites the `feed-view` requirement this change adds. `join.yaml`'s "does
NOT cover" paragraph now names only the successful join.

### D8 — A re-pointed screen's trigger is withheld until the rest of its pair has landed (#152)

**The defect `feed.yaml` found.** UI tests run 36213442819 went red on "the
feed was read, and holds nothing": a freshly created Stoa's first feed read
failed, and the read after a post was published succeeded. `FeedScreen`
reads on `onStoaAddressChanged`. `Main.qml` bound its `stoaAddress` and
`stoaGenesis` separately from the one `chosen` value, and QML ran the address
binding first. So the first read of every open carried the PREVIOUS record,
`""` coming from the list, which the core refuses as "genesis record ended
mid-field". Any later read carried the right one. That is #152's report
exactly: the error on entering a Stoa, and "the error is transient", cleared
by a re-read (its 2026-09-24 evidence). It also hit both returns onto the
feed, from a thread and from moderation, because each re-points the feed from
`""`. `DThreadScreen` had the same shape, reading on `threadId` alone, and
was correct only because QML happened to run that binding after the address
and the record.

**Why no test saw it.** The route tests asserted what the navigator held
(`chosen.genesis`) or what the last call carried, against fakes answering
every read alike. The two explanations, "the read carried the record" and
"a read was made", gave the same answer. The tests added in
`tst_navigation.qml` answer a read only when it carries the chosen Stoa's
record, and check every read rather than the last.

**Chosen: `Main.qml` withholds the trigger.** The feed's address binding reads
`feed.stoaGenesis === root.chosen.genesis`, and the thread's id binding reads
the thread's address and record the same way. If the trigger's binding runs
first, it sees the old record, yields `""` and triggers nothing. When the
record lands, the trigger re-evaluates, is released, and the one read it
starts carries the pair. The screens keep one trigger each and are unchanged
in behaviour when driven directly, as their component tests drive them.

**Rejected alternatives, each measured or read:**

- **Re-read on both halves**, the shape `DJoinScreen` uses for its lookup. It
  was the first fix tried here. The last read was right, but one transition
  sent a read carrying `""` for a Stoa whose record the view holds before
  correcting it. `view-navigation`'s "The view holds no Stoa of its own"
  says the view "MUST NOT send a placeholder or an empty one in place of a
  real one", so the fix broke the same sentence the defect did. Measured: the
  every-read check went red on it ("opened from the list: read 1 of 2 carried
  the record").
- **Defer the read with `Qt.callLater`.** It coalesces to one read, but
  asynchronously. The component suite and `feed.yaml`'s `expect:` steps rely
  on the read having happened when the transition returns (D5).
- **Hand each screen one object instead of three properties.** That is the
  honest data shape, and it would change the interface every standalone
  screen test uses: `stoaAddress` and `stoaGenesis` appear in 20 test files.
  It is the right follow-up if a third screen acquires a pair.

**What breaks without it, measured.** With the feed's guard removed, exactly
one test goes red, `test_the_feed_is_read_with_the_record_on_every_route_onto_it`,
with the core's refusal of `''`. Each of its three routes is red alone
against the unfixed code: from the list, back from a thread, back from
moderation. The thread's guard is invisible in the committed binding order. With
`threadId` bound above the address and record and no guard, three
`tst_navigation.qml` tests go red, among them
`test_the_thread_is_read_with_the_record_when_it_is_opened` ("No thread was
given to this view."). With the guard, all 568 component tests pass in
that reversed order and in the committed one.

**What it does not address.** #152's report also allows a pre-#130 profile
holding a stored record that is empty or short. Nothing here touches stored
data, so this fix may not be the whole of #152, and the PR does not close it.

**No spec change: the behaviour is already contracted.** `view-navigation`'s
"The view holds no Stoa of its own, and the feed is reached from the list"
requires the record to travel with the address and forbids sending "an empty
one in place of a real one". "A thread is opened from a feed row and can be
left" says the same for the thread and for the return to the feed. The defect
broke both, and the fix makes the view do what they already say.

## Risks / Trade-offs

- **#152's report may have a second cause**, a stored record from before
  #130 (D8). → `feed.yaml` opens a freshly created Stoa on every PR, so a
  regression of the binding race goes red here. A stored-data case needs an
  old profile, which no run here has.
- **The trigger guards in `Main.qml` are a rule the next screen must copy.**
  A screen re-pointed by the navigator that reads on one property of a pair
  has the defect D8 fixed. → The tests in D8 cover the two screens that read;
  a third is the point at which the one-object interface stops being a
  follow-up.
- **The moderation step's power to fail rests on the settle floor** (D5). →
  sitometres is pinned exactly and `tst_ui_tool_pins.sh` fails on drift; the
  D6 red run is the measurement. A sitometres bump should re-run that break.
- **`thread.yaml` and `moderation.yaml` observe contracts that live only in
  unarchived change folders** (`ui-thread-view`, `moderation-screen`;
  proposal). → The steps cite them; archiving those changes is for their own
  closers and the owner.
- **sitometres assigns a field's `text` rather than typing** (carried from the
  `e2e-ui-suite` change's Risks). → `createTitleField` reacts through
  `onTextChanged` and `DComposer.draft` aliases its field's `text`, so an
  assignment reaches both. A move to `onTextEdited` would turn a spec red,
  which is the right direction.
- **Four more jobs per PR** (D1). → Parallel; the store cache is shared.
