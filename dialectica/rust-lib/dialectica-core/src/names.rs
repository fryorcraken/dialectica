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
//! # Derived from the PUBLIC KEY'S OWN BYTES, with no hash in between
//!
//! The three draws read key bytes `18..23` directly. There is no digest, no
//! domain separator and no preimage: `adjective = ADJECTIVES[be16(key[18..20]) %
//! 8192]`, and so on for the noun and the place.
//!
//! **This replaced `H(NAME_PREFIX || public_key)`, and the reason is that the
//! separator had nothing left to separate the name FROM.** A separator exists to
//! make two derivations over one key independent functions of it. The
//! counterpart was the author address, `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 ||
//! public_key)`; issue #80 deletes it, so there is one derivation over the key
//! and the separator separated the name from nothing.
//!
//! **What this costs is stated here because nothing else records it.** With no
//! preimage there is nowhere to put a scheme version, so a wordlist can never
//! change without every name changing at once and no property of either name
//! saying which scheme produced it. The owner took this deliberately: a name is
//! a pure local function, never published and recomputed wherever it is shown,
//! so two peers on different builds rendering different names is a client-side
//! rendering difference rather than a disagreement about anything being agreed
//! on. It is a one-way door all the same.
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
//! filed as its own piece — `key-identity-sweep` — and until it lands the feed
//! path cannot render a name. What forbids the wrong remedy in the meantime is
//! the `generated-names` spec's own requirement that **no reply carries a
//! display name**, which is the authority here. When the feed path can render a
//! name, *What a name is NOT* below governs what may be claimed of it: a name is
//! never unique and never an identifier, so it recognises rather than
//! distinguishes.
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
//! a different *public key*; they cannot forge the key and cannot forge a
//! signature, so nothing they publish is attributable to the identity they
//! imitate. The attack is purely social and the public key is what defeats it.
//!
//! **No method accepts one.** Not a lookup, not a moderation target, not a vote
//! target. The public key is the identity.
//!
//! # The byte allocation, and why disjointness is now load-bearing
//!
//! Three channels tell identities apart — this name, the mark, and the
//! abbreviated key on screen — and under this scheme **all three read the same
//! 32 bytes**. The `generated-names` spec allocates them pairwise disjoint
//! ranges:
//!
//! | channel | key bytes |
//! |---|---|
//! | abbreviation — head | `0..3` |
//! | mark | `4..11` |
//! | abbreviation — middle | `14..17` |
//! | **name** | **`18..23`** |
//! | abbreviation — tail | `29..31` |
//!
//! Bytes `12..13` and `24..28` are read by no channel. They are **unallocated,
//! not reserved**: nothing depends on their value and no channel may be extended
//! onto them without the spec changing.
//!
//! **This is the mechanism three documents in this tree once correctly called
//! fictional, made real.** While the name hashed the key under its own separator
//! and the mark read an address — a different digest — the two could not
//! overlap, so byte allocation between them was neither required nor possible,
//! and any claim of a "reserved range" described nothing. That reasoning is
//! withdrawn: with one shared value and no hash anywhere, two channels reading
//! one byte are two searches that partly coincide, and byte-disjointness is the
//! only thing making name-grinding and mark-grinding costs multiply rather than
//! add. A byte the abbreviation **displays** is worse than merely shared — an
//! attacker reads their progress off the screen while grinding it.
//!
//! Only this crate's half of that is enforceable here: the mark and the
//! abbreviation are QML, so `tst_identicon.qml` is where the pairwise check
//! lives, and what this crate can check is that the name's measured window is
//! the allocated one and overlaps neither of the other two ranges.

// **No hash is imported here, and that absence is the change.** The derivation
// reads key bytes directly; `sha2` appears below only inside `mod tests`, where
// it hashes the WORDLISTS to pin them entry by entry — a check on the lists'
// contents, not a step in deriving a name.
use crate::identity::{KeyError, PublicKey};

mod adjectives;
mod nouns;
mod places;

pub use adjectives::ADJECTIVES;
pub use nouns::NOUNS;
pub use places::PLACES;

/// The literal text between the noun and the place.
///
/// **Fixed text, emitted unconditionally, reading no hash bytes.** It is not a
/// slot — the natural reading of "three words plus a connector" is that there
/// are four, and there are three. It carries no entropy and never varies with
/// the key, which is exactly what makes it the one part of a name a cramped
/// caller may drop: *pensive aporia lampsakos* has identical information
/// content and misleads no reader about who published something.
pub const CONNECTOR: &str = "of";

