# Correctness review — `key-sweep`

Dimension: **correctness only.** Five siblings hold security, readability,
architecture, spec-test and design.

## Verdict

**No correctness defects found.** Every claim I could reduce to a command was
run rather than read, and each held. The two findings below are both *recorded
observations for the closer*, not defects in the code — they are boxed because
each names something a person must confirm before merge, and an unticked box is
the only thing the merge gate can see.

## What was measured, and how

Everything below was executed in an isolated worktree
(`.claude/worktrees/review-ks-correctness`, branch `review/key-sweep/correctness`
off `piece/key-sweep` at `a5f83ba`), mutated freely, and restored. The tree was
verified clean by `git status --short` returning empty before this file was
written.

**The test-list accounting holds exactly as `tasks.md` states it.** Built
independently from `cargo test -- --list` on both `origin/main` and
`piece/key-sweep`: 987 identifiers each side, each side verified free of
duplicates so the set difference is exact rather than a multiset artefact.
**13 gone, 13 new**, matching the tester's count name-by-name. Eleven are
straight renames of the same assertion in the same module. The two that are not:

- `an_author_address_is_not_a_bare_hash_of_the_key` →
  `no_derivation_turns_a_public_key_into_an_address`. An inverted claim rather
  than a rename, and the inversion is the correct one: the old test guarded a
  derivation that existed, the new one asserts none does.
- `an_author_address_and_a_stoa_address_never_collide` has **no counterpart**,
  which is right — it was a property *of* the deleted derivation. I checked the
  surviving half is still covered: `op.rs:1352
  an_op_id_never_collides_with_a_stoa_address` holds the op-id/Stoa separation,
  and `STOA_ADDRESS_PREFIX` is still pinned at `identity.rs:871`. Nothing was
  lost behind the balanced count.

**The retired-pin claim is true, and I reproduced it rather than trusting it.**
`identity.rs:957` pins `f875158a79d255a6dd83307d5918298cd819eafb8218ef14cd34ebf2bc385ef4`
as the deleted derivation's own output. I reinstated
`SHA256(b"/dialectica/1/Address/Author\0\0\0\0" || 0x01 || key)` inline over
`SecretKey::from_bytes(&[7; 32])` and asserted equality: **it passes.** The pin
is the real retired value, so the test is a witness and not a tautology.

**`verify_authored_op`'s lost parameter changed no real check.** Confirmed by
reading every call site: the only production caller was `SignedOp::verify`,
which computed the claimed author as `.address()` on the op's *own* key — a
value against itself, unfailable for any input. The probe and keystore
round-trips that did pass two values now compare keys, and each gained an
explicit control clause ("that signature is genuinely valid under its own key")
without which a build refusing every op would pass. That is a strengthening.

**`Address` is Stoa-only in fact, not just in prose.** Swept every
`Address`-typed signature across `dialectica-core/src` and `rust-lib/src`.
Every surviving one is a Stoa address — bound to a parameter named `stoa`, or
produced by `identity::stoa_address` / `Genesis::address` /
`moderation::address_of(genesis)`. `transport.rs:358-359`'s `named` /
`channel_is_for` are the `StoaMismatch` pair, both Stoa. I did **not**
pattern-match on names: `keystore::stoa_address` and `stoa_address_at_path`
were the two that returned author addresses, and both are gone.

**No data migration is needed, verified rather than assumed.**
`log/sqlite.rs:763` stores `entry.op.op.author.to_bytes()` — the public key —
and always did. Nothing on disk is reinterpreted.

**Mutation testing.** `cargo mutants` on `identity.rs`: 45 mutants, 28 caught,
12 unviable, **5 missed** — all five are pre-existing `Display`/`Debug`/
`Signature::to_hex` formatting paths that this change does not touch.
Critically, `PublicKey::to_hex`, `Address::to_hex` and `verify_authored_op`
were all **caught**, and those are precisely the functions the sweep re-pointed
onto.

**Hand mutations of the two re-pointed reply fields.** Substituting a
*plausible wrong identifier of the right shape* — `stoa_address(key_bytes)`,
still 64 hex characters, which is the failure mode a length or parse check
cannot see:

- `thread.rs:634` mutated → caught by
  `every_item_carries_the_key_that_signed_and_no_second_identifier` **and** by
  `the_wire_reports_the_author_as_one_key_and_no_name`. Both layers bite.
- `feed.rs:273` mutated → caught by **6** tests including the end-to-end
  `a_request_naming_a_stoa_on_disk_comes_back_as_the_feed_in_json`.
- Reintroducing an `address` field into `Whoami::Identity`'s JSON → caught by
  both `the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
  and `who_am_i_reports_an_identity_by_its_public_key_and_no_address`.

**Gates run.** 987 Rust tests green. 286 QML assertions green with zero
failures and no undefined binding from `check_bindings`. **`nix build .#lgx`
green**, run from this worktree — so the `#[cfg(logos_scaffold)]` adapter is
independently confirmed to compile, not merely reported as compiling.

## What a green gate here cannot see

Stated because a passing gate is not the same as a covered claim:

- **`cargo test` and clippy compile nothing behind `#[cfg(logos_scaffold)]`.**
  The cfg is driven by `generated/provider_gen.rs`, which no local checkout
  stages — a local `cargo check` cannot reach that code at all. `nix build
  .#lgx` is the only gate that compiles it, and I ran it green.
- **`cargo fmt --check` does not follow path dependencies into
  `dialectica-core`**, so the nine unformatted files `tasks.md` names as
  pre-existing remain unseen by CI. This change does not worsen it.
