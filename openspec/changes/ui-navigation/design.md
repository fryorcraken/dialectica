# Design — `ui-navigation`

## What this change does

Five types were registered in `dialectica-ui/src/qml/qmldir` and instantiated
nowhere. This change mounts the two that belong in the running view
(`DOnboardingScreen`, `DStatusBar`, and through the first of them
`DIdentityChip`), records the reason for the two that stay unmounted
(`DVouchStamp`, `DKeyNameWindow`), and adds a gate that turns "registered and
unreachable" from a thing nobody looks for into a build failure.

Measured on this tree before the change, `grep -rn` over `dialectica-ui/src/qml/`
for the five names returns `qmldir` lines and comment mentions only — no
instantiation. `grep -c "function test_"` reports 50 in
`tst_onboarding_states.qml`, 13 in `tst_status_bar.qml`, 14 in
`tst_identity_chip.qml` and 15 in `tst_vouch_stamp.qml`: 92 test functions over
four types a user could not reach.

## Decisions

### D1 — The reachability gate computes transitive reachability from `Main.qml`, not mention

**Chosen:** a static checker that starts at the view's root (`Main.qml`), finds
the type names it instantiates, follows them into those types' own sources, and
repeats to a fixed point. A registration is discharged when its name is in that
reachable set, or when `qmldir` carries an adjacent `# UNINSTANTIATED:` comment
stating why it is not.

**The alternative that looks equivalent and is not:** "is this type's name
mentioned in any `.qml` under `src/qml/`?" It is one `grep` and it is wrong
here, measurably. `DTip` is instantiated by `DStatusBar.qml:182` and by
`DVouchStamp.qml:128` — and before this change both of *those* were themselves
unmounted. A mention-based check reports `DTip` as reachable on the strength of
two callers that no user could reach either. It would have passed the whole
unreachable island: exactly the defect the gate exists to catch, reported clean.

That is this repo's recorded "a gate the defect satisfies" shape, and the reason
the spec says "instantiated somewhere **the view's root reaches**" rather than
"instantiated somewhere". The requirement's word is `reaches`, and the check has
to compute it.

**Removing the transitive walk turns exactly these tests red:**
`tst_check_qml_reachable.py::a_type_reachable_only_through_an_unreachable_type_is_reported`,
which stages a three-file tree where `B` instantiates `C` and nothing
instantiates `B`. Under a mention-based check `C` reads as reachable. This is
the one assertion that distinguishes the two implementations, and it is why it
exists.

**Why a static reader rather than a runtime probe.** The spec requires that
whatever checks this "read the registrations and the view's sources together,
rather than asserting against a component it constructed", and gives the reason:
a component suite supplies the very reachability whose absence is the defect.
A `qmltestrunner` spec instantiating `Main.qml` can assert that the mounted
screens are *there*, and this change adds such assertions — but that cannot
discharge the reachability requirement, because a type the spec fails to reach is
indistinguishable from a type nobody thought to check. The static check
enumerates the registrations; a runtime spec cannot.

**A stale claim corrected on the way past, because two files rest on it.**
`proposal.md`'s Impact section says "`Main.qml` is instantiated by no spec
today", and `check_qml_members.sh:12` says "No spec instantiates Main.qml".
**Both are false on this tree**: `tst_stoa_screens.qml:1600` declares
`Component { id: mainComponent; Main {} }` and thirteen tests drive it through
`mainComponent.createObject`. The comment in the gate is load-bearing — it is the
stated reason that gate exists — so it is corrected here rather than left to
mislead the next reader into thinking the blind spot it describes is still open.
What remains true, and is the part the gate's argument actually needs, is that
`qmllint` checks a *different resolution* than the app performs; that clause
survives the correction.

