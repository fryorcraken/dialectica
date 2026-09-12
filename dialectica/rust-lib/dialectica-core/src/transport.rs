//! Moving ops between peers: where a channel is, what goes on it, and what a
//! peer is allowed to believe about what comes back.
//!
//! # Why this file exists
//!
//! Everything before this change made ops; nothing sent one. §4.1 names the
//! shape — "one reliability channel per Stoa" reached through
//! `channelCreate(channelId, contentTopic, senderId)` — and this module is the
//! half of that which can be tested: the three strings that call takes, the
//! bytes that go on the channel, and the decision made about bytes that arrive.
//!
//! The `modules().delivery_module` call itself is NOT here and cannot be. It
//! calls `lp_*` symbols undefined in a test binary (§2.3), so it lives in the
//! adapter, with this module on either side of it. `wire.rs` already
//! established that arrangement for `channelExists`; this is the same seam,
//! widened to carry ops.
//!
//! # What this module refuses to invent, and why each refusal is the design
//!
//! **No ordering.** `channelMessageReceived(channelId, senderId, payload,
//! timestamp)` carries no Lamport timestamp and no message id, and its
//! `timestamp` is the receiving peer's own `CLOCK_REALTIME` read (§4.4, §13). So
//! [`InboundOp::arrival`] is [`Arrival::unordered`] and nothing here reads the
//! clock. The temptation is precise and worth naming: the `timestamp` is already
//! in the event signature, it is an integer, and recording it would compile.
//! It would also make one message sort differently on every peer that received
//! it — [`crate::arrival`] calls that "recording arrival sequence while
//! believing we recorded a shared order", and it is worse than no order at all
//! because the degraded order is at least convergent.
//!
//! **No dialectica-side Lamport clock.** §13 withdrew the argument that one
//! could never work, so this is not "impossible" — it is *not this change*.
//! Building one here would mean every op minted from now on carries a counter
//! value, which is a wire-format decision, and making it as a side effect of
//! wiring a socket is exactly the accidental widening §2.5 asks us not to do.
//!
//! **No authority.** [`accept_inbound`] answers authenticity and structure and
//! stops. Whether the signer is a moderator (§6), whether a revision's author
//! owns its target (§5.7), whether the Stoa's policy admits them (§7.1) — each
//! needs state a boundary does not have, and §3.3 puts all three on read. The
//! moderation and revision resolvers already do this work; duplicating a
//! weaker copy of it here is how the two come to disagree.
//!
//! **`senderId` reaches no decision.** §4.1: "`senderId` is not an author
//! identity, and the plan should not treat it as one." It is an
//! application-chosen transport string, so an attacker sets it to whatever they
//! like. [`accept_inbound`] does not take it as a parameter at all — the
//! strongest available statement that it cannot leak into a judgement, since a
//! value that is not in scope cannot be consulted.
//!
//! # Which tests were watched failing, and the two that a mutation did NOT kill
//!
//! Every test here was proven able to fail by mutating the implementation and
//! watching it go red. The table records which mutation each test caught,
//! because "it can fail" is worth less than "it fails for the reason its name
//! claims":
//!
//! | Mutation | Killed by |
//! |---|---|
//! | Record the payload length as a Lamport value | `an_accepted_op_is_recorded_as_unordered`, `an_ingested_op_is_recorded_as_unordered_in_the_store` |
//! | Compare only the first 2 bytes of the Stoa address | `the_stoa_check_compares_the_whole_address_and_not_a_prefix` — **and nothing else** |
//! | `if !op.verify()` → `if false` | 9 tests, including every named forgery shape and both seam tests |
//! | Run the size check after the decode | `a_payload_over_the_sds_cap_is_refused_before_it_is_decoded`, `the_size_check_runs_before_the_decode_and_not_after` |
//! | Drop the newest instead of the oldest when full | `a_full_queue_drops_the_oldest_and_keeps_accepting_new_arrivals` — **and nothing else** |
//! | Append a deterministic `/e0` epoch to the channel id | `the_channel_id_survives_a_close_and_reopen_with_no_epoch`, `the_channel_id_and_the_content_topic_are_pinned_to_the_plans_strings` |
//!
//! **Two "and nothing else" rows are the point of the table.** The 2-byte prefix
//! match survived `a_genuine_op_replayed_onto_another_stoas_channel_is_refused`,
//! which uses two hash-derived addresses that differ in byte 0 — the exact defect
//! family this repo has recorded twice, where a fixture cannot tell two
//! explanations apart. Only the test built from *constructed* addresses sharing
//! 31 bytes catches it. Likewise the drop-newest mutation passed both the bound
//! test and the ordering test, because both are satisfied by either policy.
//!
//! **And one mutation was deliberately NOT claimed as a kill.**
//! `the_boundary_takes_no_timestamp_and_so_records_the_same_arrival_always`
//! stayed green under the Lamport mutation above, and correctly so: a
//! deterministic function of the payload agrees between peers, which is the
//! property that test asserts. It kills a genuine *local clock* read — the
//! mutation that matters — and saying so is more useful than claiming a kill it
//! does not have.

use crate::arrival::Arrival;
use crate::identity::Address;
use crate::op::{Op, OpError, SignedOp};

/// The SDS message cap, and the only total-size bound an op ever gets.
///
/// §4.4: **150 KiB**, "a hard cap — a network-wide gossipsub validation limit,
/// not unilaterally raisable".
///
/// # This is the check `op.rs` said belonged here
///
/// [`crate::op`]'s `MAX_FIELD_LEN` caps each *field* at this same number and is
/// explicit that the cap does not compose — a metadata op with both fields at
/// the cap decodes at 307,274 bytes, and the attachment path reaches 768,076.
/// Its doc comment says where the missing bound goes:
///
/// > **the total-size check belongs at the transport boundary, where the SDS
/// > frame is known.** This module is handed a byte slice and cannot see the
/// > frame it arrived in, so a combined bound here would be a guess at a number
/// > the caller already has. Put it where the frame is; do not add it here.
///
/// This is where the frame is. [`accept_inbound`] checks the whole payload
/// against this before decoding, so the composition hole closes at the one place
/// that can see a whole message.
///
/// Applied to the **payload** — the op's canonical bytes plus its 64-byte
/// signature — and not to the decoded op, because the payload is what SDS
/// carried and therefore what SDS's cap governs.
pub const MAX_PAYLOAD_BYTES: usize = 150 * 1024;

/// The content topic for a Stoa's channel.
///
/// §4.1, verbatim: `contentTopic` = the Stoa, hashed and bucketed:
/// `/dialectica/1/s/<hex>/proto`.
///
/// # The Stoa address is already the hash, so nothing is hashed again here
///
/// §4.1's "hashed and bucketed" describes what the value *is*, not a second
/// operation to perform: an [`Address`] is the hash of the genesis record
/// (§4.8), so interpolating it satisfies §4.1's rule that a human-readable name
/// must never appear in a topic. Hashing it a second time would satisfy that
/// rule too and would be a different string for no gain — and every peer would
/// have to agree about which.
///
/// # Why this is one topic per Stoa and not per thread
///
/// §4.2 is emphatic and counterintuitive: autosharding hashes only
/// `application` + `version`, so every `/dialectica/1/*` topic lands on one
/// shard and a Core node subscribes to all shards regardless. Topics are "a
/// local filter, not routing"; minting more "buys nothing and costs real
/// money", with LIP-23 measuring Store response time doubling from 10 to 100
/// content topics. So the temptation to put a thread id here is not merely
/// deferred — it is measured as actively wrong.
pub fn content_topic(stoa: &Address) -> String {
    format!("/dialectica/1/s/{}/proto", stoa.to_hex())
}

/// The channel id for a Stoa's channel: the rendezvous every peer must compute
/// alike.
///
/// §4.1: `channelId` = the Stoa, and **the same value for every peer in it**.
///
/// # A pure function of the Stoa address, and nothing else is in scope
///
/// §4.3 states the consequence of getting this wrong, and it is the sharpest
/// failure mode in the plan:
///
/// > **So the channel id can carry no per-peer state.** Not a session counter,
/// > not a local sequence number, not anything that varies with one peer's
/// > history. A value that differs between peers does not produce an error: it
/// > produces two Stoas that cannot see each other, silently and permanently.
///
/// This function takes one argument and reads nothing else — no clock, no
/// counter, no configuration, no `self`. That is the whole defence, and it is
/// structural rather than asserted: there is no local input available to leak,
/// so a future edit that wanted to mix one in would have to change the
/// signature, which is a visible act.
///
/// # No epoch, deterministic or otherwise
///
/// §4.3 rules out both forms, and the second reason is the durable one. A
/// per-peer epoch partitions the Stoa the moment two peers disagree about it. A
/// *deterministic* epoch is ruled out because it would make every boundary a
/// rendezvous problem — and because it existed only to route around
/// `logos-delivery#4116`, a crash on channel re-creation that "was never
/// deterministically reproduced". §4.3 withdrew the workaround and the
/// restriction it bought ("rejoining a Stoa required a restart") along with it:
/// **"If re-creating a channel kills the node, that is an upstream bug and it
/// gets fixed upstream. Do not reintroduce an epoch here as the remedy."**
///
/// So closing and reopening this same id is supported, and re-deriving it after
/// a rejoin yields the same string by construction — there is no state to
/// advance.
///
/// # Why it is prefixed rather than being the bare hex
///
/// The channel id shares a namespace with every other application using the
/// shared delivery node, and the bare 64-hex form claims nothing about who
/// minted it. `/dialectica/1/s/<hex>` carries the same version discriminant the
/// content topic does, so a future §4.5 per-thread split — `(stoa, thread)`, per
/// §4.5's "derive `channelId` as a pure function of the addressed object" — adds
/// a segment rather than reinterpreting an opaque string.
pub fn channel_id(stoa: &Address) -> String {
    format!("/dialectica/1/s/{}", stoa.to_hex())
}

/// This peer's `senderId` for a Stoa's channel.
///
/// §4.1: `senderId` = **one per user per Stoa, permanent** — every participant's
/// is different (the API requires it); what is stable is that a given user keeps
/// theirs across sessions.
///
/// # The three requirements, and why the per-Stoa identity satisfies all of them
///
/// §4.3 calls this "the tell": `senderId` and `channelId` are adjacent
/// parameters with **opposite** requirements, one to differ between peers and one
/// to be agreed on. The requirements on this one are:
///
/// 1. **Different for every participant.** SDS states it as a MUST — a
///    Participant ID is "globally unique, immutable" and the sender "MUST include
///    its own globally unique identifier". Satisfied because this is the author's
///    per-Stoa *address*, a hash over their root secret (§5.2), so two users
///    collide only if they share a root key.
/// 2. **Stable across sessions.** The Reliable Channel API asks that it "SHOULD
///    be unique and persisted between sessions", and §4.1 assigns that duty to
///    dialectica: "nothing in the delivery module persists it". Satisfied without
///    any persistence machinery, because it is *derived* rather than stored — the
///    keystore's root plus the Stoa address yields the same value every time. A
///    stored `senderId` would need a migration path and could go missing; a
///    derived one cannot.
/// 3. **Stable for as long as the channel is held**, because §4.1 notes it binds
///    at channel creation. Satisfied a fortiori: §5.2's identity is permanent, so
///    there is nothing to rotate to.
///
/// # This is NOT an author identity, and the type does not let it become one
///
/// §4.1: "`senderId` is not an author identity, and the plan should not treat it
/// as one." It returns a `String`, not an [`Address`] — deliberately, so that a
/// value produced here cannot be passed to anything expecting an author address
/// without an explicit re-parse. Authorship comes from the op's signature and
/// nothing else, and [`accept_inbound`] does not take a `senderId` at all.
///
/// What it *does* disclose is real and is the intended scope: §4.1 records that
/// every receiving peer is handed the sender id with every message, so this links
/// a Stoa's posts to one pseudonym — "and no further", which is exactly what
/// §5.2's per-Stoa identity is for.
///
/// # Taken as an address rather than looked up
///
/// The argument is the author's per-Stoa address, which the caller gets from
/// `Keystore::stoa_address`. This crate cannot read a keystore path or the
/// environment (§2.3), and CLAUDE.md's "pass what it needs; do not have it reach
/// for ambient state" applies — a function that went looking for a keystore would
/// be doing discovery at a moment its caller does not control.
pub fn sender_id(author_for_stoa: &Address) -> String {
    // Prefixed rather than bare hex, for the same reason the channel id is: the
    // value shares a namespace with every other application on the shared
    // delivery node, and a bare 64-hex string claims nothing about what minted
    // it. The `a` segment ("author") distinguishes it from the `s` segment the
    // channel id and content topic use, so the three are never confusable by
    // eye in a log — which matters because mixing up `channelId` and `senderId`
    // is the failure §4.3 spends a section on.
    format!("/dialectica/1/a/{}", author_for_stoa.to_hex())
}

