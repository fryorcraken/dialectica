# Delete the author address, now that the public key is the identifier

## Why

Issue #80 makes the **public key the sole author identifier**. Its first half
landed as `key-identity` (`9357417`): the name, the mark and the on-screen
abbreviation now read **disjoint raw byte slices of the public key**, with no
hash and no prefixes. `NAME_PREFIX` and `name_digest()` are gone.

That piece deliberately left the author address in place, so the tree would
compile between pieces. `PublicKey::address`'s own doc comment records the
arrangement: *"Scheduled for deletion by `key-identity-sweep` (issue #80) … This
survives only because the sweep owns the call-site rewiring. Do not add a
caller."* **This change is that sweep.**

Leaving it is not untidiness. Three things are actively wrong while it stands:

- **A spec requirement now contradicts the shipped design.** The `identity`
  capability requires *"An address is derived from a record, never from a bare
  key"*, and `generated-names` — rewritten by `key-identity` — states that **the
  public key is the sole author identifier** and that there is *"no address to
  derive from"*. Two live capabilities give opposite answers about the same
  value.

- **A reply carries an identifier a caller cannot use.** A feed row reports its
  author as a hex address and carries no public key. `generated-names` establishes
  that a caller holding only an address **cannot arrive at the right name** — the
  name reads the key's own bytes. So every feed row today names an author in a
  form from which neither the name nor the mark can be computed.

- **The re-derivation check it exists to enable is already vacuous on the ingest
  path.** `SignedOp::verify` computes the claimed author by calling `.address()`
  on the op's **own** key and passes both to `verify_authored_op`, which compares
  `key.address()` against it — a value against itself. It cannot fail. The check
  is real only on probe and keystore round-trips, where it degrades to comparing
  two derivations of one key.

## What Changes

- **BREAKING: the author address is deleted.** The derivation
  (`PublicKey::address`) and its domain prefix go, along with every method that
  returns one and every reply field that carries one.

- **BREAKING: every reply that named an author by address now names it by public
  key.** The feed row's author field, the thread item's author field, the
  capability probe's identity field, and the onboarding slate's candidate and
  kept-identity fields each carry the key's hex where they carried an address's.
  Both values are 32 bytes and render as 64 hex characters, so **no field changes
  shape, length or type** — what changes is which value it holds. A caller that
  compared a stored address to a new reply gets no error, only a mismatch, which
  is why this is called out as breaking rather than as a widening.

- **The thread item stops carrying two author fields and carries one.**
  `thread-read` required both an address *and* a key on every item, because the
  name read the key while the mark read the address and neither was recoverable
  from the other. **That premise is gone** — both channels now read the key — so
  the second field is a value nothing derives from.

- **The verification contract stops binding a key to a separate claimed
  identifier, because there is no longer a separate identifier to bind it to.**
  What survives is stated honestly: a verification establishes that the key
  carried by the op signed the op's bytes, and the author *is* that key. The
  forgery the old requirement described — sign with your own key, claim someone
  else's address — **is not expressible** once the author is the key that signs.

- **The onboarding candidate stops carrying an address.** It already carries the
  public key beside it.

- **Three specs still say the mark is derived from the address, and each is
  corrected.** `thread-read`, `identity-onboarding` and `view-identity-onboarding`
  each justify carrying an address by naming it as the **mark's derivation
  input**. `key-identity` already moved the mark onto key bytes `4..11`, so those
  sentences describe a derivation that no longer exists. **This is a correction of
  stale prose, not a behaviour change** — the mark moved in the previous piece, and
  `generated-names` is the authority on where it reads. Left uncorrected, deleting
  the address would read as taking the mark's input away.

- **One design option is genuinely given up, and it is recorded rather than
  quietly dropped.** `identity` justifies the record-hashed address by saying it is
  what keeps the door open to key rotation: a record holding one key today could
  grow into a key *log*, and addresses would survive it. Deleting the address
  closes that door. Rotation is already forbidden by a standing requirement, so
  nothing shipped depends on it — but the affordance was the stated reason for the
  record, and a change removing the record should say so rather than strike the
  sentence as though it were incidental.

