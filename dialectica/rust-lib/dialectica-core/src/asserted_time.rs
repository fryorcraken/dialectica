//! The author's asserted time, on its way to a reader — as text, never a number.
//!
//! # Why this module exists, and why it returns a string
//!
//! An op carries a wall-clock its author chose. A reader is shown a time and
//! expects one, so the value has to reach the surface; and it decides nothing,
//! so it must not reach the surface in a form anything can decide with.
//!
//! **A number that means a time invites a sort**, and a view that sorted on this
//! one would produce a ranking every author can forge. So the contract
//! constrains the shape rather than only the uses: a read surfaces
//! [`AssertedTime`], whose value is display text. A view that wished to sort by
//! it would have to parse a string back into an instant first — which is the
//! intent. It makes the wrong thing visibly a wrong thing in the diff, rather
//! than a plausible-looking field access.
//!
//! **This is a measured failure mode rather than a hypothesis.** The nearest
//! comparable project reads an author-asserted timestamp for ranking decay with
//! nothing clamping it, so a post claiming a future time receives an unbounded
//! multiplier and pins itself above every honest post. Its validator does notice
//! future timestamps and produces a warning — which is never called on the
//! ingest path. The defect was not the missing check; it was that the value was
//! available as a number to whoever wanted to rank by it.
//!
//! # Clamping is defence in depth, and not the defence
//!
//! The defence is that the value orders nothing. A clamp on a field that decided
//! an ordering would still leave every op within the allowance reorderable by
//! its author. What the clamp buys is that a reader is not shown "posted in
//! 2387" — a display defect rather than a ranking one — and that a peer whose
//! own clock is wrong does not silently present other peers' honest times as
//! absurd.
//!
//! **The reading peer's own clock is itself untrusted for this**, and this
//! module does not pretend otherwise: a peer with a badly skewed clock clamps
//! honest ops and passes hostile ones. That is acceptable **only because nothing
//! depends on the outcome** — the clamp changes what is rendered and nothing
//! else. This asymmetry is exactly why clamping may not be promoted into an
//! ordering rule later.
//!
//! Because the clamp is computed against a local clock, two peers may present
//! one op differently, and a caller must not treat the displayed time as a value
//! two peers agree on.
//!
//! # The stored op is never altered
//!
//! Clamping is a **read-time presentation rule** and can be nothing else. The
//! wall-clock is inside the signed preimage, so a peer that rewrote it would
//! hold an op that no longer verifies and whose id no longer matches. That is
//! not a trade-off to revisit; it is why the boundary cannot clamp.

/// How far ahead of the reading peer's own clock an asserted time may be before
/// it is clamped for display.
///
/// Twenty-four hours. Comfortably above any honest clock skew — an NTP-corrected
/// clock sits within seconds, and an unsynchronised one drifts minutes a month —
/// and far below the range in which a rendered date stops looking odd.
///
/// **Pinned by a hardcoded assertion**, for the reason
/// [`crate::arrival::ADVANCE_BOUND`] is: `cargo mutants` does not mutate a
/// `const`.
pub const FUTURE_ALLOWANCE_MS: u64 = 24 * 60 * 60 * 1000;

/// The earliest asserted time that is presented as the author wrote it.
///
/// 2010-01-01T00:00:00Z, in milliseconds. Before any op of this system could
/// have been written, and **above zero**, which is the property that matters: a
/// floor of zero would clamp nothing at the bottom, and a zero wall-clock — the
/// value an unset system clock produces — would render as 1970.
pub const FLOOR_MS: u64 = 1_262_304_000_000;

/// An author's asserted time, ready to render.
///
/// # Two fields and no third
///
/// There is **no** field carrying the instant as a number, and adding one would
/// undo this module. The ordering position a view needs is carried separately by
/// whatever read produced this, so a caller placing items in order has a field
/// that is correct to use and never needs to reach for the time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertedTime {
    /// The time as text, already formatted for rendering.
    ///
    /// ISO 8601 in UTC, to the second: `2026-09-14T11:01:44Z`. A machine-neutral
    /// rendering rather than a localised one, because localisation needs a
    /// locale and a timezone that only the view knows — and a core that guessed
    /// at either would be making a presentation decision on behalf of a surface
    /// it cannot see.
    pub text: String,
    /// Whether [`text`](Self::text) is the author's value or a clamped stand-in.
    ///
    /// **Carried rather than inferred.** A view cannot recompute this: it would
    /// need the author's raw value, which is exactly what this module declines
    /// to hand out.
    pub clamped: bool,
}

