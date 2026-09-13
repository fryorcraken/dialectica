//! Generated display names: a readable name for a key, computed and never typed.
//!
//! # What this is for
//!
//! A 32-byte address is unreadable, and unreadable pseudonymity is not
//! pseudonymity in any useful sense (PLAN.md §5.2.1) — a reader who cannot tell
//! two participants apart at a glance cannot follow an argument between them,
//! which is the one thing this forum is named for. So every identity renders
//! under three drawn words: *measured aporia of lampsacus*.
//!
//! # Derived from the PUBLIC KEY, not the address, and the difference matters
//!
//! `H(NAME_PREFIX || public_key)`. An address is `SHA256(AUTHOR_ADDRESS_PREFIX
//! || 0x01 || public_key)` — a hash of a *record* kept extensible against a
//! future key log (§5.1, §5.3). Deriving a name from the key means the name
//! tracks the key that signs, which is what a reader is actually being shown,
//! and keeps "does a name change when a key does?" a question that arrives
//! loudly rather than one pre-answered by which value happened to be hashed.
//!
//! **The consequence is the reason [`crate::feed::FeedRow`] carries a name at
//! all.** A reply reporting an author reports an *address*, from which no key is
//! recoverable — so a view holding one cannot compute the name. Core holds the
//! key, because the signed op carries it, so core renders the name. `feed.rs`
//! carried a doc comment asserting the opposite for as long as the field was
//! missing, which is why the field was missing.
//!
//! # Determinism is the whole contract
//!
//! The same key gives the same name on every peer, at every time, with no
//! lookup, no stored state and nothing varying between runs participating. A
//! name that two peers could compute differently is one identity rendering as
//! two people, which users report as impersonation.
//!
//! **A name is therefore not published and cannot be.** A published name is a
//! value two peers could disagree about, and a value travelling as data is one a
//! relay could strip or forge. It is recomputed wherever it is shown.
//!
//! # What a name is NOT
//!
//! **Not unique, not an identifier, and never numbered.** There is no registry
//! and no authority to hold a namespace: a Stoa has no membership list, peers
//! join and leave without announcing it, and two peers can each believe a name
//! is free. Numbering a collision would be worse — appending `#2` requires
//! agreeing which identity was second, which is arrival order, a per-peer fact,
//! so two peers would number the same pair oppositely and each be certain the
//! other was looking at the impostor.
//!
//! **Not a credential.** Anyone willing to press a regeneration button reaches
//! any name they like, so no argument of the form "an attacker would have to
//! grind for that" is available. What an attacker gets is a lookalike *name* on
//! a different *address*; they cannot forge the address and cannot forge a
//! signature, so nothing they publish is attributable to the identity they
//! imitate. The attack is purely social and the address is what defeats it.
//!
//! **No method accepts one.** Not a lookup, not a moderation target, not a vote
//! target. The address is the identity.

use crate::identity::{KeyError, PublicKey};
use sha2::{Digest, Sha256};

mod adjectives;
mod denylist;
mod nouns;
mod places;

pub use adjectives::ADJECTIVES;
pub use denylist::TRUE_ATTRIBUTION_PAIRS;
pub use nouns::NOUNS;
pub use places::PLACES;

/// Domain separation for a display name.
///
/// A fixed 32 bytes in the same padded style as every prefix in
/// [`crate::identity`], and distinct from all of them. The padding is not
/// decoration: a variable-length prefix concatenated with variable-length data
/// is the classic way to make two different inputs hash the same, and a fixed
/// width removes the question rather than arguing about it.
///
/// **The version is in the string and is load-bearing.** The spec makes any
/// change to the scheme or the wordlists a new version rather than an edit,
/// because the derivation maps digest bytes to list *indices*: removing one word
/// reindexes the list and every identity that drew at or after it renders
/// differently — on peers that have updated and not on peers that have not. The
/// same key then renders as two different people depending on who is looking.
/// A version bump makes the old and new schemes two distinct derivations rather
/// than two peers' answers to one question.
///
/// **Separation from the address prefixes is what makes the name and the mark
/// independent**, and this is the mechanism — not any allocation of bytes. The
/// name hashes the key under this prefix; the mark reads the *address*, a
/// different digest of the same key. An attacker grinding keys for a target's
/// name gets an unrelated address and mark each time, and grinding for the mark
/// gets an unrelated name, so the two must be landed together and the costs
/// multiply rather than add. Two documents independently invented a
/// shared-digest story in which ranges of address bytes were "reserved" for the
/// name; no such mechanism exists, and the independence is real without it.
const NAME_PREFIX: &[u8; 32] = b"/dialectica/1/Name/Display\0\0\0\0\0\0";

/// The literal text between the noun and the place.
///
/// **Fixed text, emitted unconditionally, reading no hash bytes.** It is not a
/// slot — the natural reading of "three words plus a connector" is that there
/// are four, and there are three. It carries no entropy and never varies with
/// the key, which is exactly what makes it the one part of a name a cramped
/// caller may drop: *measured aporia lampsacus* has identical information
/// content and misleads no reader about who published something.
pub const CONNECTOR: &str = "of";

/// The first byte past the name's slice of the digest.
///
/// Six bytes for the first draw of three 16-bit slots, six more as the one
/// complete redraw the denylist requires. **Beyond this the derivation fails
/// rather than reading on**: reading without bound is what lets two
/// implementations disagree about how far to read and so produce different names
/// for one key, which is the failure the whole scheme exists to prevent. A
/// bounded slice is also what makes the derivation checkable against a fixed
/// expected value at all.
///
/// The number of bytes a name actually consumes is **data-dependent** — the
/// reserve is touched only on a refusal — and this bound is what keeps it
/// finite.
const NAME_DIGEST_BOUND: usize = 12;

/// A generated display name: three drawn words.
///
/// **Three fields and not a `String`**, so that the shape is structural. There
/// is no way to build one of these with two words or four, and the connector
/// does not exist in the data at all — it appears only in [`DisplayName::render`],
/// which is the one place it is emitted. The spec's "the connector is not a
/// slot" is therefore true by construction rather than by a test at every call
/// site.
///
/// The three are `&'static str` because they are entries of `'static` wordlists;
/// nothing is allocated to derive a name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DisplayName {
    pub adjective: &'static str,
    pub noun: &'static str,
    pub place: &'static str,
}

impl DisplayName {
    /// The three drawn words, in slot order and without the connector.
    ///
    /// For a caller too cramped to render `of`. The connector is the only part
    /// of a name that may be dropped, and it is droppable precisely because it
    /// is the only part that is not derived — so this hands over the whole of
    /// what the key selected, and nothing is elided.
    pub fn words(&self) -> [&'static str; 3] {
        [self.adjective, self.noun, self.place]
    }