- **The feed row's author field changes with no spec requirement governing it, and
  that is a gap this change inherits rather than creates.** No capability owns the
  feed reply's shape: `generated-names` forbids a name on a feed row, and nothing
  states what a feed row *does* carry. So the edit — an address's hex becomes the
  key's hex — is made with the code as its only authority.

  It is made anyway, because leaving it is worse: the row would carry an
  identifier whose derivation no longer exists. **A side effect worth naming is
  that the feed's display name becomes derivable**, the row gaining the
  derivation's input by the same edit, which closes one of the two gaps
  `expose-name` recorded as open.

  **Writing the missing requirement is not in this piece.** A feed reply contract
  is a capability's worth of work — every field, the pagination shape, the
  moderation flags — and bundling it into a deletion would make neither half
  reviewable. What this change owes is that the gap is stated rather than
  discovered later from a diff.

**Deliberately not in scope:**

- **`OP_SIGNING_PREFIX` stays.** It separates the signing digest, and is what
  stops a signature over one preimage being replayed as a signature over another.
  "No prefixes" was the owner's ruling about the *identity* derivations and
  reaches nothing on the signing path.

- **The `display_name` wire method's contract is not redesigned.** It shipped in
  `expose-name` and is specified against a public key input. A rename is under
  owner consideration and is recorded in that change's archived `design.md`; it
  is not this piece's.

- **No data migration, and this was measured rather than assumed.** The SQLite
  `author` column already stores `entry.op.op.author.to_bytes()` — the **public
  key**. `Op` carries `author: PublicKey`, and the op encoding's only 32-byte
  `Address` is the **Stoa** one. Nothing on disk and nothing inside a signed
  preimage changes, so no stored op is reinterpreted and no re-sync is forced.
  This is a wire-API break and nothing more.

## The hazard this change creates, which is the review burden

**After this change `Address` means "a Stoa address", and nothing in the type
system enforces it.** One Rust type served both kinds, separated only by which
domain prefix produced it. Deleting the author half leaves a type whose name no
longer says what it holds, and whose doc comment — *"One type for both"* — will
be false.

Three places this is sharper than it looks:

- **Two keystore methods are named for the wrong thing already.** They take a
  **Stoa** address and return an **author** address. A reader deleting "address
  methods" by name will reach for the wrong ones, and a reader keeping "Stoa
  methods" by name will keep exactly the two that must go.

- **`derive_stoa_key(root, stoa: &Address)` keeps its `Address` parameter**, and
  must. The Stoa address is an input to author-key derivation; that it is a Stoa
  address is what makes identities per-Stoa.

- **The pinned-constants requirement covers both derivations in one place.** The
  author-address pin goes; the Stoa-address pin, the signing-digest pin and the
  key-derivation pin stay, and deleting the requirement wholesale would silently
  take three live guarantees with it.

The specs below state the survivor positively — that an address identifies a
Stoa — so that the meaning is recorded somewhere a reviewer can check, rather
than left to the absence of the other kind.

## Capabilities

### New Capabilities

None. This change removes a value and re-points the requirements that named it;
there is no new behaviour to describe, and a capability for "the absence of an
address" would be a file with nothing to test.

### Modified Capabilities

- `identity`: **removes** *An address is derived from a record, never from a bare
  key* — the requirement `generated-names` now contradicts outright. **Rewrites**
  *Verification binds the key to the claimed author* against what verification
  actually establishes once the author is the signing key. **Rewrites** *The
  derivation constants are pinned against silent change* to drop the author-address
  pin and keep the other three. **Rewrites** *An address's display form parses
  strictly* and *Identity does not rotate* to say Stoa where they said address, and
  to stop resting on a record-hashed address that no longer exists.

- `thread-read`: **rewrites** *An author is reported as both an address and a
  public key, and never as a name* into a single-field contract naming the public
  key, on the ground that the two-field justification — two channels reading two
  digests — no longer holds.

