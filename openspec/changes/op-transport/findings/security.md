# Security findings — `op-transport`

Reviewed at `b1af4e3` (`piece/op-transport` tip). Dimension: **security**.
Another instance holds readability and architecture; `correctness.md` beside this
file carries my correctness pass.

This is the network boundary, so I read it as attacker-controlled throughout:
every field of `InboundMessage` chosen by whoever sent it, on a channel with no
membership that any peer can compute the identity of.

**A note on reachability, stated up front because it changes how to read
everything below.** `transport::publish` and `transport::receive` have no
production caller: `grep -rn "transport"` over `dialectica/rust-lib/src/` returns
nothing, and `wire.rs` never mentions `transport::Refusal`. The capability is
built and unwired. I have **not** downgraded any finding for that reason — each
becomes live the moment the wiring change lands, and a finding parked as
unreachable is one nobody revisits. Where a finding's severity genuinely depends
on reachability I say so in the entry.

---

- [x] **`dev-writer`** — `authoring.rs:206` / `transport.rs:95` — the publish
      cap and the message cap are both 150 KiB, so a maximal legitimate post is
      **unreceivable** and the gap is silent in both directions
      **Scenario:** the same defect as `correctness.md`'s first entry, and I open a
      second box only because the security consequence is different from the
      correctness one and needs a different fix consideration. A body at exactly
      `authoring::MAX_BODY_LEN` encodes to **153,740 bytes**, which every receiving
      peer refuses as `TooLong { bytes: 153740, limit: 153600 }`. The security
      angle: an attacker who wants a post to exist for its author and for nobody
      else does not need to do anything clever — they need only persuade the author
      to write a long post. There is no error, no warning, and no observable
      difference from a post that propagated, so **the author cannot tell a
      censored post from a delivered one**. On a system whose whole claim is
      censorship resistance, a silent per-post delivery failure controllable by
      content length is the wrong failure mode to have.
      **Measured:** published a `MAX_BODY_LEN` body through the live
      `authoring::post` path, handed the stored op's `to_bytes()` to
      `transport::receive` on the matching open channel, got
      `TooLong { bytes: 153740, limit: 153600 }` — over by **140 bytes**, the fixed
      `Post` wire overhead.
      **Reachability:** the creating half (`authoring::post`) is live and reached
      from `wire.rs::publish_post`. The refusing half is `receive`, which is what a
      *peer's* build runs. So this is live as soon as there are two peers, and does
      not wait on the `transport` wiring.

      **Deferred.** Home: `design.md` § "The publish cap and the message limit leave
      a band of unreceivable ops, and closing it is a spec decision this change
      cannot take". Kept as its own box rather than closed with a pointer to
      `correctness.md` entry 1, because you opened it deliberately for a different
      fix consideration and that consideration turns out to be the one that decides
      the answer.

      Measurement reproduced to the byte — 153,740, over by 140 — by a new test,
      `a_body_at_the_authoring_cap_encodes_past_the_message_limit`. Detail of the
      reproduction and the can-it-fail run is on the correctness box; not repeated
      here.

      **Your framing is accepted in full, and it is why the deferral is shaped the
      way it is.** The security consequence is not "a large post fails" — it is that
      **the author cannot tell a censored post from a delivered one**, on a system
      whose claim is censorship resistance, with the discriminator being content
      length and the failure silent on both sides. An attacker needs no capability at
      all: persuade someone to write a long post.

      That rules out the cheap fixes on *security* grounds specifically, not only on
      the merged-spec grounds the correctness box argues. Both of the publish-side
      guards (lower `MAX_BODY_LEN`; refuse the encoded total) convert a silent
      delivery failure into a **loud publish refusal**, which is better — but they
      close it by making some legitimate posts unpublishable, and neither tells the
      author anything about the far larger class of posts that are publishable and
      still may not propagate. The honest fix is the one that makes the *state*
      visible rather than narrowing the input: publish succeeds, and the peer records
      and reports that this op exceeds the message limit and will not propagate.

      That is a delivery outcome, and it is precisely the thing this capability's spec
      already names as owed and unbuilt — the bound, what a peer records for an op in
      flight, and what it records for one that never propagated. **This band is a
      fourth instance of the same gap, and it is the instance where the peer knows the
      answer locally and with certainty**, before anything is sent: `to_bytes().len()
      > MAX_MESSAGE_BYTES` is decidable at publish time, where "did it propagate?" is
      not. Recorded in `design.md` so whoever builds the tracker inherits it as the
      cheapest case to get right rather than rediscovering it.

      Two things **not** claimed, since the standing rule is to say what a green gate
      cannot see. Nothing now refuses the op at publish, so the defect is live on
      `main`'s behaviour after this piece merges as much as before it. And
      `docs/UI-BRIEF.md`'s publish obligation — which forbids rendering a successful
      publish as *sent* or *delivered* — mitigates the *user-facing* half by refusing
      to claim delivery, but it is a designer-facing prohibition, not a mechanism, and
      it cannot distinguish this op from any other saved-but-unpropagated one. The
      distinction is exactly what is owed.

