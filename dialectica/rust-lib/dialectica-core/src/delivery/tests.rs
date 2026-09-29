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

/// What the fake answers, set per test.
struct Script {
    create_node: Result<Value, String>,
    start_node: Result<Value, String>,
    /// Channel ids whose creation is declined, with the reply to give.
    declined_channels: HashMap<String, Result<Value, String>>,
    send: Option<Result<Value, String>>,
    /// How long a call of this kind takes before it answers.
    create_delay: Duration,
    send_delay: Duration,
    panic_on_create_node: bool,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            create_node: Ok(json!(true)),
            start_node: Ok(json!(true)),
            declined_channels: HashMap::new(),
            send: None,
            create_delay: Duration::ZERO,
            send_delay: Duration::ZERO,
            panic_on_create_node: false,
        }
    }
}

#[derive(Default)]
struct FakeState {
    calls: Mutex<Vec<Call>>,
    script: Mutex<Script>,
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
        let (delay, declined) = {
            let script = lock(&self.0.script);
            (
                script.create_delay,
                script.declined_channels.get(channel_id).cloned(),
            )
        };
        std::thread::sleep(delay);
        declined.unwrap_or_else(|| Ok(json!(channel_id)))
    }

    fn channel_send(&self, channel_id: &str, payload: &[u8]) -> Result<Value, String> {
        lock(&self.0.calls).push(Call::ChannelSend {
            channel_id: channel_id.to_string(),
            payload: payload.to_vec(),
        });
        let (delay, scripted) = {
            let script = lock(&self.0.script);
            (script.send_delay, script.send.clone())
        };
        std::thread::sleep(delay);
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
    opening.created();
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
    // NO SPEC: the spec pins only the node. A second startup here asks for no
    // channel either; the first already asked for every membership's.
    assert_eq!(peer.fake.creates().len(), 1);
}

#[test]
fn a_failed_subscription_leaves_sending_wired() {
    // NO SPEC: the spec does not say what a failed subscription does. This logs
    // it and still wires the node, the channels and the sends.
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
    // NO SPEC: a request made before `start` is dropped, not held. The next
    // start asks for every membership's channel; the op is not re-sent.
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
    // NO SPEC: the spec is silent on a REPEAT open delivery declines. Delivery
    // created the channel once and nothing has closed it, so it stays open.
    let stoa = genesis("Agora").address().unwrap();
    let identity = ChannelIdentity::of(&stoa);
    let channels = open_for(&stoa);
    drop(channels.opening(&identity)); // asked again, declined
    assert!(lock(&channels.book).open.is_open(identity.channel_id()));
}

#[test]
fn an_op_log_that_will_not_open_is_a_storage_refusal() {
    // NO SPEC: the message is dropped and logged, not retried.
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
    // NO SPEC: the spec says what a declined creation must not stop; it is
    // silent on whether `start` is still asked for. This asks only after a
    // creation delivery accepted: a decline most often means another module
    // created the node, and that module owns its start.
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
    let code = adapter_code();
    assert!(
        !code.contains(".stop(") && !code.contains(".stop_with_timeout("),
        "the adapter calls delivery's stop"
    );
    assert_eq!(
        code.matches(".create_node").count(),
        1,
        "exactly one site asks delivery to create the node"
    );
    assert_eq!(code.matches(".start_with_timeout(").count(), 1);
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
}

/// The adapter's source without its comments.
fn adapter_code() -> String {
    include_str!("../../../src/lib.rs")
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
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

#[test]
fn an_unresponsive_delivery_does_not_delay_a_join() {
    let mut peer = Peer::new("join-slow");
    peer.fake = peer
        .fake
        .script(|s| s.create_delay = Duration::from_secs(3));
    peer.start();

    let began = Instant::now();
    let reply = peer.join(&genesis("Agora"));
    let took = began.elapsed();

    assert!(!reply.contains("error"), "{reply}");
    assert!(took < Duration::from_secs(1), "the join waited {took:?}");
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
    let mut peer = Peer::new("send-slow");
    peer.fake = peer.fake.script(|s| s.send_delay = Duration::from_secs(3));
    peer.start();
    let g = genesis("Agora");
    peer.join(&g);
    peer.delivering.settle();

    let began = Instant::now();
    let id = peer.post(&g.address().unwrap(), "not waited on");
    let took = began.elapsed();

    assert!(took < Duration::from_secs(1), "the publish waited {took:?}");
    assert!(stored(&peer, &id).is_some());
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
    // NO SPEC: the spec says a channel is open only once delivery answers, and is
    // silent on a message that arrives in the gap before the answer. This holds
    // it until the open settles, so delivery's answer decides it — stored here,
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
            opening.created();
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

    // Timed around `decide` alone: the scope joins the answering thread on the
    // way out, so a clock read after it would measure the sleep, not the wait.
    let waited = std::thread::scope(|scope| {
        let opening = channels.opening(&identity);
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(opening); // settled, not created
        });
        let began = Instant::now();
        peer.processor(Arc::clone(&channels)).decide(&arriving(
            identity.channel_id(),
            op.to_bytes().unwrap(),
            1,
        ));
        began.elapsed()
    });

    assert!(
        waited >= Duration::from_millis(150),
        "it did not wait: {waited:?}"
    );
    assert!(stored(&peer, &op.op.id()).is_none());
    assert_eq!(peer.journal.with("(unknown-channel)").len(), 1);
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
fn every_discard_is_counted_and_logged_apart_from_refusals() {
    let journal = Recorder::default();
    let queue = InboundQueue::with_bound(1);
    listen(
        (0..4).map(|n| Some(arriving("c", vec![n], 0))),
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
    assert_eq!(peer.journal.with("stored inbound op").len(), 2);
}
