## Context

See `proposal.md` — Why. The gap is that `keystore.rs` can store a root secret
and nothing mints one, so `getCapabilities` is permanently `canPost:false`.

Four properties of the existing code shape every decision below, and each was
verified rather than assumed.

**`derive_stoa_key` is on the live path, and the chain is three hops.** Verified
by walking it: the adapter's `getCapabilities` closure → `Keystore::stoa_address`
→ `stoa_public_key` → `stoa_key` → `identity::derive_stoa_key(&self.root, stoa)`.
This change adds a path input to that chain; it does not route around it.

Cited **by symbol rather than by line**, and that is a correction rather than a
style preference. This paragraph gave `lib.rs:251` and `keystore.rs:651/:645/
:640/:641`, which were the pre-change numbers — by the time the change shipped,
`lib.rs:251` was a doc comment about the slate nonce. Three reviewers found the
citations stale independently, and one noted the real hazard: this project's
recorded failure mode is that the most convincing citation is the unread one, so a
reader who checks `lib.rs:251`, finds prose about a nonce, and concludes the chain
claim was fabricated would be drawing the wrong lesson from a correct claim. Line
numbers in a `design.md` rot inside the same change; symbol chains do not.

Note that `getCapabilities` **no longer uses this chain** — see the decision on
`posting_identity` below. The paragraph is kept because it is what the change was
designed against.

**The op log's schema cannot absorb a new table.** `log/sqlite.rs` holds
`LAYOUT_VERSION` in `PRAGMA user_version`, `check_layout` proves the declared
layout against the columns every read touches, and `create_schema`'s doc comment
states outright that **there is no migration path by design** and that a later
"add a migration" refactor is the change its warning is addressed to. Adding a
`chosen_paths` table to `ops.sqlite` is therefore a `LAYOUT_VERSION` bump that
permanently bricks every store written by a prior build.

**The core crate is pure and the module instance is `Default`-constructed.**
`dialectica-core` has zero SDK types and no ambient state. When this change
started, `Dialectica` held exactly one field (`persistence_path`), set only by
`on_context_ready`; it now holds a second, the `OnboardingSession` that spans a
slate and a keep. `interface: "universal"` scans the impl header's `public:`
section, so the constructor must stay genuinely parameterless — which is why the
new field is a `Default`-constructible `core` type rather than anything needing an
argument.

**The keystore file has no room for a new field, and that is asserted.**
`keystore`'s "The file is exactly its declared layout, with no room for a
verifier" scenario pins the length to the declared fields. The path record
cannot go in the keystore file without invalidating a merged requirement.

## Goals / Non-Goals

**Goals:**

- A slate of candidates, a keep, and a "who am I" — three wire methods.
- Derivation by path over one master key, through the existing per-Stoa
  derivation and not beside it.
- A recorded path that survives restart, is readable in full, and needs nothing
  machine-local to read.
- Whichever protection the keystore applied, reported to the caller.

