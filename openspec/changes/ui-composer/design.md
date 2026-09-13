# Design — the composer view

## What this change touches

`dialectica-ui/` only. No core method is added, no wire shape changes, and no
core spec is edited. The three publish methods already exist on the module
surface (`dialectica/rust-lib/src/lib.rs:257,272,285`); what did not exist was
any way to reach them from QML.

Four files change and two are new:

| File | What |
|---|---|
| `src/qml/Core.qml` | three wrappers on the existing `call()` |
| `src/qml/Composer.qml` | **new** — the open-gate composer, one component for post and reply |
| `src/qml/PublishOutcome.qml` | **new** — the three-outcome message, rendered from one value |
| `src/qml/FeedScreen.qml` | the open branch of the gate, the corrected closed branch, vote controls on rows |
| `src/qml/VoteControl.qml` | the number becomes suppressible; nothing else moves |
| `src/qml/qmldir` | registers the two new components |

## Decisions

### The delivery denial is its own element, keyed on "not a refusal"

The spec requires every success to **positively state** that whether any peer has
received the content is not something this software can report. A prohibition is
discharged by silence; this one is not, because a reader who sees a post submit
successfully assumes it went somewhere, and an interface that merely declines to
mention delivery leaves that assumption standing while being fully compliant with
every other delivery rule.

The first implementation made the denial the tail of the `stored` arm of the
qualifier's ternary — which made a requirement owed by *every* success into one
branch's copy. The `existing` branch, which is a success by this component's own
design (`isRefusal` is false, nothing failed), carried no denial at all, and
review found it.

So the denial is now its own `Text`, visible on `!isRefusal`. **Hanging a
requirement off one arm of a conditional is how it goes missing from the other;
hanging it off the condition that defines who is owed it is how it cannot.** The
same reasoning is why it is not in the apparatus column — see the closed-gate note
below.

### The one-value invariant belongs in the component, not only in its caller

The section below argues that one string makes two outcomes on screen at once
impossible. **That argument was about `Composer.outcome` and it held only
there.** `applyReply` is total — every exit sets one of four strings — but
`PublishOutcome` is a separately registered QML type whose `outcome` is a public
writable property, and it partitioned the value space **twice, along different
seams**: three elements asked "is it one of the two successes?", two asked "is it
the one refusal?".

Those are different partitions, so a value in neither positive case satisfied
both negations and rendered the refusal headline above both success sentences —
"Your post was not published." directly above "It is in this machine's log." —
with core's `detail` suppressed, because that element was gated the other way. A
reviewer measured it for `"deferred"`, `"Refused"`, `"refused "` and `"stored "`,
and confirmed a fourth outcome added to `applyReply` left the whole suite green.

**The lesson is about where an invariant lives, not about a missing branch.** A
component whose correctness is a property of who calls it is correct by luck; the
thread screen will be the second caller when the reply composer lands there, and
it would have inherited the rendering without the guarantee.

So `PublishOutcome` now computes `state` once — a total classification whose
fall-through is `"refused"` — and every element keys on it. The fall-through
direction is the honest one: an outcome the component does not understand is one
where it cannot claim anything was stored, and claiming less than happened is
recoverable where claiming storage that did not happen is not.

### The three publish outcomes are one value, not three booleans

The spec's hardest requirement to hold by construction is "the three outcomes are
mutually distinguishable" plus "an already-published op is not an error state".
Written as flags — `succeeded`, `wasNew`, `failed`, `message` — that is four
variables with sixteen combinations, thirteen of which are nonsense, and the
thirteenth that eventually renders is a success message beside an error border.

So the composer holds **one** `outcome` string, exactly as `FeedScreen.readState`
already does for the read:

    ""         nothing submitted yet
    "stored"   wasNew true  — saved on this machine
    "existing" wasNew false — already published
    "refused"  anything else — `outcomeDetail` carries core's message

