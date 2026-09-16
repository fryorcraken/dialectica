## Context

See `proposal.md` — *Why*. The first half of issue #80 landed as `key-identity`
(`9357417`): the display name, the mark and the on-screen abbreviation read
disjoint raw byte slices of the public key. It deliberately left
`PublicKey::address` standing so the tree compiled between the two pieces. This
change deletes it and rewires every call site.

Three constraints shape the approach:

- **One Rust type, `Address`, served both kinds** — an author's and a Stoa's —
  separated only by which 32-byte domain prefix produced it. Deleting the author
  half leaves a type whose name no longer says what it holds, and nothing in the
  type system enforces the narrowing.
- **`cargo test` compiles nothing behind `#[cfg(logos_scaffold)]`.** The adapter
  (`dialectica/rust-lib/src/lib.rs`) is invisible to the suite, to clippy and to
  fmt. A compile error there reaches review as a green local run.
- **No data migration is possible to get wrong, and this was measured.** The
  SQLite `author` column stores `entry.op.op.author.to_bytes()` — the public key.
  `Op` carries `author: PublicKey`; the op encoding's only 32-byte `Address` is
  the Stoa. No stored op is reinterpreted and no signed preimage moves.

## Goals / Non-Goals

**Goals:**

- Delete `PublicKey::address`, `AUTHOR_ADDRESS_PREFIX`, and every method whose
  return value was an author address.
- Make the five reply fields that named an author by address name it by key, and
  delete the three that were a second identifier beside a key already present.
- Correct every doc comment and test comment that asserts a mechanism the code
  does not have.

**Non-Goals:**

- **Renaming `Address` to `StoaAddress`.** Considered and rejected below.
- **Writing the missing feed-reply requirement.** The proposal records the gap
  deliberately; a feed contract is a capability's worth of work.
- **Redesigning `display_name`'s wire contract**, which shipped in `expose-name`.
- **Reformatting the nine unformatted files in `dialectica-core`** — the known
  CI fmt-gate gap, pre-existing and orthogonal.

## Decisions

### 1. Delete by call site, never by identifier

Two keystore methods are named for the wrong thing already: `stoa_address` and
`stoa_address_at_path` each take a **Stoa** address as a parameter and return an
**author** address. A sweep that deleted "address methods" by name would reach
for `stoa_address` (the free function in `identity.rs`, which is the Stoa
derivation and must stay) and would keep the two `Keystore::stoa_address*`
methods, which are exactly the ones that must go.

So the deletion is driven by what a function *returns*, checked at each site:

| Kept | Deleted |
|---|---|
| `identity::stoa_address(genesis_bytes) -> Address` | `Keystore::stoa_address(&self, stoa) -> Address` |
| `identity::derive_stoa_key(root, stoa: &Address)` | `Keystore::stoa_address_at_path(&self, stoa, path) -> Address` |
| `identity::derive_stoa_key_at_path(root, stoa, path)` | `Keystore::identity_address(&self) -> Address` |
| `Genesis::address() -> Result<Address, _>` | `keystore::poster_address_in(dir) -> Result<Address, _>` |
| `Address`, `Address::from_hex`, `AddressError` | `PublicKey::address(&self) -> Address` |
| `STOA_ADDRESS_PREFIX`, `OP_SIGNING_PREFIX` | `AUTHOR_ADDRESS_PREFIX` |

**Measured, not assumed:** every `Address` a keystore method *returns* is an
author address; a Stoa address only ever arrives there as a parameter. That fact
is what the two Stoa-sounding names hide, and it is why the table above is
organised by return type rather than by name.

`creator_and_poster_in` collapses to `creator_key_in`: its second element was
`creator.address()`, a value derived from the first. A pair whose second half is
a pure function of its first is not a pair — and the pairing existed to stop a
creator key and a poster key from being two different derivations, which is a
property the single return value now carries by construction rather than by two
callers agreeing.

### 2. `Address` keeps its name; the doc comment carries the narrowing

**Rejected: renaming `Address` to `StoaAddress`.** It is the change that would
make the invariant structural, and it is not made here, for two reasons.

The mechanical one is blast radius: `Address` appears in `op.rs`'s `Op.stoa`
field, in `stoa.rs`, `feed.rs`, `thread.rs`, `membership.rs`, `moderation.rs`,
`transport.rs`, `wire.rs` and the adapter, and a rename touching all of them in
the same diff as a deletion would make neither half reviewable. CLAUDE.md's rule
is *make the change easy, then make the easy change* — a rename is a separate
commit that changes no behaviour, and it is not needed to make this change
small.

The substantive one is that the rename would buy less than it appears to. The
type is still a bare `[u8; 32]` newtype with an infallible `from_bytes`, so a
misuse it would catch is one where somebody has a 32-byte value and calls it the
wrong thing — and after this change there is no other kind of 32-byte identity
value for it to be confused with. What remains confusable is `OpId`, which is
already its own type.

