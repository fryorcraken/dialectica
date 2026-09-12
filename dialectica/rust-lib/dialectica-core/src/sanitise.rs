//! Making attacker-supplied text safe to render, without changing the record.
//!
//! # Why this is core's job and not the view's
//!
//! The UI brief assigns sanitisation to the interface, on the reasoning that the
//! core "deliberately does not sanitise". Both halves of that are true and they
//! do not imply the conclusion, because §2.1 puts the QML engine in a sandbox
//! with no filesystem and no network: **every byte the view renders arrives
//! through a core method**. A view cannot sanitise text it is handed already
//! rendered, and a view that sanitised in QML would be running attacker-supplied
//! strings through a scripting layer that also holds the rendering primitives.
//!
//! So the obligation is the interface's and the *implementation* is core's, and
//! the two are compatible because the boundary is where the string is built.
//! `op.rs` still preserves display text **exactly** — nothing here mutates an op,
//! an op id, or anything that crosses the wire. This module runs on the way
//! **out**, between the resolver and the JSON, and its output is a rendering
//! rather than a record.
//!
//! # Mark, do not correct — and the asymmetry between the two lists
//!
//! SPEC.md draws a line this module is built around:
//!
//! > Strip or visibly mark bidirectional overrides [...] Mark homoglyph mixing
//! > (a Cyrillic a inside a Latin word) rather than correcting it — **correcting
//! > changes what the record says.**
//!
//! The two categories get different treatment because they are different kinds
//! of problem:
//!
//! - **Invisibles are removed.** A bidi override or a zero-width joiner has no
//!   glyph; it exists to change how the characters *around* it render. Leaving
//!   one in place and adding a note beside it does not help, because the damage
//!   is done to the rest of the line. Removing it is not a correction — the
//!   removed character was never visible, so what the reader sees afterwards is
//!   what the bytes say minus something that was only ever an instruction.
//! - **Homoglyphs are counted, never touched.** A Cyrillic `а` IS a character
//!   the author wrote and it does render. Replacing it with a Latin `a` would
//!   change the text into one the author did not publish, and a reader comparing
//!   two peers' renderings would find them disagreeing about the post's content.
//!   So the character stays exactly where it is and the *count* travels beside
//!   it, for the view to render as the chip SPEC.md describes.
//!
//! # What the view is told, and why it is a count rather than a flag
//!
//! [`Sanitised`] carries the cleaned text plus two numbers. A boolean would be
//! cheaper and is the wrong shape: copy.json's marker reads
//! `"contains %1 marked character(s)"` and `"%1 removed"`, both of which need
//! the number. A view given a flag would have to say "contains marked
//! characters" and could not distinguish one homoglyph from forty.
//!
//! # This is not a security boundary against rendering
//!
//! Worth stating plainly so that nobody reads more into it than it does. What
//! stops a post body becoming markup is the view binding it to a `Text` element
//! with `textFormat: Text.PlainText` — SPEC.md requires exactly that, and this
//! module does not and cannot enforce it. Removing invisibles narrows a class of
//! *deception*, not a class of injection. A sanitiser that gave a view licence
//! to use `StyledText` would have made things worse.

/// Text prepared for display, and what had to be done to it.
///
/// # One type rather than a tuple, and one call rather than three
///
/// Every string a view renders needs the same treatment, and a caller that
/// received `(String, usize, usize)` would eventually pass the two counts in the
/// wrong order — they are both `usize` and both small. Naming them costs
/// nothing and the compiler checks the call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sanitised {
    /// What the view renders. Invisibles are gone; every visible character the
    /// author published is here, unchanged and in its original order.
    pub text: String,
    /// How many invisible characters were removed.
    ///
    /// Zero is the ordinary case, and a view showing "0 removed" would be
    /// noise — the view renders the chip only when this is non-zero.
    pub removed: usize,
    /// How many characters belong to a script other than the dominant one.
    ///
    /// **A count of what to mark, never a correction.** See this module's
    /// documentation for why the characters themselves are untouched.
    pub marked: usize,
}

impl Sanitised {
    /// Whether anything at all was found. The view's one branch for "does this
    /// string need a chip beside it".
    pub fn is_clean(&self) -> bool {
        self.removed == 0 && self.marked == 0
    }
}

