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
  `request_parse_is_the_only_way_to_reach_a_field_read`: `[]` and `not json` are
  both refused, with different messages, and `{}` is accepted.
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

## 5. Gates

- [x] 5.1 `cargo test -p dialectica -p dialectica-core` — 485 pass, 0 fail.
- [x] 5.2 `cargo fmt --check` (no `-p`) — exit 0.
- [x] 5.3 `cargo clippy -p dialectica-core --all-targets -- -D warnings` — clean
  (needed a `NamedMethod` type alias for `type_complexity`).
- [x] 5.4 `cargo build -p dialectica-core --all-targets` — clean.
- [x] 5.5 `openspec validate wire-request-envelope --strict` — valid.
- [x] 5.6 Remove the SDK symlink before committing; verify `git status` shows no
  `logos-rust-sdk-src`.
