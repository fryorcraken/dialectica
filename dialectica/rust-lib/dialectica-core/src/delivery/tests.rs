//! The delivery wiring's tests: the worker and processor driven directly where
//! the order of events is the point, and the whole of [`Delivering`] — threads
//! and all — where the point is that a reply does not wait.
//!
//! The fake delivery answers **from its input**: a channel creation echoes the
//! channel id, a send answers with a request id counting its payload. A fake
//! answering the same thing for every call could not tell "sent on this
//! channel" from "sent on some channel".

use super::*;
use crate::authoring::Authorship;
use crate::identity::SecretKey;
use crate::op::{Op, OpClock, OpKind, SignedOp};
use crate::stoa::{Genesis, Policy};
use serde_json::{json, Value};
use std::time::Instant;

/// This peer's clock in every test. 2026-09-18T11:01:44Z.
const NOW_MS: u64 = 1_789_729_304_000;

fn now() -> u64 {
    NOW_MS
}

fn key(seed: u8) -> SecretKey {
    SecretKey::from_bytes(&[seed; 32]).unwrap()
}

fn genesis(title: &str) -> Genesis {
    Genesis {
        creator: key(1).public_key(),
        policy: Policy::Open,
        title: title.to_string(),
    }
}

fn join_request(g: &Genesis) -> String {
    json!({
        "stoa": g.address().unwrap().to_hex(),
        "genesis": hex::encode(g.canonical_bytes().unwrap()),
    })
    .to_string()
}

/// A fresh temporary directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("dialectica-delivery-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
        TempDir(path)
    }

    fn stores(&self) -> Stores {
        Stores::in_dir(self.0.clone())
    }

    fn memberships(&self) -> MembershipStore {
        MembershipStore::open(&membership_path_in(&self.0)).unwrap()
    }

    fn op_log(&self) -> SqliteOpLog {
        SqliteOpLog::open(&crate::log::op_log_path_in(&self.0)).unwrap()
    }

    /// Make one of the stores unusable by putting a directory where its file goes.
    fn break_file(&self, file: &std::path::Path) {
        std::fs::create_dir_all(file).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The module's log, as lines a test can read.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Journal for Recorder {
    fn record(&self, line: &str) {
        lock(&self.0).push(line.to_string());
    }
}

impl Recorder {
    fn lines(&self) -> Vec<String> {
        lock(&self.0).clone()
    }

    fn with(&self, needle: &str) -> Vec<String> {
        self.lines()
            .into_iter()
            .filter(|l| l.contains(needle))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Call {
    CreateNode(String),
    StartNode,
    ChannelCreate {
        channel_id: String,
        content_topic: String,
        sender_id: String,
    },
    ChannelSend {
        channel_id: String,
        payload: Vec<u8>,
    },
}

/// A door a fake delivery call waits at until the test opens it: delivery that
/// has been asked and has not answered.
///
/// **Instead of a sleep**, so "the reply did not wait" is read off what has
/// happened rather than off a clock: a call held here has not returned, however
/// slow the machine, and one the test has not released cannot return by itself.
/// The wait is capped only so a test that fails leaves no thread parked for
/// good; the cap is longer than any `eventually` in this file, so a caller that
/// waited on the call would still be waiting when the test gives up on it.
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn closed() -> Arc<Gate> {
        Arc::new(Gate {
            open: Mutex::new(false),
            changed: Condvar::new(),
        })
    }

    fn release(&self) {
        *lock(&self.open) = true;
        self.changed.notify_all();
    }

    fn wait(&self) {
        let open = lock(&self.open);
        let _ = self
            .changed
            .wait_timeout_while(open, Duration::from_secs(15), |open| !*open);
    }
}

/// What the fake answers, set per test.
struct Script {
    create_node: Result<Value, String>,
    start_node: Result<Value, String>,
    /// Channel ids whose creation is declined, with the reply to give.
    declined_channels: HashMap<String, Result<Value, String>>,
    /// Replies to the next channel creations, in order, whichever channel; once
    /// they run out a creation is answered as created.
    create_replies: VecDeque<Result<Value, String>>,
    send: Option<Result<Value, String>>,
    /// Held until released: a node creation, a channel creation, or a send,
    /// delivery has not answered.
    node_gate: Option<Arc<Gate>>,
    create_gate: Option<Arc<Gate>>,
    send_gate: Option<Arc<Gate>>,
    panic_on_create_node: bool,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            create_node: Ok(json!(true)),
            start_node: Ok(json!(true)),
            declined_channels: HashMap::new(),
            create_replies: VecDeque::new(),
            send: None,
            node_gate: None,
            create_gate: None,
            send_gate: None,
            panic_on_create_node: false,
        }
    }
}

#[derive(Default)]
struct FakeState {
    calls: Mutex<Vec<Call>>,
    script: Mutex<Script>,
    /// How many channel creations and sends have answered, as opposed to been
    /// asked: a call held at a [`Gate`] is in `calls` and not in these.
    answered: Mutex<(usize, usize)>,
}

#[derive(Clone, Default)]
struct Fake(Arc<FakeState>);

impl Fake {
    fn calls(&self) -> Vec<Call> {
        lock(&self.0.calls).clone()
    }

    fn script(&self, edit: impl FnOnce(&mut Script)) -> Self {
        edit(&mut lock(&self.0.script));
        self.clone()
    }

    fn creates(&self) -> Vec<(String, String, String)> {
        self.calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::ChannelCreate {
                    channel_id,
                    content_topic,
                    sender_id,
                } => Some((channel_id, content_topic, sender_id)),
                _ => None,
            })
            .collect()
    }

    fn sends(&self) -> Vec<(String, Vec<u8>)> {
        self.calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::ChannelSend {
                    channel_id,
                    payload,
                } => Some((channel_id, payload)),
                _ => None,
            })
            .collect()
    }

    fn count(&self, wanted: &Call) -> usize {
        self.calls().iter().filter(|c| *c == wanted).count()
    }

    /// Channel creations delivery has answered.
    fn answered_creates(&self) -> usize {
        lock(&self.0.answered).0
    }

    /// Sends delivery has answered.
    fn answered_sends(&self) -> usize {
        lock(&self.0.answered).1
    }
}

impl Delivery for Fake {
    fn create_node(&self, config: &str) -> Result<Value, String> {
        lock(&self.0.calls).push(Call::CreateNode(config.to_string()));
        let (gate, panics, reply) = {
            let script = lock(&self.0.script);
            (
                script.node_gate.clone(),
                script.panic_on_create_node,
                script.create_node.clone(),
            )
        };
        if panics {
            panic!("the fake delivery panicked creating a node");
        }
        if let Some(gate) = gate {
            gate.wait();
        }
        reply
    }

    fn start_node(&self) -> Result<Value, String> {
        lock(&self.0.calls).push(Call::StartNode);
        lock(&self.0.script).start_node.clone()
    }

    fn channel_create(
        &self,
        channel_id: &str,
        content_topic: &str,
        sender_id: &str,
    ) -> Result<Value, String> {
        lock(&self.0.calls).push(Call::ChannelCreate {
            channel_id: channel_id.to_string(),
            content_topic: content_topic.to_string(),
            sender_id: sender_id.to_string(),
        });
        let (gate, queued, declined) = {
            let mut script = lock(&self.0.script);
            (
                script.create_gate.clone(),
                script.create_replies.pop_front(),
                script.declined_channels.get(channel_id).cloned(),
            )
        };
        if let Some(gate) = gate {
            gate.wait();
        }
        lock(&self.0.answered).0 += 1;
        queued.or(declined).unwrap_or_else(|| Ok(json!(channel_id)))
    }

    fn channel_send(&self, channel_id: &str, payload: &[u8]) -> Result<Value, String> {
        lock(&self.0.calls).push(Call::ChannelSend {
            channel_id: channel_id.to_string(),
            payload: payload.to_vec(),
        });
        let (gate, scripted) = {
            let script = lock(&self.0.script);
            (script.send_gate.clone(), script.send.clone())
        };
        if let Some(gate) = gate {
            gate.wait();
        }
        lock(&self.0.answered).1 += 1;
        scripted.unwrap_or_else(|| Ok(json!(format!("request-{}", payload.len()))))
    }
}

/// The reply delivery was measured giving when it declined a call.
fn the_observed_decline(reason: &str) -> Result<Value, String> {
    Ok(json!({ "error": reason, "success": false, "value": null }))
}

/// A peer: a directory of stores, a log to read, and the wiring under test.
struct Peer {
    dir: TempDir,
    journal: Arc<Recorder>,
    delivering: Delivering,
    fake: Fake,
}

impl Peer {
    fn new(name: &str) -> Self {
        let journal = Arc::new(Recorder::default());
        Peer {
            dir: TempDir::new(name),
            delivering: Delivering::new(journal.clone()),
            journal,
            fake: Fake::default(),
        }
    }

    /// Start the wiring with a listener that delivers nothing.
    fn start(&mut self) -> bool {
        self.delivering
            .start(self.fake.clone(), self.dir.stores(), now, || {
                Ok(std::iter::empty::<Option<Arriving>>())
            })
    }

    /// Start the wiring with a listener fed from the returned sender.
    fn start_listening(&mut self) -> mpsc::Sender<Option<Arriving>> {
        let (events, feed) = mpsc::channel();
        self.delivering
            .start(self.fake.clone(), self.dir.stores(), now, move || {
                Ok(feed.into_iter())
            });
        events
    }

    fn join(&self, g: &Genesis) -> String {
        let delivering = &self.delivering;
        crate::wire::join_stoa(&join_request(g), &mut self.dir.memberships(), &mut |stoa| {
            delivering.joined(stoa)
        })
    }

    fn post(&self, stoa: &Address, body: &str) -> OpId {
        let delivering = &self.delivering;
        let reply = crate::wire::publish_post(
            &json!({ "stoa": stoa.to_hex(), "body": body }).to_string(),
            &mut self.dir.op_log(),
            &Authorship {
                key: &key(7),
                asserted_ms: NOW_MS,
            },
            &mut |id| delivering.published(id),
        );
        op_id_of(&reply)
    }

    fn worker(&self) -> Worker<Fake> {
        Worker {
            delivery: self.fake.clone(),
            channels: Arc::new(Channels::default()),
            stores: self.dir.stores(),
            journal: self.journal.clone(),
        }
    }

    fn processor(&self, channels: Arc<Channels>) -> Processor {
        Processor {
            queue: Arc::new(InboundQueue::with_bound(INBOUND_BOUND)),
            channels,
            stores: self.dir.stores(),
            journal: self.journal.clone(),
            clock: now,
            settle_limit: SETTLE_LIMIT,
        }
    }
}

fn op_id_of(reply: &str) -> OpId {
    let v: Value = serde_json::from_str(reply).unwrap();
    assert!(v.get("error").is_none(), "the publish was refused: {reply}");
    OpId::from_hex(v["opId"].as_str().unwrap()).unwrap()
}

/// Channels with one Stoa's channel open.
fn open_for(stoa: &Address) -> Arc<Channels> {
    let channels = Arc::new(Channels::default());
    let mut opening = channels.opening(&ChannelIdentity::of(stoa));
    opening.held();
    drop(opening);
    channels
}

