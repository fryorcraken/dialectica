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
//! # Three threads, and why the reply is never one of them
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
//! - **The listener** takes `channelMessageReceived` events and offers each to the
//!   bounded [`InboundQueue`] without waiting; **the processor** takes them off in
//!   arrival order and puts each through [`crate::transport::receive`].
//!
//! # Nothing here may unwind
//!
//! A panic on a dispatch thread aborts the module process (`PHASE0-FINDINGS`
//! §3), and a panic on the worker or the processor would silently end delivery.
//! Each action and each message is run under `catch_unwind` and a panic is
//! logged; a pending channel open is cleared by a `Drop` guard so a panic cannot
//! leave the processor waiting on it.

use crate::identity::Address;
use crate::log::{OpLog, OpLogError, SqliteOpLog};
use crate::membership::{membership_path_in, MembershipError, MembershipStore};
use crate::op::OpId;
use crate::sender::{sender_path_in, SenderError, SenderStore};
use crate::transport::{
    self, ChannelIdentity, InboundMessage, InboundRefusal, OpenChannels, PublishError,
};
use crate::wire::panic_detail;
use std::collections::{HashMap, VecDeque};
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
/// Nothing waits on this but the worker thread, so its length costs no reply.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(35);

/// The configuration handed to `createNode`.
///
/// - **`entryLayer: "channels"`, named** rather than left to delivery's default,
///   as `op-transport` requires: every Stoa's ops travel on a reliable channel,
///   and a node without that layer refuses every channel call.
/// - **`preset: "logos.test"`**: the Logos Test Network, cluster 2. It is the
///   network whose 150 KiB maximum message size
///   [`crate::transport::MAX_MESSAGE_BYTES`] pins, and the one #30 used.
///   `logos.dev` is cluster 3 with the transport's default size; peers on the
///   two presets never meet, so this value is part of the interop contract in
///   practice even though the spec leaves it to design.
/// - **`mode: "Edge"`**: a light node. It does not relay other peers' traffic;
///   it publishes and receives through the preset's service nodes. `Core` would
///   make every dialectica peer a relay, contributing bandwidth and not depending
///   on the fleet. `design.md` records the choice and what would reverse it.
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
/// Recognised by [`ALREADY_EXISTS`] in delivery's reason. design.md Decision 14
/// says why the wording and not a `channelExists` confirmation, and what a change
/// to that wording would cost.
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
/// through behind a `"ChannelCreate failed: "` prefix.
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
    Refused(&'static str, Option<&'a str>),
    Discarded { total: u64, bound: usize },
    Unreadable,
    MembershipUnreadable(&'a MembershipError),
    NotSubscribed(&'a str),
    ListenerEnded,
    AlreadyStarted,
    NotStarted(&'a str),
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
/// Exhaustive with no wildcard, so a seventh refusal forces a name here rather
/// than being logged under one of these.
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

/// The three files the delivery threads open, all in the host's directory.
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

// ─── Which channels are open ──────────────────────────────────────────────

/// The channels this peer has open, and the ones it has asked for and not heard
/// back about.
///
/// # Pending is its own state, because of a race delivery really has
///
/// A channel counts as open only once delivery answers that it holds it —
/// created it, or already had it ([`channel_answer`]). But
/// delivery v0.2.1 emits `channelMessageReceived` from its runtime's callback
/// thread whenever SDS hands it one — including after the runtime has created the
/// channel and before the `channelCreate` answer has reached this peer. A message
/// judged in that gap is refused as arriving on an unknown channel, and SDS has
/// already treated it as delivered, so it is gone for this peer.
///
/// So the processor, meeting a message on a channel that is **not open but
/// pending**, waits for the open to be answered before judging it. It does not
/// wait on an open channel, or on one nobody asked for. Removing the wait makes
/// `a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` red.
///
/// A count per channel id rather than a flag, because a repeated join can put a
/// second open in the queue before the first is answered.
#[derive(Default)]
struct ChannelBook {
    open: OpenChannels,
    pending: HashMap<String, usize>,
}

#[derive(Default)]
struct Channels {
    book: Mutex<ChannelBook>,
    settled: Condvar,
}

/// How long the processor waits on a pending open before judging anyway: past
/// [`CALL_TIMEOUT`], so it outlasts the call it is waiting on. The worker always
/// answers the open — a `Drop` guard settles it even on a panic — so this bound
/// is reached only if that guarantee is broken.
const SETTLE_LIMIT: Duration = Duration::from_secs(40);

impl Channels {
    /// Mark an open as asked for, returning the guard that settles it.
    fn opening<'a>(&'a self, identity: &ChannelIdentity) -> Opening<'a> {
        *lock(&self.book)
            .pending
            .entry(identity.channel_id().to_string())
            .or_insert(0) += 1;
        Opening {
            channels: self,
            identity: identity.clone(),
            held: false,
        }
    }

    fn settle(&self, identity: &ChannelIdentity, held: bool) {
        let mut book = lock(&self.book);
        if let Some(count) = book.pending.get_mut(identity.channel_id()) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                book.pending.remove(identity.channel_id());
            }
        }
        // `stoa-membership`, scenario "A declined repeat request leaves an open
        // channel open": a repeat open that is declined leaves an already-open
        // channel open, because delivery created it once and nothing has closed
        // it. `a_declined_repeat_open_leaves_the_channel_open_for_the_next_message`
        // states it through a message; `a_declined_repeat_open_leaves_an_open_channel_open`
        // through this book.
        if held {
            book.open.open(identity);
        }
        self.settled.notify_all();
    }

    /// Wait, up to `limit`, while a channel is pending and not open.
    fn await_settled(&self, channel_id: &str, limit: Duration) {
        let book = lock(&self.book);
        let _ = self.settled.wait_timeout_while(book, limit, |b| {
            !b.open.is_open(channel_id) && b.pending.contains_key(channel_id)
        });
    }

    /// The Stoa a channel is open for now, copied out so the lock is released
    /// before anything is judged.
    ///
    /// **The book is never held across a decision.** Every method here holds the
    /// guard for map operations only — and, in [`Channels::handoff`], encoding
    /// one of this peer's own ops — never while a payload is decoded, verified
    /// or appended. So an append waiting on a busy op log cannot hold up the
    /// worker's opens and sends, and the listener's hand-over check cannot wait
    /// on a payload being decided. `the_channel_book_is_not_held_while_an_op_is_appended`
    /// is red if it is.
    fn stoa_of(&self, channel_id: &str) -> Option<Address> {
        lock(&self.book).open.stoa_of(channel_id).copied()
    }

    /// What to send for an op already held, and on which channel.
    fn handoff(
        &self,
        stored: &crate::op::SignedOp,
    ) -> Result<transport::Publishable, PublishError> {
        transport::handoff(stored, &lock(&self.book).open)
    }
}