    /// The rendered name: *measured aporia of lampsacus*.
    ///
    /// Every drawn word appears complete. Nothing here truncates, abbreviates or
    /// elides — the three drawn words are the whole of the name's space, and
    /// eliding one removes a slot's worth of distinguishing content.
    pub fn render(&self) -> String {
        format!(
            "{} {} {} {}",
            self.adjective, self.noun, CONNECTOR, self.place
        )
    }
}

impl std::fmt::Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Why a name could not be derived.
///
/// **There is no arm meaning "unknown key".** A name is a function of the key
/// alone, consulting no moderator set, no genesis record and no stored state, so
/// there is nothing to look up and nothing that could fail to be found. A name
/// is derivable for any well-formed public key including one belonging to no
/// identity this peer has seen.
///
/// Both arms below are about the *input* or the *scheme*, never about
/// recognition.
#[derive(Debug, PartialEq, Eq)]
pub enum NameError {
    /// The bytes given are not a well-formed public key.
    ///
    /// A public key on this path arrives inside an inbound op and is
    /// attacker-controlled. This is an error rather than a panic because a panic
    /// here aborts the module process (PHASE0-FINDINGS §3): the caller learns
    /// only that its call timed out and every later call reports the module as
    /// not loaded, which makes a derivation that panics on a short slice a
    /// remotely triggerable denial of service.
    NotAValidPublicKey(KeyError),
    /// Both the first draw and the one reserved redraw landed on refused
    /// combinations, so the budget is spent.
    ///
    /// Arrives about once in 1.2 million identities: a first draw is refused
    /// about once in 1,090, and this needs two consecutive refusals. **Failing
    /// is correct and reading on is not** — reading past the bound makes the
    /// name's consumption unbounded, and two implementations disagreeing about
    /// how far to read produce different names for one key.
    ReserveExhausted,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::NotAValidPublicKey(e) => {
                write!(f, "cannot derive a display name: {e}")
            }
            NameError::ReserveExhausted => write!(
                f,
                "cannot derive a display name: the denylist reserve is exhausted"
            ),
        }
    }
}

/// The display name for a public key.
///
/// **Total for every well-formed key except the exhausted-reserve case**, which
/// is a property of the scheme rather than of the key being unrecognised. Takes
/// a parsed [`PublicKey`], so malformed bytes cannot reach here at all — see
/// [`display_name_from_bytes`] for the entry point that parses.
pub fn display_name(key: &PublicKey) -> Result<DisplayName, NameError> {
    name_from_digest(&name_digest(key))
}

/// The display name for raw key bytes, parsing first.
///
/// **One call rather than two steps a caller can get wrong.** The same reasoning
/// as [`crate::identity::verify_authored_op`]: splitting a parse from the
/// operation that needs it is how the parse goes missing, so the refusal is one
/// function.
///
/// A failure is **never** reported as a name. There is no placeholder, no name
/// for "unknown", and no name derived from truncated or padded input — each
/// would render as an ordinary participant, which is a name attributable to
/// nobody presented as one attributable to somebody.
pub fn display_name_from_bytes(bytes: &[u8]) -> Result<DisplayName, NameError> {
    let key = PublicKey::from_bytes(bytes).map_err(NameError::NotAValidPublicKey)?;
    display_name(&key)
}

/// The name's digest for a key: `SHA256(NAME_PREFIX || public_key)`.
///
/// `pub` so that a test can compare it byte for byte against the same key's
/// address and show the two are different digests — which is the whole of why
/// no byte allocation between the name and the mark is required or possible.
pub fn name_digest(key: &PublicKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(NAME_PREFIX);
    hasher.update(key.to_bytes());
    hasher.finalize().into()
}

/// Turn a digest into three words.
///
/// **Public, and separately callable from hashing a key, because the spec makes
/// that a testability obligation rather than a convenience.** A refused first
/// draw, an exhausted reserve and the bound are all reachable through a chosen
/// digest; through a chosen *key* they are reachable only by grinding for one.
/// A scheme whose failure paths can only be reached by grinding is a scheme
/// whose failure paths no test covers.
///
/// # The byte budget
///
/// | bytes | slot |
/// |---|---|
/// | `0..2` | adjective, first draw |
/// | `2..4` | noun, first draw |
/// | `4..6` | place, first draw |
/// | `6..8` | adjective, redraw |
/// | `8..10` | noun, redraw |
/// | `10..12` | place, redraw |
///
/// Bytes `12..32` are **never read**, which is what makes the derivation
/// checkable against a fixed value and what keeps two implementations from
/// disagreeing about how far to read.
///
/// Each slot takes its index from bytes no other slot reads, so the three words
/// are independent draws rather than three views of the same bits.
pub fn name_from_digest(digest: &[u8; 32]) -> Result<DisplayName, NameError> {
    // The first draw. If it is not refused, the reserve is never consulted —
    // which a test pins by varying bytes 6..11 alone and expecting no change.
    if let Some(name) = draw_at(digest, 0) {
        return Ok(name);
    }
    // Refused, so REDRAW ALL THREE SLOTS from the reserve, not merely the
    // offending pair. A refused pair is refused for the combination, so changing
    // one half can land on a second refused pair and the loop's termination
    // becomes a property of the denylist's shape rather than of the byte budget.
    // Whole-name redraw keeps termination arithmetic: there is exactly one
    // retry, and it either lands or fails.
    if let Some(name) = draw_at(digest, 6) {
        return Ok(name);
    }
    // The bound. Reading on from here is the failure this bound exists to
    // prevent, so this is an error and not a third draw.
    debug_assert_eq!(NAME_DIGEST_BOUND, 12, "the budget is two draws of six");
    Err(NameError::ReserveExhausted)
}

/// One draw of all three slots from six bytes at `offset`, or `None` if the
/// noun–place pair it selects is refused.
///
/// Separate from [`name_from_digest`] because the first draw and the redraw are
/// the identical operation at two offsets — writing it twice is how the two
/// quietly stop matching, and a redraw that differed from a first draw would
/// break determinism in the one case hardest to notice.
///
/// # Why a 16-bit draw reduced by `%` is exactly uniform
///
/// `65,536 / 8,192 = 8` and `65,536 / 1,024 = 64`, both whole numbers, so every
/// index of every list is produced by the same number of 16-bit values as every
/// other. There is no modulo bias to trade off and no word is favoured.
///
/// **This is what makes the power-of-two list sizes load-bearing rather than
/// incidental.** A list of 1,000 would introduce a real if tiny bias; a list of
/// a different power of two would reindex every draw. The spec forbids changing
/// a size without a version bump for exactly this reason.
///
/// Big-endian so the bytes read in the order a hand-computed test vector is
/// written.
fn draw_at(digest: &[u8; 32], offset: usize) -> Option<DisplayName> {
    let word = |i: usize| u16::from_be_bytes([digest[offset + i], digest[offset + i + 1]]);

    let adjective_index = word(0) % ADJECTIVES.len() as u16;
    let noun_index = word(2) % NOUNS.len() as u16;
    let place_index = word(4) % PLACES.len() as u16;

    if is_refused(noun_index, place_index) {
        return None;
    }

    Some(DisplayName {
        adjective: ADJECTIVES[adjective_index as usize],
        noun: NOUNS[noun_index as usize],
        place: PLACES[place_index as usize],
    })
}