/// An op another peer signed into `stoa`, with a counter `ahead_ms` past NOW.
fn their_op(stoa: Address, body: &str, ahead_ms: u64) -> SignedOp {
    let author = key(5);
    Op {
        stoa,
        author: author.public_key(),
        clock: Some(OpClock {
            counter: NOW_MS + ahead_ms,
            asserted_ms: NOW_MS,
        }),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(&author)
}

fn arriving(channel_id: &str, payload: Vec<u8>, timestamp: i64) -> Arriving {
    Arriving {
        channel_id: channel_id.to_string(),
        sender_id: "a-sender-that-chose-this".to_string(),
        payload,
        timestamp,
    }
}

fn stored(peer: &Peer, id: &OpId) -> Option<crate::log::Entry> {
    peer.dir.op_log().get(id).unwrap()
}

/// Wait, up to a bound, for something another thread does.
fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting: {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ─── The node ─────────────────────────────────────────────────────────────

#[test]
fn the_nodes_configuration_names_the_reliable_channel_layer() {
    let config: Value = serde_json::from_str(&node_config()).unwrap();
    assert_eq!(config["entryLayer"], "channels");
}

#[test]
fn the_node_preset_and_mode_are_pinned() {
    // Peers on different presets never meet, so a change here partitions this
    // build from every other. `design.md` records why these two.
    let config: Value = serde_json::from_str(&node_config()).unwrap();
    assert_eq!(config["preset"], "logos.test");
    assert_eq!(config["mode"], "Edge");
}

#[test]
fn node_creation_is_requested_once_however_often_startup_runs() {
    let mut peer = Peer::new("start-twice");
    peer.join(&genesis("Agora"));
    assert!(peer.start());
    assert!(!peer.start(), "a second startup must report it did nothing");
    peer.delivering.settle();

    assert_eq!(peer.fake.count(&Call::CreateNode(node_config())), 1);
    assert_eq!(peer.fake.count(&Call::StartNode), 1);
    // `stoa-membership`, scenario "A second startup in one process requests no
    // channel": the peer is in one Stoa, and its channel is requested once.
    assert_eq!(peer.fake.creates().len(), 1);
}

#[test]
fn a_failed_subscription_leaves_sending_wired() {
    // `op-transport`, scenario "A peer that cannot subscribe still publishes": the
    // failure is logged, channel creation is requested and the post is sent.
    let mut peer = Peer::new("no-subscription");
    let g = genesis("Agora");
    peer.join(&g);
    peer.delivering
        .start(peer.fake.clone(), peer.dir.stores(), now, || {
            Err::<std::iter::Empty<Option<Arriving>>, _>("provider unavailable".to_string())
        });
    peer.post(&g.address().unwrap(), "still sent");
    peer.delivering.settle();

    // The spec asks the log to record the *consequence* — "this peer will not
    // receive ops from other peers" — and not only the failure, so a line that
    // said "could not subscribe" alone would leave an operator not knowing what it
    // cost. Asserted on the consequence's words, case-insensitively.
    let logged: Vec<String> = peer
        .journal
        .lines()
        .into_iter()
        .filter(|l| {
            l.to_lowercase()
                .contains("will not receive ops from other peers")
        })
        .collect();
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains("provider unavailable"), "{}", logged[0]);
    assert_eq!(peer.fake.count(&Call::CreateNode(node_config())), 1);
    assert_eq!(peer.fake.creates().len(), 1);
    assert_eq!(peer.fake.sends().len(), 1);
}

#[test]
fn a_publish_before_delivery_is_wired_is_logged_and_not_sent_later() {
    // `op-transport`, scenario "A publish before delivery is wired is not sent
    // when it is": the op id is logged as not sent, nothing is sent before or
    // after startup, and startup requests the Stoa's channel once (that half is
    // `stoa-membership`'s "A join before delivery is wired has its channel
    // requested once, by startup").
    let mut peer = Peer::new("before-start");
    let g = genesis("Agora");
    peer.join(&g);
    let id = peer.post(&g.address().unwrap(), "too early");
    assert_eq!(
        peer.journal
            .with(&format!("the send of op {} was not requested", id.to_hex()))
            .len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    peer.start();
    peer.delivering.settle();
    assert!(peer.fake.sends().is_empty());
    assert_eq!(peer.fake.creates().len(), 1);
}

#[test]
fn a_declined_repeat_open_leaves_an_open_channel_open() {
    // `stoa-membership`, scenario "A declined repeat request leaves an open
    // channel open", read off the channel book. **This alone cannot tell the
    // right implementation from one that does nothing on a decline**; it goes red
    // only against closing the channel, and
    // `a_declined_repeat_open_leaves_the_channel_open_for_the_next_message` states
    // the scenario through a message and is the one that reads as the spec does.
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = open_for(&stoa);
    drop(channels.opening(&identity)); // asked again, declined
    assert!(lock(&channels.book).open.is_open(identity.channel_id()));
}

#[test]
fn an_op_log_that_will_not_open_is_a_storage_refusal() {
    // `op-transport`, scenario "A message the op log cannot take is logged and
    // not retried": the storage failure is logged. That the op is not in the log
    // afterwards is `a_message_the_op_log_cannot_take_is_logged_and_not_retried`'s.
    let peer = Peer::new("recv-no-log");
    peer.dir
        .break_file(&crate::log::op_log_path_in(&peer.dir.0));
    let stoa = genesis("Agora").address().unwrap();
    let op = their_op(stoa, "nowhere to put it", 0);
    peer.processor(open_for(&stoa)).decide(&arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        op.to_bytes().unwrap(),
        1,
    ));
    assert_eq!(
        peer.journal.with("refused (storage").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn node_creation_precedes_every_channel_operation() {
    let mut peer = Peer::new("node-first");
    peer.join(&genesis("Agora"));
    peer.join(&genesis("Lyceum"));
    peer.start();
    peer.delivering.settle();

    let calls = peer.fake.calls();
    assert_eq!(calls[0], Call::CreateNode(node_config()), "{calls:?}");
    assert_eq!(calls[1], Call::StartNode, "{calls:?}");
    assert_eq!(peer.fake.creates().len(), 2, "{calls:?}");
}

#[test]
fn a_declined_node_creation_does_not_stop_the_module() {
    let mut peer = Peer::new("node-declined");
    peer.fake = peer.fake.script(|s| {
        s.create_node = the_observed_decline("Context already initialized");
    });
    let agora = genesis("Agora");
    let lyceum = genesis("Lyceum");
    peer.join(&agora);
    peer.join(&lyceum);
    let posted = peer.post(&agora.address().unwrap(), "held before start");
    peer.start();
    peer.delivering.settle();

    assert_eq!(
        peer.journal.with("Context already initialized").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    let created: Vec<String> = peer.fake.creates().into_iter().map(|c| c.0).collect();
    for g in [&agora, &lyceum] {
        let id = ChannelIdentity::of(&g.address().unwrap());
        assert!(
            created.contains(&id.channel_id().to_string()),
            "{created:?}"
        );
    }
    // `op-transport`, "Start SHALL be requested only after delivery has accepted
    // this application's creation": the scenario's "node start is not requested".
    assert_eq!(peer.fake.count(&Call::StartNode), 0);

    // A call that reads the op log answers as it would have otherwise.
    let listed = crate::wire::list_threads_from_request(&join_request(&agora), || {
        Ok::<_, OpLogError>(peer.dir.op_log())
    });
    assert!(listed.contains(&posted.to_hex()), "{listed}");
}

#[test]
fn the_adapter_never_stops_a_node_and_creates_one_at_one_site() {
    // `op-transport`: "a stop call SHALL NOT appear in this application at all",
    // checked by reading the application. This reads the one file that can make
    // a delivery call. Comments are stripped first, so prose saying "never
    // `stop()`" does not read as the call it forbids.
    //
    // **By the method's prefix and not its exact spelling**, over the call chain
    // as rustfmt breaks it across lines: a generated `stop_async` or a renamed
    // `stop_with_timeout` is still a stop, and a second `start_async` is still a
    // second site that starts the node.
    let code = adapter_code();
    let on_delivery = methods_called_on(&code, "delivery_module");
    assert!(
        !code.contains(".stop"),
        "the adapter calls a stop: {on_delivery:?}"
    );
    assert_eq!(
        on_delivery
            .iter()
            .filter(|m| m.starts_with("create_node"))
            .count(),
        1,
        "exactly one site asks delivery to create the node: {on_delivery:?}"
    );
    assert_eq!(
        on_delivery
            .iter()
            .filter(|m| m.starts_with("start"))
            .count(),
        1,
        "exactly one site asks delivery to start the node: {on_delivery:?}"
    );
    // And nothing on this side of the seam can stop one: `Delivery` has no
    // method for it, so the worker cannot be written to call it.
}

#[test]
fn only_reliable_channel_receipts_reach_the_op_log() {
    // The adapter's subscriptions, read off its source: `channelMessageReceived`
    // and nothing else. A report on this peer's own send (`channelMessageSent`),
    // or a plain `messageReceived` outside a channel, has no route to the queue
    // because nothing subscribes to it.
    let code = adapter_code();
    let subscriptions: Vec<&str> = code
        .match_indices(".on_")
        .map(|(i, _)| {
            let rest = &code[i + 1..];
            &rest[..rest.find('(').unwrap_or(rest.len())]
        })
        .collect();
    assert_eq!(subscriptions, ["on_channel_message_received"]);
    // A route that is not spelled `.on_…` — a differently named subscription, or
    // a decoder for another event — still names the event it carries. Every
    // mention of a received message in the adapter is the reliable channel's, and
    // no report on a send or a failed one is named at all.
    assert_eq!(
        code.matches("message_received").count(),
        code.matches("channel_message_received").count(),
        "the adapter names a received message that is not a reliable channel's"
    );
    for other in ["message_sent", "message_error", "message_propagated"] {
        assert!(!code.contains(other), "the adapter names `{other}`");
    }
}

#[test]
fn the_adapter_maps_each_event_field_to_the_field_of_the_same_name() {
    // `channel_id` and `sender_id` are both `String`: a mapping that swapped them
    // compiles, and `nix build ./dialectica#lgx` is the only gate that compiles
    // the adapter at all, so a swap would refuse every message as arriving on an
    // unknown channel with every gate green. This reads the mapping as text —
    // whitespace removed, so rustfmt's wrapping does not matter — and is a pin
    // on the pairing, not a substitute for a test that runs the decoder.
    let code: String = adapter_code().split_whitespace().collect();
    for field in ["channel_id", "sender_id", "payload", "timestamp"] {
        assert!(
            code.contains(&format!("{field}:m.{field},")),
            "the adapter does not map `{field}` from the event's `{field}`"
        );
    }
}

#[test]
fn the_adapter_hands_delivery_every_recorded_membership_and_every_published_op() {
    // `op-transport`, "Every publish that succeeds … is handed to delivery", and
    // `stoa-membership`, "Creating or joining a Stoa opens its reliable channel":
    // the central requirement of #176 ("No op ever leaves the authoring peer"),
    // and the one place it can be lost with every gate cargo runs green.
    //
    // **Why a text pin.** Every test above drives `Delivering` through a sink the
    // *test* writes (`|id| delivering.published(id)`); the closure the adapter
    // passes is in `dialectica/rust-lib/src/lib.rs`, behind `cfg(logos_scaffold)`,
    // which `cargo test` does not compile and `nix build ./dialectica#lgx` compiles
    // without running. A sink replaced by `|_id| {}` compiles, links, and sends
    // nothing. Reading the text is the one layer of this repo that can see it.
    //
    // Whitespace is removed, so rustfmt's wrapping does not matter; comments are
    // stripped by `adapter_code`. The counts are hardcoded: three handlers reach
    // `publishing`, one closure serves them, two handlers pass the join sink, and
    // startup is one call site.
    let code: String = adapter_code().split_whitespace().collect();
    let expected = [
        // The one helper every publish handler goes through, and its sink.
        ("&mut|id|delivery.published(id)", 1),
        ("self.publishing(&request,core::publish_post)", 1),
        ("self.publishing(&request,core::publish_reply)", 1),
        ("self.publishing(&request,core::publish_vote)", 1),
        // The create and join handlers' sinks.
        ("&mut|stoa|self.delivery.joined(stoa)", 2),
        // Startup.
        ("self.delivery.start(", 1),
    ];
    for (text, count) in expected {
        assert_eq!(
            code.matches(text).count(),
            count,
            "the adapter should contain `{text}` exactly {count} time(s)"
        );
    }
    // A sink that ignores what it is handed: whatever its argument is called, a
    // closure taking a discarded parameter is the shape of the defect.
    for ignoring in ["|_id|", "|_stoa|", "|_|", "|_op|", "|_op_id|"] {
        assert!(
            !code.contains(ignoring),
            "the adapter contains a closure that ignores its argument: `{ignoring}`"
        );
    }
}

#[test]
fn the_adapter_forwards_each_delivery_argument_in_the_order_the_seam_names_them() {
    // `channel_id`, `content_topic` and `sender_id` are three adjacent `&str`s:
    // swapping two compiles, and the test doubles stand on the trait's side of the
    // seam, so nothing observes what the adapter forwards to the generated client.
    // A swap would create every channel under the topic's name and miss every
    // send and receive, with every gate green; the inbound mapping has its own
    // pin (`the_adapter_maps_each_event_field_to_the_field_of_the_same_name`), and
    // this is the outbound one. Read as text, whitespace removed.
    let code: String = adapter_code().split_whitespace().collect();
    for forwarded in [
        "create_node_with_timeout(config,",
        "channel_create_with_timeout(channel_id,content_topic,sender_id,",
        "channel_send_with_timeout(channel_id,payload,",
    ] {
        assert_eq!(
            code.matches(forwarded).count(),
            1,
            "the adapter does not forward `{forwarded}`"
        );
    }
}

/// The adapter's source without its comments.
fn adapter_code() -> String {
    include_str!("../../../src/lib.rs")
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every method called directly on `receiver` in `code`, as a call chain reads
/// once its line breaks are joined: `receiver\n    .method(` is `receiver.method(`.
fn methods_called_on(code: &str, receiver: &str) -> Vec<String> {
    let joined = code
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" .", ".");
    let head = format!("{receiver}.");
    joined
        .match_indices(&head)
        .map(|(at, _)| {
            joined[at + head.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

// ─── Opening channels ─────────────────────────────────────────────────────

#[test]
fn joining_requests_the_stoas_channel_after_the_membership_is_recorded() {
    let peer = Peer::new("join-opens");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let mut seen_recorded = None;
    let reply = crate::wire::join_stoa(&join_request(&g), &mut peer.dir.memberships(), &mut |s| {
        // Read through a SECOND connection, so the answer is what the file
        // holds rather than what the handler's own connection is part-way into.
        seen_recorded = Some(peer.dir.memberships().contains(s).unwrap());
        peer.delivering.joined(s);
    });
    assert!(!reply.contains("error"), "{reply}");
    assert_eq!(seen_recorded, Some(true));

    // And the channel asked for is the one derived from that Stoa's address.
    let mut peer = peer;
    peer.start();
    peer.delivering.settle();
    let id = ChannelIdentity::of(&stoa);
    let creates = peer.fake.creates();
    assert!(creates
        .iter()
        .all(|c| c.0 == id.channel_id() && c.1 == id.content_topic()));
}

#[test]
fn a_join_requests_its_channel_through_the_worker() {
    let mut peer = Peer::new("join-then");
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    peer.delivering.settle();

    let id = ChannelIdentity::of(&g.address().unwrap());
    let creates = peer.fake.creates();
    assert_eq!(creates.len(), 1, "{creates:?}");
    assert_eq!(creates[0].0, id.channel_id());
    assert_eq!(creates[0].1, id.content_topic());
}

#[test]
fn creating_a_stoa_requests_its_channel() {
    let mut peer = Peer::new("create-opens");
    peer.start();
    let delivering = &peer.delivering;
    let creator = key(3).public_key();
    let reply = crate::wire::create_stoa(
        r#"{"title":"Agora"}"#,
        || Ok(creator),
        &mut peer.dir.memberships(),
        &mut |stoa| delivering.joined(stoa),
    );
    let v: Value = serde_json::from_str(&reply).unwrap();
    let stoa = Address::from_hex(v["stoa"].as_str().unwrap()).unwrap();
    peer.delivering.settle();

    let id = ChannelIdentity::of(&stoa);
    assert_eq!(
        peer.fake
            .creates()
            .first()
            .map(|c| (c.0.clone(), c.1.clone())),
        Some((id.channel_id().to_string(), id.content_topic().to_string()))
    );
}

#[test]
fn a_refused_join_requests_no_channel() {
    // The record names one Stoa and the address another: refused, and neither
    // address reaches the sink.
    let peer = Peer::new("join-refused");
    let record = genesis("Agora");
    let claimed = genesis("Lyceum").address().unwrap();
    let request = json!({
        "stoa": claimed.to_hex(),
        "genesis": hex::encode(record.canonical_bytes().unwrap()),
    })
    .to_string();
    let mut asked: Vec<Address> = Vec::new();
    let reply = crate::wire::join_stoa(&request, &mut peer.dir.memberships(), &mut |s| {
        asked.push(*s)
    });
    assert!(reply.contains("error"), "{reply}");
    assert!(asked.is_empty(), "a refused join asked for {asked:?}");
}

#[test]
fn a_channel_delivery_declines_does_not_fail_the_join() {
    let mut peer = Peer::new("join-declined");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let id = ChannelIdentity::of(&stoa);
    peer.fake = peer.fake.script(|s| {
        s.declined_channels.insert(
            id.channel_id().to_string(),
            the_observed_decline("no reliable channel manager"),
        );
    });
    let events = peer.start_listening();
    let reply = peer.join(&g);
    peer.delivering.settle();

    // The reply is the one a join gets whether or not the channel opened.
    let unaffected = crate::wire::join_stoa(
        &join_request(&g),
        &mut MembershipStore::in_memory().unwrap(),
        &mut |_| {},
    );
    assert_eq!(reply, unaffected);
    assert!(peer.dir.memberships().contains(&stoa).unwrap());
    let logged = peer.journal.with("no reliable channel manager");
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains(&stoa.to_hex()));

    // And the channel is not open: a message on it afterwards, through the
    // running listener and processor, is refused as an unknown channel.
    let their = their_op(stoa, "arrives on a declined channel", 0);
    events
        .send(Some(arriving(
            id.channel_id(),
            their.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the message to be judged", || {
        !peer.journal.with("(unknown-channel)").is_empty()
    });
    assert!(stored(&peer, &their.op.id()).is_none());
}

/// Delivery's answer to a `channelCreate` for a channel its manager already
/// holds, as delivery v0.2.1 words it: at `logos-delivery` `bfdb5afd` (the rev
/// v0.2.1's `flake.lock` pins), `channel_lifecycle.nim` ("channel already exists:
/// " & channelId) behind the C API's "ChannelCreate failed: " prefix
/// (`library/channels_api/channel_api.nim`).
fn the_already_exists_answer(stoa: &Address) -> Result<Value, String> {
    the_observed_decline(&format!(
        "ChannelCreate failed: channel already exists: {}",
        ChannelIdentity::of(stoa).channel_id()
    ))
}

#[test]
fn a_channel_delivery_reports_already_existing_is_open() {
    // `stoa-membership`, scenario "A channel delivery reports already existing is
    // open": a message on it afterwards is stored, and a post afterwards is sent
    // on it. Red while "already exists" was read as a decline.
    let mut peer = Peer::new("join-already-exists");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let _ = peer
        .fake
        .script(|s| s.create_replies.push_back(the_already_exists_answer(&stoa)));
    let events = peer.start_listening();
    peer.join(&g);
    peer.delivering.settle();

    let their = their_op(stoa, "on a channel delivery already held", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            their.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the op to be judged", || {
        !peer.journal.with("inbound").is_empty()
    });
    assert!(
        stored(&peer, &their.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );

    peer.post(&stoa, "sent on the channel delivery already held");
    peer.delivering.settle();
    let sends = peer.fake.sends();
    assert_eq!(sends.len(), 1, "{:?}", peer.journal.lines());
    assert_eq!(sends[0].0, ChannelIdentity::of(&stoa).channel_id());
}

#[test]
fn a_creation_delivery_did_not_complete_in_time_opens_on_the_next_request() {
    // `stoa-membership`, scenario "A creation delivery did not complete in time
    // opens on the next request" — the correctness review's probe. delivery
    // v0.2.1 gives up on its own 30 s callback and answers a timeout while its
    // runtime creates the channel anyway; the repeated join is then answered
    // "already exists". Red while that answer was read as a decline: the
    // channel stayed shut for as long as delivery ran.
    let mut peer = Peer::new("join-timeout-then-exists");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let _ = peer.fake.script(|s| {
        s.create_replies
            .push_back(the_observed_decline("channel_create callback timeout"));
        s.create_replies.push_back(the_already_exists_answer(&stoa));
    });
    let events = peer.start_listening();
    peer.join(&g);
    peer.delivering.settle();
    peer.join(&g);
    peer.delivering.settle();
    assert_eq!(peer.fake.creates().len(), 2);

    let their = their_op(stoa, "delivered on the channel delivery holds", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            their.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the op to be judged", || {
        !peer.journal.with("inbound").is_empty()
    });
    assert!(
        stored(&peer, &their.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
    assert!(peer.journal.with("unknown-channel").is_empty());
}

#[test]
fn a_module_restarted_while_delivery_kept_running_has_its_channels_open() {
    // `stoa-membership`, scenario "A module restarted while delivery kept
    // running has its channels open": startup's creation is answered "already
    // exists", because delivery kept the channel from the module's last run.
    let mut peer = Peer::new("restart-already-exists");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g); // the previous run's membership
    let _ = peer
        .fake
        .script(|s| s.create_replies.push_back(the_already_exists_answer(&stoa)));
    let events = peer.start_listening();
    peer.delivering.settle();

    let their = their_op(stoa, "after the restart", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            their.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the op to be judged", || {
        !peer.journal.with("inbound").is_empty()
    });
    assert!(
        stored(&peer, &their.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn only_delivery_s_already_exists_answer_opens_a_declined_channel() {
    // The other half of the rule: an error envelope for any other reason is
    // still a decline, and the log still records delivery's reason.
    let stoa = genesis("Agora").address().unwrap();
    assert_eq!(
        channel_answer(&the_already_exists_answer(&stoa)),
        ChannelAnswer::AlreadyHeld
    );
    assert_eq!(
        channel_answer(&the_observed_decline("channel_create callback timeout")),
        ChannelAnswer::Declined("channel_create callback timeout".to_string())
    );
    assert_eq!(
        channel_answer(&Ok(json!(ChannelIdentity::of(&stoa).channel_id()))),
        ChannelAnswer::Created
    );
    // `stoa-membership`, scenario "An \"already exists\" answer that does not name
    // the channel opens it": the words are what is recognised, not the id.
    assert_eq!(
        channel_answer(&the_observed_decline(
            "ChannelCreate failed: channel already exists"
        )),
        ChannelAnswer::AlreadyHeld
    );
    // Scenario "A decline saying something else already exists does not open the
    // channel": a reason without delivery's words is a decline, whatever else it
    // says already exists or is already initialised.
    for reason in [
        "Context already initialized",
        "ChannelCreate failed: content topic already exists",
        "sender already exists",
    ] {
        assert_eq!(
            channel_answer(&the_observed_decline(reason)),
            ChannelAnswer::Declined(reason.to_string()),
            "{reason}"
        );
    }
}

#[test]
fn an_unresponsive_delivery_does_not_delay_a_join() {
    // Delivery is asked to open the channel and does not answer: the creation is
    // held at a gate the test has not opened. What is asserted is not how long
    // the join took but that it returned while delivery had still not answered,
    // which no clock is needed to tell.
    let mut peer = Peer::new("join-slow");
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    peer.start();

    let g = genesis("Agora");
    let reply = peer.join(&g);

    assert!(!reply.contains("error"), "{reply}");
    assert_eq!(
        peer.fake.answered_creates(),
        0,
        "the join returned only after delivery answered the creation"
    );
    assert!(peer
        .dir
        .memberships()
        .contains(&g.address().unwrap())
        .unwrap());

    // And the request was not lost by not waiting: released, it is answered.
    gate.release();
    peer.delivering.settle();
    assert_eq!(peer.fake.answered_creates(), 1);
}

#[test]
fn a_repeated_join_requests_the_channel_again() {
    let mut peer = Peer::new("join-twice");
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    peer.join(&g);
    peer.delivering.settle();
    assert_eq!(peer.fake.creates().len(), 2);
}

#[test]
fn a_restarted_peer_requests_every_stoas_channel_and_no_other() {
    let mut peer = Peer::new("restart-opens");
    let delivering = &peer.delivering;
    let created = crate::wire::create_stoa(
        r#"{"title":"Made here"}"#,
        || Ok(key(3).public_key()),
        &mut peer.dir.memberships(),
        &mut |s| delivering.joined(s),
    );
    let made = Address::from_hex(
        serde_json::from_str::<Value>(&created).unwrap()["stoa"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let joined = genesis("Joined here");
    peer.join(&joined);
    // Ops held for a Stoa nobody joined.
    let stranger = genesis("Known only from ops").address().unwrap();
    peer.post(&stranger, "an op naming a Stoa this peer is not in");

    peer.start();
    peer.delivering.settle();

    let mut asked: Vec<String> = peer.fake.creates().into_iter().map(|c| c.0).collect();
    asked.sort();
    let mut expected = vec![
        ChannelIdentity::of(&made).channel_id().to_string(),
        ChannelIdentity::of(&joined.address().unwrap())
            .channel_id()
            .to_string(),
    ];
    expected.sort();
    assert_eq!(asked, expected);
}

#[test]
fn a_membership_record_of_more_than_a_page_is_read_to_the_end() {
    // `stoa-membership`: startup requests the channel of EVERY Stoa the peer is in.
    // Two full pages and five more, so the page loop must advance twice and stop:
    // a step that never advances re-reads page 0 for ever inside
    // `on_context_ready` (the module answers nothing after), a step that goes
    // backwards underflows, and a step that skips a page loses Stoas — a peer in
    // more than 100 Stoas was deaf in the rest. No other test holds more than a
    // few memberships.
    //
    // The read runs on a thread and the test waits for it with a limit, so a step
    // that never ends fails this test in seconds instead of hanging the suite.
    // (An endless step also grows the vector it appends to, so the limit is short.)
    let peer = Peer::new("memberships-paged");
    let mut store = peer.dir.memberships();
    let total = 2 * MEMBERSHIP_PAGE + 5;
    let mut expected: Vec<Address> = (0..total)
        .map(|n| {
            let g = genesis(&format!("Stoa number {n}"));
            let reply = crate::wire::join_stoa(&join_request(&g), &mut store, &mut |_| {});
            assert!(!reply.contains("error"), "{reply}");
            g.address().unwrap()
        })
        .collect();
    expected.sort();

    let stores = peer.dir.stores();
    let (done, read) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(stores.memberships());
    });
    let mut listed = read
        .recv_timeout(Duration::from_secs(5))
        .expect("reading the memberships never ended, or panicked")
        .expect("the membership record is readable");
    listed.sort();
    assert_eq!(listed.len(), total, "a Stoa was skipped or read twice");
    assert_eq!(listed, expected);
}

#[test]
fn a_peer_in_more_stoas_than_a_page_requests_every_channel_at_startup() {
    // The same property through startup, where it matters: one channel creation
    // per membership past the first page, and none twice.
    let mut peer = Peer::new("start-more-than-a-page");
    let mut store = peer.dir.memberships();
    let total = MEMBERSHIP_PAGE + 3;
    for n in 0..total {
        let g = genesis(&format!("Stoa number {n}"));
        crate::wire::join_stoa(&join_request(&g), &mut store, &mut |_| {});
    }
    peer.start();
    peer.delivering.settle();

    let mut asked: Vec<String> = peer.fake.creates().into_iter().map(|c| c.0).collect();
    asked.sort();
    asked.dedup();
    assert_eq!(asked.len(), total);
    assert_eq!(peer.fake.creates().len(), total);
}

#[test]
fn an_unreadable_membership_record_opens_nothing_and_stops_nothing() {
    let mut peer = Peer::new("membership-broken");
    peer.dir.break_file(&membership_path_in(&peer.dir.0));
    peer.start();
    peer.delivering.settle();

    assert!(peer.fake.creates().is_empty());
    assert_eq!(
        peer.journal.with("could not be read").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    // A later call is answered: the publish path still stores and hands off.
    let stoa = genesis("Agora").address().unwrap();
    let id = peer.post(&stoa, "still answered");
    assert!(stored(&peer, &id).is_some());
}

// ─── The sender identifier ────────────────────────────────────────────────

/// The sender identifier a worker over `dir` supplies for `stoa`.
fn sender_supplied(peer: &Peer, stoa: &Address) -> String {
    let before = peer.fake.creates().len();
    peer.worker().open_now(stoa);
    peer.fake.creates()[before].2.clone()
}

impl Worker<Fake> {
    /// Ask for a Stoa's channel as a create or join would, and perform it now.
    fn open_now(&self, stoa: &Address) {
        self.perform(Action::Open(
            self.channels.opening(&ChannelIdentity::of(stoa)),
        ));
    }
}

#[test]
fn two_installations_holding_one_identity_supply_different_sender_identifiers() {
    // Two installations are two directories, asked about one Stoa. **What can
    // fail this:** any derivation from what the two share — the Stoa address, or a
    // constant. The spec's "holding the same identity" is satisfied by
    // construction, not exercised: no identity reaches this code, so putting one
    // keystore in both directories would change nothing the code can see (see
    // `sender::tests::nothing_a_sender_identifier_is_made_from_is_a_key`).
    let one = Peer::new("sender-install-one");
    let two = Peer::new("sender-install-two");
    let stoa = genesis("Agora").address().unwrap();
    assert_ne!(sender_supplied(&one, &stoa), sender_supplied(&two, &stoa));
}

#[test]
fn a_restart_supplies_the_same_sender_identifier() {
    let peer = Peer::new("sender-restart");
    let stoa = genesis("Agora").address().unwrap();
    let first = sender_supplied(&peer, &stoa);
    // A restart is a new worker over the same directory.
    let second = sender_supplied(&peer, &stoa);
    assert_eq!(first, second);
}

#[test]
fn two_stoas_get_two_sender_identifiers() {
    let peer = Peer::new("sender-two-stoas");
    assert_ne!(
        sender_supplied(&peer, &genesis("Agora").address().unwrap()),
        sender_supplied(&peer, &genesis("Lyceum").address().unwrap())
    );
}

// The spec's "Nothing a sender identifier is made from is a key" is checked by
// reading `sender.rs`, and its test is `sender::tests::
// nothing_a_sender_identifier_is_made_from_is_a_key`. A test here that compared
// the value supplied to `channelCreate` with a key could not fail, because no key
// reaches the code that makes it — the defect the spec-test review found in the
// scenario this replaced.

#[test]
fn a_sender_identifier_that_cannot_be_retained_opens_no_channel() {
    let peer = Peer::new("sender-unretainable");
    peer.dir.break_file(&sender_path_in(&peer.dir.0));
    let stoa = genesis("Agora").address().unwrap();
    let worker = peer.worker();
    worker.open_now(&stoa);

    assert!(peer.fake.creates().is_empty());
    let logged = peer.journal.with("no sender identifier could be retained");
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains(&stoa.to_hex()));
    // And the line says why: the store's own words, not an empty reason
    // (`SenderError`'s rendering replaced by an empty string once passed every test).
    assert!(
        logged[0].contains("the sender identifier store could not be used"),
        "{}",
        logged[0]
    );
    // `op-transport`: an open this peer never goes on to ask delivery for is
    // given up, and settled rather than left pending — or every message on the
    // channel would wait out `SETTLE_LIMIT` before being refused.
    assert!(
        !worker
            .channels
            .is_known(ChannelIdentity::of(&stoa).channel_id()),
        "the open this peer gave up is still counted as being opened"
    );
}

// ─── Publishing ───────────────────────────────────────────────────────────

#[test]
fn an_op_published_on_an_open_channel_is_sent_as_its_stored_wire_form() {
    let mut peer = Peer::new("send-open");
    peer.start();
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g);
    let id = peer.post(&stoa, "hello");
    peer.delivering.settle();

    let sends = peer.fake.sends();
    assert_eq!(sends.len(), 1, "{:?}", peer.fake.calls());
    assert_eq!(sends[0].0, ChannelIdentity::of(&stoa).channel_id());
    let as_stored = stored(&peer, &id).unwrap().op.to_bytes().unwrap();
    assert_eq!(sends[0].1, as_stored);
}

#[test]
fn an_op_the_peer_already_holds_published_again_is_sent_again() {
    let mut peer = Peer::new("send-again");
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    let id = peer.post(&g.address().unwrap(), "twice");
    // The handler reaches the sink for a publish reporting `wasNew:false` exactly
    // as for a new one; this is that second handoff.
    peer.delivering.published(&id);
    peer.delivering.settle();

    let sends = peer.fake.sends();
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[0], sends[1]);
}

#[test]
fn a_publish_into_a_stoa_with_no_open_channel_sends_nothing_and_opens_nothing() {
    let mut peer = Peer::new("send-closed");
    peer.start();
    let stoa = genesis("Not joined").address().unwrap();
    let id = peer.post(&stoa, "into the void");
    peer.delivering.settle();

    assert!(peer.fake.sends().is_empty());
    assert!(peer.fake.creates().is_empty());
    let logged = peer.journal.with("no channel is open");
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains(&id.to_hex()));
    assert!(logged[0].contains(&stoa.to_hex()));
}

#[test]
fn a_send_delivery_declines_leaves_the_op_published() {
    let mut peer = Peer::new("send-declined");
    peer.fake = peer
        .fake
        .script(|s| s.send = Some(Err("the network said no".into())));
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    let before = peer.dir.op_log().len().unwrap();
    let id = peer.post(&g.address().unwrap(), "declined");
    let bytes_at_publish = stored(&peer, &id).unwrap().op.to_bytes().unwrap();
    peer.delivering.settle();

    assert_eq!(peer.dir.op_log().len().unwrap(), before + 1);
    assert_eq!(
        stored(&peer, &id).unwrap().op.to_bytes().unwrap(),
        bytes_at_publish
    );
    let logged = peer.journal.with("the network said no");
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains(&id.to_hex()));
}

#[test]
fn an_unresponsive_delivery_does_not_delay_the_publish_reply() {
    // Delivery is asked to send and does not answer: the send is held at a gate
    // the test has not opened. `post` returning with the reply in hand while
    // that send is still unanswered is the assertion, and it does not depend on
    // how long anything took.
    let mut peer = Peer::new("send-slow");
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.send_gate = Some(gate.clone()));
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    peer.delivering.settle();

    let id = peer.post(&g.address().unwrap(), "not waited on");

    assert_eq!(
        peer.fake.answered_sends(),
        0,
        "the reply came back only after delivery answered the send"
    );
    assert!(stored(&peer, &id).is_some());

    // And the send was not lost by not waiting: released, it is answered.
    gate.release();
    peer.delivering.settle();
    assert_eq!(peer.fake.answered_sends(), 1);
    assert_eq!(peer.fake.sends().len(), 1);
}

#[test]
fn sends_follow_the_order_of_their_publishes() {
    let mut peer = Peer::new("send-order");
    peer.start();
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g);
    let root = peer.post(&stoa, "a post");
    let delivering = &peer.delivering;
    let reply = crate::wire::publish_reply(
        &json!({ "stoa": stoa.to_hex(), "parent": root.to_hex(), "body": "a reply" }).to_string(),
        &mut peer.dir.op_log(),
        &Authorship {
            key: &key(7),
            asserted_ms: NOW_MS,
        },
        &mut |id| delivering.published(id),
    );
    let child = op_id_of(&reply);
    peer.delivering.settle();

    let sent: Vec<OpId> = peer
        .fake
        .sends()
        .iter()
        .map(|(_, payload)| SignedOp::from_bytes(payload).unwrap().op.id())
        .collect();
    assert_eq!(sent, [root, child]);
}

#[test]
fn a_publish_after_a_join_is_sent_on_the_channel_the_join_opened() {
    let mut peer = Peer::new("join-then-send");
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    peer.post(&g.address().unwrap(), "after the join");
    peer.delivering.settle();

    let calls = peer.fake.calls();
    let created_at = calls
        .iter()
        .position(|c| matches!(c, Call::ChannelCreate { .. }))
        .expect("a channel was created");
    let sent_at = calls
        .iter()
        .position(|c| matches!(c, Call::ChannelSend { .. }))
        .expect("the op was sent");
    assert!(created_at < sent_at, "{calls:?}");
    assert_eq!(peer.fake.sends()[0].0, peer.fake.creates()[0].0);
}

#[test]
fn sends_made_while_an_open_is_unanswered_wait_for_it_and_keep_their_order() {
    // The two tests above let the worker finish each action before the next
    // arrives, so they hold for any implementation that does things in the order
    // it hears of them. Here the join's channel creation is held at a gate while
    // a post and a reply to it are published: two sends are waiting behind an
    // open delivery has not answered. Released, the open is answered first, and
    // then the sends go out root-first.
    //
    // An implementation that sent without waiting for the open finds no channel
    // open and sends nothing; one that took the waiting sends newest-first sends
    // the reply before its post.
    let mut peer = Peer::new("send-behind-open");
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    peer.start();
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g);
    eventually("the open to reach delivery", || {
        !peer.fake.creates().is_empty()
    });

    let root = peer.post(&stoa, "a post");
    let delivering = &peer.delivering;
    let reply = crate::wire::publish_reply(
        &json!({ "stoa": stoa.to_hex(), "parent": root.to_hex(), "body": "a reply" }).to_string(),
        &mut peer.dir.op_log(),
        &Authorship {
            key: &key(7),
            asserted_ms: NOW_MS,
        },
        &mut |id| delivering.published(id),
    );
    let child = op_id_of(&reply);
    assert!(
        peer.fake.sends().is_empty(),
        "a send was made while the open was unanswered: {:?}",
        peer.fake.calls()
    );

    gate.release();
    peer.delivering.settle();

    let calls = peer.fake.calls();
    let answered_open = calls
        .iter()
        .position(|c| matches!(c, Call::ChannelCreate { .. }))
        .expect("the channel was asked for");
    let first_send = calls
        .iter()
        .position(|c| matches!(c, Call::ChannelSend { .. }))
        .expect("the sends were made once the open was answered");
    assert!(answered_open < first_send, "{calls:?}");
    let sent: Vec<OpId> = peer
        .fake
        .sends()
        .iter()
        .map(|(channel, payload)| {
            assert_eq!(channel, ChannelIdentity::of(&stoa).channel_id());
            SignedOp::from_bytes(payload).unwrap().op.id()
        })
        .collect();
    assert_eq!(sent, [root, child]);
}

// ─── Receiving ────────────────────────────────────────────────────────────

#[test]
fn an_op_another_peer_published_is_stored_unordered() {
    let peer = Peer::new("recv-stored");
    let stoa = genesis("Agora").address().unwrap();
    let op = their_op(stoa, "from elsewhere", 0);
    peer.processor(open_for(&stoa)).decide(&arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        op.to_bytes().unwrap(),
        1,
    ));

    let entry = stored(&peer, &op.op.id()).expect("the op is in the log");
    assert_eq!(entry.arrival, crate::arrival::Arrival::unordered());
}

#[test]
fn the_window_is_judged_by_this_peers_clock_not_the_events_timestamp() {
    let peer = Peer::new("recv-window-ahead");
    let stoa = genesis("Agora").address().unwrap();
    let ahead = crate::arrival::RECEIVE_WINDOW_MS + 60_000;
    let op = their_op(stoa, "from the future", ahead);
    // The event's timestamp is later still, in delivery's nanoseconds: a
    // boundary reading it would find the op comfortably in the past.
    let later_ns = ((NOW_MS + ahead) as i64 + 3_600_000) * 1_000_000;
    peer.processor(open_for(&stoa)).decide(&arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        op.to_bytes().unwrap(),
        later_ns,
    ));

    assert!(stored(&peer, &op.op.id()).is_none());
    assert_eq!(peer.journal.with("(ahead-of-time)").len(), 1);
}

#[test]
fn an_event_timestamp_decades_in_the_past_does_not_refuse_an_op_in_the_window() {
    let peer = Peer::new("recv-window-past");
    let stoa = genesis("Agora").address().unwrap();
    let op = their_op(stoa, "on time", 0);
    let decades_ago_ns = 315_532_800_000_000_000; // 1980-01-01, in nanoseconds
    peer.processor(open_for(&stoa)).decide(&arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        op.to_bytes().unwrap(),
        decades_ago_ns,
    ));
    assert!(stored(&peer, &op.op.id()).is_some());
}