**Why a comment in `qmldir` rather than a list inside the checker.** The spec's
third scenario requires a newly registered type to be covered "without being
listed", and this repo has the `hand-maintained sweep lists go stale silently`
trap recorded: `check_qml_names.py`'s own `GRANDFATHERED` set carries the same
warning in its header. A list inside the checker is a place a new type can be
quietly added; a comment beside the registration is edited by the same hand that
adds the registration, in the same file, in the same diff. The reason travels
with the thing it excuses.

### D2 — Identity routing lives where a Stoa exists, because both probes are Stoa-scoped

**The constraint, measured rather than assumed:** `who_am_i` and
`get_capabilities` both take `{"stoa":"<hex>"}`, and
`wire.rs:478`'s `parse_stoa` refuses a missing or unparseable address —
`Address::from_hex` on `""` is an error, not a default. So **neither probe can
be asked before a Stoa is chosen.** There is no "am I anybody in general?" call
on the trait.

**Chosen:** the routing state is the feed — the screen that holds a Stoa — and
the affordance reaching onboarding is `DIdentityChip`'s existing
`createRequested()` signal, mounted in the feed's footer. `Main.qml` listens to
it and switches to `DOnboardingScreen` carrying that Stoa's address, which is
what `DOnboardingScreen.stoaAddress` requires and what every core call on that
screen takes.

**Rejected: a global identity gate in front of the Stoa list.** It is the shape
a reader expects — "no identity? onboard first" — and it cannot be built against
this contract. It would have to probe with some Stoa, and before the list is
read there is none; inventing one, or probing with the zero address, asks core a
question about a Stoa that does not exist and renders its refusal as an identity
verdict. That is the view inventing an answer, which is the failure this
codebase designs against everywhere else.

