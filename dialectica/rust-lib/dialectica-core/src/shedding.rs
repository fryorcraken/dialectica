//! Which message a bound over its limit gives up: one rule, for the payloads
//! waiting to be taken and for the parked messages alike.
//!
//! `op-transport` states it once, for the waiting payloads, and points the
//! parked messages' total bounds at it ("This is the rule 'Parked messages are
//! bounded per channel and in total' applies to its total bounds"): **the newest
//! message of the channel holding the most, counting the arriving message with
//! its own channel**; among several holding the most, the arriving message's own
//! channel if it is among them, and otherwise the one whose newest message was
//! handed over latest.
//!
//! # One tie-break, not two
//!
//! The arriving message is counted with its own channel **as the newest message
//! of all** ([`Newest::Arrival`]). So "the arriving message's own channel if it
//! is among them" is the same tie-break as "the one whose newest was handed over
//! latest": the arrival's channel always has the latest newest. And when that
//! channel is chosen, its newest — the one given up — is the arrival itself,
//! which [`choose`] reports as [`Victim::Arrival`].
//!
//! That convention lives here, once. Each caller only says what it holds, in
//! hand-over order, and what it is about to add; neither the queue nor the
//! parked store can count the arrival differently from the other.
//!
//! # Why "newest", and what does not depend on it
//!
//! A waiting or parked payload is undecoded and unverified, so its op's counter
//! cannot be read: "newest" can only mean "handed over last". Losing the newest
//! is a heuristic for losing least — see `park-pending-inbound`'s design,
//! Decision 5 — and no correctness property rests on it.

use std::collections::HashMap;

/// What a bound over its limit gives up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Victim<'a> {
    /// The arriving message: its own channel holds the most, and it is that
    /// channel's newest. Every held message is kept.
    Arrival,
    /// The newest held message on this channel, which is not the arrival's.
    NewestOf(&'a str),
}

/// When a channel's newest message was handed over: a held one's place, or the
/// arrival, which is later than every held message.
///
/// An `enum` and not a `u64` with a sentinel, so "the arrival is newest of all"
/// is the variant order and cannot be mistyped at a second call site. `Ord` is
/// derived: `Held` before `Arrival`, then by place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Newest {
    Held(usize),
    Arrival,
}

/// One channel's load: how much it holds under what the bound counts, and when
/// its newest message was handed over.
#[derive(Debug, Clone, Copy)]
struct Load {
    measure: u64,
    newest: Newest,
}

/// The message a bound over its limit gives up.
///
/// `held` is every message held now, **in the order it was handed over**, as its
/// channel and what the bound counts of it (1 for a count bound, its bytes for a
/// byte bound). `arrival` is the message about to be added, likewise.
pub(crate) fn choose<'a>(
    held: impl IntoIterator<Item = (&'a str, u64)>,
    arrival: (&'a str, u64),
) -> Victim<'a> {
    let mut loads: HashMap<&'a str, Load> = HashMap::new();
    for (place, (channel, measure)) in held.into_iter().enumerate() {
        let load = loads.entry(channel).or_insert(Load {
            measure: 0,
            newest: Newest::Held(place),
        });
        load.measure = load.measure.saturating_add(measure);
        load.newest = Newest::Held(place);
    }
    let (arriving_channel, arriving_measure) = arrival;
    let own = loads.entry(arriving_channel).or_insert(Load {
        measure: 0,
        newest: Newest::Arrival,
    });
    own.measure = own.measure.saturating_add(arriving_measure);
    own.newest = Newest::Arrival;

    let most = loads.into_iter().max_by(|(_, a), (_, b)| {
        a.measure
            .cmp(&b.measure)
            .then_with(|| a.newest.cmp(&b.newest))
    });
    match most {
        Some((channel, _)) if channel != arriving_channel => Victim::NewestOf(channel),
        // The arrival's own channel, or — impossible, the arrival is always
        // counted — nothing at all: giving up the arrival keeps every bound.
        _ => Victim::Arrival,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ones<'a>(channels: &[&'a str]) -> Vec<(&'a str, u64)> {
        channels.iter().map(|c| (*c, 1)).collect()
    }

    #[test]
    fn the_channel_holding_the_most_gives_up_its_newest() {
        assert_eq!(
            choose(ones(&["a", "a", "b"]), ("c", 1)),
            Victim::NewestOf("a")
        );
    }

    #[test]
    fn the_most_wins_over_the_latest() {
        // `b`'s newest is later than `a`'s, but `a` holds more.
        assert_eq!(
            choose(ones(&["a", "a", "a", "b", "b"]), ("c", 1)),
            Victim::NewestOf("a")
        );
    }

    #[test]
    fn the_arrival_counts_with_its_own_channel_and_loses_a_tie() {
        // `a` holds two, `b` one; with the arrival `b` holds two as well, and the
        // arrival is the newest of all.
        assert_eq!(choose(ones(&["a", "b", "a"]), ("b", 1)), Victim::Arrival);
    }

    #[test]
    fn among_other_channels_tied_for_the_most_the_latest_newest_gives_it_up() {
        // a1 b1 b2 a2: tied at two, `a`'s newest (a2) is the later. A channel's
        // FIRST place would pick `b` here, so this tells the two apart.
        assert_eq!(
            choose(ones(&["a", "b", "b", "a"]), ("c", 1)),
            Victim::NewestOf("a")
        );
        // And the mirror, so neither name nor hash order decides it.
        assert_eq!(
            choose(ones(&["b", "a", "a", "b"]), ("c", 1)),
            Victim::NewestOf("b")
        );
    }

    #[test]
    fn a_byte_measure_is_summed_per_channel() {
        // `b` holds one message and the most bytes.
        let held = [("a", 10), ("a", 10), ("b", 30)];
        assert_eq!(choose(held, ("c", 5)), Victim::NewestOf("b"));
        // The arrival's own bytes put its channel over.
        assert_eq!(choose(held, ("a", 15)), Victim::Arrival);
    }

    #[test]
    fn with_nothing_held_the_arrival_is_given_up() {
        assert_eq!(choose(std::iter::empty(), ("a", 1)), Victim::Arrival);
    }
}