What is done instead is to **state the survivor positively** in the type's doc
comment: an `Address` identifies a Stoa. The old comment's *"One type for both"*
becomes false the moment `PublicKey::address` is deleted, so leaving it would be
leaving a false claim in the file the next reader opens first.

Recorded so a later reader does not have to re-derive it: **`derive_stoa_key`'s
`stoa: &Address` parameter stays and must.** The Stoa address is an *input* to
author-key derivation — that it is a Stoa address is what makes identities
per-Stoa — so a sweep that read "`Address` beside identity derivation" as a
leftover would remove the thing that makes per-Stoa identity per-Stoa.

### 3. `verify_authored_op` loses its parameter rather than its guard being
   relaxed

The guard being deleted is `if key.address() != *author { return false }`, and
the reason is not that it became inconvenient: on the ingest path it was already
**comparing a value to itself**. `SignedOp::verify` computes the claimed author
as `self.op.author.address()` — `.address()` on the op's own key — and passes it
beside `self.op.author.to_bytes()`. The comparison is `k.address() == k.address()`
and cannot fail for any input, malformed or hostile.

Three shapes were considered:

- **Keep the parameter, drop the guard.** Rejected: a parameter nothing reads is
  a parameter a caller will supply wrongly and never learn.
- **Keep the function taking only a key and rename it.** Rejected: the name
  `verify_authored_op` still says the true thing — it verifies an op against the
  author, and the author is now the key. A rename would churn every call site for
  no gain.

  This bullet carried a second ground — *"the `op-transport` spec still calls
  this the verification step"* — which was **withdrawn after review as an
  unsupported citation**. `grep -in "verify_authored_op\|verification step"`
  over both `openspec/specs/op-transport/spec.md` and this change's delta
  returns zero matches: the spec names no function and uses no such phrase. It
  does describe the behaviour the name refers to — *"an op whose signature does
  not verify under the public key that op carries"* is refused — but that
  supports the name being accurate, not a claim about the spec's vocabulary.
  The rejection stands on the churn argument alone, which was always the real
  one.
- **Delete the parameter (chosen).** The signature becomes
  `verify_authored_op(key_bytes, op_bytes, signature_bytes) -> bool`, and the
  three values it consults are exactly the three the `identity` spec's scenario
  *Verification takes no author identifier beside the key* enumerates.

The narrowing is real and is stated rather than hidden: verification establishes
that the key the op carries signed the op's bytes. The forgery the old contract
described — sign with your own key, claim someone else's address — **is not
expressible** once the author is the key that signs. Substituting the author
substitutes the key, and the signature then fails under it.

**Nothing is lost on the probe and keystore round-trip paths either**, which is
where the check was not vacuous. There it compared two derivations of one key,
which is a tautology one step longer. The tests at those sites are rewritten to
compare the reported **key** against an independently derived one, which is the
assertion they were always trying to make.

### 4. A field that becomes a duplicate is deleted, not repointed

Six production reply sites emitted an author address. They split two ways, and
the split is not arbitrary:

**Repointed — the field was the only author identifier the reply had:**

| Site | Field | Was | Now |
|---|---|---|---|
| `feed.rs` `Row` | `author` | `author.address().to_hex()` | `author.to_hex()` |
| `thread.rs` `Item` | `author` | `author.address().to_hex()` | `author.to_hex()` |
| `wire.rs` `Capability::CanPost` | `identity` | `stoa_address_at_path(…)` | `stoa_public_key_at_path(…)` |

**Deleted — a public key was already beside it:**

| Site | Field deleted | Field that stays |
|---|---|---|
| `wire.rs` `slate_json` candidate | `address` | `publicKey` |
| `wire.rs` `Kept::Stored` | `address` | `publicKey` |
| `wire.rs` `Whoami::Identity` | `address` | `publicKey` |

Carrying both would put a derived value on the wire beside the material it
derives from, where the two could disagree and a recipient has no way to tell
which is wrong. `identity-onboarding`'s closed-field-set requirement makes this
a forced consequence rather than a choice: the field set is required to be
exactly what the other requirements name, and none of them names an address now.

`thread.rs`'s `Item` is the one place where a field genuinely *disappears*
rather than changing value: it carried `author` (an address) **and** `author_key`
(the key). The two-field justification was that the name read the key while the
mark read the address, and neither was recoverable from the other. Both channels
now read the key. So `author` keeps its name and takes the key's hex, and
`author_key` goes — chosen over the reverse (deleting `author`, keeping
`authorKey`) because `authorKey` was the field added *for* the split, and a
single-field contract should be spelled the way every other reply spells it.

