# Correctness findings — `op-transport`

Reviewed at `b1af4e3` (`piece/op-transport` tip). Dimension: **correctness**.
Another instance holds readability and architecture; `security.md` beside this
file carries my security pass.

Baseline measured in my own worktree: **626 tests pass**, of which 56 are
`transport::tests`. `tasks.md` §6 still says 56 new tests and the brief said 59;
the number that matters for a mutation measurement is the 626.

`cargo mutants` on the changed file, run against the **core crate's own**
manifest (`dialectica-core/Cargo.toml --file dialectica-core/src/transport.rs`):
**32 mutants, 26 caught, 5 unviable, 1 missed**, in 7m. Against the workspace
manifest it finds 0 mutants and exits 0, so a clean result obtained that way
means nothing.

---

- [x] **`dev-writer`** — `authoring.rs:206` / `transport.rs:95` — a body at the
      accepted publish cap produces an op **no conforming peer can receive**
      **Scenario:** `authoring::MAX_BODY_LEN == op::MAX_FIELD_LEN == 150 KiB ==
      transport::MAX_MESSAGE_BYTES`. `authoring::post` accepts a body of exactly
      `MAX_BODY_LEN` — `a_maximal_body_publishes_rather_than_panicking`
      (`authoring.rs:1453`) asserts it must — and the resulting wire form is
      **153,740 bytes**, which `transport::receive` refuses as
      `TooLong { bytes: 153740, limit: 153600 }`. So the live publish path signs,
      stores and reports success for an op that every receiving peer drops at the
      first guard, with no error anywhere: the author sees their post, nobody else
      ever will.
      **Measured:** I published a `MAX_BODY_LEN` body through
      `authoring::post` and handed the stored op's `to_bytes()` to
      `transport::receive` on the matching Stoa's open channel. Result:
      `Undecodable`? No — `TooLong { bytes: 153740, limit: 153600 }`, over the
      limit by **140 bytes**, which is exactly the fixed wire overhead of a
      minimal `Post` (version + kind + 32-byte stoa + 32-byte author + two
      presence tags + two length prefixes + 64-byte signature).
      **Why this is a correctness defect and not a scope question:** the two caps
      are documented as deliberately separate numbers bounding different things
      (`transport.rs:83-94` argues at length that aliasing them would be wrong,
      and it is right), but nothing anywhere bounds **body + overhead** against
      the message limit. `op-format`'s own docs name the transport boundary as the
      home for the total bound, and that bound exists — it just is not consulted
      by the path that creates ops. The fix is a publish-side guard against
      `MAX_MESSAGE_BYTES` on the encoded op, not a change to either cap.
      **Note on reachability:** `transport::publish` has no production caller, but
      this defect does **not** depend on it. `authoring::post` is the live path
      and is reached from `wire.rs`'s `publish_post`; the refusing half is
      `receive`, which is what every *other* peer's build runs. The defect is live
      the moment two peers exist.

      **Deferred** — the defect is confirmed and recorded, but the fix you name
      cannot land here. Home: `design.md` § "The publish cap and the message limit
      leave a band of unreceivable ops, and closing it is a spec decision this
      change cannot take".

      **Your measurement reproduces exactly**, and I did not take it on trust. A new
      test, `a_body_at_the_authoring_cap_encodes_past_the_message_limit`
      (`transport.rs`), publishes a `MAX_BODY_LEN` body and hands the wire form to
      `receive`: **153,740 bytes, overhead 140, over the limit by 140**, refused as
      `TooLong { bytes: 153740, limit: 153600 }`. Every figure in your entry,
      including the 140-byte `Post` overhead, confirmed to the byte.

      **Why not the fix you prescribe.** "A publish-side guard against
      `MAX_MESSAGE_BYTES` on the encoded op" contradicts a **merged** requirement.
      `openspec/specs/content-authoring/spec.md:184-189` carries the scenario *"A
      body at the cap is published"* — a body of exactly `op-format`'s field cap
      SHALL publish — and `:205-209` requires the publish cap and the field cap be
      *one value*. A total-size guard refuses that body, failing a scenario already
      in `openspec/specs/` and pinned by
      `authoring.rs::a_maximal_body_publishes_rather_than_panicking`. So this is a
      contract decision across two capabilities, not an implementation one, and
      `dev-writer`'s contract says a decision about observable behaviour belongs in
      the spec rather than in my head.

      `design.md` lays out the three candidate answers (lower `MAX_BODY_LEN`; guard
      the encoded total; accept the band and report it) with what each costs, and
      notes that only the third contradicts no merged scenario — and that it is
      unbuildable here, because "this op will not propagate" **is** a delivery
      outcome, the thing this change already records as owed and unbuilt. That
      alignment is the substantive finding: your defect and the delivery-outcome gap
      are one gap seen from two sides.

      **A fourth option ruled out by measurement, which I had not predicted.**
      Raising `MAX_MESSAGE_BYTES` to `150 * 1024 + 1024` — done to prove the new test
      can fail — broke a third test: `an_op_at_the_limit_is_admitted` failed with
      `Undecodable(FieldTooLong(154484))`. That fixture reaches the message limit by
      padding a body, so a raised limit demands a body above `MAX_FIELD_LEN`, which
      the decoder then refuses. The two caps are coupled tightly enough that one's
      boundary fixture becomes unbuildable when the other moves. Recorded in
      `design.md`.

      **Can-it-fail, predicted versus observed.** Predicted: the new test fails at its
      `payload.len() > MAX_MESSAGE_BYTES` guard, and
      `the_message_limit_is_pinned_to_the_transports_stated_value` fails on the
      hardcoded 153,600. Both observed, with the expected message ("153740 bytes
      against a 154624 limit"). The third failure above was **not** predicted and is
      the more informative result. Limit restored; 742 tests pass.

      The test asserts `payload.len() == MAX_BODY_LEN + overhead` from measured values
      rather than against a literal `153_740`, so a drifted cap moves the assertion
      with it instead of leaving it passing off the boundary — the defect family
      `the_publish_body_cap_is_the_format_field_cap` exists to prevent.

- [ ] **`tester`** — `transport.rs:266` — `OpenChannels::is_empty` can be
      replaced by `true` and the whole suite still passes
      **Scenario:** `cargo mutants` reports `replace OpenChannels::is_empty ->
      bool with true` as **MISSED**. Both call sites
      (`transport.rs:2050` in `publishing_without_an_open_channel_fails_and_opens_nothing`,
      and `transport.rs:2397` in `shutdown_closes_every_open_channel`) assert the
      method returns **true**, so no test ever observes it returning false. The
      assertion at 2050 reads `assert!(channels.is_empty(), "publishing opened a
      channel")` and would pass unchanged if publishing *had* opened one.
      **Measured:** 626 of 626 tests pass under this mutation — 1 missed of 32
      mutants, the only one.
      **Severity: low, and this is a test gap rather than a defect.** The
      load-bearing assertion two lines down (`!channels.is_open(...)`, line 2052)
      does catch the real property, and that one is killed by its own mutants. What
      is missing is any test that asserts `is_empty()` is false after an `open`.

---

## What I tried to break and could not

**The five guards' ordering.** I established what each rejects and whether an
earlier guard can be satisfied cheaply to reach a later one's work. It cannot, and
the order is the one that costs an attacker the most:

1. The channel lookup is a `HashMap` hit on a borrowed `&str` — no allocation, no
   scan. An unknown channel costs one hash.
2. The size check is a `>` on `payload.len()`, before any decode. I confirmed the
   ordering is observable, not just textual: `the_size_check_runs_before_the_decode`
   uses a payload that is both over-long *and* undecodable and demands `TooLong`,
   which is the only fixture in the file that can tell the two orderings apart.
3. The decode is `op-format`'s, over the whole payload.
4. Verification.
5. The Stoa comparison.

The expensive work (signature verification, at guard 4) sits behind both the size
bound and a full decode, so reaching it costs the attacker a well-formed 150 KiB-or-under
op. There is no cheap path to it.

**The author's headline mutation reproduces exactly.** Replacing
`signed.op.stoa != channel_stoa` with a byte-0 comparison leaves
`an_op_naming_another_stoa_is_refused` — the test *named* for that comparison —
**passing**, because its two hash-derived addresses differ in byte 0. Only
`two_stoas_sharing_an_address_prefix_are_not_confused`, whose addresses are
*constructed* to share 31 bytes, fails. 625 of 626 tests pass under that mutation.
The mitigation is present, correct, and is the only thing standing between this
file and the prefix-comparison defect this project has shipped twice before.

**I swept the other tests for the same shape** — a fixture whose values differ for
a reason other than the one under test — and found none. The two other places it
could hide are both handled: `two_channels_sharing_an_id_prefix_are_not_confused`
constructs one-character-longer and one-character-shorter channel ids rather than
relying on two unrelated strings, and
`the_derivation_is_pinned_to_a_known_answer` uses a `sha256sum`-derived value
rather than reading the implementation's own output back.

**Two more of the author's claimed-caught mutations, spot-checked and confirmed:**
disabling the signature check (`if false && !signed.verify()`) fails 4 tests
(`a_forged_sender_identifier_grants_nothing`,
`an_op_whose_signature_does_not_verify_is_refused_distinguishably`,
`an_op_whose_key_does_not_bind_to_its_claimed_author_is_refused`,
`an_unauthentic_op_is_not_admitted_on_the_promise_of_a_later_check`); recording the
arrival timestamp as a Lamport value fails 6, including
`what_is_recorded_does_not_vary_with_receive_sequence`, which is the one that
catches it as a *per-peer* order rather than merely as a non-`None` value.

**Decoder hostility.** `Cursor` (`cursor.rs`) is genuinely total on this path:
`checked_add` before every bounds check, `slice::get` rather than indexing, a
failed `take` that does not advance the head, and its one `expect` provably
unreachable (`take` returned exactly `N`). Truncated input, trailing bytes, lying
length prefixes, wrong-length keys and invalid UTF-8 all land in a named `OpError`.
`SignedOp::from_bytes` splits the signature with `checked_sub(64)`, so a payload
shorter than 64 bytes is `Truncated` rather than a panic.

**The three tests the author flagged as weak.** My judgement on whether the
weakness matters:

- `identity_does_not_vary_with_local_state` — the spec has since been rewritten
  (`b1af4e3`'s spec delta) to split this into "does not vary with the peer's
  history" plus "takes the Stoa address and nothing else", which is the honest
  pair. The test witnesses the first; the second is held by the signature and
  pinned by the known-answer test. **The weakness no longer matters** because the
  contract no longer claims more than is checkable.
- `arbitrary_bytes_are_refused_without_a_panic` — asserting only that the call
  returns *is* the right assertion, because a panic aborts rather than failing.
  Its fixture set is the strong part: every prefix of a valid op, plus every
  single-byte mutation of the first 96 bytes. **Not a real weakness.**
- `a_send_failure_does_not_lose_the_op` — dropping the `Publishable` is a faithful
  model, because `publish` returns rather than sends and so genuinely has no path
  that could undo the append. The spec's new requirement "There is no route by
  which a handoff failure could unpublish the op" now says exactly that. **Not a
  real weakness**, and I could construct no input that violates it.

**The undelivered-op "cannot".** My brief told me to check the reasoning, since
"cannot be done" is this project's least reliable sentence. It has already been
corrected: commit `d65b362` rewrote `design.md` from *"cannot be discharged"* to
naming three things **owed** (the bound, what a peer records for an op in flight,
what it records for one that never propagated) and stating that this change does
not supply them. The spec carries a matching requirement. Nothing left to report.

**Store-failure paths.** `AppendFailsLog` (added in `89070d6`) is the first thing
in the suite that constructs `Refusal::Storage` and `PublishError::NotStored` from
`receive`/`publish` rather than by hand, and the three tests using it are
well-aimed: each puts a valid op through every earlier guard so the append is the
only thing that can refuse. `the_two_ways_a_publish_fails_disagree_about_whether_the_op_exists`
pins the distinction a caller actually acts on. This area is clean.

**Idempotence and totality.** `OpenChannels::open` is idempotent by construction
(one `HashMap::insert` keyed on a value the identity determines), `close` is total,
`close_all` sorts so a caller's close sequence is assertable. `ChannelIdentity::of`
is a pure function of one argument with no route to construct one otherwise. I
found no input violating any claimed property.
