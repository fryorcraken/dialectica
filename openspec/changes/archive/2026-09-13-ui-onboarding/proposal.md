# The onboarding screen: five keys, a choice, and the three things it must not claim

## Why

A fresh install has no identity, and **nothing in the view can get one**.
`generate_identity_slate`, `keep_identity` and `who_am_i` are built, merged and
contracted by the `identity-onboarding` capability — and none of the three is
reachable from QML: `Core.qml` wraps `list_threads` and `get_capabilities` and
nothing else, so the three methods exist and no code path calls them.
`Main.qml` says so in a comment: there is no onboarding.

The consequence is not a missing screen, it is a dead product. With no
identity the capability probe answers `canPost: false` forever, so the feed
renders its gate reason and the user has no action available anywhere in the
interface that would change it. Creating an identity is the first item of the
MVP scope, and it is the only one whose core half is finished.

The visual and copy reference is the Claude Design handoff bundle
(`tmp/ui-design/handoff/`), whose screen 01 is this screen. Its copy is used
verbatim where it is true; where it claims something the software does not do,
this proposal records the divergence below rather than specifying it.

## What Changes

- **The `Core` bridge gains three wrappers** — a slate call, a keep call and a
  who-am-I call — each a named wrapper in the one place a core call is written,
  so a renamed method is one edit and a typo is not a silent dead screen.
- **An onboarding screen exists**, with the states a slate/keep flow has: no
  identity yet, a slate on offer, a candidate selected, a keep that succeeded,
  a keep that was refused, and a read that failed. The states are mutually
  exclusive, for the reason `FeedScreen` computes one `readState` from one
  variable rather than holding several booleans.
- **The app branches on launch** between onboarding and the feed, on the answer
  to who-am-I rather than on any local flag. A view that remembered "we
  onboarded" would show a feed to a user whose keystore had gone.
- **A refused keep is an answer, not a failure.** Core replies
  `{"kept":false,"reason":…}` — a success at the wire level — so a view with
  only the `Core.call` error branch would read a refusal as a success and show
  an identity that was never stored. The screen has to distinguish three
  outcomes where the bridge distinguishes two.
- **`docs/UI-BRIEF.md` gains nothing and loses nothing here**, but two of its
  statements are now implemented rather than promised. See Impact.

## Capabilities

### New Capabilities

- `view-identity-onboarding`: what the QML view shows while a user acquires an
  identity, what it must never claim about the identity they chose, and how the
  app decides on launch whether to show onboarding at all.

Checked against `openspec list --specs` before naming it. The near-duplicate to
avoid is `identity-onboarding`, which is the **core** contract — what a slate
is, what keeping guarantees, what the replies may carry. This capability owns
none of that and deliberately restates none of it: it owns the *view's*
obligations over that contract, which are a different set of properties and are
verifiable by a different gate (the QML test suite, not `cargo test`). The two
specs meet exactly at the reply shapes, which this one reads and never defines.

There is no existing view capability to extend — this is the first. A later
view piece (the Stoa list, the composer) should ask whether its requirements
generalise into one `view-*` capability before adding a second sibling; one
capability is not evidence of a shared concept, so that question is not
answered here.

### Modified Capabilities

None. `identity-onboarding`, `identity`, `keystore` and `module-wire-contract`
are all read and none is changed: this change adds a caller, and a caller that
required the contract to move would be a caller built wrong.

## What the mockup shows that core does not serve

Recorded here rather than specified, because a requirement no implementation
can satisfy is worse than a silence. Each is a real gap and each has an owner
that is not this change.

- **A generated name on every candidate row.** The mockup's rows carry a name
  in 20px serif above the address; the slate reply carries none, and the
  `identity-onboarding` requirement "A generated name and a mark are not
  settled by this capability" forbids it carrying one, enforced by a closed
  field set. The view cannot compute it either: the name derives from the
  **public key** under a domain-separated hash that is not built. So the spec
  below requires the address and the mark, and requires that the row not
  present anything else as a name. The name arrives with the change that builds
  the derivation, and the row must have room for it — which is a layout
  obligation, not a behaviour one.
- **The mockup's placeholder name reads as a fantasy handle** — `vermilion
  patient sandworm` — which the brief names as the draft that went wrong, and
  its `ON UNIQUENESS` apparatus copy asserts a word count. **The copy ships with
  no count at all**, reading "may hold the same name": this change asserts no
  word count anywhere, in the copy or in a test.

  **An earlier version of this paragraph said the opposite** — that the count
  was "corrected" from three to four, and that four was settled. Both halves
  were wrong. The count has now moved three times (three, then four, then three
  again on a different basis, `d3e7579`), which is the argument for stating none:
  a pin on a number fails when someone rewords it, not when it becomes untrue.
  The sentence's obligation — not unique, not identifiers, the address
  distinguishes — needs no count to carry it. `design.md` argues the decision;
  this entry exists so a later reader of the archive does not find the rejected
  alternative recorded here as the chosen one.
- **The mockup's body copy claims cross-Stoa unlinkability** — "the key is
  yours in this Stoa only — it cannot be linked to you anywhere else". The MVP
  ships one identity used in every Stoa, so that sentence is **false today**,
  and the brief's constraint 2 names asserting it as the one failure here that
  could actually harm someone. The spec requires the first half of the sentence
  and forbids the second, and requires no replacement claim in its place.
- **The mockup shows no encryption state and no recovery warning.** The keep
  reply carries `encrypted`, and the who-am-I reply carries
  `recoveryNeedsTheRecord`; the brief's obligations 7 and 8 require both to be
  showable. The spec requires them after a keep. The mockup is a screen 01 that
  ends at "keep"; the honest answer arrives one step later.
- **Refresh is unlimited, and this the mockup and core agree on.** Checked
  rather than assumed: the slate reply is five candidates with `count`
  reporting five, and regeneration is required to be unrefusable on the ground
  of how many preceded it. The view reads `count` rather than hardcoding five,
  so a later count change is core's to make alone.

## Impact

- `dialectica-ui/src/qml/Core.qml` — three named wrappers. No change to
  `call()`, whose one error branch is what makes three wrappers cheap.
- `dialectica-ui/src/qml/` — a new onboarding screen, assembled from the
  components already in the tree (`ScreenFrame`, `Identicon`, `AddressLabel`,
  `FlatButton`, `MarginNote`, `Theme`). None is restyled or re-implemented;
  `AddressLabel` in particular owns the abbreviation and has a `full` mode,
  which is the mode this screen uses because the user is choosing a key.
- `dialectica-ui/src/qml/Main.qml` — the launch branch, and the comment
  asserting there is no onboarding.
- `dialectica-ui/tests/` — a new QML test file. The suite runs on every PR.
- `docs/PLAN.md` §5.2.1 and §9.1 — the view half of onboarding stops reading as
  forthcoming. §9.2's MVP item 1 is not struck: core plus a view is the item,
  and this change is the second half of it.
- `docs/UI-BRIEF.md` — **no edit is required for this spec to be true.** Its
  constraint 2, its obligations 6, 7 and 8 and its onboarding paragraph all
  describe this screen correctly and are what the spec is written against. One
  paragraph is now weaker than what ships and is flagged for `dev-writer`
  rather than edited here: the onboarding bullet under constraint 2 says core
  "serves this" without saying that core serves **no name**, which is the one
  thing a designer laying out a candidate row needs to know. See the spec's
  requirement on what a row may present.
- No core change, no wire change, no version bump.