/// What goes on the channel for an op this peer has stored.
///
/// Exactly [`SignedOp::to_bytes`] — the canonical op bytes with the 64-byte
/// signature trailing — and deliberately not a JSON envelope, a length header,
/// or anything else framing it. Three reasons, and the third is the one that
/// would be expensive to discover later:
///
/// 1. **The signed preimage is already a prefix of the message**, which is the
///    property `op.rs` chose the trailing-signature layout for. A decoder never
///    has to skip over a wrapper to find the bytes a signature covers.
/// 2. **SDS supplies the framing.** A reliable channel delivers a reassembled
///    payload as one unit, so a length prefix would be re-stating what the
///    transport already established, and the two could disagree.
/// 3. **Nothing peer-local can be added without it being visible.** A wrapper
///    is where a `senderId`, a local timestamp or a channel id would end up —
///    §4.5's "never let channel identity leak into payloads or storage keys" is
///    satisfied here by there being no payload envelope to leak into.
///
/// # Publishing is downstream of storing, never instead of it
///
/// The argument is the [`SignedOp`] itself rather than an [`Op`] plus a key,
/// because a peer publishes what it has already recorded. §3.3 makes the op log
/// the authority and every view "a cache that can be rebuilt by replay", so an
/// op that went to the network without reaching the log would be invisible to
/// its own author's next read — and a publish that failed would be
/// indistinguishable from one that never happened. Store, then publish what was
/// stored.
pub fn outbound_payload(op: &SignedOp) -> Vec<u8> {
    op.to_bytes()
}

/// Why a payload that arrived on a channel is not an op this peer will record.
///
/// # Each variant is a different mistake, and that is a requirement
///
/// The same rule `GenesisError` and [`OpError`] follow, and it matters more here
/// because this is the one boundary facing the open network. A single
/// catch-all "invalid op" cannot distinguish a forgery attempt from a peer
/// running a newer build, and those call for opposite responses: one is an
/// attack worth counting, the other is a client worth upgrading to. §2.5's
/// error shape carries one message, so the message has to say which.
#[derive(Debug, PartialEq, Eq)]
pub enum InboundError {
    /// The payload is larger than SDS's cap could have carried (§4.4).
    ///
    /// Refused **before decoding**, which is the whole point: a decoder handed
    /// 700 KB would faithfully decode it, having allocated 700 KB on a hostile
    /// peer's say-so. Carries both numbers because "too large" without them
    /// leaves an operator unable to tell a misconfiguration from an attack.
    TooLarge { bytes: usize, cap: usize },
    /// The bytes are not a well-formed op. Carries [`OpError`], so every
    /// distinction that decoder draws — a truncation, a bad length prefix, an
    /// unknown op kind, an unknown version — survives to the caller rather than
    /// being flattened into "malformed".
    Malformed(OpError),
    /// The op is well-formed and the signature does not authenticate it.
    ///
    /// **The forgery case**, and it covers both shapes at once because
    /// [`SignedOp::verify`] does: bytes altered after signing, and a valid
    /// signature from a key that is not the claimed author's. Kept as one
    /// variant rather than split, because the two are not distinguishable from
    /// outside — a failed Ed25519 verification does not say which of the two it
    /// was, and a variant claiming to know would be guessing.
    Forged,
    /// The op is authentic and addresses a different Stoa than the channel it
    /// arrived on.
    ///
    /// **Not redundant with the signature check**, and the reason is worth
    /// being precise about. The Stoa is inside the signed preimage, so an op
    /// cannot be *edited* onto another Stoa — `op.rs` pins that. What remains
    /// possible is a **replay**: lift a genuine, correctly-signed op from the
    /// Agora's channel and publish the unmodified bytes on the Lyceum's. Every
    /// signature check passes, because nothing was forged. Only comparing the
    /// op's own `stoa` against the channel this peer is listening on catches it.
    ///
    /// Carries both addresses, because "wrong Stoa" without them cannot be
    /// investigated.
    WrongStoa { expected: Address, found: Address },
}

impl std::fmt::Display for InboundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InboundError::TooLarge { bytes, cap } => write!(
                f,
                "an inbound payload claims {bytes} bytes, over the {cap}-byte \
                 SDS message cap, so no channel could have delivered it"
            ),
            InboundError::Malformed(e) => write!(f, "an inbound payload is not an op: {e}"),
            InboundError::Forged => write!(
                f,
                "an inbound op's signature does not authenticate the author it \
                 claims"
            ),
            InboundError::WrongStoa { expected, found } => write!(
                f,
                "an inbound op addresses Stoa {} but arrived on the channel for \
                 {}, so it is a replay from another Stoa",
                found.to_hex(),
                expected.to_hex()
            ),
        }
    }
}

/// An op that passed the boundary, with what the transport said about it.
///
/// # A pair rather than a bare [`SignedOp`], because the log takes two arguments
///
/// [`crate::log::OpLog::append`] takes `(SignedOp, Arrival)`, and the whole
/// value of returning both together is that the caller cannot supply its own
/// [`Arrival`]. A boundary that returned only the op would leave "and what do I
/// record about its arrival?" to each call site — and the wrong answer is
/// available and compiles, because the event carries a `timestamp`.
///
/// So the honest answer is constructed here, once, and handed over inseparably
/// from the op it describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundOp {
    /// The op, verified authentic and addressed to this channel's Stoa.
    pub op: SignedOp,
    /// What the transport supplied about its ordering: today, nothing.
    pub arrival: Arrival,
}

/// Decide whether a payload that arrived on a Stoa's channel may be recorded.
///
/// # This is the hostile-input boundary, and it is the only one
///
/// CLAUDE.md's standing rule: "Never trust an inbound message. Everything
/// arriving over the network is attacker-controlled: forged authorship,
/// malformed JSON, oversized payloads, ops targeting documents the sender has no
/// business touching. Validate at the boundary, before it reaches any state
/// machine."
///
/// The checks run in **increasing cost order**, and the order is load-bearing
/// rather than tidy:
///
/// 1. **Size**, against [`MAX_PAYLOAD_BYTES`] — before any allocation, because
///    refusing after decoding means having already paid for the decode.
/// 2. **Structure**, via [`SignedOp::from_bytes`] — which is where every length
///    prefix, discriminant and UTF-8 check in `op.rs` applies.
/// 3. **Authenticity**, via [`SignedOp::verify`] — the Ed25519 verification,
///    which is the expensive one and so goes after the two cheap refusals.
/// 4. **Address**, against the Stoa this channel serves — the replay check.
///
/// A peer that ran authenticity first would do curve arithmetic on every
/// malformed byte string a hostile peer cared to send, which is a
/// cheap-to-send, expensive-to-refuse asymmetry.
///
/// # What it does NOT check, restated because the omissions are the contract
///
/// Not moderator authority, not revision ownership, not posting policy, not
/// whether the target of a `Moderate` or `Revise` exists. The last is worth
/// naming separately: a dangling target is **ordinary**, not suspicious — §3.3
/// makes two peers holding different op sets the normal case, and the op log's
/// contract suite pins that a revision whose target is absent is stored and
/// readable. A boundary that required the target to be present would drop
/// exactly the ops that arrive out of order and never recover them.
///
/// # `stoa` is this peer's, never the message's
///
/// The `stoa` argument is the address of the channel this peer opened, which it
/// derived itself through [`channel_id`]. It is not read out of the payload —
/// that would make the check tautological, comparing the op against itself.
pub fn accept_inbound(stoa: &Address, payload: &[u8]) -> Result<InboundOp, InboundError> {
    // Before anything touches the bytes. The cap is the SDS frame's, and this
    // is the only place in the crate that can see a whole frame — `op.rs`'s
    // per-field cap explicitly defers the total to here.
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(InboundError::TooLarge {
            bytes: payload.len(),
            cap: MAX_PAYLOAD_BYTES,
        });
    }

    let op = SignedOp::from_bytes(payload).map_err(InboundError::Malformed)?;

    // Authenticity before addressing, because a `WrongStoa` reported for an op
    // whose signature does not hold would be describing a claim nobody made.
    if !op.verify() {
        return Err(InboundError::Forged);
    }

    // The replay check. Authentic bytes, genuinely signed, on the wrong
    // channel.
    if &op.op.stoa != stoa {
        return Err(InboundError::WrongStoa {
            expected: *stoa,
            found: op.op.stoa,
        });
    }

    Ok(InboundOp {
        op,
        // NEVER the event's `timestamp`. §4.4 and §13: it is the receiving
        // peer's own `CLOCK_REALTIME` read, so it differs per peer for one
        // message and orders nothing. `Arrival::unordered` is named rather than
        // reached by passing two `None`s precisely so that grepping for it finds
        // every place the contract's gap is absorbed — and this is that place.
        arrival: Arrival::unordered(),
    })
}

/// The JSON a caller gets back when an op was published.
///
/// `{"published":true,"opId":"<hex>"}`. The op id is echoed because a caller
/// that asked to publish a stored op needs to correlate the later
/// `channelMessageSent` / `channelMessageError` event with the op it was about,
/// and §4.4's ACK semantics make that correlation the only thing a caller can
/// learn: "ACK means 'some participants received it'", never that a particular
/// peer has it.
pub fn published_json(op: &Op) -> String {
    serde_json::json!({ "published": true, "opId": op.id().to_hex() }).to_string()
}

/// The JSON a caller gets back when an inbound op was recorded, or was not new.
///
/// `{"accepted":true,"opId":"<hex>","stored":bool}`.
///
/// **`stored` distinguishes a new op from one already held**, which is
/// [`crate::log::Appended`]'s distinction surfaced rather than flattened. It is
/// not an error either way: §3.1 makes ops idempotent by op id, and the op log's
/// contract records that retransmission, causal-history backfill and SDS-Repair
/// all deliver ops a peer already has — so a duplicate is ordinary traffic. A
/// caller still wants to know, because a newly stored op is one to rebuild a
/// view from and a duplicate is not.
pub fn accepted_json(op: &Op, stored: bool) -> String {
    serde_json::json!({
        "accepted": true,
        "opId": op.id().to_hex(),
        "stored": stored,
    })
    .to_string()
}

// ─── Request parsing, so the adapter stays a forwarding line ──────────────

/// What a transport handler needs out of a request: which Stoa, and which op.
///
/// A struct rather than a tuple, because both fields are opaque 32-byte values
/// and `(Address, OpId)` at a call site gives a reader no way to tell which slot
/// is which — the same reasoning `arrival.rs` gives for [`crate::arrival::OpEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishRequest {
    pub stoa: Address,
    pub op: crate::op::OpId,
}

/// Parse `{"stoa":"<hex>"}`, or return the error shape to send back.
///
/// `Result<_, String>` with the wire reply already in the error arm, matching
/// [`crate::wire::parse_channel_id`]: a caller cannot accidentally invent a
/// second error shape while converting one.
///
/// Each failure is named separately — absent, wrong-typed, unparseable — because
/// "missing field" for a field that is right there sends someone looking in the
/// wrong place. `wire.rs` makes the same distinction and this is deliberately the
/// same three arms rather than a fourth slightly-different copy.
pub fn parse_stoa(parsed: &serde_json::Value) -> Result<Address, String> {
    match parsed.get("stoa") {
        Some(serde_json::Value::String(s)) => Address::from_hex(s)
            .map_err(|e| crate::wire::error_json(&format!("stoa: {e}"))),
        Some(_) => Err(crate::wire::error_json("stoa must be a string")),
        None => Err(crate::wire::error_json("missing field: stoa")),
    }
}

/// Parse `{"stoa":"<hex>","op":"<hex>"}` for a publish request.
pub fn parse_publish_request(request: &str) -> Result<PublishRequest, String> {
    let parsed: serde_json::Value = serde_json::from_str(request)
        .map_err(|e| crate::wire::error_json(&format!("invalid JSON: {e}")))?;
    let stoa = parse_stoa(&parsed)?;
    let op = match parsed.get("op") {
        Some(serde_json::Value::String(s)) => crate::op::OpId::from_hex(s)
            .map_err(|e| crate::wire::error_json(&format!("op: {e}")))?,
        Some(_) => return Err(crate::wire::error_json("op must be a string")),
        None => return Err(crate::wire::error_json("missing field: op")),
    };
    Ok(PublishRequest { stoa, op })
}