It also states the wrong thing. An identity here is per-Stoa by design (PLAN §5.2
and `DOnboardingScreen`'s own "The Stoa this identity is being chosen for"), so
"you have no identity" is not a fact the view can assert before knowing which
Stoa is meant.

**Consequence worth naming, because it is a real limitation rather than a
tidy result:** a peer in no Stoas at all reaches no routing state and so is
offered no onboarding. That is not a hole this change leaves open by oversight —
it is what the contract permits, since creating or joining a Stoa is the action
available to such a peer and `create_stoa` fails with the keystore's own reason
when there is no usable key. Recorded here so the next reader does not read the
absence as a missed case.

### D3 — Three outcomes, and the disagreement is rendered rather than collapsed

The spec requires that "a user with no identity, a user with an identity that
cannot be used, and a user who can act are three states, and a routing that
renders only two of them must be silently merging a pair."

**Chosen:** the feed asks both probes on every render and derives the routing
from the pair:

| `who_am_i.hasIdentity` | `get_capabilities.canPost` | routed to |
|---|---|---|
| `false` | (anything) | the create-identity affordance → onboarding |
| `true` | `false` | the closed posting gate, with core's reason verbatim — **no create affordance** |
| `true` | `true` | the open feed, identity chip filled |

The middle row is the one the spec calls out, and the reason it is the load-
bearing one: routing that user to identity *creation* is, per
`lib.rs:247-249`, irreversible — "Keeping is refused where an identity already
exists. Replacing one discards every identity derived from it while the ops they
signed remain published." A user with a fixable permissions problem offered a
new key is offered the one action that cannot be undone, instead of the reason
that names their fix.

`DIdentityChip` is bound `hasIdentity: <the who_am_i answer>` and **not**
`capability.canPost`. The chip's own header says to bind `canPost === true`,
which was right while the chip was the posting gate's own indicator and is wrong
for this placement: binding `canPost` here collapses rows two and three and
shows "Create an identity" to a user who has one. The chip header's instruction
is updated in this change to state both bindings and which placement takes which.

### D4 — Neither probe is cached, and the routing holds no derived boolean either

`FeedScreen.qml:41-43` already re-probes `get_capabilities` on every render,
citing PLAN §9.1: "caching it across a keystore change is how a button outlives
the key that justified it." This change adds `who_am_i` to the same `reload()`,
under the same rule, and adds **no navigation-level state** derived from either.

**Chosen:** `Main.qml` gains no `hasIdentity` property. The screen selection
still derives from the two nullable properties it already had (`chosen`,
`previewing`) plus one more for onboarding; identity never enters the selection.
The navigator routes *on a signal* (`createRequested()`), which is an event, not
a retained answer — so there is nothing at the navigation layer that can outlive
the keystore it described.

**Rejected: `property bool hasIdentity` on the navigator**, refreshed on
transitions. It is the obvious way to make the routing readable in one place,
and it is precisely the defect one level up: a key that becomes unusable between
two renders leaves that boolean describing a state that no longer exists, and a
navigation layer holding it is wrong for every screen at once rather than for one
control. The spec names this ("A retained 'this user has an identity' outlives
the keystore it stands for"), and the prohibition is structural here rather than
remembered: the property does not exist, so no screen can read a stale one.

The `who_am_i` answer is stored on `FeedScreen` as `identity`, written only by
`reload()` and by nothing else — the same lifetime as `capability`, which is one
render. That is not a cache: it is the current render's answer, discarded and
re-fetched when the screen renders again.

### D5 — The screen selection stays one derived string; onboarding joins it

`Main.qml`'s existing shape is a `readonly property string screenShown` computed
from nullable properties, with no `StackView`, so the state has one source rather
than two that can disagree. The proposal says this change keeps it, and it does.

Onboarding joins as a third nullable property, `onboarding`, set and cleared by
functions that clear the others — the same discipline the existing `preview()`
and `open()` follow, and for the same stated reason: a caller that assigns
directly re-creates the impossible state, and a guard that must be remembered at
every call site is what CLAUDE.md says to replace with a shape that enforces it.

**Why `onboarding` carries the Stoa rather than being a bare bool.** The screen
needs `stoaAddress`, and the return needs to know which feed to go back to.
A bool plus a separate "which Stoa" is two values that can disagree — one set,
the other stale — which is the same unrepresentable-state argument that produced
`chosen` and `previewing`. One object, or null.

### D6 — The route out of onboarding is unconditional, and the navigator owns it

The spec: "The return MUST NOT be conditional on the keep having succeeded. …
A route out offered only on success is a route absent in exactly the cases where
the user is stuck."

`DOnboardingScreen` has no back affordance and **must not grow one**: its
capability contracts that it "SHALL NOT navigate anywhere itself", and its only
outward signal is `identityKept()`. So the affordance is rendered by `Main.qml`
*above* the screen, not inside it, bound to nothing about the screen's phase.
It is always there — in `intro`, in `slate`, in `refused`, in `failed`, and in
`kept`.

**Rendered outside the card rather than passed in as a property.** A
`property bool showBack` on the screen would be a navigation concern inside a
screen that is contracted not to have one, and a screen author could bind it to
a phase without anything failing. Rendering it in the navigator makes the
unconditional-ness structural: there is no phase in scope at the place the
affordance is declared.

On `identityKept()`, `Main.qml` clears `onboarding` back to the feed it came
from and calls `feed.reload()`, which re-asks `who_am_i` and
`get_capabilities` — so the identity shown is a fresh answer to the identity
report, never the keep signal, which carries no identity by design.

### D7 — `DStatusBar` mounts on every screen, and its delivery lamp is a documented case-2 placeholder

**The spec is unambiguous about placement:** chrome "MUST be rendered on
**every** main-area screen rather than on a subset", because "a user cannot tell
a screen that omits the indicator from a machine with nothing to report". So the
bar is a sibling of the screen column in `Main.qml`, outside the per-screen
`visible:` bindings, rendered in all states.

**The delivery lamp question the brief asked me to decide: it IS a §9.2 case-2
placeholder, and it now has a PLAN entry.**

The argument. Ruling 4's case 1 says a control MUST be wired where core serves
it; case 2 permits an inert control or placeholder where core cannot, *on the
condition that it is documented in PLAN.md*. The delivery lamp is squarely
case 2 and not case 1: `delivery_module.lidl` carries the outcome as three
channel events — `channelMessageSent`, `channelMessageError`,
`messagePropagated` — which arrive **asynchronously, after the publish call has
returned**, so no synchronous call on the trait produces a signal to bind. There
is no method to wire it to. That is a missing call, which is exactly what a
case-2 entry names and what removing the entry later depends on.

**Nothing forbids the placeholder here**, which is the narrowing that governs
the other entries: `stoa-navigation-view`'s "Every number rendered is one this
peer can actually answer" is about rendered *counts*, and `composer-view`'s is
about a vote *score*. A lamp is neither.

**And the placeholder is honest by construction rather than by choice**, which
is what makes this acceptable rather than merely permitted. `DStatusBar`'s
`deliveryState` defaults to `"degraded"`, not `"ok"`, and `normalisedState()`
maps every unrecognised value to `degraded` too — so an unbound delivery lamp
claims nothing. The spec requires exactly that ("Unbound chrome does not report
health… it renders its least-claiming appearance instead"), and the component
already satisfies it. `Main.qml` therefore binds `storageState` and `zoneState`
from what it can honestly answer and **leaves `deliveryState` unbound**, which is
the least-claiming reading available and is not an oversight.

The PLAN entry added by this change is case-2 entry 6.

**The reasoning migrated out of `docs/PLAN.md` into this decision**, verbatim,
because it is what rules out the tempting heuristic:

> That also makes the return value a *worse* signal than the events, not merely
> a missing one: a sink that accepts an op tells you the transport took it, which
> is `channelMessageSent` and says nothing about whether any peer received it.
> `messagePropagated` is the fact a user cares about. Publishing and delivering
> are two events at two times, and this decision stops the API pretending they
> are one.

So a delivery lamp bound to "the publish call returned" would render the
transport's acceptance as the user's delivery — the exact conflation the
asynchronous design exists to prevent. That is why the lamp is left unbound
rather than filled with the one signal that is to hand.

PLAN.md keeps the `op-transport` obligation, which is still ahead, and now points
here for this half rather than carrying a second copy of it.

**`DStatusBar.qml:57` cites `docs/PLAN.md:3471-3481` for this, and that line
range is stale** — it now lands in §9.2's MVP list. The comment is updated to
cite the passage by name rather than by line number, which is the form that
survives PLAN.md shedding as changes land.

### D8 — The six tooltip strings are an open item, and it is not mine to close

`DStatusBar.qml:50-53` records that no test checks the six tooltip strings
against the bundle, that they must travel verbatim (because `deliveryNoPeers`'
second clause is what tells an empty feed from an unreadable store), and that
whether the verbatim obligation belongs in a spec is unanswered.

**I did not adopt it and did not drop it.** Two reasons, and the first is the
one that decides it:

- **`view-navigation`'s Purpose scopes it to transitions and explicitly not to
  what a screen renders.** A requirement pinning six strings is a rendering
  requirement. Putting it here would be this capability specifying content,
  which its own Boundary section forbids in as many words.
- The strings are `DStatusBar`'s content, and no capability currently owns
  `DStatusBar`'s content at all — which is the actual gap. It needs a home
  before it needs a test.

**Owner: a `spec-writer`, as a scope question — does the MVP contract the status
bar's copy?** It is recorded here rather than in a findings file because findings
are deleted at merge. Where it stands today: the bar renders with the three
tooltip properties unset, and `DStatusBar` "invents nothing when they are unset",
so no string is shown and none is wrong. That is a strictly smaller claim than
the bundle's, and it is stable — the obligation only bites when someone fills
them in.

**The owner has since descoped copy from this piece** ("I don't give a shit about
the text in the bundle, I want the DESIGN… to be honoured"), which settles what
this change does and leaves the question itself open for whoever contracts the
bar's rendering.

**One thing found while reading `copy.json`, worth recording because it is a trap
for whoever does wire these.** Two of the six strings are not strings but
specimen data from the mockup: `deliveryOk` reads "Delivery: 2 peers reachable,
nothing queued" and `storageOk` reads "Storage: store read without error, 31
posts held". Wiring those verbatim would put a fabricated peer count and post
count on screen — against the bundle's own README rule 3 ("Never show a count of
anything global. Every number counts what this machine holds") and against
`stoa-navigation-view`'s merged "Every number rendered is one this peer can
actually answer". The four without numbers are safe verbatim; these two need
parameterising or the number dropping. The "verbatim" instruction in `SPEC.md`
and this constraint genuinely conflict for exactly these two strings.

### D9 — A singleton is reached by a member read, not an instantiation

Found by running the gate rather than by reasoning: the first version asked
whether anything wrote `DTheme { }` and reported `DTheme`, `Core` and
`DStoaReference` as dead code. All three are reached constantly — as
`DTheme.paper`, `Core.listThreads(...)`. A singleton **cannot** be instantiated,
so asking whether anything instantiates one is a category error that condemns
every singleton in the module.

**Chosen:** a singleton is reachable when something the root reaches *reads a
member off it*. That is the honest analogue of instantiation for a type that
cannot be instantiated, and it fails in the right direction — a singleton nothing
reads is genuinely dead, and one read only by an unreachable type is still caught,
because the walk that finds the read is the same transitive walk.

The two forms are not interchangeable and the gate does not treat them so:
counting `Foo {` for a singleton, or `Foo.` for a component, would let the wrong
syntax mark a type reached. `tst_check_qml_reachable.py::a_singleton_nothing_reads`
pins it.

### D10 — Both screens re-read when they are re-pointed

Found by the tests, which is the reason to write them alongside the code: four
assertions failed on the first run, and the cause was the same in each.
`FeedScreen` and `DThreadScreen` are each mounted **once** and re-pointed at
whichever Stoa or thread the navigator chose. Both called `reload()` only from
`Component.onCompleted`, which runs when the address is still empty — so the
first Stoa a user opened would have been the only one ever read, and the first
thread the only thread.

**Chosen:** `onStoaAddressChanged: screen.reload()` and
`onThreadRootChanged: screen.reload()`. Each keys on the value that identifies
what is being read, not on everything that travels with it: a genesis record
arriving separately for the same address is the same Stoa, so re-reading on it
would issue a second identical call.

This is also what re-probes both identity answers on arrival at a feed, which is
D4's rule discharged across the transition rather than only within a screen.

**Removing either handler turns exactly these tests red:**
`tst_navigation.qml::a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`,
`::an_unusable_identity_is_not_offered_identity_creation`,
`::both_probes_are_re_read_on_every_render` and
`::no_record_is_invented_for_a_stoa_the_view_has_none_for`.

### D11 — The design bundle is the visual authority, and where our tree diverges

The owner ruled, through the runner, that the design bundle at
`tmp/ui-design/handoff/` is the deliverable: "the DESIGN AKA the SCREEN AKA THE
UI", with `reference/Dialectica App.dc.html` as the primary reference and
`qml/Theme.qml` as the visual system. Copy was explicitly descoped.

**`DTheme.qml` already matches `Theme.qml` exactly** — every colour, font,
metric and rule, verified value by value. The visual system needed no change;
`DTheme` is a faithful superset, adding only the seven mark inks and
`markMutedAlpha`, which are the identicon's own scope.

Three divergences from the bundle, each deliberate:

- **`hasIdentity` is `readonly` and derived, not a settable bool.** The bundle's
  `examples/FeedScreen.qml` declares `property bool hasIdentity: false` and
  gates on it, and SPEC.md:21 makes screens 03 and 05 one screen with that flag.
  The property name and the gating are honoured; what differs is that ours is an
  expression over the answer `reload()` most recently received. So the bundle's
  interface is what a screen author meets, and there is no setter through which a
  stale value could be written — D4's rule held by construction rather than by
  everyone remembering it.
- **Inert controls where the bundle draws live ones.** Per the owner's ruling,
  documented in PLAN.md §9.2 case 2 rather than omitted: the delivery lamp, and
  the storage lamp's mocked source. The bundle's README rule 4 ("with no identity
  the acting affordances are not drawn at all") is **overridden** by the owner's
  later ruling that inert is acceptable when documented.
- **No fonts are shipped**, so the design does not render at its intended
  density. `DTheme` names Spectral and IBM Plex Mono and neither is in
  `dialectica-ui/src/`, so Qt silently falls back to a system font — the repo's
  silent-failure house style exactly. PLAN.md case 2 entry 8 records it. Both
  families are OFL; bundling them is a binary-asset change and a piece of its
  own.

## What this change deliberately does not do

**It does not wire `DKeyNameWindow`, and wiring it would break a gate.** It is
the name-disjointness gate's measurement apparatus, and
`tst_identicon.qml:638-663` asserts it exposes no renderer — verified in this
tree, the assertion drives a `renderers` list of eleven names and fails if any is
defined. Its comment states why: a renderer moves the component under
*Malformed key material is refused rather than crashed on*, and its zero-padding
then becomes a defect. The test itself notes that the "no consumer" half is
held by a spec requirement rather than by anything `qmltestrunner` can see —
this change supplies that requirement, and the `qmldir` record is where it lands.

**It does not wire `DVouchStamp`.** Voting is out of the MVP by owner ruling 1,
so its non-instantiation is a scope decision rather than an oversight, and the
`qmldir` record says so.

## The boundary with `piece/ui-thread-view`, and how the owner's ruling moved it

The spec-writer flagged a risk that both pieces specify one transition. Checked
against `view-navigation`'s "A thread is opened from a feed row and can be left":
the requirement's scenarios constrain what is *handed across* (the Stoa, the root
post's identifier, the founding record where the view holds one) and that a
return affordance exists. It contains no scenario about what the thread screen
displays, and carries the explicit clause "What the thread screen renders is
outside this requirement". **That delta is clean** — the two pieces' requirements
do not overlap.

**The implementation boundary is where it moved.** This change originally scoped
out building a thread screen at all, on the ground that the destination is the
other piece's and a placeholder would put a second implementation in the tree.
The owner then ruled that every screen the design shows must be built, mocked
where core cannot feed it, so `DThreadScreen.qml` is built here from the design
bundle's screen 06.

**What that means for the other piece, stated plainly because it is now a
collision risk rather than a theoretical one:** `piece/ui-thread-view` will find
a thread screen already in the tree. It owns that screen's *contract* — what it
renders, how it nests, what a hidden root does — and this file claims none of
that. `DThreadScreen.qml`'s own header says so. The screen built here is a
design-faithful implementation that piece should adopt and constrain, not a
second design for it to reconcile.

Two things in it are worth that piece's attention, both chosen without a
requirement to point at:

- **Nesting is computed by walking the parent chain**, because `read_thread`'s
  trait doc requires it: "The items are flat and each names its parent. Nesting
  is the view's to compute… No item reports a depth or an indentation level." A
  chain this peer cannot complete stops at what it holds rather than guessing,
  and the walk is bounded by the item count so a cycle in peer-supplied data
  terminates instead of hanging the view. `parent` is a field a peer fills in.
- **A hidden root is rendered marked**, not dropped — the trait contracts that a
  hidden root comes back with its body withheld while a hidden reply is omitted,
  because "a thread read that dropped its own subject would be indistinguishable
  from a thread this peer never received".
