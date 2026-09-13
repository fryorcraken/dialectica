# Design — the onboarding view

## Context

Core's three onboarding methods are built and merged and no QML reaches them.
This change is entirely `dialectica-ui/`: three named wrappers on the `Core`
bridge, one new screen, and a launch branch in `Main.qml`. No core change, no
wire change, no version bump.

The reply shapes this view is written against, read from
`dialectica/rust-lib/dialectica-core/src/wire.rs` rather than from prose:

| call | success | the other success |
|---|---|---|
| `generate_identity_slate` | `{"slate":hex,"count":N,"candidates":[{index,path,address,publicKey}]}` | — |
| `keep_identity` | `{"kept":true,"address","publicKey","path","encrypted"}` | `{"kept":false,"reason"}` |
| `who_am_i` | `{"hasIdentity":true,"address","publicKey","path","recoveryNeedsTheRecord"}` | `{"hasIdentity":false,"reason"}` |

Two of the three have **two success shapes**, and that is the fact the whole
design turns on. `Core.call()` normalises a reply to `{ok:true,value}` or
`{ok:false,error}`; a refusal is `ok:true`. A view that branched on `ok` alone
would report an identity that was never stored.

## Decisions

### One `phase` string, not a set of booleans

`phase` is the single value the spec's last requirement asks for: *"which one is
determined by a single value rather than by a combination of independent
flags"*. It takes exactly one of

```
"intro"    nothing asked for, no candidates       (the opening state)
"slate"    candidates on offer
"refused"  a keep came back kept:false            (candidates still on screen)
"kept"     a keep came back kept:true
"failed"   the failure shape, or a reply this screen cannot read
```

Considered and rejected: `hasSlate` + `kept` + `failure` as three properties,
which is what the mockup's flat structure invites. `FeedScreen` already made
this decision for the same reason — two independent booleans is how a storage
failure eventually renders as an empty feed — and the spec restates it as an
explicit requirement here. With one string, "at most one of kept, refused and
failed is shown" is true by construction rather than by every `visible:` binding
being got right; the `visible:` bindings are all `phase === "…"` comparisons
that cannot overlap.

`"refused"` is a **phase of its own** rather than a flag alongside `"slate"`,
which is what makes the trap structural rather than remembered. The three keep
outcomes are three assignments to one variable, so there is no path on which a
refusal leaves `phase` at a value that renders a kept identity. `candidates` is
untouched by a refusal, which is how the spec's "a refusal leaves the candidates
on screen" holds without `"refused"` needing to carry them.

### The kept identity is a separate object, populated only from a kept reply

`keptIdentity` is `null` until a reply says `kept:true` **and** carries an
address; then it is `{address, publicKey, path, encrypted}` taken from the
reply. Two things follow that the spec requires separately and that this shape
gives at once:

- *"the identity shown is the reply's, not the row's"* — the selected candidate
  is never read when building it, so there is no code path that could substitute
  it. The selection's only job is to say what to send.
- *"a reply missing the fields the kept state needs is a failure"* — a
  `kept:true` with no address sets `phase = "failed"` rather than reaching
  `"kept"` with an empty string, because the address check is on the way in, not
  at the render site.

**Every exit from a phase clears the same things.** `enterFailed()` nulled
`keptIdentity` while `requestSlate()`'s success path did not, so a keep
followed by "Try again" left `phase` at `"slate"` with a kept identity still
held. Not visible today — every render site and `Main`'s branch read `phase` —
but the spec asks this state machine to be single-valued, and a second stale
answer sitting beside the one value is what makes the next reader's
`keptIdentity` wrong for free. Both exits now clear it.

`encrypted` is stored as **`undefined` when the field is absent**, not
normalised to `false`. The spec is explicit that an absent `false` and a
reported `false` mean different things, so the render site has three cases
(`true`, `false`, absent → no claim) and the property has to be able to hold
three values. The same applies to `recoveryNeedsTheRecord` on the who-am-I
reply. This is the one place where *not* normalising is the correct choice, and
it is why these two fields are copied across raw rather than coerced with
`=== true` the way `hasMore` is in `FeedScreen`.

### The launch branch is a `who_am_i` call in `Main.qml`, re-asked on demand

`Main.qml` holds `identityState` (`"unknown"` / `"present"` / `"absent"` /
`"failed"`) and `identityReason`, computed by `askWhoAmI()`. Onboarding shows on
`"absent"`; the feed on `"present"`; the failure text on `"failed"`; and
`"unknown"` renders nothing, which is only reachable before
`Component.onCompleted` has run.

