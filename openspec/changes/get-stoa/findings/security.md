# Security review — `get-stoa`

Dimension covered: **security only** (correctness, readability and
architecture are other reviewers' rows).

## No findings

No unticked boxes below — the review found nothing that needs a fix. What was
checked, and why it came back clean:

**Authenticity, authority and scope are checked together, every time, and the
suite proves it.** `stoa_metadata::binding_metadata` calls
`Moderators::authorises`, which conjoins signature verification
(`entry.op.verify()`), moderator-set membership (`self.contains(&author)`,
derived only from the genesis record's creator) and Stoa scope
(`entry.op.op.stoa == self.stoa`) — the same three-part check
`moderation.rs` already used for hide/unhide, now `pub(crate)` and reused
rather than re-spelled. Nothing here is cached from append time; every read
re-derives the answer, which is what the spec's "whether they hold MUST be
decided each time a Stoa's metadata is resolved" requires and what
`the_resolver_does_not_trust_the_read_to_have_scoped_the_ops` exists to catch
if a future `iter_stoa` implementation forgets to narrow. I mutated
`authorises` to drop `entry.op.verify()` (replacing it with `true`) and reran
`cargo test -p dialectica-core stoa_metadata`: 4 of 24 tests in that module
failed immediately
(`a_metadata_op_forging_the_creators_authorship_does_not_bind`,
`a_forged_higher_counter_op_does_not_displace_a_binding_one`,
`a_stoa_whose_only_metadata_ops_fail_to_bind_resolves_exactly_as_one_with_none`,
`resolution_does_not_depend_on_the_sequence_ops_arrived_in`), confirming the
signature check is load-bearing for the suite, not merely present in the
source. Mutation reverted; tree is clean (`git status` shows nothing to
commit).

**`getStoa` cannot join a Stoa or leak membership state.** Neither
`wire::get_stoa` nor the adapter's `get_stoa` takes a membership store; the
resolver receives the log as `&L`, and `append` needs `&mut`, so there is no
path from this call to a mutation — satisfied by construction rather than by
a guard that a future edit could forget, matching design.md decision 9 and
the "MUST NOT change any state" requirement.

**Untrusted input is validated before any expensive or stateful work.**
`get_stoa` parses the envelope, decodes and verifies `stoa`/`genesis`
(`Address::from_hex`'s length check runs before the hex decode, so an
oversized `stoa` field can't drive an oversized allocation;
`genesis_for`/`Membership::verified` re-derives the address from the record
and rejects a mismatch, so a substituted genesis record cannot install a
forged moderator set), and only then opens the store — in that order.
`a_request_that_fails_verification_never_opens_the_store` pins the ordering
with an opener that records whether it ran, over four different malformed
requests, and none of them touch it.

**The whole handler is wrapped in the panic guard**, and it's exercised
rather than assumed: `a_panicking_store_is_the_error_shape_and_the_next_call_is_answered`
drives a store whose every method panics, asserts `get_stoa` still returns
the `{"error":...}` shape, and asserts the *next* call succeeds — proving the
guard doesn't leave the module in a broken state. A separate
`an_adversarial_log_is_answered_rather_than_aborting` test (plus
`stoa_metadata`'s own `an_adversarial_log_resolves_without_panicking`) feeds
the resolver forged ops, cross-Stoa ops, ops of every other kind, `u64::MAX`
and absent counters, and 150 KiB fields containing bidi overrides — all
resolved without a panic.

**`get_stoa` is wired into the envelope-level sweeps**, not bolted on beside
them: it's in `every_request_taking_method()` and `a_served_request()`, so
the non-object-request, oversized-request and served-request sweeps all
cover it automatically (confirmed by reading the sweep test bodies, not just
the fixture list).

**Case-insensitive address parsing introduces no collision risk.**
`Address::from_hex` delegates to `hex::decode`, which treats hex digits
case-insensitively and always yields the same 32 bytes regardless of input
case; `to_hex` always renders lowercase. Two differently-cased spellings of
one address can only ever decode to the one `Address` value, so the
"self-authenticating record" property (`Genesis::matches` re-derives the
address and compares) is unaffected by the case change. This is a new test
(`an_address_in_uppercase_or_mixed_case_parses_to_the_same_address`) on
pre-existing, unmodified `from_hex`/`to_hex` code.

**Areas checked and found out of scope / unmodified, so not re-reviewed
here:** `stoa.rs` (genesis decode/title-cap enforcement — unchanged by this
diff), `log/mod.rs` and `log/sqlite.rs`'s `iter_stoa` (pre-existing, used
elsewhere already), `moderation-resolution`'s "moderator set is the creator
alone" (pre-existing design, not touched), and the CI workflow (no diff).

No dependency changes in this diff (`git diff main --stat` touches only
`identity.rs`, `lib.rs`, `moderation.rs`, `stoa_metadata.rs` (new),
`wire.rs`, and the adapter's `src/lib.rs`).

`cargo mutants` on `stoa_metadata.rs` was attempted but abandoned: the
baseline build alone exceeded a 20s timeout in `cargo-mutants`'s isolated
tmp build (this workspace's from-scratch compile, including the SDK crate,
takes ~40s), so no mutant results were produced in reasonable time. The
targeted manual mutation above (dropping the signature check, the
single highest-value security mutation for this module) stands in its
place.