/// The reply describing a Stoa's channel: where to reach it and under what id.
///
/// `{"channelId":"…","contentTopic":"…"}`. Exists so the adapter's
/// `channelCreate` call has one place to get all three strings from, and so a
/// caller can be told what was opened — §4.8's "show what is being joined before
/// joining it" needs the values to be inspectable rather than internal.
///
/// **`senderId` is deliberately not here.** §4.1 makes it one per user per Stoa
/// and permanent, derived from §5.2's per-Stoa identity — so it comes from the
/// keystore, which this crate cannot read (§2.3), and it is the one of the three
/// that must NOT be the same for every peer. Putting it in a reply beside the two
/// shared values would invite exactly the confusion §4.3 calls "the tell": two
/// adjacent parameters with opposite requirements.
pub fn channel_json(stoa: &Address) -> String {
    serde_json::json!({
        "channelId": channel_id(stoa),
        "contentTopic": content_topic(stoa),
    })
    .to_string()
}

// ─── Bounding the inbound queue ───────────────────────────────────────────

/// How many inbound payloads may wait to be ingested before the oldest is
/// dropped.
///
/// # Why a bound exists at all, which PLAN.md §2.3 assigns rather than suggests
///
/// > **Event queues are unbounded.** `std::sync::mpsc` with no backpressure, no
/// > bound and no drop policy; the C trampoline never blocks and never fails. A
/// > consumer slower than the event rate grows the queue until OOM — and
/// > ingesting `channelMessageReceived` during a backfill is precisely that
/// > shape. **Bound it ourselves.**
///
/// That is a remote memory-exhaustion lever on a censorship-resistant forum: SDS
/// has no membership (§4.4), so anyone may publish on a Stoa's channel as fast
/// as they can, and the trampoline that fills the queue cannot refuse. The
/// consumer is slower than the producer by construction, because ingesting means
/// an Ed25519 verification and a SQLite write per op.
///
/// # Why the value is this, and what it costs
///
/// 1024 payloads. At §4.4's 150 KiB cap that is a 150 MB worst case if every
/// slot held a maximum-size message, which is the number to argue with — but the
/// realistic figure is far lower, since an op carrying a post body rather than an
/// attachment CID is hundreds of bytes. The bound's job is to make the ceiling
/// *finite and stated* rather than to be optimal: an unbounded queue has no
/// worst case at all.
///
/// It is deliberately well above any legitimate burst. §4.7 records causal
/// history at 2 entries deep by default and SDS-R response groups at ~128, so a
/// backfill arriving faster than 1024 unprocessed messages is not a peer
/// catching up — it is a flood.
pub const MAX_QUEUED_PAYLOADS: usize = 1024;

/// A bounded queue of inbound payloads awaiting ingestion.
///
/// # The drop policy is oldest-first, and the choice is not obvious
///
/// When the queue is full something must go, and the two options are opposite:
///
/// - **Drop the newest** (refuse the arrival). Keeps a consistent prefix of what
///   arrived, and under a flood that prefix is *the flood* — the queue fills
///   with an attacker's traffic and every legitimate op behind it is refused
///   until the backlog drains. An attacker who can saturate the queue once locks
///   the peer out of the Stoa.
/// - **Drop the oldest** (this one). Under a flood the peer keeps making
///   progress on the most recent traffic, and what is lost is what it was
///   already furthest behind on.
///
/// Oldest-first wins because **dropping an op here is recoverable and stalling
/// is not.** SDS retransmits unacknowledged messages (§4.4), causal history and
/// SDS-Repair exist precisely to redeliver what a peer missed, and §3.3 makes a
/// peer holding a partial set the normal case — so a dropped payload is a gap the
/// transport is already built to close. A peer that stopped accepting new
/// arrivals, by contrast, would have to be restarted.
///
/// # Drops are counted, never silent
///
/// [`InboundQueue::dropped`] is the whole difference between a bound and a leak.
/// A peer discarding ops with no record could not distinguish "this Stoa is
/// quiet" from "I am shedding load", and those call for opposite responses —
/// exactly the confusion §11.1 forbids at the read path, arriving instead at the
/// write path.
#[derive(Debug, Default)]
pub struct InboundQueue {
    /// Oldest at the front. A `VecDeque` rather than a `Vec` because the drop
    /// policy removes from the front and arrivals push to the back, and a `Vec`
    /// would make the eviction O(n) per arrival under exactly the sustained load
    /// the bound exists for.
    queued: std::collections::VecDeque<Vec<u8>>,
    dropped: u64,
}

impl InboundQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept a payload, evicting the oldest if the queue is already full.
    ///
    /// Returns whether anything was evicted, so a caller can log the shed
    /// without having to poll [`InboundQueue::dropped`] for a change.
    ///
    /// **Oversized payloads are dropped here rather than queued**, and that is
    /// the one filtering decision this type makes. A payload over §4.4's cap
    /// cannot become an op no matter how long it waits, so holding one occupies
    /// a slot that a legitimate arrival could use — which is the memory pressure
    /// the bound exists to relieve, granted to an attacker for free. It counts
    /// as a drop, because that is what it is.
    pub fn push(&mut self, payload: Vec<u8>) -> Pushed {
        if payload.len() > MAX_PAYLOAD_BYTES {
            self.dropped += 1;
            return Pushed::RefusedOversize;
        }
        let evicted = if self.queued.len() >= MAX_QUEUED_PAYLOADS {
            self.queued.pop_front();
            self.dropped += 1;
            true
        } else {
            false
        };
        self.queued.push_back(payload);
        if evicted {
            Pushed::EvictedOldest
        } else {
            Pushed::Queued
        }
    }

    /// The next payload to ingest, oldest first.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.queued.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queued.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }

    /// How many payloads this peer has discarded without ingesting them.
    ///
    /// Monotonic, and never reset: it answers "has this peer been shedding
    /// load?", which a counter that could go back to zero could not.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// What [`InboundQueue::push`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pushed {
    /// Queued with room to spare.
    Queued,
    /// Queued, and the oldest waiting payload was discarded to make room.
    EvictedOldest,
    /// Not queued: larger than any SDS message could be, so it could never
    /// become an op.
    RefusedOversize,
}

// ─── The seam with the store ──────────────────────────────────────────────
//
// Two functions, and between them they are the whole interface this change
// needs from the write path. Both take `&mut impl OpLog`, which is the trait
// `log/mod.rs` already defines and both of its implementations already satisfy
// — so nothing new is invented here, and the write-path change can land beside
// this one without either touching the other's files.

/// Record an inbound op, having accepted it at the boundary.
///
/// # Why this exists rather than the caller doing both steps
///
/// It is two lines, and writing it once is the point: the pairing of
/// [`accept_inbound`] with [`OpLog::append`] is the invariant — **nothing
/// reaches the store without passing the boundary first** — and a caller free to
/// call `append` directly is a caller who can skip validation. There is one
/// ingest path, and this is it.
///
/// The error type is [`InboundError`] for a refusal and [`OpLogError`] for a
/// storage failure, kept apart because they mean opposite things: a refusal is
/// the boundary working, and a storage failure is this peer being broken. §2.5
/// collapses both into one error shape at the wire, but the collapse happens
/// there rather than here, so a caller that wants to count refusals separately
/// from disk failures can.
pub fn ingest<L: crate::log::OpLog>(
    log: &mut L,
    stoa: &Address,
    payload: &[u8],
) -> Result<Ingested, IngestError> {
    let accepted = accept_inbound(stoa, payload).map_err(IngestError::Refused)?;
    let appended = log
        .append(accepted.op.clone(), accepted.arrival)
        .map_err(IngestError::Storage)?;
    Ok(Ingested {
        op: accepted.op,
        stored: appended == crate::log::Appended::Stored,
    })
}

/// What [`ingest`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingested {
    /// The op, as recorded.
    pub op: SignedOp,
    /// Whether it was new to this peer. `false` is ordinary traffic, not a
    /// failure — §3.1 makes ops idempotent by op id.
    pub stored: bool,
}

/// Why an inbound payload did not become a recorded op.
#[derive(Debug, PartialEq, Eq)]
pub enum IngestError {
    /// The boundary refused it. This peer is working; the payload was bad.
    Refused(InboundError),
    /// The store could not be written. The payload may have been perfectly
    /// good, and **this is not a reason to drop it silently**: §11.1's rule that
    /// a storage failure is the error shape and never an empty result applies on
    /// the write side too. A peer that swallowed this would lose ops with no
    /// record of having done so.
    Storage(crate::log::OpLogError),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::Refused(e) => write!(f, "{e}"),
            IngestError::Storage(e) => write!(f, "{e}"),
        }
    }
}

/// The payload to publish for an op this peer holds, looked up by id.
///
/// # The other half of the seam, and it reads rather than writes
///
/// §3.3 makes the op log the authority, so "publish op X" is answered by reading
/// X out of the store and framing what is there — never by re-encoding an op the
/// caller happened to be holding. The difference matters for one reason: the
/// bytes in the store are the bytes that were signed, and a re-encode that
/// differed in any way would produce a payload that fails verification on the
/// peer receiving it. Reading is what makes that unreachable.
///
/// Returns `Ok(None)` when the log does not hold the op. That is a defined
/// answer and not an error, for the reason [`OpLog::get`] nests its types the
/// way it does: the op may simply not be here, which is different from the store
/// being unreadable.
///
/// # Checked against the Stoa it is being published to
///
/// An op is refused if its own Stoa is not the one whose channel it would go on.
/// Without this check a caller could publish the Agora's op to the Lyceum's
/// channel, where every receiving peer would refuse it as
/// [`InboundError::WrongStoa`] — so the check exists to fail here, locally and
/// legibly, rather than as silent rejection on every peer in the network.
pub fn outbound_for<L: crate::log::OpLog>(
    log: &L,
    stoa: &Address,
    id: &crate::op::OpId,
) -> Result<Option<Vec<u8>>, OutboundError> {
    let Some(entry) = log.get(id).map_err(OutboundError::Storage)? else {
        return Ok(None);
    };
    if &entry.op.op.stoa != stoa {
        return Err(OutboundError::WrongStoa {
            expected: *stoa,
            found: entry.op.op.stoa,
        });
    }
    Ok(Some(outbound_payload(&entry.op)))
}

/// Why an op this peer holds is not publishable to the channel asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum OutboundError {
    Storage(crate::log::OpLogError),
    /// The op belongs to a different Stoa than the channel it would go on.
    WrongStoa { expected: Address, found: Address },
}

