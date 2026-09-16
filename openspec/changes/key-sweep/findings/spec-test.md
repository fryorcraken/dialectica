# Findings — spec-test review of `key-sweep`

Read: the six delta capabilities under `openspec/changes/key-sweep/specs/`, the
live `openspec/specs/` tree, and the tests. The implementation bodies were not
read except at the three lines mutated below.

Mutations run: 3 (two reported here, one baseline). Every one restored;
`git status --porcelain` clean before commit.

## Findings

- [ ] **`tester`** — `openspec/changes/key-sweep/tasks.md:16-19`, the stage-block
      accounting — the test-list shape is described as "2 deletions, 2 additions"
      when it is actually a **2-for-1 collapse plus 1 genuinely new test**.
      **Scenario:** a reader auditing the deletions looks for two deleted tests
      each replaced by nothing, and two additions each covering new ground. What
      happened is that `an_author_address_is_not_a_bare_hash_of_the_key` and
      `an_author_address_and_a_stoa_address_never_collide` were *merged into one*
      successor, `no_derivation_turns_a_public_key_into_an_address`, and
      separately `verification_takes_no_author_identifier_beside_the_key` is net
      new. The net count is identical, so no gate can see the difference.
      **Measured:** name-by-name diff of `origin/main` (`c45cfa3`) against
      `piece/key-sweep` (`a5f83ba`) over `dialectica/rust-lib`: 13 removed, 13
      added, 11 one-for-one renames, then two removed against one added in the
      crypto block and one added in the verification block. Suite confirmed at
      987 green (957 lib + 30 integration).
      **Severity: low — accounting precision, not a coverage gap.** Both collapsed
      properties genuinely are properties *of the deleted derivation*, so the
      tester's substantive claim holds; only the shape is mis-stated. Worth
      correcting because the next reader reconciling the count will otherwise
      re-derive this discrepancy from scratch.

## Verified clean, in prose

Everything the brief asked me to check specifically held up, and several items
are stronger than claimed.

**The `STOA_ADDRESS_PREFIX` mutation is a genuine witness, and more so than
reported.** Repointing the constant to the retired author prefix
(`b"/dialectica/1/Address/Author\0\0\0\0"`) makes
`no_derivation_turns_a_public_key_into_an_address` fail at `identity.rs:968`
with `assertion left != right failed` and **both sides printing the identical
pinned hex** `f875158a79d255a6dd83307d5918298cd819eafb8218ef14cd34ebf2bc385ef4`.
That is the proof the pinned string really is the deleted derivation's own
output over the `0x01 || key` preimage, rather than 32 arbitrary bytes no
derivation could reach. A reintroduced author address under the old prefix would
reproduce it and fail here. This is a measurement, not a judgement.

**The `who_am_i` replacement derives its expectation independently.** The old
self-consistency check (does the reported key derive the reported address, when
the implementation computed that address from the very key it reported) is gone.
`wire.rs:4425-4434` recomputes the expected key from the fixed master key via
`derive_stoa_key_at_path(&[7u8; 32], &a_stoa(), path)` — a second, independent
route to the value — and separately asserts the field parses as a public key so
an empty string cannot satisfy a presence check.

**The two `// NO SPEC:` markers are genuine**, confirmed against the live spec
tree rather than the delta. `feed.rs:657` — there is no `feed` capability in
`openspec/specs/` at all, and `generated-names:78` only *forbids* a display name
on a feed row while stating nothing about what a row does carry. `wire.rs:6696`
— the live `thread-read` spec names the author *value* throughout but never
specifies a JSON field spelling. Neither marker sits on behaviour a spec
mandates.

**`verification_takes_no_author_identifier_beside_the_key` is honestly named.**
Its comment (`identity.rs:1312-1334`) states plainly which half each mechanism
covers: clause one structurally, via a typed `fn(&[u8], &[u8], &[u8]) -> bool`
binding that fails to *compile* on a reintroduced parameter; clause two only in
its `supplied` half, with the explicit admission that "arity cannot see what a
body derives internally" and a pointer to what closes the `derived` half. The
rename from `..._consults_the_key_..._and_nothing_beside_it` is the right call —
it identifies the overclaiming name as the defect rather than papering over it.