#[test]
fn a_refusal_is_logged_by_kind_without_text_the_sender_chose() {
    let peer = Peer::new("recv-refusal-log");
    let open = genesis("Agora").address().unwrap();
    let foreign_channel = "/another-application/zzyzx-channel-identifier";
    let sender = "zzyzx-sender-identifier";
    let payload = b"zzyzx-payload-bytes".to_vec();
    peer.processor(open_for(&open)).decide(&Arriving {
        channel_id: foreign_channel.to_string(),
        sender_id: sender.to_string(),
        payload: payload.clone(),
        timestamp: 1,
    });

    let lines = peer.journal.lines();
    assert_eq!(
        peer.journal.with("refused (unknown-channel").len(),
        1,
        "{lines:?}"
    );
    for line in &lines {
        assert!(!line.contains(foreign_channel), "{line}");
        assert!(!line.contains(sender), "{line}");
        assert!(!line.contains("zzyzx-payload-bytes"), "{line}");
        assert!(!line.contains(&hex::encode(&payload)), "{line}");
    }
}

#[test]
fn an_unknown_channel_is_refused_as_one_before_the_op_log_is_opened() {
    // Security review, `Processor::decide` opened `ops.sqlite` before the channel
    // was looked up: the cheapest refusal paid a database open, and with the log
    // unopenable a message on a channel nobody opened was logged as a storage
    // failure. Red before the processor judged ahead of opening the log.
    let peer = Peer::new("recv-unknown-before-log");
    let stoa = genesis("Agora").address().unwrap();
    peer.dir
        .break_file(&crate::log::op_log_path_in(&peer.dir.0));
    peer.processor(open_for(&stoa))
        .decide(&arriving("/another-app/channel", vec![1, 2, 3], 1));

    assert_eq!(
        peer.journal.with("refused (unknown-channel)").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    assert!(peer.journal.with("storage").is_empty());
}

#[test]
fn the_channel_book_is_not_held_while_an_op_is_appended() {
    // Architecture review: `Channels::receive` held the book's mutex across
    // verification and the append, so an append waiting on a busy `ops.sqlite`
    // held up the worker's opens and sends. Here another connection holds the
    // database's write lock, so the processor's append waits on it (rusqlite's
    // busy timeout); while it waits, the book must be free. Red before the
    // processor released the lock ahead of judging.
    let peer = Peer::new("recv-book-free-during-append");
    let stoa = genesis("Agora").address().unwrap();
    let channels = open_for(&stoa);
    drop(peer.dir.op_log()); // the file and its schema exist
    let blocker = rusqlite::Connection::open(crate::log::op_log_path_in(&peer.dir.0)).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();

    let op = their_op(stoa, "appended while the database is busy", 0);
    decide_elsewhere(
        peer.processor(Arc::clone(&channels)),
        arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            op.to_bytes().unwrap(),
            1,
        ),
    );
    // **Watched, not sampled once after a sleep.** The processor takes the book
    // for microseconds to look the channel up, judges, and then waits on the busy
    // database; where old code went wrong it held the book from the lookup until
    // the append finished. A single check after a fixed sleep passes that old code
    // whenever the processor is slower to get going than the sleep. So the book is
    // polled for a second (under rusqlite's five-second busy timeout, so the
    // append is still waiting at the end), and the test fails on twenty polls in a
    // row finding it held — a run long enough that the processor's own brief
    // lookups cannot be it, and short enough that old code, which held it
    // throughout, cannot miss it.
    let mut held_in_a_row = 0;
    let mut most_in_a_row = 0;
    let until = Instant::now() + Duration::from_secs(1);
    while Instant::now() < until {
        match channels.book.try_lock() {
            Err(std::sync::TryLockError::WouldBlock) => {
                held_in_a_row += 1;
                most_in_a_row = most_in_a_row.max(held_in_a_row);
            }
            _ => held_in_a_row = 0,
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    blocker.execute_batch("ROLLBACK").unwrap();

    assert!(
        most_in_a_row < 20,
        "the channel book was held while the op waited to be appended \
         ({most_in_a_row} polls in a row)"
    );
    eventually("the op to be stored once the database is free", || {
        stored(&peer, &op.op.id()).is_some()
    });
}

#[test]
fn the_peer_keeps_processing_after_an_unreadable_message_and_a_refusal() {
    let peer = Peer::new("recv-keeps-going");
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    let valid = their_op(stoa, "after the junk", 0);
    let processor = peer.processor(open_for(&stoa));

    // The listener meets an event it cannot read, then a junk payload, then a
    // valid op.
    listen(
        [
            None,
            Some(arriving(channel.channel_id(), b"not an op".to_vec(), 1)),
            Some(arriving(channel.channel_id(), valid.to_bytes().unwrap(), 2)),
        ]
        .into_iter(),
        &processor.channels,
        &processor.queue,
        &*peer.journal,
    );
    processor.queue.close();
    processor.run();

    assert!(stored(&peer, &valid.op.id()).is_some());
    assert_eq!(peer.journal.with("could not be read").len(), 1);
    assert_eq!(peer.journal.with("(undecodable)").len(), 1);
}

#[test]
fn a_message_arriving_while_its_channel_opens_is_judged_after_the_answer() {
    // `op-transport`, scenario "A message arriving while its channel opens is
    // stored once delivery reports the channel created": the message is held
    // until the open settles, so delivery's answer decides it — stored here,
    // where refusing it as an unknown channel would lose an op SDS already
    // counts as delivered.
    let peer = Peer::new("recv-during-open");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let op = their_op(stoa, "raced the answer", 0);

    std::thread::scope(|scope| {
        let mut opening = channels.opening(&identity);
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            opening.held();
        });
        peer.processor(Arc::clone(&channels)).decide(&arriving(
            identity.channel_id(),
            op.to_bytes().unwrap(),
            1,
        ));
    });

    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer() {
    let peer = Peer::new("recv-during-declined-open");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let op = their_op(stoa, "raced a decline", 0);

    // "After delivery's answer, not before it" is read off a flag the answering
    // thread sets immediately before it answers, and `decide` is asked whether it
    // had been set when it returned. A refusal made without waiting returns with
    // the flag still down, however the threads happen to be scheduled.
    let answered = std::sync::atomic::AtomicBool::new(false);
    let answered_when_decided = std::thread::scope(|scope| {
        let opening = channels.opening(&identity);
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            answered.store(true, std::sync::atomic::Ordering::SeqCst);
            drop(opening); // settled, not created
        });
        peer.processor(Arc::clone(&channels)).decide(&arriving(
            identity.channel_id(),
            op.to_bytes().unwrap(),
            1,
        ));
        answered.load(std::sync::atomic::Ordering::SeqCst)
    });

    assert!(
        answered_when_decided,
        "the message was refused before delivery had answered the open"
    );
    assert!(stored(&peer, &op.op.id()).is_none());
    assert_eq!(peer.journal.with("(unknown-channel)").len(), 1);
}