**Non-Goals (design-level, beyond the proposal's scope):**

- **No name or mark derivation.** The spec's last requirement forbids this
  capability defining either. A slate carries the public key and the address;
  what words come out of them is a separate contract.
- **No migration of the op log's schema.** See Decisions.
- **No passphrase acquisition.** Deliberately unsettled upstream. This change
  reports what happened; it does not decide what should.
- **No replace-identity operation.** The spec requires a second keep be refused
  and says replacing deliberately is a separate operation.

## Decisions

### The path is a third HKDF input, under a bumped salt version

`derive_stoa_key(root, stoa)` stays exactly as it is, byte for byte, and a new
`derive_stoa_key_at_path(root, stoa, path)` sits beside it under a **new salt
string**, `/dialectica/2/Identity/Stoa`.

The `identity` MODIFIED requirement demands the two schemes be distinguishable
"so that one scheme's identities cannot be silently reproduced by the other",
and names versioning as the mechanism. The bump is what makes path 0 a
*different* key from the two-input derivation rather than the same one.

**The alternative was making path 0 equal the old scheme**, which would keep
every already-derived identity valid. It is rejected because the proposal's own
framing is that making them equal "may well be wanted — but it must be a
decision somebody took, not a collision", and nothing has been derived under the
old scheme in production: there is no keystore in the field, because this change
is the one that creates the first. So the cost of the bump is zero today and
buys the separation the spec asks for. It would not be zero later.

**The old function is not deleted.** `the_wire_constants_are_pinned_to_known_answers`
pins its output, three merged `identity` requirements are written against it, and
the spec's "Where a derivation scheme taking a path and one taking only the root
and the Stoa both exist" presupposes both exist. Deleting it would make the
non-collision scenario untestable.

**The path's encoding is a fixed-width big-endian `u32`, appended to the info
after the Stoa address.** `stoa || path` is unambiguous because the Stoa address
is a fixed 32 bytes — there is no variable-length concatenation here, which is
the hazard `identity.rs`'s prefix padding exists to avoid. `u32` rather than a
BIP-32 path string because a string is a parser at the boundary, and what the
slate offers is an index (the owner's words: "several derivation path (1, 2, 3,
etc)"). A string path is additive later; it would need its own encoding decision
and its own refusals, and nothing needs it now.

### The path record is its own SQLite file, not a table in `ops.sqlite`

`identity.sqlite` beside `ops.sqlite`, with its own `PRAGMA user_version`.

The op log's `check_layout` makes its schema a claim it verifies, with no
migration path by design — so a new table there is a `LAYOUT_VERSION` bump that
refuses every existing store outright. That is the whole reason for the split,
and it is a constraint the existing code states about itself rather than one
inferred.

Two further properties fall out of the split and are worth having on their own:

- **A different failure domain.** An op log that fails to open is a feed that
  cannot render; an identity record that fails to open is a user who cannot
  post. Collapsing them means one file's corruption takes both.
- **The record is separately backup-able.** The spec requires that "nothing
  about the record's storage prevents" a later export. A file holding only the
  path record is a file that can be copied whole; a table inside a store holding
  every op the peer ever saw is not.

**The alternatives considered.** A plain JSON file was rejected for the reason
the keystore's `write_atomically` records — an interrupted write over a good file
destroys it, and getting atomic-write-plus-staging right a second time is
duplicating the hardest part of `keystore.rs`. SQLite gives atomicity and
durability from a dependency already in the tree, with no new crate. Putting it
in the keystore file was rejected because `keystore`'s own spec asserts the file
has no room for another field, and because the record is explicitly *not*
required to be secret.

**Two columns, keyed by Stoa.** `(stoa BLOB PRIMARY KEY, path INTEGER NOT NULL)`.
The primary key is what makes "one chosen path per Stoa" hold by construction
rather than by a guard at each write — CLAUDE.md's complexity-in-the-data-structure
rule — and it is what makes the spec's "Distinct choices for distinct Stoas are
recorded separately" scenario a property of the schema.

### The slate is derived on demand from a nonce, not held as server state

A slate is `(nonce, [candidate; 5])` where the candidate at index `i` has path
`derive_path(nonce, i)`. The **nonce is what the reply carries** and what a keep
quotes back; the candidates are recomputed from it.

This is the decision that took the most revision, so the rejected shape is
recorded. The obvious implementation holds the current slate in the module
struct and matches a selection against it. That works and costs a mutable field,
but it makes two of the spec's requirements awkward in a way the nonce shape
makes structural:

- **"A selection made against a superseded set is refused."** With held state,
  this is "compare against the *current* slate", which is correct only if
  regeneration reliably overwrites — a guard at one call site. With a nonce, a
  selection quotes the nonce it was made against, and a nonce that is not the
  live one is refused because the values do not match. The superseded case is
  the same code path as the wrong-nonce case, so there is no second path to get
  wrong.
- **"Generating a slate writes nothing" / "A discarded slate leaves no trace."**
  A slate that is a nonce plus a derivation is nothing to discard. There are no
  candidate secrets held across the two calls at all, which is the strongest
  form of the spec's clearing requirement — material that was never retained
  cannot fail to be cleared.

The module still holds the live nonce, because "superseded" has to mean something
and only the module spans two calls. But it holds **32 bytes of public
randomness**, not key material — which is why the state is cheap and why the
clearing obligation lands on `Zeroizing` inside one function rather than on a
long-lived field.

`derive_path(nonce, i)` is `u32::from_be_bytes` over the first four bytes of
`SHA256(SLATE_PATH_PREFIX || nonce || i)`, with the top bit masked off so the
value is always below `2^31`. The mask is not cosmetic: it keeps every path
representable as a positive SQLite `INTEGER` and as a non-hardened BIP-32 index,
so the recorded value does not need reinterpreting if the LEZ wallet direction
in `proposal.md` is ever taken.

### The path's range is one constant, because the mask and the read guard are one rule

`onboarding::PATH_LIMIT` is `0x8000_0000`. `derive_path` masks with
`PATH_LIMIT - 1`; `identity_store::path_from_row` refuses `>= PATH_LIMIT` on read
and `record_path` refuses it on write.

**Recorded because the first version got this wrong in a way that reads as
correct.** `path_from_row` bounded the path to `u32`, and its doc comment claimed
the rule had "one answer to *is it applied everywhere?*". It did not: the mask
bounded writes to below `2^31` and the guard bounded reads to below `2^32`, so
every value in `[2^31, 2^32)` was inside the guard, outside what any slate can
offer, and accepted silently. Security review measured the consequence — a
hand-edited, restored or file-synced `identity.sqlite` row of `0x8000_0001`
derives a *working* Ed25519 key, because `derive_stoa_key_at_path` has no range
precondition, and `whoAmI` then reports an address that is not the user's with
nothing refusing anywhere. That is the outcome the refusal exists to prevent,
reached through the refusal.

The alternative considered and rejected was to change the literal in the guard
from `u32::MAX` to `0x8000_0000`. It closes the measured hole and leaves the
defect: two rules that agree today, either of which a later reader can widen
alone. Naming the bound once is CLAUDE.md's "complexity in the data structure"
applied to a constant — there is one thing to change, so the two cannot drift.

**The guard is on the write side as well as the read side**, which is not
symmetry for its own sake: a row this build wrote and then refused to read back
would be a store it had bricked itself, and `create_schema` states there is no
migration path by design.

**What this does not claim.** It bounds a path to the range a slate *can* offer,
not to the five a particular nonce *did*. Those five are not knowable at the
store layer — the nonce is deliberately not stored, and the record outlives every
slate — so the property is "a value this build's derivation could have produced".
That is what the refusal's reasoning requires and the strongest available here.
Narrowing further would mean storing the nonce, which is the held-slate shape
this design rejects above.

**The spec states no range for the recorded path**, which is why the original
bound was free to be the wrong one. Flagged to `spec-writer` rather than left as a
`NO SPEC:` marker, because this is not an arbitrary filling of a silence: the
range is the mask's, and the mask the spec's requirement that a recorded path be
what the derivation produced.

**Why derived rather than five random `u32`s.** Five random values would need all
five stored or re-randomised, which reintroduces the held-slate problem. One
nonce reproduces all five, and the spec's "Requesting another set yields
different candidates" follows from a fresh nonce rather than from a uniqueness
check.

**Distinctness within a slate is checked, not assumed.** The five paths are
derived from one nonce and could in principle collide — a 2^-31-ish event per
pair, so not reachable in practice, but "not reachable" is not "cannot happen"
and the spec requires no two candidates share a public key. Rather than a
retry loop, the derivation walks the index forward until it has five distinct
paths: `derive_path(nonce, i)` for `i = 0, 1, 2, …`, keeping what is new. That
is bounded (it terminates on the first five distinct values, and the walk is
capped) and it keeps the nonce-reproducibility property, because a keep
recomputes the same walk.

### A keep writes the keystore and the path record, and the keystore goes first

Keeping is two writes — the keystore and the path record — and the spec requires
it "either complete or change nothing".

**The keystore is written first, and this ordering is the whole of the
atomicity story.** `Keystore::create` writes atomically through a random staging
path. So:

- If the keystore write fails, **no path was recorded anywhere, and the
  `identity.sqlite` the adapter opened on the way in carries no row.** Not
  "nothing was written anywhere", which is what this entry said and which is
  false: `IdentityStore::open` runs before the keep and stamps a schema onto a
  fresh file, so a failed keystore write leaves a zero-row store with
  `user_version = 1` behind. Harmless — `path_for` returns `None` and `whoAmI`
  takes the no-choice row correctly — but the precise claim is both true and just
  as strong, and design review caught the overstatement.
- If the keystore write succeeds and the path record fails, the keystore exists
  with no recorded path. That is the one partial state; what it costs is below.

The reverse order would be worse in a way worth stating: a recorded path with no
keystore is a record naming a master key that does not exist, and a later keep
would silently inherit it.

**What this does not claim.** It is not a two-phase commit, and a crash between
the two writes leaves the keystore on disk. The honest statement is that the
spec's requirement is met at the level of *reported* state: no keep that did not
complete reports an identity, and a load after a failed keep finds no identity.
Making the pair genuinely atomic would need the path record inside the keystore
file, which `keystore`'s spec forbids.

### The second-keep refusal is the path record's, not the keystore's

A keep writes the keystore **only where no file exists**, and the refusal for a
choice already made comes from `chosen_paths`' primary key.

**This reverses the first implementation, and the reason is the most instructive
thing in this document.** That version used `Keystore::create`'s `AlreadyExists`
as the second-keep guard, which reads as elegant — one mechanism, already atomic,
already refusing. It is one mechanism doing **two jobs**, and the second one was
wrong:

- The keystore is **one file per the whole install**. The path record is
  **per Stoa**. So `AlreadyExists` refuses on install-scope while the spec's
  refusal is Stoa-scope.
- Consequence, measured by design review: a user who kept an identity in Stoa A
  and then tried to keep one in Stoa B was refused at `create`, **before
  `record_path` was ever called**, with a reason naming a keystore they did not
  know they had. `chosen_paths` could never hold a second row through any wire
  call, so the spec's "Distinct choices for distinct Stoas are recorded
  separately" was **unreachable through the API** — and this document's claim that
  the primary key discharges that scenario was false. It guarded a table that
  structurally could not receive a second insert.

So the two refusals are separated, each to the thing that knows:

| situation | scope | who refuses |
|---|---|---|
| a master key is already on disk | install | nobody — it is reused, and is the expected state for every Stoa after the first |
| this Stoa already has a chosen path | Stoa | `chosen_paths`' primary key |

**Two things this bought that have to be paid for, both recorded rather than
discovered later:**

- **The `encrypted` report needed a second source.** Where the keystore already
  exists this call writes nothing, so "the `Unlock` this keep used" is not the
  truth about the file — it is the protection a write that did not happen would
  have applied. That branch reads the file, via `Keystore::is_encrypted`. This is
  *not* the re-read the decision below rejects: that one is about re-reading a
  file this call just wrote, where the value this code used is the authority.
  Here there is no value this code used. An unreadable existing keystore refuses
  the keep rather than guessing, because a keep that reported an identity while
  unable to say whether its master key is protected has answered a question it
  does not know the answer to.
- **The refusal's message had to become its own error.** The primary key surfaces
  as SQLite's `UNIQUE constraint failed`, which `IdentityStoreError::Storage`
  renders as *"check the path and its containing directory"* — the wrong fix for
  what is now the commonest refusal this store has. `ChoiceAlreadyRecorded` names
  the existing choice instead. It is matched on SQLite's **error code**, not its
  message text, so a reworded diagnostic cannot silently send every refusal back
  to generic storage. `each_keep_refusal_reason_is_pinned_to_its_own_situation`
  failed on exactly this when the refusal moved, which is what that test was
  written for.

**The risk this trades into, and why it is accepted.** The one-file guard was
also, accidentally, what stopped a second keep from minting and writing a *new*
master key over the old one — which would strand every identity derived from the
first. That protection now rests on the session reusing the opened keystore and
on `keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key`
asserting the written file re-derives both Stoas' kept addresses.
`a_second_keep_for_one_stoa_is_still_refused_after_the_second_stoa_fix` exists
beside it deliberately: the two constrain each other, because the fix is only
correct if the refusal it moved is still there.

### The master key a slate was offered under is held with the nonce, not re-minted

`OnboardingSession` holds `(keystore, live_slate)` together for the module's
lifetime, and the mint-or-open decision is in `core`.

**Recorded because the first implementation was wrong in a way every guard
passed.** A slate is a **`(master key, nonce)` pair**: the nonce fixes the five
*paths*, the master key fixes the five *identities* at those paths. The module
held only the nonce, and the adapter minted a fresh key on each of the two calls.
So on a fresh install — the only install onboarding exists for — the slate showed
candidates of key *A*, the keep recomputed the same five paths against key *B*,
wrote *B*, reported `kept: true`, and named an address the user had never seen.

Three reviewers reached it independently. The spec calls storing an identity the
user did not choose unrecoverable *"because the choice cannot be recomputed"*, and
an address *"the only unforgeable way to tell two candidates apart"* — so the one
value the choice was made on was the value that changed. Every refusal in
`keep_selection` is about the *selection*; none was about the key the selection
was made against.

**The alternative considered and rejected**: carry a commitment to the master key
in the slate reply and refuse the keep when it does not match. That detects the
divergence and does not fix it — it converts a wrong answer into a refusal the
user can do nothing about, because the key that produced their slate is already
gone. Holding it is what makes the right answer available.

**What holding costs**: a root secret in memory for the module's lifetime rather
than for one call. That is the same lifetime a *kept* keystore's root has, so the
window widens only on the fresh-install path and only until the user keeps or the
module stops. Nothing is written — the mint writes no file, which is the spec's
"Generating a slate SHALL NOT write to storage" — and `Keystore`'s root is
`Zeroizing`, so the held copy is wiped on drop.

**Two user-visible properties this fixed as a side effect**, both of which
`docs/UI-BRIEF.md` already promised a designer:

- **"Refresh for more" means more.** Each refresh now offers more candidates of
  one identity's key rather than candidates of a different key each press.
- **A second Stoa reuses the master key** the first keep wrote, which is what
  "one master key per install" has to mean to be worth saying.

**Why it is in `core`.** It was a 12-line helper on the adapter struct, and the
adapter is `#[cfg(logos_scaffold)]` — `build.rs` sets that cfg only when the
generated provider exists, which it never does under `cargo test`. The one
function deciding which master key a slate and a keep each saw was compiled out of
the only gate that runs any logic, and the adapter's own comment says a body there
that grows past one line *"is logic no test can reach"*. It was right. What stays
in the adapter is what genuinely cannot move: the host's directory, and the
environment the protection is read from.

**On the fixture that hid it.** `a_kept_identity_is_the_candidate_the_slate_offered_at_that_index`
is the test written for this exact property and it could not fail on it, because
it hands both calls the same fixed `[7u8; 32]` keystore. A fixture supplying one
key to both sides cannot distinguish "the slate and the keep agree" from "the
harness gave them the same one" — this project's recorded defect family, at the
fixture boundary rather than inside a fixture. The regression test supplies **no**
key: its opener reports `NotFound`, so minting happens, and its assertion is a
relationship between two replies rather than a comparison against a constant.

### `getCapabilities` reports the path-derived identity, through `core`

The probe's identity now comes from `posting_identity(stoa, keystore, paths)` —
the path record consulted, then the path-taking derivation.

**Recorded because leaving it alone is what broke it.** `proposal.md` declares
`posting-capability` *"not modified, deliberately — the probe's shape, its reasons
and its derivation are untouched"*. The shape was untouched and the **contract was
broken**: the salt bump gave the path-taking scheme `/dialectica/2/…` while the
probe's lookup stayed on the pathless `/dialectica/1/…`, and `identity.rs`'s own
test asserts the two schemes *must* disagree. So `whoAmI` and `getCapabilities`
answered "who posts here" with two different addresses for one user in one Stoa,
with no field in either reply to tell them apart. `posting-capability`'s
requirement is explicit — the reported identity *"SHALL be the one an op published
now would be attributed to, derived from the key that would actually sign it"* —
and names the failure: *"the user sees one handle and posts under another."*

The salt bump was right. It left a caller behind, and the "not modified" paragraph
is how that went unnoticed: the distinction it exists to draw is exactly the one
it got backwards.

**Two consequences taken on purpose:**

- **`getCapabilities` now depends on `IdentityStore`.** That is a real widening of
  what the probe reads, and the alternative — retire the pathless trio from the
  live path — answers a different question, because the probe would then have no
  identity to report at all. A master key with no recorded choice for this Stoa is
  now `canPost: false` with a reason naming the missing choice, which is the same
  state `whoAmI`'s fourth row names, reported through the same constant so the two
  methods cannot describe one situation in two vocabularies. Previously the probe
  answered `canPost: true` here, asserting posting ability for an address with no
  recorded path that nothing in the signing path would ever use.
- **The lookup's error widened from `KeystoreError` to `String`.** It said the only
  thing that can stop a user posting is the keystore, which stopped being true.
  Adding an `Other(String)` arm to `KeystoreError` was the obvious alternative and
  is rejected: that enum's arms are documented as distinguishable *so a reason can
  name a fix*, and a catch-all carrying another module's failure is what the
  doctrine exists to prevent. `capability_for` reduced the error to a `String` on
  the next line anyway.

**Why it is in `core`.** Same reason as the session: the derivation was chosen in
the adapter's closure, every probe test injects a stub returning a literal, so no
test could compare the probe's identity to `whoAmI`'s and none could be written
while the choice lived there. Moving it is what made
`the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` possible,
and that test asserts against a signature verifying — the requirement's own
wording — rather than against two derivations that could both be wrong.

**Left behind, and flagged rather than fixed here:** `Keystore::stoa_key`,
`stoa_public_key` and `stoa_address` — the pathless trio — now have **no
production caller**. They remain because `identity.rs`'s non-collision test needs
the primitive, and because deleting three public methods from `keystore`'s type
widens this change into a contract it declares untouched. A later change should
either retire them or say why they stay; this one has no business doing it
silently.

### `whoAmI` reads the path record, and its absence is a distinguishable reason

The spec requires three states told apart: an identity present, none stored, and
one stored but unloadable. The reply is an enum with one payload each, following
`Capability`'s shape for exactly the reason its doc comment gives — a struct with
two `Option`s can express states the contract does not have.

The three states map onto what the two stores say:

| keystore | path record | reply |
|---|---|---|
| opens | has a path for this Stoa | the identity, with address and public key |
| `NotFound` | — | none, reason: no identity yet |
| any other error | — | none, reason: the keystore error's own message |
| opens | no path for this Stoa | none, reason: naming the missing record |

The fourth row is the state the two-store split creates and is why it is listed:
a master key with no recorded path for a Stoa is a real state, and it is not the
same as having no identity. Its reason names the record, so it does not read as
"you are nobody".

**The reason strings are `KeystoreError::Display`'s**, not paraphrases, for the
reason `capability_for` records: that type has a documented obligation to name
the fix, and paraphrasing would maintain the same guidance twice.

### The protection report is derived from what was written, not re-read

`keepIdentity`'s reply carries `"encrypted": bool`. It is taken from the `Unlock`
the keep was performed with — the same value that decided the protection byte —
rather than from re-opening the file and parsing its header.

Re-reading would be the more defensive shape and it is rejected on purpose: it
would report the protection of whatever is at the path *now*, which on a
directory an attacker can write to is not necessarily the file just written. The
value that is true is the one this code used.

**The passphrase itself is not decided here**, and this change adds no source
constant for one. The keep takes the unlock the caller's environment supplies, by
the same `unlock_from_env` route `open_from_env` already uses for reads: a
passphrase in `DIALECTICA_PASSPHRASE` encrypts, and its absence stores in the
clear as `Protection::None`. That is not a new policy — it is the existing
keystore's two documented paths — and the report is what makes the unencrypted
case a state the UI can name rather than a silent default. The spec's refusal of
a build-time constant is honoured by there being no constant to find.

### Recovery-needs-the-record is a static fact this build states

The spec's requirement is "confined to what is checkable now: that the module
reports the unbacked state". Since no export or remote backup exists in this
change, the module reports unbacked unconditionally, with a reason, and the
reply shape carries the boolean rather than only prose — so the change that
implements backup flips a value rather than changing a shape.

This is reported as a field on `whoAmI` rather than as its own method. A separate
method would be one more thing a view must remember to call, and the fact is
about the identity being reported, so it belongs beside it.

### Disk content is refused, never coerced — one principle, three applications

The store refuses rather than repairs in three places, and they are one decision
rather than three:

| what is wrong | refused as |
|---|---|
| a stored path outside the writable range | `PathOutOfRange` |
| a stored Stoa key that is not 32 bytes | `StoaNotAnAddress` |
| a second choice for a Stoa that has one | `ChoiceAlreadyRecorded` |

**The principle: coercing disk content names an identity nobody chose.** A clamped
path derives a perfectly valid key, a padded or truncated address names a different
Stoa, and a replaced choice strands every op the previous identity signed. In each
case the user is handed something that *works* and is not theirs, with no error
anywhere — and the spec calls storing an identity the user did not choose
unrecoverable, *"because the choice cannot be recomputed"*.

Recorded here because the first two were argued only in `#[cfg(test)]` comments and
in the error variants' own docs. That is not nothing — but `findings/` is deleted
before merge and the tracker is scaffolding, while a later reader reaching for
`INSERT OR REPLACE` or a `try_into().unwrap()` reads the archive, not the variant
docs. Design review made the point and it is right: both refusals turn on one
principle, and stating it once where decisions live is stronger than stating it
twice where they do not.

**The `NO SPEC:` markers stay in the tests.** They mark that the *spec is silent*,
which is a different claim from "here is why we chose this" and is addressed to a
different reader — the spec/test reviewer enumerating unspecified behaviour. Moving
the reasoning here does not make the silence go away, and `spec-writer` still owns
deciding whether the contract should say something.

### Minting a key is fallible, because it is now on a handler path

`SecretKey::generate` and `Keystore::generate` return a `Result`.

They did not. `SecretKey::generate` carried an `expect` on `getrandom`, and its doc
comment justified it with *"not reachable from a dispatch handler — key generation
happens at keystore setup, not while serving an inbound op."* That was true when
written and **this change made it false**: on a fresh install every
`generateIdentitySlate` and `keepIdentity` mints, so every one of them reached the
`expect`. Correctness review found it, and found it in the worst arrangement — the
mint was in the adapter, *outside* `core::guarded`, so `catch_unwind` never saw it
and PHASE0-FINDINGS §3's measured consequence applied in full: the module process
aborts, the caller waits out its 20s timeout, every later call reports
`MODULE_NOT_LOADED`.

Two fixes were available and both were taken, because they answer different halves:

- Moving the mint into `core` put it **inside `guarded`**, so the panic would now be
  caught and converted. That alone closes the abort.
- Making the signature fallible removes the panic rather than catching it. This is
  the one that matters structurally: reverting it is a compile error at
  `Keystore::generate`, not a silent change of failure mode. `SlateNonce::generate`
  had already taken this shape for exactly this reason and its comment cited the
  contrast with `SecretKey::generate`; rather than update that comment to preserve
  the asymmetry, the asymmetry is gone.

`RandomnessUnavailable` is its own type rather than a `KeyError` arm — every other
arm there says "these bytes are not the thing you claimed", and this says "the
machine could not give me entropy". It converts into `KeystoreError::NoRandomness`,
also a new arm rather than the existing `Io`, whose message is *"keystore could not
be read"* and would send a reader to check a file that does not exist yet.

**What no test can show, said plainly:** `getrandom` cannot be made to fail from a
test without installing a seccomp policy, and a test that installed one would be
testing the sandbox. `minting_a_key_is_fallible_rather_than_a_panic` pins the
*shape* — the signature, the message naming a fix, the conversion — which is what
would have to be undone to reintroduce the panic. The failure itself is unobservable
and is not claimed.

**A gap this exposed, and closed:** `every_error_message_names_a_fix` enumerates
`KeystoreError`'s variants **by hand** and, unlike its sibling
`every_reason_is_distinguishable_from_every_other`, had **no count guard** — so
`NoRandomness` went in silently uncovered. It now carries the same hardcoded
`assert_eq!(all.len(), …)` the sibling already used, which is what would have caught
it. An exhaustive `match` would be stronger (a compile error rather than a failing
assertion) and was written and then dropped: the file already had the count idiom in
one of the two tests, and adding a second mechanism for one rule is the duplication
this codebase argues against more than it is worth the extra strength.

### Two comments and a helper corrected rather than left to rot

Grouped because they are one class — a document or comment that outlived the code —
and because CLAUDE.md treats that class as a defect rather than tidiness.

- **`parse_stoa` now has six call sites, not three.** It was extracted for this
  change's three handlers while the three pre-existing inline copies were left.
  They were behaviourally identical, which is why nothing failed and why it was
  worth fixing anyway: the bill arrives on the next change that tightens the parse,
  which lands in one place while three handlers keep the old behaviour and all their
  tests keep passing. Converting them is a no-behaviour-change refactor that
  should have preceded the feature.
- **`parse_index` uses `usize::try_from`, not `as usize`.** On a 64-bit target the
  cast is lossless; on a 32-bit one it truncates, so `{"index": 4294967296}` would
  become `0` and keep candidate 0 — the exact coercion the spec forbids, reached by
  a cast rather than by a decision. CI builds no 32-bit target, so this is
  unreachable today; the point is that the property stops depending on the target.
  Its doc comment also now names all three callers and argues from the **severe**
  one: it previously explained the refusal entirely in pagination terms, so a reader
  arriving from `keep_identity` was told about serving the wrong page of a feed.
- **The slate reply's key set is asserted exactly.** The test's name and
  `slate_json`'s doc comment both claimed "the exact shape" while the test checked
  only that each expected key was *present*. The spec-test reviewer measured it: a
  `displayName` added to every candidate left all 553 tests green, which is precisely
  the scenario "A generated name and a mark are not settled by this capability"
  exists to prevent. Now an **added** key fails too — verified by re-applying that
  mutation.

**`path` in the three replies is marked `NO SPEC:`.** No requirement or scenario
names a reply field for it; exposing it is a decision (it is not secret, and a view
that can show which path is about to be kept can render the recovery warning
truthfully). Marked so a later reader can decide whether it was right rather than
inheriting it by silence — readability review noted that a reviewer grepping
`NO SPEC` would otherwise conclude the wire replies carry nothing unspecified.

## Risks / Trade-offs

**A crash between the keystore write and the path write leaves a keystore with
no recorded path** → **The user can now repair this by choosing again, and could
not when this entry first claimed they could.** The correction is worth keeping
because the wrong version was convincing.

It said the state was *"recoverable rather than silent"* and that the path record
was *"derivable from nothing but the user's next choice"*. Design review showed
that choice **could never be made**: the next `keepIdentity` hit `Keystore::create`
→ `AlreadyExists` and returned before `record_path` was reached. So `whoAmI`'s
reason told the user to generate a slate and keep a candidate, `getCapabilities`
agreed, and the keep then refused with a message about a keystore they did not
know they had — a loop with no exit, through every wire method this change adds,
repairable only by deleting the keystore file by hand and throwing away the master
key. The mechanism this entry named was right; the conclusion drawn from it was
not.

The second-keep fix closes it as a side effect rather than by design: a keep whose
keystore already exists now reuses it and proceeds to `record_path`, so the next
choice does land. `whoAmI`'s reason is therefore now a fix that works, which is
what `KeystoreError::Display`'s name-the-fix obligation requires of it.

What is still *not* offered is an automatic repair, because repairing means
choosing a path on the user's behalf and the spec forbids coercing a selection.
That part was always right.

**The salt bump means an identity derived under the old two-input scheme is not
reachable through the new path-taking one** → Deliberate, and the point of the
requirement. No keystore exists in the field to be stranded, because this change
mints the first one. Recorded here because the cost is not zero for any later
change that wants to reverse it.

**The slate's five candidates are derived from one nonce, so a nonce that leaked
would let someone enumerate the five paths a user was offered** → A path is not
secret; the spec says so explicitly ("The record SHALL NOT be required to be
secret"). Knowing which paths were offered reveals nothing the kept identity's
published address does not already reveal, and the master key is what the
candidates actually need.

**`derive_stoa_key_at_path` leaves a derived per-Stoa seed on the stack unwiped,
and now does so five times per slate** → Residual memory, not a reachable leak:
nothing reads those bytes back, and `identity.rs` is explicit that memory lifetime
is *"`crate::keystore`'s to own"*. What changed is that the layer up no longer owns
it for this path. That deferral was written when `derive_stoa_key` ran once at
keystore setup; it now runs five times per `generateIdentitySlate`, on a method a
caller may invoke without limit — and regeneration being unbounded is itself a
pinned requirement, so 200 rounds in a test is 1000 unwiped seeds.

Not fixed here, and the reason is scope rather than judgement: `zeroize` would have
to reach into `identity.rs`, which currently has no such import and whose whole
posture is that it holds no memory obligations. That is a change to `identity`'s
contract and belongs in its own. What **is** fixed is the comment in
`onboarding.rs` that asserted the obligation was discharged — security review found
it claiming *"there is no plain copy of the seed to wipe, because none is made"* on
the line above a `to_bytes()` call.

**`Keystore::stoa_key`, `stoa_public_key` and `stoa_address` now have no production
caller** → The probe was the last one, and it moved to the path-taking trio. They
stay because `identity.rs`'s non-collision test needs the primitive and because
deleting three public methods from the secret-holding type widens this change into
a contract `proposal.md` declares untouched. Recorded rather than done silently:
architecture review flagged the *path* trio as speculatively added when only one of
its three had a caller, and this is the same observation arriving from the other
direction. A later change should retire them or say why they stay.

**Two SQLite files where there was one** → One more open on the keep path, and
one more file for a backup to remember. Accepted against bricking every existing
op log, which is the alternative.

**`identity.sqlite`'s `check_layout` proves its columns exist and nothing about
their constraints** → This entry previously said the store had **no**
`check_layout` at all, recorded as "a known asymmetry rather than copied". That
described a tree that does not exist: `identity_store.rs` has one, it names both
columns, it is called from `from_connection`, and two tests cover the behaviour the
entry called absent. The guard was declined in the design and then written during
implementation, and the reversal was never carried back — readability review caught
it, and it is the more misleading direction, because a document claiming *less*
than the code does fails nothing.

What is genuinely asymmetric is narrower and is the real risk. `check_layout` runs
`SELECT stoa, path FROM chosen_paths LIMIT 0`, which proves the two column *names*
exist and nothing about keys or constraints. So a replaced file whose
`chosen_paths` has no `PRIMARY KEY` opens `Ok`, and `path_for`'s
`query_row(...).optional()` then returns whichever of two rows for one Stoa SQLite
hands back first — an identity chosen by physical row order. Security review
measured it. It is not closed here: the fix is a constraint check at open, which is
its own change, and the disk-content family it belongs to is the same one
`path_from_row`'s bound belongs to. **Recorded so the next reader knows the
one-path-per-Stoa invariant is a property of files *this build* wrote, not of every
file it will open.**
