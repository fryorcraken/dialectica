//! The channel book: which channels are open and which are being opened, and
//! the two answers it gives — what to do with a payload just taken ([`Taken`])
//! and which review a settle or the first startup begins ([`Review`]). Plain
//! data and rules, with no I/O and no thread: the lock that orders every read
//! against every change is [`Channels`](super::Channels)'s.

use crate::identity::Address;
use crate::transport::{ChannelIdentity, OpenChannels};
use std::collections::{HashMap, HashSet};

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
///
/// [`channel_answer`]: super::channel_answer
/// [`Opening`]: super::Opening
/// [`Action::Open`]: super::Action::Open
#[derive(Default)]
pub(super) struct ChannelBook {
    pub(super) open: OpenChannels,
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
pub(super) enum Taken {
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
///
/// [`Processor::review`]: super::Processor::review
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Review {
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
    pub(super) fn is_known(&self, channel_id: &str) -> bool {
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
    pub(super) fn on_take(&self, channel_id: &str) -> Taken {
        match self.open.stoa_of(channel_id) {
            Some(stoa) => Taken::Judge(Some(*stoa)),
            None if self.opening.contains_key(channel_id) => Taken::Park,
            None => Taken::Judge(None),
        }
    }

    /// A create, a join or startup asked for this channel.
    pub(super) fn request(&mut self, identity: &ChannelIdentity) {
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
    pub(super) fn settle(&mut self, identity: &ChannelIdentity, held: bool) -> Option<Review> {
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
    pub(super) fn startup_review(&self) -> Review {
        Review::Startup(self.opening.keys().cloned().collect())
    }
}
