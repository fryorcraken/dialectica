import QtQuick
import QtTest
import "../src/qml"

// What the feed SAYS, as against what state it is in. `tst_feed_states.qml`
// asserts `readState`, `rows.length` and `failure` — the state machine — and no
// test read a word the screen renders.
//
// That gap is what this file closes, and it is not hypothetical. Review replaced
// the empty state's locality sentence with the literal string **"This Stoa is
// empty."** and every test in the suite passed. That is not a missing
// disclaimer: it is the interface making a claim about the whole Stoa, which no
// peer is in a position to make — it can only know what it holds. `SPEC.md` puts
// the same rule in the copy itself: "Not *no posts yet* but *you have not
// received anything for this Stoa yet*."
//
// **Prefix, then relation — never a pinned sentence.** A test pinning the copy
// verbatim fails when someone rewords it, which is not a defect, and passes when
// someone rewords it into a falsehood that keeps the same opening, which is. So
// each sentence is FOUND by a short distinguishing prefix and then asserted on
// for the property it exists to carry. The prefixes are hardcoded here and must
// not be updated to match a change in the source: if one stops matching, read
// the new sentence and decide whether it still discharges the obligation before
// touching this file.
TestCase {
    id: spec
    name: "FeedCopy"

    property var savedBridge: undefined

    function init() { spec.savedBridge = Core.bridge }
    function cleanup() { Core.bridge = spec.savedBridge }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    Component { id: feedComponent; FeedScreen {} }

    function rowsJson(n) {
        var parts = []
        for (var i = 0; i < n; ++i) {
            parts.push('{"thread":"t' + i + '","currentVersion":"t' + i + '",'
                + '"author":"' + "cd".repeat(32) + '",'
                + '"body":{"text":"post ' + i + '","removed":0,"marked":0},'
                + '"attachments":[],"isRevised":false,"isHidden":false}')
        }
        return parts.join(",")
    }

    // `stoaAddress` must be non-empty or `reload()` short-circuits before it
    // calls the bridge, and the screen stays in the unread state — which would
    // make every absence assertion below pass for the wrong reason.
    function screenFor(rows, hasMore) {
        Core.bridge = bridgeFor({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[' + rowsJson(rows) + '],"page":0,'
                + '"hasMore":' + (hasMore ? "true" : "false") + '}'
        })
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            width: 1000
        })
    }

    function failedScreen() {
        Core.bridge = bridgeFor({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"error":"the store could not be opened"}'
        })
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            width: 1000
        })
    }

    // Every string the screen carries, visible or not.
    //
    // **Deliberately ignores `visible`**, and that is a correctness point rather
    // than a shortcut: QML reports EFFECTIVE visibility, and a TestCase is itself
    // invisible offscreen, so every descendant reads `visible === false`
    // whatever its own binding says — including text that renders
    // unconditionally. A walk filtered on `visible` would return nothing here
    // and every absence assertion below would pass vacuously.
    //
    // What that costs is stated rather than hidden: this instrument cannot tell
    // "the screen says X now" from "X is somewhere in the file". Where the
    // difference matters — a sentence that must appear in one state and not
    // another — the structural assertion is in `tst_feed_extent_claim.qml`,
    // which reasons about the governing ancestor instead.
    function everyStringOn(item) {
        var out = []
        collect(item, out)
        return out
    }

    function collect(item, out) {
        if (item === null || item === undefined)
            return
        if (item.text !== undefined && typeof item.text === "string"
            && item.text !== "")
            out.push(item.text)
        var kids = item.children
        for (var i = 0; i < kids.length; ++i)
            collect(kids[i], out)
    }

    // The one string containing `needle`, or "" if none. Fails loudly on two
    // matches: an assertion written against "the" sentence must not silently
    // pick one of a pair.
    function oneStringContaining(item, needle) {
        var all = everyStringOn(item)
        var hits = []
        for (var i = 0; i < all.length; ++i) {
            if (all[i].indexOf(needle) >= 0)
                hits.push(all[i])
        }
        compare(hits.length, 1,
                "expected exactly one string containing \"" + needle
                + "\", got " + hits.length + ": " + JSON.stringify(hits))
        return hits[0]
    }

    // ---- the corpus is real ----------------------------------------------

    // Every test below is an assertion about strings this walk returns, so a walk
    // that returns nothing would make the lot of them vacuous. That is not a
    // hypothetical failure mode in this repo: a sweep whose corpus-builder was
    // mutated to return "" left all thirteen of its tests passing.
    //
    // Floored by REACHING NAMED SENTENCES rather than by a count. A count-pin
    // fails on any re-layout and passes on a corpus that collected the wrong
    // text; naming the sentences is immune to both, and says which parts of the
    // screen the walk got as far as.
    function test_the_walk_reaches_every_state_this_file_asserts_about() {
        var empty = screenFor(0, false)
        var emptyText = everyStringOn(empty).join("\n")
        verify(emptyText.indexOf("You have not received anything") >= 0,
               "the walk must reach the empty state's heading")
        verify(emptyText.indexOf("Not newest first") >= 0,
               "and the body-level ordering sentence")
        verify(emptyText.indexOf("Pages are what this machine holds") >= 0,
               "and the paging locality sentence, which is built even when the "
               + "control is not shown — a `visible: false` layout child is "
               + "constructed, not omitted")
        empty.destroy()

        var failed = failedScreen()
        verify(everyStringOn(failed).join("\n").indexOf("could not be read") >= 0,
               "and the failed state's heading")
        failed.destroy()
    }

    // ---- the ordering sentence: the one line this change creates ----------

    // `tasks.md` leaves the `tests` row's question open: is the ordering
    // sentence worth pinning, given it was invisible to the suite in its
    // previous location too? It is, and this is why.
    //
    // The sentence is the whole reason the apparatus note could be deleted
    // rather than merely moved. `tasks.md` 2.3 records that `ON THIS ORDERING`
    // was the one obligation that did NOT survive the column's removal: the
    // ordering label "same order for everyone" is honest but NEUTRAL — it
    // declines to claim recency without denying it, and a reader meeting a forum
    // feed assumes newest-first unless told otherwise. Delete this sentence and
    // the screen is back to relying on the reader not to make the ordinary
    // assumption, with the label still there and every gate green.
    function test_the_feed_denies_being_newest_first_rather_than_being_silent() {
        var screen = screenFor(30, false)

        var sentence = oneStringContaining(screen, "Not newest first")

        // The denial is the load-bearing half, and it is asserted as a RELATION
        // rather than as the sentence: what must not happen is the screen
        // claiming, or failing to deny, an ordering by time.
        verify(sentence.indexOf("Timestamps do not reach this machine") >= 0,
               "the sentence must say WHY the order is not by time, or it reads "
               + "as a preference rather than as a limit; got: " + sentence)

        // And no OTHER string may undo it. A second element claiming recency
        // would leave this test green while the screen contradicted itself.
        var all = everyStringOn(screen)
        for (var i = 0; i < all.length; ++i) {
            if (all[i].indexOf("Not newest first") >= 0)
                continue
            var lower = all[i].toLowerCase()
            verify(lower.indexOf("newest first") < 0
                   && lower.indexOf("most recent first") < 0
                   && lower.indexOf("latest first") < 0,
                   "nothing else on the screen may claim a recency ordering the "
                   + "core cannot provide; found: \"" + all[i] + "\"")
        }
        screen.destroy()
    }

    // ---- the empty state: a fact about this copy, never about the Stoa ----

    // The strongest of review's four mutations, and the only one that was not
    // already filed: replacing this sentence with "This Stoa is empty." left
    // every test in the suite passing.
    //
    // `test_an_empty_store_is_the_ok_state_with_no_rows` in `tst_feed_states.qml`
    // covers the state and cannot see the claim — which is the point. An empty
    // read is the state where a peer is MOST tempted to speak for the Stoa,
    // because "nothing here" is the naive rendering of it.
    function test_the_empty_state_says_whose_copy_the_emptiness_is_a_fact_about() {
        var screen = screenFor(0, false)
        compare(screen.readState, "ok", "an empty store is a success")
        compare(screen.rows.length, 0)

        var sentence = oneStringContaining(screen, "The store was read without error")

        // Two halves, because either alone is satisfiable by a sentence that
        // misinforms. It must locate the emptiness in THIS COPY —
        verify(sentence.indexOf("your copy") >= 0,
               "the empty state must name whose copy the emptiness is a fact "
               + "about; got: " + sentence)
        // — and deny the reading a reader would otherwise take.
        verify(sentence.indexOf("not about the Stoa") >= 0,
               "and must deny the claim about the Stoa, which is the reading an "
               + "empty screen invites; got: " + sentence)
        // And say that other peers may hold what this one does not, which is the
        // fact that makes the denial true rather than merely cautious.
        verify(sentence.indexOf("Other peers may hold") >= 0,
               "and must say the posts may exist elsewhere; got: " + sentence)

        screen.destroy()
    }

    // The same obligation from the other side, and the assertion that actually
    // catches review's mutation.
    //
    // The test above finds a sentence by its opening and checks what it says; a
    // mutation that DELETES that sentence and writes "This Stoa is empty."
    // somewhere else would fail it on the `compare(hits.length, 1)` — but a
    // mutation that keeps the sentence and adds the forbidden claim beside it
    // would not. This sweeps every string instead, so no element anywhere on the
    // screen can make the global claim.
    //
    // **The set is not a list of forbidden phrasings** — that would be a
    // hand-maintained sweep list, stale the moment someone invents a twelfth way
    // to say it. It is the two ways the word "Stoa" can appear in a claim about
    // extent, which is the grammatical shape rather than the wording.
    function test_no_state_of_the_feed_claims_the_stoa_itself_is_empty() {
        var cases = [
            { rows: 0, hasMore: false, what: "the empty state" },
            { rows: 30, hasMore: true, what: "a full page with more to come" },
            { rows: 3, hasMore: false, what: "a partial page" }
        ]

        for (var c = 0; c < cases.length; ++c) {
            var screen = screenFor(cases[c].rows, cases[c].hasMore)
            var all = everyStringOn(screen)

            for (var i = 0; i < all.length; ++i) {
                var s = all[i].toLowerCase()

                // "this stoa is empty", "the stoa is empty", "stoa has no posts"
                // — an unqualified statement about the Stoa's contents. No peer
                // can make one: it holds what it was sent and knows nothing of
                // what it was not.
                verify(s.indexOf("stoa is empty") < 0,
                       cases[c].what + " must not claim the STOA is empty — no "
                       + "peer can know that. Found: \"" + all[i] + "\"")
                verify(s.indexOf("stoa has no") < 0,
                       cases[c].what + " must not claim what the Stoa does not "
                       + "hold. Found: \"" + all[i] + "\"")
            }
            screen.destroy()
        }
    }

    // ---- the failed state: not an empty one -------------------------------

    // Empty-versus-failed in the COPY rather than in the state machine. The
    // state half is covered; that a reader can TELL is not, and the two are
    // different properties — two states can be correctly distinct in `readState`
    // and indistinguishable on screen.
    function test_an_unreadable_store_says_it_is_not_an_empty_one() {
        var failed = failedScreen()
        compare(failed.readState, "failed")

        var sentence = oneStringContaining(failed, "This is not an empty Stoa")
        verify(sentence.indexOf("on disk and unreadable") >= 0,
               "the failed state must say the posts are held but unreadable, "
               + "which is what distinguishes it from holding none; got: "
               + sentence)

        // Core's own words reach the screen. A reworded failure is a failure
        // whose fix the reader cannot act on.
        verify(everyStringOn(failed).join("\n")
               .indexOf("the store could not be opened") >= 0,
               "core's error text must reach the screen verbatim")
        failed.destroy()
    }
}