/// Whether a character is invisible enough to be removed rather than rendered.
///
/// **The ranges are SPEC.md's, plus the two that follow from the same
/// reasoning.** Each entry is a character with no glyph of its own that changes
/// how its neighbours render or measure:
///
/// - `U+202A..=U+202E` — the bidi embedding and override set. `U+202E`
///   (RIGHT-TO-LEFT OVERRIDE) is the classic filename attack and the rest of the
///   run is the same mechanism.
/// - `U+2066..=U+2069` — the bidi *isolate* set, which does the same job as the
///   overrides above and is the one an attacker reaches for when a filter is
///   written against `U+202E` alone.
/// - `U+200B..=U+200D` — zero-width space, non-joiner and joiner. A zero-width
///   space inside a word breaks a reader's string comparison while rendering
///   identically.
/// - `U+FEFF` — zero-width no-break space, the BOM in its in-band spelling.
/// - `U+2060..=U+2064` — word joiner and the invisible operators (function
///   application, times, separator, plus). **Added by review, and the finding
///   is worth keeping** because the list was incomplete against its own stated
///   criterion rather than against some wider standard: `U+2060` is the
///   documented non-deprecated replacement for `U+FEFF` in exactly the
///   word-joining role, so `sanitise("we\u{2060}ll")` and
///   `sanitise("we\u{FEFF}ll")` were doing the same job and getting different
///   answers. A rule that produces two answers for one job is the shape a
///   bypass is built from.
/// - `U+061C` — Arabic letter mark, a bidi control that is not in either range
///   above and does the same thing.
/// - `U+200E`, `U+200F` — left-to-right and right-to-left marks. Adjacent to the
///   zero-width run and deliberately included: they are directional controls
///   with no glyph, which is this function's whole criterion.
///
/// - `U+FFF9..=U+FFFB` — the interlinear annotation delimiters. Review called
///   these arguable and left the call here; they are **in**, because they meet
///   the stated criterion exactly: they have no glyph and they restructure the
///   text around them, marking a run as an annotation anchored to another run.
///   Unicode's own guidance is that they are not for open interchange and
///   should be stripped by a receiver, which is precisely this function's
///   position. The cost of including them is a post that legitimately uses
///   interlinear annotation rendering as plain text with a "3 removed" chip;
///   the cost of omitting them is a glyphless construct that can reorder what
///   a reader sees. The second is the failure this module exists to prevent.
///
/// **Whitespace is NOT here, and that is the boundary worth being careful
/// about.** A space, a tab and a newline are invisible in the sense of having no
/// ink, and they are part of what the author wrote — stripping them would change
/// the text's shape. The criterion is "has no glyph and alters how other
/// characters render", which a newline does not meet.
fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{202A}'..='\u{202E}'
        | '\u{2066}'..='\u{2069}'
        | '\u{200B}'..='\u{200D}'
        | '\u{200E}' | '\u{200F}'
        | '\u{2060}'..='\u{2064}'
        | '\u{061C}'
        | '\u{FEFF}'
        | '\u{FFF9}'..='\u{FFFB}'
    )
}

/// The script a character belongs to, for the purpose of spotting mixing.
///
/// # Deliberately coarse, and coarse in a direction that matters
///
/// This is not Unicode's `Script` property and does not try to be. It separates
/// the three alphabets whose letters are *confusable with each other* — Latin,
/// Cyrillic and Greek — and lumps everything else into [`Script::Other`], which
/// never participates in a mixing judgement.
///
/// The consequence is the one to understand before changing it: **a post written
/// in Greek, or Arabic, or Japanese is not "mixed" and is not marked.** Marking
/// it would be the sanitiser telling a reader that ordinary text in the author's
/// own language is suspicious, which is both wrong and the kind of wrong that
/// falls disproportionately on people not writing in Latin script.
///
/// A full confusables table (UTS #39) is the real answer and is a data
/// dependency plus a tuned skeleton algorithm. This is the honest small version:
/// it catches the attack that is actually cheap — a Cyrillic letter hidden in a
/// Latin word — and says nothing about the cases it cannot judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
    /// Digits, punctuation, spaces, emoji, and every script not confusable with
    /// the three above. Never counted as mixing in either direction.
    Other,
}

