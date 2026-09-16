# Design review — key-sweep

**The decisions are in good shape.** All seven entries under `design.md`'s
Decisions are taken as recorded in the code, and every citation I could check is
real rather than plausible. Specifically verified, because the brief asked:

- **Deletion driven by return type** (Decision 1). Every row of the kept/deleted
  table is true against the tree: `identity::stoa_address` survives at
  `identity.rs:745`, `Genesis::address` at `stoa.rs:355`, `STOA_ADDRESS_PREFIX`
  and `OP_SIGNING_PREFIX` at `identity.rs:69` and `:77`; `AUTHOR_ADDRESS_PREFIX`
  appears only inside two comments describing its removal. The trap is recorded
  twice more where a reader will actually meet it —
  `Keystore::stoa_public_key`'s doc comment (`keystore.rs:789–798`) spells out
  that `stoa_address` took a Stoa address and returned an author one.
- **`verify_authored_op`'s vacuous guard** (Decision 3). Recorded *with* the
  evidence, not asserted: `SignedOp::verify` on `origin/main` passed
  `&self.op.author.address()` beside `&self.op.author.to_bytes()`, which I read
  in the old `op.rs`. The tester's stub-`verify_op_bytes` measurement is carried
  into three comments (`op.rs:1425`, `op.rs:1445`, `transport.rs:1439`).
- **`Address` left Stoa-only** (Decision 2). The `StoaAddress` newtype is
  considered, rejected with both a mechanical and a substantive reason, and
  named as deferred in Risks. `derive_stoa_key`'s parameter is documented as an
  *input* at `identity.rs:119–121`.
- **`Identicon` / `AddressLabel`** (task 4.3, Risks). Both left generic, both
  comments corrected; `Identicon.qml:87–97` no longer claims the rename is this
  piece's.
- **Both `// NO SPEC:` markers exist and both cite a design entry that exists** —
  `feed.rs:657` cites §5, `wire.rs:6696` and `wire.rs:1819` cite §4.
- **Decision 7's six line citations are accurate against `origin/main`** —
  `op.rs:479`, `:721`, `:734`, `:1412`, `:1424` each carry the claimed text. This
  is the opposite of the fabricated-citation failure the brief warned about.
- **`docs/PLAN.md` is consistent.** §5.1's record-hashing argument is struck
  through with the reasoning migrated to the `identity` requirement *Identity
  does not rotate*; §5.2, §11.1 layer 4, the two rendering obligations, the
  thread-read bullet and §11.1's normalisation paragraph are all in the past
  tense. The one surviving "The address is the identity" (`PLAN.md:3060`) is
  Stoa-scoped by its own paragraph and correct.

Three gaps follow. None is a code contradiction.

