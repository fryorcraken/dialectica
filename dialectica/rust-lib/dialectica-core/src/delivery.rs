//! The adapter half of `op-transport`: the node, a reliable channel per Stoa,
//! sends, and the listener's path to the inbound boundary.
//!
//! # Why this is here and not in the adapter
//!
//! `modules().delivery_module` cannot appear in this crate — it calls `lp_*`
//! symbols undefined in a test binary — and `cargo test` does not compile the
//! file that can call it. So every decision about delivery lives here, behind
//! [`Delivery`], a four-method seam the adapter implements with four one-line
//! calls. What is left in the adapter is those calls and the event subscription;
//! everything a test can see is on this side.
//!
//! # Four threads, and why the reply is never one of them
//!
//! - **Dispatch** (the module's event loop) only *enqueues*: a publish, create or
//!   join hands [`Delivering`] an op id or a Stoa and returns. `content-authoring`
//!   forbids a reply to wait on delivery, and a synchronous call into another
//!   process would hold it for the whole IPC timeout against an unresponsive
//!   delivery — long enough for the view to time out and render a stored op as a
//!   failure.
//! - **The worker** makes every delivery call, one at a time, in the order they
//!   were enqueued. One FIFO consumer is what makes "sends are made in the order
//!   their publishes were answered" and "a send after an open is made after that
//!   open is answered" true by construction rather than by coordination.
//! - **The listener** takes `channelMessageReceived` events, refuses one on a
//!   channel this peer is neither holding nor opening, or one over the message
//!   limit, and offers the rest to the bounded [`InboundQueue`] without waiting
//!   on the boundary.
//! - **The processor** takes them off in arrival order and puts each through the
//!   inbound boundary, [`crate::transport::receive_via`] — or, when its channel
//!   is being opened, **parks** it ([`crate::parked`]) and goes straight on. It
//!   also runs the **reviews** that decide parked messages once their channel's
//!   open settles. It never waits on an open.
//!
//! # The three seams parking turns on
//!
//! `op-transport`'s parking requirements come down to three questions, and each
//! is answered in exactly one place:
//!
//! - **Is this payload parked?** [`ChannelBook::on_take`], asked once per payload
//!   as the processor takes it.
//! - **Does this event begin a review?** [`ChannelBook::settle`] for delivery's
//!   answer and for an open given up, [`ChannelBook::startup_review`] for the
//!   module's first startup. Nothing else makes a [`Review`].
//! - **Is every message decided once, in order?** [`Channels::take`]: the channel
//!   book, the waiting payloads and the waiting reviews share one lock, and a
//!   waiting review is always taken before a waiting payload.
//!   [`ParkedStore::take_channel`](crate::parked::ParkedStore::take_channel)
//!   removes what a review decides in the same transaction that reads it.
//!
//! `design.md` (the `park-pending-inbound` change) says why each is shaped so.
//!
//! # Nothing here may unwind
//!
//! A panic on a dispatch thread aborts the module process (`PHASE0-FINDINGS`
//! §3), and a panic on the worker or the processor would silently end delivery.
//! Each action, each event the listener reads, each message decided or parked
//! and each parked message a review decides is run under its own
//! `catch_unwind`, and a panic is logged. A channel open is settled by a `Drop`
//! guard, so a panic cannot leave it being opened — and its parked messages
//! unreviewed — for the rest of the process.

use crate::identity::Address;
use crate::log::{Appended, OpLog, OpLogError, SqliteOpLog};
use crate::membership::{membership_path_in, MembershipError, MembershipStore};
use crate::op::OpId;
#[cfg(test)]
use crate::parked::ParkBounds;
use crate::parked::{parked_path_in, ParkError, ParkOutcome, Parked, ParkedStore, PARK_BOUNDS};
use crate::sender::{sender_path_in, SenderError, SenderStore};
use crate::shedding::{choose, Victim};
use crate::transport::{
    self, ChannelIdentity, InboundMessage, InboundRefusal, OpenChannels, PublishError,
};
use crate::wire::panic_detail;
use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

// ─── The seam ─────────────────────────────────────────────────────────────

/// The four delivery calls this application makes, and no others.
///
/// **There is no `stop`**, and that is `op-transport`'s prohibition made
/// structural on this side: the worker cannot stop a node because the seam it
/// holds has no way to ask. The adapter could still call it directly;
/// `the_adapter_never_stops_a_node_and_creates_one_at_one_site` reads the adapter
/// for that.
///
/// Each returns delivery's reply as the generated client hands it back, with a
/// transport failure as `Err(reason)`. What counts as "delivery declined" is
/// decided once, in [`declined`], rather than by each implementation.
pub trait Delivery: Send + 'static {
    /// `createNode(cfg)`.
    fn create_node(&self, config: &str) -> Result<serde_json::Value, String>;
    /// `start()`.
    fn start_node(&self) -> Result<serde_json::Value, String>;
    /// `channelCreate(channelId, contentTopic, senderId)`.
    fn channel_create(
        &self,
        channel_id: &str,
        content_topic: &str,
        sender_id: &str,
    ) -> Result<serde_json::Value, String>;
    /// `channelSend(channelId, payload)`.
    fn channel_send(&self, channel_id: &str, payload: &[u8]) -> Result<serde_json::Value, String>;
}

/// How long one delivery call may take before this peer stops waiting.
///
/// **Longer than delivery's own 30 s callback timeout, on purpose.** delivery
/// v0.2.1 waits up to `CALLBACK_TIMEOUT{30}` for its runtime on every channel
/// call and then answers. The IPC default is 20 s. With the default, a
/// `channelCreate` delivery completed at 25 s would be recorded here as not
/// answered — not open — while delivery holds it open and its messages arrive,
/// each refused as an unknown channel. Waiting past delivery's own bound means
/// delivery's answer, not this peer's impatience, decides.
///
/// Nothing waits on this but the worker thread, so its length costs no reply,
/// and no inbound message waits on it either: a message on a channel being
/// opened is parked. That it outlasts delivery's 30 s is checked at compile time
/// beside [`DELIVERY_CALLBACK_TIMEOUT`], and against delivery's 30 s itself by
/// `this_peers_wait_on_a_creation_outlasts_deliverys_own`.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(35);

/// delivery v0.2.1's own callback timeout, `CALLBACK_TIMEOUT{30}` in
/// `delivery_module_plugin.h`: how long delivery waits on its runtime before it
/// answers a channel call. Named only so the relation below can be checked.
const DELIVERY_CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);

// `op-transport`: "The longest this peer waits for delivery to answer one
// channel creation MUST be longer than the time delivery allows itself to answer
// one" — or an answer delivery gives at 25 s is recorded here as none, and the
// channel's parked messages are refused while delivery holds it. This holds the
// two constants against each other, so a shortened `CALL_TIMEOUT` fails to
// compile. It cannot hold delivery's real 30 s: lowering
// `DELIVERY_CALLBACK_TIMEOUT` along with it compiles, which
// `this_peers_wait_on_a_creation_outlasts_deliverys_own` catches against a
// literal.
const _: () = assert!(DELIVERY_CALLBACK_TIMEOUT.as_millis() < CALL_TIMEOUT.as_millis());

/// The configuration handed to `createNode`.
///
/// - **`entryLayer: "channels"`, named** rather than left to delivery's default,
///   as `op-transport` requires: every Stoa's ops travel on a reliable channel,
///   and a node without that layer refuses every channel call.
/// - **`preset: "logos.test"`**: the Logos Test Network, cluster 2. It is the
///   network whose 150 KiB maximum message size
///   [`crate::transport::MAX_MESSAGE_BYTES`] pins, and the one #30 used.
///   `logos.dev` is cluster 3 with the transport's default size (also 150 KiB)
///   — both read at `logos-delivery` `bfdb5afd`, `networks_config.nim`, the rev
///   delivery v0.2.1 pins. Peers on two clusters are on two networks, so this
///   value is part of the interop contract in practice even though the spec
///   leaves it to design.
/// - **`mode: "Edge"`**: a light node. It does not relay other peers' traffic;
///   it publishes and receives through the preset's service nodes. `Core` would
///   make every dialectica peer a relay, contributing bandwidth and not depending
///   on the fleet. `delivery-wiring`'s design records the choice and what would
///   reverse it.
pub fn node_config() -> String {
    serde_json::json!({
        "entryLayer": "channels",
        "preset": "logos.test",
        "mode": "Edge",
    })
    .to_string()
}

