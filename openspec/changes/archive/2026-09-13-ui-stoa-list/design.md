# Stoa list and join screens — design

## Context

See `proposal.md` — *Why*, and its three divergences from the mockup. The spec
(`specs/stoa-navigation-view/spec.md`) is the contract. What follows is only what
the code had to decide that neither document settles.

Four constraints shape every decision below.

1. **`list_stoas` returns `{"stoa","foundingTitle"}` and no genesis record.** The
   core retains the record; the listing does not hand it back. Verified against
   `dialectica/rust-lib/src/lib.rs:170`. So the view holds a record only for a
   Stoa it *just created or just joined in this session*, and the spec's share
   and feed-navigation requirements are conditional on that.
2. **Basecamp gives the QML engine no network and no filesystem.** Any clipboard
   has to come out of the Qt modules CI installs — `qt6-declarative` plus the
   `qtquick`, `qtquick-controls`, `qtquick-layouts`, `qtquick-templates` and
   `qtquick-window` QML modules (`.github/workflows/ci.yml`). `Qt.labs.platform`
   is not among them.
3. **The wire carries bare hex, with no `stoa:` prefix.** `join_stoa` takes
   `{"stoa":"<hex>","genesis":"<hex>"}`, and **no core path produces or accepts a
   `stoa:` display prefix on an address** — grep `dialectica/` for `stoa:` and
   the one hit is `rust-lib/src/lib.rs`'s `error_json(&format!("stoa: {e}"))`,
   an error-message prefix rather than a wire format. (An earlier version of this
   paragraph offered that grep as returning nothing, which it does not; the
   conclusion is unchanged and the citation was wrong.) The mockup's `stoa:b02d…`
   is a display flourish, which `AddressLabel.abbreviate` already tolerates via
   its optional `^([a-z]+:)?` group. Nothing the view sends back to core may
   carry one.
4. **No core call answers a founding title for a reference this peer has not
   joined.** `join_stoa` is the only method that returns a `foundingTitle` for a
   given `(stoa, genesis)` pair — `list_stoas` answers for Stoas already held and
   `create_stoa` for one just made, and neither can be asked about a pasted
   reference. The title *is* inside the genesis record the user pasted, so this is
   a gap in the API rather than in the data; closing it in the view would mean
   decoding core's genesis encoding in QML, which is a second implementation of an
   encoding whose one authority is the core. That is the constraint the core/UI
   split exists to hold, and the same argument that keeps address verification out
   of `StoaReference.parse`. **D11 is what this forces, and it is the largest
   single consequence in this document.**

## Goals / Non-Goals

**Goals**

- Two screens — the list and the join preview — plus a create affordance and a
  share affordance, all reachable from `Main.qml` without a developer-supplied
  Stoa.
- A share format whose output is exactly the input the paste field accepts, so
  the round trip is one decision rather than two that can drift apart.
- Three read states on the list that cannot collapse into each other, in the
  shape `FeedScreen` established.

**Non-Goals**

- **No core change.** The `list_stoas` gap is reported, not closed.
- **No second address abbreviation.** `AddressLabel` owns the 8-8-6 form.
- **No current title, no per-row held-post count, no `nothing received yet`.**
  Divergences 2 and 3 in the proposal; the positions are left empty rather than
  filled with something else.
- **No restyle.** Every visual decision comes from `Theme` and the existing
  components.

## Decisions

### D1 — The shareable thing is a JSON object, one line, bare hex inside

`{"stoa":"<full hex>","genesis":"<full hex>"}`, serialised with
`JSON.stringify`, is what the share produces and what the paste field parses.

**Why JSON.** It is the one encoding both ends of this round trip already have —
`JSON.parse` in the view, `serde` in the core — so neither end needs a parser
written for this feature, and a parser written for this feature is a parser that
can disagree with itself between the share and the paste. It is also
*self-describing*: a user who pastes half of it gets a parse failure naming the
input as malformed, where a positional format (`stoa:<hex>:<hex>`) would silently
accept a truncated second field as a short record and push the failure into the
core, which would then refuse it as a verification mismatch — the one failure the
spec requires be kept distinct from a malformed paste.

**Alternatives considered.**

- *A `stoa:` URI carrying both halves* — `stoa:<addr>?g=<genesis>`. Prettier, and
  it matches the mockup's visual. Rejected: it needs percent-decoding and a query
  parser in QML, both hand-written, for no property JSON does not already have.
  The mockup's `stoa:` string is a *display* abbreviation of the address alone,
  which the spec forbids as a shareable thing regardless, so matching it visually
  buys nothing.
