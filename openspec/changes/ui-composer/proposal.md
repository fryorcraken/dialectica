# The composer: posting, replying and voting from the view

## Why

**Core can publish and the view cannot ask it to.** `publish_post`,
`publish_reply` and `publish_vote` are on the module surface and the
`content-authoring` spec contracts all three. `Core.qml` wraps `list_threads`
and `get_capabilities` and nothing else, so none of the three is reachable from
QML. `Main.qml` says so in a comment: "no composer". `VoteControl.qml` exists
and is instantiated nowhere.

That makes MVP items 3, 4 and 5 — post, reply, upvote/downvote — unreachable
from the interface even though every core method they need is merged and tested.

This change is the view half, and it is not a wiring job. Three of the
obligations `content-authoring` and the UI brief record are **discharged only by
the interface**, and each is a decision this spec has to take rather than
inherit:

- A publish reply carries **no delivery outcome by design**, so the view must
  report a success without saying the post was sent or that anyone can see it.
- `wasNew: false` means the op already existed. It is neither a failure nor a
  fresh success, and the failing design is the one that reports success and
  shows nothing new.
- **A vote publishes an op that nothing reads.** No call returns a score, a
  tally, or the viewer's own prior votes. The control must not imply a ranking
  it does not produce, and it must not teach the user the app is broken.

## What Changes

- A new `composer-view` capability: what the view must show before, during and
  after a publish, and what it must refuse to claim.
- **`Core.qml` gains three wrappers** — post, reply and vote — so the publish
  path is reachable from QML through the single call path that already carries
  the one error branch.
- **The gate extends to every posting affordance**, not just the feed's reply
  box. `FeedScreen.qml:417-444` already renders the `canPost: false` branch with
  the reason verbatim and no text field; that pattern is followed rather than a
  second one invented. What is added is the **open** branch, which does not exist
  anywhere today, and the **fix affordance** rule 2 asks for alongside the reason.
  The gate's heading is also corrected: the bundle's names only replying and
  promises delivery — see the copy audit below.
- **A vote control that shows no number.** `VoteControl.qml` is instantiated with
  its `score` left unbound and the number suppressed — see Decisions. Its
  existing properties and its zero floor are not restyled.
- **Body length is bounded in the composer, in bytes, before submission**, and the
  bound is made visible as the user types rather than on refusal.
- **The composer does not sanitise what the user typed.** Core's `sanitise`
  module runs on the way *out* to a reader and has no inbound wire method; the
  view warns and publishes the bytes as typed. See Decisions.
- **A draft survives every refusal**, and so does a retry. Stated as a
  requirement rather than left to implementation, because the refusal most likely
  to occur — a reply whose parent has not propagated — is the one expected to
  succeed on a retry, and because the view cannot tell that refusal from a reply
  to a non-post. See Decisions.
- `docs/UI-BRIEF.md` is corrected where this change makes it wrong: it claims
  "the feed offers two orderings" and that a count is "not available yet"
  alongside a Composition section that predates the merged publish contract.

## Capabilities

Each claim below was checked against the files named, not recalled.

### New Capabilities

- `composer-view` — what the view does around a publish: which affordances are
  gated on the probe and which are not, what a draft is owed, what may and may
  not be claimed after a publish succeeds, how `wasNew: false` is reported, how
  each publish refusal is distinguished for a reader, and what the vote control
  may display given that nothing reads a vote.

### Modified Capabilities

None. This was checked rather than assumed:

- **`content-authoring` — no delta.** Every requirement here is about what the
  view does with a reply core already contracts. Nothing about the request
  shapes, the refusals or the reply fields changes. A delta would restate a
  requirement in order to leave it as it was.
- **`posting-capability` — no delta, and the probe's contract is load-bearing
  here.** Two of its existing requirements do this change's work already:
  "The reason names the fix" is why the view renders the reason verbatim rather
  than rewording it, and "Reason text is for a reader, not for a caller to match
  on" is why the view's fix affordance **must not** be selected by matching on
  the reason's prose. The second is a constraint this change obeys, not one it
  changes.
- **`module-wire-contract` — no delta.** The composer is a caller of the
  envelope, not a change to it.

## Decisions

### The vote control shows the viewer's own vote and no number at all

This is the contentious decision and the one to scrutinise hardest.

