## Why

**0.0.1 ships per-Stoa identity selection while its own milestone says it ships
one identity per user.** A peer with a machine key who opens a Stoa is told *"a
master key exists but no identity has been chosen for this Stoa; generate a slate
and keep one of its candidates"*, is offered "Choose an identity for this Stoa",
and lands on the per-Stoa candidate slate. Posting, replying and voting are all
refused until a candidate is kept **in that Stoa**, and a second Stoa asks again.
That is the per-Stoa mechanism issue #108 schedules for 0.0.3, wired live. (Issue
#149.)

**The leak is in core, not only in the view.** Measured against this tree:

- `create_stoa` names the **machine key** as creator (`creator_key_in` →
  `identity_public_key`, the root used directly).
- `getCapabilities`, `whoAmI` and every publish resolve the key through
  `chosen_paths` — `stoa_key_at_path(stoa, recorded_path)` — and refuse where no
  path is recorded.

So the creator of a Stoa posts under a key that is **not** the creator key the
genesis record names. A Stoa's creator is its sole moderator, so once moderation
is published from this peer the creator's own posts would come from a
non-moderator — the failure `keystore.rs`'s `creator_key_in` doc records as having
shipped once already. And a user in two Stoas is two keys, which is exactly what
0.0.1 says it is not.

**The specs disagree with each other about it, which is why nothing caught it.**
`view-identity-onboarding` and `stoa-navigation-view` both state that *"one key
signs in every Stoa in this release"*; `identity`, `identity-onboarding` and
`content-authoring` contract per-Stoa derivation and a per-Stoa record as the
identity in use. Each file is internally consistent; together they describe two
systems, and the code built the second.

## The decision: option (a) — the machine key is the identity everywhere, and the slate is withheld from the view

Issue #149 offers two shapes and leaves the choice here.

- **(a)** Stop the per-Stoa slate being reachable in 0.0.1, and make every gate
  read the one machine key.
- **(b)** Keep the slate reachable but collapse `generateIdentitySlate` /
  `keepIdentity` onto the machine key.

**(a) is chosen.** (b) cannot be expressed without contradicting what a slate is:
`identity-onboarding` requires every candidate in a set to be distinct in key and
path and requires a keep to store the candidate the user saw, so a slate that
always resolves to one key is either a slate of one (a "choice" with nothing to
choose) or several rows that are the same identity. Either rewrites that
capability's contract wholesale, and #108 would then have to rewrite it back.
(a) leaves the per-Stoa machinery — derivation, slate, keep, the record — built
and tested exactly as it is, so #108 stays what its own text says it is:
*"switching that call back on, not a redesign."*