/// The public key's bytes the name reads: `18..24`, six of them.
///
/// **One range rather than a start and a length**, and that is a decision rather
/// than a style. Two constants can disagree — a start moved without its length
/// is a window that has silently slid onto a neighbouring byte, which produces a
/// different name for every key with nothing else changed and nothing in any
/// output to show it. A single range cannot be half-moved.
///
/// **It is an allocation, not merely an offset.** The spec gives the mark
/// `4..11` and the abbreviation `0..3`, `14..17` and `29..31`; this range is
/// pairwise disjoint from both, which under a scheme with no hash between the
/// key and any channel is the whole of what keeps the three searches
/// independent. Moving it is a change to that allocation and not a local edit.
///
/// **It bounds by being what the derivation slices, not by being asserted.** An
/// earlier version of the bound it replaces was compared to its own literal in a
/// `debug_assert_eq!` — a tautology that compiled out in release and could not
/// fail under any edit to the draws. [`name_from_key_bytes`] takes its six bytes
/// as `key_bytes[name_key_bytes()]`, so moving the range moves the read: there is
/// no second place holding a copy of the window that could disagree with this
/// one.
///
/// **What that gives is a structural invariant, not a compile error**, and this
/// comment claimed the stronger thing until it was measured. Two things hold and
/// two do not:
///
/// - *Holds:* the two constants cannot disagree, because [`NAME_BYTE_COUNT`] is
///   derived from this range rather than written beside it, and the derivation
///   reads one slice rather than indexing the key directly.
/// - *Does not hold:* "a draw past the range does not compile". `word(i)` indexes
///   the slice through a closure, so `i` is a runtime value — `word(4)` to
///   `word(5)` **compiles** and panics with `index out of bounds: the len is 6
///   but the index is 6`. Widening this range to `18..25` **also compiles**, and
///   fails two tests at runtime rather than at the build.
///
/// So the enforcement is test-caught, and the tests that catch it are
/// `the_restated_channels_are_the_spec_s_byte_sets_and_not_merely_disjoint_ones`
/// and `the_spec_allocation_this_crate_restates_is_internally_consistent`.
/// Weakening those weakens this.
///
/// The number of bytes a name consumes is therefore fixed rather than
/// data-dependent: no input makes the derivation read a seventh. Reading without
/// bound is what lets two implementations disagree about how far to read and so
/// produce different names for one key, which is the failure this whole scheme
/// exists to prevent.
/// **A function rather than a `const Range`**, and that is not a style choice.
/// A `const` of a non-`Copy` type is re-materialised as a fresh temporary at
/// every use, so `NAME_KEY_BYTES.any(..)` mutates a temporary and `rustc` warns
/// about it by default (`const_item_mutation`). A function hands out a genuine
/// range each call, and iterating it needs no `.clone()` scattered at the call
/// sites — which is the shape that would have invited someone to "simplify" the
/// clone away and silently iterate nothing.
const fn name_key_bytes() -> std::ops::Range<usize> {
    18..24
}

/// How many bytes [`name_key_bytes`] spans: three 16-bit draws.
///
/// Derived from the range rather than written beside it, so the two cannot
/// disagree. This is the array length the slice is converted into, which is what
/// makes a widened range a compile error instead of a runtime surprise.
const NAME_BYTE_COUNT: usize = name_key_bytes().end - name_key_bytes().start;

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
/// three unconditional reductions over six key bytes, with nothing that can
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
    name_from_key_bytes(&key.to_bytes())
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