**Both absent cases collapse to `"absent"` while the reason is kept verbatim in
`identityReason`.** Core distinguishes "no keystore" from "keystore unreadable"
by the reason string alone — `hasIdentity:false` in both cases — and the spec
requires both to reach onboarding with the two distinguishable. Holding the
reason unparsed is the only way to do that without the view inventing a
classification core does not publish: any `indexOf("could not read")` here would
be a second copy of core's error taxonomy, maintained in the wrong module, and
would silently reclassify on a reword. So the view carries the string and the
later screen that wants to tell them apart reads it.

There is no `onboarded` flag anywhere and no `localStorage`-equivalent. The keep
does not set one either: `OnboardingScreen` emits `identityKept` and `Main`
responds by **calling `who_am_i` again**, so even the transition the view just
watched happen is decided by the module. That is a deliberate extra round trip —
it costs one call and buys the property that no screen state is ever derived
from an action having been invoked.

### Selection is a candidate's own index, and the sentinel is unaddressable

`selectedIndex` holds the selected candidate's own `index`, or
`nothingSelected` (`-1`). Every path that replaces `candidates` assigns the
sentinel in the same function, so a position cannot survive a refresh.

**The sentinel must not be a value a candidate can carry, and that is enforced
at the boundary rather than at the comparison.** The first version of this
spelled `-1` inline and let any reply's `index` through: a candidate carrying
`index:-1` then made `row.chosen` evaluate `-1 === -1`, so that row drew the
chosen background, the chosen border and a visible `SELECTED` word **while
nothing was selected**. Pressing Keep hit the guard and returned silently;
clicking the row called `select(-1)`, which changed nothing. A dead button the
user could not escape, and the review measured it: 1 visible marker where a
normal slate shows 0.

The fix is `isCandidate()`, run over every entry **before `candidates` is
assigned** — a candidate must be an object, carry a non-negative integer
`index`, and carry a non-empty string `address`. Three alternatives were
considered:

- **A range check at the render site.** Rejected: it leaves a candidate on
  screen that the user can see and cannot choose, which is the same dead end
  with a narrower blast radius.
- **A non-numeric sentinel** (`null`, `undefined`). Rejected because
  `selectedIndex` is an `int` property and QML would coerce, reintroducing `0`
  as the collision — strictly worse, since `0` is a *valid* index.
- **Trusting core.** Core emits `index` as a `usize` in `0..SLATE_SIZE`
  (`wire.rs:656`, asserted at `wire.rs:3075`), so this is unreachable through
  core today. Rejected because the view's tests supply replies through the same
  bridge, and the standing rule is that the view validates what it is handed.

Refusing the whole slate, rather than dropping the bad entry, is what keeps
"the count comes from the reply" honest: a slate with a hole silently shows
four of five.

**`keepSelected()` now has one guard, not two.** It had `selectedIndex ===
nothingSelected` followed by `candidate === null`, and review proved the first
untestable — every fixture's indexes were non-negative, so the second caught
everything and deleting the first left the suite green. The answer was not a
third fixture: with the sentinel unaddressable, "is something selected" and
"does the selection name a candidate" are **the same question**, and
`candidateAt()` is the one thing that answers it. A second guard that can only
be true when the first is is not a guard, it is a comment that runs. Deleting
the surviving guard now fails 2 tests.

The refusal lives in the handler, not the button: `FlatButton` has no `enabled`
property and emits `clicked()` from its own `MouseArea` regardless of
appearance, so the dimming is appearance and `keepSelected()` is the mechanism.

The keep request sends `slateId` (the set identifier the reply carried) and the
candidate's **own `index` field**, not its position in the array. They are equal
as core builds them, but the spec says the request carries "the identifier of
the set that candidate was offered in and the position it was selected at", and
the candidate's own index is the value core published for that purpose. Reading
the array position instead would be the view re-deriving a value it was handed.

### A value of the wrong type is absent, never stringified

Every `String(x)` over a value the module supplied has been removed. The
coercion looks defensive and is the opposite: `String({code:7})` yields the
literal text `[object Object]`, and review found it reaching the screen as a
refusal reason. That is strictly worse than the screen's own fallback string,
which exists for exactly the case where no usable reason arrived — the fallback
says something true, and `[object Object]` names no fix while looking like it
was meant.

So the policy across all three sites is **treat a non-string as absent**:

- a refusal `reason` falls back to the screen's own sentence;
- `identityReason` in `Main.qml` becomes `""`, which is honestly "the module
  gave no reason this view could read" — a placeholder that was the same text
  for every unreadable reason would distinguish nothing, and distinguishing the
  two absent cases is that field's whole job;
- a non-string `slate` identifier is a **failure**, not a fallback, because it
  is the value a later keep sends back to core to say which set the selection
  was made against. There is no honest default for it.

### Every `Text` is `Text.PlainText`, explicitly

Not one element is left on `AutoText`, which sniffs its input and switches to
rich text when a string looks like markup. `FeedScreen` records why and CI greps
for it. On this screen the module-supplied values are a refusal reason, an
address and a path — none peer-supplied today — so this is structural rather
than a live vulnerability, and the spec says so. The address goes through
`AddressLabel` (already `PlainText`); the reason and the path go through `Text`
elements that set it.

`AddressLabel { full: true }` is used for every candidate row and for the kept
identity, because this screen is where the decision is made. No elision is
hand-rolled anywhere here — the bundle's SPEC.md forbids a second
implementation, and `AddressLabel` owns the one.

### Two copy strings from the bundle are corrected, not pasted

Both are recorded here because a later reader comparing the screen to
`copy.json` will otherwise read the difference as drift.

- **`onboarding.body`** ends *"and the key is yours in this Stoa only — it
  cannot be linked to you anywhere else"*. The MVP uses one identity in every
  Stoa (`docs/PLAN.md` §5.2: "the public key is the join"), so that clause is
  false today, and PLAN.md says verbatim: *"Cross-Stoa unlinkability is
  suspended, not withdrawn … Do not describe the MVP as having it, and do not
  describe the design as having dropped it."* The screen ships the first half
  and puts **nothing** in the second half's place — saying less is available,
  saying something else true-sounding is not, and a replacement privacy claim
  would be the same failure with different words.
- **`onboarding.apparatus.uniqueness`** says *"the same three words"*. **The
  count is removed rather than corrected**, and this is the durable decision
  here.

  It was first corrected to "four", because §5.2.1 had settled four at the
  time. The count has now moved three times — three, then four (merged in #22,
  three argued and closed in #27), now three again on a different basis
  (adjective + noun + "of" + place, on `docs/name-shape-sweep`). Each move made
  the shipped copy wrong **and** left a test pinning the wrong number, which
  then had to be argued with before the copy could be corrected. A pin on a
  number fails on reword rather than on misinformation, which is the wrong
  failure: it resists the fix instead of catching the defect.

  So the sentence states its obligation and no number, and the test asserts the
  obligation and sweeps for *any* count spelling rather than pinning one. The
  obligation does not depend on the count — uniqueness is not merely unbuilt
  but unavailable, since there is no authority to hold a namespace, so the
  interface must stay correct when two identities present the same name, and
  the correctness is that the address is always present. That survives every
  future change to how many words a name has.

### No row shows a name, and the row is shaped so one can arrive

Core carries no name — the closed field set in `identity-onboarding` forbids it,
and the derivation is a separate contract being specified in `piece/generated-
names`. The row therefore shows the mark and the address and **nothing in a name
position**: not the path, not the index, not a truncation of the address. Each
would be read as the thing being chosen and none of them is.

What the row does have is a `ColumnLayout` beside the mark holding the address
alone, so the name becomes one `Text` added above it when the derivation lands.
That is a layout affordance rather than a placeholder: there is no empty element
reserving space and no `""` binding that a future field would fill by accident.

## Risks / Trade-offs

- **`Component.onCompleted` fires the launch call.** A test that constructs
  `Main` gets a `who_am_i` call immediately, which is the same shape
  `FeedScreen` already has and the same one its tests work with. The alternative
  — an explicit `start()` the host calls — would leave the view blank if a host
  forgot it, which is worse than a call a test has to expect.
- **The extra `who_am_i` after a keep** costs a round trip that the keep reply's
  own address could have saved. Taken deliberately: see the launch-branch
  decision.
- **The screen has no passphrase affordance**, so `encrypted` will be `false` on
  an ordinary install and the screen will say so plainly. That is the honest
  render of a real configuration, not a gap this change closes — whether a
  passphrase is obtained is deliberately unsettled by `identity-onboarding`.

## Open questions

None blocking. The name derivation and the "recovery needs the record" export
path are both owned elsewhere and both have a place to land in this screen
without it changing shape.
