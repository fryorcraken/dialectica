//! Parking: the three seams (`ChannelBook::on_take`, `ChannelBook::settle` and
//! `startup_review`, `Channels::take`), then what each requirement of
//! `op-transport`'s parking contract looks like from a message.
//!
//! Most tests drive a processor on the test's thread — `decide` to take one
//! message, `run_reviews` to run what a settle queued — so the order of events
//! is the test's and no clock is read. The running wiring is used where the
//! point is that one thread does not wait on another.

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// Channels with `stoa`'s channel being opened, and the guard that settles it.
fn opening_for(stoa: &Address) -> (Arc<Channels>, Opening) {
    let channels = Arc::new(Channels::default());
    let opening = channels.opening(&ChannelIdentity::of(stoa));
    (channels, opening)
}

/// A message carrying `op` on its own Stoa's channel.
fn on_its_channel(op: &SignedOp) -> Arriving {
    arriving(
        ChannelIdentity::of(&op.op.stoa).channel_id(),
        op.to_bytes().unwrap(),
        1,
    )
}

/// Settle `opening` as delivery holding the channel.
fn held(mut opening: Opening) {
    opening.held();
}

/// The log line the processor writes for an op it stored.
fn store_line(op: &SignedOp) -> String {
    format!("stored inbound op {}", op.op.id().to_hex())
}

// ─── The seams ────────────────────────────────────────────────────────────

#[test]
fn only_a_channel_being_opened_and_not_open_parks_what_is_taken_on_it() {
    // `ChannelBook::on_take`, the one place parking is decided, over the three
    // states the requirement names — and the fourth that reads like the second:
    // an open channel whose opening is asked for again.
    let open = ChannelIdentity::of(&genesis("Agora").address().unwrap());
    let opening = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let unknown = ChannelIdentity::of(&genesis("Athenaeum").address().unwrap());
    let mut book = ChannelBook::default();
    book.request(&open);
    assert_eq!(book.settle(&open, true), Some(Review::Held(open.channel_id().into())));
    book.request(&open); // asked again, unanswered
    book.request(&opening);

    assert_eq!(book.on_take(open.channel_id()), Taken::Judge);
    assert_eq!(book.on_take(opening.channel_id()), Taken::Park);
    assert_eq!(book.on_take(unknown.channel_id()), Taken::Judge);
}

#[test]
fn a_settle_begins_a_review_only_when_held_or_when_the_last_request_leaves_it_unopened() {
    // `ChannelBook::settle`, the one place a settle becomes a review, over every
    // shape the requirement distinguishes.
    let channel = ChannelIdentity::of(&genesis("Agora").address().unwrap());
    let id = || channel.channel_id().to_string();

    // One request, held: event 1.
    let mut book = ChannelBook::default();
    book.request(&channel);
    assert_eq!(book.settle(&channel, true), Some(Review::Held(id())));

    // One request, declined or given up: event 2.
    let mut book = ChannelBook::default();
    book.request(&channel);
    assert_eq!(book.settle(&channel, false), Some(Review::Unopened(id())));
    assert!(!book.is_known(channel.channel_id()));

    // Two requests: the first declined begins nothing, the second held does.
    let mut book = ChannelBook::default();
    book.request(&channel);
    book.request(&channel);
    assert_eq!(book.settle(&channel, false), None);
    assert_eq!(book.on_take(channel.channel_id()), Taken::Park);
    assert_eq!(book.settle(&channel, true), Some(Review::Held(id())));

    // Two requests, both declined: only the last begins a review.
    let mut book = ChannelBook::default();
    book.request(&channel);
    book.request(&channel);
    assert_eq!(book.settle(&channel, false), None);
    assert_eq!(book.settle(&channel, false), Some(Review::Unopened(id())));

    // A repeat for a channel already open, declined: still open, so no review.
    let mut book = ChannelBook::default();
    book.request(&channel);
    book.settle(&channel, true);
    book.request(&channel);
    assert_eq!(book.settle(&channel, false), None);
    assert_eq!(book.on_take(channel.channel_id()), Taken::Judge);
}

#[test]
fn the_startup_review_names_the_channels_startup_counts_as_being_opened() {
    let a = ChannelIdentity::of(&genesis("Agora").address().unwrap());
    let b = ChannelIdentity::of(&genesis("Lyceum").address().unwrap());
    let mut book = ChannelBook::default();
    book.request(&a);
    book.request(&b);
    let expected: HashSet<String> = [a.channel_id(), b.channel_id()]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(book.startup_review(), Review::Startup(expected));
}

