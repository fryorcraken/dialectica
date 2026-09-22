# Design — the moderation screen, inert

## Context

**The owner reversed the third of four MVP scope rulings, on its screen half
only.** That ruling had read: "moderation stays out, and the MVP ships no
moderation screen" — the reasoning being that §6's moderation design is
blocked at the contract rather than merely deferred, since the `Dialectica`
trait exposes no moderation-publishing method and there is nothing for a
screen to call. The owner decided the MVP ships screen 07 anyway, with every
control calling nothing, because they want to see it. **The publishing half
of the ruling is untouched**: the trait still exposes no moderation-publishing
method, so there is genuinely nothing for the controls to call — only the
"and ships no screen" clause was reversed.

Two merged requirements also had to move. The scope rulings were never
license to override a permanent requirement on their own — a fourth ruling
governing all of them said so explicitly: **a scope note does not override a
merged requirement; where a requirement forbids something, the requirement
governs, and relaxing it is a spec change rather than a scope note.** This
change is that mechanism working as intended, not an exception to it: the
owner wanted the design bundle's row counts, a scope note alone could not
deliver them because `stoa-navigation-view`'s "every number rendered is one
this peer can actually answer" forbade it, so the requirement itself was
amended through a reviewed spec delta.

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
peer may moderate this Stoa — a `getModerationCapability`-shaped call was
proposed for this and does not exist on the `Dialectica` trait today — so a
disabled control would assert something the software has not checked, on the
one screen where a user could act on it to their cost. Live-looking controls
with an honest notice claim less than disabled ones.

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

