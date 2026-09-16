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

- [ ] **`dev-writer`** — `design.md` Decisions — **the rotation affordance is
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

- [ ] **`dev-writer`** — `design.md` Decisions — **the slate distinctness clause
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

- [ ] **`dev-writer`** — `design.md:126–129` — **a spec citation that the spec
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

## Noted, not blocking

`design.md:206` and the `identity` delta at `specs/identity/spec.md:53` both say
"**three** pins remain". `the_wire_constants_are_pinned_to_known_answers`
actually carries **five** assertions — Stoa address, signing digest, per-Stoa key,
and the path-taking derivation at paths 1 and 0 (`identity.rs:870–915`). The
count is defensible if the two path pins are read as the same derivation at two
inputs, and the test's own comment uses the same "three where there were four"
framing, so the code and the record agree with each other. Recorded only so that
a later reader counting `assert_eq!`s does not think a pin went missing.
