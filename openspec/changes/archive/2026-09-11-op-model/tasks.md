# Tasks

**This is a record, not a plan.** The code landed before these documents
existed, in the two commits named in `proposal.md`. What follows is what was
actually done, in the order it was done — reconstructed from the commits and
from re-reading the code, not a checklist anyone worked through at the time.
Writing it the other way round would be inventing a history.

## 1. Make room (the refactor commit)

- [x] 1.1 Lift `stoa.rs`'s private bounds-checked cursor into
      `dialectica-core/src/cursor.rs` as a crate-internal `Cursor` with an
      `OutOfBounds` error saying only which of the two things went wrong.
- [x] 1.2 Give it `take`, `take_array`, `take_length` and `finish`, with
      `checked_add` on the offset so a hostile length prefix cannot wrap past a
      naive bounds check.
- [x] 1.3 Port `stoa.rs` onto it with no behaviour change, mapping `OutOfBounds`
      into `GenesisError` through a `From` so every bounds failure maps the same
      way. Verify the suite is green on this commit alone.

## 2. The op types

- [x] 2.1 `OpKind` with the four kinds the plan names (post, revise, moderate,
      vote), each discriminant explicit and commented as a wire value. Creating
      a Stoa is not among them — that is a genesis record, not a signed op.
- [x] 2.2 `ModerationAction` (hide, unhide) and `VoteDirection` (up, down) as
      fields rather than as further op kinds.
- [x] 2.3 `Op` carrying Stoa address, author public key and kind; `SignedOp`
      wrapping an `Op` with its signature rather than nesting the signature
      inside the signed value.
- [x] 2.4 `OpId` as a distinct 32-byte type with hex display and strict parsing,
      and `OpError` with one variant per malformation.

## 3. The encoding

- [x] 3.1 `canonical_bytes`, leading with version then **kind**, then Stoa,
      author, and the kind's own fields. Length prefixes on variable-length
      fields; presence tags on optional ids.
- [x] 3.2 Write `the_kind_byte_is_inside_the_signed_preimage` against the case
      that actually collides — a moderation and a vote sharing Stoa, author,
      target and a trailing discriminant byte, since `HIDE` and `UP` are both 0.
      Two ops of different lengths would not have tested it.
- [x] 3.3 Write `a_signature_over_one_kind_does_not_verify_as_another`, which
      asserts both that the forgery fails *and* that the original verifies, so
      it cannot pass because both are broken.

## 4. Strict decoding

- [x] 4.1 `Op::decode` on the shared cursor, refusing each malformation with its
      own error: unknown version, unknown kind, unknown moderation action,
      unknown vote direction, invalid presence tag, truncation, trailing bytes,
      a lying length prefix, an over-long field, invalid UTF-8, invalid author
      key.
- [x] 4.2 Check the field-length cap **before** allocating, in
      `take_checked_length`, and grow the attachment vector as elements arrive
      rather than reserving on the claimed count.
- [x] 4.3 Cover truncation at *every* prefix length rather than a few
      hand-picked ones, for every kind — a missing bounds check fails only at
      the boundary its field happens to straddle.
- [x] 4.4 `a_hostile_op_is_never_a_panic_for_any_input_shape` as the blanket
      property, since the specific cases cannot cover every shape an attacker
      may send.

## 5. Id and signing

- [x] 5.1 `Op::id` as a domain-separated hash under `OP_ID_PREFIX`, distinct
      from every address prefix, with tests that hold the preimage constant and
      vary only the prefix.
- [x] 5.2 Pin the id to an independently derived known answer, with a comment
      saying how to re-derive it and instructing a future reader not to update
      the expectation to match.
- [x] 5.3 `sign` / `verify` delegating to `identity.rs` rather than
      re-implementing the address re-derivation that binds key to claimed author.
- [x] 5.4 Pin what verification does *not* answer:
      `verification_answers_authenticity_and_not_authority` and
      `a_revision_by_a_different_author_is_authentic_and_still_not_valid`.

## 6. PLAN.md

- [x] 6.1 Strike through the three §13 questions the format answers — are votes
      an op, is a hide reversible, when the `policy` field lands — recording the
      answers rather than leaving closed questions open.
- [x] 6.2 Record in §13 that the ordering rule has no input at the delivery
      contract we have, and that this blocks the store rather than the format.

## 7. The documents (this change)

- [x] 7.1 Read the archived `stoa-genesis` change in full as the shape and
      quality bar, and the agents README for the document model.
- [x] 7.2 Mutation-verify each property the spec would claim, before claiming
      it: break it in the source, run the suite, record the failures, restore.
      Table in `design.md`. Worktree confirmed clean afterwards.
- [x] 7.3 Probe re-encoding canonicity exhaustively over single-byte mutations
      (12,938 accepted inputs, all re-encoding to themselves), then remove the
      probe — this change adds no behaviour, including test behaviour. Recorded
      as an open question instead.
- [x] 7.4 Audit the existing tests for the defect class this project keeps
      hitting: a test that cannot fail for the reason it names. Findings in the
      PR description and the final report.
- [x] 7.5 Evaluate extracting the shared encoding rules into a general
      capability, now that a second encoding exists. Declined; reasoning in
      `design.md` so a third encoding's author knows the question was asked.
- [x] 7.6 Write `proposal.md`, `specs/op-format/spec.md`, `design.md` and this
      file. Decide whether to archive.

## 8. Gates

- [x] 8.1 `cargo test --manifest-path <worktree>/dialectica/rust-lib/dialectica-core/Cargo.toml`
      — green. Note the workspace-root manifest builds only the outer crate,
      which has no tests of its own; `dialectica-core` is where `op.rs` lives.
- [x] 8.2 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
      — both clean.
