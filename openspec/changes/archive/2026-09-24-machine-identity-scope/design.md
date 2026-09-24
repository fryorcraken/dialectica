## Context

`proposal.md`'s Why covers the defect. In code terms, three functions in
`dialectica-core/src/wire.rs` answer "who is this peer here":

- `posting_identity`, what the posting probe (`getCapabilities`) reports;
- `publishing_key`, the key a post, reply or vote is signed with;
- `whoami_for`, what the identity report (`whoAmI`) names.

Each took `(stoa, keystore, record)` and answered
`stoa_*_at_path(stoa, recorded_path)`, refusing with `NO_CHOICE_FOR_THIS_STOA`
where no path was recorded. The adapter (`dialectica/rust-lib/src/lib.rs`) opened
the record of per-Stoa choices (`identity.sqlite`, table `chosen_paths`) for every
probe, report and publish. A Stoa's creator, meanwhile, was always the machine key
(`creator_key_in` → `Keystore::identity_public_key`). The view reached the per-Stoa
slate through `Main.qml`'s `createIdentityFor(stoa, …)`, raised by the feed
footer chip's "Choose an identity for this Stoa".

The adapter is `cfg(logos_scaffold)`: `cargo test`, clippy and `cargo mutants`
never compile it. `nix build path:./dialectica#lgx` is the only gate that does.

## Goals / Non-Goals

**Goals:** the three functions answer with the machine key and are not able to
consult the record; the adapter stops opening the record for them; the view
stops reaching `DOnboardingScreen` and routes a missing identity to the Stoa list.

**Non-Goals:** everything in `proposal.md`'s "What this is not". Also: the
authoring layer (`authoring.rs`) takes the signing key as a parameter and decides
nothing about which key, so it is untouched. Its tests that name "the identity
derived for its Stoa" test the layer with a fixture key, not the module's choice.

## Decisions

### D1. Option (a): the slate is withheld from the view, and every gate reads the machine key

Issue #149 left the shape open between two options:

- **(a)** `DOnboardingScreen`'s slate flow stops being reachable from the feed's
  identity gate in 0.0.1, and the gate reads the single machine identity from
  `create_identity` / `DStoaListScreen`'s key-minting flow.
- **(b)** `keepIdentity` / `generateIdentitySlate` are simplified for 0.0.1 so they
  always resolve to the one machine key, and the per-Stoa slate UI is deferred
  until #108 lands in 0.0.3.

(a) is built, and the reason is the proposal's: (b) cannot be expressed without
contradicting what a slate is. `identity-onboarding` requires every candidate in
a set to be distinct in key and path, and requires a keep to store the candidate
the user saw. A slate that always resolves to one key is either a slate of one (a
"choice" with nothing to choose) or several rows that are the same identity.
Either rewrites that capability's contract, and #108 would then rewrite it back.
Under (a) the per-Stoa machinery (derivation, slate, keep, record) stays built
and tested as it is, so #108 stays what its own text says it is: *"switching that
call back on, not a redesign."*

The issue also recorded a second complaint from the running app: the onboarding
screen's intro phase gates the first candidates behind "Show me some keys", which
reads as "useless, should just show the keys". The issue's own analysis is that
this is the same root cause and not a second defect. The deferred fetch is
deliberate for the per-Stoa screen (`DOnboardingScreen.qml`: a view that rationed
slates would impose a limit the module does not have), and it only reads as
broken because the screen was reached where no per-Stoa choice should be offered
at all. Withholding the screen removes the complaint, so the screen is not changed.

### D2. `generateIdentitySlate` and `keepIdentity` stay on the wire and keep working

These two are not refused in 0.0.1. They work as contracted, and nothing in 0.0.1
reads what they record. Refusing both was considered and rejected in the
proposal. Every slate and keep requirement in `identity-onboarding` is written as
what the module does, so refusing them would contradict a dozen requirements,
which would then need rewriting to describe "the core beneath the method". Every
wire-level onboarding test would also have to move, and all of that would guard
against a caller that does not exist: the view was the only caller, and D9 stops
it calling. The contract that does change: a keep no longer changes the identity
in use (`a_recorded_per_stoa_choice_does_not_change_the_key_in_use`).

The same fact reaches three more `identity-onboarding` requirements, which the
delta modifies:

