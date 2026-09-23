# spec-test review — get-stoa

Scope: `openspec/changes/get-stoa/specs/{stoa-metadata,identity}/spec.md` against
the `mod tests` blocks in `dialectica/rust-lib/dialectica-core/src/stoa_metadata.rs`,
`wire.rs` and `identity.rs`. Read only the spec deltas and the test code, plus the
minimal implementation needed to understand fixtures and to scope one mutation, per
the role's read restriction.

## Part 1 — scenario coverage

Every scenario in `stoa-metadata`'s "Current metadata resolves by last-write-wins,
falling back to genesis" has a test in `stoa_metadata.rs`'s resolver suite,
correctly placed (the resolver is exercised directly over a `MemoryOpLog`/
`UnscopedLog`/`UnreadableLog`, not through the wire, so the layer that can actually
see counter comparisons and the scope check is the one running the assertion).
Every scenario in "A Stoa's metadata is answerable...", "The reply carries the
current metadata...", "getStoa refuses what it cannot answer..." and "Resolving...
never aborts" has a matching test in `wire.rs`, and the uppercase-address scenario
is additionally cross-checked against `identity`'s general requirement through
`joinStoa` as well as `getStoa`, with a comment naming the sibling test — coverage
was clearly deliberate, not incidental. `identity.rs`'s display-form tests
(`an_address_in_uppercase_or_mixed_case_parses_to_the_same_address`,
`an_over_long_hex_address_is_refused_before_it_is_decoded`, and the WrongLength/
NotHex boundary tests) cover the `identity` delta's scenarios at the right layer.

One scenario has no test at all: "Asking about a Stoa does not join it." This is
not an oversight — `tasks.md` 2.4 documents the choice and gives a real reason
(`get_stoa`'s signature takes no membership store, so it cannot record membership
by construction, and a test that created a store, called `get_stoa`, then listed it
would pass identically against a null/no-op implementation, so it would not be
pinning anything a mutation could break). I read `get_stoa`'s signature
(`pub fn get_stoa<L: OpLog>(request: &str, store: impl FnOnce() -> Result<L, ..>)`)
and confirm it is structurally incapable of touching a membership store. Flagging
this for the record rather than as a defect, since "satisfied by construction, not
by a test" is a judgement call the spec-writer or tester may want to affirm
explicitly (e.g. a one-line comment at the scenario, or in the requirement itself)
rather than leave discoverable only in `tasks.md`.

No scenario in either spec delta looked untestable as written.

## Part 2 — mutation testing

Read the full `stoa_metadata.rs` test module (regenerating no implementation
beyond what was needed to follow the fixtures) and the `get_stoa`-related sections
of `wire.rs`. Nothing showed the three known defect shapes (length-only
discriminators, hash-moved-so-it-must-be-fine, position-pinned-without-value) or
the "name promises more than the body checks" pattern. The `assert_ne!` sweep in
`every_field_is_present_and_no_founding_title_is_reported` looked initially like
it could be vacuous against non-string fields, but `serde_json::Value`'s
`PartialEq<str>` correctly types that comparison as false for non-string variants,
so the loop is a real (if blunt) check across every field, not a tautology.

One mutation run, in budget:

- **Target:** `binding_metadata`'s moderator-authority gate in
  `stoa_metadata.rs` — `if !moderators.authorises(entry) { return None; }` changed
  to `if false { return None; }`, i.e. every metadata op binds regardless of who
  signed it. Chosen because it is the security-critical property the whole
  requirement rests on ("nothing about whether the signer may rename the Stoa").
- **Command:** `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core stoa_metadata`
- **Result:** caught. 7 of 24 tests in the module failed:
  `a_metadata_op_by_anyone_but_the_creator_does_not_bind`,
  `a_metadata_op_forging_the_creators_authorship_does_not_bind`,
  `an_unauthorised_higher_counter_op_does_not_displace_a_binding_one`,
  `a_forged_higher_counter_op_does_not_displace_a_binding_one`,
  `a_stoa_whose_only_metadata_ops_fail_to_bind_resolves_exactly_as_one_with_none`,
  `resolution_does_not_depend_on_the_sequence_ops_arrived_in`, and
  `the_resolver_does_not_trust_the_read_to_have_scoped_the_ops`.
- Restored immediately after; re-ran the same command and confirmed 24/24 pass
  again, matching the pre-mutation baseline. Also re-ran the full `wire::tests`
  module (260 tests) and `identity::tests` (39 tests) afterward — all green. No
  mutation is left in the tree; `git status --short` is clean.

This was the only mutation run, per the one-or-two budget. No survived mutation
to report.

## Part 3 — NO SPEC markers

None in `stoa_metadata.rs` or `identity.rs`. `wire.rs` carries many `NO SPEC:`
markers but every one found by `grep -n "NO SPEC"` predates this change or belongs
to other capabilities (publish, slate, keep, etc.); none is inside the `get_stoa`
test section (roughly lines 15150–15720) or cites `getStoa`/`stoa-metadata`. No
unmarked spec gap was found in the reviewed test code either — every behaviour the
`get_stoa` tests pin traces to a requirement or scenario in the spec delta.

## Part 4 — requirements moved between capabilities

Not applicable to this change's `stoa-metadata`/`identity` deltas: nothing is
marked `REMOVED` there. (`stoa-membership`'s one-sentence correction and
`stoa-navigation-view`'s restatement are outside this agent's assigned file scope —
their test layer is QML, not the three Rust files this dispatch covers.)

## Part 5 — spec soundness

`openspec validate get-stoa --strict` passes. Read `gh issue view 98 --repo
fryorcraken/dialectica` fresh: the built `getStoa` reply shape
(`{stoa, title, description, policy, isGenesisFallback}`) matches the issue's
sketch exactly, and the resolver reuses the `moderation-resolution` machinery as
the issue asks. The proposal's departure from the issue's `getStoa({stoa})` request
shape (adding `genesis`) is explained and consistent with the rest of the module's
established pattern (`listThreads`/`readThread` already take a genesis record). No
staleness against the issue found. No internal self-contradiction found on a full
read of both spec delta files.

## Summary

No unticked findings. Coverage is thorough and at the correct layer throughout;
the one sampled mutation (removing the moderator-authority check) was caught
robustly by seven independent tests. The only item worth a human decision is the
untested-by-design "does not join" scenario, noted above for the spec-writer/tester
to affirm rather than as something requiring code or test changes.
