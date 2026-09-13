//! Generated display names: a readable name for a key, computed and never typed.
//!
//! # What this is for
//!
//! A 32-byte address is unreadable, and unreadable pseudonymity is not
//! pseudonymity in any useful sense (PLAN.md §5.2.1) — a reader who cannot tell
//! two participants apart at a glance cannot follow an argument between them,
//! which is the one thing this forum is named for. So every identity renders
//! under three drawn words: *pensive aporia of lampsakos*.
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
//! **The consequence decides which value a reply owes, and it is the KEY.** A
//! reply reporting an author by *address* alone has handed its caller a hash
//! from which no key is recoverable, so that caller cannot compute the name —
//! which is a real gap, and [`crate::feed::FeedRow`] currently has it.
//!
//! **The fix is to carry the key, never the name.** A name beside the key it
//! derives from is two values that must agree and could disagree, with no way
//! for a recipient to tell which is wrong, and the one on the wire is the one a
//! relay could strip or forge. So no reply carries a name — not a feed row, not
//! a thread item, not a slate candidate — and core exposes this derivation
//! instead, so that a caller holding a key never has to reimplement a
//! consensus-critical scheme it holds no wordlists for.
//!
//! An earlier pass put a `displayName` on the feed row, reasoning correctly from
//! the address-only gap to the wrong remedy. Putting the public key on that row
//! is a change to its `author` contract across several merged specs, so it is
//! filed as its own piece; until it lands the feed path cannot render a name and
//! `docs/UI-BRIEF.md` obligation 6 records the obligation.
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
mod nouns;
mod places;

pub use adjectives::ADJECTIVES;
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
/// caller may drop: *pensive aporia lampsakos* has identical information
/// content and misleads no reader about who published something.
pub const CONNECTOR: &str = "of";

/// The first byte past the name's slice of the digest.
///
/// Six bytes: three 16-bit slots, one unconditional draw each. **This names no
/// byte the derivation does not read**, which is the point of it being 6 rather
/// than a wider figure with an unread tail. A bound stated wider than the draws
/// records a boundary nothing enforces — a range read by nothing, which the next
/// reader takes as load-bearing and designs around. That is the byte reservation
/// this scheme has already had to retract once.
///
/// **It bounds by being what the derivation slices, not by being asserted.** An
/// earlier version of this constant was compared to its own literal in a
/// `debug_assert_eq!` — a tautology that compiled out in release and could not
/// fail under any edit to the draws. [`name_from_digest`] now takes its six
/// bytes as `digest[..NAME_DIGEST_BOUND]`, so moving the bound moves the read
/// and a draw past it does not compile.
///
/// The number of bytes a name consumes is therefore fixed rather than
/// data-dependent: no input makes the derivation read a seventh byte. Reading
/// without bound is what lets two implementations disagree about how far to read
/// and so produce different names for one key, which is the failure this whole
/// scheme exists to prevent.
const NAME_DIGEST_BOUND: usize = 6;

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

    /// The rendered name: *pensive aporia of lampsakos*.
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
/// **Malformed key material is the only failure this capability has**, and the
/// single arm below is the whole of it. There is deliberately no second arm: an
/// error variant no input can produce is an unreachable branch that a reader
/// takes as evidence the failure exists, and a caller then handles a case that
/// cannot arrive. An earlier version carried a `ReserveExhausted` arm for a
/// denylist that no longer exists; with every draw now a single unconditional
/// reduction there is nothing left that can fail after the key parses.
///
/// **There is no arm meaning "unknown key".** A name is a function of the key
/// alone, consulting no moderator set, no genesis record and no stored state, so
/// there is nothing to look up and nothing that could fail to be found. A name
/// is derivable for any well-formed public key including one belonging to no
/// identity this peer has seen.
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
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::NotAValidPublicKey(e) => {
                write!(f, "cannot derive a display name: {e}")
            }
        }
    }
}