- **A restore brings back the kept choices, not the identity in use.** A record
  restored beside a master key reproduces each kept choice from that key and the
  recorded path, and the identity in use stays the restored machine key.
  `a_record_restored_beside_a_master_key_reproduces_the_kept_choice` asserts the
  two as separate facts, so neither can stand in for the other: the restored
  master key and record reproduce the kept key, and `whoAmI` on the restored
  device names the machine key and not the kept one.
- **Onboarding may begin on a peer that already holds a key.** A generated slate
  and a refused selection are each required to store no master key and no choice
  that were not there before, rather than "no identity" at all. Only a peer that
  held no master key still finds nobody when asked who the user is. The existing
  tests (`generating_a_slate_writes_nothing`,
  `a_selection_outside_the_set_is_refused_and_stores_nothing`) start from an empty
  directory, which is that keyless peer; neither covers a peer that already holds
  a key.

### D3. The identity-in-use functions take no Stoa and no record, so the record cannot decide who posts

`posting_identity(keystore)`, `publishing_key(keystore)`, `whoami_for(master)`,
`who_am_i(request, master)` and `get_capabilities_from_stores(request, open_keystore)`
lose their record parameter. The first three lose the Stoa too.

Alternatives considered:

- **Keep the parameters, ignore them.** This is the smallest diff and gives #108
  the least to change. It was rejected because a function handed the record and
  told not to read it is one edit away from reading it again, and that edit
  compiles, passes every signature and needs no call site to change. It is the
  shape in which the defect was live.