#[test]
fn a_payload_taken_before_a_settle_is_parked_and_the_settles_review_comes_next() {
    // `op-transport`, "Every message is decided exactly once, however close to a
    // settle it is taken": a message taken before the event and parked is
    // decided by the review that event begins. Read off `Channels::take`.
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let message = on_its_channel(&their_op(stoa, "taken before the settle", 0));
    assert_eq!(channels.hand_over(message.clone()), HandedOver::Waiting);
    // Closed, so a take that finds nothing returns `None` and fails the test
    // rather than blocking it.
    channels.close();

    let first = channels.take().unwrap();
    assert!(
        matches!(&first, Next::Payload(m, Taken::Park) if *m == message),
        "{first:?}"
    );
    held(opening);
    let second = channels.take().unwrap();
    assert!(
        matches!(&second, Next::Review(Review::Held(_))),
        "{second:?}"
    );
}

#[test]
fn a_review_is_taken_before_a_payload_that_was_waiting_when_its_event_came() {
    // The other side of the same requirement: a review "MUST decide all of them
    // before any message on that channel taken after the event that began the
    // review is judged". The payload was handed over before the settle and is
    // taken after it: the review comes first, and the payload, taken after the
    // event, is judged — not parked.
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let message = on_its_channel(&their_op(stoa, "taken after the settle", 0));
    channels.hand_over(message.clone());
    held(opening);
    channels.close(); // a take that finds nothing fails rather than blocks

    let first = channels.take().unwrap();
    assert!(
        matches!(&first, Next::Review(Review::Held(_))),
        "{first:?}"
    );
    let second = channels.take().unwrap();
    assert!(
        matches!(&second, Next::Payload(m, Taken::Judge) if *m == message),
        "{second:?}"
    );
}

// ─── Parking ──────────────────────────────────────────────────────────────

#[test]
fn a_message_on_a_channel_being_opened_is_parked_not_judged() {
    // `op-transport`, scenario "A message on a channel being opened is parked,
    // not judged".
    let peer = Peer::new("park-not-judged");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "parked", 0);

    peer.processor(Arc::clone(&channels))
        .decide(&on_its_channel(&op));

    assert_eq!(
        peer.journal.with("parked until").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
    assert!(stored(&peer, &op.op.id()).is_none());
    assert!(peer.journal.with("refused").is_empty());
    assert_eq!(peer.parked_on(&stoa), 1);
    drop(opening);
}

#[test]
fn a_message_on_a_channel_not_being_opened_does_not_wait_on_another_channels_open() {
    // `op-transport`, scenario "A message on a channel not being opened does not
    // wait on another channel's open": refused as an unknown channel while that
    // open is unanswered, and not parked.
    let peer = Peer::new("park-other-channel");
    let opening_stoa = genesis("Agora").address().unwrap();
    let elsewhere = genesis("Lyceum").address().unwrap();
    let (channels, unanswered) = opening_for(&opening_stoa);
    let op = their_op(elsewhere, "on a channel nobody asked for", 0);

    peer.processor(Arc::clone(&channels))
        .decide(&on_its_channel(&op));

    assert_eq!(peer.journal.with("refused (unknown-channel)").len(), 1);
    assert!(peer.journal.with("parked until").is_empty());
    assert!(stored(&peer, &op.op.id()).is_none());
    drop(unanswered); // still unanswered until here
}