/// Whether this noun–place pair spells a real figure's canonical name.
///
/// **The refusal is on the PAIR and never on either word.** The adjective is
/// irrelevant to this family, and both halves stay in their lists — so
/// *straton of abdera* and *measured aporia of lampsacus* both draw normally
/// while *straton of lampsacus* does not. A word-level exclusion would cost two
/// entries per figure and buy nothing.
///
/// # Why this family is mandatory rather than discretionary
///
/// The *X of Y* shape can produce exactly how a historical figure is
/// conventionally cited — *straton of lampsacus* is how Straton of Lampsacus is
/// actually referred to — so a user drawing that pair has every post they make
/// signed with a real person's full canonical identifier. That is a structural
/// property of the shape and not the separate exclusion of a handful of figures.
///
/// The arithmetic is what makes it a requirement rather than a nicety: roughly
/// 800 of the 1,024 nouns are named Greeks, each with about 1.2 canonically
/// associated places, so about 960 pairs against `1024 * 1024` = 1,048,576 is
/// about 0.092% of draws — roughly 4.6 identities in every 5,000. A handful per
/// Stoa arriving steadily, not a corner case.
///
/// `binary_search` over a sorted array rather than a `HashSet`: a hash set's
/// iteration order is unstable, which invites a future change that iterates it
/// into something observable, and a sorted array's sortedness is a property a
/// test can assert — which is exactly the property the search needs.
fn is_refused(noun_index: u16, place_index: u16) -> bool {
    TRUE_ATTRIBUTION_PAIRS
        .binary_search(&(noun_index, place_index))
        .is_ok()
}

/// Written-down names, shared with the modules whose rows must carry them.
///
/// **These are the only values in the crate that cannot be re-derived from the
/// implementation**, and that is their entire purpose. Change one byte of
/// `NAME_PREFIX`, one entry of a list, the order of two entries, which bytes a
/// slot reads, or the connector, and every peer's names change together with no
/// error anywhere — each peer stays internally consistent while agreeing with
/// nobody. A check that asks the implementation what it produced and agrees with
/// the answer cannot see that. These were produced independently, by
/// `tmp/pin.rs`, which reads the wordlists from the text files and does the
/// index arithmetic itself rather than calling [`name_from_digest`].
///
/// `feed.rs` uses them for the same reason: a row built by calling the
/// derivation and then checked by calling the derivation agrees with itself
/// whatever either does, and would pass on a row that named the wrong author's
/// key.
///
/// **If one of these fails, do not update it to match.** Work out what changed
/// and whether the network can survive it.
#[cfg(test)]
pub mod tests_support {
    pub const PINNED_NAME_FOR_KEY_4: &str = "quipful ismene of korykos";
    pub const PINNED_NAME_FOR_KEY_5: &str = "periculous kreios of narthakion";

    /// Two distinct secret-key seeds whose keys derive the SAME display name.
    ///
    /// **Found by search, not constructed by stubbing the derivation**, so the
    /// collision is a real property of the shipped scheme and wordlists rather
    /// than of a test double. The search walked seeds with a counter in the
    /// first four bytes and indexed by rendered name until one repeated; at a
    /// space of 2^33 a repeat arrives after roughly 2^16.5 keys, and this pair
    /// turned up well inside three million.
    ///
    /// They are written down rather than re-searched at test time because a
    /// search in the suite would be slow and, worse, would agree with whatever
    /// the derivation did — the point of a collision fixture is that it was
    /// fixed BEFORE the code ran.
    ///
    /// **If a test using these fails, the pair has stopped colliding**, which
    /// means the derivation or a wordlist changed. That is a scheme change; do
    /// not go and find a new pair to paper over it.
    pub const COLLIDING_SEED_A: [u8; 32] = {
        let mut s = [0u8; 32];
        s[1] = 0x00;
        s[2] = 0xc6;
        s[3] = 0x13;
        s
    };
    pub const COLLIDING_SEED_B: [u8; 32] = {
        let mut s = [0u8; 32];
        s[1] = 0x00;
        s[2] = 0xff;
        s[3] = 0xb1;
        s
    };
    /// The name both of the seeds above derive.
    pub const COLLIDING_NAME: &str = "plurative archilochos of kyrrhos";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    // ─── The pins: the only tests that can see a silent consensus change ───

    #[test]
    fn the_name_scheme_is_pinned_to_known_answers() {
        // EVERY constant this scheme rests on is consensus-critical: change one
        // byte of NAME_PREFIX, one entry of a list, the order of two entries, or
        // which bytes a slot reads, and this peer's names stop matching every
        // other peer's — with no error anywhere, because each peer remains
        // internally consistent.
        //
        // Every other test in this file is self-consistent and would pass
        // unchanged if someone edited a wordlist. This one would not. That is
        // its whole job, and it is why the expected values are WRITTEN DOWN
        // rather than read back from the implementation.
        //
        // The written-down name is also what pins the WORD COUNT. Counting the
        // words of whatever the implementation returned agrees with an
        // implementation that joins two words, or four, or three in the wrong
        // order — the count and the thing counted come from one source. A
        // written-down name is produced independently, so a change to the number
        // of words, the slot order or the connector fails it.
        //
        // If this fails, do NOT update the expected values to match. Work out
        // what changed and whether the network can survive it.
        let name = display_name(&a_key(7).public_key()).expect("a well-formed key derives a name");
        assert_eq!(
            name.render(),
            PINNED_NAME_FOR_KEY_7,
            "the name derivation changed"
        );

        // And the digest itself, so a wordlist change and a PREFIX change fail
        // separately rather than both arriving as one unexplained string.
        assert_eq!(
            hex::encode(name_digest(&a_key(7).public_key())),
            PINNED_DIGEST_FOR_KEY_7,
            "the name digest changed"
        );
    }

    /// The expected name for the key whose seed is 32 bytes of 0x07.
    ///
    /// Derived from `PINNED_DIGEST_FOR_KEY_7` by hand, so that this is an
    /// independent statement and not the implementation agreeing with itself:
    /// see `the_pinned_name_is_derivable_by_hand_from_the_pinned_digest`, which
    /// does the index arithmetic in the test rather than by calling `draw_at`.
    const PINNED_NAME_FOR_KEY_7: &str = "expatiative karpos of pythion";
    const PINNED_DIGEST_FOR_KEY_7: &str =
        "09c2b1c9373894cc7c47ee573ca06a8d9950a834d0dbb69a35ffcbcb0cdf7e74";