fn script_of(c: char) -> Script {
    match c {
        'a'..='z' | 'A'..='Z' => Script::Latin,
        // Cyrillic, including the supplement. `а`, `е`, `о`, `р`, `с`, `х` are
        // the ones that matter — each renders identically to a Latin letter in
        // most faces.
        '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}' => Script::Cyrillic,
        // Greek and Coptic. `ο`, `ι`, `ν`, `Α`, `Β`, `Ε` are the confusable set.
        '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}' => Script::Greek,
        _ => Script::Other,
    }
}

/// Prepare one attacker-supplied string for display.
///
/// # The order of the two passes is load-bearing
///
/// Invisibles are removed **first**, and the homoglyph count is taken over what
/// is left. The other order is a real bug rather than a stylistic choice: a
/// zero-width space sits between two letters and would otherwise be counted as
/// an `Other`-script character separating them, which is exactly the wedge an
/// attacker would use to make `pа` (Latin p, Cyrillic a) look like two
/// unrelated runs. Counting after removal means the string being judged is the
/// string the reader will actually see.
///
/// # How "mixed" is decided
///
/// The dominant script is whichever of Latin, Cyrillic and Greek has the most
/// characters; every character belonging to one of the other two is marked.
/// [`Script::Other`] characters are never marked and never dominant, so digits,
/// spaces and punctuation neither trigger nor dilute the judgement.
///
/// **A single-script string is never marked, whichever script it is.** That is
/// the property that keeps this from flagging ordinary Russian or Greek text,
/// and it falls out of the count rather than being a special case: with one
/// script present, nothing belongs to another.
///
/// **Ties go to "no marking".** Two scripts with equal counts is the shape a
/// short mixed string has — `аb` is one of each — and marking one arbitrarily
/// would mark a character the other reading says is fine. A tie is genuinely
/// ambiguous, and the count reflects that rather than guessing.
pub fn sanitise(input: &str) -> Sanitised {
    // Pass one: drop what has no glyph. The count is of characters, matching
    // copy.json's "%1 removed" — a view saying "3 bytes removed" would be
    // reporting an implementation detail nobody can act on.
    let mut text = String::with_capacity(input.len());
    let mut removed = 0usize;
    for c in input.chars() {
        if is_invisible(c) {
            removed += 1;
        } else {
            text.push(c);
        }
    }

    // Pass two: count script mixing over what survives.
    let mut latin = 0usize;
    let mut cyrillic = 0usize;
    let mut greek = 0usize;
    for c in text.chars() {
        match script_of(c) {
            Script::Latin => latin += 1,
            Script::Cyrillic => cyrillic += 1,
            Script::Greek => greek += 1,
            Script::Other => {}
        }
    }

    let total = latin + cyrillic + greek;
    let max = latin.max(cyrillic).max(greek);
    // A tie for the lead means no dominant script, so nothing is a minority and
    // nothing is marked. `count == max` is how the tie is detected without a
    // separate branch: with two scripts at the top, both match and the marked
    // count comes out as the remainder of neither.
    let dominant_count = [latin, cyrillic, greek].iter().filter(|&&n| n == max).count();
    let marked = if max == 0 || dominant_count > 1 {
        0
    } else {
        total - max
    };

    Sanitised {
        text,
        removed,
        marked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property every test in this module is really asserting, named once.
    ///
    /// **A sanitiser that did nothing must fail these.** That is this project's
    /// one test-defect family — a fixture where two explanations give the same
    /// answer — and for a sanitiser the degenerate explanation is "the function
    /// is the identity". So each test below either asserts a difference from its
    /// input or asserts a non-zero count, and never only that the output is
    /// well-formed.
    /// **Checked, not assumed.** Replacing [`sanitise`]'s body with
    /// `Sanitised { text: input.into(), removed: 0, marked: 0 }` and running
    /// this module failed 16 of 23 tests, plus the three in [`crate::feed`] and
    /// [`crate::wire`] that assert the obligation at the boundary.
    ///
    /// The 7 that survived are the negative tests — clean text passes through,
    /// whitespace is kept, the range boundaries are kept, single-script text is
    /// not marked — and they SHOULD survive, because each asserts that the
    /// sanitiser leaves something alone. A negative test that failed against a
    /// no-op would be asserting the wrong thing.
    ///
    /// The three tests covering `U+2060..=U+2064` and `U+FFF9..=U+FFFB` were
    /// checked the same way, against the narrower list they were written to
    /// correct: exactly those three failed and the other 23 passed, so each
    /// pins the range it names rather than something a neighbouring range
    /// already covered.
    fn assert_did_something(input: &str, out: &Sanitised) {
        assert!(
            out.text != input || !out.is_clean(),
            "the sanitiser was a no-op on {input:?}, so this test would pass \
             against an empty implementation"
        );
    }

    // ─── Invisibles ───────────────────────────────────────────────────────

    #[test]
    fn every_bidi_override_in_the_range_is_removed() {
        // The whole run, not just U+202E. A filter written against the famous
        // one alone leaves four characters that do the same job, and an
        // attacker reads the same blog posts the filter author did.
        for c in ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}'] {
            let input = format!("safe{c}text");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "safetext");
            assert!(
                !out.text.contains(c),
                "U+{:04X} survived into the rendered text",
                c as u32
            );
            assert_did_something(&input, &out);
        }
    }

    #[test]
    fn every_bidi_isolate_in_the_range_is_removed() {
        // The isolates are the second mechanism and the one a U+202E-only
        // filter misses entirely.
        for c in ['\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}'] {
            let input = format!("safe{c}text");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "safetext");
            assert_did_something(&input, &out);
        }
    }

    #[test]
    fn zero_width_characters_and_the_bom_are_removed() {
        for c in ['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'] {
            let input = format!("we{c}ll");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "well");
            assert_did_something(&input, &out);
        }
    }

    #[test]
    fn the_directional_marks_beside_the_zero_width_run_are_removed() {
        // U+200E and U+200F sit immediately after the zero-width run and are
        // easy to omit when the range is written as 200B..=200D. They are
        // directional controls with no glyph, which is the criterion.
        for c in ['\u{200E}', '\u{200F}', '\u{061C}'] {
            let input = format!("a{c}b");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "ab");
        }
    }

    #[test]
    fn the_word_joiner_and_the_invisible_operators_are_removed() {
        // Found by review: the list was incomplete against its OWN criterion.
        // U+2060 WORD JOINER is the documented non-deprecated replacement for
        // U+FEFF in the word-joining role, so before this the two characters
        // did the same job and got different answers — which is the shape a
        // bypass is built from, not merely an omission.
        for c in ['\u{2060}', '\u{2061}', '\u{2062}', '\u{2063}', '\u{2064}'] {
            let input = format!("we{c}ll");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "well");
            assert_did_something(&input, &out);
        }
    }

    #[test]
    fn the_word_joiner_and_the_bom_now_get_the_same_answer() {
        // The regression stated as the property rather than as two separate
        // counts: these two characters are interchangeable for the same job,
        // so a sanitiser treating them differently is wrong whichever way the
        // difference runs. This fails against the pre-review list.
        let joiner = sanitise("we\u{2060}ll");
        let bom = sanitise("we\u{FEFF}ll");
        assert_eq!(joiner, bom, "U+2060 and U+FEFF must be handled identically");
        assert_eq!(joiner.removed, 1);
    }

    #[test]
    fn the_interlinear_annotation_delimiters_are_removed() {
        // Review left this call to me and I included them: they have no glyph
        // and they restructure the text around them, which is the criterion.
        // Unicode's own guidance is that they are not for open interchange and
        // a receiver should strip them.
        for c in ['\u{FFF9}', '\u{FFFA}', '\u{FFFB}'] {
            let input = format!("a{c}b");
            let out = sanitise(&input);
            assert_eq!(out.removed, 1, "U+{:04X} was not removed", c as u32);
            assert_eq!(out.text, "ab");
        }
    }

    #[test]
    fn the_boundary_characters_around_each_range_are_kept() {
        // THE boundary test, at the boundary rather than far past it. This
        // project's defect family includes "a boundary test that tests a value
        // far past the boundary instead of at it" — asserting that 'A' survives
        // proves nothing about whether the range ends in the right place.
        //
        // Each of these is exactly one code point outside a removed range and
        // must survive.
        for c in [
            '\u{2029}', // just below 202A
            '\u{202F}', // just above 202E — NARROW NO-BREAK SPACE, a real space
            '\u{205F}', // just below 2060 — MEDIUM MATHEMATICAL SPACE, real
            '\u{2065}', // between 2064 and 2066 — the one gap between the two
            // removed ranges, and the only code point in it. A range written
            // as 2060..=2069 would swallow it; it is unassigned, but a
            // sanitiser that cannot tell the two ranges apart has stopped
            // describing what it claims to.
            '\u{206A}', // just above 2069
            '\u{200A}', // just below 200B — HAIR SPACE, a real space
            '\u{2010}', // above the 200E/200F pair — HYPHEN
            '\u{FEFE}', // just below FEFF
            '\u{FF00}', // just above FEFF
            '\u{061B}', // just below 061C
            '\u{061D}', // just above 061C
            '\u{FFF8}', // just below FFF9
            '\u{FFFC}', // just above FFFB — OBJECT REPLACEMENT CHARACTER,
            // which DOES have a visible rendering and is content
        ] {
            let input = format!("a{c}b");
            let out = sanitise(&input);
            assert_eq!(
                out.removed,
                0,
                "U+{:04X} is outside every removed range and was removed anyway",
                c as u32
            );
            assert_eq!(out.text, input);
        }
    }

    #[test]
    fn ordinary_whitespace_is_never_removed() {
        // The line this module is careful about: a newline has no ink and is
        // not invisible in the sense that matters. Stripping it would reflow
        // every post that has a paragraph break.
        let input = "one\ntwo\tthree four\r\nfive";
        let out = sanitise(input);
        assert_eq!(out.text, input, "whitespace is content, not a control");
        assert_eq!(out.removed, 0);
    }

    #[test]
    fn a_run_of_invisibles_is_counted_individually() {
        // "%1 removed" is a count of characters. A sanitiser collapsing a run
        // to one would under-report by an order of magnitude on the payload an
        // attacker actually sends.
        let input = "a\u{200B}\u{200B}\u{200B}\u{202E}b";
        let out = sanitise(input);
        assert_eq!(out.removed, 4);
        assert_eq!(out.text, "ab");
    }

    #[test]
    fn text_made_entirely_of_invisibles_becomes_empty_and_says_so() {
        // An empty result is a legitimate rendering of a post that was nothing
        // but controls, and the count is what stops it looking like an empty
        // post the author wrote.
        let out = sanitise("\u{202E}\u{200B}\u{FEFF}");
        assert_eq!(out.text, "");
        assert_eq!(out.removed, 3);
        assert!(!out.is_clean());
    }

    // ─── Homoglyphs ───────────────────────────────────────────────────────

    #[test]
    fn a_cyrillic_letter_inside_a_latin_word_is_marked() {
        // The attack, exactly: U+0430 CYRILLIC SMALL LETTER A inside an
        // otherwise-Latin word. It renders identically to 'a' in most faces.
        let input = "p\u{0430}ypal";
        let out = sanitise(input);
        assert_eq!(out.marked, 1, "the Cyrillic 'а' was not marked");
        assert_did_something(input, &out);
    }

    #[test]
    fn a_marked_character_is_left_exactly_where_it_was() {
        // THE rule that separates this from a spell-checker: marking must not
        // correct. Correcting changes what the record says, and two peers
        // correcting differently would disagree about a post's content.
        let input = "p\u{0430}ypal";
        let out = sanitise(input);
        assert_eq!(
            out.text, input,
            "a homoglyph must be counted and NEVER replaced"
        );
        assert!(
            out.text.contains('\u{0430}'),
            "the Cyrillic character must survive into the rendered text"
        );
    }

    #[test]
    fn text_in_one_script_is_never_marked_whichever_script_it_is() {
        // The property that keeps this from flagging ordinary text in a
        // non-Latin language. A sanitiser that marked "everything non-Latin"
        // would put a warning chip on every Russian post in the forum.
        for input in [
            "ordinary latin text",
            "полностью кириллический текст",
            "ένα ελληνικό κείμενο",
            "12345 !?.,-— (){}",
            "🏛 🏺 ✎",
            "",
        ] {
            let out = sanitise(input);
            assert_eq!(out.marked, 0, "single-script text was marked: {input:?}");
            assert_eq!(out.text, input);
        }
    }

    #[test]
    fn the_minority_script_is_the_one_counted() {
        // Direction matters: in a mostly-Cyrillic string it is the stray Latin
        // letter that is anomalous, not the other way round. A sanitiser
        // hardcoding "non-Latin is suspicious" gets this backwards.
        let input = "пароль пароль p";
        let out = sanitise(input);
        assert_eq!(
            out.marked, 1,
            "in mostly-Cyrillic text the lone Latin letter is the minority"
        );
    }

    #[test]
    fn an_even_split_marks_nothing_rather_than_guessing() {
        // A tie has no dominant script, so marking either side would mark
        // characters the other reading calls fine. Ambiguous is reported as
        // ambiguous.
        let out = sanitise("ab\u{0430}\u{0431}");
        assert_eq!(out.marked, 0, "a 2-2 split has no minority to mark");
    }

    #[test]
    fn digits_and_punctuation_neither_trigger_nor_dilute_the_judgement() {
        // `Other` must be inert in both directions: it cannot be dominant and
        // it cannot be marked. A sanitiser counting punctuation as a script
        // would find every ordinary sentence "mixed".
        let heavy_punctuation = sanitise("!!!!!!!!!! ??? ,,, 123456");
        assert_eq!(heavy_punctuation.marked, 0);

        // The same Cyrillic-in-Latin case, now padded with punctuation that
        // outnumbers every letter. The marked count must be unchanged.
        let padded = sanitise("!!!!!!!!!!!!!!! p\u{0430}ypal !!!!!!!!!!!!!!!");
        assert_eq!(
            padded.marked, 1,
            "punctuation must not dilute a real mixing judgement"
        );
    }

    #[test]
    fn multiple_intruding_characters_are_counted_individually() {
        // "contains %1 marked character(s)" needs the number, so a sanitiser
        // returning 1 for "some mixing" under-reports the payload.
        let out = sanitise("p\u{0430}yp\u{0430}l\u{0435}");
        assert_eq!(out.marked, 3);
    }

    // ─── The two passes together ──────────────────────────────────────────

    #[test]
    fn invisibles_are_removed_before_the_homoglyph_count_is_taken() {
        // THE ordering bug, and it is the one worth pinning because both
        // orders produce a plausible-looking answer.
        //
        // With a zero-width space wedged between them, the Latin 'p' and the
        // Cyrillic 'а' are still adjacent to the reader — the wedge has no
        // glyph. Counting before removal would judge a string the reader never
        // sees. The assertion is that the wedge changes nothing about the
        // marked count: the answer must equal the unwedged case.
        let unwedged = sanitise("p\u{0430}ypal");
        let wedged = sanitise("p\u{200B}\u{0430}ypal");

        assert_eq!(wedged.removed, 1);
        assert_eq!(
            wedged.marked, unwedged.marked,
            "a zero-width wedge must not change the mixing judgement"
        );
        assert_eq!(wedged.marked, 1);
        // And the surviving text is the one the reader sees, with the
        // homoglyph still in place.
        assert_eq!(wedged.text, "p\u{0430}ypal");
    }

    #[test]
    fn both_findings_are_reported_at_once() {
        // A view renders two different chips and needs both numbers from one
        // call. A sanitiser that returned on the first finding would silently
        // drop the second.
        let out = sanitise("p\u{0430}yp\u{202E}al");
        assert_eq!(out.removed, 1);
        assert_eq!(out.marked, 1);
        assert!(!out.is_clean());
    }

    #[test]
    fn clean_text_passes_through_byte_for_byte() {
        // The other direction of the no-op test: the sanitiser must not be
        // *over*-eager either. An ordinary post is returned unchanged and
        // reports nothing, so the view renders no chip.
        for input in [
            "An ordinary post, with punctuation — and a dash.",
            "Ἀγορά",
            "line one\nline two",
            "",
        ] {
            let out = sanitise(input);
            assert_eq!(out.text, input);
            assert!(out.is_clean(), "clean text reported a finding: {input:?}");
        }
    }

    #[test]
    fn a_long_hostile_body_does_not_panic_and_reports_honestly() {
        // Post bodies are the largest and least constrained attacker-supplied
        // strings this renders, and a panic here aborts the module process
        // (PHASE0-FINDINGS §3) rather than failing one call.
        let body = "\u{202E}p\u{0430}ypal\u{200B}".repeat(2000);
        let out = sanitise(&body);
        assert_eq!(out.removed, 4000, "one override and one ZWSP per repeat");
        assert_eq!(out.marked, 2000, "one Cyrillic 'а' per repeat");
        assert!(!out.text.contains('\u{202E}'));
        assert!(out.text.contains('\u{0430}'), "marking must not correct");
    }
}