/// Delivery's reason for declining, when it declined.
///
/// Three shapes are a decline, and **the envelope is read before the value**
/// (`callee_error` first, as `channel_exists_reply` does), because delivery
/// answers `Ok` at the IPC level with an error envelope in the body — observed
/// live as `{"error":"Context not initialized","success":false,"value":null}`:
///
/// - a transport failure (`Err`): timeout, provider unavailable;
/// - an object whose `error` is a non-empty string;
/// - an object whose `success` is `false`, with or without a reason.
///
/// Anything else is delivery reporting it did the thing.
///
/// **An empty `error` is no reason.** `StdLogosResult.error` is a `std::string`
/// defaulting to `""`, and logos-cpp-sdk's `lpPushExpr` serialises it verbatim,
/// so a success can arrive as `{"success":true,"value":…,"error":""}`. Counted as
/// a reason, every channel would be declined and every send reported failed.
/// `an_empty_error_string_is_not_a_reason_to_decline` is red without the filter.
pub fn declined(reply: &Result<serde_json::Value, String>) -> Option<String> {
    let value = match reply {
        Err(reason) => return Some(reason.clone()),
        Ok(v) => v,
    };
    if let Some(reason) = crate::wire::callee_error(value).filter(|r| !r.is_empty()) {
        return Some(reason.to_string());
    }
    let failed = value
        .as_object()
        .and_then(|o| o.get("success"))
        .and_then(serde_json::Value::as_bool)
        == Some(false);
    failed.then(|| "delivery reported failure and gave no reason".to_string())
}

/// What delivery's answer to `channelCreate` says about the channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelAnswer {
    /// Delivery created it.
    Created,
    /// Delivery already held it: it answered that the channel already exists.
    AlreadyHeld,
    /// Delivery does not hold it, for this reason.
    Declined(String),
}

/// Read delivery's answer to `channelCreate`.
///
/// # "Already exists" is delivery holding the channel, not declining it
///
/// `stoa-membership` requires that answer open the channel exactly as a report
/// of creation does. Delivery gives it in two cases this peer really meets: its
/// own 30 s callback gave up on a creation that its runtime then completed, and
/// this module restarted while delivery kept running. Read as a decline, either
/// left the Stoa's channel shut for as long as delivery ran — messages refused as
/// an unknown channel, sends never made.
///
/// Recognised by [`ALREADY_EXISTS`] in delivery's reason. `delivery-wiring`'s
/// design, Decision 14, says why the wording and not a `channelExists`
/// confirmation, and what a change to that wording would cost.
pub fn channel_answer(reply: &Result<serde_json::Value, String>) -> ChannelAnswer {
    match declined(reply) {
        None => ChannelAnswer::Created,
        Some(why) if why.contains(ALREADY_EXISTS) => ChannelAnswer::AlreadyHeld,
        Some(why) => ChannelAnswer::Declined(why),
    }
}

/// The words delivery answers a `channelCreate` with when its manager already
/// holds the channel: `logos-delivery` `channel_lifecycle.nim`,
/// `err("channel already exists: " & channelId)`, which delivery v0.2.1 passes
/// through behind a `"ChannelCreate failed: "` prefix — read at `bfdb5afd`, the
/// `logos-delivery` rev v0.2.1's `flake.lock` pins. A test-only constant,
/// `transport::delivery_topic_rule::DELIVERY_REV`, names that rev, and a test
/// beside it fails when `dialectica/flake.lock` locks delivery anywhere else.
///
/// Matched as a substring of the reason, not the whole reason: the prefix and
/// the trailing channel id are the C API's and the manager's, and neither is
/// what says the channel is held.
pub const ALREADY_EXISTS: &str = "channel already exists";

// ─── The module's log ─────────────────────────────────────────────────────

/// Where this module's log lines go.
///
/// A seam rather than `eprintln!` at each site because the spec contracts what
/// the log **says** — a refusal by kind, a discard with its running count — and
/// what it must **not** say: the payload, the sender identifier, a channel id
/// this peer did not open. Those are properties a test can only check if it can
/// read the lines.
pub trait Journal: Send + Sync + 'static {
    fn record(&self, line: &str);
}

/// The module's stderr, which the host collects as the module's log.
pub struct Stderr;

impl Journal for Stderr {
    fn record(&self, line: &str) {
        eprintln!("{line}");
    }
}