    #[test]
    fn the_pinned_name_is_derivable_by_hand_from_the_pinned_digest() {
        // The pin above is only independent if it can be reached WITHOUT the
        // function under test. This does the index arithmetic here, from the
        // digest's bytes, and indexes the lists directly — so it fails if
        // `draw_at` reads different bytes, reduces differently, orders the slots
        // differently, or emits a different connector.
        //
        // `PINNED_NAME_FOR_KEY_7` and `PINNED_DIGEST_FOR_KEY_7` were produced by
        // `tmp/pin.rs`, which reads the wordlists from the TEXT FILES the
        // modules were generated from. So three independent routes — that
        // program, this arithmetic, and `display_name` — must agree.
        let digest = hex::decode(PINNED_DIGEST_FOR_KEY_7).expect("the pin is hex");

        // Bytes 0..2, 2..4, 4..6, big-endian, reduced into each list.
        let adjective_index = u16::from_be_bytes([digest[0], digest[1]]) % 8_192;
        let noun_index = u16::from_be_bytes([digest[2], digest[3]]) % 1_024;
        let place_index = u16::from_be_bytes([digest[4], digest[5]]) % 1_024;

        // Written out rather than computed from the constant, so this is a
        // statement about WHICH entries rather than a restatement of the
        // arithmetic above.
        assert_eq!(adjective_index, 2_498);
        assert_eq!(noun_index, 457);
        assert_eq!(place_index, 824);

        // The first draw must not be refused, or the pinned name would be the
        // REDRAW and this arithmetic would be the wrong story about it.
        assert!(
            !is_refused(noun_index, place_index),
            "the pinned key's first draw must be permitted, or the pin is \
             pinning the redraw path by accident"
        );

        assert_eq!(
            format!(
                "{} {} {CONNECTOR} {}",
                ADJECTIVES[adjective_index as usize],
                NOUNS[noun_index as usize],
                PLACES[place_index as usize]
            ),
            PINNED_NAME_FOR_KEY_7
        );
    }

    #[test]
    fn the_redraw_path_is_pinned_to_a_written_down_name() {
        // The spec requires the redraw path be pinned and not only the common
        // one, because a scheme that redrew wrongly would pass every pin above.
        //
        // Reached through a CONSTRUCTED DIGEST rather than a key: a refused first
        // draw is reachable through a chosen key only by grinding for one.
        //
        // The digest's first six bytes select `TRUE_ATTRIBUTION_PAIRS[0]`, which
        // is `(11, 436)` = `agatharchides of knidos` — a real figure's canonical
        // name, which is exactly what the family refuses. The reserve selects
        // index 7 of each list.
        let refused = (11u16, 436u16);
        assert_eq!(
            TRUE_ATTRIBUTION_PAIRS[0], refused,
            "this fixture names the first denylist pair explicitly, so a change \
             to the denylist's head fails here rather than silently retargeting"
        );
        assert_eq!(
            format!("{} {CONNECTOR} {}", NOUNS[11], PLACES[436]),
            "agatharchides of knidos",
            "the refused pair is the one this test claims it is"
        );

        let digest = digest_drawing((3, refused.0, refused.1), (7, 7, 7));
        let name = name_from_digest(&digest).expect("the reserve draw is permitted");

        // WRITTEN DOWN, from `tmp/pin.rs`'s reading of the text files, not read
        // back from `name_from_digest`.
        assert_eq!(name.render(), PINNED_REDRAW_NAME);

        // And it is the RESERVE's draw, not the refused one with a slot swapped.
        assert_ne!(name.noun, NOUNS[refused.0 as usize]);
        assert_ne!(name.place, PLACES[refused.1 as usize]);
        assert_ne!(name.adjective, ADJECTIVES[3]);
    }

    /// The name the reserve bytes `(7, 7, 7)` select: index 7 of each list.
    const PINNED_REDRAW_NAME: &str = "aberrative aedon of acherousia";

    /// SHA-256 over every entry of each list, in order, each followed by `\n`.
    ///
    /// **Produced by `sha256sum` over the source text files, never by hashing
    /// the arrays.** The three commands, which a reviewer can re-run:
    ///
    /// ```text
    /// sha256sum tmp/adj-final-sorted.txt      # adjectives
    /// sha256sum tmp/nouns-normalized.txt      # nouns, blank first line dropped
    /// sha256sum tmp/places-final-sorted.txt   # places
    /// ```
    ///
    /// `tmp/nouns-normalized.txt` is `grep . tmp/nouns-final-sorted.txt`; that
    /// file carries a leading blank line, which `tmp/gen.rs` and `tmp/pin.rs`
    /// both filter, so dropping it is what makes the file and the array the same
    /// sequence. Each file ends in a trailing newline, so the concatenation
    /// `entry + "\n"` reproduces the file's bytes exactly.
    const PINNED_ADJECTIVES_SHA256: &str =
        "8c998df498623340f706c3b8a3cc8be42c1126cf2f98d17efc14695fe94de8b2";
    const PINNED_NOUNS_SHA256: &str =
        "9c082a491ebba9e3459b62d717ea897de87d0732c94704470ee6557d93230c79";
    const PINNED_PLACES_SHA256: &str =
        "bed083891cd6262152d0e1371709dc9f9d14f4704b1b74b3684ab8bea9f815f1";

    #[test]
    fn every_wordlist_is_pinned_entry_by_entry_and_in_order() {
        // **A REORDERING is the consensus change the three name pins cannot
        // see**, and this test exists because that gap was measured rather than
        // supposed: exchanging `araden` and `araithyrea` — places 100 and 101 —
        // changes no entry's spelling, no list's size and no entry's uniqueness,
        // so the character sweep, the dedup sweep and the size assertions all
        // pass, and neither name pin draws either index. The full suite ran
        // green with the two swapped.
        //
        // That is exactly the silent divergence the spec's versioning
        // requirement is about: the derivation maps digest bytes to list
        // INDICES, so moving one entry renames every identity drawing at or
        // after it — on peers that have updated and not on peers that have not,
        // with no error anywhere, each peer internally consistent and agreeing
        // with nobody.
        //
        // A name pin covers the handful of indices it happens to draw. A hash
        // over the whole list covers all 8,192 and all 1,024, which is the only
        // shape that matches the requirement: ANY reorder, ANY substituted
        // entry, ANY added or removed one.
        //
        // The expected values were produced by `sha256sum` over the text files
        // the arrays were generated from — never by hashing the arrays and
        // writing down the answer, which would be the implementation agreeing
        // with itself. The doc comment above carries the three commands.
        //
        // If this fails, do NOT update the expected values to match. A wordlist
        // change is a scheme change: it needs a new version in `NAME_PREFIX`,
        // not a new constant here.
        for (list, which, expected) in [
            (ADJECTIVES, "adjectives", PINNED_ADJECTIVES_SHA256),
            (NOUNS, "nouns", PINNED_NOUNS_SHA256),
            (PLACES, "places", PINNED_PLACES_SHA256),
        ] {
            let mut hasher = Sha256::new();
            for entry in list {
                hasher.update(entry.as_bytes());
                hasher.update(b"\n");
            }
            let digest: [u8; 32] = hasher.finalize().into();
            assert_eq!(
                hex::encode(digest),
                expected,
                "the {which} list changed: an entry was edited, added, removed \
                 or MOVED. Every identity drawing at or after the affected index \
                 now renders differently from every peer that has not updated."
            );
        }
    }