- [ ] **`spec-writer`** — `specs/op-transport/spec.md` (requirement "The delivery
      node is shared and is never stopped by this peer") — the scenario is
      **vacuously satisfied**, and one of its three clauses names a handler that
      does not exist
      **Scenario:** the requirement was rewritten in `b1af4e3` from three
      unobservable scenarios into one, "No stop call exists in this application's
      lifecycle handlers", whose clauses are: neither handler stops the node,
      neither creates one, **and the shutdown handler closes the channels this peer
      opened**. `dialectica/rust-lib/src/lib.rs` has no shutdown handler and no
      leave-Stoa handler — its methods are `version`, `ping`, `panic_probe`,
      `delivery_channel_exists`, `get_capabilities`, `list_threads`,
      `publish_post`, `publish_reply`, `publish_vote`, `on_context_ready`. Nothing
      anywhere outside `transport.rs` constructs an `OpenChannels` or calls
      `close_all`.
      So the first two clauses pass because there is nothing to inspect, and the
      third is **false**, not vacuous: no handler closes anything. The rewrite
      deliberately made this requirement "checkable by reading the lifecycle
      handlers" — but reading them shows there are none, which the requirement's
      own text does not admit.
      **Why this is a security finding rather than a scope note:** a channel left
      open holds filter subscriptions and remote peer slots on a shared node
      serving other modules, which the requirement's own justification says is the
      harm. A requirement that reads as discharged while the discharging code does
      not exist is exactly the shape that stops anyone writing it. Either the
      requirement should say the handlers are owed and not supplied — the way the
      publish requirement now honestly says three things are owed — or the third
      clause should come out until a handler exists to satisfy it.
      **Measured:** `grep -rn "close_all\|OpenChannels"` over
      `dialectica/rust-lib/src/lib.rs` and `dialectica-core/src/wire.rs`: zero
      matches.

---

## What I attacked and could not break

**`sender_id` and `timestamp` are genuinely inert — verified structurally, not on
the happy path.** My brief asked me to confirm *ignored* means no path reads them.
`grep` for both identifiers over the whole file returns, outside `#[cfg(test)]`
and doc comments, **only the two struct field declarations**. There is no read at
all, on any branch, so there is no path — happy or otherwise — that could reach
them. That is stronger than a test could establish, and it is the right way for
this property to hold.

The tests behind it are also well-aimed rather than decorative:
`the_sender_identifier_is_not_part_of_what_is_stored` asserts over the stored op's
**whole wire form** with a `windows()` scan for the sender id's bytes, rather than
field by field — a field-by-field check only looks at fields someone thought to
check. And `a_forged_sender_identifier_grants_nothing` uses the Stoa **creator's**
key as the forged sender id, which is the most privileged value the field could
claim; it is refused as `FailsVerification` and nothing is stored.

**Peer input reaching an index, a length, or an allocation.** There is none in
`receive`. No indexing, no slicing, no arithmetic — the size check is a single `>`
and the decode is delegated. In `op-format` below it:

- `Cursor::take` does `checked_add` then `slice::get`, so a `u32::MAX` length
  prefix cannot wrap into a small end offset that passes a naive bounds check.
  `cursor.rs`'s `a_length_that_would_overflow_is_refused_rather_than_wrapping`
  pins exactly that.
- `take_string_list` deliberately does **not** `Vec::with_capacity(count)`, with a
  comment saying why: the count is a hostile claim and reserving on it is the
  memory-exhaustion lever the cap exists to close.
- I tried the amplification attack that shape invites anyway: a **140-byte**
  payload claiming **150,000** attachments. Result: `Undecodable(Truncated)` in
  **37µs**. The loop dies on the first element's length prefix because the input is
  exhausted, so work is bounded by input length rather than by the claim.

**A check skipped by taking a different call path.** There is only one inbound
path. `receive` is the sole public entry point that appends from peer bytes, its
five guards are sequential statements with no early-exit branch that bypasses a
later one, and the single `log.append` is the last statement in the function — so a
refusal has no partial mutation to roll back. `publish` is the other append site
and takes a `SignedOp` the local peer already holds, never peer bytes. Nothing
constructs a `ChannelIdentity` except `ChannelIdentity::of`, and nothing inserts
into `OpenChannels` except `open`, which takes the identity rather than a
`(channel_id, stoa)` pair — so the map's invariant cannot be violated by a caller,
and the Stoa comparison cannot be made to compare against a lie.

**The size check is before the decode, and the bound is not a second number that
can drift.** Confirmed both. `transport.rs:450` precedes `transport.rs:459`, and
`the_size_check_runs_before_the_decode` makes the ordering *observable* with a
payload that is both over-long and undecodable — the only fixture that can tell the
two orderings apart. On drift: `MAX_MESSAGE_BYTES` is deliberately a second
constant from `op::MAX_FIELD_LEN`, and the docs at `transport.rs:83-94` argue
correctly that they bound different things and aliasing them would make a change to
either silently change the other. `the_message_limit_is_pinned_to_the_transports_stated_value`
pins it against both `153_600` and `150 * 1024`. The spec was also honestly
rewritten in `b1af4e3` to say that agreement with the *network's* limit is not
checkable here, because no limit reaches this capability from the transport —
which is true, and better than the previous text's claim that the two are compared.

**A peer's own published op is not received back.** What actually prevents a
double-store is not a filter at this boundary and is not the sender id: it is
`op-log`'s deduplication by op id, plus the transport's own behaviour of not firing
the receive event for a participant's own messages.
`an_op_the_peer_already_holds_arriving_again_is_one_op` publishes then receives the
same op and asserts `Appended::AlreadyPresent`, one entry, and — the load-bearing
part — that the **first** arrival's metadata is the one kept. So even if the event
did fire for own messages, the outcome is one op. The spec is explicit that the
sender id is deliberately not the filter, and the code matches.

**Constant-time comparison.** Nothing in this file compares secret material. The
Stoa comparison is over public addresses (a Stoa address is the channel name any
peer can compute), the channel lookup is a `HashMap` on a public string, and
signature verification is delegated to `ed25519-dalek`. No secret-dependent branch
exists here.

**Error messages leaking what a caller should not learn.** `Refusal`'s `Display`
discloses, at most: two Stoa addresses (`StoaMismatch`), a payload length and the
limit (`TooLong`), the decoder's own error (`Undecodable`), and the store's error
string (`Storage`). The first three are values the sending peer already chose or
already knows. `every_refusal_is_reported_distinguishably` additionally asserts no
variant renders with `::` in it, so a Rust type name cannot reach the
`{"error":"..."}` wire field. `transport::Refusal` does not currently reach
`wire.rs` at all. The one thing worth watching when the wiring lands is
`Refusal::Storage`, which carries the store's own message — a SQLite error can name
a file path — but that is the same exposure `authoring::Refusal::Storage` already
has on the live path and is not this capability's to decide.

**No reachable panic, and the doc comment claiming so is accurate.**
`transport.rs:431-438` claims "no indexing, no slicing and no arithmetic on this
path". I checked rather than took it: correct for `receive` itself, and the
delegated decode is total as described above. `arbitrary_bytes_are_refused_without_a_panic`
covers every prefix of a valid op plus every single-byte mutation of the first 96
bytes; `a_hostile_channel_or_sender_identifier_does_not_panic` crosses nine hostile
channel ids against the same nine sender ids (empty, NUL-bearing, 100,000
characters, 1,000 multi-byte glyphs) with `timestamp: i64::MIN`. I added no input
that panicked.

Worth recording as a limit of the measurement rather than a finding: `i64::MIN` as
a timestamp is only safe *because* the timestamp is never read. If a future change
reads it — even to log it — a cast or a duration computation on it is where an
overflow would appear, and `the_timestamp_handed_in_does_not_change_what_is_recorded`
already covers `i64::MIN` and `i64::MAX` so that change would have to confront it.