#[test]
fn a_payload_that_does_not_decode_is_parked_and_refused_by_its_review() {
    // `op-transport`, scenario "A payload that does not decode is parked, and
    // refused by its review": parking judges nothing.
    let peer = Peer::new("park-undecodable");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let processor = peer.processor(Arc::clone(&channels));
    processor.decide(&arriving(
        ChannelIdentity::of(&stoa).channel_id(),
        b"not an op".to_vec(),
        1,
    ));
    assert_eq!(peer.journal.with("parked until").len(), 1);
    assert!(peer.journal.with("refused").is_empty());

    held(opening);
    processor.run_reviews();
    assert_eq!(
        peer.journal.with("refused (undecodable)").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_message_that_cannot_be_parked_is_logged_and_not_retried() {
    // `op-transport`, scenario "A message that cannot be parked is logged and not
    // retried". A directory where the file goes makes the store unopenable.
    let peer = Peer::new("park-unwritable");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let file = crate::parked::parked_path_in(&peer.dir.0);
    peer.dir.break_file(&file);
    let op = their_op(stoa, "nowhere to park it", 0);
    let processor = peer.processor(Arc::clone(&channels));

    processor.decide(&on_its_channel(&op));
    assert_eq!(
        peer.journal.with("not parked (storage").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );

    std::fs::remove_dir_all(&file).unwrap();
    held(opening);
    processor.run_reviews();
    assert!(stored(&peer, &op.op.id()).is_none());
    assert_eq!(peer.parked_on(&stoa), 0);
}

#[test]
fn a_parked_op_is_not_readable_as_an_op_and_does_not_move_its_stoas_clock() {
    // `op-transport`, scenarios "A parked op is not readable as an op" and "A
    // parked op does not move its Stoa's clock". The op log never sees a parked
    // row, so nothing read from it can.
    let peer = Peer::new("park-not-an-op");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    let held_before = their_op(stoa, "held before", 0);
    peer.processor(open_for(&stoa))
        .decide(&on_its_channel(&held_before));
    let clock_before = peer.dir.op_log().clock(&stoa).unwrap();

    let (channels, opening) = opening_for(&stoa);
    let ahead = their_op(stoa, "parked, with a later counter", 60_000);
    peer.processor(Arc::clone(&channels))
        .decide(&on_its_channel(&ahead));

    assert_eq!(peer.parked_on(&stoa), 1);
    assert!(stored(&peer, &ahead.op.id()).is_none());
    assert_eq!(peer.dir.op_log().len().unwrap(), 1);
    assert_eq!(peer.dir.op_log().clock(&stoa).unwrap(), clock_before);
    let listed = crate::wire::list_threads_from_request(&join_request(&g), || {
        Ok::<_, OpLogError>(peer.dir.op_log())
    });
    assert!(listed.contains(&held_before.op.id().to_hex()), "{listed}");
    assert!(!listed.contains(&ahead.op.id().to_hex()), "{listed}");
    drop(opening);
}

// ─── Reviews ──────────────────────────────────────────────────────────────

#[test]
fn messages_parked_on_an_open_delivery_declines_are_refused_as_an_unknown_channel() {
    // `op-transport`, scenario "Messages parked on an open delivery declines are
    // refused as an unknown channel" — and, with the guard dropped as the worker
    // drops it on a timeout, "… this peer gives up are refused once it is given
    // up": both are the second event.
    let peer = Peer::new("review-declined");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let processor = peer.processor(Arc::clone(&channels));
    for n in 0..2 {
        processor.decide(&on_its_channel(&their_op(stoa, &format!("declined {n}"), 0)));
    }
    assert!(peer.journal.with("refused").is_empty(), "before the answer");

    drop(opening);
    processor.run_reviews();
    assert_eq!(
        peer.journal.with("refused (unknown-channel)").len(),
        2,
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(peer.parked_on(&stoa), 0);
}

#[test]
fn a_parked_message_is_stored_once_delivery_answers_the_channel_already_exists() {
    // `op-transport`, scenario "A parked message is stored once delivery reports
    // the channel already exists", through the worker: the answer is delivery's
    // "already exists", which the worker reads as held.
    let peer = Peer::new("review-already-exists");
    let stoa = genesis("Agora").address().unwrap();
    let channels = Arc::new(Channels::default());
    let _ = peer
        .fake
        .script(|s| s.create_replies.push_back(the_already_exists_answer(&stoa)));
    let worker = Worker {
        channels: Arc::clone(&channels),
        ..peer.worker()
    };
    let opening = channels.opening(&ChannelIdentity::of(&stoa));
    let op = their_op(stoa, "parked before already-exists", 0);
    let processor = peer.processor(Arc::clone(&channels));
    processor.decide(&on_its_channel(&op));

    worker.open(opening);
    processor.run_reviews();
    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_declined_request_leaves_parked_messages_parked_while_another_request_is_unsettled() {
    // `op-transport`, scenario "A declined request leaves parked messages parked
    // while another request is unsettled".
    let peer = Peer::new("review-two-requests");
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    let channels = Arc::new(Channels::default());
    let first = channels.opening(&channel);
    let second = channels.opening(&channel);
    let op = their_op(stoa, "waits for the second request", 0);
    let processor = peer.processor(Arc::clone(&channels));
    processor.decide(&on_its_channel(&op));

    drop(first); // declined
    processor.run_reviews();
    assert!(
        peer.journal.with("refused").is_empty(),
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(peer.parked_on(&stoa), 1);

    held(second);
    processor.run_reviews();
    assert!(stored(&peer, &op.op.id()).is_some());
}

#[test]
fn nothing_but_the_three_events_reviews_a_parked_message() {
    // `op-transport`, scenario "Nothing but the three events reviews a parked
    // message": more messages on its channel and on an open one, the Stoa
    // joined again, and a post published elsewhere, with delivery not answering.
    let mut peer = Peer::new("review-nothing-else");
    let open = genesis("Agora");
    let stuck = genesis("Lyceum");
    let (events, asked) = peer.start_counted(vec![]);
    peer.join(&open);
    peer.delivering.settle(); // Agora is open
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    peer.join(&stuck);
    let stuck = stuck.address().unwrap();

    let parked = their_op(stuck, "parked first", 0);
    events.send(Some(on_its_channel(&parked))).unwrap();
    eventually("the first message to be parked", || {
        peer.journal.with("parked until").len() == 1
    });
    let later = their_op(stuck, "parked after it", 1);
    let elsewhere = their_op(open.address().unwrap(), "on the open channel", 0);
    events.send(Some(on_its_channel(&later))).unwrap();
    events.send(Some(on_its_channel(&elsewhere))).unwrap();
    handed_over(&asked, 3);
    peer.join(&genesis("Lyceum")); // joined again, still unanswered
    peer.post(&open.address().unwrap(), "a post into another Stoa");
    eventually("the message on the open channel to be stored", || {
        stored(&peer, &elsewhere.op.id()).is_some()
    });

    assert!(stored(&peer, &parked.op.id()).is_none());
    assert!(
        peer.journal.with("refused").is_empty(),
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(peer.parked_on(&stuck), 2);
    assert_eq!(peer.fake.answered_creates(), 1, "an open was answered");
    gate.release();
}

static AT_PARK_AHEAD: AtomicU64 = AtomicU64::new(0);
fn at_park_ahead() -> u64 {
    AT_PARK_AHEAD.load(Ordering::SeqCst)
}

#[test]
fn a_parked_op_is_judged_against_this_peers_clock_at_its_review() {
    // `op-transport`, scenario "A parked op is judged against this peer's clock
    // at its review": more than an hour ahead when parked, within the hour when
    // reviewed. Judged at park, it would have been refused.
    let peer = Peer::new("review-clock-advanced");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let window = crate::arrival::RECEIVE_WINDOW_MS;
    let op = their_op(stoa, "ahead when parked", window + 60_000);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.clock = at_park_ahead;
    AT_PARK_AHEAD.store(NOW_MS, Ordering::SeqCst);
    processor.decide(&on_its_channel(&op));
    assert!(peer.journal.with("refused").is_empty());

    AT_PARK_AHEAD.store(NOW_MS + 120_000, Ordering::SeqCst);
    held(opening);
    processor.run_reviews();
    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

static AT_PARK_WITHIN: AtomicU64 = AtomicU64::new(0);
fn at_park_within() -> u64 {
    AT_PARK_WITHIN.load(Ordering::SeqCst)
}

#[test]
fn a_parked_op_that_has_come_to_be_ahead_of_this_peers_time_is_refused_at_its_review() {
    // `op-transport`, scenario "A parked op that has come to be ahead of this
    // peer's time is refused at its review".
    let peer = Peer::new("review-clock-moved-back");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "within the hour when parked", 0);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.clock = at_park_within;
    AT_PARK_WITHIN.store(NOW_MS, Ordering::SeqCst);
    processor.decide(&on_its_channel(&op));

    AT_PARK_WITHIN.store(
        NOW_MS - crate::arrival::RECEIVE_WINDOW_MS - 60_000,
        Ordering::SeqCst,
    );
    held(opening);
    processor.run_reviews();
    assert!(stored(&peer, &op.op.id()).is_none());
    assert_eq!(peer.journal.with("(ahead-of-time)").len(), 1);
}

#[test]
fn parked_messages_are_decided_before_later_messages_on_their_channel_in_hand_over_order() {
    // `op-transport`, scenario "Parked messages are decided before later messages
    // on their channel, in hand-over order": A and B parked, the channel
    // reported created, then C taken on it. C was handed over before the answer
    // and is taken after it, which is the case a review queued behind payloads
    // would get wrong.
    let peer = Peer::new("review-order");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let [a, b, c] = ["A", "B", "C"].map(|body| their_op(stoa, body, 0));
    for op in [&a, &b, &c] {
        hand_over(on_its_channel(op), &channels, &*peer.journal);
    }
    let processor = peer.processor(Arc::clone(&channels));
    channels.close(); // a take that finds nothing fails rather than blocks
    for _ in 0..2 {
        match channels.take() {
            Some(next @ Next::Payload(_, Taken::Park)) => processor.act(next),
            other => panic!("expected a park, took {other:?}"),
        }
    }
    held(opening);
    processor.run();

    let lines = peer.journal.lines();
    let at = |op: &SignedOp| {
        lines
            .iter()
            .position(|l| l.contains(&store_line(op)))
            .unwrap_or_else(|| panic!("{} was not stored: {lines:?}", op.op.id().to_hex()))
    };
    assert!(at(&a) < at(&b) && at(&b) < at(&c), "{lines:?}");
}

#[test]
fn parked_messages_a_review_could_not_read_are_decided_by_the_channels_next_review() {
    // `op-transport`, scenario "Parked messages a review could not read are
    // decided by the channel's next review". The file is moved aside and a
    // directory put in its place for the first review, then moved back.
    let peer = Peer::new("review-unreadable");
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "read at the second review", 0);
    let processor = peer.processor(Arc::clone(&channels));
    processor.decide(&on_its_channel(&op));

    let file = crate::parked::parked_path_in(&peer.dir.0);
    let aside = file.with_extension("aside");
    std::fs::rename(&file, &aside).unwrap();
    peer.dir.break_file(&file);
    held(opening);
    processor.run_reviews();
    assert_eq!(
        peer.journal.with("could not be read for review (storage").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );

    std::fs::remove_dir_all(&file).unwrap();
    std::fs::rename(&aside, &file).unwrap();
    held(channels.opening(&channel)); // joined again: "already exists"
    processor.run_reviews();
    assert!(
        stored(&peer, &op.op.id()).is_some(),
        "{:?}",
        peer.journal.lines()
    );
}

#[test]
fn a_message_a_review_decided_is_not_decided_again() {
    // `op-transport`, scenario "A message a review decided is not decided
    // again": a repeated held answer and a startup review after the store
    // find nothing of it.
    let peer = Peer::new("review-once");
    let stoa = genesis("Agora").address().unwrap();
    let channel = ChannelIdentity::of(&stoa);
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "decided once", 0);
    let processor = peer.processor(Arc::clone(&channels));
    processor.decide(&on_its_channel(&op));
    held(opening);
    processor.run_reviews();

    held(channels.opening(&channel));
    processor.run_reviews();
    processor.act(Next::Review(Review::Startup(
        [channel.channel_id().to_string()].into(),
    )));

    assert_eq!(peer.journal.with(&store_line(&op)).len(), 1);
    assert!(peer.journal.with("already held").is_empty());
    assert!(peer.journal.with("refused").is_empty());
}

// ─── The running wiring ───────────────────────────────────────────────────

/// A running peer with Agora open and Lyceum's open asked of delivery and held
/// at the returned gate. Returns the two Stoas.
fn one_open_one_unanswered(
    peer: &mut Peer,
) -> (mpsc::Sender<Option<Arriving>>, Arc<Gate>, Address, Address) {
    let events = peer.start_listening();
    let open = genesis("Agora");
    peer.join(&open);
    peer.delivering.settle();
    let gate = Gate::closed();
    let _ = peer.fake.script(|s| s.create_gate = Some(gate.clone()));
    let stuck = genesis("Lyceum");
    peer.join(&stuck);
    eventually("the second open to reach delivery", || {
        peer.fake.creates().len() == 2
    });
    (
        events,
        gate,
        open.address().unwrap(),
        stuck.address().unwrap(),
    )
}

#[test]
fn an_unanswered_open_holds_up_no_other_channel() {
    // `op-transport`, scenario "An unanswered open holds up no other channel".
    let mut peer = Peer::new("wiring-holds-nothing-up");
    let (events, gate, open, stuck) = one_open_one_unanswered(&mut peer);
    for n in 0..3 {
        let op = their_op(stuck, &format!("on the unanswered open {n}"), 0);
        events.send(Some(on_its_channel(&op))).unwrap();
    }
    let valid = their_op(open, "behind three on the unanswered open", 0);
    events.send(Some(on_its_channel(&valid))).unwrap();

    eventually("the op on the open channel to be stored", || {
        stored(&peer, &valid.op.id()).is_some()
    });
    assert_eq!(peer.fake.answered_creates(), 1, "the second open was answered");
    assert_eq!(peer.journal.with("parked until").len(), 3);
    assert!(peer.journal.with("refused").is_empty());
    gate.release();
}

#[test]
fn messages_parked_on_an_unanswered_open_are_stored_once_delivery_reports_the_channel_created()
{
    // `op-transport`, scenario "Messages parked on an unanswered open are stored
    // once delivery reports the channel created".
    let mut peer = Peer::new("wiring-parked-then-created");
    let (events, gate, _, stuck) = one_open_one_unanswered(&mut peer);
    let ops: Vec<SignedOp> = (0..3)
        .map(|n| their_op(stuck, &format!("parked {n}"), 0))
        .collect();
    for op in &ops {
        events.send(Some(on_its_channel(op))).unwrap();
    }
    eventually("all three to be parked", || {
        peer.journal.with("parked until").len() == 3
    });

    gate.release();
    for op in &ops {
        eventually("a parked op to be stored", || {
            stored(&peer, &op.op.id()).is_some()
        });
    }
    assert_eq!(peer.parked_on(&stuck), 0);
}

#[test]
fn messages_taken_around_a_settle_are_each_decided_once() {
    // `op-transport`, scenario "Messages taken around a settle are each decided
    // once": handed over one after another, starting while the only request is
    // unanswered and continuing after delivery reports the channel created.
    let mut peer = Peer::new("wiring-around-a-settle");
    let (events, gate, _, stuck) = one_open_one_unanswered(&mut peer);
    let ops: Vec<SignedOp> = (0..40u64)
        .map(|n| their_op(stuck, &format!("around the settle {n}"), n))
        .collect();
    for (n, op) in ops.iter().enumerate() {
        if n == 20 {
            gate.release();
        }
        events.send(Some(on_its_channel(op))).unwrap();
    }
    for op in &ops {
        eventually("every op to be stored", || {
            stored(&peer, &op.op.id()).is_some()
        });
    }
    for op in &ops {
        assert_eq!(peer.journal.with(&store_line(op)).len(), 1);
    }
    assert!(peer.journal.with("already held").is_empty());
    assert_eq!(peer.parked_on(&stuck), 0);
}

#[test]
fn startup_refuses_a_parked_message_on_a_channel_it_does_not_open() {
    // `op-transport`, scenario "Startup refuses a parked message on a channel it
    // does not open". The first run parks and stops with the open unsettled:
    // its guard is dropped into a book no processor runs.
    let peer = Peer::new("startup-refuses");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    peer.processor(Arc::clone(&channels))
        .decide(&on_its_channel(&their_op(stoa, "parked last run", 0)));
    drop(opening);
    assert_eq!(peer.parked_on(&stoa), 1);

    let mut peer = peer.restarted();
    peer.dir.break_file(&membership_path_in(&peer.dir.0));
    peer.start();
    eventually("the parked message to be refused", || {
        !peer.journal.with("refused (unknown-channel)").is_empty()
    });
    assert_eq!(peer.parked_on(&stoa), 0);
}

#[test]
fn a_message_parked_before_a_restart_waits_for_and_is_stored_by_startups_open() {
    // `op-transport`, scenarios "Startup leaves a parked message on a channel it
    // opens for that open's answer" and "A message parked before a restart is
    // stored once startup's open is answered". Node creation is held, so
    // startup's open of the channel waits behind it.
    //
    // **How "before delivery answers" is told without a pause:** a junk payload
    // handed over after startup is parked, and a payload is taken only once
    // every waiting review has run — so by its park line, the startup review has
    // run and has left the message parked.
    let peer = Peer::new("startup-leaves");
    let g = genesis("Agora");
    let stoa = g.address().unwrap();
    peer.join(&g); // the membership the restart finds
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "parked last run", 0);
    peer.processor(Arc::clone(&channels))
        .decide(&on_its_channel(&op));
    drop(opening);

    let mut peer = peer.restarted();
    let node = Gate::closed();
    let _ = peer.fake.script(|s| {
        s.node_gate = Some(node.clone());
        s.create_replies.push_back(the_already_exists_answer(&stoa));
    });
    let events = peer.start_listening();
    events
        .send(Some(arriving(
            ChannelIdentity::of(&stoa).channel_id(),
            b"junk after startup".to_vec(),
            1,
        )))
        .unwrap();
    eventually("the junk to be parked", || {
        !peer.journal.with("parked until").is_empty()
    });
    assert!(stored(&peer, &op.op.id()).is_none());
    assert!(
        peer.journal.with("refused").is_empty(),
        "{:?}",
        peer.journal.lines()
    );
    assert_eq!(peer.parked_on(&stoa), 2);

    node.release();
    eventually("the op parked last run to be stored", || {
        stored(&peer, &op.op.id()).is_some()
    });
    eventually("the junk to be refused", || {
        !peer.journal.with("refused (undecodable)").is_empty()
    });
}

// ─── Bounds ───────────────────────────────────────────────────────────────

#[test]
fn a_channel_at_its_count_bound_discards_the_arrival_and_every_earlier_one_is_stored() {
    // `op-transport`, scenario "A channel at its count bound discards the arrival
    // and keeps what it parked", through the processor, with the bounds shrunk on
    // the one processor this builds.
    let peer = Peer::new("bound-per-channel");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.bounds.per_channel_count = 2;
    let ops: Vec<SignedOp> = (0..3)
        .map(|n| their_op(stoa, &format!("bounded {n}"), 0))
        .collect();
    for op in &ops {
        processor.decide(&on_its_channel(op));
    }
    assert_eq!(
        peer.journal.with("discarded from the parked messages").len(),
        1,
        "{:?}",
        peer.journal.lines()
    );

    held(opening);
    processor.run_reviews();
    assert!(stored(&peer, &ops[0].op.id()).is_some());
    assert!(stored(&peer, &ops[1].op.id()).is_some());
    assert!(stored(&peer, &ops[2].op.id()).is_none());
}

#[test]
fn a_discard_from_the_parked_messages_shares_the_running_count_and_is_logged_apart() {
    // `op-transport`, scenario "A discard from the parked messages shares the
    // running count and is logged apart".
    let peer = Peer::new("bound-shared-count");
    let open = genesis("Agora").address().unwrap();
    let opening_stoa = genesis("Lyceum").address().unwrap();
    let channels = Arc::new(Channels::with_bound(1));
    held(channels.opening(&ChannelIdentity::of(&open)));
    let opening = channels.opening(&ChannelIdentity::of(&opening_stoa));
    for n in 0..2 {
        hand_over(
            on_its_channel(&their_op(open, &format!("queued {n}"), 0)),
            &channels,
            &*peer.journal,
        );
    }
    let mut processor = peer.processor(Arc::clone(&channels));
    processor.bounds.per_channel_count = 1;
    for n in 0..2 {
        processor.decide(&on_its_channel(&their_op(
            opening_stoa,
            &format!("parked {n}"),
            0,
        )));
    }

    let queue = peer.journal.with("queue full");
    let parked = peer.journal.with("discarded from the parked messages");
    assert_eq!(queue.len(), 1, "{:?}", peer.journal.lines());
    assert_eq!(parked.len(), 1, "{:?}", peer.journal.lines());
    assert!(parked[0].contains("; 2 discarded since"), "{}", parked[0]);
    for line in queue.iter().chain(&parked) {
        assert!(!line.contains("refused"), "{line}");
    }
    assert!(!queue[0].contains("parked"), "{}", queue[0]);
    drop(opening);
}

#[test]
fn a_parked_message_carries_no_sender_identifier_into_its_review() {
    // `op-transport`: "A parked message is put through the boundary without a
    // sender identifier, because none is kept for it." The boundary decides
    // nothing from one, so this is the store's property seen from the
    // processor: a distinctive sender identifier is nowhere in the file.
    let peer = Peer::new("park-no-sender");
    let stoa = genesis("Agora").address().unwrap();
    let (channels, opening) = opening_for(&stoa);
    let op = their_op(stoa, "parked without its sender", 0);
    let mut message = on_its_channel(&op);
    message.sender_id = "zzyzx-sender-identifier".to_string();
    peer.processor(Arc::clone(&channels)).decide(&message);
    assert_eq!(peer.parked_on(&stoa), 1);

    let bytes = std::fs::read(crate::parked::parked_path_in(&peer.dir.0)).unwrap();
    assert!(!bytes
        .windows(b"zzyzx".len())
        .any(|w| w == b"zzyzx"));
    drop(opening);
}
