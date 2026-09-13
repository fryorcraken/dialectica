//! Emits the three wordlist modules from the curated text files.
//!
//! ```text
//! cargo run --example gen_wordlists --manifest-path \
//!     dialectica/rust-lib/dialectica-core/Cargo.toml
//! ```
//!
//! **The generated `.rs` files are what ships**, and this exists so that they
//! are *generated* rather than transcribed. The lists are 10,240 entries and
//! hand-transcription is where a homoglyph or a dropped line enters — both
//! census passes that fed `wordlists/` introduced Cyrillic homoglyphs into
//! hand-typed Greek, and one of them caught it in itself. Generation cannot
//! introduce a character that was not in the source, and [`check`] asserts ASCII
//! before anything is written.
//!
//! **This program and its inputs are tracked on purpose.** The whole argument
//! for generated `&[&str]` arrays over a parsed data file is auditability: a
//! reviewer diffs a generated array against the text file it came from. That
//! argument needs the text file to exist on a fresh checkout. It previously did
//! not — both lived under a gitignored `tmp/`, so on any checkout but the
//! author's the generated arrays were exactly the hand-transcribed arrays the
//! decision rejected, auditable by nobody.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Read one entry per line, dropping blanks so a stray trailing newline is not
/// an entry.
fn read_list(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The screens, applied before anything is emitted.
///
/// These are the spec's three mechanical screens minus attestation, which no
/// program can check: ASCII-transliterable and deduplicated, plus the size the
/// reduction arithmetic rests on. A list failing any of them is a build failure
/// rather than a shipped defect.
fn check(name: &str, list: &[String], expected: usize) {
    assert_eq!(list.len(), expected, "{name}: wrong length");
    let unique: BTreeSet<&String> = list.iter().collect();
    assert_eq!(unique.len(), expected, "{name}: holds a duplicate");
    for e in list {
        assert!(e.is_ascii(), "{name}: {e:?} is not ASCII");
        assert_eq!(*e, e.to_ascii_lowercase(), "{name}: {e:?} is not lowercase");
        assert!(!e.is_empty(), "{name}: empty entry");
        assert!(
            !e.starts_with(' ') && !e.ends_with(' ') && !e.contains("  "),
            "{name}: {e:?} is not well-formed"
        );
        assert!(
            e.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
            "{name}: {e:?} has a bad character"
        );
    }
}

fn emit(path: &Path, konst: &str, doc: &str, list: &[String]) {
    let mut out = String::new();
    out.push_str(doc);
    out.push_str(&format!("pub const {konst}: &[&str] = &[\n"));
    for e in list {
        out.push_str(&format!("    {e:?},\n"));
    }
    out.push_str("];\n");
    fs::write(path, out).unwrap();
}

fn main() {
    // Relative to this crate's own manifest, so the program runs from anywhere
    // and does not carry one machine's absolute paths — which the earlier
    // `tmp/gen.rs` did, and which is a second reason it could not be run by
    // anyone but its author.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let words = crate_dir.join("wordlists");
    let src = crate_dir.join("src/names");

    let adjectives = read_list(&words.join("adjectives.txt"));
    let nouns = read_list(&words.join("nouns.txt"));
    let places = read_list(&words.join("places.txt"));

    check("adjectives", &adjectives, 8192);
    check("nouns", &nouns, 1024);
    check("places", &places, 1024);

    // The one rule that keeps anything out, and it is about an entry's SPELLING
    // rather than any word's meaning: a noun carrying ` of ` renders as
    // *pensive zenon of kition of lampsakos*, two places on one name with no way
    // for a reader to tell which the place slot supplied.
    for n in &nouns {
        assert!(!n.contains(" of "), "noun {n:?} carries the connector");
    }

    emit(
        &src.join("adjectives.rs"),
        "ADJECTIVES",
        ADJ_DOC,
        &adjectives,
    );
    emit(&src.join("nouns.rs"), "NOUNS", NOUN_DOC, &nouns);
    emit(&src.join("places.rs"), "PLACES", PLACE_DOC, &places);

    println!(
        "adjectives {} nouns {} places {}",
        adjectives.len(),
        nouns.len(),
        places.len()
    );
}

const ADJ_DOC: &str = "\
//! The adjective list: 8,192 English adjectives.
//!
//! **Generated — do not edit by hand.** Built from `/usr/share/dict/words` by
//! `examples/gen_wordlists.rs` and kept in `wordlists/adjectives.txt`, so every
//! entry traces to a dictionary on disk rather than to anyone's recall. That is
//! what the spec's attestation screen asks for and it is the one screen no test
//! can check.
//!
//! The slot is open: **any English adjective**, with no tone, register,
//! familiarity or pronounceability screen. `luminous`, `pensive` and `restless`
//! draw alongside any other. The register of a name is carried by the *X of Y*
//! shape and the two Greek words in it, not by this list.
//!
//! The size is exactly 8,192 because `65,536 / 8,192 = 8`, so a 16-bit draw
//! reduced by `%` is exactly uniform and no word is favoured. Changing the size
//! — in either direction, and even to correct an entry — reindexes every draw
//! and is a scheme version bump, never an edit.

";

const NOUN_DOC: &str = "\
//! The noun list: 1,024 attested ancient Greek nouns.
//!
//! **Generated — do not edit by hand.** Built from `wordlists/nouns.txt`, whose
//! four pools were written region by region and deduplicated before selection:
//! abstractions, named historical Greeks, mythological figures, and ordinary
//! concrete nouns. 1,892 were counted and 1,024 kept, so the list was **cut
//! down to the power of two below its source and never padded up to reach
//! one** — the direction the spec requires, because a padded list ships
//! entries invented to fill it.
//!
//! **The four pools sit on equal footing and no screen asks what a word
//! means.** There is no project-vocabulary exclusion and no excluded figure:
//! `stoa`, `agora`, `archon`, `tyrannos`, `platon`, `aristoteles` and
//! `sokrates` are all in. Every successive draft that added a fourth filter —
//! familiarity, legibility, a single-word rule, a tone-and-authority
//! apparatus, a refused-combination denylist — was withdrawn on challenge.
//! Only three screens apply: ASCII-transliterable, deduplicated, attested.
//!
//! **No entry carries the connector as a word**, and this is the ONLY rule in
//! the capability that keeps anything out. A source supplying named Greeks
//! supplies them already qualified — `zenon kitieus`, `straton lampsakenos` —
//! and an entry spelled `zenon of kition` would render *pensive zenon of kition
//! of lampsakos*, two places attached to one name with no way for a reader to
//! tell which the place slot supplied. The bare names stay and draw normally:
//! what a noun means is no part of whether it is in.

";

const PLACE_DOC: &str = "\
//! The place list: 1,024 ancient Greek and mythological places.
//!
//! **Generated — do not edit by hand.** Built from `wordlists/places.txt`,
//! swept region by region across the mainland, the Attic demes, Crete, the
//! Aegean and Ionian islands, Cyprus, the Asia Minor coast, Magna Graecia and
//! Sicily, the Black Sea colonies, sanctuaries, mountains, rivers, regions and
//! mythological geography. 1,131 were counted, 107 cut, 1,024 kept.
//!
//! **What was cut, and why it is not a semantic screen.** Latinised doublets
//! (`piraeus` beside `peiraion`), near-duplicate transliterations
//! (`gortynia` beside `gortyn` and `gortys`), words that are not toponyms at
//! all (`lelantine` is an adjective), and Greek exonyms for non-Greek regions.
//! Each is the deduplication or attestation screen applied in substance rather
//! than a judgement about what a place connotes.
//!
//! **An entry may hold an internal space** — `lokroi epizephyrioi`,
//! `antiocheia maiandros`, `arsinoe kyprou`, `euxeinos pontos`. A single-word
//! rule is one of the screens the spec forbids, and it is the costly one: a
//! census written against that rule put this list's honest yield well below
//! 1,024 because multi-word toponyms were being discarded by a rule the design
//! never stated.

";
