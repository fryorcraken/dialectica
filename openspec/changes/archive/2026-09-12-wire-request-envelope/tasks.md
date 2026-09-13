## 1. Prove the defect before fixing it

The defect is latent, so the first job is a test that can see it. Every existing
hostile-input fixture is refused with or without the fix, so "an error came back"
is not a verification — the message is.

- [x] 1.1 Add `a_request_that_is_not_an_object_is_refused_for_its_shape`,
  asserting the reply's message equals `REQUEST_NOT_AN_OBJECT` rather than merely
  that an error came back. Verify by watching it FAIL on the unfixed handlers
  with `left: "missing field: payload"` — proving the array was refused by the
  field check and not by any envelope check.
- [x] 1.2 Add `the_three_refusals_a_caller_can_earn_are_three_different_messages`,
  pinning non-object / unparseable / missing-field apart. Verify it fails on the
  unfixed code for the same reason.
- [x] 1.3 Add `a_handler_whose_fields_are_all_optional_refuses_a_non_object`,
  building the all-optional handler today's surface lacks. Verify by mutating
  `Request::parse` to accept non-objects and watching `[]` be served as
  `{"hasMore":false,"items":[],"page":0}` — the defect made visible.

## 2. Reshape so the check cannot be forgotten (design.md §1)

- [x] 2.1 Add `const REQUEST_NOT_AN_OBJECT`, verified by
  `the_non_object_message_is_pinned_to_a_known_answer` asserting the literal.
- [x] 2.2 Add `Request(serde_json::Map<String, Value>)` with a private field,
  `parse(&str) -> Result<Request, String>` whose `Err` is already the wire shape,
  and `get(&str) -> Option<&Value>`. Verify with
  `request_parse_refuses_every_non_object_json_value`: `[]` and `not json` are
  both refused, with different messages, and `{}` is accepted. (Renamed from
  `request_parse_is_the_only_way_to_reach_a_field_read` — that name asserted an
  exclusivity the body does not check and that
  `the_sixth_method_the_boundary_does_not_stop` disproves.)
- [x] 2.3 Move `genesis_for` and `parse_index` from `&serde_json::Value` to
  `&Request`. Verify the crate compiles and the existing feed and genesis tests
  still pass unchanged.

## 3. Route every parsing handler through it

Verified collectively by the §1 tests, which sweep every method in
`every_request_taking_method()`.

- [x] 3.1 `ping`
- [x] 3.2 `get_capabilities`
- [x] 3.3 `list_threads_inner`
- [x] 3.4 `list_threads_from_request`
- [x] 3.5 `parse_channel_id`
- [x] 3.6 Verify no request-parsing `serde_json::from_str` remains outside
  `Request::parse`: grep the file and confirm the only non-test hit is inside
  `parse` itself.
- [x] 3.7 Leave `panic_probe` unchecked on purpose (design.md §4), verified by
  `panic_probe_still_panics_on_a_non_object_rather_than_refusing_it` and marked
  `NO SPEC` there.
- [x] 3.8 Re-export `Request` and `REQUEST_NOT_AN_OBJECT` from the crate root
  beside the handlers, so a new handler finds the constructor where it looks for
  the wire surface. Verify the crate builds.

## 4. Cover the spec's remaining scenarios

- [x] 4.1 Arrays and each scalar (`7`, `"s"`, `true`, `null`) refused on every
  request-taking method — the sweep in `a_request_that_is_not_an_object_…`.
- [x] 4.2 `{}` refused for its missing field and never for its shape —
  `an_empty_object_is_refused_for_its_missing_field_and_never_for_its_shape`.
- [x] 4.3 An object supplying only required fields is served —
  `an_object_supplying_only_its_required_fields_is_served`, which is what stops a
  check that refused everything from satisfying 4.1.
- [x] 4.4 An unrecognised field does not refuse the request —
  `an_unrecognised_field_does_not_refuse_the_request`.
