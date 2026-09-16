# Security review — `key-sweep`

Dimension: **security only**. Correctness, readability and architecture are other
instances' lanes and are not covered here.

## What was attacked, and what held

**The central claim was verified on the pre-change code and holds.** On
`origin/main`, `SignedOp::verify` (`op.rs:746`) passed `&self.op.author.address()`
as the claimed author and `&self.op.author.to_bytes()` as the presented key, so
`verify_authored_op`'s guard `key.address() != *author` compared a value against
itself. It could not refuse any input on the op-ingest path, hostile or malformed.
Nothing was lost by deleting it.

**Nothing real was lost on the paths where an address arrived from outside the
op.** Those were exactly three — the capability probe round-trip
(`wire.rs:3325`), the keystore keep round-trip (`wire.rs:4034`), and the
probe/whoAmI agreement test (`wire.rs:5491`) — plus `keystore.rs:3402` and
`onboarding.rs:671`. On main these compared a reply-reported `Address` against an
independently derived signing key. Each now passes the reply-reported **public
key** and a signature made by the independently derived key, so the binding is
checked by the signature rather than by an equality. That is at least as strong:
a signature made by key *S* verifies under reported key *R* only if *R* = *S*,
whereas the old `Address == Address` could in principle have been satisfied by a
hash collision. The replacement is a strengthening, not a like-for-like swap.

**`OP_SIGNING_PREFIX` is untouched** — `b"/dialectica/1/Signed/Op\0\0\0\0\0\0\0\0\0"`
at `identity.rs:77`, byte-identical to main (`identity.rs:61` there). Only the
doc comment below `signing_digest` changed. `signing_digest` still prefixes every
preimage, and `verify_op_bytes` still uses `verify_strict`, never `verify`.

**Stoa addresses are untouched.** `STOA_ADDRESS_PREFIX` is
`b"/dialectica/1/Address/Stoa\0\0\0\0\0\0"` at `identity.rs:69`, byte-identical
to main; `stoa_address` (`identity.rs:745`) is unchanged line for line;
`OP_ID_PREFIX`, `STOA_KEY_SALT` and `STOA_KEY_SALT_WITH_PATH` are all unchanged.
Every surviving `.address()` call in the crate resolves to `Genesis::address()`,
the Stoa derivation — checked exhaustively, 163 hits, no author-address
derivation among them.

**Moderation authorisation cannot have been weakened.** `moderation.rs`,
`membership.rs` and `log/` are **entirely unmodified by this piece**. `Moderators`
already held `creator: PublicKey` on main and `contains` already compared
`&self.creator == key`, so authorisation was key-based before this change and is
key-based after it. The CLAUDE.md standing rule is unaffected.

**The signature check is genuinely load-bearing across the crate.** Stubbing
`verify_op_bytes` to `true` fails **43 tests** (41 lib + 2 end-to-end), spanning
feed, thread, moderation, revision, transport, wire, keystore, onboarding and
end-to-end. With the address guard gone, the signature is the sole authentication
mechanism, and it is pinned in depth rather than in one place.

**The wire value is now more constrained, not less.** `PublicKey::from_bytes`
(`identity.rs:267`) refuses non-points and low-order points; `Address::from_bytes`
is infallible over any 32 bytes. `feed.rs` and `thread.rs` emit `author.to_hex()`
from an already-verified `PublicKey`, and `thread.rs` collapsed two fields to one,
removing the possibility of the pair disagreeing. No downstream consumer assumed
an address property: the UI reads `publicKey` throughout and validates it is a
non-empty string before rendering.

**The identicon byte allocation is intact.** `tst_identicon.qml` is untouched, so
the disjoint three-channel allocation (abbreviation 0..3/14..17/29..31, mark
4..11, name 18..23) is still measured exactly as PR #86 landed it. The
`Identicon.address` property keeping its name while receiving a key is a naming
matter for the readability lane, not a security defect — the component is generic
over both and still has a real Stoa-address caller.

**No dependency change.** `Cargo.toml` and `Cargo.lock` are untouched, so there is
no new dependency to assess for maintenance or licence compatibility.

## What a gate here cannot see

