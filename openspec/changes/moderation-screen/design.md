# Design — the moderation screen, inert

## Context

The owner reversed `docs/PLAN.md` ruling 3's screen half. The MVP ships screen 07
with every control calling nothing, because the owner wants to see it. The
publishing half of ruling 3 is untouched: the `Dialectica` trait exposes no
moderation-publishing method, so there is genuinely nothing for the controls to
call.

Two merged requirements also had to move, because a scope note in PLAN.md does not
override one — PLAN.md says so itself, and this change is that mechanism working
rather than an exception to it.

## Decisions

### D1 — The screen renders its own inertness, and a requirement holds it there

**Chosen:** a standing notice above the confirmation, saying the controls publish
nothing and naming the absent core method as the reason, contracted by
`moderation-view`'s "The screen states that nothing it offers takes effect".

**Considered and rejected: rely on nothing visibly happening.** This is the
default and it is the dangerous one. A user who presses "Mark as moderated" and
observes nothing forms one of two beliefs, and the software chose which: that the
post is now hidden for the Stoa's readers, or that the app is broken. **The first
is worse, and on this screen it is the more likely**, because a real moderation
would also change nothing visible here — the confirmation does not list its own
results. A moderator who believes a post is hidden stops dealing with it.

**Considered and rejected: disable the controls.** A greyed-out button says "not
available to you", which is a claim about authority. Nothing answers whether this
peer may moderate this Stoa — `getModerationCapability` is designed in PLAN.md
§9.1 and does not exist — so a disabled control would assert something the
software has not checked, on the one screen where a user could act on it to their
cost. Live-looking controls with an honest notice claim less than disabled ones.

**Why it is a requirement rather than a comment:** the notice reads as clutter to
anybody who has not worked out the above, and the buttons read as merely
unfinished. Without a contract, the first person tidying the screen deletes the
sentence that makes it safe. **Removing the notice turns
`tst_moderation_screen.qml::test_the_screen_states_that_it_publishes_nothing`,
`::test_the_notice_names_the_missing_core_method_as_the_reason` and
`::test_the_notice_says_the_lists_are_examples` red** — that is what the guard is
worth.

### D2 — The screen makes no core call at all, and that is contracted separately

**Chosen:** `moderation-view` requires that no control call the core and that no
reply be rendered as one, over and above D1's requirement that the screen say so.

The two look redundant and are not. D1 is about what the screen *says*; D2 is
about what it *is*. Without D2, a screen that made a call and discarded the reply
would satisfy D1 while its inertness became a property of its current bindings
rather than of what it is — and the first person to wire one control would find no
contract telling them the others are inert on purpose rather than unfinished.

`DModerationScreen.qml` contains no `Core.` at all, which is visible to the
reachability gate's singleton walk as well as to a reader.

**Testing it needed a recording fake, and a null implementation would defeat a
weaker one.** A fake answering every method identically cannot distinguish "no
call was made" from "a call was made and ignored". The house fake in
`tst_navigation.qml` records every call, so
`::test_nothing_on_the_moderation_screen_calls_the_core` presses every inert
control and compares the call count before and after. It also asserts the count
was **non-zero before** — the feed did call the core — so a zero afterwards means
this screen made none, rather than the fake having been unreachable.

### D3 — `enterOnly` replaces five hand-written clear-the-others blocks

**Chosen:** one `stateNames` list plus `enterOnly(name, payload)`, with every
transition naming only the state it enters.

**This is the refactor that makes the change easy, and it was not speculative.**
The navigator's five states each had a setter naming the other states and setting
each to `null` — a guard that must be got right at every call site, which is the
shape CLAUDE.md says to replace with one the data enforces. Adding this change's
sixth state required editing five functions.

**It was already wrong when I got here**, which is the argument for reshaping
rather than adding a sixth correct edit. `openThread` and `createIdentityFor` had
been written naming only three of the four sibling states, so the invariant held
by accident of which screens could reach which rather than by anything in the
code. The new state made `createIdentityFor`'s omission reachable and live.

**What breaks without it:**
`tst_navigation.qml::test_entering_onboarding_from_moderation_clears_the_moderation_state`
fails against the pre-reshape file — `moderating` stayed set beside `onboarding`,
with `screenShown` resolving to "onboarding" only by ternary order.
`::test_no_transition_leaves_two_states_set_at_once` derives its count from
`stateNames` rather than restating four property names, so a seventh state is
covered the moment it is added; the restated version counted four and would have
reported "one state set" with `moderating` sitting beside it.

**`openThread`'s half is satisfied by construction rather than tested**, and the
test says so rather than claiming otherwise: it refuses outright when `chosen` is
null, so it is unreachable from moderation and its missing clear could never
fire.

**A rejected alternative worth naming:** a `StackView`. It was rejected before
this change for a reason this change does not alter — a push/pop lifecycle
alongside the state properties is a second source of truth that can disagree with
them. `enterOnly` keeps one source and removes the per-site guard, which was the
actual complaint.

### D4 — `DModeratedList` is extracted, and a `Repeater` replaces a `ListView`

**Chosen:** one component for a heading plus a bounded block of rows plus the
separator rule, with the row body passed as a delegate; rows built by a
`Repeater`.

**The extraction:** the two lists differ only in what a row contains. Written
twice, a change to the separator logic has to be made twice and will be wrong in
one of them — which is how two lists come to disagree about something neither is
about. A `kind: "author" | "post"` branch inside one fixed shape was rejected for
the same reason: one component with two jobs, and a branch to get right at each of
the six elements that differ.