/// Decide `message` on a thread of its own, so a test can watch whether it
/// returns while an open is still unanswered.
///
/// Detached on purpose: a `decide` that waits when it should not waits up to
/// [`SETTLE_LIMIT`], and the test must be able to fail long before that.
fn decide_elsewhere(processor: Processor, message: Arriving) {
    std::thread::spawn(move || processor.decide(&message));
}

#[test]
fn a_message_on_a_channel_not_being_opened_does_not_wait_on_another_channels_open() {
    let peer = Peer::new("recv-other-channel");
    let opening = genesis("Agora").address().unwrap();
    let elsewhere = genesis("Lyceum").address().unwrap();
    let channels = Arc::new(Channels::default());
    // Agora's open is requested and never answered in this test until the end.
    let unanswered = channels.opening(&ChannelIdentity::of(&opening));
    let op = their_op(elsewhere, "on a channel nobody asked for", 0);

    decide_elsewhere(
        peer.processor(Arc::clone(&channels)),
        arriving(
            ChannelIdentity::of(&elsewhere).channel_id(),
            op.to_bytes().unwrap(),
            1,
        ),
    );

    // Judged, and refused, with Agora's open still unanswered: `unanswered` is
    // dropped only below, so nothing but the refusal can end this wait.
    eventually("the message to be refused without waiting", || {
        !peer.journal.with("refused (unknown-channel)").is_empty()
    });
    assert!(stored(&peer, &op.op.id()).is_none());
    drop(unanswered);
}

