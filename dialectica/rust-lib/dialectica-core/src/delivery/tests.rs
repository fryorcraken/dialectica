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
    /// Held until released: a channel creation, or a send, delivery has not
    /// answered.
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
        let script = lock(&self.0.script);
        if script.panic_on_create_node {
            drop(script);
            panic!("the fake delivery panicked creating a node");
        }
        script.create_node.clone()
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

    assert_eq!(peer.journal.with("could not subscribe").len(), 1);
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
/// holds, as delivery v0.2.1 words it: `logos-delivery` `4a85db1b`,
/// `channel_lifecycle.nim` ("channel already exists: " & channelId) behind the
/// C API's "ChannelCreate failed: " prefix (`library/channels_api/channel_api.nim`).
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
    peer.worker().perform(Action::Open(*stoa));
    peer.fake.creates()[before].2.clone()
}

#[test]
fn two_installations_holding_one_identity_supply_different_sender_identifiers() {
    // Two installations are two directories. The identity is irrelevant by
    // construction — nothing an installation holds reaches the value — which is
    // the point: two holding the SAME keystore still differ.
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

#[test]
fn the_sender_identifier_is_not_the_authors_key() {
    // **What this can and cannot see.** No key reaches the code that mints a
    // sender identifier (`SenderStore::sender_for` takes a Stoa address), so an
    // implementation that derived it from one could not be written without
    // changing that signature, and this test could not be made to fail by any
    // change that compiles. The property is held by construction; what this
    // pins is the encoding check for a peer that signs as `key(7)` in the Stoa it
    // joins. The tests that can fail on how the value is produced are the
    // "two installations", "restart" and "two Stoas" ones beside it.
    let peer = Peer::new("sender-not-key");
    let sender = sender_supplied(&peer, &genesis("Agora").address().unwrap());
    let signing = key(7).public_key();
    assert!(!sender.contains(&signing.to_hex()));
    assert!(!sender.contains(&signing.to_hex().to_uppercase()));
}

#[test]
fn a_sender_identifier_that_cannot_be_retained_opens_no_channel() {
    let peer = Peer::new("sender-unretainable");
    peer.dir.break_file(&sender_path_in(&peer.dir.0));
    let stoa = genesis("Agora").address().unwrap();
    peer.worker().perform(Action::Open(stoa));

    assert!(peer.fake.creates().is_empty());
    let logged = peer.journal.with("no sender identifier could be retained");
    assert_eq!(logged.len(), 1, "{:?}", peer.journal.lines());
    assert!(logged[0].contains(&stoa.to_hex()));
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
    std::thread::sleep(Duration::from_millis(300)); // judged, and waiting to append
    let free = channels.book.try_lock().is_ok();
    blocker.execute_batch("ROLLBACK").unwrap();

    assert!(free, "the channel book was held while the op waited to be appended");
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

    let mut opening = channels.opening(&identity);
    let op = their_op(stoa, "waits out another channel's answer", 0);
    decide_elsewhere(
        peer.processor(Arc::clone(&channels)),
        arriving(identity.channel_id(), op.to_bytes().unwrap(), 1),
    );
    std::thread::sleep(Duration::from_millis(100));
    drop(channels.opening(&other)); // another open settles and wakes every waiter
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        peer.journal.lines().is_empty(),
        "judged before its own open settled: {:?}",
        peer.journal.lines()
    );

    opening.held();
    drop(opening);
    eventually("the op to be stored once its open is answered", || {
        stored(&peer, &op.op.id()).is_some()
    });
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
    // One case per refusal the boundary makes, through the processor. A table
    // rather than a test each: the requirement is one sentence over all of them.
    // The names are design.md's, hardcoded; `distinct` is the property that
    // matters beyond the spelling — no two refusals share a name.
    let stoa = genesis("Agora").address().unwrap();
    let other = genesis("Lyceum").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let open = identity.channel_id();
    let window = crate::arrival::RECEIVE_WINDOW_MS;
    let mut forged = their_op(stoa, "a forgery", 0);
    forged.op.author = key(9).public_key();
    let cases: [(&str, &str, Vec<u8>); 6] = [
        ("unknown-channel", "/elsewhere/zzyzx", vec![1, 2, 3]),
        (
            "too-long",
            open,
            vec![0; crate::transport::MAX_MESSAGE_BYTES + 1],
        ),
        ("undecodable", open, b"not an op".to_vec()),
        ("fails-verification", open, forged.to_bytes().unwrap()),
        (
            "stoa-mismatch",
            open,
            their_op(other, "copied onto another channel", 0)
                .to_bytes()
                .unwrap(),
        ),
        (
            "ahead-of-time",
            open,
            their_op(stoa, "from the future", window + 60_000)
                .to_bytes()
                .unwrap(),
        ),
    ];

    let mut names = Vec::new();
    for (kind, channel, payload) in cases {
        let peer = Peer::new(&format!("recv-refusal-{kind}"));
        peer.processor(open_for(&stoa)).decide(&Arriving {
            channel_id: channel.to_string(),
            sender_id: "zzyzx-sender".to_string(),
            payload,
            timestamp: 1,
        });
        let lines = peer.journal.lines();
        assert_eq!(
            peer.journal.with(&format!("refused ({kind})")).len(),
            1,
            "{kind}: {lines:?}"
        );
        for line in &lines {
            assert!(!line.contains("zzyzx"), "{kind}: {line}");
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
fn every_discard_is_counted_and_logged_apart_from_refusals() {
    let journal = Recorder::default();
    let queue = InboundQueue::with_bound(1);
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    listen(
        (0..4).map(|n| Some(arriving(channel.channel_id(), vec![n], 0))),
        &open_for(&stoa),
        &queue,
        &journal,
    );

    let discards = journal.with("discarded");
    assert_eq!(discards.len(), 3, "{:?}", journal.lines());
    assert!(discards[2].contains("3 discarded since"), "{}", discards[2]);
    assert!(discards[2].contains("full at 1;"), "{}", discards[2]);
    for line in &discards {
        assert!(!line.contains("refused"), "{line}");
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
