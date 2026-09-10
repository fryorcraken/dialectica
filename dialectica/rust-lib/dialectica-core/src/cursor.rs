//! A bounds-checked read head, shared by every decoder in this crate.
//!
//! # Why this is a module and not a local struct
//!
//! Every canonical encoding dialectica decodes arrives from a peer, so every
//! decoder faces the same hostile input and needs the same two questions asked
//! in one place: "did the input end?" and "is anything left over?". Hand-rolled
//! slicing at each field is how a decoder acquires a panicking index, and a
//! panic here is reachable from inbound data — PHASE0-FINDINGS §3 measured what
//! that costs: the module process **aborts**, the caller waits out a 20s
//! timeout, and every later call reports `MODULE_NOT_LOADED`.
//!
//! `stoa.rs` had this first, as a private struct returning `GenesisError`. The
//! op decoder needs the identical read head and a different error type, so the
//! choice was to copy it or to generalise it. Copying a bounds check is how the
//! second copy acquires the off-by-one the first one fixed.
//!
//! # The error type is deliberately not generic
//!
//! [`OutOfBounds`] says only *which* of the two things went wrong; it carries no
//! message and names no format. Each decoder maps it into its own error enum,
//! because "truncated" means something different in each — and a decoder that
//! surfaced this type directly would be leaking a shared internal into its
//! public contract.

/// What a read past the end, or a trailing byte, is before a decoder names it.
///
/// Two variants because they are genuinely different mistakes: the input ran
/// out mid-field, or a complete record was followed by bytes that are not part
/// of it. A decoder that collapsed them would tell a reader "malformed" when
/// the truth is "there is a second record stuck to the end of this one".
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutOfBounds {
    /// The input ended before the requested bytes did.
    Truncated,
    /// Bytes remained after the record was fully read.
    Trailing,
}

/// A read head over a byte string, which never panics and never wraps.
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, at: 0 }
    }

    /// The next `n` bytes, or [`OutOfBounds::Truncated`].
    ///
    /// `checked_add` is load-bearing rather than defensive habit: `at + n` on a
    /// hostile length prefix can overflow and wrap to a small value that passes
    /// a naive bounds check, handing back a slice the caller never asked for.
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], OutOfBounds> {
        let end = self.at.checked_add(n).ok_or(OutOfBounds::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(OutOfBounds::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    /// Exactly `N` bytes as a fixed-size array.
    ///
    /// The array form exists so a caller reading a fixed-width field cannot
    /// write the length twice — once in the `take` and once in a
    /// `copy_from_slice` into a buffer of some other size. The `try_into`
    /// cannot fail, because `take` returned exactly `N` bytes.
    pub(crate) fn take_array<const N: usize>(&mut self) -> Result<[u8; N], OutOfBounds> {
        let slice = self.take(N)?;
        Ok(slice.try_into().expect("take returned exactly N bytes"))
    }

    /// A `u32` big-endian length prefix, widened to `usize`.
    ///
    /// Big-endian because it is the byte order the rest of this crate's wire
    /// formats use, and a format that mixed the two would be a standing trap.
    pub(crate) fn take_length(&mut self) -> Result<usize, OutOfBounds> {
        Ok(u32::from_be_bytes(self.take_array::<4>()?) as usize)
    }

    /// Consume the cursor, requiring the input to be exhausted.
    pub(crate) fn finish(self) -> Result<(), OutOfBounds> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(OutOfBounds::Trailing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_returns_the_requested_bytes_and_advances() {
        let mut c = Cursor::new(b"abcdef");
        assert_eq!(c.take(2).unwrap(), b"ab");
        assert_eq!(c.take(3).unwrap(), b"cde");
        assert_eq!(c.take(1).unwrap(), b"f");
        assert_eq!(c.finish(), Ok(()));
    }

    #[test]
    fn take_past_the_end_is_truncated_rather_than_a_panic() {
        // The whole reason this type exists. A slice index here would abort the
        // module process (PHASE0-FINDINGS §3), and the input is peer-supplied.
        let mut c = Cursor::new(b"ab");
        assert_eq!(c.take(3), Err(OutOfBounds::Truncated));
    }

    #[test]
    fn a_failed_take_does_not_advance_the_head() {
        // Otherwise a decoder that maps one error and continues would read from
        // a position the failed call moved, which is how a rejected field
        // silently shifts every field after it.
        let mut c = Cursor::new(b"ab");
        assert_eq!(c.take(3), Err(OutOfBounds::Truncated));
        assert_eq!(c.take(2).unwrap(), b"ab");
    }

    #[test]
    fn a_length_that_would_overflow_is_refused_rather_than_wrapping() {
        // `at + n` with a hostile prefix. Without `checked_add` this wraps to a
        // small end offset, `get` succeeds, and the decoder is handed a slice
        // that has nothing to do with the length it asked for.
        let mut c = Cursor::new(b"abcdef");
        assert_eq!(c.take(2).unwrap(), b"ab");
        assert_eq!(c.take(usize::MAX), Err(OutOfBounds::Truncated));
    }

    #[test]
    fn finish_refuses_leftover_bytes() {
        let mut c = Cursor::new(b"abc");
        assert_eq!(c.take(2).unwrap(), b"ab");
        assert_eq!(c.finish(), Err(OutOfBounds::Trailing));
    }

    #[test]
    fn finish_on_an_empty_input_is_ok() {
        assert_eq!(Cursor::new(b"").finish(), Ok(()));
    }

    #[test]
    fn take_of_zero_bytes_is_allowed_and_moves_nothing() {
        // An empty variable-length field is legitimate — a genesis record with
        // an empty title encodes a zero prefix — so this must not be an error.
        let mut c = Cursor::new(b"ab");
        assert_eq!(c.take(0).unwrap(), b"");
        assert_eq!(c.take(2).unwrap(), b"ab");
    }

    #[test]
    fn take_array_reads_a_fixed_width_field() {
        let mut c = Cursor::new(b"\x01\x02\x03\x04\x05");
        assert_eq!(c.take_array::<2>().unwrap(), [1, 2]);
        assert_eq!(c.take_array::<3>().unwrap(), [3, 4, 5]);
    }

    #[test]
    fn take_array_past_the_end_is_truncated() {
        let mut c = Cursor::new(b"\x01");
        assert_eq!(c.take_array::<4>(), Err(OutOfBounds::Truncated));
    }

    #[test]
    fn take_length_reads_a_big_endian_u32() {
        // Pinned as big-endian on purpose: the byte order is part of every wire
        // format in this crate, and a decoder that read it the other way would
        // accept a 16-million-byte field where a 1-byte one was meant.
        let mut c = Cursor::new(b"\x00\x00\x01\x00");
        assert_eq!(c.take_length().unwrap(), 256);
    }

    #[test]
    fn take_length_on_a_short_input_is_truncated() {
        let mut c = Cursor::new(b"\x00\x00");
        assert_eq!(c.take_length(), Err(OutOfBounds::Truncated));
    }
}