#[test]
fn a_poisoned_channel_book_still_waits_for_this_channels_open() {
    // Correctness review: once the book's mutex was poisoned, `wait_timeout_while`
    // returned `Err` at its first wake-up and the wait was discarded — so a
    // message on a channel still opening was judged as soon as ANY open settled,
    // refused as an unknown channel, and lost. Red before the wait recovered the
    // guard and looped on its own condition.
    let peer = Peer::new("recv-poisoned-book");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let other = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let channels = Arc::new(Channels::default());
    let poisoner = Arc::clone(&channels);
    let _ = std::thread::spawn(move || {
        let _held = poisoner.book.lock();
        panic!("a contained panic while the book was held");
    })
    .join();
    assert!(channels.book.is_poisoned());

    // **Read off a flag, not off a pause.** Another channel's open settles every
    // millisecond for as long as `decide` runs, each settle waking every waiter;
    // this channel's own open is answered only after `decide` has had 200 ms to
    // end early, and only after the flag saying so is raised. `decide` returns
    // whether the flag was up when it did: a wait that ended at the first wake-up
    // returns with the flag down, on however slow a machine, and one that waits
    // for its own open cannot return before the flag is up. The 200 ms is the
    // margin between `decide` being entered and it reaching its wait, which is its
    // first statement; a stalled thread is the only way to pass wrongly.
    let mut opening = channels.opening(&identity);
    let op = their_op(stoa, "waits out another channel's answer", 0);
    let processor = peer.processor(Arc::clone(&channels));
    let message = arriving(identity.channel_id(), op.to_bytes().unwrap(), 1);

    let entered = std::sync::atomic::AtomicBool::new(false);
    let answered = std::sync::atomic::AtomicBool::new(false);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let answered_when_decided = std::thread::scope(|scope| {
        let (entered, answered, stop, channels, other) =
            (&entered, &answered, &stop, &channels, &other);
        scope.spawn(move || {
            let until = Instant::now() + Duration::from_secs(12);
            while !stop.load(std::sync::atomic::Ordering::SeqCst) && Instant::now() < until {
                drop(channels.opening(other)); // settles, and wakes every waiter
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        scope.spawn(move || {
            while !entered.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
            std::thread::sleep(Duration::from_millis(200));
            answered.store(true, std::sync::atomic::Ordering::SeqCst);
            opening.held();
            drop(opening);
        });
        entered.store(true, std::sync::atomic::Ordering::SeqCst);
        processor.decide(&message);
        let seen = answered.load(std::sync::atomic::Ordering::SeqCst);
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        seen
    });

    assert!(
        answered_when_decided,
        "judged before its own open settled: {:?}",
        peer.journal.lines()
    );
    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait() {
    // `op-transport`, scenario "A message waiting on an open delivery never
    // answers is judged after a bounded wait": refused as an unknown channel
    // while the open is still unanswered, and a valid op after it on an open
    // channel is then stored. The limit is shortened here; the running wiring's
    // is `SETTLE_LIMIT`.
    let peer = Peer::new("recv-bounded-wait");
    let open = genesis("Agora").address().unwrap();
    let never = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let channels = open_for(&open);
    let unanswered = channels.opening(&never);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.settle_limit = Duration::from_millis(200);
    let queue = Arc::clone(&processor.queue);

    let waiting = their_op(genesis("Lyceum").address().unwrap(), "never answered", 0);
    let after = their_op(open, "behind the unanswered open", 0);
    queue.offer(arriving(never.channel_id(), waiting.to_bytes().unwrap(), 1));
    queue.offer(arriving(
        ChannelIdentity::of(&open).channel_id(),
        after.to_bytes().unwrap(),
        2,
    ));
    queue.close();
    std::thread::spawn(move || processor.run());

    eventually("the op behind the unanswered open to be stored", || {
        stored(&peer, &after.op.id()).is_some()
    });
    assert_eq!(peer.journal.with("refused (unknown-channel)").len(), 1);
    assert!(stored(&peer, &waiting.op.id()).is_none());
    drop(unanswered); // still unanswered until here
}

#[test]
fn opens_settling_for_other_channels_do_not_extend_the_bounded_wait() {
    // `op-transport`: every message waiting on an open is judged "no later than
    // that fixed time after it started". Every settle wakes every waiter, so a wait whose
    // clock restarted on each wake-up could be postponed for ever by unrelated
    // opens — one per second would do it — and the limit would bound nothing under
    // the conditions it exists for. Here an open for another channel settles every
    // millisecond for as long as the test lets it, and the message waiting on an
    // open that is never answered must still be refused, an unknown channel, at
    // its own limit.
    //
    // The storm runs until the refusal is seen (or 12 s pass), so the assertion
    // does not depend on when the processor began waiting: a wait restarted by
    // wake-ups never ends while the storm lasts, however slow the machine.
    let peer = Peer::new("recv-wait-not-extended");
    let never = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let other = ChannelIdentity::of(&genesis("Athenaeum").address().unwrap());
    let channels = Arc::new(Channels::default());
    let _unanswered = channels.opening(&never);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.settle_limit = Duration::from_millis(300);
    let queue = Arc::clone(&processor.queue);
    queue.offer(arriving(
        never.channel_id(),
        b"waits for an open".to_vec(),
        1,
    ));
    queue.close();
    std::thread::spawn(move || processor.run());

    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let until = Instant::now() + Duration::from_secs(12);
            while !stop.load(std::sync::atomic::Ordering::SeqCst) && Instant::now() < until {
                drop(channels.opening(&other)); // settles, and wakes every waiter
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        eventually(
            "the message to be refused while opens keep settling",
            || !peer.journal.with("refused (unknown-channel)").is_empty(),
        );
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
    });
}

#[test]
fn many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each() {
    // `op-transport`, scenario "Many messages on one unanswered open hold other
    // channels up for one wait, not one each" — the security re-review's probe.
    // Four messages on a channel whose open is never answered, then a valid op on
    // a channel that is open. With the wait kept per message the op waited four
    // limits behind them; kept per open, it waits one.
    //
    // Timed from before the processor starts, which is no later than the first
    // message begins waiting, so the bound asserted is if anything tighter than
    // the scenario's. The processor runs on its own thread, so a wait that never
    // ends fails `eventually` instead of hanging the suite.
    let peer = Peer::new("recv-one-wait-per-open");
    let open = genesis("Agora").address().unwrap();
    let stuck = genesis("Lyceum").address().unwrap();
    let never = ChannelIdentity::of(&stuck);
    let channels = open_for(&open);
    let unanswered = channels.opening(&never);
    let limit = Duration::from_millis(400);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.settle_limit = limit;
    let queue = Arc::clone(&processor.queue);

    let waiting: Vec<SignedOp> = (0..4u64)
        .map(|n| their_op(stuck, &format!("never answered {n}"), n))
        .collect();
    for (n, op) in (1..).zip(&waiting) {
        queue.offer(arriving(never.channel_id(), op.to_bytes().unwrap(), n));
    }
    let after = their_op(open, "behind four messages on the stuck open", 0);
    queue.offer(arriving(
        ChannelIdentity::of(&open).channel_id(),
        after.to_bytes().unwrap(),
        9,
    ));
    queue.close();

    let started = Instant::now();
    std::thread::spawn(move || processor.run());
    eventually("the op behind the stuck open to be stored", || {
        stored(&peer, &after.op.id()).is_some()
    });
    let took = started.elapsed();

    assert!(
        took < 2 * limit,
        "the op behind the stuck open waited {took:?}: more than one wait of {limit:?}"
    );
    assert_eq!(
        peer.journal.with("refused (unknown-channel)").len(),
        waiting.len(),
        "{:?}",
        peer.journal.lines()
    );
    for op in &waiting {
        assert!(stored(&peer, &op.op.id()).is_none());
    }
    drop(unanswered); // still unanswered until here
}

/// One message on `channel`, decided with a short limit while its open is
/// unanswered, refused once that limit passes: the open's wait has expired.
///
/// On a thread of its own, so a wait that never ends fails here in seconds
/// instead of hanging the suite.
fn wait_the_open_out(peer: &Peer, channels: &Arc<Channels>, channel: &ChannelIdentity) {
    let mut processor = peer.processor(Arc::clone(channels));
    processor.settle_limit = Duration::from_millis(100);
    let message = arriving(channel.channel_id(), b"waits the open out".to_vec(), 1);
    let deciding = std::thread::spawn(move || processor.decide(&message));
    eventually("the message to wait the open out", || {
        deciding.is_finished()
    });
    assert_eq!(
        peer.journal.with("refused (unknown-channel)").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn an_open_whose_wait_has_expired_still_opens_its_channel_when_delivery_answers() {
    // `op-transport`, scenario "An open whose wait has expired still opens its
    // channel when delivery answers", and the paragraph "An expired wait changes
    // nothing else about the open". A message waits the open out and is refused.
    // The next one on that channel is handed over — not refused there, since the
    // channel is still being opened — and waits in the queue, the boundary not
    // yet deciding it; delivery then reports the channel created; and the
    // boundary, released, stores it.
    let peer = Peer::new("recv-expired-then-answered");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let mut opening = channels.opening(&identity);
    wait_the_open_out(&peer, &channels, &identity);

    let processor = peer.processor(Arc::clone(&channels));
    let op = their_op(stoa, "handed over after the wait expired", 0);
    hand_over(
        arriving(identity.channel_id(), op.to_bytes().unwrap(), 2),
        &channels,
        &processor.queue,
        &*peer.journal,
    );
    assert_eq!(
        processor.queue.len(),
        1,
        "refused on hand-over: {:?}",
        peer.journal.lines()
    );
    opening.held();
    drop(opening);
    processor.queue.close();
    processor.run();

    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(
        peer.journal.with("unknown-channel").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again() {
    // `op-transport`, scenario "A new request for a channel whose wait has
    // expired lets a message wait again". The first open is never answered and a
    // message waits it out. The peer then asks for the channel again, a message
    // arrives, and delivery answers the new request: the message waited for that
    // answer and is stored.
    //
    // "Waited for the answer" is read off a flag raised immediately before the
    // answer, as `a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer`
    // does: a message judged without waiting returns with it still down. The
    // second wait's limit is long, so only the answer can end it.
    let peer = Peer::new("recv-expired-then-requested");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let first = channels.opening(&identity);
    wait_the_open_out(&peer, &channels, &identity);

    let mut processor = peer.processor(Arc::clone(&channels));
    processor.settle_limit = Duration::from_secs(10);
    let op = their_op(stoa, "waits on the new request", 0);
    let answered = std::sync::atomic::AtomicBool::new(false);
    let answered_when_decided = std::thread::scope(|scope| {
        let mut again = channels.opening(&identity);
        let answered = &answered;
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            answered.store(true, std::sync::atomic::Ordering::SeqCst);
            again.held();
            drop(again);
        });
        processor.decide(&arriving(identity.channel_id(), op.to_bytes().unwrap(), 2));
        answered.load(std::sync::atomic::Ordering::SeqCst)
    });

    assert!(
        answered_when_decided,
        "judged without waiting on the new request: {:?}",
        peer.journal.lines()
    );
    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(peer.journal.with("unknown-channel").len(), 1);
    drop(first); // the first request, still unanswered until here
}

// NO SPEC: `op-transport` starts an open's wait when a message first begins
// waiting on it, and lets a later request start a new one. It does not say what
// a request made WHILE a message is already waiting does to that message. Here
// the waiting message keeps the deadline it began with — a request cannot extend
// it — and only messages taken after it wait on the new request.
#[test]
fn a_request_made_while_a_message_waits_does_not_extend_that_messages_wait() {
    // A storm of requests for the same channel, each given up at once, runs for
    // as long as the message waits: were the message's deadline re-read from
    // the book, each request would restart it and the wait would last as long as
    // the storm. The storm stops only once the refusal is seen, so the assertion
    // does not depend on when the wait began.
    let peer = Peer::new("recv-wait-not-extended-by-requests");
    let never = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let channels = Arc::new(Channels::default());
    let _unanswered = channels.opening(&never);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.settle_limit = Duration::from_millis(300);
    let queue = Arc::clone(&processor.queue);
    queue.offer(arriving(
        never.channel_id(),
        b"waits for an open".to_vec(),
        1,
    ));
    queue.close();
    std::thread::spawn(move || processor.run());

    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let until = Instant::now() + Duration::from_secs(12);
            while !stop.load(std::sync::atomic::Ordering::SeqCst) && Instant::now() < until {
                drop(channels.opening(&never)); // a new request, given up at once
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        eventually(
            "the message to be refused while requests keep coming",
            || !peer.journal.with("refused (unknown-channel)").is_empty(),
        );
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
    });
}

#[test]
fn an_open_this_peer_gives_up_without_asking_delivery_does_not_hold_a_message_up() {
    // `op-transport`, scenario "An open this peer gives up without asking
    // delivery does not hold a message up". No sender identifier can be retained,
    // so the worker gives the Stoa's open up without asking delivery; once the
    // log has recorded that, a message on the Stoa's channel is refused as an
    // unknown channel — inside `eventually`'s ten seconds, where the running
    // wiring's wait on an open is `SETTLE_LIMIT`.
    let mut peer = Peer::new("recv-given-up-unasked");
    peer.dir.break_file(&sender_path_in(&peer.dir.0));
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let events = peer.start_listening();
    peer.join(&g);
    eventually("the log to record the open given up", || {
        !peer
            .journal
            .with("no sender identifier could be retained")
            .is_empty()
    });

    let started = Instant::now();
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            their_op(stoa, "on a channel given up", 0)
                .to_bytes()
                .unwrap(),
            1,
        )))
        .unwrap();
    eventually("the message to be refused", || {
        !peer.journal.with("refused (unknown-channel)").is_empty()
    });
    assert!(started.elapsed() < SETTLE_LIMIT);
    assert!(peer.fake.creates().is_empty(), "{:?}", peer.fake.calls());
}

#[test]
fn a_message_on_an_open_channel_does_not_wait_on_a_repeated_open() {
    let peer = Peer::new("recv-repeated-open");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = open_for(&stoa);
    // The channel is open, and its opening is requested again and not answered.
    let repeated = channels.opening(&identity);
    let op = their_op(stoa, "on a channel that is open", 0);

    decide_elsewhere(
        peer.processor(Arc::clone(&channels)),
        arriving(identity.channel_id(), op.to_bytes().unwrap(), 1),
    );

    // Stored with the repeat still unanswered: `repeated` is dropped only below.
    eventually("the op to be stored without waiting", || {
        stored(&peer, &op.op.id()).is_some()
    });
    drop(repeated);
}

/// The clock the next test hands its processor: an hour and a minute behind while
/// an open is unanswered, and the right time once [`ANSWERED`] is set.
static ANSWERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn a_clock_that_is_wrong_until_the_open_is_answered() -> u64 {
    if ANSWERED.load(std::sync::atomic::Ordering::SeqCst) {
        NOW_MS
    } else {
        NOW_MS - crate::arrival::RECEIVE_WINDOW_MS - 60_000
    }
}

#[test]
fn the_clock_is_read_after_the_wait_for_an_open_and_not_before_it() {
    // `op-transport`: the window is judged against this peer's clock "read when
    // the payload is processed". A message that waits for an open is processed
    // after the wait, so a clock read before it — at the moment the message is
    // taken off the queue — is the wrong reading. This clock is wrong until the
    // open is answered and right after, and the op's counter is `NOW_MS`: read
    // early it is more than the window ahead and is refused; read after the wait
    // it is within the window and is stored.
    let peer = Peer::new("recv-clock-after-wait");
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let op = their_op(stoa, "on time once the open is answered", 0);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.clock = a_clock_that_is_wrong_until_the_open_is_answered;

    std::thread::scope(|scope| {
        let mut opening = channels.opening(&identity);
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            ANSWERED.store(true, std::sync::atomic::Ordering::SeqCst);
            opening.held();
        });
        processor.decide(&arriving(identity.channel_id(), op.to_bytes().unwrap(), 1));
    });

    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_declined_repeat_open_leaves_the_channel_open_for_the_next_message() {
    // Through the whole wiring: the channel is opened, a second join asks for it
    // again, delivery declines the repeat, and a message arriving afterwards on
    // that channel is stored and not refused. A declined repeat that closed the
    // channel is the change this fails on.
    let mut peer = Peer::new("repeat-declined");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let events = peer.start_listening();
    peer.join(&g);
    peer.delivering.settle();
    let _ = peer.fake.script(|s| {
        s.create_replies
            .push_back(the_observed_decline("no reliable channel manager"))
    });
    peer.join(&g);
    peer.delivering.settle();
    assert_eq!(peer.fake.creates().len(), 2, "the repeat was not asked for");
    assert_eq!(
        peer.journal.with("no reliable channel manager").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );

    let their = their_op(stoa, "after a declined repeat", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            their.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the op to be stored", || {
        stored(&peer, &their.op.id()).is_some()
    });
    assert!(peer.journal.with("unknown-channel").is_empty());
}