/// The display name for a public key.
///
/// **Total, and the return type says so.** Every well-formed key yields a name:
/// three unconditional reductions over six digest bytes, with nothing that can
/// refuse, retry or run out. There is no `Result` here because there is no
/// failure to report — a [`PublicKey`] cannot be constructed from malformed
/// bytes, so the one failure this capability has is already behind us by the
/// time this is called. See [`display_name_from_bytes`] for the entry point that
/// parses, which is where that failure lives.
///
/// **This totality is load-bearing rather than tidy.** While this returned a
/// `Result`, `feed.rs` carried an `else { continue }` that dropped a post whose
/// author's name could not be derived — a name failure censoring content in a
/// censorship-resistant forum, on a branch no input could reach. Making the
/// derivation total deletes the branch rather than testing it.
pub fn display_name(key: &PublicKey) -> DisplayName {
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
    Ok(display_name(&key))
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
/// that a testability obligation rather than a convenience.** Reaching a chosen
/// slot combination through a chosen *key* means grinding for one, so the
/// pinning and uniformity requirements are checkable only if a digest can be
/// supplied directly.
///
/// **Total, and infallible by construction.** Every draw is a single
/// unconditional reduction: nothing refuses a combination, nothing retries, and
/// there is no budget to exhaust. So the bytes a name consumes are fixed rather
/// than data-dependent, every index of every list is reachable, and the `2^33`
/// space is reached exactly rather than approximately.
///
/// # The byte budget
///
/// | bytes | slot |
/// |---|---|
/// | `0..2` | adjective |
/// | `2..4` | noun |
/// | `4..6` | place |
///
/// Bytes `6..32` are **never read**. The slice below is taken at
/// [`NAME_DIGEST_BOUND`] rather than indexed past it, so the bound is what the
/// derivation reads rather than a figure a comment asserts: widening a draw past
/// it does not compile.
///
/// Each slot takes its index from bytes no other slot reads, so the three words
/// are independent draws rather than three views of the same bits.
pub fn name_from_digest(digest: &[u8; 32]) -> DisplayName {
    // The bound, applied rather than asserted. Everything below reads from this
    // slice, so there is no path that reaches a seventh byte.
    let drawn: &[u8; NAME_DIGEST_BOUND] = digest[..NAME_DIGEST_BOUND]
        .try_into()
        .expect("a 32-byte digest always yields its first NAME_DIGEST_BOUND bytes");

    // Big-endian so the bytes read in the order a hand-computed test vector is
    // written.
    let word = |i: usize| u16::from_be_bytes([drawn[i], drawn[i + 1]]);

    // Why a 16-bit draw reduced by `%` is exactly uniform: 65,536 / 8,192 = 8
    // and 65,536 / 1,024 = 64, both whole numbers, so every index of every list
    // is produced by the same number of 16-bit values as every other. There is
    // no modulo bias to trade off and no word is favoured.
    //
    // This is what makes the power-of-two list sizes load-bearing rather than
    // incidental. A list of 1,000 would introduce a real if tiny bias; a list of
    // a different power of two would reindex every draw. The spec forbids
    // changing a size without a version bump for exactly this reason.
    let adjective_index = word(0) % ADJECTIVES.len() as u16;
    let noun_index = word(2) % NOUNS.len() as u16;
    let place_index = word(4) % PLACES.len() as u16;

    DisplayName {
        adjective: ADJECTIVES[adjective_index as usize],
        noun: NOUNS[noun_index as usize],
        place: PLACES[place_index as usize],
    }
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
/// `examples/pin_name.rs`, which reads the wordlists from the text files in
/// `wordlists/` and does the index arithmetic itself rather than calling
/// [`name_from_digest`] — it does not link the derivation at all.
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
        let name = display_name(&a_key(7).public_key());
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
    /// does the index arithmetic in the test rather than by calling
    /// `name_from_digest`.
    const PINNED_NAME_FOR_KEY_7: &str = "expatiative karpos of pythion";
    const PINNED_DIGEST_FOR_KEY_7: &str =
        "09c2b1c9373894cc7c47ee573ca06a8d9950a834d0dbb69a35ffcbcb0cdf7e74";

    #[test]
    fn the_pinned_name_is_derivable_by_hand_from_the_pinned_digest() {
        // The pin above is only independent if it can be reached WITHOUT the
        // function under test. This does the index arithmetic here, from the
        // digest's bytes, and indexes the lists directly — so it fails if
        // `name_from_digest` reads different bytes, reduces differently, orders
        // the slots differently, or emits a different connector.
        //
        // `PINNED_NAME_FOR_KEY_7` and `PINNED_DIGEST_FOR_KEY_7` were produced by
        // `examples/pin_name.rs`, which reads the wordlists from the TEXT FILES
        // in `wordlists/` that the modules were generated from, and which does
        // not link the derivation. So three independent routes — that program,
        // this arithmetic, and `display_name` — must agree.
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
    fn the_pinned_cases_span_each_list_rather_than_clustering() {
        // The spec requires pinned cases to reach **a low and a high index in
        // each of the three slots**, so that a pin is evidence about the index
        // arithmetic and not only about one region of one list. A pin drawn from
        // a key reaches whatever index that key's digest happens to select —
        // key 7 lands at (2498, 457, 824) — and no key can be chosen to land on
        // a wanted index without grinding for one. So the span is reached
        // through CONSTRUCTED DIGESTS, which is the testability seam
        // `name_from_digest` is public for.
        //
        // This replaces a pin on the redraw path. There is no redraw to pin:
        // every draw is now one unconditional reduction, so the only thing a
        // second pinned case can add is coverage of the index arithmetic at the
        // ends of each list — which is what the spec asks for and what the old
        // reserve pin, clustered at index 7 of all three lists, did not give.
        //
        // Both names below are WRITTEN DOWN, produced by `examples/pin_name.rs`
        // reading the text files, not read back from `name_from_digest`.
        //
        // Index 0 of each list. A digest of six zero bytes draws (0, 0, 0)
        // because `0 % n == 0` for every n, so this fixture needs no arithmetic
        // to justify the indices it claims.
        let low = name_from_digest(&digest_drawing(0, 0, 0));
        assert_eq!(low.render(), PINNED_NAME_AT_LOW_INDICES);

        // The last index of each list: 8,191 and 1,023. Reached by drawing the
        // 16-bit value equal to the index itself, which is below every list's
        // length and so survives the reduction unchanged.
        let high = name_from_digest(&digest_drawing(8_191, 1_023, 1_023));
        assert_eq!(high.render(), PINNED_NAME_AT_HIGH_INDICES);

        // The two must actually differ in every slot, or "spanning" is one name
        // written twice. This is what makes the pair evidence about the
        // arithmetic across each list rather than about one region of it.
        assert_ne!(low.adjective, high.adjective);
        assert_ne!(low.noun, high.noun);
        assert_ne!(low.place, high.place);
    }

    /// Index 0 of each list: the first entry of adjectives, nouns and places.
    const PINNED_NAME_AT_LOW_INDICES: &str = "abandonable acheron of abai";
    /// The last index of each list: adjective 8,191, noun 1,023, place 1,023.
    const PINNED_NAME_AT_HIGH_INDICES: &str = "zygopterous zythos of zone";

    /// SHA-256 over every entry of each list, in order, each followed by `\n`.
    ///
    /// **Produced by `sha256sum` over the source text files, never by hashing
    /// the arrays.** The three commands, which a reviewer can re-run from the
    /// repository root:
    ///
    /// ```text
    /// sha256sum dialectica/rust-lib/dialectica-core/wordlists/adjectives.txt
    /// sha256sum dialectica/rust-lib/dialectica-core/wordlists/nouns.txt
    /// sha256sum dialectica/rust-lib/dialectica-core/wordlists/places.txt
    /// ```
    ///
    /// **These paths are tracked, and that is load-bearing rather than tidy.**
    /// The files lived under a gitignored `tmp/` and the three commands failed
    /// on every checkout but their author's — so the provenance chain that
    /// justifies generating these arrays rather than parsing a data file had no
    /// artefact behind it, and the pins below degraded to values a reader could
    /// only believe. `examples/gen_wordlists.rs` reads exactly these files.
    ///
    /// Each file holds one entry per line and ends in a trailing newline, so the
    /// concatenation `entry + "\n"` reproduces the file's bytes exactly. That is
    /// what lets a hash over the shipped array be compared against a hash over
    /// the source file, which is the only shape in which this pin is evidence
    /// about anything: hashing the array and writing down the answer would be
    /// the implementation agreeing with itself.
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
            .map(|s| display_name(&a_key(s).public_key()).render())
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

        // **The two assertions above pass without domain separation, and that
        // is why this third one exists.** Measured: setting `NAME_PREFIX` to the
        // author-address separator — a total loss of separation — left both of
        // them passing, because the address hashes `PREFIX || 0x01 || key` while
        // the name hashes `PREFIX || key`. It is the record-count byte that
        // separates those two digests, not the prefix, so the test named for
        // domain separation was blind to domain separation being removed.
        //
        // This asserts the separation directly: hashing the key under ANOTHER
        // capability's separator, in this module's own `PREFIX || key` shape,
        // must not reach the name digest. The separators are private consts in
        // other modules, so each is WRITTEN DOWN here rather than imported —
        // which is the right shape anyway, since importing them would let a
        // future edit move a separator and this test together.
        //
        // If one of these fails, a separator has been duplicated. Do not update
        // the literal to match; two capabilities hashing the same preimage means
        // grinding for one grinds for the other, and the costs add instead of
        // multiplying.
        const OTHER_SEPARATORS: [(&str, &[u8; 32]); 5] = [
            ("author address", b"/dialectica/1/Address/Author\0\0\0\0"),
            ("stoa address", b"/dialectica/1/Address/Stoa\0\0\0\0\0\0"),
            ("op signing", b"/dialectica/1/Signed/Op\0\0\0\0\0\0\0\0\0"),
            ("op id", b"/dialectica/1/Id/Op\0\0\0\0\0\0\0\0\0\0\0\0\0"),
            ("slate path", b"/dialectica/1/Slate/Path\0\0\0\0\0\0\0\0"),
        ];
        for (which, separator) in OTHER_SEPARATORS {
            assert_ne!(
                separator, NAME_PREFIX,
                "the name's separator is the {which} separator, so the two \
                 capabilities are one function of the key"
            );
            let under_other: [u8; 32] = {
                let mut h = Sha256::new();
                h.update(separator);
                h.update(key.to_bytes());
                h.finalize().into()
            };
            assert_ne!(
                name_digest(&key),
                under_other,
                "the name digest equals the key hashed under the {which} \
                 separator, so name derivation is not domain-separated from it"
            );
        }
    }

    #[test]
    fn a_name_cannot_be_derived_from_an_address() {
        // A caller holding only an address cannot arrive at the right name, which
        // is precisely why core must return it. Feed the address bytes in where
        // the DIGEST goes and the name differs from the key's real name.
        let key = a_key(11).public_key();
        let real = display_name(&key);
        let from_address = name_from_digest(key.address().as_bytes());
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
        // `pensive aporia of lampsakos` and 5 for `pensive aporia of lokroi
        // epizephyrioi` — both correct, and a test asserting 4 would fail the
        // multi-word entries the spec requires the list to accept.
        for seed in 1u8..30 {
            let name = display_name(&a_key(seed).public_key());
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
            let name = display_name(&a_key(seed).public_key());
            assert!(ADJECTIVES.contains(&name.adjective), "{}", name.adjective);
            assert!(NOUNS.contains(&name.noun), "{}", name.noun);
            assert!(PLACES.contains(&name.place), "{}", name.place);
        }
    }

    #[test]
    fn a_name_without_the_connector_names_the_same_identity() {
        // The one permitted relaxation: the connector may be dropped because it
        // is the only part not derived from the key.
        let name = display_name(&a_key(9).public_key());
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
            name_from_digest(&a),
            name_from_digest(&b),
            "bytes past the bound must not participate"
        );
    }

    #[test]
    fn each_slot_reads_bytes_no_other_slot_reads() {
        // Vary one slot's two bytes and the OTHER TWO slots must not move. A
        // scheme feeding one byte to two slots fails this, and would have made
        // the three words three views of the same bits.
        let base = [0u8; 32];
        let reference = name_from_digest(&base);

        for (slot, bytes) in [("adjective", 0usize), ("noun", 2), ("place", 4)] {
            let mut varied = base;
            varied[bytes] = 0x5a;
            varied[bytes + 1] = 0xa5;
            let moved = name_from_digest(&varied);

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
        // 16-bit range every WORD must appear, and appear the SAME number of
        // times. A list whose size did not divide 65,536 would fail the second
        // half while passing the first.
        //
        // **Driven through `name_from_digest`, not through the test's own
        // modulo.** An earlier version computed `draw as u16 % len as u16` in
        // this body and never called the derivation at all, so it tested the
        // arithmetic of `%` — a property of Rust — rather than the derivation's
        // use of it. Measured: replacing the adjective reduction with
        // `(word(0) % (ADJECTIVES.len() as u16 - 1)) + 1`, which makes index 0
        // unreachable and the reduction biased, left that version passing. It
        // now fails here.
        //
        // Counted by WORD rather than by index, because the word is what a
        // reader sees and what a second implementation must agree on; an index
        // is an internal step. Words are unique per list
        // (`no_list_holds_a_duplicate`), so the counts are equivalent, and
        // counting the visible thing means a mutation that reindexed without
        // changing the modulus is still caught.
        for (slot, list, label) in [
            (0usize, ADJECTIVES, "adjectives"),
            (1, NOUNS, "nouns"),
            (2, PLACES, "places"),
        ] {
            let mut counts: std::collections::HashMap<&str, usize> =
                list.iter().map(|w| (*w, 0usize)).collect();

            for draw in 0u32..=u16::MAX as u32 {
                // Every other slot is held at zero, so only this slot's two
                // bytes vary and the word read back is this slot's draw.
                let mut digest = [0u8; 32];
                digest[slot * 2] = (draw >> 8) as u8;
                digest[slot * 2 + 1] = draw as u8;

                let name = name_from_digest(&digest);
                let word = name.words()[slot];
                *counts
                    .get_mut(word)
                    .unwrap_or_else(|| panic!("{label}: {word} is not in its own list")) += 1;
            }

            let expected = 65_536 / list.len();
            assert!(
                counts.values().all(|&c| c == expected),
                "{label}: reduction is not uniform — some word is drawn more \
                 often than another, so the 2^33 space is not reached exactly"
            );
            assert!(
                counts.values().all(|&c| c > 0),
                "{label}: a word is unreachable, so the list is larger than the \
                 space the derivation can select from"
            );
            // The sweep must have covered the whole list, or "every word" is a
            // claim about however many words happened to be counted.
            assert_eq!(
                counts.len(),
                list.len(),
                "{label}: the count table lost an entry"
            );
        }
    }

    // ─── Nothing filters a drawn name ──────────────────────────────────────

    /// A digest whose three draws select exactly `(adjective, noun, place)`.
    ///
    /// Constructing the digest is the whole point and is the reason
    /// [`name_from_digest`] is public: reaching a chosen slot combination
    /// through a chosen KEY means grinding for one, so a requirement about
    /// *which* words come back is checkable only if a digest can be supplied
    /// directly.
    ///
    /// Each index is written big-endian into its slot's two bytes. An index
    /// below its list's length survives the `%` unchanged, so the digest this
    /// builds selects the indices it names.
    fn digest_drawing(adjective: u16, noun: u16, place: u16) -> [u8; 32] {
        let mut d = [0u8; 32];
        for (i, v) in [adjective, noun, place].iter().enumerate() {
            d[i * 2] = (v >> 8) as u8;
            d[i * 2 + 1] = *v as u8;
        }
        d
    }

    #[test]
    fn every_combination_the_draws_select_is_returned() {
        // The requirement is that NOTHING is refused, substituted, suppressed or
        // redrawn — for what the words are, what they mean, whom they name, or
        // what they spell together. A previous version of this scheme held a
        // denylist of noun–place pairs and redrew all three slots when one was
        // hit; the owner deleted it, and this is the test that the deletion is
        // real rather than merely current.
        //
        // Swept over many combinations rather than asserted at one point,
        // because a filter is exactly the kind of thing that applies to some
        // inputs and not others — a single sample can miss it by landing
        // outside whatever the filter covered.
        //
        // `zenon of kition` is swept deliberately: it is a real historical
        // figure's canonical citation, it is the example the spec uses, and it
        // is precisely what the deleted denylist existed to refuse. It must now
        // come back like any other draw.
        let zenon = NOUNS
            .iter()
            .position(|&n| n == "zenon")
            .expect("zenon is in the noun list") as u16;
        let kition = PLACES
            .iter()
            .position(|&p| p == "kition")
            .expect("kition is in the place list") as u16;

        // Indices chosen to sweep each list rather than to name particular
        // words: both ends, the middles, and the one combination that matters
        // by meaning. Every value is below its list's length, so each survives
        // the `%` unchanged and the digest selects the index it names.
        let cases: [(u16, u16, u16); 6] = [
            (0, 0, 0),
            (8_191, 1_023, 1_023),
            (5_136, zenon, kition),
            (100, 500, 700),
            (4_096, 512, 512),
            (2_498, 457, 824),
        ];

        for (a, n, p) in cases {
            let name = name_from_digest(&digest_drawing(a, n, p));
            assert_eq!(
                name.adjective, ADJECTIVES[a as usize],
                "the adjective slot did not return the word its draw selected"
            );
            assert_eq!(
                name.noun, NOUNS[n as usize],
                "the noun slot did not return the word its draw selected"
            );
            assert_eq!(
                name.place, PLACES[p as usize],
                "the place slot did not return the word its draw selected"
            );
        }
    }

    #[test]
    fn a_real_figures_canonical_citation_is_returned_like_any_other_draw() {
        // The withdrawn screen, pinned as WITHDRAWN rather than merely absent.
        //
        // The deleted denylist refused noun–place pairs that spell how a real
        // historical figure is conventionally cited. The owner's ruling is that
        // drawing such a name is a coincidence rather than a harm: the system
        // asserts nothing about a name's bearer, and the address is the
        // identity. So this asserts the pair is RETURNED.
        //
        // Pinned positively for the same reason `the_lists_carry_no_exclusion_of_any_kind`
        // is: a later pass that quietly reintroduced a refusal would otherwise
        // leave every test green. An assertion that something is absent cannot
        // fail when the thing comes back.
        let zenon = NOUNS.iter().position(|&n| n == "zenon").unwrap() as u16;
        let kition = PLACES.iter().position(|&p| p == "kition").unwrap() as u16;
        let pensive = ADJECTIVES.iter().position(|&a| a == "pensive").unwrap() as u16;

        // The indices are asserted as literals as well as looked up, so this
        // fails if a list is reordered rather than silently following the move.
        // They were produced by `examples/pin_name.rs` — digest
        // `141003f701af…` draws (5136, 1015, 431) — not read back from here.
        assert_eq!((pensive, zenon, kition), (5_136, 1_015, 431));

        let name = name_from_digest(&digest_drawing(pensive, zenon, kition));
        assert_eq!(
            name.render(),
            "pensive zenon of kition",
            "a real figure's canonical citation must draw like any other name"
        );
    }

    #[test]
    fn a_name_is_a_function_of_six_bytes_and_nothing_else() {
        // Two digests agreeing on bytes 0..6 and differing on every byte after.
        // Equal names, WHATEVER the words drawn are — so no property of the
        // drawn words feeds back into the derivation, which is what forbids a
        // filter reading its own output.
        //
        // Distinct from `the_derivation_reads_no_byte_past_its_bound` in what it
        // rules out: that one is about the BOUND, this one about the absence of
        // FEEDBACK. A scheme that read only six bytes but redrew on a refused
        // pair would pass that test and fail this one, because the redraw would
        // have to read further to redraw from anywhere.
        for (a, n, p) in [
            (0u16, 0u16, 0u16),
            (5_136, 1_015, 431),
            (8_191, 1_023, 1_023),
        ] {
            let mut x = digest_drawing(a, n, p);
            let mut y = digest_drawing(a, n, p);
            for i in NAME_DIGEST_BOUND..32 {
                x[i] = 0x00;
                y[i] = 0xff;
            }
            assert_ne!(x, y, "the fixture must actually differ past the bound");
            assert_eq!(
                name_from_digest(&x),
                name_from_digest(&y),
                "a name must be a function of bytes 0..6 alone"
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
        let short_bytes = b"too short";
        let short = display_name_from_bytes(short_bytes);
        assert!(short.is_err(), "a 9-byte key must be refused");
        assert!(
            short.unwrap_err().to_string().contains("cannot derive"),
            "the error must say a name could not be derived"
        );

        // **No name derived from truncated or padded input.** The half this
        // replaces built the zero-padded 32-byte value, called the derivation
        // and threw the result away with `let _ =`, asserting nothing — so it
        // read as covering the no-padding rule while being unable to fail for
        // it. The rule is that a short key is refused OUTRIGHT rather than
        // widened to a length that parses, so what must be asserted is that the
        // short call does not arrive at the name the padded value reaches.
        //
        // **The fixture has to be chosen, not assumed**, and that is the whole
        // difficulty of testing this rule. Zero-padding `b"too short"` produces
        // bytes that are NOT a decompressable Edwards point, so a padding
        // implementation would be refused at the parse anyway and a test built
        // on it passes while proving nothing — measured, not supposed. The
        // prefix below is one whose zero-padded form does parse, so padding is
        // a genuinely reachable route to a name and the assertion after it is
        // what closes the route rather than the parse closing it by accident.
        let padded_prefix = short_key_whose_padding_is_a_valid_key();
        let mut padded = [0u8; 32];
        padded[..padded_prefix.len()].copy_from_slice(&padded_prefix);

        let padded_name = display_name_from_bytes(&padded).expect(
            "the zero-padded value must itself be a usable key, or this test \
             proves nothing about padding",
        );

        // The short form of that same prefix must still be refused.
        assert!(
            display_name_from_bytes(&padded_prefix).is_err(),
            "a {}-byte key must be refused rather than widened",
            padded_prefix.len()
        );

        // An implementation that zero-padded a short key would return
        // `Ok(padded_name)` from the short call and fail here. `is_err()` above
        // would also catch that, but only this says WHICH name was avoided, so
        // a future implementation that refused short keys for some unrelated
        // reason while padding elsewhere is still caught.
        assert_ne!(
            display_name_from_bytes(&padded_prefix).ok(),
            Some(padded_name),
            "the short key reached the padded value's name, so the derivation \
             pads rather than refusing"
        );
    }

    /// A short byte string whose zero-padding to 32 bytes IS a valid public key.
    ///
    /// Searched rather than written down, because which prefixes have this
    /// property is a fact about Ed25519 point decompression and not something a
    /// reader can check by eye — the intuitive choice, zero-padding an ASCII
    /// string, does not have it. Cheap: valid points are dense, so the first few
    /// candidates suffice.
    fn short_key_whose_padding_is_a_valid_key() -> Vec<u8> {
        for n in 1u16..=1024 {
            let mut candidate = [0u8; 32];
            candidate[0] = (n & 0xff) as u8;
            candidate[1] = (n >> 8) as u8;
            if display_name_from_bytes(&candidate).is_ok() {
                // The first two bytes are the whole of the prefix; the rest is
                // the padding a padding implementation would have added.
                return candidate[..2].to_vec();
            }
        }
        panic!("no zero-padded prefix in the search range parsed as a key");
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
        //
        // **This is now satisfied by the return TYPE**, which is the strongest
        // form it could take: `display_name` returns a `DisplayName` and not a
        // `Result`, so "an unrecognised key is refused" is not expressible. What
        // is left to assert is that the name is a real, complete one rather than
        // an empty or partial value the type would also permit.
        let stranger = SecretKey::generate().unwrap().public_key();
        let name = display_name(&stranger);
        assert_eq!(name.words().len(), 3);
        for word in name.words() {
            assert!(!word.is_empty(), "a stranger's name has an empty slot");
        }
        assert!(ADJECTIVES.contains(&name.adjective));
        assert!(NOUNS.contains(&name.noun));
        assert!(PLACES.contains(&name.place));
    }

    #[test]
    fn a_name_carries_no_marking_of_authority() {
        // A name is rendered next to moderator badges, where a reader may read
        // the pair as one claim. Nothing in a name distinguishes a moderator's
        // from anyone else's — the rendered form is three words and a connector
        // either way, with no marker to carry standing.
        let moderator = display_name(&a_key(1).public_key());
        let participant = display_name(&a_key(2).public_key());
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
        // named historical Greeks supplies them already QUALIFIED, so an entry
        // spelled `zenon of kition` would render *pensive zenon of kition of
        // lampsakos* — two places attached to one name, leaving a reader unable
        // to tell which of them the place slot supplied.
        //
        // Both halves of that example are real entries under the kappa rule —
        // `zenon` at noun index 1015, `kition` at place index 431 — which is
        // what makes it a demonstration rather than an illustration: the
        // collision is with list contents rather than with invented words.
        //
        // (Those are ARRAY indices, not file line numbers. The literals start
        // at line 27 of `nouns.rs` and line 23 of `places.rs`, so a grep's line
        // number is the index plus that offset — a trap worth naming, because
        // reading a grep hit as an index is how a wrong index gets written
        // down confidently.)
        //
        // This is a rule about ONE LITERAL SUBSTRING and not a semantic screen:
        // what the noun means is still no part of whether it is in. The bare
        // `zenon` is in the list and draws normally — see
        // `a_real_figures_canonical_citation_is_returned_like_any_other_draw`.
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
        // `strategos` is in this loop because the comment above names it as one
        // of the five the deleted tests asserted absent, and it was pinned by
        // nothing — the exact gap this test says it closes, left open for the
        // one term the prose singled out.
        for term in [
            "stoa",
            "agora",
            "archon",
            "strategos",
            "tyrannos",
            "genesis",
        ] {
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

        let name_a = display_name(&a.public_key());
        let name_b = display_name(&b.public_key());

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

    // ─── The scheme is versioned and frozen ────────────────────────────────

    #[test]
    fn a_different_scheme_version_gives_a_different_name_for_one_key() {
        // "Names under two scheme versions SHALL be distinguishable, so that one
        // version's names cannot be silently reproduced by the other." This is
        // what a version bump BUYS, and without it the bump is bookkeeping: two
        // schemes that mint the same names for the same keys have not been
        // separated, they have only been relabelled.
        //
        // **No API is widened to reach this.** An earlier `tasks.md` claimed the
        // scenario needed `NAME_PREFIX` exposed; it does not. `mod tests` is
        // inside this module and `use super::*` already brings the private const
        // in, so the test builds a v2 separator from the shipped v1 one.
        //
        // The v2 digest is hashed HERE rather than by calling `name_digest`,
        // which is what makes this a comparison of two independent routes rather
        // than the derivation compared with itself.
        assert_eq!(
            NAME_PREFIX[12], b'1',
            "the scheme version lives at byte 12 of the separator; if it has \
             moved, this test is bumping the wrong byte and would pass while \
             comparing v1 with v1"
        );
        let mut v2_prefix = *NAME_PREFIX;
        v2_prefix[12] = b'2';
        assert_ne!(
            &v2_prefix, NAME_PREFIX,
            "the two separators must actually differ, or every comparison below \
             is one scheme compared with itself"
        );

        // Many keys rather than one: a single pair could differ by coincidence
        // of one draw, where "the two schemes are separated" is a claim about
        // every key.
        let mut differed = 0;
        for seed in 1u8..40 {
            let key = a_key(seed).public_key();

            let v2_digest: [u8; 32] = {
                let mut h = Sha256::new();
                h.update(v2_prefix);
                h.update(key.to_bytes());
                h.finalize().into()
            };
            let under_v2 = name_from_digest(&v2_digest);
            let under_v1 = display_name(&key);

            assert_ne!(
                under_v1.render(),
                under_v2.render(),
                "seed {seed} renders identically under both scheme versions, so \
                 a v1 name is silently reproducible by v2"
            );
            differed += 1;
        }
        // The loop must have run, or "every key differed" is true of no key.
        assert_eq!(differed, 39, "the sweep did not exercise every seed");
    }

    #[test]
    fn removing_a_word_renames_identities_that_drew_past_it() {
        // The mechanism a version bump exists to prevent, exhibited rather than
        // asserted. "The derivation maps digest bytes to list INDICES, so
        // removing one word reindexes the list and every identity that drew at
        // or after the removed index renders differently — on peers that have
        // updated and not on peers that have not."
        //
        // Reachable without widening anything: the lists are `const` arrays that
        // `use super::*` already brings in, so the shortened list is a `Vec`
        // built here and the reindexing is done by hand. This is a statement
        // about what a removal WOULD do, not a test that a removal happened —
        // `every_wordlist_is_pinned_entry_by_entry_and_in_order` is what catches
        // an actual removal.
        const REMOVED: usize = 100;

        let shortened: Vec<&'static str> = PLACES
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != REMOVED)
            .map(|(_, w)| *w)
            .collect();
        assert_eq!(
            shortened.len(),
            PLACES.len() - 1,
            "exactly one entry must have been removed"
        );

        // Below the removed index: nothing moves. This half is what makes the
        // test a statement about REINDEXING rather than about the list simply
        // being different — a test asserting only that names changed would pass
        // on a shortened list that had been shuffled.
        let mut unchanged = 0;
        for i in 0..REMOVED {
            assert_eq!(
                shortened[i], PLACES[i],
                "index {i} is below the removal and must be untouched"
            );
            unchanged += 1;
        }
        assert_eq!(unchanged, REMOVED, "the below-the-cut sweep did not run");

        // At and above it: every index now names the word that used to sit one
        // place later, so an identity that drew index i renders as a DIFFERENT
        // place than it did before.
        //
        // Compared against `PLACES[i + 1]` — a written-down relation — rather
        // than merely asserting inequality: `shortened[i] != PLACES[i]` alone
        // would pass on any reshuffling, where the spec's claim is the specific
        // one that everything shifts down by exactly one.
        let mut renamed = 0;
        for i in REMOVED..shortened.len() {
            assert_eq!(
                shortened[i],
                PLACES[i + 1],
                "index {i} must now hold what index {} held",
                i + 1
            );
            renamed += 1;
        }
        assert_eq!(
            renamed,
            PLACES.len() - 1 - REMOVED,
            "the above-the-cut sweep did not run"
        );

        // And the renaming is real rather than nominal: the words genuinely
        // differ, so an identity drawing here would render as someone else.
        // Asserted over the whole tail rather than at one index, because a
        // single adjacent duplicate in the list would make a one-index check
        // pass while the property failed there.
        let mut moved = 0;
        for i in REMOVED..shortened.len() {
            if shortened[i] != PLACES[i] {
                moved += 1;
            }
        }
        assert_eq!(
            moved,
            shortened.len() - REMOVED,
            "some index at or above the removal renders the same word as before, \
             so the list holds an adjacent duplicate and the reindexing is \
             invisible there — which `no_list_holds_a_duplicate` should have \
             caught"
        );
    }
}