Within (a), one further choice, made here rather than left to the dev:
**`generateIdentitySlate` and `keepIdentity` stay on the wire and keep working as
contracted; nothing in 0.0.1 consults what they record.** The alternative —
refusing both methods for this release — was rejected: it puts every slate and
keep requirement in `identity-onboarding` in contradiction with the module (each is
written as what the module does — the first opens *"The module SHALL be able to
present a set of candidate identities"*), would need a dozen requirements rewritten
to describe "the core beneath the method" instead, and moves every wire-level
onboarding test, all to protect against a caller that does not exist — the view
is the only caller, and this change stops it calling. What *is* contracted is the
honest half: a kept choice does not change the identity in use, and a test pins
that.

## What Changes

- **The identity in use in every Stoa is the machine key** — the key
  `createIdentity` reports and a Stoa this peer creates names as its creator.
  Every post, reply and vote this peer signs is signed by it, in every Stoa.
  **BREAKING** for the wire's meaning (not its shape) of `publishPost`,
  `publishReply`, `publishVote`, `getCapabilities` and `whoAmI`: the key they sign
  with or report no longer depends on the Stoa.
- **No per-Stoa choice is needed to post.** A peer holding a machine key can post,
  reply and vote in any Stoa with no slate generated and nothing kept. The reason
  *"no identity has been chosen for this Stoa"* becomes unreachable from the
  probe, from `whoAmI` and from a publish.
- **The record of per-Stoa choices is not consulted** by anything that signs or
  reports the identity in use. A record that holds a choice, holds none, or cannot
  be read at all changes nothing about who posts.
- **`whoAmI`'s identity reply loses `path`.** The machine key derives from no
  path, so there is none to report. `recoveryNeedsTheRecord` stays and is `false`:
  the identity in use is recoverable from the master key alone. **BREAKING** for
  the reply's field set; no reachable view reads `path` from this reply.
- **The view never reaches the per-Stoa onboarding screen.** The feed's
  no-identity affordance leads to the Stoa list, where this machine's key is
  created; `DOnboardingScreen` is no longer instantiated and is recorded as
  deliberately uninstantiated, with #108 as the reason. The affordance does not
  present an identity as something chosen for one Stoa.
- **The per-Stoa derivation stays built and pinned.** Its reproducibility and
  cross-Stoa unlinkability are properties of the derivation, and they remain
  contracted as such. The unlinkability *of the identity in use* is suspended for
  this release, and the specs say so rather than claiming it.

## What this is not

- **Not a change to `createIdentity`.** It is `first-run-identity`'s, unchanged:
  it mints the one machine key, records no per-Stoa choice, and never replaces a
  key. It is now also the only way a peer comes to hold an identity.
- **Not a migration of existing data.** Per-Stoa rows already recorded are kept
  and ignored — see Impact. Deleting them would be deleting the only material
  from which ops already signed under those keys can be re-derived.
- **Not the home screen.** `DStoaListScreen`'s layout is issue #150's (the
  `hasMachineKey` two-state design). This change routes the feed *to* the list
  and contracts nothing about what the list draws.
- **No new Stoa-less query.** The feed asks `whoAmI(stoa)` and keeps asking it:
  it is the question "who am I in this Stoa", whose answer in this release is the
  machine key everywhere, and which #108 turns back into a per-Stoa answer
  without the feed changing. The home screen's `hasMachineKey` is a different
  question — it names no Stoa and stays Stoa-less after #108 — so it belongs to
  #150 rather than being a second copy of this one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity`: one requirement added (the machine key is the identity in every
  Stoa, with the two-Stoa regression scenario); "A user has one identity per
  Stoa" and the unlinkability requirement amended so they describe the per-Stoa
  derivation, which remains built, rather than the identity in use — and the
  unlinkability requirement renamed, because its old heading is false in this
  release.
- `identity-onboarding`: the identity-in-use report is the machine key and
  carries no path; the recovery statement reports that the master key alone
  suffices; a recorded path is described as the kept choice, not as the identity
  in use; the path-carrying requirement no longer covers the identity-in-use
  reply; and "Keeping an identity does not replace an existing one" is narrowed
  to what the code already does — a second keep *for the same Stoa* is refused —
  because read against "the machine key is an identity in every Stoa" its old
  wording would refuse every keep once a key exists. Three further requirements
  are modified because their scenarios assumed a peer holding no key while it
  onboards, and read against the machine key they contradicted the new `identity`
  requirement: a restored record reproduces the kept choices and leaves the
  identity in use as the machine key (it previously said the record names the
  identities in use); a slate generated on a peer that already holds a key no
  longer asserts that "who am I" finds nobody; and a refused selection is required
  to record no choice and write no master key, rather than to store "no identity".
  Three scenarios here (and one in `content-authoring`) keep their names while
  their content changes, the convention this capability already uses, because
  `validate` refuses a MODIFIED block that drops a scenario.
  `generateIdentitySlate` and `keepIdentity` themselves are unchanged, with one
  addition to what a failed keep does: because a stored master key is now an
  identity in every Stoa, a keep that fails after storing one removes it, and
  where storage refuses the removal the keep's reason says a master key was left
  rather than reading as "nothing changed".
- `content-authoring`: the author requirement no longer says the identity is
  derived from the Stoa — it says the module decides it and the caller never
  supplies it — and is renamed to match.
