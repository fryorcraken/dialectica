## Context

See `proposal.md` for why. The constraints that shape the approach:

- **The view cannot find out whether a key exists by itself.** Basecamp gives
  the QML engine no filesystem access, so every fact about the keystore comes
  from the core. Before this change the core could answer "is there a key?"
  only through `create_identity`, which mints one when there is none, and
  through `who_am_i` and `get_capabilities`, which both take a Stoa that a
  fresh install does not have.
- **`create_identity` never replaces a key.** Its `exists()` branch reports an
  existing key with `wasNew:false` and writes nothing. This change leaves that
  guard alone and depends on it (Decision 9).
- **Only `lgs basecamp build` compiles the adapter** (`dialectica/rust-lib/src/lib.rs`,
  behind `cfg(logos_scaffold)`). So every decision lives in `dialectica-core`,
  where `cargo test` can reach it, and the adapter is a one-line forward.
- **`Main.qml` shows and hides screens with `visible:` bindings and never
  destroys them.** The home screen is built once, so "each time it is shown"
  has to mean a visibility change, not a construction.
- **Every QML type name needs the `D` prefix** (CLAUDE.md, "Never name a QML
  type something basecamp also registers"). `check_qml_names.py` enforces it.

## Goals / Non-Goals

**Goals:**

- A read-only core query whose answer tells "no key" apart from "a key that
  cannot be read", so that no view has to guess.
- A home screen whose three key states are one value, where each affordance a
  state does not offer is missing from the element tree, not just hidden.

**Non-Goals:**

- Any change to `create_identity`'s behaviour, reply or guard.
- Launching the view under basecamp. The component suite drives the screen's
  state machine through a fake bridge and cannot see how Loaders lay out in
  the host. The owner has to check that by eye.

## Decisions

### 1. Widen the core API with a read-only query, instead of mocking the flag in the view

**Owner's decision, made in the runner's conversation.** The alternative on the
table was the MVP route: mock `hasMachineKey` in the view and drive the two
reference screens from that. The owner rejected it for one concrete reason:
**without a real answer, the no-key state and its "Create this machine's key"
come back after every restart.** A mocked or session-held flag starts false on
every launch. That puts the user back on the screen the issue exists to remove,
where they press the button and get the "already had a key, nothing was
replaced" message.

Other routes considered, and what ruled each out:

- **Ask `create_identity` and read `wasNew`.** This mints a key as a side effect
  of rendering a screen, and the spec forbids that ("MUST NOT call the operation
  that creates a master key in order to learn the key state").
- **Ask `who_am_i` or `get_capabilities`.** Both take a Stoa, and the home
  screen of a fresh install has none. `test_no_other_identity_probe_is_made_on_arrival`
  still pins that neither is called.
- **Persist a flag in the view.** The sandbox gives the view no storage, and the
  spec forbids a key state "decided by any value the view persisted across runs".
  A stored flag would also be wrong the moment the keystore changed underneath
  it.

CLAUDE.md asks that widening the core API be a deliberate act. This is that act,
and the reason for it is written here.

The method is `get_master_key` on the wire and `Core.getMasterKey()` in the
view. It takes `{}`, and like `create_identity` it goes through
`Request::parse`, so a non-object or oversized request is refused. It is in
`every_request_taking_method`, `NO_REQUIRED_FIELD` and `a_served_request`.
Leaving it out of the first turns
`the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares` red
and names the method.

### 2. An unreadable keystore is the error shape, not `hasMasterKey:false` plus a reason

**Owner's decision, and it departs from the probe precedent on purpose.**
`posting-capability` sets the rule that `get_capabilities` and `who_am_i`
follow: *the probe answers, it does not fail*. Every storage state comes back
as `{"canPost":false,"reason":…}` or `{"hasIdentity":false,"reason":…}`, and the
error shape is kept for requests that cannot be read. The owner was shown that
precedent and chose the error shape anyway.

The reason is what the caller does with a negative answer. For the posting
probe, a negative closes the composer, which is safe. For this query, a
negative opens "Create this machine's key". If a key is present but unreadable
(tampered, too-open permissions, encrypted with no passphrase), the mint then
refuses, because it never replaces a key. The user ends up asked to create a key
they already hold, with no way to do it. `hasMasterKey:false` plus a `reason`
leaves that trap one ignored field away. The error shape removes it, because
`Core.call` turns it into `ok:false`, and no reading of `ok:false` reaches the
no-key state.

The view gives this state its own rendering (Decision 6), so the failure is
shown to the user, not swallowed.

**What breaks without it:**
`every_refusal_but_not_found_is_the_error_shape_in_the_keystores_words`,
`a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence` and
`a_keystore_that_is_not_a_keystore_is_a_failure_not_an_absence` go red. This was
measured by widening `master_key_from`'s `NotFound` arm to `Err(_)`. Each then
answered `{"hasMasterKey":false}`.

### 3. "No key" means the open found no file. It is never decided by `Path::exists()`

`master_key_from` maps `KeystoreError::NotFound` to `MasterKey::NotHeld`, and
every other error passes through as a failure. `NotFound` comes from
`read_checked`'s `File::open`, and only for `ErrorKind::NotFound`.

The tempting alternative is `if !path.exists() { NotHeld }`, which is the shape
the mint uses. It would be wrong here: `Path::exists` returns `false` whenever
the `stat` fails, so a keystore in a directory the process cannot search would
be reported as no key. In the mint that is harmless, because the `create` that
follows fails loudly. In a query, `false` is the answer itself.

**What breaks without it:**
`a_keystore_in_a_directory_that_cannot_be_searched_is_a_failure_not_an_absence`
goes red. This was measured by adding the `exists()` check at the top of
`open_from_env_with_protection`. The test checks up front that the user running
it cannot search a mode-000 directory. Running as root it panics with a message
instead of passing without testing anything.

### 4. Protection comes from the same read that decodes the key, and the handler takes no `unlock`

The spec requires that protection be "read from the stored key itself, not from
whatever passphrase the caller currently has available".
`keystore::open_from_env_with_protection` reads the file once, decides
`encrypted` from its header and decodes the key from the same bytes.
`open_from_env` now routes through it and drops the flag. It landed as a
refactor in its own commit that changes no behaviour.

Alternatives considered:

- **Open, then call `Keystore::is_encrypted`.** This is what the mint's
  existing-key branch does. It reads the file twice, so the reported protection
  describes whatever is at the path at the second read. `unlock_for`'s doc
  already argues for folding two reads into one, for the same reason.
- **Take an `unlock` argument, as `create_identity` does.** That would make it
  possible to report the caller's configuration as though it were the file's
  protection, which is exactly what the spec forbids. With no such parameter,
  the mistake cannot be made. The keystore environment test pins the half that
  is behaviour: an unencrypted file is reported unencrypted while
  `DIALECTICA_PASSPHRASE` is set.

**The handler takes an opener closure, not a path.** An encrypted keystore
opens only with a passphrase in the environment. `set_var` is process-global,
and `keystore.rs` owns the one test that may touch it. So the `wire.rs` tests
inject an opener for outcomes only the environment can produce (`Locked`,
`encrypted:true`), and use the adapter's real opener for everything the
filesystem can produce. The adapter passes
`open_from_env_with_protection(&default_path_in(&dir))`, the same path that
`create_identity` and `creator_key_in` resolve. That is why
`the_key_reported_is_the_creator_key_a_stoa_this_peer_creates_records` can
compare the reported key with the creator in a decoded genesis record.

The adapter still names no `Keystore` accessor, so CI's "the adapter derives
the creator and the poster in one place" grep still passes.
`identity_public_key()` is called in `master_key_from`, where `cargo test` can
reach it.

### 5. Reply shape: `hasMasterKey`, and a two-arm enum that makes the field set closed

`{"hasMasterKey":true,"publicKey":…,"encrypted":…}` or `{"hasMasterKey":false}`.
The name follows `who_am_i`'s `hasIdentity`, so a view reads both answers the
same way. The type is `enum MasterKey { Held { public_key, encrypted }, NotHeld }`.
A `NotHeld` carrying a key, or a `Held` without one, cannot be built, so the
spec's closed field set holds by construction. The tests compare whole JSON
objects anyway, because `to_json` is where a stray field would be added.

The rejected alternative was reusing the mint's `Minted` shape. Its `wasNew`
means nothing for a read, and there is no value it could take for the no-key
answer.

There is deliberately no third arm for "could not be read". That outcome is the
error shape (Decision 2), so it never reaches this type.

### 6. In the view, one value with three states, where the issue's flag had two

The issue asks for the screen to be "gated on one `hasMachineKey` flag, per the
bundle's `FeedScreen.qml` precedent for `hasIdentity` (one flag, not several
booleans combined ad hoc)". The one-value half of that precedent is kept: the
key-dependent part of the screen reads nothing except `machineKey.state`. The
boolean half is not, because this question has three answers:

- `FeedScreen.hasIdentity` is `identity.hasIdentity === true`, so every failure
  becomes "no identity". On the feed that is the safe direction, because the
  result is that no composer is drawn.
- Here the same fold goes the dangerous way. "No key" draws the create-key
  action, and for a peer with an unreadable key that is the trap described in
  Decision 2. A boolean cannot carry the third outcome without merging it into
  one of the other two.

So `machineKey` is one object: `{state:"none", refusal}`,
`{state:"held", publicKey, encrypted}` or `{state:"unreadable", reason}`.
Decision 13 says why a refused mint is a field of the first. It is
one property and not a `keyState` string beside a `publicKey` string, because
two properties can disagree, and then every block would have to check both. A
held state with no key to show cannot be built: only `heldKey()` creates it, and
only when there is a non-empty key.

The normalisation (`keyFromQuery`) lives in the screen, not in `Core.qml` beside
`identityFrom`. `identityFrom` was moved to `Core` because two screens used it.
This one has a single consumer, and moving it would be speculative.

The affirmative states are entered only on the exact booleans: `=== false` for
none and `=== true` for held. **What breaks if that is loosened to
`!v.hasMasterKey`:** `test_a_reply_stating_neither_outcome_is_not_the_no_key_state`
goes red on `{}`. This was measured.

### 7. "Not instantiated" is a `Loader` whose `active` is one comparison

The bundle's rule is: *"Never show a compose field/affordance the user cannot
use — show the reason and the fix instead."* SPEC.md applies it to this screen
as "Create a Stoa is not drawn at all (not disabled, not greyed)", and the spec
makes it testable: the title field and the action "MUST both be absent from the
screen's element tree". A `visible: false` element is still in the tree. A
walker, accessibility, or a later binding that flips it can still reach it.

So each key-dependent block is a `Loader` with
`active: screen.machineKey.state === "<state>"`: the key block, the
could-not-be-read block, the create affordance and the key line. An inactive
Loader has no item.

The paste section is declared outside every Loader, so "pasting stays available
in every state" holds by construction.

**What breaks without it:** replacing the create Loader's `active` with
`active: true; visible: …held` turns
`test_the_create_affordance_is_not_instantiated_when_no_key_is_held` and
`test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`
red. This was measured. Their walker, `namedAnywhere`, does not skip hidden
elements, and that is the point.

### 8. The key state is asked on every showing, through `onVisibleChanged`

`askKeyState()` runs in `Component.onCompleted` (when the screen is visible)
and in `onVisibleChanged` (when it becomes visible). The reason is the
proposal's: keeping a per-Stoa identity inside a feed also writes the master
key, so a screen that remembered its first answer would still be offering that
user a key they already hold. The calls are synchronous, so an answer from an
earlier showing cannot land after a later one and overwrite it.

**What breaks without it:** `test_the_key_state_is_asked_again_on_each_showing`
and `test_a_mint_failure_does_not_outlive_the_showing_it_happened_in` go red
(measured). So does `test_returning_home_from_a_feed_asks_the_key_state_again`,
which uses the real navigator.

**Harness note, observed on Qt 6.10.3.** A test cannot re-show a screen made
with `createObject(null)`: after `visible = false`, setting `visible = true`
leaves a parentless item invisible and emits nothing. A screen parented to the
`TestCase` reads `visible` false from the start. `makeShownList` therefore
builds the screen inside a parentless host `Item`, which matches how `Main.qml`
mounts it.

### 9. A successful mint enters the key-held state whatever `wasNew` says, and the backend guard stays

SPEC.md: *"Create this machine's key never renders while a key exists, so the
'already had a key, nothing was replaced' message has no path to the screen.
Keep the guard in the backend anyway."*

The first half is now structural. The action exists only in the no-key state,
and any successful mint reply that names a key moves the screen to key-held,
which renders the same text however it was reached. The old `wasNew:false`
wording is deleted, not hidden.
`test_the_key_held_state_renders_the_same_however_it_was_reached` compares all
three routes.

The second half is why the first is safe. The view reads the key state and then
acts on it, so there is always a window in which another writer (another
Basecamp instance on the same profile, a keep in a feed) can create the key
first. `create_identity`'s `exists()` branch is what makes a press in that
window report the existing key instead of replacing it. Removing that guard
because "the button is never drawn when a key exists" would turn that window
into data loss. The guard stays pinned by
`a_mint_over_an_existing_keystore_replaces_nothing_and_reports_it_as_not_new`,
which this change does not touch.

### 10. The unencrypted warning is conditional on the reply

**Owner's decision.** The bundle's 0B draws "Stored unencrypted on this
machine…" every time. The warning is instead drawn only when the reply that set
up the key-held state says `encrypted: false`. `true` draws nothing, and a
missing field makes no claim either way: `heldKey()` stores anything that is not
a boolean as `null`. This keeps the rule the old mint block followed:
protection is taken from a reply, never assumed. A warning drawn every time
would tell every user with a passphrase something false.

### 11. Bundle components map onto the repo's existing names. No new QML type is added

The issue asks for the bundle's `qml/` reference components to be adapted into
this repo's `D`-prefixed equivalents, not imported under their bundle names.
The mapping, for the components the issue names:

| Bundle | Here | Notes |
|---|---|---|
| `StatusBar.qml` | `DStatusBar` | **Already exists and is dialectica's own**, not Basecamp chrome (SPEC.md "Status: three lamps"). Mounted once in `Main.qml` as shared chrome, outside every screen's `visible:` binding, so it already goes with the home screen. Not moved or duplicated. |
| `FlatButton.qml` | `FlatButton` | Existing, grandfathered in `check_qml_names.py`. |
| `ScreenFrame.qml` | `ScreenFrame` | Existing, grandfathered. |
| `IdentityChip.qml` | `DIdentityChip` | **Not used on this screen.** `stoa-navigation-view`'s requirement against claiming a per-Stoa identity keeps "identity" off it, and the bottom-of-file note in `DStoaListScreen.qml` records why the chip was removed. |
| (address abbreviation) | `AddressLabel` | The one 8-8-6 form. The key line goes through it, as the issue asks. |

The new blocks are `Loader` `sourceComponent`s inside `DStoaListScreen.qml`, not
new files. So no new `qmldir` name exists to collide with a host registration,
and `check_qml_names.py` and `check_qml_reachable.py` have nothing new to look
at. Making the key line a `DKeyLine` component was considered and rejected: it
has one use, and a file for it would add a registration without adding a
second caller.

### 12. The copy is `copy.json`'s `homeMachineKey`, verbatim, plus one sentence of this change's

Every string the spec's table lists is taken verbatim. That includes the heading
change from "Stoas you hold" to "Stoas you joined", and the new placeholder
"Title of the new Stoa". The placeholder is a separate `Text` shown over an
empty `TextInput`. It is never the field's `text`, so it cannot be submitted.

The bundle draws only 0A and 0B, so the could-not-be-read state's two strings
are this change's own. Both are contracted verbatim by "A key state that could
not be read is told apart from both others": the statement "Whether this
machine holds a key could not be read.", above the reason, and the action "Try
reading the key again". The statement says what failed and nothing about
whether a key exists, and it avoids the key label, so a check for "no key line"
cannot be fooled by it.

### 13. A refused mint is a field of the no-key value, so the next answer removes it

The spec requires that "a failure rendered after a press belongs to the showing
in which the press was made" (proposal: a refusal from an earlier press "would
describe an attempt the new answer may already have overtaken"). The refusal is
therefore stored as `machineKey.refusal`, inside the no-key value, rather than
in a property beside `machineKey`. Every ask replaces the whole value, so a new
answer carries no refusal unless a new press puts one there. Nothing has to
remember to clear it.

The rejected alternative is the first pass's shape: a separate `mintFailure`
property, cleared by hand at the top of `askKeyState()`. It worked, but the
clearing line was a guard with nothing in the data behind it. It would have to
be repeated at every new entry to the no-key state, and a later caller of the
ask that skipped it would reintroduce the stale refusal with every other test
green. It landed as a refactor in its own commit that changes no behaviour.

**What breaks without it:** `test_a_mint_failure_does_not_outlive_the_showing_it_happened_in`
goes red. This was measured by making `keyFromQuery`'s no-key branch carry the
previous value's `refusal` forward. The fixture answers "no key" on both
showings, so only the showing changes, and the rendered "No key was created."
and the core's reason are what the test checks.

### 14. "Try reading the key again" is the showing's ask, placed inside the could-not-be-read Loader

The spec requires that acting on it call the query once, never mint, and let the
reply decide the key state "by the same rules as an answer to a showing's ask".
So its `onClicked` is `askKeyState()`, the same function a showing calls, not a
second function that normalises the reply. With one path, the two cannot drift:
the strict `=== true` and `=== false` of Decision 6 apply to both, and a reply
that fails again leaves the screen in the could-not-be-read state with the new
reason.

The reason the action exists is the proposal's. Without it, a user who fixes the
keystore while the app runs cannot reach the key-held state short of leaving
the screen, and a peer with no Stoa and no reference to paste has nowhere to
go. The only other route is a restart.

The button sits inside the could-not-be-read Loader (Decision 7), so "MUST NOT
be rendered in the no-key state or the key-held state" holds because it does
not exist there. It is not a visibility check.

**What breaks without it:** `test_the_read_again_action_belongs_to_the_could_not_be_read_state_alone`
goes red on the no-key fixture, and nothing else does. This was measured by
changing that Loader to `active: true` with `visible:` bound to the state. The
test finds the action by its label and its `clicked` signal, walking invisible
elements too, so a hidden action is still found.

## Risks / Trade-offs

- **[Loader layout under basecamp is unseen]** → The component suite checks
  element order by `mapToItem` y-coordinates, which shows the Loaders are
  measured and stacked. It says nothing about how the result looks. The owner
  should launch the view and compare it with 0A and 0B.
- **[A dangling symlink at the keystore path reads as no key held]** →
  `File::open` reports `NotFound` for a dangling link. That is true in the sense
  that no key can be reached. A later mint would then meet whatever
  `Keystore::create` does with the path. This is left as it is: the mint's
  handling of that path is not this change's to alter.
- **[The adapter is compiled only by `lgs basecamp build`]** → It is one line
  forwarding to `core::get_master_key` with the same `default_path_in` the
  neighbouring handlers use. The Rust tests reach the same composition through
  `OnboardingDir::keystore_path`, and the lgs build proves it compiles. Nothing
  tests it end to end.

## Migration Plan

The core API change adds one method. Nothing that calls the existing methods
changes.

The view contract change is the proposal's BREAKING item. The "always offered"
create requirement is replaced, and the tests that pinned it
(`test_the_create_affordance_is_present_when_no_key_exists`,
`test_a_second_press_reports_the_existing_key_rather_than_a_failure`,
`test_the_identity_step_is_offered_even_when_the_membership_cannot_be_read`) were
inverted or replaced in the same commit as the view change. The core and the
view ship as a pair from one tag, so a view never runs against a core that lacks
`get_master_key`. If it did, the call would fail, and the screen would show the
could-not-be-read state, the least-claiming of the three.

Rollback is reverting the view commit. The core method can stay: it is
read-only and has no other caller.
