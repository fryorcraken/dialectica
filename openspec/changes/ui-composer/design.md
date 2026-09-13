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

Chosen: `FeedScreen.ownVotes`, a plain object keyed by the row's `currentVersion`
(the op the vote targets), holding −1 or +1. A per-post key makes cross-marking
unrepresentable rather than checked — there is no code path that could write one
post's vote into another's slot, because the key *is* the post.

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

`compose.apparatus` moves into the apparatus column where it belongs, verbatim.

### There is no reply composer on the feed, because the feed has no thread view

The spec contracts replying and the composer supports it — `Composer.kind` is
`"post"` or `"reply"`, and the reply path passes a `parent`. But `FeedScreen`
renders **thread heads**, and a reply box under a thread head would be replying to
a thread's current version from a feed row, which is a thread-view affordance on
a screen that is not one.

So `Composer.qml` is built for both and `FeedScreen` instantiates the post one.
The reply instantiation arrives with the thread screen, which is a different
change. The component is tested in both modes, so the reply path is exercised
rather than merely written.

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