#[test]
fn a_message_the_op_log_cannot_take_is_logged_and_not_retried() {
    // A log that can be opened again afterwards, and a queue that shows what was
    // held. The first op is decided while the log cannot be opened; the log is
    // then repaired, a second op is queued, and the processor runs until the queue
    // is empty. Only the second is in the log: a first op held for another
    // attempt — put back on the queue, or waited on — would be stored by the
    // repair, or would keep `decide` from returning at all.
    //
    // The queue is the test's own and is drained by `run` rather than watched for
    // a time, so an implementation that retries later than any pause a test could
    // choose is still found: what it retries is on the queue when `run` starts.
    let peer = Peer::new("recv-storage-not-retried");
    let stoa = genesis("Agora").address().unwrap();
    let channels = open_for(&stoa);
    let queue = Arc::new(InboundQueue::with_bound(INBOUND_BOUND));
    let processor = || Processor {
        queue: Arc::clone(&queue),
        channels: Arc::clone(&channels),
        stores: peer.dir.stores(),
        journal: peer.journal.clone(),
        clock: now,
        settle_limit: SETTLE_LIMIT,
    };
    let channel = ChannelIdentity::of(&stoa);
    let log_file = crate::log::op_log_path_in(&peer.dir.0);
    peer.dir.break_file(&log_file);

    let lost = their_op(stoa, "arrives while the log will not open", 0);
    let lost_message = arriving(channel.channel_id(), lost.to_bytes().unwrap(), 1);
    let first = processor();
    let deciding = std::thread::spawn(move || first.decide(&lost_message));
    eventually("the message to be decided and given up", || {
        deciding.is_finished()
    });
    assert_eq!(
        peer.journal.with("refused (storage").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );

    std::fs::remove_dir_all(&log_file).unwrap();
    let after = their_op(stoa, "arrives once the log opens", 1);
    queue.offer(arriving(channel.channel_id(), after.to_bytes().unwrap(), 2));
    queue.close();
    processor().run();

    assert!(
        stored(&peer, &after.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
    assert!(
        stored(&peer, &lost.op.id()).is_none(),
        "the op that met the broken log was held and stored later"
    );
}

#[test]
fn taking_a_message_from_delivery_does_not_wait_on_the_boundary() {
    // `op-transport`: "Taking a message from delivery MUST NOT wait on the
    // boundary." The processor is held on the first message, whose channel is
    // being opened and not yet answered; more messages follow and delivery's
    // event stream then ends. The listener must take every one and reach the end
    // while the boundary has decided nothing — it logs its end only when it has.
    let mut peer = Peer::new("listener-not-held");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    let events = peer.start_listening();
    peer.join(&g);
    eventually("the open to reach delivery", || {
        !peer.fake.creates().is_empty()
    });

    let ops: Vec<SignedOp> = (0..5)
        .map(|n| their_op(stoa, &format!("waiting behind the open {n}"), n))
        .collect();
    for (n, op) in ops.iter().enumerate() {
        events
            .send(Some(arriving(
                channel.channel_id(),
                op.to_bytes().unwrap(),
                n as i64,
            )))
            .unwrap();
    }
    drop(events);
    eventually("the listener to take them all and end", || {
        !peer.journal.with("inbound listener has ended").is_empty()
    });
    assert_eq!(peer.fake.answered_creates(), 0, "the open was answered");

    // Nothing was lost by not waiting: once the open is answered, all five are
    // judged against the channel it opened.
    gate.release();
    for op in &ops {
        eventually("a waiting op to be stored", || {
            stored(&peer, &op.op.id()).is_some()
        });
    }
}

#[test]
fn every_refusal_is_logged_under_its_own_name_and_none_carries_what_the_sender_chose() {
    // One case per refusal the boundary makes, through the processor — the
    // seven `refusal_kind` names, `storage` (a log that will not open) included. A
    // table rather than a test each: the requirement is one sentence over all of
    // them. The names are design.md's, hardcoded; `distinct` is the property that
    // matters beyond the spelling — no two refusals share a name.
    //
    // **Every input the sender chose carries the marker `zzyzx`** — the channel
    // id, the sender identifier, and the payload (as bytes inside it, or as the
    // body of a signed op) — so a line that echoed any of them is found by one
    // search, in the three spellings a careless `format!` would give a byte
    // string. A payload of single distinctive bytes could not tell an echo from a
    // coincidence, and the guard below fails the test if a payload does not
    // actually carry the marker.
    const MARKER: &[u8] = b"zzyzx";
    let stoa = genesis("Agora").address().unwrap();
    let other = genesis("Lyceum").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let open = identity.channel_id();
    let window = crate::arrival::RECEIVE_WINDOW_MS;
    let mut forged = their_op(stoa, "zzyzx a forgery", 0);
    forged.op.author = key(9).public_key();
    let oversized: Vec<u8> = MARKER
        .iter()
        .cycle()
        .take(crate::transport::MAX_MESSAGE_BYTES + 1)
        .copied()
        .collect();
    // (name, channel the message arrives on, its payload, whether the op log is
    // made unopenable first)
    let cases: [(&str, &str, Vec<u8>, bool); 7] = [
        (
            "unknown-channel",
            "/elsewhere/zzyzx",
            b"zzyzx payload".to_vec(),
            false,
        ),
        ("too-long", open, oversized, false),
        ("undecodable", open, b"zzyzx not an op".to_vec(), false),
        (
            "fails-verification",
            open,
            forged.to_bytes().unwrap(),
            false,
        ),
        (
            "stoa-mismatch",
            open,
            their_op(other, "zzyzx copied onto another channel", 0)
                .to_bytes()
                .unwrap(),
            false,
        ),
        (
            "ahead-of-time",
            open,
            their_op(stoa, "zzyzx from the future", window + 60_000)
                .to_bytes()
                .unwrap(),
            false,
        ),
        (
            "storage",
            open,
            their_op(stoa, "zzyzx nowhere to put it", 0)
                .to_bytes()
                .unwrap(),
            true,
        ),
    ];
    // What a line echoing the payload would contain, however it spelled the bytes.
    let echoes = [
        "zzyzx".to_string(),
        hex::encode(MARKER),
        format!("{:?}", MARKER.to_vec())
            .trim_matches(['[', ']'])
            .to_string(),
    ];

    let mut names = Vec::new();
    for (kind, channel, payload, break_log) in cases {
        assert!(
            payload.windows(MARKER.len()).any(|w| w == MARKER),
            "{kind}: the payload does not carry the marker, so this case proves nothing"
        );
        let peer = Peer::new(&format!("recv-refusal-{kind}"));
        if break_log {
            peer.dir
                .break_file(&crate::log::op_log_path_in(&peer.dir.0));
        }
        peer.processor(open_for(&stoa)).decide(&Arriving {
            channel_id: channel.to_string(),
            sender_id: "zzyzx-sender".to_string(),
            payload,
            timestamp: 1,
        });
        let lines = peer.journal.lines();
        assert_eq!(
            peer.journal.with(&format!("refused ({kind}")).len(),
            1,
            "{kind}: {lines:?}"
        );
        for line in &lines {
            for echo in &echoes {
                // The line's head only: one that echoed the 150 KiB payload
                // would otherwise print all of it.
                let head: String = line.chars().take(200).collect();
                assert!(!line.contains(echo.as_str()), "{kind}: {head}");
            }
        }
        names.push(kind);
    }
    let mut distinct = names.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), names.len());
}

#[test]
fn a_received_op_reaches_the_log_through_the_running_listener() {
    // The whole inbound path on its threads: listener → queue → processor.
    let mut peer = Peer::new("recv-pipeline");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g);
    let events = peer.start_listening();
    peer.delivering.settle(); // the channel is open once the worker settles

    let op = their_op(stoa, "over the wire", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            op.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    eventually("the op to be stored", || {
        stored(&peer, &op.op.id()).is_some()
    });

    // And the listener's end is logged when delivery goes away.
    drop(events);
    eventually("the listener's end to be logged", || {
        !peer.journal.with("inbound listener has ended").is_empty()
    });
}

// ─── The bound ────────────────────────────────────────────────────────────

#[test]
fn the_inbound_bound_is_pinned() {
    assert_eq!(INBOUND_BOUND, 256);
}

#[test]
fn the_waiting_messages_never_exceed_the_bound() {
    let queue = InboundQueue::with_bound(INBOUND_BOUND);
    for n in 0..INBOUND_BOUND + 5 {
        queue.offer(arriving("c", vec![n as u8], n as i64));
        assert!(queue.len() <= INBOUND_BOUND);
    }
    assert_eq!(queue.len(), INBOUND_BOUND);
}

#[test]
fn the_queue_the_running_wiring_builds_is_bounded_at_the_pinned_count() {
    // `op-transport`, "Inbound payloads waiting for the boundary are bounded": "by
    // a fixed count". The tests above build their own `InboundQueue`, so none of
    // them sees the queue `Delivering::start` builds — `with_bound(usize::MAX)`
    // there removed the bound with every one of them green. This floods that one,
    // through `start`, `listen` and `hand_over`.
    //
    // The boundary is held up on a message whose channel is still opening, then
    // more junk than the bound arrives on an open channel. At most
    // `INBOUND_BOUND` wait in the queue and one is held by the processor, so at
    // least `total - INBOUND_BOUND - 1` are discarded; how many exactly depends on
    // whether the processor had taken the first message when the flood began, so
    // the assertion is the bound's arithmetic and not a count read back from the
    // implementation. The bound itself is read off the log line, against the
    // hardcoded 256 that `the_inbound_bound_is_pinned` also pins.
    let mut peer = Peer::new("bound-running-wiring");
    let (events, asked) = peer.start_counted(vec![]);
    let (open, gate) = hold_the_boundary_up(&mut peer, &events);
    let channel = ChannelIdentity::of(&open);

    let junk = INBOUND_BOUND + 5;
    for n in 0..junk {
        events
            .send(Some(arriving(channel.channel_id(), vec![n as u8], 1)))
            .unwrap();
    }
    let sent = junk + 1; // and the message that holds the boundary up
    handed_over(&asked, sent);

    let discards = peer.journal.with("discarded unread");
    assert!(
        discards.len() >= junk - INBOUND_BOUND,
        "{} of {sent} discarded, with at most {INBOUND_BOUND} waiting and one held: {:?}",
        discards.len(),
        peer.journal.lines()
    );
    for line in &discards {
        assert!(line.contains("queue full at 256;"), "{line}");
    }
    let last = discards.last().unwrap();
    assert!(
        last.contains(&format!("{} discarded since", discards.len())),
        "{last}"
    );

    // Nothing was lost but the discards: once the boundary is released, every
    // message that waited is decided — the junk as undecodable.
    gate.release();
    let waited = junk - discards.len();
    eventually("every waiting message to be decided", || {
        peer.journal.with("refused (undecodable)").len() == waited
    });
}

