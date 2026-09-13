## Stages

- [x] ~~spec — `spec-writer`~~ — **no delta.** Both contracts already decide
      what the code was getting wrong: `module-wire-contract` scopes the
      envelope rule to "every method that reads a field of its request" and
      names the panic probe as the *only* exception, and `content-authoring`'s
      "The signing identity is the one the probe reports" settles which key
      signs. `proposal.md` records the check rather than the conclusion,
      including why adding a requirement naming these handlers would make the
      contract weaker. Struck through, per the flow's rule for a piece with no
      spec delta, so the row is not an unticked one nobody is doing.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] CI green, PR merged — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`

## 1. The refactor, which must be green on its own

- [x] 1.1 Add `PublishRequest` carrying the prologue the three handlers shared —
      parse, `reject_forbidden_fields`, read `stoa` — with `publishing` holding
      the guard and the tail either side of one closure, and `refused` turning a
      `Refusal` into the wire shape in one place.
- [x] 1.2 Rewrite `publish_post`, `publish_reply` and `publish_vote` as that
      closure reading only their own fields.
- [x] 1.3 Verify **no test changed**: `cargo test --manifest-path
      dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` is green
      at 733 + 26 against exactly the assertions it made before, which is the
      proof this commit changes no behaviour. `wire.rs` is absent from
      `cargo fmt --check`'s output both before and after.

## 2. Make the failures visible, before fixing either

- [x] 2.1 Add the three handlers to `every_request_taking_method` with fixtures
      in `a_served_request`, and **watch three sweeps go red** on the unfixed
      code: `a_request_that_is_not_an_object_is_refused_for_its_shape`
      (`publish_post must refuse [] for its shape, got {"error":"missing field:
      stoa"}`), `the_three_refusals_a_caller_can_earn_are_three_different_
      messages`, and `every_request_taking_method_refuses_an_oversized_request`.
- [x] 2.2 Write `the_key_a_publish_signs_with_is_the_identity_the_probe_reports`
      and watch it fail against `keystore.stoa_key(&stoa)`, naming two concrete
      and different addresses rather than merely "not equal".
- [x] 2.3 Prove the CI exemption was load-bearing rather than vestigial by
      running the gate's own logic, minus the exemption, against
      `origin/main`'s adapter: it fails, naming `stoa_key`.

## 3. The envelope fix

- [x] 3.1 Swap `PublishRequest::parse`'s bare `serde_json::from_str` for
      `Request::parse`, and move `reject_forbidden_fields`, `required_string`,
      `required_op_id` and `required_direction` to `&Request`.
- [x] 3.2 Delete `required_stoa`, whose three answers `parse_stoa` already
      gives through a `&Request` — two readers of one field being the shape that
      kept the publish path outside the envelope in the first place.
- [x] 3.3 Verify the three sweeps from 2.1 now pass over all fourteen methods.

## 4. The signing key

- [x] 4.1 Add `core::wire::publishing_key`, the same derivation
      `posting_identity` reports, refusing a Stoa with no recorded choice with
      `NO_CHOICE_FOR_THIS_STOA` rather than falling back to any key.
- [x] 4.2 Point the adapter's `publishing` at it, and verify the test from 2.2
      passes — including its assertion that the *pathless* key disagrees with
      the probe, so the test can tell the fix from the defect.
- [x] 4.3 Add `a_publish_is_refused_when_no_identity_has_been_chosen_for_the_
      stoa`, so the fix cannot be satisfied by signing with something.

## 5. The sweep list, which was itself the defect

- [x] 5.1 Add `the_sweep_covers_every_request_taking_method_the_dispatch_trait_
      declares`: `include_str!` the adapter, read the request-taking methods out
      of the **trait declaration** by signature, and assert the sweep covers
      every one bar the panic probe.
- [x] 5.2 Prove it fails for the reason it names by removing `publish_vote` from
      the list and watching it report exactly
      `["publish_vote"]`, then restore.

## 6. The gates, and what each cannot see

- [x] 6.1 `cargo test -p dialectica -p dialectica-core`: 736 + 26, green.
- [x] 6.2 `cargo clippy -p dialectica-core --all-targets -- -D warnings`: clean.
- [x] 6.3 `cargo fmt --check`: `wire.rs` clean. The ten other files it reports
      in this crate are the pre-existing set CI cannot reach at all — the gate
      does not follow path dependencies — and are not this change's to fix.
- [x] 6.4 Run the adapter-derivation gate's own Python locally, since CI is the
      only thing that checks the adapter: it passes without the exemption.
- [x] 6.5 Run the test-count gate's logic locally: 762 declared, 762 ran.
- [ ] 6.6 **Build LGX is the only gate that compiles the adapter.** This change
      edits `cfg(logos_scaffold)` code, so nothing run locally checks it —
      `cargo test`, clippy and fmt all stop at `core.rs`. A duplicate method in
      two `cfg(logos_scaffold)` blocks passed all three on this repo and was
      caught only by Build LGX, seven minutes in. Unticked deliberately: it is
      the `closer`'s CI row to observe, not something this agent can claim.
