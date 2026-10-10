//! The inbound queue: the payloads waiting for the processor, bounded by a
//! fixed count ([`INBOUND_BOUND`]), and the rule for which payload a full queue
//! loses ([`InboundQueue::offer`]). Plain data and rules, with no I/O and no
//! thread: the lock every offer and every take is made under is
//! [`Channels`](super::Channels)'s.

use super::Arriving;
use crate::shedding::{choose, Victim};
use std::collections::VecDeque;

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
///
/// [`refused_on_hand_over`]: super::refused_on_hand_over
pub const INBOUND_BOUND: usize = 256;

/// What offering a message did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Offered {
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
///
/// [`Channels`]: super::Channels
pub(super) struct InboundQueue {
    messages: VecDeque<Arriving>,
    pub(super) bound: usize,
}

impl InboundQueue {
    pub(super) fn with_bound(bound: usize) -> Self {
        InboundQueue {
            messages: VecDeque::new(),
            bound,
        }
    }

    pub(super) fn offer(&mut self, message: Arriving) -> Offered {
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
    pub(super) fn pop(&mut self) -> Option<Arriving> {
        self.messages.pop_front()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.messages.len()
    }
}