/// Every line this module logs about delivery, worded once.
///
/// **What no line carries**, because the sender chose it: an arriving payload, an
/// arriving sender identifier, or the channel id a message arrived on. A refusal
/// is logged by kind and nothing else; the one detail kept is a storage error,
/// which is this peer's own text.
enum Note<'a> {
    NodeRequested,
    NodeCreationDeclined(&'a str),
    NodeStartDeclined(&'a str),
    ChannelOpened(&'a Address),
    ChannelAlreadyHeld(&'a Address),
    ChannelDeclined(&'a Address, &'a str),
    SenderNotRetained(&'a Address, &'a SenderError),
    Sent(&'a OpId, &'a Address),
    SendDeclined(&'a OpId, &'a str),
    NotSent(&'a OpId, &'a Address),
    NotHandedOff(&'a OpId, &'a str),
    Stored(&'a OpId),
    AlreadyStored(&'a OpId),
    Refused(&'static str, Option<&'a str>),
    Discarded { total: u64, bound: usize },
    Parked,
    ParkDiscarded { total: u64 },
    NotParked(&'a str),
    ReviewUnreadable(&'a str),
    Unreadable,
    MembershipUnreadable(&'a MembershipError),
    NotSubscribed(&'a str),
    ListenerEnded,
    AlreadyStarted,
    NotStarted(&'a str),
    NoWorker(&'a str),
    WorkerGone(&'a str),
    ThreadNotStarted(&'static str, &'a str),
    Panicked(&'static str, &'a str),
}

impl std::fmt::Display for Note<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::NodeRequested => write!(
                f,
                "dialectica: delivery node created; start requested ({})",
                node_config()
            ),
            Note::NodeCreationDeclined(why) => write!(
                f,
                "dialectica: delivery declined node creation: {why}; channels are still \
                 requested, because a node another module created serves them too"
            ),
            Note::NodeStartDeclined(why) => {
                write!(f, "dialectica: delivery declined node start: {why}")
            }
            Note::ChannelOpened(stoa) => {
                write!(f, "dialectica: channel open for Stoa {}", stoa.to_hex())
            }
            Note::ChannelAlreadyHeld(stoa) => write!(
                f,
                "dialectica: channel open for Stoa {}: delivery already held it",
                stoa.to_hex()
            ),
            Note::ChannelDeclined(stoa, why) => write!(
                f,
                "dialectica: channel NOT open for Stoa {}: {why}; it is requested again at \
                 the next start or the next create or join of that Stoa",
                stoa.to_hex()
            ),
            Note::SenderNotRetained(stoa, why) => write!(
                f,
                "dialectica: channel not requested for Stoa {}: no sender identifier could \
                 be retained ({why})",
                stoa.to_hex()
            ),
            Note::Sent(op, stoa) => write!(
                f,
                "dialectica: op {} handed to the channel for Stoa {}",
                op.to_hex(),
                stoa.to_hex()
            ),
            Note::SendDeclined(op, why) => write!(
                f,
                "dialectica: delivery did not take op {}: {why}; the op is published and \
                 stays in the log",
                op.to_hex()
            ),
            Note::NotSent(op, stoa) => write!(
                f,
                "dialectica: op {} not sent: no channel is open for Stoa {}; the op is \
                 published and stays in the log",
                op.to_hex(),
                stoa.to_hex()
            ),
            Note::NotHandedOff(op, why) => write!(
                f,
                "dialectica: op {} not sent: {why}; the op is published",
                op.to_hex()
            ),
            Note::Stored(op) => write!(f, "dialectica: stored inbound op {}", op.to_hex()),
            Note::AlreadyStored(op) => write!(
                f,
                "dialectica: inbound op {} already held; nothing new was stored",
                op.to_hex()
            ),
            Note::Refused(kind, None) => write!(
                f,
                "dialectica: inbound message refused ({kind}); nothing was stored"
            ),
            Note::Refused(kind, Some(detail)) => write!(
                f,
                "dialectica: inbound message refused ({kind}: {detail}); nothing was stored"
            ),
            Note::Discarded { total, bound } => write!(
                f,
                "dialectica: inbound message discarded unread, queue full at \
                 {bound}; {total} discarded since the module started"
            ),
            // No channel identifier, though the channel is one this peer asked
            // for: a park says only that the boundary moved on, and why.
            Note::Parked => write!(
                f,
                "dialectica: inbound message parked until its channel's open settles"
            ),
            Note::ParkDiscarded { total } => write!(
                f,
                "dialectica: inbound message discarded from the parked messages, over a \
                 parking bound; {total} discarded since the module started"
            ),
            Note::NotParked(why) => write!(
                f,
                "dialectica: inbound message dropped, not parked (storage: {why}); it is \
                 not held for another attempt"
            ),
            Note::ReviewUnreadable(why) => write!(
                f,
                "dialectica: parked messages could not be read for review (storage: \
                 {why}); they stay parked for their channel's next review"
            ),
            Note::Unreadable => write!(
                f,
                "dialectica: inbound delivery event discarded: its fields could not be read"
            ),
            Note::MembershipUnreadable(why) => write!(
                f,
                "dialectica: no channel requested at start: the Stoas this peer is in could \
                 not be read ({why})"
            ),
            Note::NotSubscribed(why) => write!(
                f,
                "dialectica: could not subscribe to channelMessageReceived ({why}); this \
                 peer will NOT receive ops from other peers"
            ),
            Note::ListenerEnded => write!(
                f,
                "dialectica: the inbound listener has ended — delivery went away, and this \
                 peer no longer receives ops"
            ),
            Note::AlreadyStarted => write!(
                f,
                "dialectica: delivery wiring already started in this process; nothing \
                 requested again"
            ),
            Note::NotStarted(what) => write!(
                f,
                "dialectica: delivery wiring has not started, so {what} was not requested"
            ),
            Note::NoWorker(what) => write!(
                f,
                "dialectica: the delivery worker could not be started, so {what} was not \
                 requested"
            ),
            Note::WorkerGone(what) => write!(
                f,
                "dialectica: the delivery worker has stopped, so {what} was not requested"
            ),
            Note::ThreadNotStarted(which, why) => {
                write!(f, "dialectica: could not start the {which} thread: {why}")
            }
            Note::Panicked(what, detail) => {
                write!(f, "dialectica: {what} panicked and was contained: {detail}")
            }
        }
    }
}

fn record(journal: &dyn Journal, note: Note<'_>) {
    journal.record(&note.to_string());
}

/// The name a refusal is logged under.
///
/// Exhaustive with no wildcard, so a new refusal forces a name here rather than
/// being logged under one of these.
fn refusal_kind(refusal: &InboundRefusal) -> &'static str {
    match refusal {
        InboundRefusal::UnknownChannel => "unknown-channel",
        InboundRefusal::TooLong { .. } => "too-long",
        InboundRefusal::Undecodable(_) => "undecodable",
        InboundRefusal::FailsVerification => "fails-verification",
        InboundRefusal::StoaMismatch { .. } => "stoa-mismatch",
        InboundRefusal::AheadOfTime { .. } => "ahead-of-time",
        InboundRefusal::Storage(_) => "storage",
    }
}

/// A mutex's guard, poisoned or not.
///
/// Poisoning means a thread panicked while holding the lock, and every panic
/// here is already contained and logged. The data is a set of channel ids or a
/// queue of messages, both still coherent after any single statement, so taking
/// the guard back is the answer that keeps delivery running; `unwrap` would turn
/// one contained panic into a second, uncontained one.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

// ─── The stores ───────────────────────────────────────────────────────────

/// The four files delivery's code opens, all in the host's directory: the op log
/// (worker and processor), the sender identifiers (worker only), the
/// memberships (dispatch, once, at start), and the parked messages (processor
/// only).
///
/// Which threads open which file is what makes each safe: `delivery-wiring`'s
/// design, Decision 13.
///
/// A path and not open handles: a worker or processor opens what it needs per
/// action, for the reason the adapter opens per call — a handle held for the
/// module's lifetime would have to answer what happens when it goes stale.
#[derive(Clone, Debug)]
pub struct Stores {
    dir: PathBuf,
}

impl Stores {
    pub fn in_dir(dir: PathBuf) -> Self {
        Stores { dir }
    }

    fn op_log(&self) -> Result<SqliteOpLog, OpLogError> {
        SqliteOpLog::open(&crate::log::op_log_path_in(&self.dir))
    }

    fn senders(&self) -> Result<SenderStore, SenderError> {
        SenderStore::open(&sender_path_in(&self.dir))
    }

    fn parked(&self) -> Result<ParkedStore, ParkError> {
        ParkedStore::open(&parked_path_in(&self.dir))
    }

    /// Every Stoa the peer is in, from the membership record and nothing else.
    ///
    /// **Not from the op log.** A Stoa this peer holds ops for is not a Stoa it
    /// chose to be in; `stoa-membership` forbids opening a channel for one.
    fn memberships(&self) -> Result<Vec<Address>, MembershipError> {
        let store = MembershipStore::open(&membership_path_in(&self.dir))?;
        let mut stoas = Vec::new();
        let mut page = 0;
        loop {
            let listed = store.list(page, MEMBERSHIP_PAGE)?;
            stoas.extend(listed.items.iter().map(|m| m.stoa));
            if !listed.has_more {
                return Ok(stoas);
            }
            page += 1;
        }
    }
}

/// How many memberships are read per page at start. A read size, not a limit:
/// the loop pages to the end.
const MEMBERSHIP_PAGE: usize = 100;

// ─── Which channels are open, and what waits on the processor ─────────────

/// The channels this peer has open, and the ones it has asked for and not heard
/// back about.
///
/// # Being opened is its own state, because of a race delivery really has
///
/// A channel counts as open only once delivery answers that it holds it —
/// created it, or already had it ([`channel_answer`]). But delivery v0.2.1 emits
/// `channelMessageReceived` from its runtime's callback thread whenever SDS hands
/// it one — including after the runtime has created the channel and before the
/// `channelCreate` answer has reached this peer. A message judged in that gap is
/// refused as arriving on an unknown channel, and SDS has already treated it as
/// delivered, so it is gone for this peer.
///
/// So a message taken on a channel that is **not open but being opened** is
/// parked ([`ChannelBook::on_take`]), and decided by a review once the open
/// settles ([`ChannelBook::settle`]).
///
/// # Being opened from the request, not from the call
///
/// An open counts from the moment a create, a join or startup asks for it —
/// while it still waits in the worker's queue behind node creation and other
/// opens — because delivery can hand over a message on a channel before this peer
/// has asked delivery for it at all: a module restarted under a running delivery
/// is handed its Stoas' messages as soon as it subscribes. The [`Opening`] guard
/// is made where the request is made and travels to the worker inside
/// [`Action::Open`], so every path that never reaches delivery — no worker, a
/// worker gone, no sender identifier — settles the open by dropping it, and its
/// parked messages are reviewed then rather than left for the next startup.
/// `a_message_on_a_channel_whose_open_waits_behind_another_is_judged_once_that_open_settles`
/// and `a_restarted_peer_keeps_what_delivery_hands_over_before_startup_asks_for_its_channel`
/// are red while the worker made the guard as it called delivery.
#[derive(Default)]
struct ChannelBook {
    open: OpenChannels,
    /// Each channel being opened, with how many requests for it have not
    /// settled. A count rather than a flag, because a repeated join can put a
    /// second open in the queue before the first is answered, and the channel
    /// is being opened until the last of them settles.
    opening: HashMap<String, usize>,
}

/// What the boundary does with a payload it has taken: [`ChannelBook::on_take`]'s
/// answer.
///
/// # The one reading carries the Stoa
///
/// `op-transport`: "The state is read once for each payload, and the payload is
/// parked or judged on that one reading." So `Judge` carries the Stoa the
/// channel was open for at that reading — `None` when it was not open — and the
/// boundary judges against that, not against a second lookup made after the
/// lock is released. A settle landing between the take and the judgement would
/// otherwise have the two readings disagree: taken as unknown, then stored.
/// `a_payload_taken_on_an_unknown_channel_is_refused_though_its_channel_opens_before_it_is_judged`
/// is red with the lookup made at judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Taken {
    /// Put it through the boundary now, against this Stoa. With `None` — a
    /// channel neither open nor being opened — the boundary refuses it as an
    /// unknown channel.
    Judge(Option<Address>),
    /// Its channel is being opened and is not open: park it, judging nothing.
    Park,
}

/// A review of parked messages, named for the event that begins it.
///
/// Made in exactly two places, [`ChannelBook::settle`] and
/// [`ChannelBook::startup_review`]. The processor runs it ([`Processor::review`]).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Review {
    /// Delivery reported that it holds this channel: judge each message parked
    /// on it, as a message taken on an open channel is judged.
    Held(String),
    /// The last unsettled request for this channel settled some other way, and
    /// it is not open: refuse each message parked on it as an unknown channel.
    Unopened(String),
    /// The module's first startup in this process: refuse each message parked on
    /// a channel not in this set — the channels open or being opened then — as
    /// an unknown channel, and leave the rest for their channel's next review.
    Startup(HashSet<String>),
}

impl ChannelBook {
    /// Whether a channel is open or being opened: what hand-over asks before a
    /// message may take a place among the waiting payloads.
    fn is_known(&self, channel_id: &str) -> bool {
        self.open.is_open(channel_id) || self.opening.contains_key(channel_id)
    }

    /// **Whether a payload just taken is parked** — the one place that decides
    /// it, asked once per payload, under the lock it was taken under.
    ///
    /// `op-transport`, "A message on a channel being opened is parked, and nothing
    /// waits on an open": open, whether or not a further request is unsettled →
    /// judged; not open and being opened → parked; neither → judged, and refused
    /// as an unknown channel. Parked is the one state in which judging now would
    /// refuse a message the open's answer may yet admit.
    fn on_take(&self, channel_id: &str) -> Taken {
        match self.open.stoa_of(channel_id) {
            Some(stoa) => Taken::Judge(Some(*stoa)),
            None if self.opening.contains_key(channel_id) => Taken::Park,
            None => Taken::Judge(None),
        }
    }

    /// A create, a join or startup asked for this channel.
    fn request(&mut self, identity: &ChannelIdentity) {
        let requests = self
            .opening
            .entry(identity.channel_id().to_string())
            .or_insert(0);
        *requests = requests.saturating_add(1);
    }

    /// **One request for this channel settled; the review it begins, if any** —
    /// the one place a settle becomes a review.
    ///
    /// `op-transport`, "Parked messages are reviewed on three events and no
    /// others", events 1 and 2:
    ///
    /// - **Held**, in answer to any request: the channel is open, and its parked
    ///   messages are judged ([`Review::Held`]). Whatever else is unsettled for it.
    /// - **Not held, and the last unsettled request, and not open**: nothing will
    ///   open the channel now, and its parked messages are refused
    ///   ([`Review::Unopened`]).
    /// - **Anything else** begins no review: a decline while another request is
    ///   unsettled leaves the parked messages for that request's answer, and a
    ///   decline of a repeat for a channel already open finds nothing parked.
    ///
    /// A declined or given-up request never closes an open channel:
    /// `stoa-membership`, "A declined repeat request leaves an open channel
    /// open", because delivery created it once and nothing has closed it.
    fn settle(&mut self, identity: &ChannelIdentity, held: bool) -> Option<Review> {
        let channel_id = identity.channel_id();
        let last = match self.opening.get_mut(channel_id) {
            Some(requests) if *requests > 1 => {
                *requests -= 1;
                false
            }
            _ => {
                self.opening.remove(channel_id);
                true
            }
        };
        if held {
            self.open.open(identity);
            return Some(Review::Held(channel_id.to_string()));
        }
        (last && !self.open.is_open(channel_id)).then(|| Review::Unopened(channel_id.to_string()))
    }

    /// **The review the module's first startup begins** — event 3 — once it has
    /// counted the channel of every Stoa it asks for as being opened.
    ///
    /// The set is taken now, at the event, as the requirement reads ("whose
    /// channel is then neither open nor being opened"). At startup the book is
    /// new, so nothing is open yet and the set is startup's own requests.
    fn startup_review(&self) -> Review {
        Review::Startup(self.opening.keys().cloned().collect())
    }
}

/// Everything the listener, the worker and the processor share, under one lock:
/// the channel book, the payloads waiting to be taken, and the reviews waiting
/// to be run.
///
/// # One lock, because two of the spec's guarantees are about an order
///
/// `op-transport`, "Every message is decided exactly once, however close to a
/// settle it is taken": the taking of a message and an event that begins a
/// review of its channel "MUST be ordered, one before the other". The event
/// changes the book; the taking reads it. With the book and the waiting
/// payloads behind two locks, a settle could land between the processor popping
/// a payload and reading its channel's state, and which side of the event that
/// payload fell on would be a matter of scheduling. Here [`Channels::settle`]
/// changes the book and queues the review in one critical section, and
/// [`Channels::take`] pops a payload and reads its channel's state in another,
/// so every take is wholly before or wholly after every event.
///
/// # A waiting review is taken before any waiting payload
///
/// The same requirement's other half: a review "MUST decide all of them before
/// any message on that channel taken after the event that began the review is
/// judged". A payload taken after a settle that opened its channel reads
/// [`Taken::Judge`]; were the review behind it in a queue, that payload would be
/// judged before the parked messages the review decides. So reviews do not
/// queue among payloads: [`Channels::take`] returns any waiting review first.
/// Reviews are not counted towards the waiting payloads' bound, and are never
/// discarded — each is one of this peer's own requests settling.
///
/// # Nothing here is held across a decision
///
/// Every method holds the lock for map and queue operations only — and, in
/// [`Channels::handoff`], encoding one of this peer's own ops — never while a
/// payload is decoded, verified, appended or parked. So an append waiting on a
/// busy op log cannot hold up the worker's opens and sends, and hand-over
/// cannot wait on a payload being decided.
/// `the_channel_book_is_not_held_while_an_op_is_appended` is red if it is.
struct Channels {
    shared: Mutex<Shared>,
    /// Signalled when a payload or a review is queued, or the queue is closed.
    ready: Condvar,
}

struct Shared {
    book: ChannelBook,
    waiting: InboundQueue,
    reviews: VecDeque<Review>,
    /// Discards since the module started, from the waiting payloads and from
    /// the parked messages alike: `op-transport` keeps one running count.
    discarded: u64,
    /// Tests only, so a processor run on the test's thread ends once what is
    /// queued is done. Not in the running wiring's build at all: a review can
    /// still be due after delivery's events end, so nothing there may close it.
    #[cfg(test)]
    closed: bool,
}

impl Default for Channels {
    fn default() -> Self {
        Channels::with_bound(INBOUND_BOUND)
    }
}

/// What handing a message over did.
#[derive(Debug, PartialEq, Eq)]
enum HandedOver {
    /// Refused before it took a place: an unknown channel, or over the limit.
    Refused(InboundRefusal),
    /// It is waiting to be taken.
    Waiting,
    /// The queue was full, and a payload — perhaps this one — was discarded.
    /// Carries the running count and the bound.
    Discarded { total: u64, bound: usize },
}

/// What the processor takes next.
#[derive(Debug)]
enum Next {
    Review(Review),
    /// A payload, with what its channel's state when it was taken says to do.
    Payload(Arriving, Taken),
}

impl Channels {
    fn with_bound(bound: usize) -> Self {
        Channels {
            shared: Mutex::new(Shared {
                book: ChannelBook::default(),
                waiting: InboundQueue::with_bound(bound),
                reviews: VecDeque::new(),
                discarded: 0,
                #[cfg(test)]
                closed: false,
            }),
            ready: Condvar::new(),
        }
    }

    /// Count a channel as being opened, returning the guard that settles it.
    ///
    /// The guard owns a handle on the book rather than borrowing it, so it can
    /// outlive the call that made it.
    fn opening(self: &Arc<Self>, identity: &ChannelIdentity) -> Opening {
        lock(&self.shared).book.request(identity);
        Opening {
            channels: Arc::clone(self),
            identity: identity.clone(),
            held: false,
        }
    }

    /// One request settled: change the book and queue the review it begins, in
    /// one critical section.
    fn settle(&self, identity: &ChannelIdentity, held: bool) {
        let mut shared = lock(&self.shared);
        if let Some(review) = shared.book.settle(identity, held) {
            shared.reviews.push_back(review);
            self.ready.notify_all();
        }
    }

    /// Queue the review the module's first startup begins.
    fn begin_startup_review(&self) {
        let mut shared = lock(&self.shared);
        let review = shared.book.startup_review();
        shared.reviews.push_back(review);
        self.ready.notify_all();
    }

    /// Whether a channel is open or being opened. Tests only: the running code
    /// asks [`ChannelBook::is_known`] under the lock it acts under.
    #[cfg(test)]
    fn is_known(&self, channel_id: &str) -> bool {
        lock(&self.shared).book.is_known(channel_id)
    }

    /// Refuse a message before it waits, or offer it to the waiting payloads.
    ///
    /// The check and the offer are one critical section, so whether a channel is
    /// open or being opened "is judged when delivery hands the message over" and
    /// not at some other moment.
    fn hand_over(&self, message: Arriving) -> HandedOver {
        let mut shared = lock(&self.shared);
        if let Some(refusal) = refused_on_hand_over(&message, &shared.book) {
            return HandedOver::Refused(refusal);
        }
        let offered = shared.waiting.offer(message);
        self.ready.notify_all();
        match offered {
            Offered::Waiting => HandedOver::Waiting,
            Offered::Discarded => {
                shared.discarded = shared.discarded.saturating_add(1);
                HandedOver::Discarded {
                    total: shared.discarded,
                    bound: shared.waiting.bound,
                }
            }
        }
    }

    /// **What the processor does next** — the seam that orders every take
    /// against every event that begins a review.
    ///
    /// A waiting review first; otherwise the oldest waiting payload, with its
    /// channel's state read once, now ([`ChannelBook::on_take`]); otherwise
    /// block until there is one. `None` only once a test has closed it and
    /// nothing is left; the running wiring's build has no way to close it.
    fn take(&self) -> Option<Next> {
        let mut shared = lock(&self.shared);
        loop {
            if let Some(review) = shared.reviews.pop_front() {
                return Some(Next::Review(review));
            }
            if let Some(message) = shared.waiting.pop() {
                let taken = shared.book.on_take(&message.channel_id);
                return Some(Next::Payload(message, taken));
            }
            #[cfg(test)]
            if shared.closed {
                return None;
            }
            shared = self.ready.wait(shared).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Count one discard from the parked messages in the running count.
    fn count_discard(&self) -> u64 {
        let mut shared = lock(&self.shared);
        shared.discarded = shared.discarded.saturating_add(1);
        shared.discarded
    }

    /// The Stoa a channel is open for now, copied out so the lock is released
    /// before anything is judged. A review's lookup; a payload is judged on the
    /// reading it was taken under ([`Taken::Judge`]).
    fn stoa_of(&self, channel_id: &str) -> Option<Address> {
        lock(&self.shared).book.open.stoa_of(channel_id).copied()
    }

    /// What to send for an op already held, and on which channel.
    fn handoff(
        &self,
        stored: &crate::op::SignedOp,
    ) -> Result<transport::Publishable, PublishError> {
        transport::handoff(stored, &lock(&self.shared).book.open)
    }

    /// Let [`Channels::take`] return `None` once nothing is left. Tests only.
    #[cfg(test)]
    fn close(&self) {
        lock(&self.shared).closed = true;
        self.ready.notify_all();
    }
}

/// An open asked for and not yet settled. Dropping it settles the open — as held
/// only if [`Opening::finish`] said so — so no path between the request and
/// delivery's answer, a panic, a refused send to the worker or a worker that
/// never takes it included, can leave the channel being opened, or the messages
/// parked on it unreviewed.
struct Opening {
    channels: Arc<Channels>,
    identity: ChannelIdentity,
    held: bool,
}

impl Opening {
    /// Delivery reported that it holds the channel: created it, or already had it.
    /// Tests only; the worker says so with [`Opening::finish`].
    #[cfg(test)]
    fn held(&mut self) {
        self.held = true;
    }

    /// Settle the open now, as held or not, rather than whenever this is dropped.
    ///
    /// The worker finishes an open before it logs the outcome, so a line saying
    /// the open was answered or given up is never read while the channel still
    /// counts as being opened.
    fn finish(mut self, held: bool) {
        self.held = held;
        // `self` is dropped here, and the drop is the settle: [`Drop`] below
        // hands `held` to [`Channels::settle`]. One path settles, whether the
        // worker finishes the open or the guard is merely dropped.
    }
}

impl Drop for Opening {
    fn drop(&mut self) {
        self.channels.settle(&self.identity, self.held);
    }
}

// ─── Outbound: the worker ─────────────────────────────────────────────────

/// One thing the worker does.
enum Action {
    /// `createNode`, then `start` if creation was not declined.
    StartNode,
    /// `channelCreate` for a Stoa, carrying the guard that has counted its
    /// channel as being opened since the request was made. See [`Opening`].
    Open(Opening),
    /// `channelSend` for an op the log holds.
    Send(OpId),
    /// A test's barrier: answered once everything queued before it is done.
    #[cfg(test)]
    Drain(mpsc::Sender<()>),
}

struct Worker<D: Delivery> {
    delivery: D,
    channels: Arc<Channels>,
    stores: Stores,
    journal: Arc<dyn Journal>,
}

impl<D: Delivery> Worker<D> {
    fn run(self, actions: mpsc::Receiver<Action>) {
        for action in actions {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| self.perform(action))) {
                record(
                    &*self.journal,
                    Note::Panicked("a delivery action", &panic_detail(&*payload)),
                );
            }
        }
    }

    fn perform(&self, action: Action) {
        match action {
            Action::StartNode => self.start_node(),
            Action::Open(opening) => self.open(opening),
            Action::Send(id) => self.send(&id),
            #[cfg(test)]
            Action::Drain(done) => {
                let _ = done.send(());
            }
        }
    }

    /// Ask delivery to create and start the node.
    ///
    /// `op-transport`, "Start SHALL be requested only after delivery has accepted
    /// this application's creation" (scenario "A declined node creation does not
    /// stop the module": node start is not requested). A decline most often means
    /// another module in this context created the node already
    /// (`"Context already initialized"`), and that module owns its start.
    fn start_node(&self) {
        let created = self.delivery.create_node(&node_config());
        if let Some(why) = declined(&created) {
            record(&*self.journal, Note::NodeCreationDeclined(&why));
            return;
        }
        let started = self.delivery.start_node();
        match declined(&started) {
            Some(why) => record(&*self.journal, Note::NodeStartDeclined(&why)),
            None => record(&*self.journal, Note::NodeRequested),
        }
    }

    /// Ask delivery to open a Stoa's channel under this installation's sender
    /// identifier for it — or, with no identifier retained, ask for nothing.
    ///
    /// Every return settles the open: as held only on delivery's report that it
    /// holds the channel, and otherwise as given up — the no-identifier return
    /// included, which never asks delivery at all. Each settles **before** its
    /// log line, so the review it begins is queued by the time the line can be
    /// read.
    fn open(&self, opening: Opening) {
        let identity = opening.identity.clone();
        let stoa = identity.stoa();
        let sender = match self.stores.senders().and_then(|mut s| s.sender_for(stoa)) {
            Ok(sender) => sender,
            Err(why) => {
                opening.finish(false);
                record(&*self.journal, Note::SenderNotRetained(stoa, &why));
                return;
            }
        };
        let reply = self.delivery.channel_create(
            identity.channel_id(),
            identity.content_topic(),
            sender.as_str(),
        );
        match channel_answer(&reply) {
            ChannelAnswer::Created => {
                opening.finish(true);
                record(&*self.journal, Note::ChannelOpened(stoa));
            }
            ChannelAnswer::AlreadyHeld => {
                opening.finish(true);
                record(&*self.journal, Note::ChannelAlreadyHeld(stoa));
            }
            ChannelAnswer::Declined(why) => {
                opening.finish(false);
                record(&*self.journal, Note::ChannelDeclined(stoa, &why))
            }
        }
    }

    /// Hand an op the log holds to its Stoa's channel, as its stored wire form.
    ///
    /// **Read back by id**, not carried here from the publish: the payload is then
    /// the op's wire form as the log holds it — for a re-publish, the bytes stored
    /// the first time — which is what `op-transport` requires be sent.
    fn send(&self, id: &OpId) {
        let held = self.stores.op_log().and_then(|log| log.get(id));
        let entry = match held {
            Ok(Some(entry)) => entry,
            Ok(None) => {
                return record(
                    &*self.journal,
                    Note::NotHandedOff(id, "the log does not hold it"),
                )
            }
            Err(e) => {
                return record(
                    &*self.journal,
                    Note::NotHandedOff(id, &format!("it could not be read back: {e}")),
                )
            }
        };
        let publishable = match self.channels.handoff(&entry.op) {
            Ok(p) => p,
            Err(PublishError::NoChannel { stoa, id }) => {
                return record(&*self.journal, Note::NotSent(&id, &stoa))
            }
            Err(e) => return record(&*self.journal, Note::NotHandedOff(id, &e.to_string())),
        };
        let reply = self
            .delivery
            .channel_send(&publishable.channel_id, &publishable.payload);
        match declined(&reply) {
            Some(why) => record(&*self.journal, Note::SendDeclined(id, &why)),
            None => record(&*self.journal, Note::Sent(id, &entry.op.op.stoa)),
        }
    }
}

/// The dispatch side's handle on the worker: enqueue, never wait.
///
/// It holds the channel book too, because a request for a channel counts the
/// channel as being opened here, on the dispatch side, before it is queued.
struct Outbox {
    actions: mpsc::Sender<Action>,
    channels: Arc<Channels>,
}

// ─── Inbound: the bounded queue ───────────────────────────────────────────

/// How many inbound messages may wait for the boundary.
///
/// `op-transport` requires a fixed count, and leaves the number to design.
/// 256, from two bounds pulling against each other:
///
/// - **Memory.** This queue bounds a count, and a payload over the 150 KiB
///   message limit is refused on hand-over ([`refused_on_hand_over`]) rather than
///   held, so the payload bytes waiting are at most 256 × 150 KiB = 38,400 KiB
///   = 37.5 MiB in a module process sharing its host, whatever the largest
///   message the node carries. #30's 1024 would be 150 MiB by the same sum.
/// - **Loss.** A discard here is final for this peer: by the time
///   `channelMessageReceived` fires, SDS has treated the message as delivered and
///   will not repair it. #30's justification for discarding freely ("a dropped op
///   is recoverable through retransmission") does not hold. 256 is meant to be a
///   burst deeper than any SQLite append falls behind by; the processor's speed
///   per op has **not been measured**, so that is a judgement, not a figure.
pub const INBOUND_BOUND: usize = 256;

/// One `channelMessageReceived`, owned, as the listener received it.
///
/// Every field the event carries — including the sender identifier and the
/// timestamp, which decide nothing — for the reason [`InboundMessage`] carries
/// them all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arriving {
    pub channel_id: String,
    pub sender_id: String,
    pub payload: Vec<u8>,
    pub timestamp: i64,
}

impl Arriving {
    /// This message as the boundary takes it, every field included.
    fn inbound(&self) -> InboundMessage<'_> {
        InboundMessage {
            channel_id: &self.channel_id,
            sender_id: &self.sender_id,
            payload: &self.payload,
            timestamp: self.timestamp,
        }
    }
}

/// A parked message as the boundary takes it at its review: **with no sender
/// identifier and no timestamp**, because none was kept. `op-transport`: "A
/// parked message is put through the boundary without a sender identifier …
/// The boundary decides nothing from one", and the window is judged against
/// this peer's clock, never the event's timestamp. Empty and zero are what
/// "none" is in a struct that mirrors the event's fields.
fn parked_inbound(parked: &Parked) -> InboundMessage<'_> {
    InboundMessage {
        channel_id: &parked.channel_id,
        sender_id: "",
        payload: &parked.payload,
        timestamp: 0,
    }
}

/// What offering a message did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offered {
    Waiting,
    /// The queue was full, and the newest payload of the channel holding the
    /// most — the offered one, or one already waiting — was discarded unread.
    Discarded,
}

/// The payloads waiting to be taken, bounded by a fixed count. Plain data: the
/// lock is [`Channels`]'s.
///
/// # A full queue discards the newest payload of the channel holding the most
///
/// Counting the arrival with its own channel ([`crate::shedding::choose`], the
/// rule the parked messages' total bounds use too). This narrows #30's question
/// — which payload a full queue loses — to the channel causing the pressure: a
/// flood on one Stoa's channel loses that channel's own newest payloads, and a
/// quiet Stoa's message arriving behind the flood is kept. Within the flooding
/// channel it is still the newest that goes, as under the old rule, which
/// discarded the arrival. That losing the newest loses least is a heuristic on
/// SDS's catch-up order, which nothing here measures, and nothing depends on it
/// for correctness (`park-pending-inbound`'s design, Decision 5). When one
/// channel holds everything waiting, this is the old rule exactly.
///
/// # Offering never waits on the boundary
///
/// [`InboundQueue::offer`] pushes, or sheds and pushes, and returns: it is a
/// scan of at most the bound under [`Channels`]'s lock, which no decision
/// holds.
struct InboundQueue {
    messages: VecDeque<Arriving>,
    bound: usize,
}

impl InboundQueue {
    fn with_bound(bound: usize) -> Self {
        InboundQueue {
            messages: VecDeque::new(),
            bound,
        }
    }

    fn offer(&mut self, message: Arriving) -> Offered {
        if self.messages.len() < self.bound {
            self.messages.push_back(message);
            return Offered::Waiting;
        }
        let waiting = self.messages.iter().map(|m| (m.channel_id.as_str(), 1));
        let at = match choose(waiting, (message.channel_id.as_str(), 1)) {
            // The arrival's channel holds the most, and the arrival is its
            // newest: discarded, and every waiting payload kept.
            Victim::Arrival => return Offered::Discarded,
            // The queue is in arrival order, so the last waiting payload on the
            // channel chosen is that channel's newest.
            Victim::NewestOf(victim) => self.messages.iter().rposition(|m| m.channel_id == victim),
        };
        // `choose` names only a channel holding something, so `rposition` finds
        // it; were it not to, the arrival is the one dropped, and the bound holds.
        if let Some(at) = at {
            self.messages.remove(at);
            self.messages.push_back(message);
        }
        Offered::Discarded
    }

    /// The oldest waiting payload.
    fn pop(&mut self) -> Option<Arriving> {
        self.messages.pop_front()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.messages.len()
    }
}

// ─── Inbound: the listener's loop and the processor ───────────────────────

/// Drain delivery's `channelMessageReceived` events into the queue, until the
/// subscription ends.
///
/// `events` yields `Some` for an event whose fields were read and `None` for one
/// whose were not. The adapter supplies it as the SDK subscription mapped
/// through the generated decoder, so this loop is the whole of the listener's
/// logic and a test drives it with a plain iterator.
///
/// # It ends, and says so, when delivery goes away
///
/// The subscription's iterator ends when `recv()` fails. On the SDK the builder
/// pins, `recv()` polls the provider's `Abandoned` status every 200 ms and fails
/// once it is reported — on a runtime with the status channel: one whose
/// `logos_protocol.h` defines `LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE` (the
/// symbol is `lp_client_set_subscription_status_cb`). **Not "0.9"**: both cuts of
/// logos-protocol 0.9 report MINOR 9 and the first lacks the channel, as the header
/// at the rev `dialectica/flake.lock` pins says. On a runtime without it there is
/// no status and `recv()` parks forever: the thread is then leaked, but it is
/// this thread alone, holding nothing any other thread waits on.
///
/// # A panic is contained per event, not per loop
///
/// The iterator's `next()` is where the SDK receives an event and the generated
/// decoder reads its fields — the one piece of the inbound path that touches
/// event data before the queue. Containing a panic there around the whole loop
/// would end reception for the rest of the process, every Stoa at once, over one
/// event. So each `next()` and its hand-over run under their own `catch_unwind`,
/// and the loop carries on. `a_panic_reading_one_event_does_not_end_reception` is
/// red with one `catch_unwind` around the loop.
///
/// The cost: an iterator that panicked on *every* call without consuming an
/// event would spin here, logging. The SDK's does not — the event is received
/// before it is decoded, so the next call receives the next one.
fn listen<I>(mut events: I, channels: &Channels, journal: &dyn Journal)
where
    I: Iterator<Item = Option<Arriving>>,
{
    loop {
        let taken = catch_unwind(AssertUnwindSafe(|| match events.next() {
            None => false,
            Some(Some(message)) => {
                hand_over(message, channels, journal);
                true
            }
            Some(None) => {
                record(journal, Note::Unreadable);
                true
            }
        }));
        match taken {
            Ok(true) => {}
            Ok(false) => return,
            Err(payload) => record(
                journal,
                Note::Panicked("reading an inbound event", &panic_detail(&*payload)),
            ),
        }
    }
}

/// Offer one message to the queue — unless [`refused_on_hand_over`] refuses it
/// here, when it takes no place and is logged as the boundary logs that refusal.
fn hand_over(message: Arriving, channels: &Channels, journal: &dyn Journal) {
    match channels.hand_over(message) {
        HandedOver::Refused(refusal) => {
            record(journal, Note::Refused(refusal_kind(&refusal), None))
        }
        HandedOver::Waiting => {}
        HandedOver::Discarded { total, bound } => {
            record(journal, Note::Discarded { total, bound })
        }
    }
}

/// The refusal a message meets before it may wait for the boundary, if any: a
/// channel neither open nor being opened, whatever the payload's size; then a
/// payload over the message limit.
///
/// # Why an unknown channel is refused before the queue
///
/// `op-transport`: such a message "MUST NOT take a place among the waiting
/// payloads", is refused as an unknown channel "when delivery hands it over",
/// and is neither counted towards the bound nor as a discard. The node is shared,
/// so another application's channel traffic arrives here, and any peer may send
/// on any identifier it picks; queued, that traffic filled the queue and forced
/// final discards of every Stoa's ops (`delivery-wiring`'s design, Decision 10).
/// `traffic_on_a_channel_this_peer_is_not_opening_takes_no_place_in_the_queue`
/// is red without it.
///
/// The check is made under [`Channels`]'s lock, which is never held while a
/// payload is decided, so taking a message from delivery still does not wait on
/// the boundary.
///
/// # Why an oversized payload is refused before the queue
///
/// `op-transport`: a payload over the message limit, on a channel open or being
/// opened, "MUST NOT take a place among the waiting payloads" either. The queue
/// bounds a count, so without this its memory was the bound times the largest
/// message the *node* carries — and the node may be one another module created.
/// Refused here, what waits is bounded by [`INBOUND_BOUND`] ×
/// [`crate::transport::MAX_MESSAGE_BYTES`], this application's own limit. The
/// predicate is [`crate::transport::refuse_oversized`], the one the boundary
/// asks, so the two cannot disagree about a payload of exactly the limit.
/// `an_oversized_payload_on_an_open_channel_takes_no_place_in_the_queue` is red
/// without it; `a_payload_at_the_limit_waits_its_turn` pins where it falls.
///
/// The unknown channel is asked first, so a message on a channel neither open
/// nor being opened is refused as that whatever its size.
fn refused_on_hand_over(message: &Arriving, book: &ChannelBook) -> Option<InboundRefusal> {
    if !book.is_known(&message.channel_id) {
        return Some(InboundRefusal::UnknownChannel);
    }
    transport::refuse_oversized(&message.payload).err()
}

struct Processor {
    channels: Arc<Channels>,
    stores: Stores,
    journal: Arc<dyn Journal>,
    clock: fn() -> u64,
    /// The bounds a test parks by: [`PARK_BOUNDS`] unless the test shrinks them
    /// to reach a bound. **Absent from the running wiring's build**, where
    /// [`Processor::bounds`] is the constant itself.
    #[cfg(test)]
    bounds: crate::parked::ParkBounds,
}

impl Processor {
    /// The one place a processor is built, [`Delivering::start`] and the tests'
    /// fixtures alike.
    fn new(
        channels: Arc<Channels>,
        stores: Stores,
        journal: Arc<dyn Journal>,
        clock: fn() -> u64,
    ) -> Self {
        Processor {
            channels,
            stores,
            journal,
            clock,
            #[cfg(test)]
            bounds: PARK_BOUNDS,
        }
    }

    /// The bounds parking holds the parked messages to.
    ///
    /// **[`PARK_BOUNDS`] by construction in the running wiring**: outside a test
    /// build there is no field a constructor or `start` could set otherwise.
    /// That is the lesson of the wait this replaced, whose limit set wrongly in
    /// `start` passed every test (`delivery-wiring`'s spec-test re-review, round
    /// 3). A test build reads the field, so a test can shrink it.
    #[cfg(not(test))]
    fn bounds(&self) -> &crate::parked::ParkBounds {
        &PARK_BOUNDS
    }

    #[cfg(test)]
    fn bounds(&self) -> &crate::parked::ParkBounds {
        &self.bounds
    }

    /// Take and act on each thing in turn, for the life of the module.
    ///
    /// It does not stop when delivery's events end: a review can still be due
    /// — an open answered after the subscription ended — and the messages parked
    /// for it would otherwise wait for the next startup.
    fn run(self) {
        while let Some(next) = self.channels.take() {
            self.act(next);
        }
    }

    /// Run one review, or judge or park one payload, as [`Channels::take`]
    /// decided when it took it.
    fn act(&self, next: Next) {
        match next {
            Next::Review(review) => self.contained("a review of parked messages", || {
                self.review(review)
            }),
            // Judged against the Stoa read when it was taken: the one reading.
            Next::Payload(message, Taken::Judge(stoa)) => {
                self.contained("an inbound message", || {
                    self.judge(message.inbound(), |_| stoa)
                })
            }
            Next::Payload(message, Taken::Park) => {
                self.contained("parking an inbound message", || self.park(&message))
            }
        }
    }

    /// Run `work`, logging a panic rather than letting it end the processor.
    fn contained(&self, what: &'static str, work: impl FnOnce()) {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(work)) {
            record(&*self.journal, Note::Panicked(what, &panic_detail(&*payload)));
        }
    }

    /// Park one payload taken on a channel being opened, judging nothing.
    ///
    /// `op-transport`: a parked message "MUST NOT be judged, refused or stored
    /// then", whatever its bytes; save a discard under a parking bound. What
    /// cannot be written is a storage failure, and the payload is not held for
    /// another attempt.
    fn park(&self, message: &Arriving) {
        let outcome = self.stores.parked().and_then(|mut store| {
            store.park(&message.channel_id, &message.payload, self.bounds())
        });
        // Each message the park discarded — those evicted for it, or the
        // payload itself — counted and logged alike.
        let (discards, parked) = match outcome {
            Ok(ParkOutcome::Parked { evicted }) => (evicted, true),
            Ok(ParkOutcome::Discarded) => (1, false),
            Err(e) => return record(&*self.journal, Note::NotParked(&e.to_string())),
        };
        for _ in 0..discards {
            let total = self.channels.count_discard();
            record(&*self.journal, Note::ParkDiscarded { total });
        }
        if parked {
            record(&*self.journal, Note::Parked);
        }
    }

    /// Run the review its event began: `op-transport`, "Parked messages are
    /// reviewed on three events and no others", one function per event.
    fn review(&self, review: Review) {
        match review {
            Review::Held(channel_id) => self.decide_parked(&channel_id),
            Review::Unopened(channel_id) => self.refuse_parked(&channel_id),
            Review::Startup(known) => self.refuse_unknown_at_startup(&known),
        }
    }

    /// Event 1, delivery holds the channel: judge each message parked on it, in
    /// the order they were handed over, as a message taken on an open channel is
    /// judged — against the channels open now, not when it was parked. Each is
    /// judged under its own `catch_unwind`, so one that panics does not cost the
    /// rest.
    fn decide_parked(&self, channel_id: &str) {
        for parked in self.take_parked(channel_id) {
            self.contained("an inbound message", || {
                self.judge(parked_inbound(&parked), |channel_id| {
                    self.channels.stoa_of(channel_id)
                })
            });
        }
    }

    /// Event 3, the module's first startup: refuse what is parked on every
    /// channel not in `known` — the channels startup counted as being opened —
    /// and leave the rest for their channel's next review.
    fn refuse_unknown_at_startup(&self, known: &HashSet<String>) {
        let channels = match self.stores.parked().and_then(|store| store.channels()) {
            Ok(channels) => channels,
            Err(e) => return record(&*self.journal, Note::ReviewUnreadable(&e.to_string())),
        };
        for channel_id in channels.iter().filter(|c| !known.contains(*c)) {
            self.refuse_parked(channel_id);
        }
    }

    /// Event 2, and event 3 for each channel it refuses: refuse every message
    /// parked on a channel as arriving on an unknown channel, logged as the
    /// boundary logs that refusal.
    fn refuse_parked(&self, channel_id: &str) {
        for _ in self.take_parked(channel_id) {
            let refusal = InboundRefusal::UnknownChannel;
            record(&*self.journal, Note::Refused(refusal_kind(&refusal), None));
        }
    }

    /// Take a channel's parked messages out of the store — or, when they cannot
    /// be read, log it and take nothing, leaving them for the channel's next
    /// review.
    fn take_parked(&self, channel_id: &str) -> Vec<Parked> {
        match self
            .stores
            .parked()
            .and_then(|mut store| store.take_channel(channel_id))
        {
            Ok(parked) => parked,
            Err(e) => {
                record(&*self.journal, Note::ReviewUnreadable(&e.to_string()));
                Vec::new()
            }
        }
    }

    /// Put one message through the boundary and log what it decided.
    fn judge(&self, inbound: InboundMessage<'_>, stoa_of: impl FnOnce(&str) -> Option<Address>) {
        let decided = self.pass(inbound, stoa_of);
        self.record_decision(decided);
    }

    /// Put one message through the inbound boundary, [`transport::receive_via`].
    ///
    /// **The clock is read here, when the message is judged** — for a parked
    /// message, at its review — never the event's timestamp, which is delivery's
    /// reading at receipt, in nanoseconds. `op-transport` requires the window be
    /// judged against this peer's own clock "read when the payload is judged".
    ///
    /// **`stoa_of` says which reading of the channel book the message is judged
    /// on**, under the message's own channel identifier: for a payload, the one
    /// [`Channels::take`] made as it took it ([`Taken::Judge`]); for a parked
    /// message, a lookup now, at its review, because a review judges "against
    /// the channels open when it is judged". Either copies the Stoa out, so no
    /// lock is held while the message is judged.
    ///
    /// The op log is opened only for an op that passed: a payload refused on its
    /// channel or its bytes costs no database open, and is refused under its own
    /// name even when the log will not open.
    fn pass(
        &self,
        inbound: InboundMessage<'_>,
        stoa_of: impl FnOnce(&str) -> Option<Address>,
    ) -> Result<transport::Admitted, InboundRefusal> {
        let now_ms = (self.clock)();
        transport::receive_via(
            inbound,
            stoa_of,
            now_ms,
            |judged| {
                // `op-transport`, scenario "A message the op log cannot take is
                // logged and not retried": an op log that will not open is a
                // storage failure, and the message is dropped, not held.
                let mut log = self.stores.op_log().map_err(InboundRefusal::Storage)?;
                transport::admit(judged, &mut log)
            },
        )
    }

    /// Log what the boundary decided about one message.
    fn record_decision(&self, decided: Result<transport::Admitted, InboundRefusal>) {
        match decided {
            Ok(admitted) => match admitted.appended {
                Appended::Stored => record(&*self.journal, Note::Stored(&admitted.id)),
                Appended::AlreadyPresent => {
                    record(&*self.journal, Note::AlreadyStored(&admitted.id))
                }
            },
            Err(InboundRefusal::Storage(e)) => {
                let detail = e.to_string();
                record(&*self.journal, Note::Refused("storage", Some(&detail)))
            }
            Err(refusal) => record(&*self.journal, Note::Refused(refusal_kind(&refusal), None)),
        }
    }
}

// ─── The lifecycle ────────────────────────────────────────────────────────

/// Delivery's wiring for one module process: started once, then fed by the
/// handlers.
///
/// `Default` so the adapter's module struct keeps its one parameterless
/// constructor (`interface: "universal"` miscompiles any other).
pub struct Delivering {
    journal: Arc<dyn Journal>,
    wiring: Wiring,
}

/// Where startup has got to, as one value.
///
/// Three states, not a `started` flag beside an optional outbox: that pair had a
/// fourth combination — started, with no worker — whose requests were logged as
/// "has not started". Here that state is [`Wiring::NoWorker`], with a line of its
/// own.
enum Wiring {
    /// Startup has not run: a request is logged and dropped.
    NotStarted,
    /// Startup ran and the OS refused the worker thread: nothing outbound can be
    /// requested for the rest of the process.
    NoWorker,
    /// Startup ran; requests go to the worker.
    Running(Outbox),
}

impl Default for Delivering {
    fn default() -> Self {
        Delivering::new(Arc::new(Stderr))
    }
}

impl Delivering {
    pub fn new(journal: Arc<dyn Journal>) -> Self {
        Delivering {
            journal,
            wiring: Wiring::NotStarted,
        }
    }

    /// Wire delivery, once per process.
    ///
    /// In order:
    ///
    /// 1. count the channel of every Stoa the membership record holds as being
    ///    opened;
    /// 2. queue the startup review (review event 3), whose set of known channels
    ///    is exactly those — after step 1, and before anything can settle;
    /// 3. subscribe to `channelMessageReceived`, so nothing a channel receives
    ///    can precede the listener;
    /// 4. start the processor, whose first take is that review, and the worker;
    /// 5. ask for the node, then for each of the channels counted in step 1. The
    ///    node is first in the worker's queue, so its creation is requested
    ///    before any channel operation.
    ///
    /// **A second call does nothing and returns `false`**, so node creation and
    /// start are requested at most once per process however often startup runs.
    /// The first call returns `true` — it ran — whether or not every step it
    /// took succeeded; a step that failed is in the log.
    ///
    /// A failure at any step is logged and the rest still happens: a declined
    /// subscription leaves sending working, an unreadable membership record leaves
    /// later joins working, and no step can stop the module answering calls.
    pub fn start<D, S, I>(
        &mut self,
        delivery: D,
        stores: Stores,
        clock: fn() -> u64,
        subscribe: S,
    ) -> bool
    where
        D: Delivery,
        S: FnOnce() -> Result<I, String>,
        I: Iterator<Item = Option<Arriving>> + Send + 'static,
    {
        // `stoa-membership`, "Startup running again in the same module process
        // MUST NOT request any channel" (scenario "A second startup in one
        // process requests no channel"), beside `op-transport`'s node asked for
        // once. A second call asks for nothing at all, because the first call
        // already asked for every membership's. `NoWorker` counts as started: a
        // second call must not start a second listener and processor.
        if !matches!(self.wiring, Wiring::NotStarted) {
            record(&*self.journal, Note::AlreadyStarted);
            return false;
        }
        self.wiring = Wiring::NoWorker;
        let channels = Arc::new(Channels::default());

        // `op-transport`: "The module's startup MUST count the channel of every
        // Stoa it asks for as being opened before it checks any message delivery
        // hands over" (scenario "A restarted peer keeps what delivery hands over
        // before startup asks for its channel"). Marked here, before the
        // subscription exists, rather than as the worker reaches each open behind
        // node creation — a delivery that kept running hands those channels'
        // messages over from the first event.
        let opens = self.startup_opens(&stores, &channels);

        // Review trigger 3: the first startup in this process, once it has
        // counted every channel it asks for. Queued before the subscription and
        // before any open can settle, so it is the first thing the processor
        // runs, and its set of known channels is exactly startup's. A second
        // `start` returned above, so a later startup begins no review.
        channels.begin_startup_review();

        // `op-transport`, scenario "A peer that cannot subscribe still publishes":
        // a failed subscription is logged and startup carries on — the node, the
        // channels and the sends are still requested — because a peer that
        // cannot receive can still publish. The processor is started either way:
        // messages parked by the last run still have reviews due.
        match subscribe() {
            Ok(events) => self.spawn_listener(events, Arc::clone(&channels)),
            Err(why) => record(&*self.journal, Note::NotSubscribed(&why)),
        }
        self.spawn_processor(Arc::clone(&channels), stores.clone(), clock);

        let (actions, pending) = mpsc::channel();
        let worker = Worker {
            delivery,
            channels: Arc::clone(&channels),
            stores,
            journal: Arc::clone(&self.journal),
        };
        if !self.spawn("dialectica delivery worker", worker, move |w| {
            w.run(pending)
        }) {
            // `true`, not `false`: this call ran, and the listener and processor
            // it started are running. The wiring stays `NoWorker`, so every
            // later request says the worker could not be started. `opens` is
            // dropped with this return, so every startup open is given up — none
            // will ever be asked of delivery — and its parked messages are
            // refused by the review that begins.
            return true;
        }
        let outbox = Outbox { actions, channels };
        let _ = outbox.actions.send(Action::StartNode);
        for opening in opens {
            // A refused send hands the action back inside its error, and
            // dropping that settles the open.
            let _ = outbox.actions.send(Action::Open(opening));
        }
        self.wiring = Wiring::Running(outbox);
        true
    }

    /// Count the channel of every Stoa the membership record holds as being
    /// opened, returning the guards that settle each.
    ///
    /// An unreadable record is logged and marks nothing: later joins still work.
    fn startup_opens(&self, stores: &Stores, channels: &Arc<Channels>) -> Vec<Opening> {
        match stores.memberships() {
            Ok(stoas) => stoas
                .iter()
                .map(|stoa| channels.opening(&ChannelIdentity::of(stoa)))
                .collect(),
            Err(why) => {
                record(&*self.journal, Note::MembershipUnreadable(&why));
                Vec::new()
            }
        }
    }

    /// A membership was recorded: ask for its Stoa's channel.
    ///
    /// The channel counts as being opened from here — `op-transport`, "A channel
    /// is being opened from the moment a create, a join or the module's startup
    /// asks for it" — not from when the worker reaches the request.
    pub fn joined(&self, stoa: &Address) {
        self.request(
            |channels| Action::Open(channels.opening(&ChannelIdentity::of(stoa))),
            || format!("the channel for Stoa {}", stoa.to_hex()),
        );
    }

    /// An op was published: ask for it to be sent on its Stoa's channel.
    pub fn published(&self, id: &OpId) {
        self.request(
            |_| Action::Send(*id),
            || format!("the send of op {}", id.to_hex()),
        );
    }

    /// Queue what `action` makes, when there is a worker to take it.
    ///
    /// `action` is only called then, so no channel is counted as being opened
    /// for a request that is logged and dropped; and one the worker refuses is
    /// handed back inside the send's error and dropped, which settles it.
    fn request(&self, action: impl FnOnce(&Arc<Channels>) -> Action, what: impl Fn() -> String) {
        match &self.wiring {
            // `op-transport`, scenario "A publish before delivery is wired is not
            // sent when it is", and `stoa-membership`, "A join before delivery is
            // wired has its channel requested once, by startup": a request made
            // before delivery is wired is logged and dropped, not held. Startup
            // asks for every membership's channel anyway; an op published then is
            // not re-sent.
            Wiring::NotStarted => record(&*self.journal, Note::NotStarted(&what())),
            Wiring::NoWorker => record(&*self.journal, Note::NoWorker(&what())),
            Wiring::Running(outbox) => {
                if outbox.actions.send(action(&outbox.channels)).is_err() {
                    record(&*self.journal, Note::WorkerGone(&what()));
                }
            }
        }
    }

    /// Start the listener. Its end is logged, and closes nothing: what is
    /// already waiting is still taken, and the processor stays for the reviews
    /// still due ([`Processor::run`]).
    fn spawn_listener<I>(&self, events: I, channels: Arc<Channels>)
    where
        I: Iterator<Item = Option<Arriving>> + Send + 'static,
    {
        let journal = Arc::clone(&self.journal);
        self.spawn("dialectica inbound listener", events, move |events| {
            if let Err(payload) =
                catch_unwind(AssertUnwindSafe(|| listen(events, &channels, &*journal)))
            {
                record(
                    &*journal,
                    Note::Panicked("the inbound listener", &panic_detail(&*payload)),
                );
            }
            record(&*journal, Note::ListenerEnded);
        });
    }

    /// Start the processor, built the one way a processor is built
    /// ([`Processor::new`]), for the life of the module ([`Processor::run`]).
    fn spawn_processor(&self, channels: Arc<Channels>, stores: Stores, clock: fn() -> u64) {
        let processor = Processor::new(channels, stores, Arc::clone(&self.journal), clock);
        self.spawn("dialectica inbound processor", processor, Processor::run);
    }

    /// Start a named thread, logging rather than panicking when the OS refuses.
    /// `std::thread::spawn` panics on that refusal; `Builder` reports it.
    fn spawn<T: Send + 'static>(
        &self,
        name: &'static str,
        state: T,
        body: impl FnOnce(T) + Send + 'static,
    ) -> bool {
        let spawned = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(move || body(state));
        match spawned {
            Ok(_) => true,
            Err(e) => {
                record(&*self.journal, Note::ThreadNotStarted(name, &e.to_string()));
                false
            }
        }
    }

    /// A test's barrier: returns once the worker has finished everything queued
    /// before it. Named apart from the settles of an open, which are the
    /// production meaning of the word here.
    #[cfg(test)]
    fn drain(&self) {
        let (done, wait) = mpsc::channel();
        if let Wiring::Running(outbox) = &self.wiring {
            let _ = outbox.actions.send(Action::Drain(done));
            wait.recv_timeout(Duration::from_secs(30))
                .expect("the worker drains");
        }
    }
}

#[cfg(test)]
mod tests;
