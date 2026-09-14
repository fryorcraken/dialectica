//! Computes a pinned name from a PUBLIC KEY by hand, without calling the
//! derivation under test.
//!
//! ```text
//! cargo run --example pin_name --manifest-path \
//!     dialectica/rust-lib/dialectica-core/Cargo.toml -- <public-key-hex>
//! ```
//!
//! **This is the whole point of a pin.** The expected value must be produced
//! independently, so that exchanging a wordlist entry, reordering a list,
//! changing which bytes a slot reads or changing the connector FAILS the test. A
//! pin read back from `display_name` would agree with whatever `display_name`
//! did — which is this repo's named test defect: an assertion whose two sides
//! come from one source.
//!
//! So this program reproduces the scheme from its written definition rather than
//! from the crate:
//!
//! ```text
//! adjective = ADJECTIVES[be16(key[18..20]) % 8192]
//! noun      = NOUNS     [be16(key[20..22]) % 1024]
//! place     = PLACES    [be16(key[22..24]) % 1024]
//! ```
//!
//! and reads the three lists from the **text files** in `wordlists/` rather than
//! from `src/names/*.rs`. It deliberately does not link the derivation: there is
//! no `use dialectica_core::names` here, and adding one would silently turn
//! every pin into the implementation agreeing with itself.
//!
//! **The input is the key itself, with no hash in between**, which is what
//! issue #80 changed. An earlier version of this program took a digest —
//! `SHA256(NAME_PREFIX || public_key)` — and drew from its first six bytes. There
//! is no digest now and no separator; the name reads key bytes `18..23`, the
//! range the `generated-names` spec allocates to it, and the window's offset is
//! as much a part of the pin as the wordlists are. A window that slipped one byte
//! reads different values and reaches a different name.
//!
//! It draws once. There is no second draw to compute — every draw is a single
//! unconditional reduction, so a name is a function of six bytes and nothing
//! else.

use std::fs;
use std::path::PathBuf;

/// The first key byte the name reads. The spec's allocation table puts the
/// name at `18..23`; this program states that figure from the SPEC rather than
/// importing the crate's constant, which is what keeps the pin independent.
const NAME_FIRST_BYTE: usize = 18;

fn read_list(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .map(|l| l.to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn main() {
    let words = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wordlists");
    let adjectives = read_list(&words.join("adjectives.txt"));
    let nouns = read_list(&words.join("nouns.txt"));
    let places = read_list(&words.join("places.txt"));

    // Asserted rather than assumed: the reduction below is only uniform, and
    // the indices below only meaningful, at these sizes.
    assert_eq!(adjectives.len(), 8192);
    assert_eq!(nouns.len(), 1024);
    assert_eq!(places.len(), 1024);

    let key_hex = std::env::args()
        .nth(1)
        .expect("pass the 32-byte public key as hex");
    let key: Vec<u8> = (0..key_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&key_hex[i..i + 2], 16).expect("the key is hex"))
        .collect();
    assert_eq!(key.len(), 32, "a public key is 32 bytes");

    let be16 = |i: usize| u16::from_be_bytes([key[i], key[i + 1]]);
    let a = be16(NAME_FIRST_BYTE) % 8192;
    let n = be16(NAME_FIRST_BYTE + 2) % 1024;
    let p = be16(NAME_FIRST_BYTE + 4) % 1024;

    println!("indices: adjective {a} noun {n} place {p}");
    println!(
        "name: {} {} of {}",
        adjectives[a as usize], nouns[n as usize], places[p as usize]
    );
}