- *The address and record as two separate paste fields.* Rejected: it doubles the
  number of ways a user can pair the wrong halves, and the whole point of the
  share is that one copy produces one paste.
- *Base64 of the JSON.* Rejected: it makes the address unreadable in the shared
  string, and the address is the half a recipient is supposed to be able to eye
  against the one they were expecting.

**The prefix is stripped on the way in, never added on the way out.**
`StoaReference.parse` accepts an address written `stoa:ab12…` because a user may
have copied one from a screen, and normalises it to bare hex before it reaches
core. `StoaReference.shareText` writes bare hex. A prefix that survived into `join_stoa` would be
a hash that verifies against nothing, surfacing as a verification failure —
exactly the wrong one of the spec's three outcomes.

**Stripping must be a loop, not a pass.** The first implementation stripped one
prefix, so `stoa:stoa:<hex>` forwarded a prefixed address and produced precisely
the verification accusation this paragraph says cannot happen — the invariant the
comment asserted was not the one the code enforced, found in review rather than
by the test, which only ever supplied one prefix. The defect was a *fixed number
of passes*, so a second `if` would have been the same defect one step further
out. It also meant the preview rendered `stoa:<hex>` as "the address in full", so
the screen whose job is showing the address exactly was showing something else.

### D2 — The clipboard is a hidden `TextEdit`, selected and copied

`ClipboardSink.qml`: a zero-size, non-visible `TextEdit` with a `copy(text)`
function that assigns `text`, `selectAll()`, `copy()`, `deselect()`.

**Why.** `TextEdit` is in QtQuick proper, which the view already imports, so this
adds no module to the flake and nothing to CI's install list. `Qt.labs.platform`'s
`Clipboard`, and a `QClipboard` exposed from C++, both would; and the view has no
C++ at all, which is the property the core/UI split exists to preserve.