**The 11 renames repointed assertions rather than weakening them**, and two
strengthened. `an_op_whose_key_does_not_bind_to_its_claimed_author_is_refused`
(`transport.rs:1481-1489`) gained the control clause its spec scenario names —
that the forged signature is genuinely *valid* under the attacker's key —
without which a junk-signature fixture would be refused identically and the test
could not tell the forgery it names from any other bad signature. And
`a_public_key_alone_is_enough_and_no_other_value_reaches_its_name`
(`wire.rs:13200-13207`) repairs a *pre-existing* tautology: the old
address-based fixture drew a value that was not a valid curve point, so the
assertion degenerated to `None != Some(_)`, which no implementation could fail.
The substitute is a key, which always parses.

**Second mutation, independently chosen.** Repointing `FeedRow::author` from
`entry.op.op.author.to_hex()` to `entry.op.op.stoa.to_hex()` — a plausible
"wrong 64-hex value" regression — is caught by **four** tests
(`the_author_is_the_public_key_that_signed`,
`a_row_carries_the_public_key_and_no_derived_display_name`,
`a_name_in_a_posts_body_reaches_no_author_field`,
`two_keys_that_derive_one_name_stay_two_rows_with_two_keys`). No surviving
mutation. `two_posts_by_one_author_carry_one_key` correctly does *not* fail, since
it only asserts two rows by one author agree — a property a Stoa address also
satisfies — and that property is pinned by its siblings.

**Every delta requirement has a test.** Traced across all six capabilities:
`identity`'s four pinned derivations and the retired-pin scenario
(`identity.rs`), the verification requirement's four scenarios
(`identity.rs:1284-1356`); `op-format`'s author-naming and length-accounting
scenarios via `an_op_carries_no_ordering_fields` (`op.rs:1895-1923`), whose
byte-by-byte accounting is what makes a second author identifier unable to fit;
`op-transport`'s five distinguishable refusals (`transport.rs`); `thread-read`'s
four scenarios (`thread.rs:1660`, `wire.rs:6690`), with the closed key-set
assertion comparing the *whole sorted set* so a restored `authorKey` fails on an
added key; `identity-onboarding`'s closed field sets (`wire.rs:3705-3735`,
likewise whole-set equality) and the keep-position requirement; and
`view-identity-onboarding` across `tst_onboarding_states.qml`. No delta scenario
is untestable as written — the `identity` spec is notably careful here, stating
the rotation scenario as "how many identifiers an identity is reported by"
rather than over a counterfactual that could never be run.

The keep-position test `on_a_fresh_install_the_identity_kept_is_the_candidate_
the_slate_showed` (`wire.rs:5100`) deserves specific mention: it iterates **every**
position in the slate and supplies *no* keystore to either call, so the fixture
cannot hand both sides the same value. That defeats this repo's recorded defect
family at the fixture boundary, and its comment says so explicitly.

**The requirement moved between capabilities survived.** `identity`'s REMOVED
*An address is derived from a record, never from a bare key* carries a migration
note and a scoping note; the properties it held that still apply to Stoa
addresses are covered by the surviving *A Stoa address's display form parses
strictly* and the Stoa-address pin. Nothing moved verbatim-but-altered.

**The spec is self-consistent and not stale.** The removal explicitly resolves a
contradiction it names — `identity`'s old requirement stood against
`generated-names`' *A display name is derived from a public key and from nothing
else*. `docs/PLAN.md` on the branch sheds the behaviour correctly: §5.1's
record-hashing construction is struck through with a "Superseded by issue #80"
note pointing at the `identity` requirement, and every other mention is either
struck, a supersession note, or a Stoa-address scoping clarification. The fenced
code block at line 889 is not struck because Markdown cannot strike a fenced
block; it is sandwiched between struck prose with the supersession note directly
below, which is the best available shape.

**One note for the record, not a finding.** The worktree could not build until
`dialectica/logos-rust-sdk-src` — a gitignored vendored directory present only
in the main checkout — was symlinked in. Any reviewer or agent given a fresh
worktree hits `failed to load manifest for dependency logos-rust-sdk` on their
first `cargo test`. That is a worktree-provisioning matter rather than anything
about this piece.