    #[test]
    fn a_name_is_the_same_every_time_and_across_a_rebuild() {
        // Determinism, which every other requirement rests on. Two derivations
        // of one key, and the same key rebuilt from its bytes — the closest a
        // unit test gets to "survives a restart", since nothing here is stored.
        let key = a_key(3).public_key();
        assert_eq!(display_name(&key), display_name(&key));

        let rebuilt = PublicKey::from_bytes(&key.to_bytes()).unwrap();
        assert_eq!(display_name(&key), display_name(&rebuilt));
    }

    #[test]
    fn different_keys_generally_give_different_names() {
        // The derivation must depend on its input. Without this, a scheme
        // returning one constant name passes every determinism test above.
        let names: Vec<String> = (1u8..40)
            .map(|s| display_name(&a_key(s).public_key()).unwrap().render())
            .collect();
        let mut distinct = names.clone();
        distinct.sort();
        distinct.dedup();
        assert!(
            distinct.len() > 30,
            "39 keys produced only {} distinct names",
            distinct.len()
        );
    }

    // ─── The digests are different functions ───────────────────────────────

    #[test]
    fn the_name_digest_is_neither_the_address_nor_a_bare_hash() {
        // The claim the whole channel-disjointness argument rests on: the name
        // and the mark read DIFFERENT DIGESTS, so there is no shared space in
        // which they could overlap and no allocation of address bytes to the
        // name is required or possible.
        let key = a_key(5).public_key();
        assert_ne!(
            name_digest(&key),
            *key.address().as_bytes(),
            "the name digest must not be the address"
        );

        // And not an undomain-separated hash of the key either, or the prefix
        // would be doing nothing.
        let bare: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(key.to_bytes());
            h.finalize().into()
        };
        assert_ne!(name_digest(&key), bare);
    }

    #[test]
    fn a_name_cannot_be_derived_from_an_address() {
        // A caller holding only an address cannot arrive at the right name, which
        // is precisely why core must return it. Feed the address bytes in where
        // the DIGEST goes and the name differs from the key's real name.
        let key = a_key(11).public_key();
        let real = display_name(&key).unwrap();
        let from_address = name_from_digest(key.address().as_bytes()).unwrap();
        assert_ne!(
            real, from_address,
            "an address must not reach the same name as its key"
        );
    }

    // ─── The shape ─────────────────────────────────────────────────────────

    #[test]
    fn a_name_is_three_drawn_words_and_a_fixed_connector() {
        // **Counted by SLOT, not by space-separated token.** A place entry may
        // be a two-word toponym, so `rendered.split(' ').count()` is 4 for
        // `measured aporia of lampsacus` and 5 for `measured aporia of
        // alexandria troas` — both correct, and a test asserting 4 would fail
        // the multi-word entries the spec requires the list to accept.
        for seed in 1u8..30 {
            let name = display_name(&a_key(seed).public_key()).unwrap();
            let rendered = name.render();

            assert_eq!(name.words().len(), 3, "three drawn words: {rendered}");
            assert_eq!(
                rendered,
                format!(
                    "{} {} {CONNECTOR} {}",
                    name.adjective, name.noun, name.place
                ),
                "the connector sits between the noun and the place"
            );
            // The connector never varies with the key, so it carries no entropy.
            assert!(
                rendered.contains(&format!(" {CONNECTOR} ")),
                "the connector is the same literal text every time: {rendered}"
            );
        }
    }

    #[test]
    fn the_slots_draw_from_the_lists_they_are_specified_to_draw_from() {
        for seed in 1u8..40 {
            let name = display_name(&a_key(seed).public_key()).unwrap();
            assert!(ADJECTIVES.contains(&name.adjective), "{}", name.adjective);
            assert!(NOUNS.contains(&name.noun), "{}", name.noun);
            assert!(PLACES.contains(&name.place), "{}", name.place);
        }
    }

    #[test]
    fn a_name_without_the_connector_names_the_same_identity() {
        // The one permitted relaxation: the connector may be dropped because it
        // is the only part not derived from the key.
        let name = display_name(&a_key(9).public_key()).unwrap();
        assert_eq!(name.words(), [name.adjective, name.noun, name.place]);
        assert_eq!(
            name.render().replace(&format!(" {CONNECTOR} "), " "),
            name.words().join(" "),
            "dropping the connector leaves the three drawn words intact"
        );
    }

    // ─── The byte budget ───────────────────────────────────────────────────

    #[test]
    fn the_derivation_reads_no_byte_past_its_bound() {
        // Two digests agreeing inside the bound and differing on EVERY byte
        // beyond it. A derivation that read on would produce different names.
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        for i in 0..NAME_DIGEST_BOUND {
            a[i] = (i as u8) * 7 + 1;
            b[i] = (i as u8) * 7 + 1;
        }
        for i in NAME_DIGEST_BOUND..32 {
            a[i] = 0x00;
            b[i] = 0xff;
        }
        assert_eq!(
            name_from_digest(&a).unwrap(),
            name_from_digest(&b).unwrap(),
            "bytes past the bound must not participate"
        );
    }

    #[test]
    fn each_slot_reads_bytes_no_other_slot_reads() {
        // Vary one slot's two bytes and the OTHER TWO slots must not move. A
        // scheme feeding one byte to two slots fails this, and would have made
        // the three words three views of the same bits.
        let base = [0u8; 32];
        let reference = name_from_digest(&base).unwrap();

        for (slot, bytes) in [("adjective", 0usize), ("noun", 2), ("place", 4)] {
            let mut varied = base;
            varied[bytes] = 0x5a;
            varied[bytes + 1] = 0xa5;
            let moved = name_from_digest(&varied).unwrap();

            match slot {
                "adjective" => {
                    assert_ne!(
                        moved.adjective, reference.adjective,
                        "adjective did not move"
                    );
                    assert_eq!(moved.noun, reference.noun);
                    assert_eq!(moved.place, reference.place);
                }
                "noun" => {
                    assert_eq!(moved.adjective, reference.adjective);
                    assert_ne!(moved.noun, reference.noun, "noun did not move");
                    assert_eq!(moved.place, reference.place);
                }
                _ => {
                    assert_eq!(moved.adjective, reference.adjective);
                    assert_eq!(moved.noun, reference.noun);
                    assert_ne!(moved.place, reference.place, "place did not move");
                }
            }
        }
    }

    #[test]
    fn every_index_of_every_list_is_reachable_and_uniformly_so() {
        // Both halves in one sweep, because they are one property: over the full
        // 16-bit range every index must appear, and appear the SAME number of
        // times. A list whose size did not divide 65,536 would fail the second
        // half while passing the first.
        //
        // Counted with a vector rather than asserted from the arithmetic,
        // because the arithmetic is the thing under test.
        for (len, name) in [
            (ADJECTIVES.len(), "adjectives"),
            (NOUNS.len(), "nouns"),
            (PLACES.len(), "places"),
        ] {
            let mut counts = vec![0usize; len];
            for draw in 0u32..=u16::MAX as u32 {
                counts[(draw as u16 % len as u16) as usize] += 1;
            }
            let expected = 65_536 / len;
            assert!(
                counts.iter().all(|&c| c == expected),
                "{name}: reduction is not uniform"
            );
            assert!(
                counts.iter().all(|&c| c > 0),
                "{name}: an index is unreachable"
            );
        }
    }

    // ─── The denylist and the redraw ───────────────────────────────────────

    #[test]
    fn the_denylist_is_sorted_deduplicated_and_in_range() {
        // The property `binary_search` needs and which nothing else checks. An
        // unsorted array makes the search miss entries SILENTLY — refusing some
        // pairs and admitting others, with no error anywhere.
        for window in TRUE_ATTRIBUTION_PAIRS.windows(2) {
            assert!(
                window[0] < window[1],
                "the denylist must be sorted and deduplicated: {:?} then {:?}",
                window[0],
                window[1]
            );
        }
        for &(noun, place) in TRUE_ATTRIBUTION_PAIRS {
            assert!(
                (noun as usize) < NOUNS.len(),
                "noun index {noun} out of range"
            );
            assert!(
                (place as usize) < PLACES.len(),
                "place index {place} out of range"
            );
        }
        assert!(
            !TRUE_ATTRIBUTION_PAIRS.is_empty(),
            "the true-attribution family is mandatory, not optional"
        );
    }

    /// A digest whose first draw selects `(noun, place)` and whose reserve
    /// selects `(reserve_noun, reserve_place)`.
    ///
    /// Constructing the digest is the whole point: a refused draw is reachable
    /// through a chosen digest and reachable through a chosen KEY only by
    /// grinding for one.
    fn digest_drawing(first: (u16, u16, u16), reserve: (u16, u16, u16)) -> [u8; 32] {
        let mut d = [0u8; 32];
        for (i, v) in [first.0, first.1, first.2, reserve.0, reserve.1, reserve.2]
            .iter()
            .enumerate()
        {
            d[i * 2] = (v >> 8) as u8;
            d[i * 2 + 1] = *v as u8;
        }
        d
    }

    #[test]
    fn a_refused_pair_redraws_every_slot_from_the_reserve() {
        // Not "the refused pair is not returned", which a scheme substituting one
        // slot also satisfies. The digest is built so that the two differ in ALL
        // THREE slots, so a single-slot substitution fails here.
        let (noun, place) = TRUE_ATTRIBUTION_PAIRS[0];
        // A reserve draw that is deliberately different in every slot.
        let reserve_noun = (noun + 1) % NOUNS.len() as u16;
        let reserve_place = (place + 1) % PLACES.len() as u16;
        assert!(
            !is_refused(reserve_noun, reserve_place),
            "the fixture's reserve draw must itself be permitted"
        );

        let digest = digest_drawing((0, noun, place), (500, reserve_noun, reserve_place));
        let name = name_from_digest(&digest).unwrap();

        assert_eq!(
            name.adjective, ADJECTIVES[500],
            "the adjective was not redrawn"
        );
        assert_eq!(name.noun, NOUNS[reserve_noun as usize]);
        assert_eq!(name.place, PLACES[reserve_place as usize]);

        // And the refused combination really is not what came back.
        assert_ne!(
            (name.noun, name.place),
            (NOUNS[noun as usize], PLACES[place as usize]),
            "the refused pair was returned"
        );
        // The adjective moved too, which is what distinguishes a whole-name
        // redraw from a pair substitution.
        assert_ne!(name.adjective, ADJECTIVES[0]);
    }

    #[test]
    fn a_redraw_is_deterministic() {
        let (noun, place) = TRUE_ATTRIBUTION_PAIRS[0];
        let digest = digest_drawing((0, noun, place), (500, 1, 1));
        assert_eq!(
            name_from_digest(&digest).unwrap(),
            name_from_digest(&digest).unwrap()
        );
    }

    #[test]
    fn an_unrefused_draw_never_consults_the_reserve() {
        // Two digests differing ONLY in bytes 6..11. If the first draw is
        // permitted the reserve must be untouched, so the names must be equal.
        let a = digest_drawing((10, 20, 30), (0, 0, 0));
        assert!(
            !is_refused(20, 30),
            "the fixture's first draw must be permitted"
        );
        let mut b = a;
        b[6..12].fill(0xff);
        assert_ne!(a, b, "the fixture must actually differ in the reserve");
        assert_eq!(name_from_digest(&a).unwrap(), name_from_digest(&b).unwrap());
    }

    #[test]
    fn exhausting_the_reserve_fails_rather_than_reading_on() {
        // Both draws refused. The derivation must report a failure and return no
        // name — reading a third draw from bytes 12.. is the unbounded read this
        // bound exists to prevent.
        let (n1, p1) = TRUE_ATTRIBUTION_PAIRS[0];
        let (n2, p2) = TRUE_ATTRIBUTION_PAIRS[1];
        let digest = digest_drawing((0, n1, p1), (0, n2, p2));
        assert_eq!(
            name_from_digest(&digest),
            Err(NameError::ReserveExhausted),
            "two refused draws must exhaust the reserve"
        );
    }

    #[test]
    fn a_refused_pair_is_refused_on_the_pair_and_not_on_either_word() {
        // Both halves stay in their lists. The same noun with a different place
        // must draw normally, or the refusal has cost a word rather than a pair.
        let (noun, place) = TRUE_ATTRIBUTION_PAIRS[0];
        assert!(is_refused(noun, place));

        let mut found_permitted_place = false;
        for candidate in 0..PLACES.len() as u16 {
            if !is_refused(noun, candidate) {
                found_permitted_place = true;
                break;
            }
        }
        assert!(
            found_permitted_place,
            "the noun must still pair with some place"
        );

        let mut found_permitted_noun = false;
        for candidate in 0..NOUNS.len() as u16 {
            if !is_refused(candidate, place) {
                found_permitted_noun = true;
                break;
            }
        }
        assert!(
            found_permitted_noun,
            "the place must still pair with some noun"
        );
    }

    #[test]
    fn no_name_a_key_can_reach_is_a_refused_combination() {
        for seed in 1u8..=255 {
            let name = display_name(&a_key(seed).public_key()).unwrap();
            let noun = NOUNS.iter().position(|&n| n == name.noun).unwrap() as u16;
            let place = PLACES.iter().position(|&p| p == name.place).unwrap() as u16;
            assert!(
                !is_refused(noun, place),
                "key {seed} rendered a refused pair: {name}"
            );
        }
    }

    // ─── Malformed input ───────────────────────────────────────────────────

    #[test]
    fn malformed_key_material_is_refused_rather_than_crashed_on() {
        // A public key on this path arrives inside an inbound op and is
        // attacker-controlled, and a panic aborts the module process. Every one
        // of these must be a Result, never an abort, and never a name.
        for n in [0usize, 1, 31, 33, 64, 1024] {
            let out = display_name_from_bytes(&vec![0u8; n]);
            assert!(out.is_err(), "a {n}-byte key must be refused");
        }

        // 32 bytes that are not a decompressable Edwards point.
        let mut not_a_point = [0u8; 32];
        not_a_point[0] = 2;
        assert!(display_name_from_bytes(&not_a_point).is_err());

        // And the low-order all-zero point, which decompresses and is refused at
        // the parse.
        assert_eq!(
            display_name_from_bytes(&[0u8; 32]),
            Err(NameError::NotAValidPublicKey(KeyError::WeakPublicKey))
        );
    }

    #[test]
    fn a_failure_is_never_reported_as_a_name() {
        // No placeholder, no name for "unknown", no name from padded input. Each
        // would render as an ordinary participant — a name attributable to
        // nobody, presented as one attributable to somebody.
        let short = display_name_from_bytes(b"too short");
        assert!(short.is_err());
        assert!(
            short.unwrap_err().to_string().contains("cannot derive"),
            "the error must say a name could not be derived"
        );

        // Padding a short key to 32 bytes must not be what the implementation
        // does: a 9-byte key and the same bytes zero-padded must not agree.
        let mut padded = [0u8; 32];
        padded[..9].copy_from_slice(b"too short");
        // Whether the padded value parses at all is incidental; what matters is
        // that the SHORT one produced no name.
        let _ = display_name_from_bytes(&padded);
    }

    #[test]
    fn arbitrary_bytes_do_not_abort_the_process() {
        // A sweep rather than a handful: every length from 0 to 70, and a
        // high-byte pattern, all of which must return rather than panic.
        for n in 0usize..70 {
            let _ = display_name_from_bytes(&vec![0xffu8; n]);
            let _ = display_name_from_bytes(&vec![0x01u8; n]);
        }
    }

    // ─── A name is not a credential ────────────────────────────────────────

    #[test]
    fn an_unknown_key_still_derives_a_name() {
        // There is nothing to look up, so there is nothing that could fail to be
        // FOUND. A freshly generated key this peer has never seen must derive.
        let stranger = SecretKey::generate().unwrap().public_key();
        assert!(display_name(&stranger).is_ok());
    }

    #[test]
    fn a_name_carries_no_marking_of_authority() {
        // A name is rendered next to moderator badges, where a reader may read
        // the pair as one claim. Nothing in a name distinguishes a moderator's
        // from anyone else's — the rendered form is three words and a connector
        // either way, with no marker to carry standing.
        let moderator = display_name(&a_key(1).public_key()).unwrap();
        let participant = display_name(&a_key(2).public_key()).unwrap();
        for name in [moderator, participant] {
            let rendered = name.render();
            // Three slots — counted by slot rather than by token, since a place
            // may be a two-word toponym.
            assert_eq!(name.words().len(), 3);
            assert!(
                rendered.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                "a name carries no marking: {rendered}"
            );
        }
    }

    // ─── The two screens, over every list ──────────────────────────────────

    #[test]
    fn every_entry_of_every_list_is_ascii_lowercase_and_well_formed() {
        // ASCII is a BIDI decision rather than a typographic preference: these
        // are the one piece of rendered text this project fully composes from a
        // fixed list, so keeping them ASCII means a generated name can never
        // itself carry a bidi override or a homoglyph. It removes the attack from
        // this surface rather than mitigating it.
        //
        // **An INTERNAL SPACE is permitted, and that is the point of this
        // test's history.** An earlier draft of the spec imposed a single-word
        // screen, and this test enforced it with a `!contains whitespace`
        // assertion. That screen is exactly the third screen the spec forbids —
        // only ASCII-transliterable and deduplicated apply — and it is the
        // costly one: it discards `alexandria troas` and `heraclea pontica`,
        // and it is what a census blamed for putting the place list out of
        // reach. What is checked instead is that the spacing is WELL FORMED: no
        // leading or trailing space, and no double space, so an entry is one
        // place named in one or more words rather than a formatting accident.
        for (list, which) in [
            (ADJECTIVES, "adjectives"),
            (NOUNS, "nouns"),
            (PLACES, "places"),
        ] {
            for entry in list {
                assert!(entry.is_ascii(), "{which}: {entry:?} is not ASCII");
                assert_eq!(
                    *entry,
                    entry.to_ascii_lowercase(),
                    "{which}: {entry:?} is not lowercase"
                );
                assert!(!entry.is_empty(), "{which}: an empty entry");
                assert!(
                    !entry.starts_with(' ') && !entry.ends_with(' '),
                    "{which}: {entry:?} has a leading or trailing space"
                );
                assert!(
                    !entry.contains("  "),
                    "{which}: {entry:?} has a double space"
                );
                assert!(
                    entry.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                    "{which}: {entry:?} has a character that is neither a \
                     lowercase letter nor a space"
                );
            }
        }
    }

    #[test]
    fn a_multi_word_place_entry_is_accepted_and_renders_as_one_place() {
        // The screen removal, asserted rather than assumed. A two-word toponym
        // draws and renders as ONE place, with the connector still preceding
        // the whole of it — `... of alexandria troas`, never `... of alexandria`
        // with the second half lost.
        let multi_word: Vec<&&str> = PLACES.iter().filter(|p| p.contains(' ')).collect();
        assert!(
            !multi_word.is_empty(),
            "the place list must hold at least one multi-word toponym, or the \
             single-word screen has crept back in"
        );

        let index = PLACES.iter().position(|p| p.contains(' ')).unwrap() as u16;
        let name = DisplayName {
            adjective: ADJECTIVES[0],
            noun: NOUNS[0],
            place: PLACES[index as usize],
        };
        assert!(
            name.render()
                .ends_with(&format!("{CONNECTOR} {}", PLACES[index as usize])),
            "a multi-word place must render whole, after the connector: {}",
            name.render()
        );
        assert_eq!(name.words().len(), 3, "a two-word place is still one slot");
    }

    #[test]
    fn no_list_holds_a_duplicate() {
        // One entry per word, per person and per place. A duplicate is not
        // merely untidy: it makes one name reachable by two indices and skews the
        // uniformity the reduction argument depends on.
        for (list, which) in [
            (ADJECTIVES, "adjectives"),
            (NOUNS, "nouns"),
            (PLACES, "places"),
        ] {
            let mut sorted: Vec<&str> = list.to_vec();
            sorted.sort_unstable();
            let before = sorted.len();
            sorted.dedup();
            assert_eq!(before, sorted.len(), "{which} holds a duplicate");
        }
    }

    #[test]
    fn the_lists_are_the_sizes_the_arithmetic_rests_on() {
        // The sizes are what make the reduction unbiased and what the collision
        // arithmetic is computed against. A size that is not a power of two
        // introduces a bias; a different power of two reindexes every draw.
        assert_eq!(ADJECTIVES.len(), 8_192);
        assert_eq!(NOUNS.len(), 1_024);
        assert_eq!(PLACES.len(), 1_024);

        // 8192 * 1024 * 1024 = 2^33.
        assert_eq!(
            ADJECTIVES.len() as u64 * NOUNS.len() as u64 * PLACES.len() as u64,
            8_589_934_592
        );
    }

    #[test]
    fn no_noun_entry_carries_the_connector_as_a_word() {
        // The `X of Y` shape is what makes this reachable. A source supplying
        // named historical Greeks supplies them already QUALIFIED — `zenon
        // kitieus`, `straton lampsakenos` — and such an entry renders
        // *measured zeno of citium of lampsacus*, which reads as two places
        // attached to one name and leaves a reader unable to tell which of them
        // the place slot supplied.
        //
        // This is a rule about ONE LITERAL SUBSTRING and not a semantic screen:
        // what the noun means is still no part of whether it is in.
        let connector_as_word = format!(" {CONNECTOR} ");
        for entry in NOUNS {
            assert!(
                !entry.contains(&connector_as_word),
                "noun {entry:?} carries the connector as a word"
            );
        }
    }

    #[test]
    fn the_lists_carry_no_exclusion_of_any_kind() {
        // **The screens are three and they are all mechanical: ASCII,
        // deduplicated, attested.** Every successive draft that added a fourth
        // was withdrawn on challenge — familiarity, which cut the place list by
        // 28%; a rebadged "legibility", which cut it by 88%; a single-word rule,
        // which made 1,024 places look unreachable; and a tone-and-authority
        // apparatus.
        //
        // This test is the inverse of the three it replaced. Those asserted
        // that `stoa`, `platon`, `sokrates`, `archon` and `strategos` were
        // ABSENT. The contract now says no word is kept out for what it says,
        // what it connotes or whom it names, so their absence would be the
        // defect and their presence is the requirement.
        //
        // Pinned as PRESENT rather than merely "not asserted absent", because a
        // curation pass that quietly dropped them would otherwise reintroduce
        // the withdrawn screen with every test still green.
        for term in ["stoa", "agora", "archon", "tyrannos", "genesis"] {
            assert!(
                NOUNS.contains(&term),
                "{term} was excluded; there is no exclusion screen"
            );
        }
        for figure in ["platon", "aristoteles", "sokrates"] {
            assert!(
                NOUNS.contains(&figure),
                "{figure} was excluded; no figure is kept out for whom it names"
            );
        }
    }

    #[test]
    fn two_distinct_keys_can_share_a_name_and_neither_is_marked() {
        // "A name is never unique, never an identifier, and never numbered."
        //
        // Uniqueness is UNAVAILABLE rather than merely unbuilt: there is no
        // registry and no authority to hold a namespace, so two peers can each
        // believe a name is free. Numbering would be worse than leaving a
        // collision alone — appending a suffix requires agreeing which identity
        // was second, which is arrival order, a per-peer fact, so two peers
        // would number the same pair oppositely and each be certain the other
        // was the impostor.
        //
        // The pair was found by SEARCHING the real derivation (see
        // `tests_support::COLLIDING_SEED_A`), so this exercises the shipped
        // scheme rather than a stub. A stubbed derivation would prove the feed
        // handles a collision but say nothing about whether one is reachable.
        let a = SecretKey::from_bytes(&tests_support::COLLIDING_SEED_A).unwrap();
        let b = SecretKey::from_bytes(&tests_support::COLLIDING_SEED_B).unwrap();

        // The fixture must actually be two DIFFERENT identities, or "both are
        // served unchanged" is satisfied by one key compared with itself.
        assert_ne!(
            a.public_key().to_bytes(),
            b.public_key().to_bytes(),
            "the fixture must be two distinct keys"
        );
        assert_ne!(
            a.public_key().address(),
            b.public_key().address(),
            "two distinct keys must have distinct addresses — the addresses are \
             what tells a colliding pair apart"
        );

        let name_a = display_name(&a.public_key()).unwrap();
        let name_b = display_name(&b.public_key()).unwrap();

        // They collide, and on the WRITTEN-DOWN name rather than merely on each
        // other: `assert_eq!(name_a, name_b)` alone would pass on a derivation
        // that returned one constant for every key.
        assert_eq!(name_a.render(), tests_support::COLLIDING_NAME);
        assert_eq!(name_b.render(), tests_support::COLLIDING_NAME);

        // Neither carries a number, a suffix or any other distinguishing mark.
        // Checked as a character class over the whole rendered name, so a `#2`,
        // a ` (2)` or a trailing digit all fail.
        for name in [name_a, name_b] {
            let rendered = name.render();
            assert!(
                rendered.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                "a colliding name must carry no added mark: {rendered}"
            );
            assert_eq!(name.words().len(), 3, "still three drawn words");
        }
    }

    #[test]
    fn a_refused_pair_leaves_both_of_its_words_drawing_freely() {
        // The denylist refuses a COMPOSITION, never a word. Both halves of every
        // refused pair must still be in their lists — this is what distinguishes
        // the true-attribution family from the exclusions the contract forbids.
        for &(noun, place) in TRUE_ATTRIBUTION_PAIRS {
            assert!(
                NOUNS.contains(&NOUNS[noun as usize]),
                "a refused pair cost a noun its place in the list"
            );
            assert!(
                PLACES.contains(&PLACES[place as usize]),
                "a refused pair cost a place its place in the list"
            );
        }
    }
}