**Why a component rather than a call at each site.** `AddressLabel.copyRequested()`
has three receivers in this change (a list row, the preview's address, the create
outcome's address) and the spec requires what is copied to differ from what is
*displayed* at two of them — the display is abbreviated, the copy is the full
share string. Putting the sink in one file means the "what actually gets copied"
decision is made once, and `copyText` stays the one property that answers it.

**What this cannot do, and the test says so.** Under `QT_QPA_PLATFORM=offscreen`
there is no system clipboard, so a test can assert *what the sink was asked to
copy* and not *what landed on the clipboard*. The sink therefore records its last
argument in a `lastCopied` property — which is the testable half — and the
unverifiable half is named in the test file rather than asserted around.

### D3 — Each screen holds one `readState` string, as `FeedScreen` does

`"unread" | "ok" | "failed"` on the list; `"empty" | "previewing" | "malformed" |
"joining" | "joined" | "failed"` on the join screen.

The spec's hardest requirement on the list is that three outcomes never render
alike, and the reason `FeedScreen` uses one variable rather than several booleans
is that no combination of flags can then put two states on screen at once. The
same argument applies here and applies harder on the join screen, where the
states include *two different failures that must look different from each other*.
A `malformed` that is a separate string from `failed` cannot accidentally render
through the same branch; a pair of `isMalformed`/`hasError` booleans can, and the
bug would be a missing `&& !`.

`joined` is a state and not a boolean for the same reason: the spec forbids
rendering success on the strength of having dispatched the call, and a state
machine that can only be in `joining` *or* `joined` makes that a property of the
shape rather than of remembering to guard.

### D4 — The "same title, different Stoa" comparison is over titles only

`JoinScreen.lookalikes` filters the held listing for `foundingTitle === preview's
foundingTitle && stoa !== preview's stoa`.

The spec calls this out as the one place consulting the held listing is required,
distinguished from the inference it forbids elsewhere. The code keeps them apart
structurally: `lookalikes` is computed from titles and never read after a join,
and the join outcome is computed from `reply.ok` alone and never reads
`lookalikes` or `heldStoas`. Two functions, neither of which can reach the other's
input.

The address-equal case (`stoa === preview's stoa`) is excluded by the filter, so
"a Stoa you already hold at the same address" renders no lookalike panel — which
is the spec's second scenario, satisfied by construction rather than by a guard.

### D5 — `Main.qml` becomes a two-state navigator, and its Stoa properties go

`Main.qml` keeps one property, `chosen`, which is either `null` or
`{stoa, foundingTitle, genesis}`. The feed renders only when it is non-null.

The spec forbids a defaulted Stoa property on the top-level view, and the reason
is that a second source for the value is a build that can ship a hardcoded Stoa.
Removing the properties is what makes that unrepresentable — there is no longer a
place to put one. `chosen.genesis` is `""` where the view holds no record, which
`FeedScreen` already passes through to core unchanged; the core then refuses it
and the feed renders that refusal, which the spec names as the honest outcome.

**`Main.qml` does not gain a `StackView`.** It is in QtQuick.Controls, which CI
does install — but a two-screen navigator whose entire state is "is `chosen`
null" needs no stack, no history and no transitions, and a `StackView` would add
a push/pop lifecycle that can disagree with `chosen` about which screen is up.
One `visible:` binding each cannot.

### D6 — `Core.qml` gains exactly three wrappers, and `perPage` is the view's

`createStoa(title)`, `joinStoa(stoa, genesis)`, `listStoas(page, perPage)`.

The spec requires each core method be named in one place, which is what these
are for. `listStoas` takes `perPage` from the caller rather than defaulting it
inside the wrapper, because the wrapper's job is to name a method and shape a
request — choosing how many rows a screen shows is the screen's job, and a
default buried in the wrapper is a number two screens would silently share.

## Risks / Trade-offs

- **The share affordance is absent on most rows, and that looks like a bug.** →
  It is the spec's required rendering, and the absence is explained in the
  apparatus column rather than left to be inferred. The real fix is the core
  piece that widens the listing item; until then the honest rendering is the one
  that cannot produce an unjoinable string.
- **A user who joins a Stoa can share it, and after a restart cannot. A user who
  CREATES one cannot share it at all, from the moment it exists.** → Same cause,
  same fix, but the two are not symmetric and an earlier version of this line
  said they were. `create_stoa` returns `{stoa, foundingTitle, policy}` and no
  genesis record, so the view never holds one for a created Stoa — the
  degradation bites immediately rather than at the next launch. Noted here
  because it is the shape in which the gap will be reported as a defect, and the
  report will be correct; and because the milder framing makes the creation case
  look like it works until someone tries it.
- **`TextEdit.copy()` is untestable headless.** → The sink records what it was
  asked to copy, the tests assert on that, and the test file says plainly which
  half is unverified. The alternative — asserting nothing and claiming coverage —
  is the defect family this repo has shipped before.
- **The lookalike panel reads the listing, which may have failed.** → When the
  listing failed, `heldStoas` is `[]` and no lookalike is shown. That is a
  false *negative* (a real lookalike goes unmentioned), never a false positive,
  and it is the safe direction: a panel that appeared because of a failed read
  would be asserting a comparison nothing performed.

### D9 — Every screen owns its way out, and the two navigator states exclude each other

`FeedScreen` gained a `closed()` signal and an unconditional "All Stoas" button;
`Main.qml` gained `preview()`, `open()` and `closeFeed()`, and the first two each
clear what the other owns.

**Arriving somewhere is half a transition.** `FeedScreen` had no signals at all,
so nothing could clear `chosen`, and the first row a user opened was the last
screen they saw until they restarted — taking the list, the share affordance and
the join field with it. 103 tests passed while that held, because a suite that
asserts up to a transition and nothing after it cannot see a one-way trip.

Three decisions inside that:

- **A signal, not a direct write.** The feed does not know what is above it, so
  the caller decides what "back" means. That keeps D5's argument intact: the
  navigator still holds one nullable property per screen and needs no
  `StackView`.
- **The affordance is unconditional.** The state a control exists to leave must
  not be the state that withdraws it. The feed's own read can fail, and that is
  when a user most wants out.
- **The test asserts the control, not the signal.** A `closed()` signal nothing
  renders a button for is a route only a test can take, and would satisfy a
  weaker assertion while leaving the user equally stranded.

**`chosen` and `previewing` are now mutually exclusive**, which is the second
half. `screenShown` is an ordered ternary, so a preview requested while a feed
was open was silently swallowed — latent today, because nothing on the feed emits
one, but the spec's own model is that an address inside a post is an affordance a
reader acts on, and a post lives on the feed. The setters clear each other, so
the state `(chosen ≠ null, previewing ≠ null)` cannot be constructed and the
ternary renders the state rather than resolving a clash. Functions rather than a
comment explaining the precedence: a comment would have documented a trap instead
of removing it, and the precedence was an accident of ordering rather than a
decision worth recording.

### D10 — `visibleRows` carries the read-state guard, `lastListing` holds the raw answer

`StoaListScreen.rows` became `lastListing` (the last answer, current or not) plus
a derived `readonly visibleRows` that is empty unless `readState === "ok"`.

A failed reload deliberately does not blank the previous page — a failure must
not destroy a good listing underneath a banner — so the screen held rows that
`readState` said were not read, and safety depended on **every reader**
remembering `readState === "ok" ? rows : []`. That guard was already written
twice, in two files: once on the `Repeater` model at the far end of the file from
the state making it necessary, and again in `Main.qml`. (This said "213 lines"
and the file gives 283 — `git show a9888f8:dialectica-ui/src/qml/StoaListScreen.qml`
and grep `readState`. The argument never rested on the digit, so the digit is
gone rather than corrected; a number nobody re-derives is a claim that rots.)

Two copies is CLAUDE.md's signal to let the data absorb it; a third reader would
have had to know to write it a third time, and the one who forgets renders a
listing the screen has said was unread.

The rename is the load-bearing half. `rows` invited the wrong read; `lastListing`
makes a reader ask "last as of when?" before using it. Both call-site guards are
now gone rather than merely correct.

### D8 — An outcome is stored with the reference it describes, not beside it

`JoinScreen` holds one `outcome` object — `{stoa, genesis, state, failure,
foundingTitle}` — and `joinState`, `failure` and `foundingTitle` are `readonly`
properties derived through `currentOutcome`, which yields `null` unless the
stored pair equals the pair on screen.

**This replaced three independent mutable properties, and the reason is a defect
found in review.** `Main.qml` ships ONE reused `JoinScreen`; `stoaAddress` is a
binding on `previewing`, while the outcome was separate state. So previewing a
second reference moved the address and left the verdict behind: paste a
legitimate reference, join it, paste an attacker's, and the attacker's address
rendered under a "Joined." panel with the join button gone and `join_stoa` never
called for it. The previous Stoa's founding title came along too, under "FIXED
FOREVER".

**Why not `reset()` on navigation**, which is the obvious fix and the wrong one:
a reset must be *remembered*, at every present and future entry point, and the
one place it is forgotten is a screen making a claim about the wrong Stoa. The
derivation makes a stale outcome unrepresentable rather than unlikely — there is
no variable that can hold one Stoa's verdict while another is displayed. This is
CLAUDE.md's "complexity in the data structure, not the logic", and the same
argument that put the `-1` sentinel in the onboarding screen.

Two consequences worth recording:

- **An outcome legitimately survives re-previewing the identical reference.** The
  core answered for that exact `(stoa, genesis)` pair, so reporting it is
  reporting a fact. My first regression test asserted the opposite — that Cancel
  should wipe it — and failed against the fix; the fix was right and the test was
  wrong. The invariant is "an outcome describes exactly the reference it was
  returned for", never "clear on navigation". `test_an_outcome_survives_re_previewing_the_very_same_reference`
  pins this so nobody later "fixes" it into a wipe.
- **The pair, not the address, is the identity.** A different genesis against the
  same address is a different reference and inherits nothing, because the pair is
  what the core verified.

### D7 — Absence assertions scan the card body, never the apparatus column

`tst_stoa_screens.qml` has a `bodyText(screen)` helper, and every "the screen
does not say X" assertion runs over it rather than over `visibleText(screen)`.

**The apparatus column is annotation explaining the design, not interface.** It
reached the shipped view by mistake and a separate piece is removing it. On a
rendered join screen it is 747 of 1371 characters — measured, not estimated — so
a whole-screen scan is more than half margin note, and an absence assertion over
it would silently prove less the day the column goes while its name went on
claiming the same coverage.

Two things follow, and the second is the one that actually bit:

- `bodyText` subtracts the apparatus rather than walking a named body child.
  `ScreenFrame` exposes its body as a default-property alias with no
  `objectName`, so a structural lookup would break silently when that structure
  changed; subtraction breaks loudly instead.
- **An absence assertion is only as strong as its corpus, and a corpus with no
  candidate in it proves nothing.** `test_nothing_on_the_preview_promises_a_per_stoa_identity`
  originally scanned a *fresh preview*, whose body says nothing whatever about
  what joining does — the only sentence on that subject was in the apparatus.
  Planting "generates you an identity for it alone" in the joined panel left the
  assertion passing. It now drives the screen to the joined state and asserts the
  corpus contains the honest sentence before asserting the false one is absent.

This is the same family as the four instances found in review — assert the
property the user is affected by, not the value feeding it — with the corpus
playing the part the binding plays elsewhere.

### D11 — The preview has no title, and says so rather than captioning a blank

`JoinScreen`'s founding-title panel is conditional on `titleKnown`
(`foundingTitle !== ""`), and a `titleUnknownNote` renders in its place. Both
bind to the one derived property, so they cannot both show or both hide.

**This is the resolution of a defect a design review found by driving the real
screen, and the defect is the most serious thing this change shipped.** D4's
`lookalikes` returns `[]` when `foundingTitle === ""`, and D8 made `foundingTitle`
a derivation of `currentOutcome`, which is `null` until a join reply exists. Each
decision is right on its own; together they meant the lookalike panel — the
impersonation defence, whose entire purpose is to warn a reader **before** they
commit — could only render **after** they had committed.

Measured through `Main.qml`'s real paste route, a held *Nym Research* in the
listing and an attacker's same-titled reference in the field:

```
before join():  foundingTitle=<>              lookalikes=0  panel=0  heldStoas=1
after  join():  foundingTitle=<Nym Research>  lookalikes=1  panel=1
```

The warning was real and arrived one action too late to be a warning.

**Why the suite did not see it, and this generalises past this change.** Every
lookalike and founding-title test reached the screen through
`joinComponent.createObject(null, {foundingTitle: …})`. QML accepts a props value
as a **readonly** property's initial value, so those fixtures hold a title
alongside a `null` outcome — **a pair the shipped screen cannot construct**. They
proved something true of a state that does not exist. That is this repo's one
test defect family in its purest form: a fixture where two explanations give the
same answer. The two new tests go through the paste route instead, and both were
watched failing before this decision was written.

**"Revert D8" is not the fix**, and checking that mattered: `git show 8de7571`
has the same structure, with `foundingTitle` written only inside `join()`. D8 made
the impossibility permanent rather than introducing it.

**Alternatives considered.**

- *Relax D4's guard so the comparison runs on an empty title.* Rejected, and it
  is the trap rather than the near miss: `""` matches every untitled Stoa in the
  listing, so the panel would fire on Stoas that present no lookalike at all —
  a false positive on the one screen whose warnings must be believed.
- *Carry a title over from the last outcome when this reference has none.*
  Rejected for the reason D8 exists: it captions an untrusted address with a name
  the user already trusts, which is the impersonation the panel exists to expose,
  delivered by the view itself and invisible to the comparison because both sides
  would be the same string from the same source.
- *Decode the genesis record in QML to read its title.* Rejected — constraint 4.
  A second implementation of core's encoding, in the module that has no business
  holding one.
- *Leave the panel empty and say nothing.* Rejected, and this is the option the
  branch shipped. `FOUNDING TITLE — FIXED FOREVER` over an empty value tells a
  reader this Stoa's founding title is **blank** — which is a legal value the
  list renders, so the reader cannot tell it from "not known here", and the two
  mean opposite things on the screen where they decide whether to trust an
  address. Equally, an absent lookalike panel reads as "checked, nothing found";
  a reader who infers that has been misled by a check that never ran, which is
  the impersonation arriving *through* the defence rather than around it.

**What actually closes it is a core change this piece does not make**: a call
that answers a founding title for an un-joined reference — `getStoa` in PLAN.md
§9.1's shape, or a narrower `describe_reference` taking the pair and decoding
without recording membership. Until one exists, the honest rendering is the one
that states the absence of a check instead of letting its silence be read as its
result. **The check is late, not missing**, and the screen now says which.

## Behaviour the spec did not decide

The choices below are observable behaviour the spec is silent on. Each is marked
`NO SPEC:` in the code, and each is a decision the spec-writer should evaluate
rather than a settled one — `grep -rn "NO SPEC:" dialectica-ui/` is the list, not
a count written here. (It said "Four" and the code marked five; the page size was
missing, which is the one with the most visible consequence of the set.)

- **The reference encoding.** The spec requires both halves and names no format.
  D1 chose JSON. This is the one with a compatibility cost: a user who has
  already copied a reference holds a string in this shape, so changing it later
  strands them. It is worth being in the spec.
- **An empty paste field.** Refused as not-a-reference, rather than the button
  quietly doing nothing. The spec's three paste outcomes do not cover it.
- **A creation success carrying no `stoa` field.** Treated as a failure. The
  spec requires the returned address be rendered and does not say what happens
  when there is none to render.
- **A reference half that is present but is not a string** — a number, an object,
  an array. Refused rather than coerced, so nothing but a string reaches the
  core. The spec says the view's check is limited to "whether the input carries
  the two halves at all", which does not settle what "carries" means for a
  non-string.
- **The listing's page size.** `StoaListScreen.perPage` is 25, chosen to fill a
  card without a scroll at the mockup's 1000px width. It decides how many Stoas a
  user sees before paging, which is the one user-visible consequence in this
  section — the others are all failure-handling — and the spec names no page size
  for this listing.

## Open Questions

None. The one genuinely open item — whether `list_stoas` gains the retained
genesis record — is a core change this piece does not make, and the spec is
written to be correct either way.