impl std::fmt::Display for OutboundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutboundError::Storage(e) => write!(f, "{e}"),
            OutboundError::WrongStoa { expected, found } => write!(
                f,
                "op belongs to Stoa {} and cannot be published to the channel \
                 for {}",
                found.to_hex(),
                expected.to_hex()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{sign_op_bytes, stoa_address, PublicKey, SecretKey};
    use crate::op::{ModerationAction, OpId, OpKind, VoteDirection};
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    fn a_genesis(title: &str) -> Genesis {
        Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: title.to_string(),
        }
    }

    fn a_stoa(title: &str) -> Address {
        a_genesis(title)
            .address()
            .expect("a short title is well under MAX_TITLE_BYTES")
    }

    /// A post in `stoa`, signed by the key for `seed`.
    ///
    /// The author key and the `Op::author` field are the same key by
    /// construction, because `verify_authored_op` re-derives the address from
    /// the key and a fixture that got this wrong would be a forgery every test
    /// then had to work around.
    fn a_signed_post(stoa: Address, seed: u8, body: &str) -> SignedOp {
        let key = a_key(seed);
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    /// One op of every kind, so a property asserted "for any op" is asserted
    /// over the whole format rather than over posts.
    ///
    /// Mirrors `op.rs`'s `one_of_each_kind`. Kept local rather than reaching
    /// into that module's test fixtures, which are private to it.
    fn one_of_each_kind(stoa: Address) -> Vec<SignedOp> {
        let key = a_key(2);
        let author = key.public_key();
        let target = OpId::from_hex(&"7a".repeat(32)).unwrap();
        [
            OpKind::Post {
                thread: None,
                parent: None,
                body: "first".to_string(),
                attachments: vec![],
            },
            OpKind::Post {
                thread: Some(target),
                parent: Some(target),
                body: "a reply".to_string(),
                attachments: vec!["cid-one".to_string()],
            },
            OpKind::Revise {
                target,
                body: "edited".to_string(),
                attachments: vec![],
            },
            OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
            OpKind::Moderate {
                target,
                action: ModerationAction::Unhide,
            },
            OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
            OpKind::StoaMetadata {
                title: "renamed".to_string(),
                description: "a description".to_string(),
            },
        ]
        .into_iter()
        .map(|kind| {
            Op {
                stoa,
                author: author.clone(),
                kind,
            }
            .sign(&key)
        })
        .collect()
    }

    // ─── The rendezvous ───────────────────────────────────────────────────

    #[test]
    fn the_channel_id_is_a_pure_function_of_the_stoa_address() {
        // §4.3: every peer in a Stoa must compute the SAME value. Derived twice
        // from the same address with unrelated work in between — a derivation
        // consulting a counter, a clock or any `self` state would differ
        // between the two calls.
        let stoa = a_stoa("Agora");
        let first = channel_id(&stoa);
        // Unrelated derivations, to advance anything that might be advancing.
        for other in ["Lyceum", "Academy", "Stoa Poikile"] {
            let _ = channel_id(&a_stoa(other));
            let _ = content_topic(&a_stoa(other));
        }
        let second = channel_id(&stoa);
        assert_eq!(
            first, second,
            "the channel id moved between two derivations from one address"
        );
    }

    #[test]
    fn two_peers_deriving_the_channel_id_from_one_address_agree() {
        // The property at the level §4.3 states it, and the failure it names:
        // "A value that differs between peers does not produce an error: it
        // produces two Stoas that cannot see each other, silently and
        // permanently."
        //
        // Two peers cannot be stood up here, so what is simulated is the only
        // thing that differs between them: the address is reconstructed from
        // its hex form, as a peer that received a Stoa address over the network
        // would have it, rather than shared as a value.
        let mine = a_stoa("Agora");
        let theirs = Address::from_hex(&mine.to_hex()).expect("an address round-trips through hex");
        assert_eq!(channel_id(&mine), channel_id(&theirs));
        assert_eq!(content_topic(&mine), content_topic(&theirs));
    }

    #[test]
    fn two_stoas_get_two_channel_ids_and_two_topics() {
        // The other direction, and it is not implied by agreement: a derivation
        // returning one constant would pass every agreement test above.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum, "the fixture needs two distinct Stoas");
        assert_ne!(channel_id(&agora), channel_id(&lyceum));
        assert_ne!(content_topic(&agora), content_topic(&lyceum));
    }

    #[test]
    fn the_channel_id_carries_the_whole_address_and_not_a_prefix() {
        // A derivation truncating the address would produce agreeing values
        // and collide two Stoas onto one channel — which reads as a cross-Stoa
        // leak, the same failure `contract.rs` constructs long-prefix
        // addresses to catch in the op log.
        //
        // Constructed, not hunted: `Address::from_bytes` takes a `[u8; 32]` and
        // nothing about a channel id requires a hash-derived address, so two
        // addresses agreeing on their first 31 bytes are a valid fixture and
        // are buildable rather than searched for. A test using two hash-derived
        // addresses would differ in byte 0 and pass with any prefix length.
        let mut bytes = [0x11u8; 32];
        let one = Address::from_bytes(bytes);
        bytes[31] = 0x12;
        let two = Address::from_bytes(bytes);
        assert_ne!(one, two, "the fixture needs two distinct addresses");

        assert_ne!(
            channel_id(&one),
            channel_id(&two),
            "two addresses differing only in the last byte share a channel"
        );
        assert_ne!(content_topic(&one), content_topic(&two));
    }

    #[test]
    fn the_channel_id_and_the_content_topic_are_pinned_to_the_plans_strings() {
        // §4.1 gives the content topic verbatim as
        // `/dialectica/1/s/<hex>/proto`. Pinned as a literal, not composed from
        // the same `format!` the implementation uses — asserting against a
        // re-derivation is the test agreeing with itself, and every peer in the
        // network has to produce this exact byte string.
        //
        // If this fails, do NOT update the expected value. A changed channel id
        // partitions the network from every peer running the old one, with no
        // error anywhere.
        let stoa = Address::from_bytes([0xabu8; 32]);
        let hex = "ab".repeat(32);
        assert_eq!(content_topic(&stoa), format!("/dialectica/1/s/{hex}/proto"));
        assert_eq!(channel_id(&stoa), format!("/dialectica/1/s/{hex}"));
    }

    #[test]
    fn the_channel_id_survives_a_close_and_reopen_with_no_epoch() {
        // §4.3, after the #4116 workaround was withdrawn: "The channel id stays
        // a pure function of the addressed object. No epoch in it — not a
        // per-peer one, and not a deterministic one either."
        //
        // A rejoin is modelled as what it is at this layer: deriving the id
        // again. Anything epoch-shaped would make the second derivation differ,
        // which §4.3 says partitions peer A at `/e8` from peer B still at
        // `/e7`.
        let stoa = a_stoa("Agora");
        let joined = channel_id(&stoa);
        let rejoined = channel_id(&stoa);
        assert_eq!(joined, rejoined, "an epoch crept into the channel id");
        // And the id contains no segment that looks like one. A derivation
        // appending `/e0` would satisfy the equality above forever while still
        // being the shape §4.3 rules out.
        assert_eq!(
            joined.matches('/').count(),
            4,
            "the channel id grew a segment: {joined}"
        );
    }

    #[test]
    fn no_human_readable_title_reaches_the_topic_or_the_channel_id() {
        // §4.1: "Never put a human-readable Stoa name in a topic. Filter, Store
        // and LightPush disclose content topics to peers, linking IP to
        // interest."
        //
        // The title is chosen to be findable if it leaked — a derivation that
        // interpolated `genesis.title` anywhere would put it here verbatim.
        let title = "Dissidents Anonymous";
        let stoa = a_stoa(title);
        for derived in [content_topic(&stoa), channel_id(&stoa)] {
            assert!(
                !derived.contains(title),
                "a Stoa title reached a transport identifier: {derived}"
            );
            assert!(
                !derived.to_lowercase().contains("dissident"),
                "a fragment of the title reached a transport identifier: {derived}"
            );
        }
    }

    // ─── The sender id ────────────────────────────────────────────────────

    #[test]
    fn two_users_in_one_stoa_get_different_sender_ids() {
        // §4.3 quotes SDS's MUST: the sender "MUST include its own globally
        // unique identifier in the `sender_id` field". This is the requirement
        // that runs OPPOSITE to the channel id's — two peers in one Stoa must
        // agree on the channel and differ on this.
        //
        // Derived through real keystores, so what is tested is the composition
        // a caller actually performs rather than a hand-built address.
        let stoa = a_stoa("Agora");
        let alice = crate::keystore::Keystore::generate();
        let bob = crate::keystore::Keystore::generate();
        assert_ne!(
            sender_id(&alice.stoa_address(&stoa)),
            sender_id(&bob.stoa_address(&stoa)),
            "two users share a sender id, which SDS states as a MUST violation"
        );
    }

    #[test]
    fn one_user_keeps_the_same_sender_id_across_sessions() {
        // §4.1: "what is stable is that a given user keeps theirs across
        // sessions", and the Reliable Channel API's SHOULD that it be "persisted
        // between sessions" — a duty §4.1 assigns to dialectica because "nothing
        // in the delivery module persists it".
        //
        // A new session is modelled as what it is: the same root secret, a
        // freshly derived value. Nothing is stored, so nothing can be lost —
        // which is the property that discharges the SHOULD without persistence
        // machinery.
        let stoa = a_stoa("Agora");
        let keystore = crate::keystore::Keystore::generate();
        let first_session = sender_id(&keystore.stoa_address(&stoa));
        let second_session = sender_id(&keystore.stoa_address(&stoa));
        assert_eq!(first_session, second_session);
    }

    #[test]
    fn one_user_gets_a_different_sender_id_in_each_stoa() {
        // §5.2's per-Stoa identity, which is what makes the disclosure scope
        // acceptable: §4.1 records that a sender id "links a Stoa's posts to one
        // pseudonym and no further". A sender id shared across Stoas would link
        // a user's pseudonyms, which is precisely what per-Stoa identity exists
        // to prevent.
        let keystore = crate::keystore::Keystore::generate();
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(
            sender_id(&keystore.stoa_address(&agora)),
            sender_id(&keystore.stoa_address(&lyceum)),
            "one sender id spans two Stoas, linking the user's pseudonyms"
        );
    }

    #[test]
    fn a_sender_id_is_never_equal_to_the_channel_id_it_travels_with() {
        // The two values are adjacent arguments to one call with opposite
        // requirements, and §4.3 spends a section on the confusion. They are
        // domain-separated by their middle segment, so a transposition at a call
        // site is visible in a log rather than silent.
        //
        // The sharp case is a user whose per-Stoa address happens to equal the
        // Stoa address: without separate prefixes the two strings would be
        // identical. Constructed rather than hoped for.
        let same = Address::from_bytes([0x33u8; 32]);
        assert_ne!(
            sender_id(&same),
            channel_id(&same),
            "a sender id and a channel id can collide"
        );
        assert_ne!(sender_id(&same), content_topic(&same));
    }

    #[test]
    fn a_sender_id_is_not_an_author_address_and_does_not_parse_as_one() {
        // §4.1: "`senderId` is not an author identity, and the plan should not
        // treat it as one." The return type is a `String` so it cannot be passed
        // where an `Address` is wanted — and the value is not bare hex, so a
        // caller who tried to re-parse it as one fails loudly rather than
        // succeeding and treating a transport handle as authorship.
        let author = Address::from_bytes([0x44u8; 32]);
        let id = sender_id(&author);
        assert!(
            Address::from_hex(&id).is_err(),
            "a sender id parses as an author address: {id}"
        );
    }

    #[test]
    fn the_sender_id_is_pinned_to_its_exact_form() {
        // Pinned like the channel id, and for a related reason: SDS assumes a
        // Participant ID is "immutable", and a peer whose sender id changed
        // between builds would look to SDS-R like a different participant —
        // §4.1 records that repair backoff and response-group membership are both
        // computed from it.
        let author = Address::from_bytes([0x55u8; 32]);
        assert_eq!(
            sender_id(&author),
            format!("/dialectica/1/a/{}", "55".repeat(32))
        );
    }

    // ─── Request parsing ──────────────────────────────────────────────────

    #[test]
    fn a_publish_request_is_parsed_into_a_stoa_and_an_op() {
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "to publish").op.id();
        let request = format!(
            r#"{{"stoa":"{}","op":"{}"}}"#,
            stoa.to_hex(),
            op.to_hex()
        );
        assert_eq!(
            parse_publish_request(&request).unwrap(),
            PublishRequest { stoa, op }
        );
    }

    #[test]
    fn a_malformed_publish_request_names_which_field_is_wrong() {
        // Each failure is a different mistake, and "missing field" for a field
        // that is right there sends someone looking in the wrong place. The same
        // three arms `wire.rs::parse_channel_id` distinguishes, so this is one
        // rule rather than a fourth slightly-different copy of it.
        let stoa = a_stoa("Agora").to_hex();
        let op = "7a".repeat(32);
        let cases = [
            ("not json", "invalid JSON"),
            (r#"{}"#, "missing field: stoa"),
            (r#"{"stoa":7}"#, "stoa must be a string"),
            (r#"{"stoa":"nothex"}"#, "stoa:"),
            (r#"{"stoa":"00ff"}"#, "stoa:"),
        ];
        for (bad, expect) in cases {
            let err = parse_publish_request(bad).unwrap_err();
            assert!(err.contains(expect), "for {bad:?} got {err}");
            // And the error arm is already the wire shape, so a caller cannot
            // invent a second one while converting.
            let v: serde_json::Value = serde_json::from_str(&err)
                .unwrap_or_else(|e| panic!("error arm must be valid JSON ({e}): {err}"));
            assert!(v.get("error").is_some(), "got {err}");
        }

        // The `op` field's own three arms, with a valid `stoa` so the failure is
        // unambiguously about `op`.
        for (bad, expect) in [
            (format!(r#"{{"stoa":"{stoa}"}}"#), "missing field: op"),
            (format!(r#"{{"stoa":"{stoa}","op":7}}"#), "op must be a string"),
            (format!(r#"{{"stoa":"{stoa}","op":"nothex"}}"#), "op:"),
            (format!(r#"{{"stoa":"{stoa}","op":"00ff"}}"#), "op:"),
        ] {
            let err = parse_publish_request(&bad).unwrap_err();
            assert!(err.contains(expect), "for {bad:?} got {err}");
        }
        // And the fully-valid form parses, so the loop above is not passing
        // because every input is rejected.
        assert!(parse_publish_request(&format!(r#"{{"stoa":"{stoa}","op":"{op}"}}"#)).is_ok());
    }

    #[test]
    fn the_channel_reply_carries_both_shared_values_and_no_sender_id() {
        // A caller is told what was opened, which §4.8's "show what is being
        // joined before joining it" needs. `senderId` is deliberately absent:
        // it is the one of the three that must NOT be shared, and putting it
        // beside the two that must be is how the confusion §4.3 warns about
        // starts.
        let stoa = a_stoa("Agora");
        let v: serde_json::Value = serde_json::from_str(&channel_json(&stoa)).unwrap();
        assert_eq!(v["channelId"], channel_id(&stoa));
        assert_eq!(v["contentTopic"], content_topic(&stoa));
        assert!(
            v.get("senderId").is_none(),
            "a per-peer value must not appear beside the shared ones: {v}"
        );
    }

    // ─── Publishing ───────────────────────────────────────────────────────

    #[test]
    fn the_outbound_payload_is_the_signed_op_and_nothing_around_it() {
        // No envelope, no length prefix, no wrapper — because a wrapper is
        // where a senderId, a local clock reading or a channel id would end up.
        // Asserted for every kind, since a per-kind wrapper would be odder and
        // still possible.
        for op in one_of_each_kind(a_stoa("Agora")) {
            assert_eq!(outbound_payload(&op), op.to_bytes());
        }
    }

    #[test]
    fn a_published_payload_is_exactly_what_a_receiver_accepts() {
        // THE round trip, and the property that makes the two halves one
        // protocol: what `outbound_payload` emits is what `accept_inbound`
        // takes. Either side drifting — a wrapper added on send, a header
        // expected on receive — breaks the network while both halves' own tests
        // stay green.
        let stoa = a_stoa("Agora");
        for op in one_of_each_kind(stoa) {
            let payload = outbound_payload(&op);
            let accepted = accept_inbound(&stoa, &payload)
                .unwrap_or_else(|e| panic!("a peer's own published op was refused: {e}"));
            assert_eq!(accepted.op, op, "the op did not survive the round trip");
        }
    }

    #[test]
    fn an_accepted_op_is_byte_identical_to_the_one_published() {
        // Stronger than equality of the decoded value, and the reason is the
        // signature: a boundary that decoded and re-encoded would produce an
        // equal `SignedOp` whose bytes differ, and the op would then fail to
        // verify on the next peer it was forwarded to. The op log's contract
        // pins the same property at the store, for the same reason.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "exact");
        let accepted = accept_inbound(&stoa, &outbound_payload(&op)).unwrap();
        assert_eq!(accepted.op.to_bytes(), op.to_bytes());
        assert!(
            accepted.op.verify(),
            "the boundary disturbed the signature"
        );
    }

    // ─── The arrival the boundary records ─────────────────────────────────

    #[test]
    fn an_accepted_op_is_recorded_as_unordered() {
        // §4.4 and §13: `channelMessageReceived` supplies no Lamport timestamp
        // and no message id, and its `timestamp` is a local clock read. So the
        // honest record is "the transport ordered this: no".
        //
        // Asserted against hardcoded `None`s rather than against
        // `Arrival::unordered()`, for the reason `arrival.rs` gives about its
        // own equivalent test: comparing against the constructor would agree
        // with a constructor that had started defaulting the Lamport value to
        // 0, and a real 0 is indistinguishable from a fabricated one forever
        // after.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "unordered");
        let accepted = accept_inbound(&stoa, &outbound_payload(&op)).unwrap();
        assert_eq!(accepted.arrival.lamport(), None);
        assert_eq!(accepted.arrival.message_id(), None);
        assert!(!accepted.arrival.is_ordered_by_transport());
    }

    #[test]
    fn the_boundary_takes_no_timestamp_and_so_records_the_same_arrival_always() {
        // The mutation this is built to kill: threading the event's `timestamp`
        // into the `Arrival`. It is available at the call site, it is an
        // integer, and it would compile.
        //
        // `accept_inbound`'s signature has no timestamp parameter, so the
        // mutation cannot be made without changing the signature — and this
        // test witnesses the observable consequence: two peers accepting the
        // SAME op record identical arrivals. A boundary reading any local value
        // would produce two different ones here, and the two peers would then
        // sort that op differently with no error anywhere.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "one message, two peers");
        let payload = outbound_payload(&op);

        let peer_one = accept_inbound(&stoa, &payload).unwrap();
        let peer_two = accept_inbound(&stoa, &payload).unwrap();
        assert_eq!(
            peer_one.arrival, peer_two.arrival,
            "two peers recorded different arrivals for one message"
        );
        assert_eq!(peer_one, peer_two, "the whole accepted op must agree");
    }

    // ─── Hostile input: oversize ──────────────────────────────────────────

    #[test]
    fn a_payload_over_the_sds_cap_is_refused_before_it_is_decoded() {
        // §4.4's 150 KiB cap, and `op.rs`'s explicit deferral of the TOTAL-size
        // check to the transport boundary: "the total-size check belongs at the
        // transport boundary, where the SDS frame is known."
        //
        // The payload is one byte over, so the test cannot pass by refusing
        // something absurd — a cap that silently drifted to 1 MiB would still
        // refuse a 4 GiB claim and still pass a test that only probed absurd
        // values. That is the defect `MAX_FIELD_LEN`'s own pinning test exists
        // to prevent, applied here.
        let stoa = a_stoa("Agora");
        let payload = vec![0u8; MAX_PAYLOAD_BYTES + 1];
        assert_eq!(
            accept_inbound(&stoa, &payload),
            Err(InboundError::TooLarge {
                bytes: MAX_PAYLOAD_BYTES + 1,
                cap: MAX_PAYLOAD_BYTES,
            })
        );
    }

    #[test]
    fn the_size_check_runs_before_the_decode_and_not_after() {
        // The ordering, which is the whole value of the check. An oversized
        // payload that is ALSO well-formed garbage must come back as `TooLarge`
        // and not as `Malformed` — if it reports `Malformed`, the decoder ran
        // first and the allocation the cap exists to prevent already happened.
        //
        // A 700 KB run of 0xFF decodes to nothing (0xFF is not a known op
        // version), so a boundary checking size second would report
        // `Malformed(UnknownVersion(255))` here. That is what makes this test
        // able to tell the two orderings apart.
        let stoa = a_stoa("Agora");
        let payload = vec![0xFFu8; MAX_PAYLOAD_BYTES * 4];
        match accept_inbound(&stoa, &payload) {
            Err(InboundError::TooLarge { .. }) => {}
            other => panic!("the size check ran after the decode: {other:?}"),
        }
    }

    #[test]
    fn a_payload_exactly_at_the_cap_is_not_refused_for_its_size() {
        // The boundary from the other side: the cap is inclusive, so a payload
        // at exactly 150 KiB is one SDS could have carried. It fails for being
        // junk, which is a DIFFERENT error — an off-by-one that refused it as
        // oversized would reject legitimate maximum-size traffic, and the
        // `TooLarge` message would send someone looking at the wrong thing.
        let stoa = a_stoa("Agora");
        let payload = vec![0u8; MAX_PAYLOAD_BYTES];
        match accept_inbound(&stoa, &payload) {
            Err(InboundError::TooLarge { .. }) => {
                panic!("a payload exactly at the cap was refused as oversized")
            }
            Err(InboundError::Malformed(_)) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn the_payload_cap_is_pinned_to_the_number_the_plan_states() {
        // §4.4: "150 KiB max message size, hard cap — a network-wide gossipsub
        // validation limit, not unilaterally raisable."
        //
        // Pinned as a literal because a cap that drifted upward would pass
        // every other test here: they probe one byte over and one byte under a
        // value they read from this same constant. Only a hardcoded number
        // catches the constant itself moving.
        assert_eq!(MAX_PAYLOAD_BYTES, 153_600);
    }

    // ─── Hostile input: malformed ─────────────────────────────────────────

    #[test]
    fn truncated_bytes_are_refused_as_malformed_and_say_where_they_ended() {
        // Every prefix of a real op, so the test covers a truncation at every
        // offset rather than at one arbitrary one. None may be accepted, and
        // each must come back as `Malformed` carrying the decoder's own
        // distinction rather than a catch-all.
        let stoa = a_stoa("Agora");
        let full = outbound_payload(&a_signed_post(stoa, 2, "a body long enough to cut"));
        for cut in 0..full.len() {
            match accept_inbound(&stoa, &full[..cut]) {
                Err(InboundError::Malformed(_)) => {}
                other => panic!("a {cut}-byte prefix of an op was not Malformed: {other:?}"),
            }
        }
        // And the untruncated payload IS accepted, so the loop above is not
        // passing because the fixture was broken to begin with.
        assert!(accept_inbound(&stoa, &full).is_ok());
    }

    #[test]
    fn trailing_bytes_after_a_complete_op_are_refused() {
        // Not merely "the op decodes" — a decoder that stopped at the end of
        // the op and ignored what followed would let one payload carry an op
        // plus arbitrary attacker data, and two peers could disagree about
        // whether that payload was one op or two.
        let stoa = a_stoa("Agora");
        let mut payload = outbound_payload(&a_signed_post(stoa, 2, "complete"));
        payload.push(0x00);
        match accept_inbound(&stoa, &payload) {
            Err(InboundError::Malformed(_)) => {}
            other => panic!("bytes after a complete op were accepted: {other:?}"),
        }
    }

    #[test]
    fn an_unknown_op_version_is_distinguishable_from_corruption() {
        // "Newer client" and "corrupt" call for opposite responses, and `op.rs`
        // keeps them apart — `UnknownVersion` rather than a misparse. The
        // boundary must not flatten that: carrying `OpError` through is what
        // makes an operator able to tell an upgrade from an attack.
        let stoa = a_stoa("Agora");
        let mut payload = outbound_payload(&a_signed_post(stoa, 2, "from the future"));
        payload[0] = 0x7F;
        assert_eq!(
            accept_inbound(&stoa, &payload),
            Err(InboundError::Malformed(OpError::UnknownVersion(0x7F)))
        );
    }

    #[test]
    fn an_unknown_op_kind_is_distinguishable_from_an_unknown_version() {
        // The second axis of the same distinction. Both are "a build newer than
        // this one", and they point at different things to look at — a format
        // generation versus one new act. Flattening them into `Malformed` with
        // no detail is what this asserts against.
        let stoa = a_stoa("Agora");
        let mut payload = outbound_payload(&a_signed_post(stoa, 2, "a new kind"));
        // Byte 1 is the kind, per `op.rs`'s documented layout:
        //   version 1 | kind 1 | stoa 32 | author 32 | <kind-specific>
        payload[1] = 0x5A;
        assert_eq!(
            accept_inbound(&stoa, &payload),
            Err(InboundError::Malformed(OpError::UnknownKind(0x5A)))
        );
    }

    #[test]
    fn an_empty_payload_is_refused() {
        // The degenerate case, which is reachable: a channel can deliver a
        // zero-length payload. `SignedOp::from_bytes` subtracts 64 for the
        // signature and must not underflow — an `unwrap` on that subtraction
        // would panic, and PHASE0-FINDINGS §3 measured that a panic ABORTS the
        // module process. So this is a denial-of-service probe, not a
        // formality.
        let stoa = a_stoa("Agora");
        assert_eq!(
            accept_inbound(&stoa, &[]),
            Err(InboundError::Malformed(OpError::Truncated))
        );
    }

    #[test]
    fn a_hostile_length_prefix_does_not_allocate_on_its_claim() {
        // A four-byte length prefix can claim 4 GiB. `op.rs`'s
        // `take_length_within_cap` refuses before reading, and this asserts the
        // property survives at the boundary: a payload whose body length claims
        // far more than the payload holds is refused, and cheaply.
        //
        // The payload is tiny and the claim enormous, so a decoder that
        // reserved on the claim would be visible as a memory spike or an abort
        // rather than as a failed assertion — which is the honest limit of what
        // a unit test can witness here.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "small");
        let mut payload = outbound_payload(&op);
        // A `Post`'s body length prefix sits after the header and the two
        // option tags: 1 + 1 + 32 + 32 + 1 + 1 = 68.
        const POST_BODY_LEN_AT: usize = 68;
        payload[POST_BODY_LEN_AT..POST_BODY_LEN_AT + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        match accept_inbound(&stoa, &payload) {
            Err(InboundError::Malformed(OpError::FieldTooLong(n))) => {
                assert_eq!(n, u32::MAX as usize);
            }
            other => panic!("a 4 GiB length claim was not refused by the cap: {other:?}"),
        }
    }

    // ─── Hostile input: forgery ───────────────────────────────────────────

    #[test]
    fn an_op_signed_by_someone_other_than_its_claimed_author_is_refused() {
        // THE forgery: a valid signature over untampered bytes that is still
        // not from the author the op claims. Only re-deriving the address from
        // the key catches it, which is what `verify_authored_op` does and what
        // the boundary must actually call.
        let stoa = a_stoa("Agora");
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            stoa,
            author: victim.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "I did not write this".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        // The fixture must genuinely be a forgery, or the test proves nothing.
        assert!(!forged.verify(), "the fixture must be an actual forgery");

        assert_eq!(
            accept_inbound(&stoa, &forged.to_bytes()),
            Err(InboundError::Forged)
        );
    }

    #[test]
    fn an_op_whose_body_was_altered_after_signing_is_refused() {
        // The other shape of the same refusal: the author is real, the
        // signature is theirs, and a relay changed the bytes in flight.
        //
        // # The mutation has to stay valid UTF-8, and finding that out was the
        // # point of writing the test
        //
        // The first version of this flipped `^= 0xFF` on a body byte, and it
        // failed with `Malformed(InvalidText)` rather than `Forged`: XOR-ing
        // 0xFF onto an ASCII character produces a byte above 0x7F that is not a
        // valid UTF-8 sequence, so `op.rs`'s decoder refused it before
        // `verify()` was ever reached.
        //
        // That is correct behaviour and a green test either way would have been
        // the defect — it would have claimed to exercise the signature check
        // while actually exercising the UTF-8 check, and a stubbed-out
        // `verify()` would have left it passing. So the mutation swaps one
        // ASCII letter for another: the bytes stay decodable, the decoder has
        // nothing to complain about, and only the signature can reject it.
        let stoa = a_stoa("Agora");
        let genuine = a_signed_post(stoa, 2, "the original");
        let mut payload = outbound_payload(&genuine);
        // A `Post`'s body text begins after the header, the two option tags and
        // the 4-byte length prefix: 1 + 1 + 32 + 32 + 1 + 1 + 4 = 72.
        const POST_BODY_AT: usize = 72;
        assert_eq!(payload[POST_BODY_AT], b't', "the fixture's body moved");
        payload[POST_BODY_AT] = b'T';

        // The altered payload still decodes — so this test cannot pass by
        // accident through the decoder, which is what the first version did.
        assert!(
            Op::decode(&payload[..payload.len() - 64]).is_ok(),
            "the mutation must leave the op decodable, or the signature check \
             is not what refuses it"
        );
        assert_eq!(
            accept_inbound(&stoa, &payload),
            Err(InboundError::Forged),
            "a body altered in flight was accepted"
        );
    }

    #[test]
    fn an_op_whose_signature_was_replaced_with_another_ops_is_refused() {
        // A signature lifted from a genuine op of the same author and attached
        // to a different op. Both halves are authentic in isolation; the pair
        // is not. This is the shape a naive "does this decode and is the author
        // a real key?" check would accept.
        let stoa = a_stoa("Agora");
        let one = a_signed_post(stoa, 2, "the first");
        let two = a_signed_post(stoa, 2, "the second");
        assert_ne!(one.signature, two.signature, "the fixture needs two sigs");

        let spliced = SignedOp {
            op: one.op.clone(),
            signature: two.signature.clone(),
        };
        assert_eq!(
            accept_inbound(&stoa, &spliced.to_bytes()),
            Err(InboundError::Forged)
        );
    }

    #[test]
    fn an_op_carrying_an_all_zero_signature_is_refused() {
        // The laziest forgery, and worth its own test because it is the one an
        // attacker tries first and the one a stubbed-out verification would
        // accept. Ed25519 parses any 64 bytes — validity is deferred to
        // verification — so this reaches `verify` rather than failing to
        // decode.
        let stoa = a_stoa("Agora");
        let genuine = a_signed_post(stoa, 2, "unsigned, really");
        let mut payload = outbound_payload(&genuine);
        let sig_at = payload.len() - 64;
        payload[sig_at..].fill(0);
        assert_eq!(
            accept_inbound(&stoa, &payload),
            Err(InboundError::Forged)
        );
    }

    #[test]
    fn a_forgery_is_refused_for_every_op_kind() {
        // The signature check must not be kind-dependent. A boundary that
        // verified posts and waved moderation ops through would be the exact
        // conflation §6.2 measured in the nearest kin project, which "checks
        // moderator authority only on the send path and never on the read
        // path".
        let stoa = a_stoa("Agora");
        let attacker = a_key(3);
        for genuine in one_of_each_kind(stoa) {
            let forged = SignedOp {
                signature: sign_op_bytes(&attacker, &genuine.op.canonical_bytes()),
                op: genuine.op.clone(),
            };
            assert!(!forged.verify(), "the fixture must be a forgery");
            assert_eq!(
                accept_inbound(&stoa, &forged.to_bytes()),
                Err(InboundError::Forged),
                "a forged {:?} was accepted",
                forged.op.kind
            );
        }
    }

    // ─── Hostile input: the wrong Stoa ────────────────────────────────────

    #[test]
    fn a_genuine_op_replayed_onto_another_stoas_channel_is_refused() {
        // THE check the signature cannot make. The op is authentic and
        // unmodified — lifted whole from the Agora's channel and published on
        // the Lyceum's. Every cryptographic check passes; only comparing the
        // op's own Stoa against the channel's catches it.
        //
        // This is the one hostile case where the attacker forges nothing at
        // all, which is why a boundary that stopped at `verify()` would let it
        // through.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum);

        let op = a_signed_post(agora, 2, "posted in the agora");
        let payload = outbound_payload(&op);

        // Authentic: it passes on its own channel.
        assert!(
            accept_inbound(&agora, &payload).is_ok(),
            "the fixture must be a genuine op"
        );
        assert!(op.verify(), "and it must genuinely verify");

        // And is refused on the other one, naming both addresses.
        assert_eq!(
            accept_inbound(&lyceum, &payload),
            Err(InboundError::WrongStoa {
                expected: lyceum,
                found: agora,
            })
        );
    }

    #[test]
    fn the_stoa_check_compares_the_whole_address_and_not_a_prefix() {
        // A prefix comparison here is a cross-Stoa leak: one Stoa's ops
        // rendering inside another, and one Stoa's moderator set consulted
        // against another's content. `contract.rs` constructs long-prefix
        // addresses for the op log's reads for exactly this reason, and
        // records that a review found a 2-byte prefix match passing 443 tests.
        //
        // Constructed rather than hunted, for the same reason: two hash-derived
        // addresses differ in byte 0, so a test using them passes at any prefix
        // length. These agree on 31 of 32 bytes.
        let mut bytes = [0x11u8; 32];
        let mine = Address::from_bytes(bytes);
        bytes[31] = 0x12;
        let theirs = Address::from_bytes(bytes);
        assert_ne!(mine, theirs);

        // The op is signed for `theirs` and arrives on `mine`'s channel. A
        // prefix comparison would accept it.
        let key = a_key(2);
        let op = Op {
            stoa: theirs,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "from next door".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);

        assert_eq!(
            accept_inbound(&mine, &op.to_bytes()),
            Err(InboundError::WrongStoa {
                expected: mine,
                found: theirs,
            }),
            "a 31-byte address prefix match leaked a Stoa across channels"
        );
    }

    #[test]
    fn a_forged_op_for_the_wrong_stoa_is_reported_as_forged_and_not_as_a_replay() {
        // The check order, at the one place it is observable. An op that is
        // BOTH forged and addressed elsewhere must report `Forged`: a
        // `WrongStoa` for an op whose signature does not hold would be
        // describing a claim nobody actually made, and would send an operator
        // hunting a replay that never happened.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            stoa: agora,
            author: victim.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "both wrong".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        assert_eq!(
            accept_inbound(&lyceum, &forged.to_bytes()),
            Err(InboundError::Forged)
        );
    }

    #[test]
    fn an_op_from_an_author_this_peer_has_never_seen_is_accepted() {
        // The case that must NOT be refused, and it is here because it is the
        // easy mistake in the other direction: there is no registry of authors
        // (§3.3), a Stoa is permissionless, and "I do not recognise this key"
        // is not a reason to reject anything. A boundary that maintained a
        // known-author set would make the forum invite-only by accident.
        let stoa = a_stoa("Agora");
        // A key seed used by no other fixture in this module.
        let stranger = a_key(0xEE);
        let op = Op {
            stoa,
            author: stranger.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "hello, I am new".to_string(),
                attachments: vec![],
            },
        }
        .sign(&stranger);

        let accepted = accept_inbound(&stoa, &op.to_bytes())
            .expect("an unknown author's authentic op must be accepted");
        assert_eq!(accepted.op, op);
    }

    #[test]
    fn a_moderation_op_from_a_peer_with_no_authority_is_accepted_at_the_boundary() {
        // Authenticity is not authority. The boundary stores it; the resolver
        // decides. §3.3 puts the decision on read because it needs the
        // moderator set as of that op's Lamport time, which a boundary does not
        // have — and the op log's contract suite asserts the same thing one
        // layer down.
        //
        // Refusing here would be worse than useless: it would make a
        // moderation op that becomes authorised later, when the op establishing
        // the moderator arrives, unrecoverable.
        let stoa = a_stoa("Agora");
        let random_peer = a_key(9);
        let op = Op {
            stoa,
            author: random_peer.public_key(),
            kind: OpKind::Moderate {
                target: OpId::from_hex(&"7a".repeat(32)).unwrap(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&random_peer);

        let accepted = accept_inbound(&stoa, &op.to_bytes())
            .expect("an unauthorised moderation is authentic and must reach the store");
        assert_eq!(accepted.op, op);
    }

    #[test]
    fn an_op_naming_a_target_this_peer_does_not_hold_is_accepted() {
        // The dangling-target case, which §3.3 makes ordinary rather than
        // suspicious: two peers routinely hold different op sets, so a revision
        // arriving before the post it revises is expected traffic. A boundary
        // requiring the target to be present would drop exactly the ops
        // out-of-order delivery produces and never recover them.
        let stoa = a_stoa("Agora");
        let key = a_key(2);
        let never_received = OpId::from_hex(&"cc".repeat(32)).unwrap();
        let op = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Revise {
                target: never_received,
                body: "revises something we lack".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        assert!(accept_inbound(&stoa, &op.to_bytes()).is_ok());
    }

    // ─── Every refusal is distinguishable ─────────────────────────────────

    #[test]
    fn every_hostile_case_produces_a_distinct_refusal() {
        // The requirement stated as one property over the whole set: four
        // different attacks must not collapse into one message. A collapsed
        // pair means one of the two is being reported wrongly, and a wrong
        // reason sends an operator to investigate something that is not
        // happening.
        //
        // Compared by DISPLAY STRING rather than by variant, because the
        // variant distinction is already visible in the type — what a caller
        // actually receives through §2.5's error shape is the rendered message,
        // and two variants rendering identically would be indistinguishable
        // where it counts.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let attacker = a_key(3);
        let victim = a_key(2);

        let genuine = a_signed_post(agora, 2, "genuine");
        let forged = {
            let op = Op {
                stoa: agora,
                author: victim.public_key(),
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "forged".to_string(),
                    attachments: vec![],
                },
            };
            SignedOp {
                signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
                op,
            }
        };

        // One case per DISTINCT fact about a payload. `empty` is deliberately
        // not here: it is a truncation, so it renders as one — see
        // `an_empty_payload_and_a_truncated_one_are_the_same_fact`, which pins
        // that agreement rather than leaving it as an omission here.
        let mut too_long = outbound_payload(&genuine);
        let body_len_at = 68;
        too_long[body_len_at..body_len_at + 4].copy_from_slice(&u32::MAX.to_be_bytes());

        let mut trailing = outbound_payload(&genuine);
        trailing.push(0);

        let mut wrong_version = outbound_payload(&genuine);
        wrong_version[0] = 0x7F;

        let cases: [(&str, Address, Vec<u8>); 7] = [
            ("oversized", agora, vec![0u8; MAX_PAYLOAD_BYTES + 1]),
            ("truncated", agora, outbound_payload(&genuine)[..10].to_vec()),
            ("trailing bytes", agora, trailing),
            ("an over-long field", agora, too_long),
            ("an unknown version", agora, wrong_version),
            ("forged", agora, forged.to_bytes()),
            ("replayed", lyceum, outbound_payload(&genuine)),
        ];

        let mut seen: Vec<(String, String)> = Vec::new();
        for (name, stoa, payload) in cases {
            let err = match accept_inbound(&stoa, &payload) {
                Err(e) => e.to_string(),
                Ok(_) => panic!("{name} must be refused"),
            };
            if let Some((other, _)) = seen.iter().find(|(_, e)| *e == err) {
                panic!("{name} and {other} produce the same refusal: {err}");
            }
            seen.push((name.to_string(), err));
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn an_empty_payload_and_a_truncated_one_are_the_same_fact() {
        // Pinned as a deliberate agreement rather than left as a gap in
        // `every_hostile_case_produces_a_distinct_refusal`.
        //
        // An empty payload IS a truncation — it ran out before the signature —
        // so reporting it as anything else would be inventing a distinction the
        // input does not carry. This is the one place where two hostile inputs
        // legitimately render alike, and it is here so that a reader who
        // notices the absence upstream finds the reasoning rather than assuming
        // an oversight.
        //
        // Distinguishability is a requirement about DIFFERENT mistakes. Two
        // spellings of one mistake are one mistake.
        let stoa = a_stoa("Agora");
        let truncated = accept_inbound(&stoa, &[0x01, 0x00]).unwrap_err();
        let empty = accept_inbound(&stoa, &[]).unwrap_err();
        assert_eq!(empty, truncated);
        assert_eq!(empty, InboundError::Malformed(OpError::Truncated));
    }

    #[test]
    fn a_refusal_names_the_numbers_and_addresses_needed_to_investigate_it() {
        // "Too large" with no size, or "wrong Stoa" with no addresses, is a
        // message that cannot be acted on. Each carries its operands, and the
        // rendered form is what a caller sees — so the assertion is on the
        // rendered form.
        let oversized = InboundError::TooLarge {
            bytes: 200_000,
            cap: MAX_PAYLOAD_BYTES,
        }
        .to_string();
        assert!(oversized.contains("200000"), "got {oversized}");
        assert!(oversized.contains("153600"), "got {oversized}");

        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let replay = InboundError::WrongStoa {
            expected: lyceum,
            found: agora,
        }
        .to_string();
        assert!(replay.contains(&agora.to_hex()), "got {replay}");
        assert!(replay.contains(&lyceum.to_hex()), "got {replay}");
    }

    #[test]
    fn a_malformed_refusal_carries_the_decoders_own_distinction_through() {
        // The boundary must not flatten `OpError`. Each of these is a different
        // fact about the payload, and the rendered message has to differ or the
        // detail was lost on the way out.
        let mut seen: Vec<String> = Vec::new();
        for e in [
            OpError::UnknownVersion(9),
            OpError::UnknownKind(9),
            OpError::Truncated,
            OpError::TrailingBytes,
            OpError::LengthMismatch,
            OpError::FieldTooLong(999_999),
            OpError::InvalidText,
        ] {
            let rendered = InboundError::Malformed(e).to_string();
            assert!(
                !seen.contains(&rendered),
                "two decoder errors render alike: {rendered}"
            );
            seen.push(rendered);
        }
        assert_eq!(seen.len(), 7);
    }

    // ─── The boundary never panics ────────────────────────────────────────

    #[test]
    fn no_hostile_payload_makes_the_boundary_panic() {
        // PHASE0-FINDINGS §3: a panic ABORTS the module process, the caller
        // waits out a 20s timeout, and every later call reports
        // MODULE_NOT_LOADED. So a payload that panics here is a remote denial
        // of service against the peer that received it, reachable by anyone who
        // can publish on the channel.
        //
        // This is the one test in the module that asserts nothing about the
        // ANSWER. Any `Ok` and any `Err` are acceptable; only an unwind is not.
        let stoa = a_stoa("Agora");
        let genuine = outbound_payload(&a_signed_post(stoa, 2, "a body of some length"));

        let mut payloads: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![0xFF; 63],
            vec![0xFF; 64],
            vec![0xFF; 65],
            vec![0x01; 64],
            vec![0u8; MAX_PAYLOAD_BYTES + 1],
        ];
        // Every prefix of a genuine op: the truncation at every offset.
        for cut in 0..=genuine.len() {
            payloads.push(genuine[..cut].to_vec());
        }
        // Every single-byte mutation of a genuine op, which reaches every
        // discriminant, length prefix and signature byte in turn.
        for at in 0..genuine.len() {
            let mut m = genuine.clone();
            m[at] ^= 0xFF;
            payloads.push(m);
        }
        // Maximal length prefixes at every 4-byte-aligned offset, which is the
        // allocation-on-a-claim probe applied blindly rather than at one known
        // field.
        for at in (0..genuine.len().saturating_sub(4)).step_by(4) {
            let mut m = genuine.clone();
            m[at..at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
            payloads.push(m);
        }

        for payload in &payloads {
            // The result is deliberately discarded. What is under test is that
            // control reaches the next iteration.
            let _ = accept_inbound(&stoa, payload);
        }
        assert!(
            payloads.len() > 200,
            "the fixture must actually probe a range, got {}",
            payloads.len()
        );
    }

    // ─── The reply shapes ─────────────────────────────────────────────────

    #[test]
    fn the_publish_reply_names_the_op_it_published() {
        // A caller correlates a later `channelMessageSent` with the op it was
        // about, and §4.4 makes that correlation the only thing an ACK can
        // teach: "ACK means 'some participants received it'".
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "published");
        let out = published_json(&op.op);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["published"], true);
        assert_eq!(v["opId"], op.op.id().to_hex());
    }

    #[test]
    fn the_accept_reply_distinguishes_a_new_op_from_one_already_held() {
        // §3.1's idempotence surfaced rather than flattened. A duplicate is not
        // an error — retransmission and SDS-Repair both deliver ops a peer
        // already has — but a caller rebuilding a view wants to know which it
        // got.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "accepted");
        for stored in [true, false] {
            let v: serde_json::Value =
                serde_json::from_str(&accepted_json(&op.op, stored)).unwrap();
            assert_eq!(v["accepted"], true);
            assert_eq!(v["stored"], stored);
            assert_eq!(v["opId"], op.op.id().to_hex());
        }
    }

    #[test]
    fn the_reply_shapes_are_pinned_to_their_exact_key_names() {
        // A view is written against these exact names, and renaming one is a
        // breaking change no type checker would catch. Pinned as literals for
        // the reason `wire.rs` pins the capability shape.
        //
        // **The key ORDER here is serde_json's, not this module's**, and the
        // expected strings are written to match it rather than to match the
        // order the `json!` literal is spelled in. `serde_json` is built
        // without `preserve_order`, so its object is a `BTreeMap` and keys come
        // out sorted — `opId` before `published`, `accepted` before `opId`
        // before `stored`. Discovered by watching this test fail, which is the
        // useful outcome: had it been written the other way and passed, it would
        // have been pinning an assumption rather than the output.
        //
        // JSON object order is not semantically meaningful, so a view must not
        // depend on it. What this test pins is the key NAMES and the absence of
        // extras; the ordering is incidental and is recorded so the next reader
        // does not "fix" it.
        let stoa = Address::from_bytes([0u8; 32]);
        let key = a_key(2);
        let op = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "pinned".to_string(),
                attachments: vec![],
            },
        };
        let id = op.id().to_hex();
        assert_eq!(
            published_json(&op),
            format!(r#"{{"opId":"{id}","published":true}}"#)
        );
        assert_eq!(
            accepted_json(&op, true),
            format!(r#"{{"accepted":true,"opId":"{id}","stored":true}}"#)
        );
    }

    // ─── The bounded inbound queue ────────────────────────────────────────

    #[test]
    fn the_inbound_queue_holds_what_it_is_given_in_arrival_order() {
        // Oldest out first: the queue must not reorder, because a peer ingesting
        // in a different sequence than it received in would be introducing a
        // per-peer order — and `cmp_ops` exists precisely so that no per-peer
        // order reaches the store.
        //
        // (Arrival order is itself per-peer, which is why nothing downstream
        // records it. What matters here is only that the queue is a queue.)
        let mut q = InboundQueue::new();
        for n in 0u8..5 {
            assert_eq!(q.push(vec![n]), Pushed::Queued);
        }
        assert_eq!(q.len(), 5);
        for n in 0u8..5 {
            assert_eq!(q.pop(), Some(vec![n]));
        }
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
        assert_eq!(q.dropped(), 0, "nothing was dropped");
    }

    #[test]
    fn the_inbound_queue_never_grows_past_its_bound() {
        // THE property, and the one §2.3 assigns: "A consumer slower than the
        // event rate grows the queue until OOM [...] Bound it ourselves."
        //
        // Pushed well past the bound with nothing consuming, which is exactly
        // the flood shape — SDS has no membership, so anyone may publish as fast
        // as they like and the trampoline filling the queue cannot refuse.
        let mut q = InboundQueue::new();
        for n in 0..(MAX_QUEUED_PAYLOADS * 3) {
            q.push(vec![(n % 251) as u8]);
            assert!(
                q.len() <= MAX_QUEUED_PAYLOADS,
                "the queue reached {} entries, past its {MAX_QUEUED_PAYLOADS} bound",
                q.len()
            );
        }
        assert_eq!(q.len(), MAX_QUEUED_PAYLOADS, "the queue must stay full");
    }

    #[test]
    fn a_full_queue_drops_the_oldest_and_keeps_accepting_new_arrivals() {
        // The drop policy, and the reason for it: dropping the NEWEST would let
        // an attacker who saturates the queue once lock this peer out of the
        // Stoa until the backlog drained. Dropping the oldest keeps the peer
        // making progress, and what is lost is recoverable — SDS retransmits,
        // and §3.3 makes a partial set the normal case.
        //
        // So the assertion is not merely "the length is bounded" but "the NEW
        // payload is the one that survived", which is what distinguishes the two
        // policies. A drop-newest implementation passes the bound test above and
        // fails this one.
        let mut q = InboundQueue::new();
        for n in 0..MAX_QUEUED_PAYLOADS {
            q.push(vec![(n % 251) as u8]);
        }
        assert_eq!(q.dropped(), 0, "nothing dropped while filling");

        // One more, which must displace the oldest.
        let newest = vec![0xAA, 0xBB, 0xCC];
        assert_eq!(q.push(newest.clone()), Pushed::EvictedOldest);
        assert_eq!(q.dropped(), 1);
        assert_eq!(q.len(), MAX_QUEUED_PAYLOADS);

        // Drain, and the newest arrival must be present — at the back, since it
        // arrived last.
        let mut drained = Vec::new();
        while let Some(p) = q.pop() {
            drained.push(p);
        }
        assert_eq!(
            drained.last(),
            Some(&newest),
            "a full queue refused the new arrival instead of evicting the oldest"
        );
        assert_eq!(drained.len(), MAX_QUEUED_PAYLOADS);
    }

    #[test]
    fn an_oversized_payload_is_not_allowed_to_occupy_a_queue_slot() {
        // A payload over §4.4's cap cannot become an op however long it waits,
        // so queueing one hands an attacker a slot — and a slot is the memory
        // the bound exists to protect. Refused at the door and counted as the
        // drop it is.
        let mut q = InboundQueue::new();
        assert_eq!(
            q.push(vec![0u8; MAX_PAYLOAD_BYTES + 1]),
            Pushed::RefusedOversize
        );
        assert_eq!(q.len(), 0, "an oversized payload took a slot");
        assert_eq!(q.dropped(), 1, "the refusal must be counted");

        // A payload AT the cap is legitimate and is queued — the boundary is
        // inclusive, matching `accept_inbound`.
        assert_eq!(q.push(vec![0u8; MAX_PAYLOAD_BYTES]), Pushed::Queued);
        assert_eq!(q.len(), 1);
        assert_eq!(q.dropped(), 1, "a legitimate payload must not count as a drop");
    }

    #[test]
    fn every_drop_is_counted_so_shedding_load_is_never_silent() {
        // The whole difference between a bound and a leak. A peer discarding ops
        // with no record could not tell "this Stoa is quiet" from "I am
        // shedding load" — the same confusion §11.1 forbids at the read path,
        // arriving at the write path instead.
        //
        // The expected count is derived by hand rather than from the
        // implementation: MAX_QUEUED_PAYLOADS pushes fill the queue and drop
        // nothing; the next 10 each evict one; and 3 oversized pushes each drop
        // one without queueing. So 10 + 3 = 13.
        let mut q = InboundQueue::new();
        for n in 0..MAX_QUEUED_PAYLOADS {
            q.push(vec![(n % 251) as u8]);
        }
        for _ in 0..10 {
            q.push(vec![1]);
        }
        for _ in 0..3 {
            q.push(vec![0u8; MAX_PAYLOAD_BYTES + 1]);
        }
        assert_eq!(q.dropped(), 13);
        assert_eq!(q.len(), MAX_QUEUED_PAYLOADS);
    }

    #[test]
    fn the_drop_counter_does_not_reset_when_the_queue_drains() {
        // Monotonic, because the question it answers is "has this peer been
        // shedding load?" and a counter that returned to zero could not answer
        // it. A peer that shed 10,000 ops an hour ago and is idle now has still
        // shed them.
        let mut q = InboundQueue::new();
        for n in 0..(MAX_QUEUED_PAYLOADS + 5) {
            q.push(vec![(n % 251) as u8]);
        }
        assert_eq!(q.dropped(), 5);
        while q.pop().is_some() {}
        assert!(q.is_empty());
        assert_eq!(q.dropped(), 5, "draining the queue reset the drop counter");
    }

    #[test]
    fn the_queue_bound_is_pinned_to_a_stated_number() {
        // Pinned as a literal, like the payload cap, because every other test
        // here reads the bound from this same constant — so a bound that
        // silently drifted to 10 million would pass all of them while restoring
        // the OOM lever the bound exists to close.
        //
        // The worst case is stated in the same breath, since the number is only
        // meaningful with it: 1024 slots at §4.4's 150 KiB cap is a 150 MB
        // ceiling. Arguing with that number is the point of writing it down.
        assert_eq!(MAX_QUEUED_PAYLOADS, 1024);
        assert_eq!(MAX_QUEUED_PAYLOADS * MAX_PAYLOAD_BYTES, 157_286_400);
    }

    #[test]
    fn a_queued_payload_is_still_validated_when_it_is_ingested() {
        // The queue is a buffer and NOT a boundary: it bounds memory and
        // decides nothing about authenticity. A forgery that sat in the queue
        // must still be refused on the way out, or the bound would have become
        // an accidental bypass of the one hostile-input check.
        let stoa = a_stoa("Agora");
        let attacker = a_key(3);
        let op = Op {
            stoa,
            author: a_key(2).public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "queued forgery".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        let genuine = a_signed_post(stoa, 2, "queued and genuine");

        let mut q = InboundQueue::new();
        q.push(forged.to_bytes());
        q.push(outbound_payload(&genuine));

        let mut log = MemoryOpLog::new();
        let mut refused = 0;
        while let Some(payload) = q.pop() {
            if ingest(&mut log, &stoa, &payload).is_err() {
                refused += 1;
            }
        }
        assert_eq!(refused, 1, "the forgery must still be refused after queuing");
        assert_eq!(log.len().unwrap(), 1, "only the genuine op was recorded");
        assert!(log.get(&genuine.op.id()).unwrap().is_some());
    }

    // ─── The seam with the store ──────────────────────────────────────────

    use crate::log::{MemoryOpLog, OpLog};

    #[test]
    fn an_ingested_op_reaches_the_store_and_reads_back() {
        // The seam, end to end: bytes in on a channel, an op out of the log.
        // This is the property that makes the transport and the store one path
        // rather than two that happen to compile.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "from a peer");
        let mut log = MemoryOpLog::new();

        let ingested = ingest(&mut log, &stoa, &outbound_payload(&op)).unwrap();
        assert!(ingested.stored, "a new op must report as stored");
        assert_eq!(ingested.op, op);

        let entry = log.get(&op.op.id()).unwrap().expect("the op is in the log");
        assert_eq!(entry.op, op);
        assert_eq!(
            entry.op.to_bytes(),
            op.to_bytes(),
            "the op must reach the store byte-identical, or its signature dies"
        );
    }

    #[test]
    fn an_ingested_op_is_recorded_as_unordered_in_the_store() {
        // The arrival the boundary constructs must be the one that lands, not
        // one the store or the seam substituted. Asserted through the log rather
        // than on `InboundOp`, because the store is where a substitution would
        // actually hurt: `cmp_ops` reads this.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "unordered on arrival");
        let mut log = MemoryOpLog::new();
        ingest(&mut log, &stoa, &outbound_payload(&op)).unwrap();

        let entry = log.get(&op.op.id()).unwrap().unwrap();
        assert_eq!(entry.arrival.lamport(), None);
        assert_eq!(entry.arrival.message_id(), None);
        assert!(!entry.arrival.is_ordered_by_transport());
    }

    #[test]
    fn a_refused_payload_never_reaches_the_store() {
        // THE invariant the seam exists to hold: nothing reaches the store
        // without passing the boundary. A forgery, a replay, an oversized
        // payload and junk must each leave the log empty.
        //
        // This is what a caller calling `log.append` directly would break, and
        // it is why `ingest` exists as one function rather than as two steps at
        // each call site.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let genuine = a_signed_post(lyceum, 2, "belongs elsewhere");
        let attacker = a_key(3);
        let forged = {
            let op = Op {
                stoa: agora,
                author: a_key(2).public_key(),
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "forged".to_string(),
                    attachments: vec![],
                },
            };
            SignedOp {
                signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
                op,
            }
        };

        for (name, payload) in [
            ("a forgery", forged.to_bytes()),
            ("a replay", outbound_payload(&genuine)),
            ("junk", vec![0xFFu8; 200]),
            ("an oversized payload", vec![0u8; MAX_PAYLOAD_BYTES + 1]),
            ("nothing at all", vec![]),
        ] {
            let mut log = MemoryOpLog::new();
            match ingest(&mut log, &agora, &payload) {
                Err(IngestError::Refused(_)) => {}
                other => panic!("{name} was not refused: {other:?}"),
            }
            assert_eq!(
                log.len().unwrap(),
                0,
                "{name} reached the store despite being refused"
            );
        }
    }

    #[test]
    fn the_same_op_ingested_twice_is_one_entry_and_says_so() {
        // §3.1's idempotence through the seam. Retransmission and SDS-Repair
        // both deliver ops a peer already holds, so this is ordinary traffic —
        // and the second call must report `stored: false` rather than failing,
        // because a caller rebuilding a view acts on the difference.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "delivered twice");
        let payload = outbound_payload(&op);
        let mut log = MemoryOpLog::new();

        assert!(ingest(&mut log, &stoa, &payload).unwrap().stored);
        assert!(
            !ingest(&mut log, &stoa, &payload).unwrap().stored,
            "a re-delivery must report as already held"
        );
        assert_eq!(log.len().unwrap(), 1, "one op id is one entry");
    }

    #[test]
    fn a_stored_op_is_publishable_and_the_payload_is_what_a_peer_accepts() {
        // The outbound half of the seam, closing the loop: read an op out of
        // the store, frame it, and confirm a receiving peer accepts it.
        //
        // Together with `a_refused_payload_never_reaches_the_store` this is the
        // whole protocol claim — what one peer publishes, another records.
        let stoa = a_stoa("Agora");
        let op = a_signed_post(stoa, 2, "to be published");
        let mut log = MemoryOpLog::new();
        log.append(op.clone(), Arrival::unordered()).unwrap();

        let payload = outbound_for(&log, &stoa, &op.op.id())
            .unwrap()
            .expect("a stored op is publishable");

        // And the far side of the network accepts exactly these bytes.
        let mut theirs = MemoryOpLog::new();
        let ingested = ingest(&mut theirs, &stoa, &payload).unwrap();
        assert!(ingested.stored);
        assert_eq!(ingested.op, op, "the op did not survive the hop");
    }

    #[test]
    fn publishing_an_op_the_log_does_not_hold_is_an_absence_and_not_an_error() {
        // `OpLog::get`'s nesting, preserved through the seam: the op may simply
        // not be here, which is different from the store being unreadable. A
        // caller must be able to tell "not mine to publish" from "my disk is
        // broken".
        let stoa = a_stoa("Agora");
        let never_stored = a_signed_post(stoa, 2, "never stored");
        let log = MemoryOpLog::new();
        assert_eq!(
            outbound_for(&log, &stoa, &never_stored.op.id()).unwrap(),
            None
        );
    }

    #[test]
    fn publishing_an_op_to_another_stoas_channel_is_refused_locally() {
        // The check exists to fail HERE rather than on every peer in the
        // network. Without it, a caller could put the Agora's op on the
        // Lyceum's channel; every receiver would refuse it as `WrongStoa`, and
        // the publishing peer would see nothing but silence.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let op = a_signed_post(agora, 2, "belongs to the agora");
        let mut log = MemoryOpLog::new();
        log.append(op.clone(), Arrival::unordered()).unwrap();

        assert_eq!(
            outbound_for(&log, &lyceum, &op.op.id()),
            Err(OutboundError::WrongStoa {
                expected: lyceum,
                found: agora,
            })
        );
        // And it IS publishable to its own channel, so the refusal above is
        // about the pairing and not about the op.
        assert!(outbound_for(&log, &agora, &op.op.id()).unwrap().is_some());
    }

    #[test]
    fn a_forgery_already_in_the_store_is_still_publishable() {
        // §3.3: "The store may hold junk; the reader never trusts it." The op
        // log deliberately stores unverified ops, and the contract suite pins
        // that. So `outbound_for` must not re-verify — not because publishing a
        // forgery is desirable, but because the store's contract says a
        // forgery can be in there and a function that refused it would be
        // adding a filter at a second site.
        //
        // The receiving peer refuses it, which is where the check belongs and
        // is asserted here so the division of labour is visible rather than
        // assumed.
        let stoa = a_stoa("Agora");
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            stoa,
            author: victim.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "junk in the store".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify());

        let mut log = MemoryOpLog::new();
        log.append(forged.clone(), Arrival::unordered()).unwrap();

        let payload = outbound_for(&log, &stoa, &forged.op.id())
            .unwrap()
            .expect("the store does not filter, so this is publishable");

        // And the receiver is what stops it.
        let mut theirs = MemoryOpLog::new();
        assert_eq!(
            ingest(&mut theirs, &stoa, &payload),
            Err(IngestError::Refused(InboundError::Forged))
        );
        assert_eq!(theirs.len().unwrap(), 0);
    }

    #[test]
    fn a_storage_failure_is_distinguishable_from_a_refusal() {
        // The two mean opposite things — "the payload was bad" versus "this
        // peer is broken" — and collapsing them would make a failing disk look
        // like an attack. Asserted on the rendered messages, since §2.5 gives a
        // caller one error string.
        let refused = IngestError::Refused(InboundError::Forged).to_string();
        let broken =
            IngestError::Storage(crate::log::OpLogError::Storage("disk on fire".into()))
                .to_string();
        assert_ne!(refused, broken);
        assert!(broken.contains("disk on fire"), "got {broken}");
    }

    /// Silences the unused-import warning for `PublicKey`, which is named in
    /// this module's fixtures only through `SecretKey::public_key`.
    ///
    /// Kept as a use rather than removing the import, because a reader of the
    /// fixtures above reasonably looks for the type they return.
    #[allow(dead_code)]
    fn _public_key_is_named(k: &SecretKey) -> PublicKey {
        k.public_key()
    }

    #[test]
    fn the_stoa_address_helper_agrees_with_the_genesis_record() {
        // A fixture check, not a behaviour: every test above derives its Stoa
        // from a genesis record, and `stoa_address` is what that derivation
        // reduces to. Pinned so that a fixture drifting from the real
        // derivation would fail here rather than making every other test in the
        // module assert about an address no Stoa has.
        let genesis = a_genesis("Agora");
        assert_eq!(
            a_stoa("Agora"),
            stoa_address(&genesis.canonical_bytes().unwrap())
        );
    }
}