#[test]
fn a_full_queue_keeps_what_it_holds_and_discards_the_arrival() {
    let queue = InboundQueue::with_bound(3);
    for n in 0..3 {
        assert_eq!(queue.offer(arriving("c", vec![n], 0)), Offered::Waiting);
    }
    assert_eq!(
        queue.offer(arriving("c", b"the arrival".to_vec(), 0)),
        Offered::Discarded(1)
    );
    queue.close();
    let decided: Vec<Vec<u8>> = std::iter::from_fn(|| queue.take())
        .map(|a| a.payload)
        .collect();
    assert_eq!(decided, [vec![0], vec![1], vec![2]]);
}

#[test]
fn traffic_on_a_channel_this_peer_is_not_opening_takes_no_place_in_the_queue() {
    // `op-transport`, scenario "Traffic on a channel this peer is not opening
    // takes no place in the queue" — the security review's probe, with the
    // opposite expectation. The boundary is held up on a message whose channel
    // is still opening; more messages than the bound then arrive on another
    // application's channel, and a valid op on an open one. Red while every
    // message took a place: the foreign ones filled the queue, the valid op was
    // discarded, and nothing was refused until the boundary was released.
    let mut peer = Peer::new("handover-foreign-traffic");
    let open = genesis("Agora");
    let opening = genesis("Lyceum");
    let events = peer.start_listening();
    peer.join(&open);
    peer.delivering.settle();

    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    peer.join(&opening);
    eventually("the second open to reach delivery", || {
        peer.fake.creates().len() == 2
    });
    let held_up = their_op(opening.address().unwrap(), "holds the boundary up", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&opening.address().unwrap()).channel_id(),
            held_up.to_bytes().unwrap(),
            1,
        )))
        .unwrap();

    let foreign = INBOUND_BOUND + 5;
    for n in 0..foreign {
        events
            .send(Some(arriving(
                "/another-application/channel",
                vec![n as u8],
                1,
            )))
            .unwrap();
    }
    eventually("every foreign message to be refused on hand-over", || {
        peer.journal.with("refused (unknown-channel)").len() == foreign
    });
    assert_eq!(
        peer.fake.answered_creates(),
        1,
        "the refusals waited on the boundary"
    );

    let valid = their_op(open.address().unwrap(), "arrives behind the flood", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&open.address().unwrap()).channel_id(),
            valid.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    gate.release();
    eventually("the valid op to be stored", || {
        stored(&peer, &valid.op.id()).is_some()
    });
    assert!(
        peer.journal.with("discarded").is_empty(),
        "{:?}",
        peer.journal.with("discarded")
    );
}

#[test]
fn a_refusal_made_on_hand_over_is_logged_by_kind_without_text_the_sender_chose() {
    // `op-transport`, "Each refusal MUST be recorded in the module's log naming
    // which refusal it was. The log MUST NOT carry the payload, the sender
    // identifier, or a channel identifier this peer has no channel open under."
    // The two refusals the listener makes before a message may wait — an unknown
    // channel and an oversized payload — are logged by `hand_over`, not by the
    // processor, so the tests that read the processor's lines
    // (`a_refusal_is_logged_by_kind_…`, the refusal table) do not see them. A
    // hand-over line that echoed the channel, the sender or the payload passed
    // every one of them.
    let stoa = genesis("Agora").address().unwrap();
    let open = ChannelIdentity::of(&stoa);
    let journal = Recorder::default();
    let queue = InboundQueue::with_bound(INBOUND_BOUND);
    let oversized: Vec<u8> = b"zzyzx"
        .iter()
        .cycle()
        .take(crate::transport::MAX_MESSAGE_BYTES + 1)
        .copied()
        .collect();
    let message = |channel: &str, payload: Vec<u8>| Arriving {
        channel_id: channel.to_string(),
        sender_id: "zzyzx-sender".to_string(),
        payload,
        timestamp: 1,
    };
    listen(
        [
            Some(message(
                "/elsewhere/zzyzx-channel",
                b"zzyzx payload".to_vec(),
            )),
            Some(message(open.channel_id(), oversized)),
        ]
        .into_iter(),
        &open_for(&stoa),
        &queue,
        &journal,
    );

    assert_eq!(journal.with("refused (unknown-channel)").len(), 1);
    assert_eq!(journal.with("refused (too-long)").len(), 1);
    assert_eq!(
        queue.len(),
        0,
        "a refused message took a place in the queue"
    );
    for line in journal.lines() {
        let head: String = line.chars().take(200).collect();
        for echo in ["zzyzx", &hex::encode(b"zzyzx"), "122, 122, 121, 122, 120"] {
            assert!(!line.contains(echo), "{head}");
        }
    }
}

/// A peer with one Stoa's channel open and the boundary held up: a message on a
/// second Stoa's channel waits on that channel's open, which delivery does not
/// answer until the returned gate is released. Returns the open Stoa.
fn hold_the_boundary_up(
    peer: &mut Peer,
    events: &mpsc::Sender<Option<Arriving>>,
) -> (Address, Arc<Gate>) {
    let open = genesis("Agora");
    let opening = genesis("Lyceum");
    peer.join(&open);
    peer.delivering.settle();
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    peer.join(&opening);
    eventually("the second open to reach delivery", || {
        peer.fake.creates().len() == 2
    });
    let held_up = their_op(opening.address().unwrap(), "holds the boundary up", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&opening.address().unwrap()).channel_id(),
            held_up.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    (open.address().unwrap(), gate)
}

#[test]
fn an_oversized_payload_on_an_open_channel_takes_no_place_in_the_queue() {
    // `op-transport`, scenario "An oversized payload on an open channel takes no
    // place in the queue". More payloads than the bound, each one byte over the
    // message limit, arrive on an open channel while the boundary is held up.
    // Red while a payload waited whatever its size: nothing was refused until the
    // boundary was released, and the queue filled and discarded.
    let mut peer = Peer::new("handover-oversized");
    let events = peer.start_listening();
    let (open, gate) = hold_the_boundary_up(&mut peer, &events);
    let channel = ChannelIdentity::of(&open);

    let oversized = INBOUND_BOUND + 5;
    for n in 0..oversized {
        events
            .send(Some(arriving(
                channel.channel_id(),
                vec![n as u8; crate::transport::MAX_MESSAGE_BYTES + 1],
                1,
            )))
            .unwrap();
    }
    eventually("every oversized payload to be refused on hand-over", || {
        peer.journal.with("refused (too-long)").len() == oversized
    });
    assert_eq!(
        peer.fake.answered_creates(),
        1,
        "the refusals waited on the boundary"
    );

    let valid = their_op(open, "arrives behind the oversized ones", 0);
    events
        .send(Some(arriving(
            channel.channel_id(),
            valid.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    gate.release();
    eventually("the valid op to be stored", || {
        stored(&peer, &valid.op.id()).is_some()
    });
    assert!(
        peer.journal.with("discarded").is_empty(),
        "{:?}",
        peer.journal.with("discarded")
    );
}

#[test]
fn an_oversized_payload_on_an_unknown_channel_is_refused_as_an_unknown_channel() {
    // `op-transport`: a message on a channel neither open nor being opened is
    // refused as an unknown channel "whatever its size". Red with the size asked
    // before the channel in `refused_on_hand_over`.
    let stoa = genesis("Agora").address().unwrap();
    let oversized = vec![0; crate::transport::MAX_MESSAGE_BYTES + 1];
    assert_eq!(
        refused_on_hand_over(
            &arriving("/another-application/channel", oversized.clone(), 1),
            &open_for(&stoa),
        ),
        Some(InboundRefusal::UnknownChannel)
    );
    // And on the open channel, the same payload is refused for its size.
    assert!(matches!(
        refused_on_hand_over(
            &arriving(ChannelIdentity::of(&stoa).channel_id(), oversized, 1),
            &open_for(&stoa),
        ),
        Some(InboundRefusal::TooLong { .. })
    ));
}

#[test]
fn a_payload_at_the_limit_waits_its_turn() {
    // `op-transport`, scenario "A payload at the limit waits its turn". A payload
    // of exactly the message limit is not refused on hand-over — it waits — and
    // the boundary, once released, does not refuse it for its size either. It is
    // zeros, so the boundary refuses it as undecodable: a decision, reached.
    // Passed before the hand-over refusal existed; it is the pin on where the
    // limit falls, red with `>=` in `transport::refuse_oversized`.
    let mut peer = Peer::new("handover-at-limit");
    let (events, asked) = peer.start_counted(vec![]);
    let (open, gate) = hold_the_boundary_up(&mut peer, &events);

    events
        .send(Some(arriving(
            ChannelIdentity::of(&open).channel_id(),
            vec![0; crate::transport::MAX_MESSAGE_BYTES],
            1,
        )))
        .unwrap();
    handed_over(&asked, 2);
    assert!(
        peer.journal.with("refused").is_empty(),
        "refused before the boundary was released: {:?}",
        peer.journal.lines()
    );

    gate.release();
    eventually("the payload at the limit to be decided", || {
        !peer.journal.with("refused (undecodable)").is_empty()
    });
    assert!(
        peer.journal.with("too-long").is_empty(),
        "{:?}",
        peer.journal.lines()
    );
}

/// An event stream that counts how often the listener has asked it for an event.
///
/// The listener hands one event over — queues or refuses it — before it asks for
/// the next, so once the count reaches `n + 1` the first `n` events have been
/// handed over. That is how a test says "handed over before delivery answered"
/// without reading a clock or a log line: a hand-over that queues a message
/// logs nothing.
struct Counted<I> {
    events: I,
    asked: Arc<std::sync::atomic::AtomicUsize>,
}

impl<I: Iterator> Iterator for Counted<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.events.next()
    }
}

impl Peer {
    /// Start the wiring with a listener fed from the returned sender, and a count
    /// of the events the listener has asked for ([`Counted`]).
    ///
    /// `waiting` is handed over the moment the subscription exists: sent before
    /// startup runs, as a delivery that kept running would have it ready.
    fn start_counted(
        &mut self,
        waiting: Vec<Arriving>,
    ) -> (
        mpsc::Sender<Option<Arriving>>,
        Arc<std::sync::atomic::AtomicUsize>,
    ) {
        let (events, feed) = mpsc::channel();
        for message in waiting {
            events.send(Some(message)).unwrap();
        }
        let asked = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = Counted {
            events: feed.into_iter(),
            asked: Arc::clone(&asked),
        };
        self.delivering
            .start(self.fake.clone(), self.dir.stores(), now, move || {
                Ok(counted)
            });
        (events, asked)
    }
}

/// Wait until the listener has handed over the first `n` events it was sent.
fn handed_over(asked: &std::sync::atomic::AtomicUsize, n: usize) {
    eventually("the listener to hand the events over", || {
        asked.load(std::sync::atomic::Ordering::SeqCst) > n
    });
}

#[test]
fn a_message_on_a_channel_whose_open_waits_behind_another_is_judged_once_that_open_settles() {
    // `op-transport`, scenario "A message on a channel whose open waits behind
    // another is judged once that open settles" — the spec's answer to the
    // question this test used to leave open, with the opposite expectation to
    // the one it first pinned. The second Stoa's open is queued
    // behind the first's, which delivery has not answered, when a message on
    // the second channel is handed over. Red while an open counted as being
    // opened only once the worker asked delivery: the message was refused as an
    // unknown channel on hand-over, and never stored.
    let mut peer = Peer::new("handover-open-queued");
    let first = genesis("Agora");
    let queued = genesis("Lyceum");
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    let (events, asked) = peer.start_counted(vec![]);
    peer.join(&first);
    eventually("the first open to reach delivery", || {
        peer.fake.creates().len() == 1
    });
    peer.join(&queued); // enqueued behind the unanswered first open

    let op = their_op(queued.address().unwrap(), "ahead of its open", 0);
    events
        .send(Some(arriving(
            ChannelIdentity::of(&queued.address().unwrap()).channel_id(),
            op.to_bytes().unwrap(),
            1,
        )))
        .unwrap();
    handed_over(&asked, 1);
    assert_eq!(
        peer.fake.creates().len(),
        1,
        "the queued open was asked of delivery before the message was handed over"
    );

    gate.release(); // delivery reports both channels created
    eventually("the op to be stored once its open settles", || {
        stored(&peer, &op.op.id()).is_some()
    });
    assert!(
        peer.journal.with("unknown-channel").is_empty(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_restarted_peer_keeps_what_delivery_hands_over_before_startup_asks_for_its_channel() {
    // `op-transport`, scenario "A restarted peer keeps what delivery hands over
    // before startup asks for its channel". Delivery kept running, so the first
    // thing it hands over is a message on this peer's Stoa — ready the moment the
    // subscription exists — while node creation is still unanswered and the
    // channel's open waits behind it. Red while startup marked its opens only as
    // the worker reached them: the message was refused as an unknown channel.
    //
    // **This is a guard on an ORDER inside `start`** — marks, then subscription —
    // and the order is not observable from outside except through a race: with
    // the marks moved after the subscription the listener thread and `start`
    // itself run at once, and the listener wins only if it hands the message over
    // before `start` has marked. Left at one round over one membership, the
    // listener won on a run of the mutation and not on others, so the test was
    // "likely caught" and not certain. Two things make it decisive: the peer is in
    // many Stoas, so the marking is a read of a record several pages long, which
    // the listener — a thread reading one message from a channel — outpaces by a
    // wide margin; and the whole scenario runs for several fresh peers, so the
    // mutation must win the race every time to escape. Neither makes it
    // deterministic; the rounds are margin. Measured: with `startup_opens` moved
    // to after the subscription, it was red in round 0 on four of four runs.
    for round in 0..ROUNDS {
        let mut peer = Peer::new(&format!("handover-restart-{round}"));
        let g = genesis("Agora");
        let stoa = g.address().unwrap();
        peer.join(&g); // before startup: the membership a restart finds
        let mut every_channel = vec![stoa];
        for n in 0..FILLER_STOAS {
            let filler = genesis(&format!("Filler {n}"));
            peer.join(&filler);
            every_channel.push(filler.address().unwrap());
        }
        let node = Gate::closed();
        let _ = peer.fake.script(|s| {
            s.node_gate = Some(node.clone());
            // Delivery kept every channel from the module's last run.
            for held in &every_channel {
                s.declined_channels.insert(
                    ChannelIdentity::of(held).channel_id().to_string(),
                    the_already_exists_answer(held),
                );
            }
        });
        let op = their_op(stoa, "handed over before the channel was asked for", 0);
        let (_events, asked) = peer.start_counted(vec![arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            op.to_bytes().unwrap(),
            1,
        )]);
        handed_over(&asked, 1);
        assert!(
            peer.fake.creates().is_empty(),
            "the channel was asked of delivery before the message was handed over"
        );

        node.release(); // node creation answered; every channel "already exists"
        eventually("the op to be decided once its open settles", || {
            stored(&peer, &op.op.id()).is_some() || !peer.journal.with("unknown-channel").is_empty()
        });
        assert!(
            peer.journal.with("unknown-channel").is_empty(),
            "round {round}: {:?}",
            peer.journal.with("refused")
        );
        assert!(stored(&peer, &op.op.id()).is_some(), "round {round}");
    }
}

/// How many fresh peers the restart scenario runs for, and how many other Stoas
/// each is in. See `a_restarted_peer_keeps_what_delivery_hands_over_…`.
const ROUNDS: usize = 4;
const FILLER_STOAS: usize = 60;

#[test]
fn every_discard_is_counted_and_logged_apart_from_refusals() {
    let journal = Recorder::default();
    let queue = InboundQueue::with_bound(1);
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    // `op-transport`: a discard's record "MUST NOT carry the payload or the sender
    // identifier". Both are made distinctive, and every discarded message is
    // checked for both, in the spellings a careless `format!` would give bytes.
    let sender = "zzyzx-the-sender-identifier";
    listen(
        (0..4u8).map(|n| {
            Some(Arriving {
                channel_id: channel.channel_id().to_string(),
                sender_id: sender.to_string(),
                payload: format!("zzyzx-payload-{n}").into_bytes(),
                timestamp: 0,
            })
        }),
        &open_for(&stoa),
        &queue,
        &journal,
    );

    let discards = journal.with("discarded");
    assert_eq!(discards.len(), 3, "{:?}", journal.lines());
    assert!(discards[2].contains("3 discarded since"), "{}", discards[2]);
    assert!(discards[2].contains("full at 1;"), "{}", discards[2]);
    let echoes = [
        "zzyzx".to_string(),
        hex::encode(b"zzyzx"),
        "122, 122, 121, 122, 120".to_string(),
    ];
    for line in &discards {
        assert!(!line.contains("refused"), "{line}");
        for echo in &echoes {
            assert!(!line.contains(echo.as_str()), "{line}");
        }
    }
    // And a refusal's line is not a discard's.
    let refusal = Note::Refused("unknown-channel", None).to_string();
    assert!(!refusal.contains("discarded"));
}

// ─── Nothing unwinds ──────────────────────────────────────────────────────

#[test]
fn a_panicking_delivery_call_is_contained_and_the_next_action_runs() {
    let mut peer = Peer::new("worker-panic");
    peer.fake = peer.fake.script(|s| s.panic_on_create_node = true);
    peer.join(&genesis("Agora"));
    peer.start();
    peer.delivering.settle();

    assert_eq!(peer.journal.with("panicked").len(), 1);
    assert_eq!(peer.fake.creates().len(), 1, "the open after the panic ran");
}

#[test]
fn a_panic_reading_one_event_does_not_end_reception() {
    // `op-transport`, scenario "A panic reading one message does not end
    // reception": reading one delivered message's fields panics, and a valid op
    // then arrives on an open channel; the module's log records the failure, and
    // the op is stored.
    //
    // Security review: the listener ran its whole loop under one `catch_unwind`,
    // so one panic while reading an event — the generated decoder runs inside
    // the iterator's `next()` — ended reception for the rest of the process.
    // Here the event stream panics once, then hands over a valid op. Red while
    // containment was per loop: the op was never stored, and the listener
    // logged its end.
    let mut peer = Peer::new("listener-panic");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g);
    let (events, feed) = mpsc::channel::<Option<Option<Arriving>>>();
    peer.delivering
        .start(peer.fake.clone(), peer.dir.stores(), now, move || {
            Ok(feed
                .into_iter()
                .map(|event| event.unwrap_or_else(|| panic!("reading this event panicked"))))
        });
    peer.delivering.settle(); // the channel is open

    events.send(None).unwrap();
    let op = their_op(stoa, "after the panic", 0);
    events
        .send(Some(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            op.to_bytes().unwrap(),
            1,
        ))))
        .unwrap();
    eventually("the op after the panic to be stored", || {
        stored(&peer, &op.op.id()).is_some()
    });
    assert_eq!(peer.journal.with("panicked").len(), 1);
    assert!(peer.journal.with("listener has ended").is_empty());
}