**The `Repeater`, and this is a correctness fix rather than a style choice.** A
`ListView` instantiates only the delegates it currently needs, and inside a layout
not yet given its height it needs one. **Measured on Qt 6.10.3:** with a two-row
fixture and `Layout.preferredHeight: 150`, exactly one row existed —
`::test_the_two_lists_are_separate_with_their_own_controls` failed with actual 1,
expected 2. That is this repo's silent-failure house style reaching the layout: a
row present in the model and absent from the screen, nothing logged.

**The trade, which is real:** a `Repeater` has no virtualisation, so a long list
builds every row. These are fixtures of two. When a core listing arrives and the
list can be long, this becomes a `ListView` again — and whoever makes that change
must give it a height that exists before the delegates are asked for, or re-create
the defect.

**Row data reaches the delegate as a property on the delegate's own root**, not
through `parent`. The first version put it on the `Loader`, which made the
delegate's children write `parent.parent.rowData` — a chain whose correct length
depends on how deeply a child happens to be nested, so moving an element one level
in silently reads the wrong object with nothing failing.

### D5 — The row count placeholder is a fixed string, never a derived one

**Chosen:** `counts not yet available`, constant, marked at the site that renders
it.

**A derived number was the tempting option and is the one the amended requirement
forbids by name.** A page length from another call looks like a total, is not one,
and would be wrong by an amount that grows with the Stoa. Worse, it *moves with
the data* — so a reader comparing two rows would be reading a real signal that
means something other than what the row says. **A placeholder that does not move
is honest about being a placeholder in a way a derived wrong number is not.**

**A numeral was rejected even as a constant.** "0 posts" would satisfy "does not
move" and still assert something false about every Stoa on the list.
`tst_stoa_screens.qml::test_the_row_count_placeholder_claims_no_measurement`
asserts no digit appears and that emptiness is not claimed, then drives two
fixtures differing in every quantity a screen could reach for — the number of
Stoas listed and the number of threads another call answered — and requires the
rendered string to be identical.

**The two existing count assertions survive the amendment unchanged**, which is
the useful confirmation that the narrowing was narrow.
`::test_no_row_renders_a_count_of_held_posts_or_anything_global` and
`::test_no_digit_is_rendered_that_the_reply_did_not_supply` both still pass,
because a placeholder rendering no numeral satisfies both. If a later change
renders a number there, both go red — which is the intended outcome, since the
amendment permits a placeholder rather than a number.

### D6 — The route in is not gated on whether this peer may moderate

**Chosen:** a `MODERATE` link in the feed's ordering row, always offered.

Nothing answers whether this peer may moderate a Stoa. `getModerationCapability`
is designed and absent, and `stoa-membership` states that a listed Stoa means the
user chose it rather than that the user governs it — the retained creator key is
never re-checked against the peer's current signing key, so a peer whose key
changed holds Stoas it created and can no longer moderate, silently and
indistinguishably.

**So a gate here would be a guess, and a guess in this position tells a user they
moderate a Stoa.** Offering the route claims nothing; hiding it on a guess would.
This is the same reasoning `stoa-navigation-view` applies to the create
affordance, which is likewise offered whatever the keystore's state.

## Reasoning moved out of `docs/PLAN.md`

Per the flow's PLAN-sheds rule, reasoning this change acted on moves here rather
than being left in two places. What moved:

- **Why the screen's controls being inert is a hazard rather than a neutral
  placeholder.** PLAN.md §6 and §9.2 ruling 3 now record *that* the owner
  reversed the ruling and *that* `moderation-view` contracts the account the
  screen must give; the argument for why — that a moderator who believes a post
  is hidden stops dealing with it, and that on this screen a successful
  moderation and a dead control look identical — lives in D1 above.
- **Why a derived count is worse than a constant one.** PLAN.md's case 2 entry 3
  now names the placeholder and the file that marks it; the argument that a
  moving wrong number is worse than a static honest one is D5.

**What stays in PLAN.md deliberately:** the rulings themselves, the case 2 list,
and ruling 2's reasoning about peer-local state. Those are scope and what is not
built yet, which is what PLAN.md is for — not reasoning this change consumed.

## What the gates here cannot see

- **No basecamp launch was performed.** The screen has not been seen rendering.
  Every claim is from the static gates and `qmltestrunner`, and the visual match
  to the reference is by reading the reference's CSS rather than by comparing
  pixels.
- **The reachability gate proves the screen is instantiated somewhere the root
  reaches, never that a user can get to it at runtime.** A screen mounted behind
  a condition that is never true passes it.
  `tst_navigation.qml::test_the_feed_offers_a_route_into_moderation` drives the
  affordance, which is the closest a component suite gets; a real launch is
  `piece/e2e-sitometres`'s.
- **The QML suite runs with the host absent**, so it is blind to a host type-name
  collision by construction. That is the `D` prefix's job and
  `check_qml_names.py`'s, not a test's.
- **One test in `tst_stoa_screens.qml` did not appear in the run at two different
  names, and the cause was not established.** Folding the assertions into a
  function that demonstrably runs is what made them execute. The comment there
  says so rather than naming a cause, after an earlier draft blamed a runner
  enumeration limit on the strength of a miscount — `grep -c "    function
  test_"` reads those words inside a comment as a declaration.