- `identity-onboarding`: **rewrites** the slate and kept-identity reply contents to
  drop the candidate address, keeping the public key already beside it, and
  rewrites the distinctness scenario that compared addresses. **Its closed-field
  requirement is a forced consequence** — a reply's field set is required to be
  exactly what the other requirements name, so removing a field from them narrows
  it whether or not anything says so.

- `view-identity-onboarding`: **rewrites** the candidate row's contract, which
  today requires the full **address** on screen and a mark derived **from that
  address**, and the requirement stating that the address is what tells two
  participants apart. This is the view half of the same contract.

- `op-format`: **rewrites** the sentence requiring the address to be recoverable
  from the key, the op-id separation clause naming an author address, and the
  sender-identifier requirement's "the key it carries and the address derived from
  it". The op **encoding is unchanged** — this is prose naming a value that stops
  existing.

- `op-transport`: **rewrites** the inbound validation requirement. It lists five
  distinguishably-refused failures, one of which is *"a payload carrying an op
  whose signature does not verify, **or whose presented key does not bind to the
  author it claims**"*. The second half of that clause becomes unstatable — there
  is no separately-claimed author for a key to fail to bind to — so the clause
  reduces to the signature check. **The list stays at five**, and the prose's
  "these five failures" is still right: the two halves were always one bullet,
  and it is the bullet's second mechanism that goes, not a bullet.

- `content-authoring`: **audited and deliberately unchanged.** Its publish guard
  forbids a request supplying "an author, an identity, a key, or an address", and
  the obvious edit is to drop the last. It is not made: the list is a **defensive
  prohibition on inbound fields**, not a description of what an author is, and
  refusing a field that no longer names anything costs nothing while narrowing the
  guard buys nothing. A caller sending `address` is confused about this API either
  way, and being told so is the right answer. Recorded here because "the sweep
  missed it" and "the sweep considered it" are indistinguishable from the delta
  alone.

- `stoa-membership`, `stoa-genesis`, `op-log`, `moderation-resolution`,
  `post-revision`, `stoa-navigation-view`: **audited and unchanged.** Every
  address in them is a Stoa address, a content-addressed attachment reference, or
  a genesis preimage. Listed here so that "not in the delta" reads as a checked
  result rather than an omission.

- `posting-capability`: **audited and unchanged**, and worth naming because the
  capability probe is one of the six reply sites whose value changes. Its
  requirements are written over *"the identity that would post"* and *"the one an
  op published now would be attributed to, derived from the key that would
  actually sign it"* — never over an address. The contract is already stated at
  the right altitude, so the value it reports changes while every requirement
  governing it stays true.

- `stoa-metadata` and `op-format` each close a requirement with the bare sentence
  **"The address is the identity."** Both are Stoa-scoped by their own preceding
  clause, so neither requirement changes meaning — but the unscoped sentence is
  the one a reader will cite afterwards as authority for the thing this change
  removes. Both are scoped to say *Stoa*, in this change's `stoa-metadata` and
  `op-format` deltas. (Review caught that this sentence was written in the past
  tense while neither edit had been made — `stoa-metadata` had no delta at all.
  Both deltas now carry the requirement and the one-sentence change.)

### One value, three field names — created here, filed rather than fixed

**After this change a single Ed25519 public key is reported under three
different field names across six replies**, and that condition is this change's
doing even though none of the names is. It is recorded here because the standard
this proposal sets for itself is that a gap is *stated* rather than discovered
later from a diff.

The six sites: a thread item's `author` and a feed row's `author`; a slate
candidate's `publicKey`, a kept identity's `publicKey` and whoAmI's `publicKey`;
and the capability probe's `identity`. Every one now carries the same 64-hex
public key.

