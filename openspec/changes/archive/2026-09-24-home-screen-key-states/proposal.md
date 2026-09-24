## Why

The home screen shows every task at once: the key-creation block, the Stoa list,
"Create a Stoa" and "Paste a reference" are all drawn whether or not this machine
holds a key (confirmed by the owner against the running app, 2026-09-23). So a
peer with no key is offered "Create it", which can only fail. A peer with a key
is offered "Create this machine's key" on every run, and pressing it earns "This
machine already had a key. Nothing was replaced." The owner's home-screen bundle
(`SPEC.md` "Home: the machine key", reference screens 0A and 0B) fixes this with
two states driven by one question: does this machine hold a key?

**The core has no call that can answer that question.** `create_identity` answers
it only by minting a key when none exists. `who_am_i` and `get_capabilities` both
take a Stoa, and a fresh install has none. So today the view can learn whether a
key exists only by creating one. That is why `stoa-navigation-view` currently
requires "Create a Stoa" to be offered "whatever the keystore's state": hiding it
would be "hiding it on a guess". The bundle's rule that "Create this machine's
key" **never renders while a key exists** cannot be met on any run after the
first without a read-only answer from the core. So this change adds one.

## What Changes

- **A read-only core query for this peer's master key.** It reports whether a
  master key is held and, where one is, names it and says whether it is
  protected at rest. It takes no Stoa and writes nothing. A keystore that is
  present but cannot be read is a failure, not "no key". This widens the core
  API, and CLAUDE.md asks that widening be a deliberate decision. It is recorded
  here as one because nothing else lets the view meet the bundle's rule.
- **The home screen's key state is one value with three outcomes**: no key held,
  a key held, and the key could not be read. The issue calls it
  `hasMachineKey`. A boolean cannot carry the third outcome without collapsing
  it into one of the other two. Collapsing an unreadable store into an empty one
  is the defect this repo's "empty versus unreadable" requirements exist to
  prevent.
- **No key (0A):** the key block (label, explanation, "Create this machine's key")
  is the only task on the screen. "Create a Stoa" is **not instantiated**: not
  hidden, disabled or greyed out. Pasting a reference stays available, because
  reading needs no key.
- **Key held (0B):** the key-creation block is not instantiated. "Create a Stoa"
  appears above "Paste a reference". The key becomes one line at the foot of the
  card: label, the key abbreviated through `AddressLabel`'s 8-8-6 form, and the
  unencrypted-storage warning in accent red **when the core reports the key
  unencrypted**. The bundle draws the warning unconditionally. This change keeps
  the existing rule that protection is taken from a reply and never assumed.
- **Could not be read:** neither the key block nor the create affordance is
  instantiated, and nothing says that no key is held. The screen states
  "Whether this machine holds a key could not be read." above the reason, and
  offers "Try reading the key again", which asks the query again and never
  mints. The bundle draws only 0A and 0B, so both strings are this change's own.
  Without the action, a user who fixes the keystore while the app runs has no
  way to reach the key-held state short of leaving the screen, which a peer with
  no Stoa and no reference to paste cannot do, or restarting.
- **A refused mint is reported only in the showing where it happened.** The key
  state is asked again on the next showing, and a refusal from an earlier press
  would describe an attempt the new answer may already have overtaken.
- **The "already had a key, nothing was replaced" message has no path to the
  screen.** A successful mint reply leads to the key-held state whether or not
  it says the call created the key. The backend guard (`create_identity` never
  replaces a key) is unchanged.
- **The key state is asked again each time the home screen is shown.** Keeping
  a per-Stoa identity also writes the master key. Without a re-read, a user who
  onboarded inside a Stoa would return home to a screen offering to create a key
  they already hold.
- **Copy is taken verbatim from the bundle's `copy.json` `homeMachineKey`
  block.** This includes the heading "Stoas you joined" (today "Stoas you hold")
  and the placeholder "Title of the new Stoa".
- **BREAKING (view contract):** `stoa-navigation-view`'s "Creating a Stoa asks
  for a title and nothing else, and is always offered" is removed and replaced.
  Its title-only, empty-title and core's-reason rules carry over. "Always
  offered" and its scenario "The affordance is offered when no key exists" are
  reversed.

## What this is not

- **Not a change to the mint.** `create_identity`'s behaviour, including its
  refusal to replace an existing key, is untouched. That operation's requirements
  live in the pending `first-run-identity` change's `identity-onboarding` delta.
- **Not a change to the status lamps.** The bundle's `StatusBar.qml` already
  exists here as `DStatusBar`, mounted once in `Main.qml` as shared chrome
  (`view-navigation`'s "Shared chrome states which screens it accompanies").
  This change neither adds nor moves it.
- **Not a change to the list rows, the empty/failed listing states, the preview,
  or sharing.** Those requirements stand as written.

## Capabilities

### New Capabilities

None. The home screen already belongs to `stoa-navigation-view`, and the master
key already belongs to `identity-onboarding`.

### Modified Capabilities

- `identity-onboarding`: one requirement added. Whether this peer holds a master
  key is reportable without creating one, without naming a Stoa, and without
  writing anything.
- `stoa-navigation-view`: the "always offered" create requirement is removed and
  replaced with one that offers creation only once the core reports a key.
  Requirements are added for the three key states, what each draws and does not
  instantiate, the key line, the re-read on each showing and on the
  could-not-be-read state's read-again action, and the verbatim copy.

## Impact

- `dialectica/rust-lib/dialectica-core/src/wire.rs`: a read-only handler beside
  `create_identity`. `dialectica/rust-lib/src/lib.rs`: one trait method, from
  which the builder derives the `.lidl`, so the dispatch table must still
  derive.
- `dialectica-ui/src/qml/Core.qml`: one wrapper.
- `dialectica-ui/src/qml/DStoaListScreen.qml`: the two-state layout. Any
  component adapted from the bundle's `qml/` directory takes this repo's `D`
  prefix. `check_qml_names.py` and `check_qml_members.sh` gate it.
- `dialectica-ui/tests/tst_stoa_screens.qml`: tests that pin today's behaviour
  are invalidated by design and must change with it:
  `test_the_create_affordance_is_present_when_no_key_exists`, which asserts the
  reversed requirement; `test_a_second_press_reports_the_existing_key_rather_than_a_failure`,
  which asserts the "already had a key" message this change removes; and
  `test_the_identity_step_is_offered_even_when_the_membership_cannot_be_read`,
  which drives a fixture with no answer to the new query. Every fixture
  building the screen needs one now. `test_no_identity_probe_is_made_before_the_user_asks`
  stays valid: it forbids `who_am_i`, `get_capabilities` and `create_identity`
  on arrival, and the new query is none of them.
- **Archive order:** `first-run-identity` is complete but not yet archived, and
  also adds to `identity-onboarding`. The two deltas touch different
  requirements, so either order merges cleanly.