`PublishOutcome.qml` renders from that one string. Two states cannot be on screen
at once because one variable cannot hold two values, which is the same argument
`FeedScreen`'s header comment makes for `readState` and the reason to copy the
shape rather than invent a second one.

**The refused state is the default for anything unrecognised.** Every path that
is not a parsed success with an `opId` string falls into `"refused"` — an
unreachable bridge, non-JSON, core's error shape, a success object with no
`opId`. That is the spec's "a publish SHALL NOT be reported as successful on a
reply the view could not interpret", and it holds because the success branch is
guarded on `typeof reply.value.opId === "string"` rather than on the absence of
an error.

### `wasNew` is read as `=== false`, not as falsy

`wasNew` absent and `wasNew: false` are different facts, and the shape that
distinguishes them is worth being deliberate about. A reply carrying an `opId`
and no `wasNew` is a success shape the view does not fully understand.

Chosen: treat `wasNew === true` as "stored", `wasNew === false` as "existing",
and **anything else as refused**. A missing `wasNew` is a reply that is not
core's success shape for this call, and the spec routes those to refused. The
alternative — defaulting a missing `wasNew` to one of the two successes — would
pick an outcome on no evidence, and the user would be told either that a post
was saved or that it already existed with nothing behind either claim.

This is stricter than it has to be today (core always emits the field) and the
strictness is the point: it is the same "does not rest on a guarantee made one
module away" argument `FeedScreen`'s `items` guard already makes.

### The draft is cleared on a newly stored op and kept in the other two cases

Three outcomes, three fates for the draft, and they are **not uniform**. Cleared
on `wasNew: true`; kept on `wasNew: false`; kept on every refusal. The asymmetry
is the decision rather than an inconsistency, and it is worth stating because it
looks like an oversight from either end — someone tidying toward "always clear on
success" or "never clear" would be making it uniform in a direction that is
wrong.

**Clearing on a newly stored op removes an affordance that is ready to produce a
confusing outcome.** The same text resubmitted is the same op id, so the second
submission is a deduplicated no-op reported as "already published" — a state the
user reached by using a control that looked ready to publish something. Nothing
is lost by clearing, because the text is published and readable.

**Keeping it in the other two cases is the same reasoning applied to different
facts.** On a refusal nothing was published, so the draft is the only copy. On a
deduplicated publish nothing new was written, and a user whose intention was to
publish something *different* needs the text in front of them to edit — clearing
takes away exactly what they need.

**The cost, named rather than absorbed:** a user writing a near-identical
follow-up loses their starting point and retypes it. That is a convenience,
weighed against an interface offering a control whose use produces a confusing
no-op.

**This was decided before the spec contracted it, and the record was in the wrong
place.** The argument lived only in a code comment marked `NO SPEC` — which said
the opposite of the truth once the spec grew the requirement, and told the next
reader an unmade decision sat there for them to change. Two reviewers found the
same gap from opposite directions: the design reviewer that `design.md` carried
no entry, and the spec-test reviewer that mutations inverting **both halves**
left the suite green. Unrecorded and unpinned together is a decision made by
accident, which is what this entry and the tests the tester added now close.

### The vote path ignores `wasNew` where the composer refuses on it

Two publish paths in this change take **opposite policies on the same field**,
and the divergence is deliberate. The entry above argues that a composer reply
carrying an `opId` and no `wasNew` is not core's success shape for that call and
must be refused, and calls the strictness the point. `voteOn` accepts exactly
that shape: it requires `reply.ok` and a non-empty string `opId`, and never reads
`wasNew`.

**What makes a vote different is that it has no third outcome to distinguish.**
For a post, `wasNew` decides between two *user-visible* messages — "saved on this
machine" and "already published" — and the whole reason the field is read
strictly is that guessing wrong tells the user a post exists that does not, or
sends them looking for one that will never appear. A vote has nothing
corresponding. Published once or published twice, the viewer's vote is on record
and the control shows the same thing, so the field carries no information this
view would render. Reading it strictly would mean refusing to show a vote back
that *was* recorded, which is worse on the only axis available.