**This diverges from the design bundle, and the divergence is stated here rather
than left to be discovered.** `tmp/ui-bundle-new/handoff/README.md` line 40 is
one of the four "rules easiest to break while implementing": *"Never show a count
of anything global. Every number counts what this machine holds."* The amendment
that permits this position to be filled reconciles the spec
(`stoa-navigation-view`) and the MVP scope ruling that a Stoa row's count
position ships filled with a marked placeholder (see "Reasoning moved out of
PLAN.md" below), and says nothing about the bundle — so a reader holding both
documents would not learn from either that they now differ on this exact
point.

**What the owner reversed, and what they did not.** The reversal was of the
MVP scope ruling whose subject is the moderation screen: it was reversed so
the screen could be built and *seen*. Nothing in it addresses counting. So the
honest statement is that the bundle's rule 3 has **not** been overridden by
anyone, and this change does not treat it as overridden.

**The reason no conflict arises in fact is that the placeholder is not a count**
— `counts not yet available` renders no numeral and asserts no quantity, which is
precisely what D5 chose it for. The bundle forbids showing a count that is not
this machine's; a string saying a count is unavailable shows no count at all.
Every rule survives: the bundle's, and the spec's permanent prohibition on
global counts.

**Where it would become a real divergence** is the moment anything renders a
number in that position. If a future change fills it from a core call, the
bundle's rule is the one to re-read first, because the spec's amended form
permits a number that the bundle's rule may still not — and that is a question
for the owner rather than for whoever writes the call. Recorded here because the
amendment's own stated goal is that "a reader ... finds the reasoning in the
requirement itself", and a silent gap is the one way that goal fails.

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

### D7 — The no-authority test asserts a grammatical property, not a phrase list

**Chosen:** a second-person marker within 3 words of a governance noun, over
every rendered string on the screen.

**What it replaces, with the number that condemned it.** The first version held
four hardcoded sentences (`you are a moderator`, `you moderate`, `your stoa`,
`as a moderator`). Code review measured its gap: rewording the notice to *"THIS
STOA IS UNDER YOUR GOVERNANCE"* — a plainer claim of authority than any of the
four — passed the full file, **13 of 13**. A blocklist fails on a re-word and
not on the claim, which is backwards for a test whose subject is what the screen
asserts about the reader's standing.

**Why a window and not plain co-occurrence**, which was tried first and is the
obvious form. The notice body legitimately reads *"...publishing a moderation,
so every control here is inert: acting on one changes nothing, for you or for
anyone else"* — pairing `moderation` with `you` at 15 words' distance while
claiming the exact opposite of authority. An unwindowed version failed on it.
The discriminator is adjacency: a disclaimer mentions both halves and keeps them
apart, a claim puts them side by side. 3 spans `you are a moderator` and
`stoa ... is under your` and stops well short of 15.

**What breaks without it.** Reverting to the phrase list turns no test red —
that is the whole point, and why the guard is worth recording. What proves the
new form works is the pair of mutations run against it: the review's exact
rewording now fails with `'your' stands within 3 words of 'stoa'`, and the old
blocklist's `YOU ARE A MODERATOR HERE` fails with `'moderator' stands within 3
words of 'you'`. Both were reverted; `git diff` on the screen is empty.

**Rejected: asserting the absence of a predicate**, which is what the review
suggested and what would be strongest. QML string matching cannot ask whether a
sentence claims capability over the Stoa. The grammatical pairing is the nearest
checkable proxy, and its limits are written at the test rather than left to be
found: it is blind to a claim using neither marker ("this peer governs here"),
to one split across two `Text` elements, and to one spaced wider than the
window. All three are narrower holes than four strings, and none is reachable by
re-wording the sentences the screen actually has — which the blocklist's gap
was.

### D8 — `rowData` cannot be `required`, and the comment now says why

**Chosen:** a plain `property var rowData` with a default of the shape the
delegate reads, and a comment stating the constraint rather than asserting the
opposite.

**The comment was wrong in two ways**, which review caught: it claimed the
`Loader` supplies a `required property var rowData` when neither delegate
declares `required`, and credited `setSource`-style initial properties when the
code does an imperative `onLoaded` assignment.

**`required` is not merely unused here — it cannot work.** A `Loader` builds its
component first and emits `onLoaded` second, so there is no point at which a
required property could be supplied. Measured on Qt 6.10.3 by adding `required`
to the author delegate: `Required property rowData was not initialized` per row,
and `test_the_two_lists_are_separate_with_their_own_controls` saw **0 rows where
it expects 2**. So the review's first option (add `required` to both) is not
available, and correcting the comment is the whole of the fix.

**The trade this records:** `Loader` buys the two lists a shared body and gives
up the construction-time enforcement that `required property var modelData` has
two lines above it. An unbound delegate renders blank rather than erroring. That
is a real cost, stated at the site, and the reason the defaults are the shapes
the delegates read rather than empty objects.

### D9 — A vanished test is now a failed run, without needing its cause

**Chosen:** `check_every_test_ran` in `run-qml-tests.sh`, comparing the test
names a spec declares against the names the runner reported.

**The incident it answers had no root cause and still does not.** Two tests were
written in `tst_stoa_screens.qml`, one never appeared in the run under two
different names, and folding them together is what resolved it. Review flagged
leaving that at "cause unknown" as the wrong place to stop, and it is — but the
useful response is a check that does not depend on the diagnosis. Comparing
declared names to run names fires on a test lost for *any* reason, including one
nobody has identified.

**One mechanism is now reproduced, and it is not that incident.** QtTest treats
`test_foo_data()` as the DATA PROVIDER for `test_foo()`. Declaring both removes
**both** from the run: the `_data` body is called as a provider, returns
undefined, and the other is skipped for want of rows. Measured on Qt 6.10.3 as
`3 passed, 0 failed` with neither executed, the only trace a `WARNING: ... no
data supplied` line — not a failure, and not something `check_bindings` looks
for. **No `_data` name appears anywhere in `tst_stoa_screens.qml`'s history**,
so this is the same defect *shape* rather than the cause, and it is recorded as
such.

**Two hypotheses tested and ruled out**, written down so the next person does
not spend the afternoon again: a **duplicate function name** fails loudly at
compile (`Duplicate method name`), and a **helper sharing a test's prefix or
taking an argument** runs normally — both measured with probe specs.

**What breaks without the guard.** Deleting `check_every_test_ran` turns the two
end-to-end cases in `tst_check_every_test_ran.sh` red: the sound spec must still
be accepted, and the `_data` spec must be rejected. The fixture cases pin the
regexes either side of two mistakes already made building it — reading any
`::name()` rather than result lines only (which let the skipped half of the
`_data` pair read as having run), and a `[^:]*::` name extraction that matched
nothing because `qmltestrunner::<Case>::` contains colons itself, so every test
read as missing. A check that reports everything missing is as useless as one
that reports nothing, and both directions are pinned.

## Reasoning moved out of `docs/PLAN.md`, and what replaced it

Per the flow's PLAN-sheds rule, reasoning this change acted on moved here
rather than being left in two places. What moved:

- **Why the screen's controls being inert is a hazard rather than a neutral
  placeholder.** The MVP scope ruling below (and §6's moderation design)
  record *that* the owner reversed the ruling and *that* `moderation-view`
  contracts the account the screen must give; the argument for why — that a
  moderator who believes a post is hidden stops dealing with it, and that on
  this screen a successful moderation and a dead control look identical —
  lives in D1 above.
- **Why a derived count is worse than a constant one.** The case-2 entry
  below names the placeholder and the file that marks it; the argument that a
  moving wrong number is worse than a static honest one is D5.

**What follows was deliberately left in `docs/PLAN.md`** — the rulings
themselves, the case-2 list, and ruling 2's reasoning about peer-local
state — because those are scope and what is not built yet, which is what
PLAN.md was for rather than reasoning this change consumed. Now that
PLAN.md is gone, that content has no other home, so it is quoted here in
full rather than left dangling.

### The MVP scope rulings this screen answers to

Four owner rulings governed what the MVP ships, recorded because scope
decided in a conversation is scope the next agent re-derives from the design
documents — and the design documents argue for building all of it. None of
the four is a design change; each is a decision about what ships first.

1. **Voting is out of the MVP and out of the UI target.** The relevance
   design (see `relevance-votes/design.md` §0) stays as the design; its MVP
   membership does not. From the contract side this is the same fact: core
   computes exactly one ordering and takes no `order` argument, so "by
   relevance" is not implementable today.
2. **Unread counts are out of the MVP, amended: the count is out, the
   rendered position is in.** Unread needs peer-local state that is not a
   projection of ops, and inventing the first instance of that as a feed
   field is how it gets designed badly. What the owner reversed is narrower
   than it sounds: a Stoa row renders an unread *position* carrying a
   placeholder, so the screen can be seen as designed; `listThreads` still
   gains no `unread` field, no peer-local state is built, and nothing
   computes a count. **This entry is not worked off by a core change** — no
   call arriving removes it; only a decision to build peer-local state does.
3. **Moderation stays out; the MVP ships the moderation screen, inert.**
   The trait exposes no moderation-publishing method, so there is nothing
   for a screen to call — that half is unchanged. The screen half was
   reversed by the owner, who decided the MVP ships the screen with every
   control calling nothing, because they want to see it. Moderation itself
   is still out: no op is published, nothing is hidden, no feed is filtered.
4. **The core contract is the UI authority.** Where the design bundle asks
   for something core cannot honestly serve, the bundle is amended — core
   does not grow to satisfy a mockup. From this, a two-case rule for any
   control on a screen:
   - **Case 1 — core serves it: the control MUST be wired.** Inertness is
     not an acceptable shortcut where the call exists; an unwired control is
     a defect rather than a phase.
   - **Case 2 — core cannot serve it: an inert control or a placeholder
     value is acceptable for this MVP phase, on one condition — it is
     documented.** The documentation requirement is what makes it
     acceptable: an undocumented placeholder is indistinguishable from one
     nobody noticed.

   **Two merged requirements narrow case 2, and a scope note does not
   override them:** `stoa-navigation-view`'s "every number rendered is one
   this peer can actually answer" forbids a rendered count on the Stoa list
   and the join preview (amended by this very change, as described above,
   to permit a marked placeholder); `composer-view`'s "the vote control
   displays no score" forbids a rendered score on the vote control,
   unamended. Case 2 permits a placeholder only where nothing forbids one.

### The case-2 list: documented placeholders for this MVP phase

Each entry names what core cannot serve and the file that establishes the
absence. An entry is removed when core grows to serve it, not when it is
disproved by inspection.

1. **No score anywhere.** Nothing reads `Vote` ops, so no feed row carries a
   score.
2. **Exactly one ordering, and no `order` parameter.** There is no ordering
   parameter, no comparator to select. "By relevance" is not implementable.
3. **No post count on a Stoa row** — amended by this change. The position is
   now filled with a marked placeholder (`counts not yet available`),
   per the owner reversal and the amended `stoa-navigation-view`
   requirement. **Unread is on this list too**, and it is the one entry no
   core change works off: it is excluded by decision (ruling 2 above), not
   by a missing call, so no core change removes it — only a fresh scope
   ruling reopens it.
4. **History is kept, but no contract method reads earlier versions.**
   Superseded post versions stay in the op log; no method reads them.
5. **The thread screen's vote control is inert.** `publish_vote` exists, so
   it could be wired — but voting is out of the MVP by ruling 1, and a
   working vote control on one screen would be the one place in the
   interface where the ruling is contradicted by a live affordance.
6. **No moderation-publishing method**, per ruling 3. This is now an
   ordinary case-2 entry with controls on screen that call nothing — the
   `DModerationScreen.qml` controls this design covers.
7. **No moderated-author and moderated-post lists.** Nothing enumerates what
   a Stoa has moderated; the trait exposes no listing call, only a
   per-target resolution. `DModerationScreen.qml` renders fixtures held in
   the view for this reason.

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
  names, and the cause is still not established.** Folding the assertions into a
  function that demonstrably runs is what made them execute. The comment there
  says so rather than naming a cause, after an earlier draft blamed a runner
  enumeration limit on the strength of a miscount — `grep -c "    function
  test_"` reads those words inside a comment as a declaration.

  **A recurrence is no longer silent**, which is the part that changed: D9's
  `check_every_test_ran` fails any run in which a declared test did not execute,
  whatever the cause. The file's 76 declared tests all run today, measured. What
  the gate still cannot do is explain the original loss, and it does not claim
  to.
- **The no-authority test checks grammar, not meaning.** D7 states its three
  blind spots at the test itself. A screen that claimed authority without a
  second-person marker would pass it.
