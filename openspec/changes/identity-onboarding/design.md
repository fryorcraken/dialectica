## Context

See `proposal.md` — Why. The gap is that `keystore.rs` can store a root secret
and nothing mints one, so `getCapabilities` is permanently `canPost:false`.

Four properties of the existing code shape every decision below, and each was
verified rather than assumed.

**`derive_stoa_key` is on the live path, and the chain is three hops.** Verified
at `dialectica/rust-lib/src/lib.rs:251` → `keystore.rs:651` `stoa_address` →
`:645` `stoa_public_key` → `:640` `stoa_key` → `:641`
`derive_stoa_key(&self.root, stoa)`. This change adds a path input to that
chain; it does not route around it.

**The op log's schema cannot absorb a new table.** `log/sqlite.rs` holds
`LAYOUT_VERSION` in `PRAGMA user_version`, `check_layout` proves the declared
layout against the columns every read touches, and `create_schema`'s doc comment
states outright that **there is no migration path by design** and that a later
"add a migration" refactor is the change its warning is addressed to. Adding a
`chosen_paths` table to `ops.sqlite` is therefore a `LAYOUT_VERSION` bump that
permanently bricks every store written by a prior build.

**The core crate is pure and the module instance is `Default`-constructed.**
`dialectica-core` has zero SDK types and no ambient state; the module struct
`Dialectica` holds exactly one field (`persistence_path`), set only by
`on_context_ready`. `interface: "universal"` scans the impl header's `public:`
section, so the constructor must stay genuinely parameterless.

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

Keeping is two writes — `Keystore::create` and the path record — and the spec
requires it "either complete or change nothing".

**The keystore is written first, and this ordering is the whole of the
atomicity story.** `Keystore::create` already refuses to overwrite
(`KeystoreError::AlreadyExists`) and already writes atomically through a random
staging path. So:

- If the keystore write fails, nothing was written anywhere. Clean.
- If the keystore write succeeds and the path record fails, the keystore exists
  with no recorded path. **That is the one partial state, and it is recoverable
  rather than silent**: the keystore write is the irreversible half (a fresh
  master key), the path record is derivable from nothing but the user's next
  choice, and a keep that failed reports the failure. The identity is not
  reported as kept, and `whoAmI` will not name one, because it reads the path
  record — which is the spec's "A failed keep records nothing: no identity is
  reported as kept".

The reverse order would be worse in a way worth stating: a recorded path with no
keystore is a record naming a master key that does not exist, and the *next*
keep — with a different master key — would silently inherit it.

**What this does not claim.** It is not a two-phase commit, and a crash between
the two writes leaves the keystore on disk. The honest statement is that the
spec's requirement is met at the level of *reported* state: no keep that did not
complete reports an identity, and a load after a failed keep finds no identity.
Making the pair genuinely atomic would need the path record inside the keystore
file, which `keystore`'s spec forbids.

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

## Risks / Trade-offs

**A crash between the keystore write and the path write leaves a keystore with
no recorded path** → The state is recoverable and not silent: `whoAmI` names the
missing record rather than reporting no identity, and a second keep is refused by
`AlreadyExists` rather than minting a second master key. What is *not* offered is
an automatic repair, because repairing means choosing a path on the user's behalf
and the spec forbids coercing a selection.

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

**Two SQLite files where there was one** → One more open on the keep path, and
one more file for a backup to remember. Accepted against bricking every existing
op log, which is the alternative.

**`identity.sqlite` has no `check_layout` equivalent** → The op log's version
check proves its declared layout against the columns it reads, and this store
does not. It is a two-column table read by two statements, so the failure the op
log's check exists to catch — a stamped version over absent tables — surfaces
here as a named error on the first read rather than as `Storage("no such
table")`. Recorded as a known asymmetry rather than copied, because copying it
would be copying 40 lines of machinery for a table with two columns.