The spec does not settle this — `grep -n "wasNew" spec.md` returns nothing — so
`design.md` is the only place it can live. Without the entry a reader who finds
the `wasNew === false` rule and then reads `voteOn` sees a rule followed at one
call site and missed at another, and cannot tell a deliberate divergence from a
missed one.

### The undo press publishes nothing, and the arrow stays pressable

`VoteControl` emits `voted(0)` when the viewer presses the arrow they already
voted. `FeedScreen` drops it: `if (direction !== 0)`.

**Core has no vote retraction**, so there is no op that means "I take that back".
The two alternatives someone will reach for are both wrong, and wrong in a way
the code cannot show on its own:

- **Publish the opposite direction.** A down-vote is not a retracted up-vote. It
  is a second, different, signed assertion, and a later scorer reading the log
  would count it as one.
- **Publish the same direction again.** It deduplicates to the op already
  published, so nothing changes and the user is told nothing — an action that
  looks like it did something.

**The cost, named rather than hidden: the arrow stays pressable and the press
does nothing at all**, not even a message. That is a dead affordance and it is a
real cost, accepted because the alternatives publish something false. The honest
repair needs a retraction op, which is a core change.

**This is the second place the same gap shows.** Moderation reversibility is
suspended for want of a value that would let an Unhide reverse a Hide; vote
retraction is that gap again, in the view. A reader finding one should be able to
find the other, which is why both are named here.

### The byte count is computed, and the cap is not a number in this file

The spec requires the limit in **UTF-8 bytes**, and requires the behaviour
without requiring a hardcoded number. Two problems, two separate answers.

**Counting.** A QML string is UTF-16, so `.length` is code units and undercounts
every non-ASCII draft — for a CJK draft by a factor of three. `Composer.qml`'s
`utf8Length()` walks code points and adds 1/2/3/4 by range, treating an unpaired
surrogate as 3 bytes (what a replacement character costs).

Rejected: `encodeURIComponent(s).replace(/%../g, "x").length`. It is the
one-liner everybody reaches for and it **throws** `URIError` on an unpaired
surrogate — which QML's `TextArea` will happily hand you mid-edit from a paste or
an IME. A length function that can throw is a length function that can leave the
submit button in whatever state it was in when the exception unwound.

**The number.** `MAX_BODY_LEN` is 150 KiB (`authoring.rs:206`, itself
`op::MAX_FIELD_LEN`) and **core does not expose it** — no wire method returns it,
and the spec's Decisions raise adding one as worth considering. It is not added
here: it would widen the wire contract for a number that has never moved, and the
spec explicitly scopes this change to the view.

So the view carries the number once, as `Composer.bodyByteLimit`, a property with
a comment naming `authoring.rs:206` as the value it copies. One property rather
than a literal at each call site, so the day core does expose it, this is one
binding. The spec's requirement is on the *behaviour* and the tests assert the
behaviour (over-cap refuses, multi-byte counts as bytes) rather than the number,
so a drifted cap breaks a test about the number and not the whole suite.

### The delivery defect is not papered over, and is not routed around either

A body at exactly `MAX_BODY_LEN` encodes to roughly 153,740 bytes once `Post`'s
fixed overhead is added, and the transport caps the **encoded message** — so a
body at the cap is accepted, signed, stored, and refused by every receiving peer,
silently. This was measured on `main` by a reviewer on another branch and
independently confirmed.

**This change does not close it and must not appear to.** Two things follow:

- The view does **not** add a client-side guard refusing long-but-legal posts.
  That would convert a silent delivery failure into a loud publish refusal, which
  trades one wrong behaviour for another and hides the defect behind the view.
  Recorded as rejected so it is not re-proposed as an obvious fix.
- The success message says the content was **saved on this machine** and claims
  nothing about delivery. That is required by the spec for its own reasons and it
  happens to be the only honest thing to say about a post in this range. It
  mitigates the user-facing half and closes nothing.