- [x] **`dev-writer`** — `design.md` Decisions — **the rotation affordance is
      given up, and that is not a Decision.** This change closes the door PLAN.md
      §5.1 held open deliberately: the author address hashed a *record* so it
      could grow into a key log and an identity could rotate while its identifier
      survived. Deleting the address forecloses that permanently. The reasoning
      migrated correctly out of PLAN.md — into `openspec/changes/key-sweep/specs/identity/spec.md:188–197`
      and `identity.rs:26–35` — but `design.md`'s Decisions section contains no
      entry, and `grep -i "rotat\|affordance\|key log" design.md` returns
      nothing. **This is the single largest irreversible choice in the change**
      and it is the one a reader of `design.md` alone would not learn was made.
      What is missing is the *alternatives* half: the proposal states the
      outcome, the spec states the outcome, and neither says what else could have
      been done (keep the address purely as a rotation anchor; delete it and
      accept the foreclosure; defer #80 until the credential layer exists) or
      what ruled each out. **Verified:** the decision is real and taken —
      `PublicKey::address` and `AUTHOR_ADDRESS_PREFIX` are gone from
      `identity.rs` — so the entry is a gap in the record, not a defect in the
      code.
      **Fixed.** Added as `design.md` Decision 8, *The rotation affordance is
      given up, and that is the change's one irreversible choice*, opening with
      an admission that it was not recorded as a decision until review asked.
      The alternatives half you identified as missing is the substance: all three
      of your candidates are weighed and each is given what ruled it out.
      **Keeping the address as a rotation anchor** is rejected because the
      record it hashed held exactly one key, so the anchor pointed at a single
      key and rotating would have moved the address too — the affordance was
      *latent*, not present, and would have needed the key-log design built
      before it did anything, while the address was meanwhile live on the wire in
      six replies. That is the "reserved a byte that reserved nothing" shape.
      **Deferring #80 until a credential layer exists** is the one I flag as
      needing weighing rather than dismissal, and it is rejected on PLAN §5.3's
      own argument: rotation waits on a *revocable credential* (§5.5), so the
      credential layer and not the address is what rotation was ever going to
      hang from — and waiting would have meant building `generated-names` and the
      identicon against an identifier we intended to remove.
      **One thing changed since the change was designed, and it changes the
      finding's framing**: issue #89 proposes versioning identity, so a scheme
      version could reach rotation from the other side. The entry records that a
      version seam is a different and better indirection than the record hash
      was, and closes by telling the reader not to conclude rotation is
      impossible forever — only that nothing in *this* change preserves it. So
      whether this is a permanent loss or a deferred one is now an open question
      with a named owner, rather than a settled answer.

- [x] **`dev-writer`** — `design.md` Decisions — **the slate distinctness clause
      was not dropped, it was replaced, and the substitution is unrecorded.** The
      live spec requires *"no two candidates in it share a public key"* **AND**
      *"no two share an address"* (`openspec/specs/identity-onboarding/spec.md:42–43`).
      The delta makes the second clause *"no two share a derivation path"*
      (`specs/identity-onboarding/spec.md:27–28`), and `onboarding.rs` follows:
      `Slate::from_nonce`'s doc now says "a public key or a derivation path" and
      `every_candidate_in_a_slate_is_distinct` asserts paths and keys.
      **Scenario:** a reader comparing the two specs sees an address clause
      become a path clause and has no record of why a *new* property was written
      in rather than the redundant clause simply removed. The obvious
      alternative — drop the second clause outright, since an address was a pure
      function of the key and so could only fail where the key clause already
      had, which is exactly what the test comment at `onboarding.rs:507–512`
      argues — was available and is what the code's own comment reasons toward.
      Adding path-distinctness instead is a defensible choice (it is the property
      the walk actually establishes and the one under the function's control),
      but it widens a spec requirement inside a deletion change and `design.md`
      does not mention `onboarding.rs`, the slate, or distinctness anywhere.
      **Fixed as `design.md` Decision 9 — but the premise needed correcting
      first, and the correction is the interesting part.** Your conclusion stands
      (this needed recording, and `design.md` said nothing about the slate); the
      mechanism is not what it looks like.
      **Path-distinctness was not written in as a replacement. It was already
      there.** `git diff origin/main...piece/key-sweep -- …/onboarding.rs` shows
      `every_candidate_in_a_slate_is_distinct` asserting **all three** of path,
      key and address on `origin/main`, and `Slate::from_nonce`'s doc already
      saying *"distinctness of paths is what is checked, and it gives
      distinctness of keys and addresses for free"*. The walk has always skipped
      a path it already holds. The diff deletes the address assertion and nothing
      else. So the code did not follow a widened spec — the **spec moved toward
      the code**, naming a property the implementation had established all along.
      That makes the reading you flag ("a new property was written in") the
      natural one and wrong, which is exactly why it needed an entry: a later
      reader comparing the two specs sees an address clause become a path clause
      and will infer a substitution.
      The alternative you name — drop the second clause outright — is recorded
      and rejected, on the ground your own finding gestures at: it would leave
      the requirement silent about the one property `Slate::from_nonce` is
      responsible for. A key-only clause tests the derivation function, pinned
      elsewhere; two candidates at one path is a defect *of this walk* that would
      reach a key-only assertion only by also breaking derivation. Two
      explanations, one answer — this repo's recorded defect family.
      I left `onboarding.rs:510-512`'s comment as it stands, since it is accurate
      about the address clause specifically being dropped without replacement.

- [x] **`dev-writer`** — `design.md:126–129` — **a spec citation that the spec
      does not support.** Decision 3 rejects the "rename the function" option
      partly on the ground that *"the `op-transport` spec still calls this the
      verification step."* **Measured:** `grep -in "verify_authored_op\|verification step"`
      over both `openspec/specs/op-transport/spec.md` and
      `openspec/changes/key-sweep/specs/op-transport/spec.md` returns **zero
      matches**. The delta names no function and no "verification step"; its
      scenarios are *An op whose signature does not verify is refused* and *An op
      whose key does not bind to its claimed author is refused*. The rejection is
      still right on its first ground (a rename would churn every call site for
      no gain), so this is a thin justification rather than a wrong decision —
      but the brief's standing rule is that a design entry citing a spec must
      cite one that says what is claimed, and a sibling piece has already shipped
      a fabricated citation here. Either drop the clause or replace it with what
      the spec does say.
      **Fixed, and reproduced first.**
      `grep -rn -i -E "verify_authored_op|verification step"` over both
      `openspec/specs/op-transport/spec.md` and this change's delta returns
      **zero matches**, confirming your measurement. The spec names no function
      and uses no such phrase.
      I took both of your options rather than either: the clause is **withdrawn**
      and the withdrawal is recorded in place, so the next reader sees that the
      ground was checked and failed rather than finding a silently shorter
      bullet. The entry now notes what the spec *does* say — that an op "whose
      signature does not verify under the public key that op carries" is refused
      — and is explicit that this supports the **name being accurate**, not a
      claim about the spec's vocabulary. Those are different claims and conflating
      them is how the citation got written.
      The rejection stands on the churn argument alone, which as you say was
      always the real one.

## Noted, not blocking

`design.md:206` and the `identity` delta at `specs/identity/spec.md:53` both say
"**three** pins remain". `the_wire_constants_are_pinned_to_known_answers`
actually carries **five** assertions — Stoa address, signing digest, per-Stoa key,
and the path-taking derivation at paths 1 and 0 (`identity.rs:870–915`). The
count is defensible if the two path pins are read as the same derivation at two
inputs, and the test's own comment uses the same "three where there were four"
framing, so the code and the record agree with each other. Recorded only so that
a later reader counting `assert_eq!`s does not think a pin went missing.

**Addressed, and it was less defensible than this paragraph allows.** The
readability reviewer filed the same numbers as a defect (entries 3 and 4 there),
and on checking both sides they are wrong under *either* reading: four
derivations remain, not three, because `derive_stoa_key_at_path` is a derivation
in its own right and was simply omitted — `design.md:210` said the test "pinned
four derivations" and listed only the author address, the Stoa address, the
signing digest and the per-Stoa key. So the "two path pins as one derivation"
charity rescues the pin count but not the derivation count.
All three sites now state the relation instead of a bare number: five pins over
four derivations here, six over five on `origin/main`. Your instinct to record
it rather than let a later reader recount was right — that reader would have
found a real discrepancy.