- `view-navigation`: "Onboarding is entered from the navigator and returns to it"
  is removed and replaced by an added requirement routing a missing identity to
  the Stoa list, where the machine key is created, with the per-Stoa onboarding
  screen recorded as deliberately uninstantiated. Removed rather than modified
  because every one of its scenarios is about entering and leaving a screen this
  release does not reach; the removal's Migration names what #108 restores.
- `view-identity-onboarding`: "this screen is the view's only route to acquiring
  one" amended — the screen is withheld in this release, and the affordance that
  leads to acquiring an identity does not present it as a per-Stoa choice.

`posting-capability` is **not** modified: its requirement that *"the identity
reported is the one that would sign"* is unchanged and is what the new scenarios
exercise. `keystore` is not modified: *"a single root secret from which every
per-Stoa identity is derived"* remains true of the derivation.

`first-run-identity` is complete but not archived, so its `createIdentity`
requirements are not yet in `openspec/specs/identity-onboarding/spec.md`. This
delta **modifies none of them** and adds nothing that depends on their text, so
the two archive in either order. One sentence of its rationale — *"creation would
then succeed while posting stayed refused for want of a choice in the real
Stoa"* — describes the per-Stoa model this change retires; it remains true as a
reason not to record a placeholder path, and is left as written.

## Impact

- **Core** — `dialectica/rust-lib/dialectica-core/src/wire.rs`: `posting_identity`,
  `publishing_key` and `whoami_for` stop consulting the identity store; the
  `Whoami::Identity` reply drops `path`. `dialectica/rust-lib/src/lib.rs`: the
  publish assembly and `get_capabilities` / `who_am_i` stop opening the store for
  this purpose. `cfg(logos_scaffold)` code, so `nix build .#lgx` is the only gate
  that compiles it.
- **View** — `FeedScreen.qml`'s no-identity route, `Main.qml`'s
  `createIdentityFor` and its onboarding mount, `DIdentityChip.qml`'s affordance
  caption, and `qmldir`'s record for `DOnboardingScreen`.
- **Tests** — the core tests binding a keep to `whoAmI`, the probe or a publish
  (e.g. `a_kept_identity_survives_a_restart_and_is_the_one_reported`) assert the
  model this change retires and must be re-pointed at the record, not deleted;
  `tst_navigation.qml`'s onboarding-route cases likewise.

### Existing data: the audit, and what happens to it

**The audit of the owner's `alice` profile was performed after this proposal was
first written.** Its result, the store's location and the read-only query to
re-run are in `design.md`'s Migration Plan ("Existing data: the audit, for
#108"). Any row that query returns is a Stoa where this defect was exercised.

**What happens to such a Stoa once this lands**, decided here because it is
behaviour, not data repair:

- **The row stays and is ignored.** Posting, replying and voting in that Stoa use
  the machine key; `whoAmI` and the probe report the machine key. A spec scenario
  pins exactly this.
- **Ops already signed by the per-Stoa key stay valid and stay attributed to that
  key.** Nothing re-signs or re-attributes them — an op's author *is* its key. To
  every reader, including this peer, they are a different author from the user's
  machine key: a different generated name and mark.
- **Two consequences follow, and neither is repaired here.** A post signed by the
  old key can only be revised by that key (`post-revision` drops a version whose
  author differs), so it becomes un-revisable once revision is published from the
  view. And a vote cast under the old key and a vote cast under the machine key on
  the same target are two voters; nothing de-duplicates them.
- **No moderation was signed by a per-Stoa key**, because no moderation publish
  is wired into the module. The creator key has always been the machine key, so
  a Stoa's creator gains nothing and loses nothing here — except that their posts
  now come from the key the genesis record names.
- **#108 inherits a question this change does not answer:** when per-Stoa
  identity is restored, does a row kept during 0.0.1 become the identity in use
  again, or is it treated as stale? The row is kept precisely so that choice is
  still available then.