**Before this change the three spellings named three genuinely different
values** — `identity` and `author` were author addresses, `publicKey` was the
key — so distinct names were correct. Collapsing the two values into one is what
made them redundant, and the consequence is live: `DOnboardingScreen.qml` reads
`publicKey` while `FeedScreen.qml` reads `author`, and both feed the same
`Identicon.address` property with the same value. A view holding a thread item's
`author` that wants a display name must re-spell it as `publicKey` to call
`display_name`, whose contract is `{"publicKey":"<64 hex chars>"}`. That is a
branch on where the data came from, which CLAUDE.md's *"JSON shapes are
source-independent"* exists to prevent.

**Not fixed here, deliberately.** Unifying six reply fields across four
capabilities is a breaking wire change of its own size, and bundling it into a
deletion would make neither half reviewable — the same argument this proposal
uses to defer the feed-row contract. `design.md` §4 reasons about the spelling
only *within* the thread/feed family and never across the six sites, which is
the gap this paragraph closes.

## Impact

- **`dialectica/rust-lib/dialectica-core/src/identity.rs`** loses
  `PublicKey::address` and `AUTHOR_ADDRESS_PREFIX`, and `verify_authored_op` loses
  the claimed-author parameter it can no longer be given — with it goes the guard
  that was comparing a value to itself. `Address`, its hex parser, `stoa_address`
  and both `derive_stoa_key*` functions stay, and the type's doc comment must stop
  saying "one type for both".

  **Several doc comments assert the opposite of what the code does** and are
  corrected in the same pass: `op.rs`' claim that `verify_authored_op`
  "re-derives" the address and "binds the key to the claimed address",
  `identity.rs`' claim that the guard "rejects" a valid signature by the wrong
  key, and two test comments crediting the address check for a refusal the
  **signature** check produces. Those tests pass today and will keep passing;
  what is wrong is the stated reason.

- **`keystore.rs`** loses `identity_address`, `stoa_address`,
  `stoa_address_at_path` and `poster_address_in`, and `creator_and_poster_in`
  collapses to returning the key alone. Each has a key-returning sibling already,
  so these are deletions rather than replacements. **Every `Address` a keystore
  method returns is an author address** — measured, not assumed: a Stoa address
  only ever arrives as a parameter there. That is the fact the two
  Stoa-sounding names hide.

- **`feed.rs`** and **`thread.rs`** change which value their author field
  carries; **`onboarding.rs`** loses `Candidate.address`. **`wire.rs` has six
  production reply sites emitting an author address** — two `author` fields (feed
  and thread), three `address` fields (slate candidate, kept identity, whoami) and
  one `identity` field (the capability probe) — fed by three places that mint the
  value. `wire.rs`' `"authorKey"` field already carries the public key and stays.

- **`wire.rs`' inbound `FORBIDDEN_FIELDS` guard stays.** It refuses a publish
  request that supplies `author`, `identity`, `key` or `address` itself. That is
  an input guard against a caller naming its own authorship, and it is unrelated
  to which value core *reports*.

- **A `wire.rs` test calls `.address()` and stops compiling.** That is the good
  kind of breakage and is not to be worked around: it is the compiler naming a
  site the sweep must reach.

- **Three QML files, not two.** `FeedScreen.qml` and `PostHeader.qml` carry an
  author address through to rendering — the property name `identityAddress` stops
  describing what it holds — and `DOnboardingScreen.qml` reads the `address` field
  of the slate and kept-identity replies, which is an author address too.
  `Identicon.qml` and `AddressLabel.qml` are **generic** components that also
  render Stoa addresses; both already carry comments anticipating this change, and
  neither is swept.

- **`docs/PLAN.md`** §5.1 still states the record-hashed address construction as
  live, with no strike-through. It is pruned by this change. §5.2's
  *"no wire-format change, no address change"* and the §11.1 rendering obligations
  are checked in the same pass. **The §11.1 layer-4 bullet and the two
  rendering-obligation bullets were already corrected by `key-identity`** — they
  read *"~~The address~~ The public key"* today, so this change confirms them
  rather than editing them.

- **`docs/UI-BRIEF.md` is out of scope by owner ruling** — its content was ruled
  misleading, and `openspec/specs/` is the authority.