The fix is core's: the cap must be on the encoded op, not on the body. It is
outside this change's scope and outside `dialectica-ui/` entirely.

### The composer warns about invisibles and publishes them anyway

`content-authoring` requires the body to reach the op unchanged, because an op is
signed over its bytes — a composer that stripped a zero-width space would publish,
under the user's signature, something they did not write.

So `Composer.qml` **counts** what core's display sanitiser would remove, using the
same character set (`sanitise.rs:141`'s `is_invisible`), and shows the count as a
warning. The submit affordance stays available; the warning is information, not a
gate.

**The homoglyph half is deliberately not counted here, and this is the decision
most worth scrutinising.** `sanitise.rs` marks a character belonging to a minority
script among Latin/Cyrillic/Greek. Reimplementing that in QML would put a second
copy of a *judgement* in the view — one that has script ranges, a dominance rule,
and a tie-breaking rule, every one of which can drift from core's. The invisibles
list is a set membership test, which can also drift but drifts visibly (a
character is in the list or it is not) rather than producing a different number
from the same text.

The spec requires a warning "naming how many such characters were found" for
characters the sanitiser "would remove **or mark**". Counting only the removals
satisfies the removal half exactly and under-reports the marking half. That is a
gap and it is named rather than hidden: see NO SPEC in `Composer.qml`. The
alternative I rejected was a second homoglyph implementation in QML whose
disagreement with core would be invisible to both.

### The vote control's number is suppressed by a property, not deleted

`VoteControl.qml:34` is `text: Math.max(0, root.score)` with `score` defaulting
to `0`, so instantiating it unmodified prints "0" beside every post. The spec is
explicit about why that is the worst answer available: zero is a number, it reads
as a tally, and it reads as the tally *zero* — a claim core never made and one
that is false the moment any peer votes.

Chosen: a `showScore` property, defaulting **false**, that hides the number's
`Text` element. Three alternatives, and why each lost:

- **Delete the `Text`.** The later arrival of a count becomes a change to the
  component. The bundle's own instruction is to bind the number to one `score`
  property so the ranking behind it can change without touching the control; the
  property stays for exactly that reason.
- **Bind `score` to `-1` so the floor renders nothing.** `Math.max(0, -1)` is
  `0`, so this renders "0". It does not work, and it reads as though it does.
- **Default `showScore` to true and set it false at each call site.** One
  forgotten call site prints a fabricated tally. Defaulting to false makes the
  honest rendering the one you get by not thinking about it, and makes displaying
  a number something a future change has to *decide* — which is the spec's
  "the absence SHALL be a rendered decision".

`score` keeps its floor and its colour binding, untouched.

### The viewer's own vote lives in a map on the screen, keyed by op id

The control already has a `vote` property (−1/0/+1) for exactly this. What it
needed was somewhere to read from, and the shape of that store decides the
"a vote on one post does not mark another" requirement.

Chosen: `FeedScreen.ownVotes`, a plain object keyed by the op the vote targets,
holding −1 or +1. A per-post key makes cross-marking unrepresentable rather than
checked — there is no code path that could write one post's vote into another's
slot, because the key *is* the post.

**That argument was originally written against `row.modelData.currentVersion`
read directly, and in that form it was false.** Review reproduced both halves of
the failure on peer-supplied rows: two rows omitting `currentVersion` key the map
on the JavaScript value `undefined`, which stringifies to the single key
`"undefined"` — so they share one slot and a vote on the first reads back on the
second. Worse, `JSON.stringify` omits a key whose value is `undefined`, so the
request reaching core carried **no `target` field at all**: the view asked core to
vote on nothing and then marked two controls on the answer.

The flaw in the reasoning is worth naming because it is a shape that recurs: "the
key *is* the post" held only while every row carried a distinct key, which is a
property of **peer-supplied data**, not of the code. `FeedScreen.reload()`
validates that `items` is an array and nothing about the elements inside it, and
today's core always sends the field — which is precisely the "guarantee made one
module away" that the `items` guard a few lines above **already refuses to rest
on**. The view was refusing to trust the shape of one field of a reply while
trusting another field of the same reply.

So the invariant is now established rather than assumed: `voteTarget(rowData)`
returns the row's op or `""`, in one place, and both consumers — the control's
`vote` binding and `voteOn` — go through it. It guards the row object itself for
`null` and `undefined` before reaching any field, so it is a rule about rows
rather than a `currentVersion` special case. `voteOn` restates the guard at the
call rather than inheriting it from the binding, because it is reachable from
anywhere in the file and a second caller that skipped `voteTarget` would
reintroduce both failures silently.

**What a malformed row costs, and who bears it.** Three responses were available
and the choice is decided by that question rather than by tidiness:

- **Render the row, make its vote control inert** — chosen. The reader keeps
  peer content they were sent; the only thing withheld is an affordance that
  could not have worked.
- **Drop the row.** Rejected: it hides peer content on a censorship-resistant
  forum, and hides it **silently** — the reader cannot tell a Stoa with nothing
  in it from one whose rows this peer discarded. That is the confusion this whole
  screen exists to prevent, reintroduced one level down.
- **Fail the whole read.** Rejected: it lets any peer blank a feed for free by
  sending one bad row, and it collides directly with "empty and unreadable must
  never look alike" — a feed failing on a neighbour's malformed row reads as a
  broken store.

**"Drop the row" is the one that looks tidiest from inside the code**, which is
why the alternatives are recorded rather than left implicit. The next person who
finds a second malformed field will otherwise re-litigate this from scratch and
is likely to reach for it.

The control goes **non-interactive rather than absent**, so the layout does not
shift and the row does not silently lose a feature the reader can see on its
neighbours.

It is written **only** on a success outcome, so a refused vote leaves the map
untouched and the control shows what it showed before. And it is a plain QML
property with no persistence: a reload starts empty, which is the honest state
for a peer that cannot read votes back. Nothing writes it to disk and nothing
should — a control that appeared to remember across a reload would be the view
inventing state core never reported.

**QML property-change detection on an object does not fire on mutation**, so
`recordVote()` builds a new object and assigns it. Mutating in place would update
the model and repaint nothing, which is the failure mode where the feature looks
broken in exactly the way that is hardest to attribute.

### The probe reply is normalised at the boundary, like a row's vote target

`capability` used to hold **two differently-shaped objects**: `probe.value`
wholesale when the gate opened, and a constructed `{canPost, reason}` when it
closed. One property, two shapes, and the shape decided by a condition the reader
of the property cannot see.

The cost was already in the file. The closed gate's reason `Text` carried a
`!== undefined` guard that was **dead against the closed arm** — which always
supplied a string — and live only because the open arm could omit `reason`. QML
evaluates that binding even while the closed body is invisible, so removing the
guard emitted `Unable to assign [undefined] to QString` on fourteen tests, every
one an *open*-gate case. A reader following the comment beside it would have
concluded it was redundant and deleted it for the wrong reason.

That is CLAUDE.md's named shape: a guard restated per consumer because the data
structure does not hold the invariant. `capabilityFrom()` now constructs one
shape from the probe in one place, so `reason` is a string on every path, the
guard is **unnecessary rather than explained**, and a second consumer inherits the
invariant instead of rediscovering it.

**It is the same move as `voteTarget()`, one level up.** That one establishes a
row's op where the value is produced; this one establishes the probe's answer the
same way. Having made it for peer rows and not for the probe reply left the view
holding two positions about one reply — validating the shape of what core sends
in one place while taking it on trust in another.

An open gate also carries **no** reason now. There is no blockage to name, and a
leftover reason beside an open composer would describe a state the reader is not
in.

### The closed gate keeps `FeedScreen`'s existing pattern and corrects its copy

The closed branch already exists at `FeedScreen.qml:417-444` and already renders
the reason verbatim with no text field. It is kept, with two corrections the spec
requires and a third thing added:

- The heading. `compose.blockedTitle` is *"You cannot reply in this Stoa yet."*,
  which names only replying while the gate withholds posting too. Replaced with a
  heading naming both, and carrying no delivery claim.
- `compose.noKeystore` and `compose.badPermissions` are **not used at all**. Both
  end *"the reply box comes back when a reply would actually send"* — a promise
  about delivery that nothing in this system checks. The reason rendered is core's,
  verbatim, which `posting-capability` already requires to name a fix.
- The `compose.fix` affordance, which the spec requires alongside the reason and
  which the existing branch does not have.

`compose.apparatus` is rendered **in the closed gate's own body**, verbatim.

An earlier version of this note said it "moves into the apparatus column where it
belongs". That was correct against the spec as it stood and is now wrong twice
over. The spec was changed to require the statement "in the closed gate's own
body", and explicitly forbids discharging the obligation by "placing it anywhere
a reader of the gate would not encounter it".

**The reason is structural rather than editorial.** The `APPARATUS` column is
annotation explaining the design to a reader of the design; it reached the shipped
interface by mistake and is being removed. An obligation expressed as "this text
appears in that column" disappears with the column — silently, while still being
required — and review measured exactly that: hiding `ApparatusColumn` failed one
test, which failed because it asserted the *column's* string, so the requirement
would have gone unmet with nothing failing for the right reason.

The same reasoning moved the delivery denial out of the column and into
`PublishOutcome`, beside the success it qualifies. Nothing load-bearing is left in
the apparatus list: what remains there is context a reader may skip.

### There is no reply composer on the feed, because the feed has no thread view

The spec contracts replying and the composer supports it — `Composer.kind` is
`"post"` or `"reply"`, and the reply path passes a `parent`. But `FeedScreen`
renders **thread heads**, and a reply box under a thread head would be replying to
a thread's current version from a feed row, which is a thread-view affordance on
a screen that is not one.

So `Composer.qml` is built for both and `FeedScreen` instantiates the post one.
The reply instantiation arrives with the thread screen, which is a different
change.

**An uninstantiated component's defence is exactly how much of it runs, so the
tests are named rather than summarised.** "Tested in both modes" would survive a
future change deleting most of them; these five would not:

- `test_a_reply_still_carries_its_parent` — the parent reaches core
- `test_a_post_and_a_reply_call_different_methods_with_the_right_fields` — the
  method name and the absence of a `thread` field
- `test_no_reply_refusal_blames_the_user_or_claims_permanence` — both reply
  refusals, driven separately
- `test_the_two_reply_refusals_are_rendered_identically_but_for_cores_text`
- `test_a_post_and_a_reply_refusal_differ_only_in_the_subject_word`

The first three are in `tst_composer.qml`, the last two in
`tst_composer_claims.qml`. `test_a_post_given_a_parent_does_not_carry_it_anywhere`
covers the post side of the same `replyParent` derivation, so the branch is
pinned from both directions.

The count is stated as a list because a number goes stale silently: re-derive it
with `grep -rn 'kind: "reply"' dialectica-ui/tests/` rather than trusting this
paragraph. (The architecture review said "nine tests"; counting enclosing test
functions rather than instantiation sites gives these five, and the difference is
why the list is here instead of a figure.)

This is a scope boundary rather than a gap in the spec: every requirement about
reply refusals is about what the composer does with a refusal, and the composer
does it in either mode.

## What the tests can and cannot see

- **Can see**: the composer's state machine end to end through a fake bridge —
  every outcome, the byte counting, the draft's survival, the argument JSON
  actually handed to `callModule`, the vote map, and the absence of a number in
  the control's rendered text.
- **Cannot see**: that the rendering is legible, that the gate's heading reads
  well, or that a real `TextArea` in a real window behaves as the offscreen one
  does. QtTest here drives properties and signals, not pixels.
- **Cannot see**: the transport defect above. No test in this repo can, which is
  the whole problem with it.