/// Format an author's asserted instant against the reading peer's own clock.
///
/// `now_ms` is the reader's clock, supplied by the caller rather than sampled
/// here. `dialectica-core` reads no clock anywhere, and must not start: a pure
/// crate that sampled `SystemTime` would put a nondeterministic input into a
/// read, and would be untestable at exactly the boundaries that matter.
///
/// Every representable value returns text. Nothing here fails, refuses, or
/// reports an error on account of a value — an op is never refused for its
/// wall-clock, and a read of one that was stored anyway must not be either.
pub fn format_asserted(asserted_ms: u64, now_ms: u64) -> AssertedTime {
    // The clamp targets are the BOUNDS, not the reader's clock. Clamping a
    // far-future value to `now_ms` would render it as "just now", which is a
    // more confident claim than the data supports and is indistinguishable from
    // an honest recent post.
    let (shown, clamped) = if asserted_ms > now_ms.saturating_add(FUTURE_ALLOWANCE_MS) {
        (now_ms.saturating_add(FUTURE_ALLOWANCE_MS), true)
    } else if asserted_ms < FLOOR_MS {
        (FLOOR_MS, true)
    } else {
        (asserted_ms, false)
    };

    AssertedTime {
        text: iso8601_utc(shown),
        clamped,
    }
}