/// An open in flight. Dropping it settles the open — as open only if
/// [`Opening::held`] was called — so no path out of the worker, a panic
/// included, can leave the channel pending.
struct Opening<'a> {
    channels: &'a Channels,
    identity: ChannelIdentity,
    held: bool,
}

impl Opening<'_> {
    /// Delivery reported that it holds the channel: created it, or already had it.
    fn held(&mut self) {
        self.held = true;
    }
}

impl Drop for Opening<'_> {
    fn drop(&mut self) {
        self.channels.settle(&self.identity, self.held);
    }
}

// ─── Outbound: the worker ─────────────────────────────────────────────────

/// One thing the worker does.
enum Action {
    /// `createNode`, then `start` if creation was not declined.
    StartNode,
    /// `channelCreate` for a Stoa.
    Open(Address),
    /// `channelSend` for an op the log holds.
    Send(OpId),
    /// A test's barrier: answered once everything queued before it is done.
    #[cfg(test)]
    Settle(mpsc::Sender<()>),
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
            Action::Open(stoa) => self.open(&stoa),
            Action::Send(id) => self.send(&id),
            #[cfg(test)]
            Action::Settle(done) => {
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
    fn open(&self, stoa: &Address) {
        let sender = match self.stores.senders().and_then(|mut s| s.sender_for(stoa)) {
            Ok(sender) => sender,
            Err(why) => {
                record(&*self.journal, Note::SenderNotRetained(stoa, &why));
                return;
            }
        };
        let identity = ChannelIdentity::of(stoa);
        let mut opening = self.channels.opening(&identity);
        let reply = self.delivery.channel_create(
            identity.channel_id(),
            identity.content_topic(),
            sender.as_str(),
        );
        match channel_answer(&reply) {
            ChannelAnswer::Created => {
                opening.held();
                record(&*self.journal, Note::ChannelOpened(stoa));
            }
            ChannelAnswer::AlreadyHeld => {
                opening.held();
                record(&*self.journal, Note::ChannelAlreadyHeld(stoa));
            }
            ChannelAnswer::Declined(why) => {
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
struct Outbox {
    actions: mpsc::Sender<Action>,
}

// ─── Inbound: the bounded queue ───────────────────────────────────────────

/// How many inbound messages may wait for the boundary.
///
/// `op-transport` requires a fixed count, and leaves the number to design.
/// 256, from two bounds pulling against each other:
///
/// - **Memory.** A payload may be up to 150 KiB before the boundary refuses it,
///   so the worst case is 256 × 150 KiB ≈ 37.5 MiB held in a module process that
///   shares its host. #30 chose 1024 — ≈ 150 MiB worst case.
/// - **Loss.** A discard here is final for this peer: by the time
///   `channelMessageReceived` fires, SDS has treated the message as delivered and
///   will not repair it. #30's justification for discarding freely ("a dropped op
///   is recoverable through retransmission") does not hold. The processor
///   decides a typical op in about a millisecond, so 256 waiting is a burst
///   hundreds of messages deep arriving faster than SQLite appends.
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

/// What offering a message did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offered {
    Waiting,
    /// Discarded unread: the queue was full. Carries the running total.
    Discarded(u64),
}

#[derive(Default)]
struct Waiting {
    messages: VecDeque<Arriving>,
    discarded: u64,
    closed: bool,
}

/// The messages waiting for the boundary, bounded by a fixed count.
///
/// # A full queue discards the ARRIVING message and keeps what waits
///
/// This reverses #30, which discarded the oldest. A backlog burst — SDS
/// catching a peer up — arrives roughly oldest first, so the oldest waiting
/// messages are the thread roots, and the newest arrivals are the leaves.
/// Discarding the oldest keeps replies whose parents are gone; discarding the
/// arrival loses leaves and keeps every thread it holds whole.
///
/// # Offering never waits on the boundary
///
/// [`InboundQueue::offer`] takes the lock, pushes or counts, and returns. Only
/// [`InboundQueue::take`] blocks, and only the processor calls it.
struct InboundQueue {
    waiting: Mutex<Waiting>,
    arrived: Condvar,
    bound: usize,
}

impl InboundQueue {
    fn with_bound(bound: usize) -> Self {
        InboundQueue {
            waiting: Mutex::new(Waiting::default()),
            arrived: Condvar::new(),
            bound,
        }
    }

    fn offer(&self, message: Arriving) -> Offered {
        let mut waiting = lock(&self.waiting);
        if waiting.messages.len() >= self.bound {
            waiting.discarded = waiting.discarded.saturating_add(1);
            return Offered::Discarded(waiting.discarded);
        }
        waiting.messages.push_back(message);
        self.arrived.notify_one();
        Offered::Waiting
    }

    /// The oldest waiting message, blocking until there is one. `None` once the
    /// queue is closed and empty — what is waiting is still decided.
    fn take(&self) -> Option<Arriving> {
        let mut waiting = lock(&self.waiting);
        loop {
            if let Some(message) = waiting.messages.pop_front() {
                return Some(message);
            }
            if waiting.closed {
                return None;
            }
            waiting = self
                .arrived
                .wait(waiting)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    fn close(&self) {
        lock(&self.waiting).closed = true;
        self.arrived.notify_all();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        lock(&self.waiting).messages.len()
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
/// once it is reported — on a runtime with the status channel (logos-protocol
/// 0.9). On an older runtime there is no status and `recv()` parks forever: the
/// thread is then leaked, but it is this thread alone, holding nothing any other
/// thread waits on.
fn listen<I>(events: I, queue: &InboundQueue, journal: &dyn Journal)
where
    I: Iterator<Item = Option<Arriving>>,
{
    for event in events {
        match event {
            Some(message) => {
                if let Offered::Discarded(total) = queue.offer(message) {
                    record(
                        journal,
                        Note::Discarded {
                            total,
                            bound: queue.bound,
                        },
                    );
                }
            }
            None => record(journal, Note::Unreadable),
        }
    }
}

struct Processor {
    queue: Arc<InboundQueue>,
    channels: Arc<Channels>,
    stores: Stores,
    journal: Arc<dyn Journal>,
    clock: fn() -> u64,
}

impl Processor {
    fn run(self) {
        while let Some(message) = self.queue.take() {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| self.decide(&message))) {
                record(
                    &*self.journal,
                    Note::Panicked("an inbound message", &panic_detail(&*payload)),
                );
            }
        }
    }

    /// Put one message through the boundary, and log what it decided.
    ///
    /// **The clock is read here, after any wait, when the message is processed** —
    /// never the event's timestamp, which is delivery's reading at receipt, in
    /// nanoseconds. `op-transport` requires the window be judged against this
    /// peer's own clock at processing time.
    fn decide(&self, message: &Arriving) {
        // `op-transport`, "A message arriving on a channel that is not open, but
        // whose opening this peer has requested and delivery has not yet answered,
        // MUST be judged only once that open is settled" (scenarios "…is stored
        // once delivery reports the channel created" and "…is refused once
        // delivery declines the open"). See `ChannelBook` for why, and design
        // Decision 11.
        self.channels
            .await_settled(&message.channel_id, SETTLE_LIMIT);
        let now_ms = (self.clock)();
        let inbound = InboundMessage {
            channel_id: &message.channel_id,
            sender_id: &message.sender_id,
            payload: &message.payload,
            timestamp: message.timestamp,
        };
        // `transport::receive`'s order — channel, then the bytes, then the one
        // write — with the op log opened only for an op that passed: a payload
        // refused on its channel or its bytes costs no database open, and is
        // refused under its own name even when the log will not open.
        let decided = self
            .channels
            .stoa_of(&message.channel_id)
            .ok_or(InboundRefusal::UnknownChannel)
            .and_then(|stoa| transport::judge(inbound, stoa, now_ms))
            .and_then(|judged| {
                // `op-transport`, scenario "A message the op log cannot take is
                // logged and not retried": an op log that will not open is a
                // storage failure, and the message is dropped, not held.
                let mut log = self.stores.op_log().map_err(InboundRefusal::Storage)?;
                transport::admit(judged, &mut log)
            });
        match decided {
            Ok(admitted) => record(&*self.journal, Note::Stored(&admitted.id)),
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
    started: bool,
    outbox: Option<Outbox>,
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
            started: false,
            outbox: None,
        }
    }

    /// Wire delivery, once per process.
    ///
    /// In order: subscribe to `channelMessageReceived` (so nothing a channel
    /// receives can precede the listener), start the processor and the worker,
    /// ask for the node, then ask for the channel of every Stoa the membership
    /// record holds. The node is first in the worker's queue, so its creation is
    /// requested before any channel operation.
    ///
    /// **A second call does nothing and returns `false`**, so node creation and
    /// start are requested at most once per process however often startup runs.
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
        // Its own flag, not `outbox.is_some()`: a worker that failed to start
        // leaves no outbox, and a second call must still do nothing rather than
        // start a second listener and processor.
        //
        // `stoa-membership`, "Startup running again in the same module process
        // MUST NOT request any channel" (scenario "A second startup in one
        // process requests no channel"), beside `op-transport`'s node asked for
        // once. A second call asks for nothing at all, because the first call
        // already asked for every membership's.
        if self.started {
            record(&*self.journal, Note::AlreadyStarted);
            return false;
        }
        self.started = true;
        let channels = Arc::new(Channels::default());
        let queue = Arc::new(InboundQueue::with_bound(INBOUND_BOUND));

        // `op-transport`, scenario "A peer that cannot subscribe still publishes":
        // a failed subscription is logged and startup carries on — the node, the
        // channels and the sends are still requested — because a peer that
        // cannot receive can still publish.
        match subscribe() {
            Ok(events) => self.spawn_listener(events, Arc::clone(&queue)),
            Err(why) => record(&*self.journal, Note::NotSubscribed(&why)),
        }
        self.spawn(
            "dialectica inbound processor",
            Processor {
                queue,
                channels: Arc::clone(&channels),
                stores: stores.clone(),
                journal: Arc::clone(&self.journal),
                clock,
            },
            Processor::run,
        );

        let (actions, pending) = mpsc::channel();
        let worker = Worker {
            delivery,
            channels,
            stores: stores.clone(),
            journal: Arc::clone(&self.journal),
        };
        if !self.spawn("dialectica delivery worker", worker, move |w| {
            w.run(pending)
        }) {
            return true;
        }
        let outbox = Outbox { actions };
        let _ = outbox.actions.send(Action::StartNode);
        match stores.memberships() {
            Ok(stoas) => {
                for stoa in stoas {
                    let _ = outbox.actions.send(Action::Open(stoa));
                }
            }
            Err(why) => record(&*self.journal, Note::MembershipUnreadable(&why)),
        }
        self.outbox = Some(outbox);
        true
    }

    /// A membership was recorded: ask for its Stoa's channel.
    pub fn joined(&self, stoa: &Address) {
        self.request(Action::Open(*stoa), || {
            format!("the channel for Stoa {}", stoa.to_hex())
        });
    }

    /// An op was published: ask for it to be sent on its Stoa's channel.
    pub fn published(&self, id: &OpId) {
        self.request(Action::Send(*id), || {
            format!("the send of op {}", id.to_hex())
        });
    }

    fn request(&self, action: Action, what: impl Fn() -> String) {
        match &self.outbox {
            // `op-transport`, scenario "A publish before delivery is wired is not
            // sent when it is", and `stoa-membership`, "A join before delivery is
            // wired has its channel requested once, by startup": a request made
            // before delivery is wired is logged and dropped, not held. Startup
            // asks for every membership's channel anyway; an op published then is
            // not re-sent.
            None => record(&*self.journal, Note::NotStarted(&what())),
            Some(outbox) => {
                if outbox.actions.send(action).is_err() {
                    record(&*self.journal, Note::WorkerGone(&what()));
                }
            }
        }
    }

    fn spawn_listener<I>(&self, events: I, queue: Arc<InboundQueue>)
    where
        I: Iterator<Item = Option<Arriving>> + Send + 'static,
    {
        let journal = Arc::clone(&self.journal);
        self.spawn("dialectica inbound listener", events, move |events| {
            if let Err(payload) =
                catch_unwind(AssertUnwindSafe(|| listen(events, &queue, &*journal)))
            {
                record(
                    &*journal,
                    Note::Panicked("the inbound listener", &panic_detail(&*payload)),
                );
            }
            record(&*journal, Note::ListenerEnded);
            queue.close();
        });
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
    /// before it.
    #[cfg(test)]
    fn settle(&self) {
        let (done, wait) = mpsc::channel();
        if let Some(outbox) = &self.outbox {
            let _ = outbox.actions.send(Action::Settle(done));
            wait.recv_timeout(Duration::from_secs(30))
                .expect("the worker settles");
        }
    }
}

#[cfg(test)]
mod tests;