**This is the one place where the spec is silent and a choice was made.** The
`thread-read` delta requires *"every item carries its author's public key"* and
*"no item carries an author address"*; it does not say which JSON spelling
survives. A caller reading `authorKey` today gets a missing field rather than a
wrong value, which is the failure shape that is loud rather than silent, so the
choice is defensible either way — but it is a choice, and it is marked
`// NO SPEC:` at the test that pins it.

### 5. The feed row's edit has the code as its only authority

No capability owns the feed reply's shape: `generated-names` forbids a name on a
feed row, and nothing states what a feed row *does* carry. The edit — an
address's hex becomes the key's hex — is made anyway, because leaving it would
have the row carry an identifier whose derivation no longer exists.

The proposal records this gap deliberately and says writing the requirement is
not in this piece. What this change owes is that the gap is **stated** rather
than discovered later from a diff, and the test pinning the new behaviour carries
a `// NO SPEC:` marker naming what was chosen.

**The deferral now has a durable home: issue #91**, *Specify what a feed row
carries: the reply has no governing capability*. Filed during the review pass,
because this change's `findings/` directory is deleted at archive and a deferral
that leaves without landing somewhere was dropped rather than deferred. The
issue carries the concrete risk — a caller comparing a stored `author` across
this change gets a silent mismatch rather than an error, both values being 64
hex characters — and lists what a `feed` capability would have to settle.

A side effect worth naming: the feed's display name becomes derivable, the row
gaining the derivation's input by the same edit.

### 6. The pin is retired, not relaxed, and the retirement leaves a witness

`identity.rs`'s known-answer test pinned five derivations in one function: the
author address, the Stoa address, the signing digest, the per-Stoa key, and the
per-Stoa key at an explicit path — the last of these pinned at two inputs, so
six assertions covered five derivations. Deleting the requirement wholesale
would silently take four live guarantees with it, which is the
`hand-maintained sweep lists go stale silently` trap in its other direction.

So four derivations stay pinned verbatim, across five assertions, and the author
address is replaced by a **structural assertion of its absence**: nothing
derives an address from a public key. This
is what the `identity` spec's scenario *An author address derivation is pinned*
now asks for — it pins the absence rather than the value, so that the next reader
adding an author-side derivation finds evidence that one was removed on purpose
rather than nothing at all.

The record's key count (`0x01`) goes with the prefix: only the author-address
preimage carried it.

### 7. Doc comments that assert the opposite of the code are corrected in the
   same pass

Four claims in the tree are false about the code as it stands *after* this
change, and two were already false before it. They are corrected together
because a comment is a claim, and this repo has shipped several the code
disproves:

- **`op.rs:479`** — *"The address is recoverable from the key
  (`PublicKey::address`), which is what `verify_authored_op` re-derives."* After
  the deletion there is no address and no re-derivation.
- **`op.rs:721`** — *"`SignedOp::verify` rejects, since the address it re-derives
  will not match."* The refusal is the **signature** check.
- **`op.rs:734`** — *"the function that binds the key to the claimed address"*.
  There is no claimed address.
- **`identity.rs:680–693`** — `verify_authored_op`'s whole "the reason it exists
  is the address check" paragraph.
- **`op.rs:1412` and `op.rs:1424` (test comments)** — *"the author must be the key
  that signs, or the address check in `verify_authored_op` rejects it"* and *"Only
  re-deriving the address from the key catches it"*. **These tests pass today and
  will keep passing** — what is wrong is the stated reason. The refusal they
  observe is produced by the signature check, and was even before this change:
  `SignedOp::verify` derived the claimed author from the op's own key, so the
  address comparison could not have been what refused anything.

That last pair is the one worth being precise about. A test that passes for a
reason other than the one it names is the defect family this repo has shipped
most often; correcting the prose is what makes the next reader able to tell
whether the test still covers what it claims.

### 8. The rotation affordance is given up, and that is the change's one
   irreversible choice

**This is the largest thing this change forecloses, and it was not recorded as a
decision until review asked for it.** The reasoning reached the `identity` spec
and `identity.rs`; what was missing here is the half a Decisions section exists
for — what else was available, and what ruled it out.

`PLAN.md` §5.1 held a door open deliberately. An author address hashed a
*record* containing the key rather than the bare key, and the record was the
seam: it could have grown into a key log, so that an identity rotated its
signing key while the identifier everything else referenced survived. Deleting
the address closes that. After this change an author identity **is** the public
key, with no indirection behind it, so rotation is not merely unimplemented — it
has nowhere to live.

Three shapes were available:

- **Keep the address purely as a rotation anchor.** Rejected. The record it
  hashed had exactly one key in it, so the anchor pointed at a single key and
  rotating would have changed the address too — the affordance was *latent*, not
  present, and would have needed the key-log design built before it did
  anything. Meanwhile the address was live on the wire and in six replies,
  carrying a second identifier for every identity and a second thing every
  forgery check had to get right. Paying that in every reply and every test for
  a capability no code could yet use is the "reserved a byte that reserved
  nothing" shape this repo has shipped before.
- **Defer #80 until a credential layer exists.** Rejected, and this is the one
  that needed weighing rather than dismissing. §5.3's own argument is that
  rotation waits until standing lives on a *revocable credential* (§5.5) rather
  than on a keypair — so the credential layer, not the address, is what rotation
  was ever going to hang from. Waiting would have kept a value whose stated
  justification points somewhere else, and kept it while `generated-names` and
  the identicon were being built directly on the key, which would have meant
  building both against an identifier we intended to remove.
- **Delete it and accept the foreclosure (chosen).** The address's benefit was
  hypothetical and its cost was paid continuously.

**Whether this is a permanent loss or a deferred one is now an open question
rather than a settled answer**, and that changed after this change was designed.
Issue #89 proposes **versioning the identity** — a scheme version carried
alongside a public key's bytes, in the signed preimage — so that a future scheme
can coexist with Ed25519 rather than replacing it. A version seam is a different
and better indirection than the record hash was: it reaches rotation through
"which scheme minted this identity" rather than through "which key does this
record currently name". So the door this change closes may be reopened from the
other side, and #89 is where that is decided. Recorded here because a reader
finding this entry should not conclude rotation is impossible forever — only
that nothing in *this* change preserves it.

### 9. The slate's distinctness clause names the path the walk already checked

`identity-onboarding` required that no two candidates in a slate share a public
key **and** that no two share an address. The delta's second clause is now *no
two share a derivation path*, which reads like a new property substituted for a
deleted one. It is not, and the distinction is worth recording because the
substituted-clause reading is the natural one and it is wrong.

**The path clause is not new — only its presence in the spec is.** On
`origin/main` the test already asserted all three of path, key and address, and
`Slate::from_nonce`'s doc already explained that *"distinctness of paths is what
is checked, and it gives distinctness of keys and addresses for free"*. The walk
skips a path it already holds; that is the mechanism, and it predates this
change. What this change does is delete the address assertion — an address being
a pure function of the key, that clause could only ever have failed where the key
clause already had — and then bring the spec into line with the two properties
that remain and were always established.

So the spec moved toward the code rather than the code toward a widened spec. The
alternative was to state only the key clause, matching the old spec minus its
dead half. Rejected: it would have left the requirement silent about the one
property `Slate::from_nonce` is actually responsible for. A slate is built by
walking paths and the keys are what the walk *produces*, so a key-only clause
tests the derivation function — pinned elsewhere — rather than this function's
own job. Two candidates at one path is a real defect of this walk, and it would
reach a key-only assertion only by also breaking derivation: two explanations,
one answer, which is the shape this repo keeps finding in its test failures.

## Risks / Trade-offs

- **`Address` now means "a Stoa address" and nothing enforces it.** → The doc
  comment states it positively, `derive_stoa_key`'s parameter is documented as an
  input rather than a leftover, and every surviving `Address`-returning function
  is Stoa-scoped by name or by its own comment. A `StoaAddress` newtype is the
  structural fix and is deliberately deferred (Decision 2).

- **A wire break with no compile error at the caller.** Both values are 32 bytes
  and render as 64 hex characters, so a view comparing a stored address to a new
  reply sees a mismatch rather than an error. → Stated in the proposal as BREAKING
  rather than as a widening; the three QML files that carried the value through to
  rendering are swept in the same change, and the property name `identityAddress`
  is renamed so it stops describing what it no longer holds.

- **Deleting a field means the tests that asserted it vanish, and a vanished test
  is not a passing test.** → Every test that disappears is accounted for in the
  handover: deleted because its subject is gone, or rewritten to assert the new
  contract. A test whose only assertion was `candidate.address == …` is rewritten
  against the public key rather than dropped, because the property it was checking
  — that a candidate is identified by an unforgeable value that differs between
  candidates — survives the change of which value that is.

- **`cargo test` cannot see the adapter.** `dialectica/rust-lib/src/lib.rs` is
  `#[cfg(logos_scaffold)]`, and a compile error there reached review on #85
  exactly this way. → `nix build .#lgx` is run before the PR opens, and the
  adapter's `poster_address_in` / `creator_and_poster_in` call sites are rewired
  by reading the file rather than by trusting the suite.

- **`Identicon.qml` and `AddressLabel.qml` are not swept, and that reads like an
  omission.** → They are **generic** components that also render Stoa addresses;
  both already carry comments anticipating this change. Sweeping them would
  narrow a component that has a second, surviving caller.