#[test]
fn a_worker_that_could_not_start_is_named_as_such_and_startup_does_not_run_again() {
    // Architecture review: the OS refusing the worker thread left `started` set
    // and no outbox, and every later request was logged "has not started". The
    // state is its own now; this is what it logs and that it counts as started.
    let journal = Arc::new(Recorder::default());
    let mut delivering = Delivering {
        journal: journal.clone(),
        wiring: Wiring::NoWorker,
    };
    let id = their_op(genesis("Agora").address().unwrap(), "unsendable", 0)
        .op
        .id();
    delivering.published(&id);
    let logged = journal.with("worker could not be started");
    assert_eq!(logged.len(), 1, "{:?}", journal.lines());
    assert!(logged[0].contains(&id.to_hex()));
    assert!(journal.with("has not started").is_empty());

    let dir = TempDir::new("no-worker-restart");
    assert!(!delivering.start(Fake::default(), dir.stores(), now, || {
        Ok(std::iter::empty::<Option<Arriving>>())
    }));
}

#[test]
fn a_join_the_worker_cannot_take_is_given_up_and_not_left_opening() {
    // `op-transport`: an open this peer never goes on to ask delivery for is
    // given up, and settled rather than left pending. The worker is gone — its
    // end of the queue dropped — so the join's open is refused by the send and
    // handed back inside its error. Red with that error forgotten in `request`:
    // the channel stays counted as being opened, and every message on it waits
    // out `SETTLE_LIMIT` before being refused.
    let journal = Arc::new(Recorder::default());
    let channels = Arc::new(Channels::default());
    let (actions, worker_end) = mpsc::channel();
    drop(worker_end);
    let delivering = Delivering {
        journal: journal.clone(),
        wiring: Wiring::Running(Outbox {
            actions,
            channels: Arc::clone(&channels),
        }),
    };
    let stoa = genesis("Agora").address().unwrap();

    delivering.joined(&stoa);

    assert_eq!(
        journal.with("worker has stopped").len(),
        1,
        "{:?}",
        journal.lines()
    );
    assert!(
        !channels.is_known(ChannelIdentity::of(&stoa).channel_id()),
        "the open the worker never took is still counted as being opened"
    );
}

#[test]
fn a_panicking_join_sink_does_not_change_the_reply() {
    let g = genesis("Agora");
    let quiet = crate::wire::join_stoa(
        &join_request(&g),
        &mut MembershipStore::in_memory().unwrap(),
        &mut |_| {},
    );
    let loud = crate::wire::join_stoa(
        &join_request(&g),
        &mut MembershipStore::in_memory().unwrap(),
        &mut |_| panic!("the channel request blew up"),
    );
    assert_eq!(quiet, loud);
}

#[test]
fn declined_reads_delivery_s_three_shapes_of_no() {
    assert_eq!(
        declined(&the_observed_decline("Context not initialized")),
        Some("Context not initialized".to_string())
    );
    assert_eq!(
        declined(&Err("timed out".to_string())),
        Some("timed out".to_string())
    );
    assert!(declined(&Ok(json!({ "success": false }))).is_some());
    // What delivery answers when it did the thing.
    assert_eq!(declined(&Ok(json!("/dialectica/1/c/abc"))), None);
    assert_eq!(declined(&Ok(json!(true))), None);
    assert_eq!(
        declined(&Ok(json!({ "success": true, "value": "r-1" }))),
        None
    );
}

/// The three shapes of "no" the spec names for node creation, channel creation
/// and sends ("declines, fails, or does not answer"), each with the words its log
/// line must carry: delivery's reason where it gave one, and the fact that it gave
/// none where it did not.
fn the_ways_delivery_says_no() -> [(&'static str, Result<Value, String>, &'static str); 3] {
    [
        (
            "a transport failure",
            Err("zzyzx-transport-reason".to_string()),
            "zzyzx-transport-reason",
        ),
        (
            "an error envelope, as delivery was measured answering",
            the_observed_decline("zzyzx-envelope-reason"),
            "zzyzx-envelope-reason",
        ),
        (
            "a failure with no reason",
            Ok(json!({ "success": false })),
            "gave no reason",
        ),
    ]
}

#[test]
fn a_declined_node_creation_is_read_in_every_shape_delivery_says_no() {
    // `declined_reads_delivery_s_three_shapes_of_no` covers the helper, not that
    // node creation reads its answer through it. A call site that treated only
    // `Err` as a failure would request start after delivery's error object.
    for (shape, reply, words) in the_ways_delivery_says_no() {
        let mut peer = Peer::new("node-no-shapes");
        let _ = peer.fake.script(|s| s.create_node = reply.clone());
        peer.join(&genesis("Agora"));
        peer.start();
        peer.delivering.settle();

        assert_eq!(
            peer.fake.count(&Call::StartNode),
            0,
            "{shape}: start was requested"
        );
        let logged = peer.journal.with("declined node creation");
        assert_eq!(logged.len(), 1, "{shape}: {:?}", peer.journal.lines());
        assert!(logged[0].contains(words), "{shape}: {}", logged[0]);
        // And channels are still requested: another module's node serves them.
        assert_eq!(peer.fake.creates().len(), 1, "{shape}");
    }
}

#[test]
fn a_declined_channel_creation_is_read_in_every_shape_delivery_says_no() {
    // The channel is not open afterwards, whichever way delivery said no, and the
    // log names the Stoa and delivery's reason. A call site that read only the
    // envelope, or only `Err`, would leave a refused channel open.
    let stoa = genesis("Agora").address().unwrap();
    let channel_id = ChannelIdentity::of(&stoa).channel_id().to_string();
    for (shape, reply, words) in the_ways_delivery_says_no() {
        let peer = Peer::new("channel-no-shapes");
        let _ = peer
            .fake
            .script(|s| s.create_replies.push_back(reply.clone()));
        let worker = peer.worker();
        worker.open_now(&stoa);

        assert_eq!(peer.fake.creates().len(), 1, "{shape}");
        assert!(
            !lock(&worker.channels.book).open.is_open(&channel_id),
            "{shape}: the channel is open"
        );
        let logged = peer.journal.with("channel NOT open");
        assert_eq!(logged.len(), 1, "{shape}: {:?}", peer.journal.lines());
        assert!(logged[0].contains(words), "{shape}: {}", logged[0]);
        assert!(logged[0].contains(&stoa.to_hex()), "{shape}: {}", logged[0]);
    }
}

#[test]
fn a_declined_send_is_read_in_every_shape_delivery_says_no() {
    // `a_send_delivery_declines_leaves_the_op_published` uses the transport
    // failure alone, which is not the shape delivery was measured giving. A send
    // that treated only `Err` as a failure would log the op as handed to the
    // channel when delivery answered with its error object.
    let stoa = genesis("Agora").address().unwrap();
    for (shape, reply, words) in the_ways_delivery_says_no() {
        let peer = Peer::new("send-no-shapes");
        let worker = peer.worker();
        worker.open_now(&stoa); // the channel is open: the send is asked for
        let _ = peer.fake.script(|s| s.send = Some(reply.clone()));
        let id = peer.post(&stoa, "delivery will say no");
        worker.perform(Action::Send(id));

        assert_eq!(
            peer.fake.sends().len(),
            1,
            "{shape}: the send was not asked for"
        );
        let logged = peer.journal.with("delivery did not take op");
        assert_eq!(logged.len(), 1, "{shape}: {:?}", peer.journal.lines());
        assert!(logged[0].contains(words), "{shape}: {}", logged[0]);
        assert!(logged[0].contains(&id.to_hex()), "{shape}: {}", logged[0]);
        assert!(
            peer.journal.with("handed to the channel").is_empty(),
            "{shape}: {:?}",
            peer.journal.lines()
        );
    }
}

#[test]
fn an_empty_error_string_is_not_a_reason_to_decline() {
    // Correctness review: `StdLogosResult.error` defaults to `""`, and
    // logos-cpp-sdk's `lpPushExpr` serialises it verbatim, so a success can
    // arrive as `{"success":true,"value":…,"error":""}`. Read as a decline, no
    // channel would ever open and no op would ever be sent, with every gate
    // green. Red while any string in `error` counted as a reason.
    assert_eq!(
        declined(&Ok(json!({ "success": true, "value": "r-1", "error": "" }))),
        None
    );
    // An empty reason beside an explicit failure is still a failure, with no
    // reason given.
    assert!(declined(&Ok(json!({ "success": false, "value": null, "error": "" }))).is_some());
}

#[test]
fn an_appended_twice_arrival_is_reported_already_present() {
    // The boundary's own idempotence, through the processor: the same op on the
    // channel twice is one op, and the second is not a refusal.
    let peer = Peer::new("recv-twice");
    let stoa = genesis("Agora").address().unwrap();
    let op = their_op(stoa, "twice", 0);
    let channels = open_for(&stoa);
    let message = arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        op.to_bytes().unwrap(),
        1,
    );
    let processor = peer.processor(channels);
    processor.decide(&message);
    processor.decide(&message);
    assert_eq!(peer.dir.op_log().len().unwrap(), 1);
    assert!(peer.journal.with("refused").is_empty());
    // Readability review: both arrivals were logged "stored inbound op", though
    // the second stored nothing. Red while the log did not say which.
    assert_eq!(
        peer.journal.with("stored inbound op").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    let held = peer
        .journal
        .with(&format!("inbound op {} already held", op.op.id().to_hex()));
    assert_eq!(held.len(), 1, "{:?}", peer.journal.lines());
}
