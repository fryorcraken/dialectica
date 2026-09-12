# Security findings — `wire-request-envelope`

**Reconstructed from the commit record** — see the note at the top of
`correctness.md` for why that is itself a finding about how this piece was run.

---

## S1. A request could cost the module unbounded work — **Fixed**

For: `dev-writer`

Measured, release build, against a valid feed request padded with one ignored
string field:

| padding | request bytes | reply bytes | wall time |
|---|---|---|---|
| 0 | 184 | 373 | 0.13 ms |
| 16 MiB | 16,777,400 | 373 | 13.3 ms |
| 64 MiB | 67,109,048 | 373 | 92.6 ms |

All accepted and **served**. Two things make this worse than a slow reply:

- **The amplification is inverted.** The reply carries no signal that the request
  cost anything, so nothing downstream can notice, rate-limit or log it. `ping`
  is sharper because it echoes `payload`: 32 MiB in produced a 33,554,443-byte
  reply.
- **The failure mode is an abort, not an error.** `docs/PHASE0-FINDINGS.md` §3
  measured what a panic in a dispatch handler does — the module process aborts,
  the caller waits out a 20-second timeout, and every later call reports
  `MODULE_NOT_LOADED`. An allocation failure in `from_str` is that.

**Fixed** in `66de130`. `MAX_REQUEST_BYTES = 4 MiB`, compared **before**
`from_str` — one comparison in the one place every request passes through, which
is the same argument the `Request` type rests on, one step further along.

The number is derived rather than round: `op.rs` records a measured
768,076-byte `Post` at §4.4's SDS-derived field bounds, and that as JSON with a
hex genesis beside it (hex doubles) lands near 1.6 MB. 4 MiB clears it and still
refuses three orders of magnitude below what was served.

**The cap and the unknown-field leniency are recorded as one decision**
(`design.md` §6), because ignoring unknown fields is exactly what made the
64 MiB padded request *valid* — every byte sat in a field no method reads.
Dropping the cap also costs the leniency.

Tested for **ordering**, not merely for refusing:
`an_oversized_request_is_refused_before_it_is_parsed` feeds in something both
oversized and unparseable and asserts which refusal comes back, so reversing the
two lines in `parse` turns it red. And
`a_request_at_the_cap_is_accepted_and_one_byte_over_is_refused` probes both sides
of the boundary, which is what catches a `>` written as `>=`.

**The number itself is deferred to the spec-writer** — the spec set bounds no
length at all, so this is an *absent* decision rather than a rejected one. See
F1 in `spec-test.md`.

## S2. Two narrower bounds allocated before checking — **Fixed**

For: `dev-writer`

Same shape, different layer, and kept separate for that reason:

- `Address::from_hex` refused a 64 MiB hex `stoa` only **after** decoding it into
  a 32 MiB `Vec`.
- `genesis_for` did the same with a hex genesis record.

**Fixed** in `66de130`, both now checking length first.

Two care-taken details worth keeping:

- `from_hex` is guarded **only for the over-long case**. The obvious
  `s.len() != 64` would reclassify `from_hex("nothex!!")` from `NotHex` to
  `WrongLength(4)`, and a security fix that quietly changes a caller-visible
  error is two changes in one diff.
- `genesis_for` is bounded by `stoa::MAX_CANONICAL_BYTES`, newly exported,
  because the largest record the format can hold is `stoa`'s knowledge and a
  number copied into `wire.rs` would drift from it silently.

Each fixture is over-long **and** malformed, so only an implementation checking
length before decoding can produce the refusal asserted.

The genesis-hex bound carries a `NO SPEC:` marker and is **deferred to the
spec-writer** alongside the request cap — see F1 in `spec-test.md`.

## S3. A null field read as absent, in the permissive direction — **Fixed (as a spec `SHALL NOT`)**

For: `spec-writer`, then `dev-writer`

The behaviour today is safe by accident. `page`, `perPage` and `includeHidden`
read an explicit `null` as absent and take their defaults — and those defaults
happen to be the restrictive ones. A contributor generalising from
`includeHidden` would write the unsafe version: a future `includeRemoved`,
`bypassPolicy` or moderator-view flag written to the same match arm would let a
caller reach the **permissive** branch by naming a field with no value. Refused
as a wrong type, that request gets nothing; read as absent-and-defaulted, it
gets the wider view.

**Fixed** as a spec `SHALL NOT` in `2d8cc3f`, and the load-bearing half is the
**direction**: a null may read as absent only where the resulting default is
restrictive. `parse_index`'s doc states it where the next author will copy the
match arm from, and `includeHidden` carries the local note.

Not a departure from PLAN §9.1's "Absent, not null-and-present": that governs
`decidedBy` in a *reply* and rests on §2.5's no-partial-success, which does not
transfer — an omitted request field is an unexercised option, not a
half-success. The spec says so rather than leaving a reader to wonder which won.

## S4. Duplicate keys are accepted, last-wins — **Deferred, recorded in `design.md` §8**

For: `dev-writer`

`{"page":1,"page":99}` is accepted and the last value wins, which is
`serde_json`'s behaviour.

Harmless **today** because no field has two readers. The shape it becomes if one
ever does is HTTP parameter pollution: two readers of one field disagreeing about
which value they got, with an authorisation check on one side of the
disagreement.

**Deferred**, and it landed somewhere durable: `design.md` §8 records it as a
decision, names refusing as the rejected alternative and the reason
(`serde_json` has no `deny_duplicate_keys`, so refusing means writing a
visitor), and states the trigger — build the visitor when a field acquires a
second reader.

## S5. Serde's recursion bound is load-bearing and undocumented — **Fixed (recorded)**

For: `dev-writer`

A request nested a million deep is refused with
`invalid JSON: recursion limit exceeded` — by `serde_json`'s own 128-frame limit
on `from_str`, not by any check in this crate. That is a property of the
dependency, and it would silently disappear under `disable_recursion_limit()` or
a `from_reader` variant.

**Fixed** as a recorded reliance, in `Request::parse`'s doc and `design.md` §9:
neither is to be introduced here without a depth check in its place. Recording
it is the fix — there is no code change, and a stack overflow from a deep
request would be the abort described in S1.