- [x] 4.5 A refusal carries no result field —
  `a_non_object_refusal_carries_no_result_field`.

## 5. Close the envelope's structural and cost gaps

Raised by the design reviewer and the security reviewer after §1-4 landed. Each
is a hole the sweeps above could not see.

- [x] 5.1 **Move `Request` to `wire::request`**, a file with no handler in it. The
  private field was private to `wire.rs`, where every handler lives, so
  `Request(serde_json::Map::new())` compiled there and the type's documented
  guarantee was false for exactly the population it named. Verify the bypass now
  fails to compile (E0423) and that the same line compiles *inside* the new module,
  which is what makes the failure attributable to the boundary.
- [x] 5.2 **Add `MAX_REQUEST_BYTES`, checked before `from_str`.** A 64 MiB request
  was accepted and served. Verify from both sides of the boundary, and verify the
  ORDERING with a fixture that is oversized *and* unparseable — a "refused: yes"
  assertion cannot see a check that runs after the parse. `NO SPEC`.
- [x] 5.3 Bound `Address::from_hex` and `genesis_for` on length before
  `hex::decode`, which allocated half an attacker-chosen length first. Verify with
  over-long *and* invalid input, so only a length-first implementation can produce
  the asserted refusal. Keep `from_hex`'s error taxonomy unchanged for inputs at or
  under 64 characters.
- [x] 5.4 **Pin all three null readings at handler level**, one fixture per
  differing reader: `payload` (carried as a value), `stoa` / `genesis` /
  `channelId` (wrong type, never missing), `page` / `perPage` / `includeHidden`
  (absent → restrictive). Verify each against the mutation it names. Four of the
  seven readers observe the difference, so the earlier one-test pin was a coverage
  gap.
- [x] 5.5 Fix the double parse: `list_threads_inner` takes `&Request`, so the
  second parse is unspellable rather than merely removed.
- [x] 5.6 Correct `parse_index`'s refusal message, which said `1e2` was not a whole
  number. Acceptance deliberately unchanged (design.md — Non-Goals).
- [x] 5.7 Record in `design.md`: the size cap coupled to the unknown-field
  leniency; the null decision and why PLAN §9.1 does not bind it; duplicate keys
  accepted last-wins; serde's recursion bound being relied on; the rejected
  `include_str!` sweep test; and the UI-BRIEF judgement.

## 6. Gates

Counts are what the command reported at the time, and they move as the tester and
the parallel branches land. Run the command rather than trusting the number.

- [x] 6.1 `cargo test --manifest-path <abs>/dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core`. **The `-p` flags are load-bearing** — without
  them cargo tests almost nothing and still reports `ok`. 506 pass, 0 fail; 487 at
  the branch point.
- [x] 6.2 **`cargo fmt --check` cannot see `dialectica-core` and never could** —
  the workspace manifest has no `members`, so the gate exits 0 without reaching the
  crate holding all the logic. An earlier version of this line recorded that exit 0
  as a passed gate, which is true and vacuous. Use `rustfmt --check` per file
  instead: `wire.rs` carries **8** pre-existing hunks, the same count `main` has, and
  this change introduces none of its own. Do not reformat the eight — a diff that
  reformats a file it also edits cannot be reviewed for either.
- [x] 6.3 `cargo clippy -p dialectica-core --all-targets -- -D warnings` — clean
  (needed `NamedMethod` and `NullReadingCase` type aliases for `type_complexity`).
- [x] 6.4 `cargo build -p dialectica -p dialectica-core --all-targets` — clean.
  Anything behind `cfg(logos_scaffold)` is not built by `cargo test`, so the
  adapter in `dialectica/rust-lib/src/lib.rs` was checked separately: it references
  none of the signatures this change touched.
- [x] 6.5 `openspec validate wire-request-envelope --strict` — valid.
- [x] 6.6 Remove the SDK symlink before committing; verify `git status` shows no
  `logos-rust-sdk-src`. It is gitignored, so the risk is a stale absolute store
  path rather than a stray commit.
