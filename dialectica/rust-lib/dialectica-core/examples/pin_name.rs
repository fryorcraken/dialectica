//! Computes a pinned name from a digest BY HAND, without calling the
//! derivation under test.
//!
//! ```text
//! cargo run --example pin_name --manifest-path \
//!     dialectica/rust-lib/dialectica-core/Cargo.toml -- <digest-hex>
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
//! digest    = SHA256(NAME_PREFIX || public_key)
//! adjective = ADJECTIVES[be16(digest[0..2]) % 8192]
//! noun      = NOUNS     [be16(digest[2..4]) % 1024]
//! place     = PLACES    [be16(digest[4..6]) % 1024]
//! ```
//!
//! and reads the three lists from the **text files** in `wordlists/` rather than
//! from `src/names/*.rs`. It deliberately does not link the derivation: there is
//! no `use dialectica_core::names` here, and adding one would silently turn
//! every pin into the implementation agreeing with itself.
//!
//! It draws once. There is no second draw to compute — every draw is a single
//! unconditional reduction, so a name is a function of six bytes and nothing
//! else. An earlier version printed a `draw2` from bytes 6..12 for a redraw the
//! scheme no longer has.

use std::fs;
use std::path::PathBuf;

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

    let digest_hex = std::env::args()
        .nth(1)
        .expect("pass the 32-byte name digest as hex");
    let digest: Vec<u8> = (0..digest_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digest_hex[i..i + 2], 16).expect("digest is hex"))
        .collect();
    assert!(digest.len() >= 6, "a name reads the first six digest bytes");

    let be16 = |i: usize| u16::from_be_bytes([digest[i], digest[i + 1]]);
    let a = be16(0) % 8192;
    let n = be16(2) % 1024;
    let p = be16(4) % 1024;

    println!("indices: adjective {a} noun {n} place {p}");
    println!(
        "name: {} {} of {}",
        adjectives[a as usize], nouns[n as usize], places[p as usize]
    );
}