**The state of the world, verified rather than recalled.** `publish_vote`'s reply
is `{"opId","wasNew"}` (`lib.rs:213-224`, and `content-authoring`'s "The reply
describes no effect"). `list_threads`'s rows carry `thread`, `currentVersion`,
`author`, `body`, `attachments`, `isRevised`, `isHidden` (`wire.rs:1455-1477`) —
no score, no vote count, and no record of the viewer's own votes. There is no
thread read call at all. So **no core call returns a number the control could
show**, and none returns the viewer's prior votes either.

PLAN.md's record of this names two honest options: a visible per-post tally that
is not a ranking, or a control whose copy does not overstate its effect. **The
first is unavailable** — there is no tally to show — which leaves the second, and
the question becomes what the control displays in the number's place.

**Rejected: render the score as zero.** `VoteControl.qml` computes
`Math.max(0, root.score)` with `score` defaulting to `0`, so instantiating it
unmodified prints "0" beside every post. That is the worst available answer: it
is a number, so it reads as a tally, and it reads as the tally *zero* — a claim
about how this post was received that core never made and that is false the
moment any peer has voted. An absent number says "not known here"; a zero says
"known, and nobody voted".

**Rejected: hide the control until a score exists.** This is what §9.1 originally
argued for, and the owner's scope decision overrode it. It is out of scope to
re-litigate.

**Chosen: the arrows, the viewer's own vote as button state, and no number.** The
one thing that is true, immediate and verifiable is that the user pressed a
button and the interface remembers — which is exactly what UI-BRIEF's vote
section lists as "safe". The number is suppressed rather than zeroed, and the
suppression is a requirement so that a later change exposing a count has to
decide to show it.

**The cost is named rather than absorbed.** The viewer's own vote is remembered
only for votes this view published while it is open. Nothing reads votes back, so
after a reload the control forgets — and the interface must not pretend
otherwise by, for example, leaving a pressed arrow pressed across a reload. A
control that forgets is honest about a peer that does not yet read votes; a
control that appeared to remember would be the view inventing state.

**Why `score` stays on the component.** Deleting the property would make the
later arrival of a count a change to the component and to every call site.
Leaving it unbound makes it a change to one binding. This is the bundle's own
instruction — "bind the number to a single score property so the ranking behind
it can change without touching the control" — applied to the case where there is
nothing to bind yet.

### A successful publish claims storage, never delivery

`content-authoring` requires the reply to carry no delivery outcome, and PLAN.md
records why the return value could not carry one even if wanted: delivery's
outcome arrives as an asynchronous event after the call returns, and a sink
accepting an op means the transport took it, not that any peer received it.

So the view may say the post was **saved on this machine** and must not say
sent, delivered, published to the Stoa, or that anyone else can see it. The
honest statement is weaker than users expect, which is the point — making an op
that never propagated visible is `op-transport`'s obligation and does not exist
yet, so an interface claiming delivery today would be claiming something no part
of the system checks.

### `wasNew: false` is reported as a distinct third outcome

Not a failure — nothing failed and the op is in the log. Not a fresh success —
the interface would be reporting a new post and showing none, and the user
concludes their post vanished. The brief names both failure modes and this spec
requires a third message, distinguishable from the other two, that says the
content was already published.

The alternative the brief also permits — scroll to and highlight the existing
post — is **not required here**, because the view cannot reliably find the
existing post: the feed lists thread heads, so a deduplicated *reply* has no row
to scroll to. Requiring a behaviour the view can perform for one of the two
publish kinds would be a requirement half of the surface cannot meet.

### The two reply refusals cannot be told apart by the view, so neither is treated as permanent

`content-authoring` requires the reply refusals to be distinguishable — "no such
op is held" from "the op held is not a post" — and the brief asks the first to be
presented as temporary and nobody's fault, with the draft preserved.

**The view cannot act on that distinction.** It arrives only as differing prose
inside `module-wire-contract`'s single error shape, which carries no
machine-readable discriminant, and `posting-capability` establishes for the
probe's reasons that a caller must not branch on message wording. A first draft
of this spec required the view to distinguish them anyway, which would have been
a requirement no test could satisfy honestly — the defect this project's spec
review looks for hardest.

So the spec requires the reading that is safe under either: **every** reply
refusal keeps the draft and leaves a retry available. The asymmetry decides it —
offering a retry that cannot succeed costs one press, while withholding one from
the parent-not-yet-arrived case discards the thing most likely to work and
misreports a temporary condition as permanent.

**This names a real gap rather than closing it.** Distinguishing them needs a
discriminant on the wire, which widens the core contract and belongs in its own
change. Recorded here so the coarser behaviour is not later read as an oversight.

### The composer publishes what the user typed, unsanitised, and says so

Core's sanitiser is display-side: `sanitise()` runs when a stored op is rendered
into a reply (`wire.rs`'s `sanitised_json`), and no wire method sanitises an
inbound string. `content-authoring` is explicit in the other direction — "Text a
caller supplies SHALL reach the op unchanged" — because an op is signed over its
bytes and transforming them would mean the op published is not what the caller
supplied.

So the composer **must not** strip or rewrite the user's text: doing so would
publish something the user did not write, permanently and under their signature.
What it can do is tell them what they are about to publish. The spec requires a
warning when the draft contains the characters the sanitiser would mark on the
way out, so the one person who can still change the text — the author, before
signing — is the one told about it.

**The asymmetry is deliberate and worth stating**: peer text is sanitised on
display because the reader cannot consent to it; the author's own text is not,
because they can.

### The byte cap is shown as the user types, not discovered on refusal

The cap is 150 KiB per variable-length field (`op.rs:146`), shared by the format
and the publish path. **Core does not expose the number** — no wire method
returns it — so the view cannot ask for it and any value it shows is a second
copy that can drift.

The spec therefore requires the *behaviour* (the user learns before submitting
that the draft is too long, and the draft is not truncated) without requiring the
view to hardcode the number. Whether the view hardcodes 150 KiB with a comment,
or a method is added to core to report it, is `design.md`'s — and the second is
worth considering, because a view-side copy of a cap is exactly the "two values
that agree today" shape `content-authoring` already refused once.

**The cap is bytes, not characters**, and a composer counting characters
under-counts every non-ASCII draft. The spec states the unit so a length
indicator cannot be written against the wrong one.

### A published post appears only after a reload, and the view says which

Optimistic insertion would put a row on screen that the view composed rather than
read back from core — and the fields a feed row carries include sanitiser counts
and a revision flag the view would have to invent. Inventing them is how a view
comes to render something core never said.

So: no optimistic row. The success message names what happened, and the feed is
re-read. If the re-read does not show the post — which can happen, since a reply
is not a thread head and has no row in this feed — the message is still correct
about what occurred, which is why the message must not be phrased as "your post
is now below".

### Divergences from the mockup, which is a visual and copy reference

The bundle's screens show functionality core does not have. Recorded so a
reviewer comparing the two does not read a deliberate omission as a defect:

- **Screen 04 and 05 show a vote score of `12`** beside every post. No call
  returns one; see the vote decision above.
- **Screen 04 shows "14 replies"** on a thread head. `list_threads` returns no
  reply count.
- **Screen 04 shows three orderings.** Core computes one, and `FeedScreen.qml`
  already renders one from a model for that reason.
- **Neither screen shows an open composer.** Both show the gate shut, so the
  bundle supplies verbatim copy for the blocked state and none for the open
  one. Copy for the open state is therefore specified by obligation — what it
  must and must not claim — rather than by a verbatim string the bundle does not
  contain.

### The bundle's compose copy was audited, and two of its strings are not shipped

The bundle is a copy reference whose claims are **not all true of this system**,
so every string this change would pin was checked against PLAN.md and the merged
specs rather than taken on trust. Two failed, both making a claim about what the
software guarantees.

**`compose.noKeystore` and `compose.badPermissions` both promise delivery.** Each
ends *"the reply box comes back when a reply would actually send"*. The probe
establishes whether a publish would be accepted and **stored locally**; nothing
in this system checks whether anything sends, and PLAN.md is emphatic that
publishing and delivering are two events at two times with delivery not yet
wired. The string promises the second on the strength of the first.

Neither ships, and the reason is structural rather than a copy edit: the spec
requires the view to render **the reason the probe supplied**, and
`posting-capability` already requires core's reasons to name a fix. Core's text
replaces the bundle's at both call sites. That was originally decided to avoid
maintaining the same guidance twice; the audit gives it a second and independent
justification, which the spec now states — a view-supplied reason is one nobody
checked against what the probe can establish.

**`compose.blockedTitle` is wrong in two ways.** *"You cannot reply in this Stoa
yet"* names only replying, while the gate withholds posting too; and the bundle
pairs it with the delivery promise above. The spec therefore requires the
heading to name the affordance actually withheld and forbids any part of the
closed gate from claiming a submission will send, rather than pinning the string.

Also noted, though outside this change: `onboarding.body` claims the key "cannot
be linked to you anywhere else", which PLAN.md says explicitly must not be
claimed of the MVP, and `onboarding.apparatus.uniqueness` says "three words"
where PLAN.md settled four. Both are `ui-onboarding`'s to handle.

**What is still required verbatim**, each carrying no guarantee claim:
`compose.fix` ("Show me how to fix it"), `compose.apparatus` (a statement about
this interface's own design, and true of it), and the `sanitiser.*` strings,
which report counts core itself supplies and are already used by
`SanitisedText.qml`.

## Impact

- New `openspec/specs/composer-view/spec.md` on archive.
- `dialectica-ui/src/qml/Core.qml` — three wrappers on the existing `call()`.
- `dialectica-ui/src/qml/` — a composer component, and the first instantiation of
  `VoteControl.qml`. Components came from the bundle and are reused, not
  restyled.
- `dialectica-ui/tests/` — the QML test layer, which `tst_feed_states.qml` shows
  can drive a real screen through a fake bridge.
- `docs/UI-BRIEF.md` — corrections listed under What Changes.
- **No change to the core module**, its wire surface, or any core spec. Whether
  core should expose the body cap is raised in Decisions and deferred to
  `design.md`.