`dialectica/rust-lib/src/lib.rs` is `cfg(logos_scaffold)` and is compiled by
neither `cargo test`, clippy, fmt, nor `cargo mutants`. Its diff in this piece is
comments and doc-comments only — no logic — which I confirmed by reading the
whole diff, but **no local gate proves that**; only `nix build .#lgx` compiles it.
Recorded as a limit rather than papered over.

## Findings

- [ ] **`tester`** — `moderation.rs:652`, `thread.rs:736`, `revision.rs:435`,
      `end_to_end.rs:622`, `op.rs:1462`, `feed.rs:416`, `transport.rs:1106`,
      `transport.rs:2527`, `thread.rs:2243`, `end_to_end.rs:1304` — the
      forged-authorship fixtures carry no control proving the forged signature was
      genuinely valid, so they cannot distinguish an authorship forgery from a
      junk signature
      **Scenario:** Replace the genuine attacker signature in
      `a_forged_moderation` (`moderation.rs:664-667`) with fabricated bytes —
      `Signature::from_bytes(&[7u8; 64]).unwrap()` — leaving the author field
      naming the victim. Every one of these tests still passes, because
      `assert!(!forged.verify(), "the fixture must be an actual forgery")` is a
      *refusal* guard that junk satisfies identically. The tests therefore
      demonstrate "garbage signatures are refused", never "forged authorship is
      refused" — which is the property that matters now that the signature check
      is the **sole** authentication mechanism, the address guard having been
      deleted by this very piece.
      **Measured:** with `a_forged_moderation` fabricating its signature,
      **45 of 45 moderation tests pass**, and **987 of 987 tests in the whole
      crate pass** (957 lib + 30 end-to-end, 0 failed). Separately, with the
      crate's *reference* forgery test `op.rs:1445`
      `an_op_signed_by_someone_else_is_rejected` fabricating its signature,
      **65 of 65 `op::tests` pass**.
      **Why this piece owns it:** the control clause is a requirement of this
      change's own specs — `specs/identity/spec.md:155` and
      `specs/op-transport/spec.md:57` both state it, and
      `specs/identity/spec.md:164` gives the reason in its own words: *"its second
      clause is what keeps it honest — without it, a build that refused every op
      would pass."* This piece added that clause in exactly the two places its
      specs name (`identity.rs:1304` and `transport.rs:1481`) and did not carry
      the reasoning to the ~17 unspecified forgery tests that share the defect.
      Before this change the redundant address guard meant a forged op had two
      independent reasons to be refused; now it has one, so a test family that
      cannot see which mechanism refused is materially weaker than it was.
      **Severity: medium.** No live defect — the signature check is correct and
      the 43-test stub measurement proves it is exercised. This is a test-evidence
      gap: the family would not notice if the signature check stopped
      distinguishing a forgery from junk. Highest leverage is the four shared
      helpers, which cover 11 of the sites at once; note that
      `revision.rs:435` and `end_to_end.rs:622` carry **no guard at all** despite
      `revision.rs:434`'s doc comment claiming *"Asserted to be a genuine forgery
      at every use"*.

- [ ] **`tester`** — `wire.rs:5491` —
      `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` omits
      the validity control its two sibling tests both carry
      **Scenario:** The test's negative clause (`wire.rs:5568-5575`) asserts that
      a different path's signature does not verify under the probe's reported key,
      but nothing asserts that signature is valid under `elsewhere`'s own key. Its
      two siblings — `the_reported_identity_is_the_one_an_op_is_actually_signed_under`
      (control at `wire.rs:3374-3378`) and `a_kept_identity_can_sign_as_the_identity_it_reported`
      (control at `wire.rs:4072-4076`) — both added exactly that clause in this
      piece, each with the comment *"That signature is genuinely valid under its
      own key, so the refusal is a mismatch and not a malformed signature."*
      This one did not.
      **Measured:** the asymmetry is exact — two of the three probe/keystore
      round-trips this piece reworked gained the control, the third did not.
      **Severity: low.** The derivation makes a junk signature implausible here,
      so this is consistency of evidence rather than a reachable weakness. It is a
      separate box from the one above because it is a one-line addition to a test
      this piece already edited, not part of the crate-wide helper family.

## Not findings

`PublicKey::eq` (`identity.rs:308`) compares `to_bytes()` with `==`, which is not
constant-time. A public key is not secret material and this comparison decides
authorship, not secret equality, so it is correctly a plain comparison. Recorded
so the next reviewer does not re-derive it as a defect.