/// Turn a 32-byte public key value into three words.
///
/// **Public, and separately callable from [`display_name`], because the spec
/// makes that a testability obligation rather than a convenience.** Reaching a
/// chosen slot combination through a chosen *key* means grinding for one — a
/// `PublicKey` is a curve point and its bytes cannot be chosen — so the pinning
/// and uniformity requirements are checkable only if the 32 bytes can be
/// supplied directly.
///
/// **This takes raw bytes rather than a [`PublicKey`] for that reason alone**,
/// and it is not a widening of what callers should use: production callers hold
/// a key and call [`display_name`] or [`display_name_from_bytes`], both of which
/// parse. Handing arbitrary bytes here produces a name for a value that may be
/// no key at all, which is exactly what the pinning tests need and exactly what
/// a caller rendering an author must not do.
///
/// **It replaced a `name_from_digest` taking the first six bytes of
/// `SHA256(NAME_PREFIX || key)`.** The rename is not cosmetic: the parameter
/// stopped being a digest, and a function still called `…_from_digest` invites a
/// caller to hand it one — which would silently produce a name for the wrong
/// value, since a digest is 32 bytes too and nothing would refuse it.
///
/// **Total, and infallible by construction.** Every draw is a single
/// unconditional reduction: nothing refuses a combination, nothing retries, and
/// there is no budget to exhaust. So the bytes a name consumes are fixed rather
/// than data-dependent, every index of every list is reachable, and the `2^33`
/// space is reached exactly rather than approximately.
///
/// # The byte budget
///
/// | key bytes | slot |
/// |---|---|
/// | `18..20` | adjective |
/// | `20..22` | noun |
/// | `22..24` | place |
///
/// Every other byte of the key is **never read** — including bytes `12..13` and
/// `24..28`, which no channel reads and which are unallocated rather than
/// reserved. The slice below is taken at [`name_key_bytes`] rather than indexed
/// relative to it, so the range is what the derivation reads rather than a
/// figure a comment asserts.
///
/// **What enforces that is a runtime bound, not the compiler**, and an earlier
/// version of this sentence claimed otherwise. `word(i)` indexes `drawn` through
/// a closure, so `i` is a runtime value: changing `word(4)` to `word(5)`
/// **compiles**, and panics at the draw with `index out of bounds: the len is 6
/// but the index is 6` — measured. The tests are what catch it, so a reader who
/// believes rustc is holding this will not notice if those tests are weakened.
///
/// Each slot takes its index from bytes no other slot reads, so the three words
/// are independent draws rather than three views of the same bits.
pub fn name_from_key_bytes(key_bytes: &[u8; 32]) -> DisplayName {
    // The window, applied rather than asserted. Everything below reads from this
    // slice, so there is no path that reaches a byte outside it.
    let drawn: &[u8; NAME_BYTE_COUNT] = key_bytes[name_key_bytes()]
        .try_into()
        .expect("name_key_bytes() lies inside a 32-byte key and spans NAME_BYTE_COUNT");

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
/// implementation**, and that is their entire purpose. Change one entry of a
/// list, the order of two entries, which bytes a slot reads, or the connector,
/// and every peer's names change together with no error anywhere — each peer
/// stays internally consistent while agreeing with nobody. A check that asks the
/// implementation what it produced and agrees with the answer cannot see that.
/// These were produced independently, by `examples/pin_name.rs`, which reads the
/// wordlists from the text files in `wordlists/` and does the index arithmetic
/// itself rather than calling [`name_from_key_bytes`] — it does not link the
/// derivation at all.
///
/// **If one of these fails, do not update it to match.** Work out what changed
/// and whether the network can survive it.
#[cfg(test)]
pub mod tests_support {
    /// Two distinct secret-key seeds whose keys derive the SAME display name.
    ///
    /// **Found by search, not constructed by stubbing the derivation**, so the
    /// collision is a real property of the shipped scheme and wordlists rather
    /// than of a test double. The search walked seeds with a counter in the
    /// first four bytes and indexed by rendered name until one repeated; at a
    /// space of 2^33 a repeat arrives after roughly 2^16.5 keys, and this pair
    /// turned up inside the first 150,000 seeds tried.
    ///
    /// **This pair replaces an earlier one that stopped colliding**, and the
    /// reason is exactly the scheme change the doc below warns about: issue #80
    /// moved the derivation from `H(NAME_PREFIX || key)` to raw key bytes
    /// `18..23`, so which keys collide is an entirely different question. The old
    /// pair was not papered over — it was re-searched because the derivation it
    /// was found against no longer exists.
    ///
    /// Both keys' names were verified through `examples/pin_name.rs`, which
    /// reads the wordlists from the text files and does not link the derivation:
    /// each reaches indices `(2628, 768, 971)`. So the collision is a fact about
    /// the scheme, not about the function that found it.
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
        s[0] = 0x1e;
        s[1] = 0xd0;
        s[2] = 0x01;
        s
    };
    pub const COLLIDING_SEED_B: [u8; 32] = {
        let mut s = [0u8; 32];
        s[0] = 0xd1;
        s[1] = 0x03;
        s[2] = 0x02;
        s
    };
    /// The name both of the seeds above derive.
    pub const COLLIDING_NAME: &str = "fearable pindaros of thorai";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;
    // Only the tests hash anything. The derivation itself imports no hash — it
    // reads key bytes — and this is used to pin the WORDLISTS entry by entry,
    // which is a check on the lists rather than a step in deriving a name.
    use sha2::{Digest, Sha256};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    // ─── The pins: the only tests that can see a silent consensus change ───

    #[test]
    fn the_name_scheme_is_pinned_to_known_answers() {
        // EVERY constant this scheme rests on is consensus-critical: move the
        // name's byte window, change one entry of a list, or exchange the order
        // of two entries, and this peer's names stop matching every other
        // peer's — with no error anywhere, because each peer remains internally
        // consistent.
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
        // **TWO CASES, chosen so that the bytes OUTSIDE the window differ
        // between them**, which the spec requires for a reason specific to this
        // scheme: with the name reading the key directly, a slot reading bytes
        // `16..17` instead of `18..19` produces a different name from the same
        // key with nothing else changed, and the window is not otherwise visible
        // in any output. Two keys whose out-of-window bytes coincided would let
        // a slipped window read the same values and pass.
        //
        // If this fails, do NOT update the expected values to match. Work out
        // what changed and whether the network can survive it.
        for (seed, key_hex, expected) in PINNED_CASES {
            let key = a_key(seed).public_key();

            // The key hex is pinned too, and it is not decoration: it is the
            // input `examples/pin_name.rs` was given. Without it a reader cannot
            // re-run the pin, and the name below would be a value to believe
            // rather than one to check.
            assert_eq!(
                hex::encode(key.to_bytes()),
                key_hex,
                "the public key for seed {seed} is not the one the pin was \
                 computed from, so the expected name below is about a \
                 different key"
            );

            assert_eq!(
                display_name(&key).render(),
                expected,
                "the name derivation changed for seed {seed}"
            );
        }
    }

    /// Pinned names, with the public key each was computed from.
    ///
    /// `(secret-key seed byte, public key hex, expected name)`. Produced by
    /// `examples/pin_name.rs`, which reads the wordlists from the TEXT FILES in
    /// `wordlists/` and does the index arithmetic itself — it does not link the
    /// derivation at all.
    ///
    /// **The two keys differ outside the name's window**, which is what makes a
    /// slipped window visible: see the reasoning in the test above.
    const PINNED_CASES: [(u8, &str, &str); 2] = [
        (
            7,
            "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c",
            "quartzous paris of sypalettos",
        ),
        (
            11,
            "66be7e332c7a453332bd9d0a7f7db055f5c5ef1a06ada66d98b39fb6810c473a",
            "inpardonable paidagogos of myriandros",
        ),
    ];

    #[test]
    fn a_pinned_name_is_reproducible_from_its_key_by_hand() {
        // The pins above are only independent if they can be reached WITHOUT the
        // function under test. This does the index arithmetic here, from the
        // KEY's own bytes, and indexes the lists directly — so it fails if
        // `name_from_key_bytes` reads different bytes, reduces differently,
        // orders the slots differently, or emits a different connector.
        //
        // The three routes that must agree: `examples/pin_name.rs` reading the
        // text files, this arithmetic, and `display_name`.
        for (seed, key_hex, expected) in PINNED_CASES {
            let key = hex::decode(key_hex).expect("the pinned key is hex");

            // Key bytes 18..20, 20..22, 22..24, big-endian, reduced into each
            // list. The offsets are written out rather than taken from
            // name_key_bytes(), so a moved window fails here instead of following
            // the move — which is the whole difference between a pin and a
            // restatement.
            let adjective_index = u16::from_be_bytes([key[18], key[19]]) % 8_192;
            let noun_index = u16::from_be_bytes([key[20], key[21]]) % 1_024;
            let place_index = u16::from_be_bytes([key[22], key[23]]) % 1_024;

            assert_eq!(
                format!(
                    "{} {} {CONNECTOR} {}",
                    ADJECTIVES[adjective_index as usize],
                    NOUNS[noun_index as usize],
                    PLACES[place_index as usize]
                ),
                expected,
                "seed {seed}: the hand arithmetic does not reach the pinned name"
            );
        }
    }

    #[test]
    fn the_pinned_cases_differ_outside_the_name_window() {
        // The precondition the spec states for the pins, asserted rather than
        // assumed. If the two pinned keys agreed on the bytes around the window,
        // a slot that had slipped onto a neighbouring byte would read the same
        // value in both and both pins would still pass — the pins would be
        // green about a window that had moved.
        //
        // Checked at the bytes a ONE-BYTE SLIP in either direction would reach:
        // 16 and 17 below the window, 24 and 25 above it. Those are the values a
        // slipped draw would actually read, so this is a statement about the
        // failure mode rather than a general "the keys differ".
        let a = hex::decode(PINNED_CASES[0].1).expect("hex");
        let b = hex::decode(PINNED_CASES[1].1).expect("hex");

        let mut differing = 0;
        for i in [16usize, 17, 24, 25] {
            if a[i] != b[i] {
                differing += 1;
            }
        }
        assert!(
            differing > 0,
            "the two pinned keys agree on every byte a one-byte window slip \
             would read, so a slipped window would pass both pins"
        );
    }

    #[test]
    fn some_pinned_case_is_byte_order_sensitive_in_every_slot() {
        // A second precondition on the pins, and it was found by mutation rather
        // than reasoned about in advance.
        //
        // A slot whose two key bytes are EQUAL reads the same 16-bit value
        // big-endian or little-endian, so no pin drawn from that key can fail on
        // a byte-order change in that slot. `PINNED_CASES[0]` has exactly that
        // property in its noun slot — key bytes 20 and 21 are both `0xbe` — and
        // it is measurable: reducing the noun slot with `u16::from_le_bytes`
        // leaves `the_name_scheme_is_pinned_to_known_answers` failing on seed 11
        // ALONE, with seed 7 green. Had seed 11 not been here, a wrong byte order
        // would have been a silent consensus change.
        //
        // (The same coincidence was live in the QML gate, which pinned only the
        // first case: a wrong-order noun slot in `DKeyNameWindow` passed that
        // whole file. `tst_identicon.qml` now pins both cases and asserts this
        // same precondition.)
        //
        // Stated as "some case is sensitive in every slot" rather than "every
        // case is": the first case earns its place by being the one
        // `a_pinned_name_is_reproducible_from_its_key_by_hand` reproduces, and
        // its noun coincidence is a fact about that key rather than a defect.
        let any_fully_sensitive = PINNED_CASES.iter().any(|(_, key_hex, _)| {
            let key = hex::decode(key_hex).expect("the pinned key is hex");
            // The three slots start at key bytes 18, 20 and 22.
            [18usize, 20, 22].iter().all(|&s| key[s] != key[s + 1])
        });
        assert!(
            any_fully_sensitive,
            "no pinned case holds two DIFFERENT bytes in all three slots, so a \
             slot reducing its two bytes in the wrong order would reach the same \
             index in every pinned case and every name pin would stay green \
             while this peer's names stopped matching every other peer's"
        );
    }

    #[test]
    fn the_pinned_cases_span_each_list_rather_than_clustering() {
        // The spec requires pinned cases to reach **a low and a high index in
        // each of the three slots**, so that a pin is evidence about the index
        // arithmetic and not only about one region of one list. A pin drawn from
        // a key reaches whatever index that key's own bytes happen to select,
        // and no key can be chosen to land on a wanted index without grinding
        // for one — a public key is a curve point, so its bytes are not free to
        // pick. So the span is reached through CONSTRUCTED 32-BYTE VALUES, which
        // is the testability seam `name_from_key_bytes` is public for.
        //
        // This replaces a pin on the redraw path. There is no redraw to pin:
        // every draw is now one unconditional reduction, so the only thing a
        // second pinned case can add is coverage of the index arithmetic at the
        // ends of each list — which is what the spec asks for and what the old
        // reserve pin, clustered at index 7 of all three lists, did not give.
        //
        // Both names below are WRITTEN DOWN, produced by `examples/pin_name.rs`
        // reading the text files, not read back from `name_from_key_bytes`.
        //
        // Index 0 of each list. A window of six zero bytes draws (0, 0, 0)
        // because `0 % n == 0` for every n, so this fixture needs no arithmetic
        // to justify the indices it claims.
        let low = name_from_key_bytes(&key_bytes_drawing(0, 0, 0));
        assert_eq!(low.render(), PINNED_NAME_AT_LOW_INDICES);

        // The last index of each list: 8,191 and 1,023. Reached by drawing the
        // 16-bit value equal to the index itself, which is below every list's
        // length and so survives the reduction unchanged.
        let high = name_from_key_bytes(&key_bytes_drawing(8_191, 1_023, 1_023));
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
        // change is a scheme change, and there is no longer a version to bump:
        // the name reads raw key bytes, so there is no preimage a scheme version
        // could sit in and no way to tell two schemes' names apart. Every
        // identity renames at once. That is a migration to decide on, not a
        // constant to edit here.
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

    // ─── The byte allocation: this crate's half ────────────────────────────
    //
    // **This replaces a domain-separation test, and the replacement is not a
    // like-for-like.** The deleted test asserted that the name's digest was not
    // the address, not a bare hash of the key, and not the key under any other
    // capability's separator — the mechanism that made the name independent of
    // the mark while the two read different digests. There is no digest now, so
    // there is nothing left for it to assert: the name reads the key's own
    // bytes, and so does every other channel.
    //
    // What replaces it is the property that now carries the same weight. The
    // three channels share one 32-byte space, so byte-disjointness is the only
    // thing making name-grinding and mark-grinding costs multiply rather than
    // add — and a byte the abbreviation DISPLAYS is worse than merely shared,
    // because an attacker reads their progress off the screen while grinding it.
    //
    // **Only one of the three channels is in this crate.** The mark and the
    // abbreviation are QML, so the real pairwise check is
    // `dialectica-ui/tests/tst_identicon.qml`, which measures all three by
    // probing the components. What is checkable here is the name's side: that
    // its measured window is the range the spec allocates, and that the range
    // overlaps neither of the other two allocations.
    //
    // The other two ranges are therefore WRITTEN DOWN below, as the spec's
    // figures. That is a restatement of the SPEC, not of another implementation
    // — which is the distinction that matters, and the reason this is not the
    // "computed version that cannot fail" that was deleted from the QML gate.
    // Were these read out of `Identicon.qml` somehow, a change there would drag
    // this along with it and the check would be vacuous.

    /// The key bytes the mark reads, per the `generated-names` spec.
    const SPEC_MARK_BYTES: std::ops::Range<usize> = 4..12;
    /// The key bytes the abbreviation displays, per the spec: three groups.
    const SPEC_ABBREVIATION_BYTES: [std::ops::Range<usize>; 3] = [0..4, 14..18, 29..32];

    /// Which key byte indices actually move the name, MEASURED.
    ///
    /// Varying one byte at a time and watching the output, rather than reading
    /// `name_key_bytes()` — which would make every test below a restatement of the
    /// constant they exist to check.
    ///
    /// # Why every value, and not a list of interesting ones
    ///
    /// This probe used to try seven hand-picked values per byte. That was chosen
    /// to defeat a coincidence — a reduction can map two different byte values to
    /// the same index, and `0x00 % 3 == 0xff % 3` had already made a one-value
    /// probe report the mark as not reading byte 11 — and for *unconditional*
    /// reads seven spread values are enough.
    ///
    /// **They are not enough for a conditional one, and that is a different
    /// failure.** A derivation that reads an unallocated byte only when it holds
    /// one particular value is invisible to any probe whose value list omits that
    /// value, and the tests below then certify the allocation clean. Measured:
    /// adding `^ if key_bytes[12] == 0x42 { 1 } else { 0 }` to the adjective
    /// reduction left all 38 tests in this module green under the seven-value
    /// probe, `the_name_reads_no_unallocated_byte` among them.
    ///
    /// **The fix is not a longer list.** Appending `0x42` to the seven reproduces
    /// the defect at `0x43`; a probe that enumerates values will always miss the
    /// value it does not enumerate, which is this repo's `hand-maintained sweep
    /// lists go stale silently` trap wearing a different hat. Every value is the
    /// only list that cannot be one short. It costs 32 × 256 = 8,192 derivations,
    /// which is nothing.
    ///
    /// # What this now proves, stated exactly
    ///
    /// For each byte `b`, the sweep decides "is the name a constant function of
    /// `b`, with every other byte held at zero?" — over the byte's **entire**
    /// domain, so no value of `b` alone can hide a read. What it still cannot see
    /// is a read gated on **two or more** bytes at once (`key[12] == 0x42 &&
    /// key[13] == 0x99`), which is invisible to any one-byte-at-a-time sweep from
    /// a fixed base regardless of how many values it tries. Closing that would
    /// need 256^2 pairs per pair of bytes and is not what is done here.
    ///
    /// What bounds that residue is **structural rather than another test**, and
    /// the distinction is worth being exact about because the neighbouring
    /// `the_derivation_reads_no_byte_outside_its_window` looks like it closes the
    /// gap and does not: it compares one `0x00`-outside fixture against one
    /// `0xff`-outside fixture, so a `key[12] == 0x42` carve-out passes it too.
    ///
    /// The real guarantee is that [`name_from_key_bytes`] binds
    /// `key_bytes[name_key_bytes()]` once and every draw reads that slice, so an
    /// expression reading byte 12 has to be *written in*, and writing it in is the
    /// single-line diff a reviewer sees. A sweep cannot make that unexpressible;
    /// what it does is measure, independently of the constant, that the slice is
    /// where the reading actually happens — which is what stops these tests from
    /// being a restatement of `name_key_bytes()`.
    fn measured_name_bytes() -> Vec<usize> {
        let base = [0u8; 32];
        let reference = name_from_key_bytes(&base);
        let mut read = Vec::new();
        for b in 0..32 {
            for probe in 0..=u8::MAX {
                let mut varied = base;
                varied[b] = probe;
                if name_from_key_bytes(&varied) != reference {
                    read.push(b);
                    break;
                }
            }
        }
        read
    }

    #[test]
    fn the_name_reads_exactly_the_bytes_the_spec_allocates_to_it() {
        // The measurement pinned against the spec's figure, which is also what
        // makes `measured_name_bytes` trustworthy for the disjointness test
        // below: a probe that had silently stopped detecting bytes would report
        // an empty set, and an empty set is disjoint from everything.
        assert_eq!(
            measured_name_bytes(),
            vec![18, 19, 20, 21, 22, 23],
            "the name's measured window is not the 18..23 the spec allocates"
        );
    }

    #[test]
    fn the_names_bytes_overlap_neither_of_the_other_two_channels() {
        // Pairwise, and against the spec's allocations for the channels this
        // crate cannot see. Stated pairwise rather than as a union count
        // because a union count passes when a set measures empty — the
        // two-explanations-one-answer shape this repo keeps finding.
        let read = measured_name_bytes();
        assert!(
            !read.is_empty(),
            "the name reads no byte at all, so 'disjoint' would be satisfied \
             by a measurement that found nothing"
        );

        for byte in &read {
            assert!(
                !SPEC_MARK_BYTES.contains(byte),
                "key byte {byte} is read by both the name and the mark, so \
                 grinding for a lookalike name partly grinds for a lookalike \
                 mark and the costs add instead of multiplying"
            );
            for group in SPEC_ABBREVIATION_BYTES {
                assert!(
                    !group.contains(byte),
                    "key byte {byte} is read by the name and DISPLAYED by the \
                     abbreviation — an attacker grinding a lookalike name reads \
                     their progress off the rendered key"
                );
            }
        }
    }

    #[test]
    fn the_name_reads_no_unallocated_byte() {
        // Bytes 12..13 and 24..28 are read by NO channel. They are unallocated
        // rather than reserved: nothing depends on their value, and this asserts
        // only that the name is not one of the things reading them. Extending
        // any channel onto them is a spec change.
        let read = measured_name_bytes();
        for byte in [12usize, 13, 24, 25, 26, 27, 28] {
            assert!(
                !read.contains(&byte),
                "key byte {byte} is unallocated but the name reads it"
            );
        }
    }

    #[test]
    fn the_spec_allocation_this_crate_restates_is_internally_consistent() {
        // The two ranges above are written down, so nothing stops them being
        // written down wrong — and a wrong MARK range would make the
        // disjointness test pass while the real property failed. This checks the
        // restatement against the one arithmetic fact the spec's table carries:
        // the five allocations are pairwise disjoint and total 25 of 32 bytes.
        //
        // It cannot check that the figures match `Identicon.qml` — nothing in
        // this crate can, which is why the QML gate exists and why this test is
        // named for what it actually covers.
        let mut allocated: Vec<usize> = SPEC_MARK_BYTES.collect();
        for group in SPEC_ABBREVIATION_BYTES {
            allocated.extend(group);
        }
        allocated.extend(name_key_bytes());

        let before = allocated.len();
        allocated.sort_unstable();
        allocated.dedup();
        assert_eq!(
            before,
            allocated.len(),
            "the restated allocation overlaps itself, so the disjointness \
             assertions above are checking against a contradictory table"
        );
        assert_eq!(
            allocated.len(),
            25,
            "the spec's allocation totals 25 of 32 bytes; this restatement \
             does not"
        );
        assert!(
            allocated.iter().all(|b| *b < 32),
            "an allocation names a byte outside a 32-byte key"
        );
    }

    #[test]
    fn the_restated_channels_are_the_spec_s_byte_sets_and_not_merely_disjoint_ones() {
        // **Internal consistency is not enough, and this was measured rather
        // than reasoned about.** `the_spec_allocation_this_crate_restates_is_internally_consistent`
        // above asks only that the five ranges are pairwise disjoint and total
        // 25 — a shape many wrong tables also have. Moving the restated mark
        // range from `4..12` to `6..14` keeps it disjoint from `0..4`, `14..18`,
        // `18..24` and `29..32` and keeps the total at 25, and every test in
        // this file stayed green under it. The disjointness assertions would
        // then have been checking the name against a mark window the mark does
        // not have: still green, and no longer about the shipped allocation.
        //
        // So the restatement is pinned to the spec's table ITSELF, byte set by
        // byte set. The literals below are the `generated-names` requirement
        // *The three channels read pairwise disjoint bytes of the public key*,
        // transcribed — the spec's figures, not another implementation's, which
        // is the distinction that keeps this from being a restatement of a
        // restatement.
        //
        // This still cannot see `Identicon.qml`; nothing in this crate can. What
        // it closes is the drift between this crate's copy of the table and the
        // table, which is a different failure from the one the QML gate covers
        // and was uncovered by either.
        assert_eq!(
            SPEC_MARK_BYTES.collect::<Vec<usize>>(),
            vec![4, 5, 6, 7, 8, 9, 10, 11],
            "the restated MARK window is not the spec's `4..11`, so the \
             name-versus-mark disjointness assertion is checking the wrong range"
        );
        assert_eq!(
            SPEC_ABBREVIATION_BYTES
                .iter()
                .flat_map(|g| g.clone())
                .collect::<Vec<usize>>(),
            vec![0, 1, 2, 3, 14, 15, 16, 17, 29, 30, 31],
            "the restated ABBREVIATION groups are not the spec's `0..3`, \
             `14..17` and `29..31`, so the displayed-byte assertion is checking \
             the wrong bytes — and a displayed byte reaching the name is the \
             severe case the spec singles out"
        );
        assert_eq!(
            name_key_bytes().collect::<Vec<usize>>(),
            vec![18, 19, 20, 21, 22, 23],
            "the name's window is not the spec's `18..23`"
        );

        // The seven the spec leaves unallocated, as the complement rather than
        // as a fourth literal — so this cannot agree with a table that named
        // 25 bytes while leaving a different seven over.
        let allocated: Vec<usize> = SPEC_MARK_BYTES
            .chain(SPEC_ABBREVIATION_BYTES.iter().flat_map(|g| g.clone()))
            .chain(name_key_bytes())
            .collect();
        let unallocated: Vec<usize> = (0..32usize).filter(|b| !allocated.contains(b)).collect();
        assert_eq!(
            unallocated,
            vec![12, 13, 24, 25, 26, 27, 28],
            "the bytes this table leaves over are not the spec's unallocated \
             `12..13` and `24..28`"
        );
    }

    #[test]
    fn no_other_32_byte_value_travelling_beside_a_key_reaches_its_name() {
        // A caller holding something other than the key cannot arrive at the
        // right name, which is why core must return the key.
        //
        // **This was written against the AUTHOR ADDRESS and is deliberately not
        // written against it now.** Issue #80 deletes that value, so a test
        // feeding `key.address()` here would go on passing while being about
        // nothing that exists — green, and no longer evidence of anything. The
        // property it was pinning survives the value that prompted it: a name is
        // a function of THE KEY and of no other 32-byte value that travels
        // beside it.
        //
        // So the fixtures are values that genuinely do travel beside a key and
        // are genuinely 32 bytes: a Stoa address (which #80 keeps), the key's
        // own bytes with the name's window zeroed, and the key's bytes reversed.
        // Each is a plausible confusion for a caller holding the wrong thing.
        let key = a_key(11).public_key();
        let real = display_name(&key);
        let key_bytes = key.to_bytes();

        let stoa = a_stoa_address_beside_this_key(&key);

        let mut window_zeroed = key_bytes;
        for i in name_key_bytes() {
            window_zeroed[i] = 0;
        }

        let mut reversed = key_bytes;
        reversed.reverse();

        for (which, other) in [
            ("a stoa address", stoa),
            ("the key with the name's window zeroed", window_zeroed),
            ("the key's bytes reversed", reversed),
        ] {
            // The fixture must actually differ from the key inside the window,
            // or "a different value reaches a different name" is being asked of
            // a value that is the key as far as the derivation can tell. This is
            // what stops the test passing for the wrong reason.
            let differs_in_window = name_key_bytes().any(|i| other[i] != key_bytes[i]);
            assert!(
                differs_in_window,
                "{which} agrees with the key on every byte the name reads, so \
                 it could not possibly reach a different name and this case \
                 proves nothing"
            );
            assert_ne!(
                real,
                name_from_key_bytes(&other),
                "{which} reached the same name as the key it travels beside"
            );
        }
    }

    /// A Stoa address to stand beside a key, as a bare 32-byte value.
    ///
    /// **Stoa addresses are untouched by issue #80** — it deletes the *author*
    /// address only — so this is a value that really does travel next to a key
    /// and really could be handed to a derivation by a confused caller. Derived
    /// from the key's own bytes so the case is not trivially satisfied by two
    /// unrelated random values.
    fn a_stoa_address_beside_this_key(key: &PublicKey) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"/dialectica/1/Address/Stoa\0\0\0\0\0\0");
        h.update(key.to_bytes());
        h.finalize().into()
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
    fn displaying_a_name_gives_the_same_text_as_rendering_it() {
        // **The `Display` impl had no test at all**, and `cargo mutants` found
        // it: replacing `fmt`'s body with `Ok(())` left 962 of 962 tests green,
        // the single survivor of 26 mutants. Every assertion in this file reached
        // a name through `.render()`, so a caller reaching one through `{}` or
        // `.to_string()` would have displayed the empty string with nothing able
        // to notice.
        //
        // That is the "name attributable to nobody" that
        // `a_failure_is_never_reported_as_a_name` exists to forbid, arrived at
        // through the formatting impl rather than through the error path. It is
        // latent today only because no production caller reaches the derivation
        // at all; it goes live with `key-identity-sweep`, which is exactly the
        // wrong moment to discover it.
        //
        // Both routes are asserted, because `to_string()` goes through `Display`
        // while `format!("{}")` is the one a renderer is likelier to write, and
        // an impl could in principle satisfy one and not the other.
        for seed in 1u8..30 {
            let name = display_name(&a_key(seed).public_key());
            let rendered = name.render();

            assert!(
                !rendered.is_empty(),
                "the fixture must render something, or both assertions below \
                 are satisfied by two empty strings"
            );
            assert_eq!(
                name.to_string(),
                rendered,
                "`to_string()` must give the name, not something else"
            );
            assert_eq!(
                format!("{name}"),
                rendered,
                "`{{}}` must give the name, not something else"
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
    fn the_derivation_reads_no_byte_outside_its_window() {
        // Two 32-byte values agreeing INSIDE the window and differing on EVERY
        // byte outside it — on both sides, which the old digest-based version
        // could not do because its window started at 0 and had no "before".
        // A derivation that read anywhere else would produce different names.
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        for i in 0..32 {
            if name_key_bytes().contains(&i) {
                a[i] = (i as u8) * 7 + 1;
                b[i] = (i as u8) * 7 + 1;
            } else {
                a[i] = 0x00;
                b[i] = 0xff;
            }
        }
        // The fixture must actually differ somewhere, or this passes on two
        // identical values.
        assert_ne!(a, b, "the fixture must differ outside the window");
        assert_eq!(
            name_from_key_bytes(&a),
            name_from_key_bytes(&b),
            "bytes outside the name's window must not participate"
        );
    }

    #[test]
    fn each_slot_reads_bytes_no_other_slot_reads() {
        // Vary one slot's two bytes and the OTHER TWO slots must not move. A
        // scheme feeding one byte to two slots fails this, and would have made
        // the three words three views of the same bits.
        let base = [0u8; 32];
        let reference = name_from_key_bytes(&base);

        // Offsets into the KEY, so each pair is the slot's real position rather
        // than an offset within the window — a fixture built at 0, 2, 4 would
        // vary bytes the derivation does not read and every assertion below
        // would be about three unchanged slots.
        for (slot, bytes) in [("adjective", 18usize), ("noun", 20), ("place", 22)] {
            let mut varied = base;
            varied[bytes] = 0x5a;
            varied[bytes + 1] = 0xa5;
            let moved = name_from_key_bytes(&varied);

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
        // **Driven through `name_from_key_bytes`, not through the test's own
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
                // bytes vary and the word read back is this slot's draw. Written
                // at 18 + slot*2, the slot's real position in the key: at
                // slot*2 it would vary bytes the derivation never reads and
                // every count would land on one word.
                let mut key_bytes = [0u8; 32];
                key_bytes[18 + slot * 2] = (draw >> 8) as u8;
                key_bytes[18 + slot * 2 + 1] = draw as u8;

                let name = name_from_key_bytes(&key_bytes);
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

    /// A 32-byte value whose three draws select exactly
    /// `(adjective, noun, place)`.
    ///
    /// Constructing it is the whole point and is the reason
    /// [`name_from_key_bytes`] is public: reaching a chosen slot combination
    /// through a chosen KEY means grinding for one, so a requirement about
    /// *which* words come back is checkable only if the bytes can be supplied
    /// directly.
    ///
    /// **Written at the name's window, `18..23`, not at offset 0.** Placing the
    /// draws at the start would build a value the derivation does not read, and
    /// every test using this helper would then be asserting about three zero
    /// draws — passing, and about nothing. The offsets are the spec's figures,
    /// written out rather than taken from `name_key_bytes()`, so a moved window
    /// fails these tests rather than dragging the fixtures along with it.
    ///
    /// Each index is written big-endian into its slot's two bytes. An index
    /// below its list's length survives the `%` unchanged, so the value this
    /// builds selects the indices it names.
    fn key_bytes_drawing(adjective: u16, noun: u16, place: u16) -> [u8; 32] {
        let mut d = [0u8; 32];
        for (i, v) in [adjective, noun, place].iter().enumerate() {
            d[18 + i * 2] = (v >> 8) as u8;
            d[18 + i * 2 + 1] = *v as u8;
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
            let name = name_from_key_bytes(&key_bytes_drawing(a, n, p));
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
        // They are positions in the shipped lists, produced by reading the text
        // files, not read back from here.
        assert_eq!((pensive, zenon, kition), (5_136, 1_015, 431));

        let name = name_from_key_bytes(&key_bytes_drawing(pensive, zenon, kition));
        assert_eq!(
            name.render(),
            "pensive zenon of kition",
            "a real figure's canonical citation must draw like any other name"
        );
    }

    #[test]
    fn a_name_is_a_function_of_six_bytes_and_nothing_else() {
        // Two values agreeing on the window and differing on every byte outside
        // it. Equal names, WHATEVER the words drawn are — so no property of the
        // drawn words feeds back into the derivation, which is what forbids a
        // filter reading its own output.
        //
        // Distinct from `the_derivation_reads_no_byte_outside_its_window` in
        // what it rules out: that one is about the WINDOW, this one about the
        // absence of FEEDBACK. A scheme that read only six bytes but redrew on a
        // refused pair would pass that test and fail this one, because the
        // redraw would have to read further to redraw from anywhere. It is swept
        // over chosen word combinations for that reason — a filter applies to
        // some draws and not others, so the combination has to be steered.
        for (a, n, p) in [
            (0u16, 0u16, 0u16),
            (5_136, 1_015, 431),
            (8_191, 1_023, 1_023),
        ] {
            let mut x = key_bytes_drawing(a, n, p);
            let mut y = key_bytes_drawing(a, n, p);
            for i in 0..32 {
                if !name_key_bytes().contains(&i) {
                    x[i] = 0x00;
                    y[i] = 0xff;
                }
            }
            assert_ne!(x, y, "the fixture must actually differ outside the window");
            assert_eq!(
                name_from_key_bytes(&x),
                name_from_key_bytes(&y),
                "a name must be a function of key bytes 18..23 alone"
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
        // (Those are ARRAY indices, not file line numbers. The FIRST LITERAL is
        // at line 28 of `nouns.rs` and line 24 of `places.rs` — one past the
        // `pub const NOUNS/PLACES = &[` line, which is the off-by-one to avoid —
        // so a grep's line number is the index plus that offset: `zenon` at
        // 1015 + 28 = 1043, `kition` at 431 + 24 = 455. Both verified by grep.
        // A trap worth naming, because reading a grep hit as an index is how a
        // wrong index gets written down confidently.)
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
        // **The PUBLIC KEY is what tells a colliding pair apart**, which is the
        // sentence issue #80 changes. This asserted distinct ADDRESSES and gave
        // that as the reason; the author address is being deleted, so a claim
        // resting on it would be a claim about a value that is on its way out.
        // The key is the identity, and its distinctness is what makes the two
        // rows two people.
        assert_ne!(
            a.public_key().to_bytes(),
            b.public_key().to_bytes(),
            "the fixture must be two distinct keys — the key is what tells a \
             colliding pair apart, so identical keys would make this one \
             identity compared with itself"
        );

        // And they must differ OUTSIDE the name's window too. Inside it they
        // are equal by construction (that is the collision); if that were all
        // that differed anywhere, "two distinct identities" would be a
        // distinction no channel could render and the fixture would be weaker
        // than it looks.
        let ka = a.public_key().to_bytes();
        let kb = b.public_key().to_bytes();
        assert!(
            (0..32).any(|i| !name_key_bytes().contains(&i) && ka[i] != kb[i]),
            "the colliding pair differs only inside the name's window, so no \
             other channel could tell them apart either"
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

    // ─── The scheme is frozen, with no version to bump ─────────────────────

    #[test]
    fn a_names_whole_input_is_six_key_bytes_with_no_version_alongside_them() {
        // **This inverts the test it replaces, and the inversion is the
        // change.** The deleted version built a v2 separator from the shipped v1
        // one and showed that the two schemes minted different names for one key
        // — what a version bump BUYS. There is no separator now, so there is no
        // version to vary and nothing to distinguish two schemes by. The spec
        // records this as a cost taken deliberately rather than a property lost
        // by accident.
        //
        // **Asserting an absence is the hard part**, and the honest shape is a
        // positive claim that would fail if a version reappeared: a name is a
        // function of six key bytes and NOTHING ELSE, so any two 32-byte values
        // agreeing on the window reach the same name whatever else differs. A
        // scheme that mixed in a version — a constant, a build stamp, anything —
        // would have to read it from somewhere, and if it read it from the key
        // this fails, while if it read it from ambient state
        // `a_name_is_unchanged_by_every_surrounding_state` fails.
        //
        // What CANNOT be asserted here is the negative in general: a version
        // compiled in as a literal is invisible to any runtime check, because it
        // is indistinguishable from the wordlists themselves. That is exactly
        // why the spec calls this a one-way door rather than a property under
        // test, and the wordlist pins are what actually hold the scheme still.
        let window: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];

        let mut first = [0x00u8; 32];
        let mut second = [0xffu8; 32];
        for (i, b) in name_key_bytes().zip(window) {
            first[i] = b;
            second[i] = b;
        }

        // The fixture must differ outside the window, or two identical values
        // are being compared and the test cannot fail.
        assert_ne!(first, second);
        assert_eq!(
            name_from_key_bytes(&first),
            name_from_key_bytes(&second),
            "two values agreeing only on the name's window reached different \
             names, so something outside those six bytes feeds the derivation"
        );
    }

    #[test]
    fn a_name_carries_no_marker_of_which_scheme_produced_it() {
        // The other half of the spec's scenario: with no version input, two
        // schemes' names are also reported IDENTICALLY — a name carries nothing
        // saying which wordlists produced it. So the rendered form is three
        // drawn words and a connector, with no version field, no suffix and no
        // punctuation that could hold one.
        //
        // This is a claim about the OUTPUT rather than about the input, and the
        // two are different failures: a scheme could take a version and hide it,
        // or take none and stamp one on. Swept over many keys because a marker
        // could be conditional.
        for seed in 1u8..40 {
            let name = display_name(&a_key(seed).public_key());
            let rendered = name.render();
            assert!(
                rendered.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                "seed {seed}: {rendered} carries a character that could hold a \
                 scheme marker"
            );
            assert_eq!(
                rendered,
                format!(
                    "{} {} {CONNECTOR} {}",
                    name.adjective, name.noun, name.place
                ),
                "seed {seed}: the rendered name carries something beyond the \
                 three drawn words and the connector"
            );
        }
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