- **Prefer a recorded path and fall back to the machine key.** This is what
  "posting shouldn't need a choice" invites. It was rejected because it is exactly
  what `identity` forbids ("a choice recorded for a Stoa MUST NOT change the key in
  use"), and the owner's own profile holds such a row (see Existing data).
- **Drop the parameters (chosen).** "An unreadable record does not prevent
  posting" then holds by construction: the probe, the report and the key a
  publish signs with cannot fail on the record because nothing hands it to them.
  #108 re-adds the parameter, which is a signature change that a reviewer sees.

**What breaks without it.** If the record is read again through any path:
`a_recorded_per_stoa_choice_does_not_change_the_key_in_use` (the keep leaves a
path whose key differs from the machine key) and
`one_machine_key_posts_replies_and_votes_in_two_stoas` (six authors against one
minted key) turn red. The adapter half (`publishing` and the two handlers not
calling `Self::paths`) cannot be seen by any test, because it is `cfg(logos_scaffold)`. It is
visible only in the diff and to `nix build`. That is why the core signatures carry
the guarantee instead of the adapter.

### D4. One derivation position: the probe reports the public half of the key a publish signs with

`publishing_key` is `keystore.identity_key()`. `posting_identity` is
`publishing_key(keystore).public_key().to_hex()`. `whoami_for` calls
`posting_identity`. `creator_key_in` is `identity_public_key()`, which is
`identity_key().public_key()`. So the four answers go through one accessor, not
four call sites that happen to agree. `keystore.rs`'s `creator_key_in` doc records
that the two-call-sites shape has already shipped wrong once.

**What breaks without it.** If the probe, the report or the publish is pointed at
another derivation: `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa`,
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports` and
`the_creator_a_creation_names_is_the_identity_the_probe_reports` turn red. The
last one now reaches the probe through `get_capabilities_from_stores`, the
adapter's own call. Before this change it had to inject `creator_key_in`, because
the real lookup read the record and disagreed with the creator. The same test now
also publishes a post and asserts its author is `genesis.creator` (the `identity`
scenario "A Stoa's creator posts as its creator").

### D5. `NO_CHOICE_FOR_THIS_STOA` is deleted, not left unused

The state it named ("a master key exists but no identity has been chosen for this
Stoa") is not reachable in 0.0.1. A `pub const` for a state that cannot occur
invites a caller to match on it. `posting-capability` says reason text is for a
reader, not for a caller to match. #108 re-adds it with the state.

### D6. `whoAmI` drops `path`; `recoveryNeedsTheRecord` stays and is `false`

The machine key derives from no path, so `identity-onboarding` now forbids a path
in this reply. The closed-field-set requirement makes a leftover `path` a failure,
not a harmless extra. `recoveryNeedsTheRecord` stays as a field and is `false`:
the report is still required, and a field whose value flips (per-Stoa identity
back in #108, and later a real backup) is cheaper for a view than a field that
appears and disappears.

### D7. The Stoa is still parsed where it is no longer used

`who_am_i` and `get_capabilities` still require a well-formed `stoa`, and a
malformed one is still the error shape, even though the answer no longer depends
on it. The adapter's `publishing` still calls `core::stoa_of` before it opens the
keystore.

- **The wire does not widen.** Accepting `{}` would be a new contract that #108
  then narrows again. The feed asks "who am I in *this* Stoa", and 0.0.1 answers
  that question the same way everywhere (proposal, "What this is not").
- **Order in `publishing`.** Parsing first keeps a malformed request the envelope's
  refusal rather than "no identity" when no keystore exists. CI's adapter gate
  also requires `core::stoa_of` in that file, and its comment is updated to say
  why the call remains.

**What breaks without it.** `a_malformed_whoami_request_is_the_error_shape_and_carries_no_identity`
hands `who_am_i` a keystore opener that succeeds, so a handler that stopped
parsing `stoa` would answer `hasIdentity:true` and turn it red. The same holds
for the probe's malformed-request tests.

### D8. A keep whose record write fails removes the master key it just wrote, and never one it did not

`keep_selection` writes the keystore first and the record second. When the record
write failed on a fresh install, the old code left a master key on disk with no
path pointing at it. That was harmless only while `whoAmI` read the record. Once
the machine key is the identity in use, that file is an identity, handed out by a
call that reported failure. The delta states the scenario directly: "a subsequent
load finds no recorded choice and no master key that were not there before".

The fix is `undo_a_keystore_this_keep_wrote`. It removes the file only when the
keep's own `create` branch wrote it, and that branch sets a flag (`wrote_the_keystore`). Nothing can have signed with the key, because it existed only
between two lines of one dispatch.

Alternatives: reversing the write order would leave a recorded choice for a
master key that does not exist, which `a_keep_whose_keystore_write_fails_records_no_path`
exists to prevent. Deleting the file unconditionally would destroy a pre-existing
machine key, which is the user's identity in every Stoa.

**What breaks without it.** Without the undo,
`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity` turns red
at the "master key that was not there before" assertion (confirmed: it failed first,
then passed with the fix). The test for the flag is
`a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`. I
could not run the mutation that removes the flag, because the edit was refused by
this session's permission checker. So that test's ability to fail is argued, not
measured (see Risks).

**tester, 2026-09-24: measured.** The same mutation (`if wrote_it {` → `if true {`,
making the undo unconditional) was not refused this session. Run against the full
`a_keep_whose*` group: `a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`
failed exactly as predicted (`left: None, right: Some([…keystore bytes…])`, "a
failed keep removed or rewrote a master key it did not write"), and
`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity` stayed
green, since `wrote_it` is `true` on that fixture's path anyway and the mutation
does not change its behaviour. The mutation was reverted immediately after
(`git diff --stat` shows no implementation change). The flag's discrimination is
now measured, not argued.

### D9. The view routes a missing identity to the Stoa list, and the onboarding state leaves the navigator

`Main.qml` loses the `onboarding` state, `createIdentityFor`, `closeOnboarding`,
`identityWasKept` and the mounted `DOnboardingScreen` with its Back button.
`acquireIdentity()` is `enterOnly("", null)`, which is the list, and it is what the
feed chip's `createRequested` now reaches. `qmldir` carries the
`# UNINSTANTIATED:` record. `check_qml_reachable.py` enforces the record, and
`test_the_per_stoa_onboarding_screen_is_instantiated_nowhere` enforces the
instantiation half: it searches the object tree by type, not by `objectName`, and
I confirmed it fails when a hidden `DOnboardingScreen {}` is mounted.

- **A named function rather than a second call to `closeFeed()`.** The two
  transitions are the same today. They are different requests, and #108's
  restored route will make them differ again.
- **The state is removed rather than left null.** A state nothing enters would
  still be in `stateNames` and in `screenShown`'s ternary, and tests that walk
  every state would still walk it. Removing a state is a one-line edit backwards,
  which is what `enterOnly` was built for.
- **The chip caption is copy.json's `common.createIdentity`, "Create an identity".**
  The old caption, "Choose an identity for this Stoa", was a deliberate change
  from the bundle while the chip's no-identity arm meant "no per-Stoa choice". It
  now means "no usable machine key", which is what the bundle's string says. The spec
  requires only that the text names no Stoa. The exact string was chosen here
  and is marked `NO SPEC` in `tst_identity_chip.qml`.
- **`FeedScreen` keeps asking `whoAmI(stoa)`** (proposal, "What this is not"). Its
  `hasIdentity` gate now reflects "does this machine have its key" because core
  answers that question. No view code had to change for it.
- **`DStoaListScreen` is not touched.** #150 (`piece/home-screen-key-states`) is
  rebuilding it. Two of its comments are now stale and are left for that piece:
  line ~633 says the only thing that writes a key is per-Stoa onboarding, and
  line ~662 says `DOnboardingScreen` is where a per-Stoa identity is chosen.

## Risks / Trade-offs

- [Ops already signed by a per-Stoa key now come from a "different author"] →
  This is accepted and is not repaired here (proposal, Impact). An op's author is its key.
  Such a post cannot be revised by the machine key, and votes under the two keys
  count as two voters. #108 inherits the question.
- [The adapter half of D3 is visible to no test] → The core signatures carry the
  guarantee, `nix build path:./dialectica#lgx` compiles the adapter, and CI's
  adapter gate still requires the core entry points by name.
- [D8's guard has no measured mutation] → **Resolved by the tester, 2026-09-24.**
  The flag-removing mutation (`if wrote_it {` → `if true {` in
  `undo_a_keystore_this_keep_wrote`) was not refused this session. It was applied,
  `a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`
  failed on it (predicted failure, observed failure), and it was reverted; `git
  diff --stat` confirms the implementation carries no change. The test's
  discrimination is measured.
- [`undo_a_keystore_this_keep_wrote` can itself fail] → If `remove_file` fails, the
  reply names both failures and the path, so it does not suggest nothing changed.
  That reason is chosen, not specified (`NO SPEC` in the code), and no test
  reaches it: making `remove_file` fail between two lines of one call needs a hook
  the code does not have.
- [e2e tests named for per-Stoa identity] → `the_same_keystore_posts_under_different_identities_in_two_stoas`
  and its neighbour sign with `Keystore::stoa_key` directly, so they test the
  derivation's unlinkability, which `identity` keeps. Their names read as a claim
  about who posts. That is a naming question for the tester, not a failing test.
  **Resolved by the tester, 2026-09-24.** Renamed the first to
  `the_per_stoa_derivation_still_yields_two_authors_for_the_store_to_persist` and
  added a comment stating explicitly that it signs through `Keystore::stoa_key`/
  `stoa_public_key` directly, bypassing `wire::publishing_key` and
  `wire::posting_identity`, and is not a claim about which key is in use in this
  release — that claim is `wire::one_machine_key_posts_replies_and_votes_in_two_stoas`'s.
  The neighbour,
  `a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`,
  was left as named: it claims persistence across a restart, not who posts, so
  its name does not contradict this release.

## Migration Plan

No data is migrated. Per-Stoa rows already in `chosen_paths` are kept and ignored
(proposal, "Existing data"): they are the only material from which ops already
signed under those keys can be re-derived.

### Existing data: the audit, for #108

The proposal could not locate the owner's `alice` profile store. It is
`.scaffold/basecamp/profiles/alice/xdg-data/Logos/LogosBasecampDev/module_data/dialectica/cfb6124ada8a/identity.sqlite`
(instance id `cfb6124ada8a`, not the `40c5423a292f` recorded in
`first-run-identity`'s `tasks.md`). I read it read-only on 2026-09-24 while
Basecamp was running on it, with
`sqlite3 -readonly <that path> "SELECT hex(stoa), path FROM chosen_paths"`. It
holds one row:

| stoa | path |
|---|---|
| `2F7E0752E8240F364682EA5B1EA99824672D1D955390EC2C6FBD904D5730FEC0` | `1473176945` |

So the defect was exercised once on that profile. Once this lands, that Stoa's
probe, report and publishes use the machine key
(`a_recorded_per_stoa_choice_does_not_change_the_key_in_use` is this state). Any
op already signed there under the per-Stoa key stays attributed to that key. When
#108 restores per-Stoa identity it has to decide whether this row becomes the
identity in use again or is treated as stale. The row is kept so that choice
stays open. Re-run the query rather than trusting this table. It is a snapshot.

## Open Questions

None.