/// Milliseconds since the epoch as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// # Hand-rolled rather than a date crate
///
/// This crate takes dependencies sparingly and the whole requirement is one
/// format with no locale, no timezone database and no parsing. A date crate
/// would be a dependency, a supply-chain surface and a version to track, for a
/// calendar whose rules have not changed since 1582.
///
/// Proleptic Gregorian, UTC, no leap seconds — which is what "milliseconds since
/// the Unix epoch" already means, so there is nothing here to get subtly wrong
/// about a value that decides nothing anyway.
///
/// **Total over every `u64`.** The largest representable value is ~584 million
/// years, and it formats rather than overflowing: every arithmetic step below is
/// a division or a remainder on a widened integer. A panic here would be
/// reachable from a field an author chooses, and a panic aborts the module
/// process (PHASE0-FINDINGS §3).
fn iso8601_utc(ms: u64) -> String {
    let total_secs = ms / 1000;
    let secs_of_day = total_secs % 86_400;
    let days = total_secs / 86_400;

    let (year, month, day) = civil_from_days(days);

    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Days since 1970-01-01 to a civil (year, month, day), proleptic Gregorian.
///
/// Howard Hinnant's `civil_from_days`, which is the standard branch-free
/// formulation and is what `<chrono>` uses. Shifting the era to start in March
/// is what puts the leap day at the end of a year, so the day-of-year
/// arithmetic needs no special case for February.
///
/// `u64` throughout with no subtraction that could go below zero: `days` is
/// unsigned and the era shift adds rather than subtracts, so this is total over
/// the whole input domain. The original signed formulation handles dates before
/// 1970, which this cannot reach — [`FLOOR_MS`] is in 2010 and the caller has
/// already clamped.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    // Shift the epoch to 0000-03-01. 719_468 is the day count from that date to
    // 1970-01-01.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    // Day-of-era to year-of-era, correcting for the 4-, 100- and 400-year rules.
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    // March-based month index, 0 = March.
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    // The year rolls over in January, because the era started in March.
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader's clock well after the floor, for tests that are not about the
    /// reader's clock.
    const NOW: u64 = 1_800_000_000_000; // 2027-01-15T08:00:00Z

    #[test]
    fn a_plausible_time_is_presented_unclamped() {
        let asserted = NOW - 60_000;
        let out = format_asserted(asserted, NOW);
        assert!(!out.clamped, "a time within the allowance must not clamp");
        assert_eq!(out.text, "2027-01-15T07:59:00Z");
    }

    #[test]
    fn a_far_future_time_is_clamped_and_says_so() {
        // Centuries ahead.
        let out = format_asserted(NOW + 300 * 365 * 86_400_000, NOW);
        assert!(out.clamped, "a far-future time must clamp");
        // Clamped to the ALLOWANCE, not to the reader's clock: rendering it as
        // "now" would be a more confident claim than the data supports.
        assert_eq!(out.text, iso8601_utc(NOW + FUTURE_ALLOWANCE_MS));
    }

    #[test]
    fn a_time_just_inside_the_allowance_does_not_clamp() {
        let out = format_asserted(NOW + FUTURE_ALLOWANCE_MS, NOW);
        assert!(!out.clamped, "the allowance boundary is inclusive");
    }

    #[test]
    fn a_time_one_millisecond_past_the_allowance_clamps() {
        // The boundary from the other side. Together with the test above this
        // pins WHERE the edge is, which a test using a value far from it cannot.
        let out = format_asserted(NOW + FUTURE_ALLOWANCE_MS + 1, NOW);
        assert!(out.clamped);
    }

    #[test]
    fn a_zero_time_is_clamped_to_the_floor() {
        let out = format_asserted(0, NOW);
        assert!(out.clamped, "a zero wall-clock is below the floor");
        assert_eq!(out.text, "2010-01-01T00:00:00Z");
    }

    #[test]
    fn a_time_below_the_floor_is_clamped() {
        let out = format_asserted(FLOOR_MS - 1, NOW);
        assert!(out.clamped);
        assert_eq!(out.text, "2010-01-01T00:00:00Z");
    }

    #[test]
    fn the_floor_itself_does_not_clamp() {
        let out = format_asserted(FLOOR_MS, NOW);
        assert!(!out.clamped, "the floor is inclusive");
        assert_eq!(out.text, "2010-01-01T00:00:00Z");
    }

    #[test]
    fn the_maximum_representable_instant_formats_rather_than_panicking() {
        // Reachable from a field an author chooses, and a panic aborts the
        // module process. The value clamps, so what this really pins is that
        // NOTHING on the path overflows — including the clamp arithmetic, where
        // `now_ms + FUTURE_ALLOWANCE_MS` would wrap without `saturating_add`.
        let out = format_asserted(u64::MAX, u64::MAX);
        assert!(
            !out.clamped,
            "u64::MAX against a u64::MAX reader is in range"
        );
        assert!(out.text.ends_with('Z'));
    }

    #[test]
    fn a_maximal_reader_clock_does_not_overflow_the_allowance() {
        // `now_ms.saturating_add(FUTURE_ALLOWANCE_MS)` is the line under test:
        // a plain `+` panics here in a debug build.
        let out = format_asserted(0, u64::MAX);
        assert!(out.clamped, "zero is still below the floor");
    }

    #[test]
    fn the_epoch_formats_correctly() {
        // Not reachable through `format_asserted`, which clamps it — this pins
        // the formatter itself at the bottom of its own domain.
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_leap_day_formats_as_the_twenty_ninth_of_february() {
        // 2024-02-29T12:34:56Z. The case the March-based era arithmetic exists
        // to get right, and the one a naive formatter gets wrong.
        //
        // Derived rather than looked up, so the arithmetic is checkable here:
        // 2024-02-29T00:00:00Z is 1_709_164_800_000 ms, and 12:34:56 is
        // (12 * 3600 + 34 * 60 + 56) = 45_296 s = 45_296_000 ms.
        assert_eq!(iso8601_utc(1_709_164_800_000), "2024-02-29T00:00:00Z");
        assert_eq!(iso8601_utc(1_709_210_096_000), "2024-02-29T12:34:56Z");
        // And the day before is the 28th, so the leap day was inserted rather
        // than the month simply running long.
        assert_eq!(
            iso8601_utc(1_709_164_800_000 - 86_400_000),
            "2024-02-28T00:00:00Z"
        );
    }

    #[test]
    fn a_century_non_leap_year_formats_correctly() {
        // 1900 is NOT a leap year (divisible by 100, not by 400), so
        // 1900-03-01 follows 1900-02-28. Unreachable through the public
        // function — it is below the floor — and it is the case that
        // distinguishes the 100-year rule from the 4-year one.
        //
        // 1900-03-01T00:00:00Z is -2_203_891_200 seconds from the epoch, so it
        // cannot be expressed in this `u64` formatter. 2100 is the same rule in
        // range: 2100-03-01T00:00:00Z = 4_107_542_400_000 ms.
        assert_eq!(iso8601_utc(4_107_542_400_000), "2100-03-01T00:00:00Z");
        // The day before is the 28th, not the 29th.
        assert_eq!(
            iso8601_utc(4_107_542_400_000 - 86_400_000),
            "2100-02-28T00:00:00Z"
        );
    }

    #[test]
    fn a_four_hundred_year_leap_year_formats_correctly() {
        // 2000 IS a leap year (divisible by 400), which is the rule the 100-year
        // correction would otherwise swallow.
        assert_eq!(iso8601_utc(951_782_400_000), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn sub_second_milliseconds_are_truncated_not_rounded() {
        // 999ms past a second renders as that second, not the next one. Stated
        // because rounding would make a time render as a moment that had not
        // arrived, which is the direction this whole module is careful about.
        assert_eq!(
            iso8601_utc(1_800_000_000_999),
            iso8601_utc(1_800_000_000_000)
        );
    }

    #[test]
    fn the_constants_are_pinned_to_known_answers() {
        // `cargo mutants` does not mutate a `const`, so a drifted value here
        // would be invisible to it.
        assert_eq!(FUTURE_ALLOWANCE_MS, 86_400_000, "the allowance is 24 hours");
        assert_eq!(FLOOR_MS, 1_262_304_000_000);
        // Self-checking: the floor really is the date the comment claims.
        assert_eq!(iso8601_utc(FLOOR_MS), "2010-01-01T00:00:00Z");
    }
}