- **`cargo mutants` mutates functions, not `const` values.** A changed
  `STOA_ADDRESS_PREFIX` is invisible to it; what covers that is the explicit
  pin at `identity.rs:871`, which I confirmed still exists.

## Findings

- [x] **`closer`** — `openspec/changes/key-sweep/tasks.md:124` — task 5.2's
      claim is now true, but only because the tester repaired it; the task text
      still reads as the `dev-writer`'s own account and credits two corrections
      where three were made elsewhere.
      **Scenario:** a reader auditing this change later reads 5.2 as a record of
      what the dev-writer measured, and reuses that trust for the sibling tasks
      in section 5 that nobody independently re-checked. The task list was
      already caught overstating once on this very piece.
      **Measured:** all three corrections ARE present in the tree today —
      `op.rs:1427` and `op.rs:1446` (the two test comments), `transport.rs:1445`
      and `revision.rs:45`. So there is **no code defect**; the residue is a
      provenance one. Severity: low, documentation-only. Confirm before archive
      that 5.2's wording is not carried into the archived change as though the
      dev-writer had verified it.
      **Fixed at the source, so the `closer` has nothing left to confirm.**
      Rather than leaving this as a pre-archive check, task 5.2's text is
      rewritten now: it no longer reads as the `dev-writer`'s own account. It
      states that four comments across three files were in scope, that the
      `dev-writer` corrected `op.rs`'s two, and that **the `tester` found and
      corrected `transport.rs:1441` and `revision.rs:42`** and measured the
      mechanism by stubbing `verify_op_bytes`.
      The readability reviewer filed the same line independently with the detail
      that the original named *neither* of the two it missed; both boxes are
      answered by the one rewrite.
      Your "no code defect" holds — I re-read all four sites in the tree before
      rewriting, and they are present. The wording is appended under a
      **"Corrected after review"** heading rather than silently replaced, so the
      overstatement and its repair are both in the archived record, which is the
      opposite of the failure you were guarding against.

- [x] **`spec-writer`** — `feed.rs:118-143`, `wire.rs:1819` — the feed reply's
      `author` field changes meaning with no requirement governing it, and the
      two `// NO SPEC:` markers are the only record.
      **Scenario:** a caller that stored a feed row's `author` before this
      change and compares it to one after gets no error — both are 64 hex
      characters — only a silent mismatch. Nothing in `openspec/specs/` can be
      cited to say which value is correct, so a future change could re-point it
      again with equal authority.
      **Measured:** `generated-names` forbids a name on a feed row; no
      capability states what a row *does* carry. The proposal names this gap
      explicitly and defers the feed contract as a capability's worth of work,
      which I agree is the right call for this piece — this box exists so the
      deferral is tracked rather than lost. Severity: low for this merge,
      medium if it stays open past the next feed change.
      **Deferred, to issue #91** —
      *Specify what a feed row carries: the reply has no governing capability*
      (https://github.com/fryorcraken/dialectica/issues/91), filed during this
      review pass.
      You wrote that the box exists so the deferral is tracked rather than lost,
      and a ticked box in a directory that is **deleted at archive** would have
      lost it — so the tracking needed somewhere durable before this could be
      closed. The issue carries your framing: no `feed` capability exists,
      `generated-names` only *forbids* a name while saying nothing about what a
      row does carry, and the concrete risk is that a caller comparing a stored
      `author` across this change gets a silent mismatch rather than an error
      because both values are 64 hex characters. It also lists what the
      capability would have to settle, so the next person does not restart from
      the observation.
      `design.md` §5 now names #91 as the deferral's home, so the pointer
      survives in the archived change and not only in the tracker.
      Your judgement that deferring is right for this piece is unchanged and
      recorded as such.

## Areas reviewed and found clean

Stated in prose rather than as boxes, since none needs action.

The decoders and parsers were probed for the failure classes this repo cares
about and none regressed: `verify_authored_op` still refuses empty, 31-byte,
33-byte and 64-byte keys and empty, 63-byte, 65-byte and 32-byte signatures as
`false` rather than a panic, and the test covering that survived the parameter
drop intact. No new indexing, slicing, `unwrap` on peer-derived data, or
arithmetic was introduced — the change only *removes* a derivation and
re-points field assignments, and `PublicKey::to_hex` is total over a parsed
key.

The onboarding distinctness test lost its address clause, and that is correct:
an address was a pure function of the key, so the clause could not fail while
the key clause passed. Distinctness is still asserted on both path and key at
`onboarding.rs:516-517`.

`keystore::creator_key_in`'s collapse from a pair is sound. The second element
was `creator.address()` — a pure function of the first — so the "two
derivations cannot diverge" property is now carried by there being one value
rather than by a comparison. The doc comment says so, and
`the_creator_a_creation_names_is_the_identity_the_probe_reports` correctly
records that it is weaker as a *test* for exactly the reason the code is
stronger, with the `derive_stoa_key` assertion keeping a wrong-but-consistent
derivation from passing.

`posting_identity`'s re-point is one-for-one: `stoa_address_at_path(s, p)` was
`stoa_public_key_at_path(s, p).address()`, so dropping `.address()` preserves
both the path and the key it derives.

The QML side is consistent. `PostHeader.identityAddress` → `identityKey` is
renamed at its one call site in `FeedScreen.qml`, and `Identicon` /
`AddressLabel` correctly keep `address` because both remain generic over a Stoa
address, which survives. A repo-wide sweep for `authorKey`, `identityAddress`,
`stoa_address_at_path`, `identity_address`, `poster_address_in`,
`creator_and_poster_in` and `AUTHOR_ADDRESS_PREFIX` returns only prose
explaining the deletion — no live code references a deleted symbol.
