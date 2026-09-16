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
        verify(emptyText.indexOf(spec.denialAnchor) >= 0,
               "and the body-level ordering sentence")
        verify(emptyText.indexOf(spec.orderingLabel) >= 0,
               "and the ordering label itself, which is a Repeater delegate and "
               + "so is reached by a different path than the body Text above")
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

    // ---- the ordering label and its denial --------------------------------

    // **The whole of this section turns on one distinction, and stating it is
    // what keeps these tests from forbidding the screen's own label.**
    // `feed-view` permits *newest first* and forbids *most recent first*, which
    // are not two vocabularies but two READINGS of one superlative: the
    // positional reading (latest in the forum's order, which the descending
    // Lamport sort really does give) and the temporal one (latest by the clock,
    // which nothing here can give). So no assertion below may key on the word
    // "newest" — the label is allowed to say it and the denial is required to
    // say it.
    //
    // **The anchors are hardcoded and must not be updated to match a reword.**
    // If one stops matching, read the new copy and decide whether it still
    // discharges `feed-view` before touching this file. They are chosen to be
    // the parts of each string that carry its obligation: the label is what it
    // claims, and `denialAnchor` is the clause naming the FORBIDDEN reading,
    // which is the clause that cannot be dropped without the denial ceasing to
    // deny anything.
    readonly property string orderingLabel: "newest first"
    readonly property string denialAnchor: "not latest by the clock"

    // The old anchor, kept as a PROHIBITION rather than deleted. `feed-view`
    // gained a whole direction because the spec was once jointly satisfiable by
    // a screen labelling itself "newest first" while denying "Not newest first"
    // — an honest label and an honest denial that contradict each other, with
    // no way for a reader to tell which is live. This is the string that
    // reintroduces that defect, so it is named here and forbidden by name.
    readonly property string negatingDenial: "not newest first"

    // The denial must be present, must give the author-assertion as its reason,
    // and must NOT negate the label. `feed-view`'s scenarios "The denial is
    // present and gives the author-assertion as its reason" and "The denial does
    // not negate the ordering label", which are two halves of one string and so
    // are asserted together on it.
    //
    // **`oneStringContaining` uses `compare`, which ABORTS this function, so
    // everything below it is unreachable when the denial is missing — and that
    // is deliberate here rather than overlooked.** The previous version of this
    // file had the same shape and it cost real coverage: a cross-screen sweep
    // sat below this call, and while the anchor was stale the sweep never ran in
    // any invocation. The sweep is now `test_no_string_in_any_state_...`, its
    // own function, so no failure here can hide it. What remains below is five
    // assertions ABOUT THE STRING THIS CALL RETURNED; when that string does not
    // exist they have nothing to assert on, and the `compare` reports exactly
    // that. An assertion that must hold whether or not the denial is present
    // does not belong in this function.
    function test_the_denial_names_the_forbidden_reading_without_negating_the_label() {
        var screen = screenFor(30, false)

        var sentence = oneStringContaining(screen, spec.denialAnchor)

        // -- it denies the temporal reading, and denies the right thing --
        //
        // What must be denied is ordering BY THE DISPLAYED TIME. A sentence
        // that merely mentioned the clock without saying the feed is not
        // ordered by the time would satisfy an anchor and discharge nothing.
        verify(sentence.indexOf("not ordered by it") >= 0,
               "the denial must state that the feed is NOT ORDERED by the time "
               + "it displays, which is the temporal reading `feed-view` "
               + "forbids; got: " + sentence)

        // -- and it does not negate the label --
        //
        // Asserted on the lowercased sentence so a capitalised opening cannot
        // slip past: "Not newest first" is exactly the shape this blocks.
        verify(sentence.toLowerCase().indexOf(spec.negatingDenial) < 0,
               "the denial must name the forbidden READING, not the label's own "
               + "superlative -- \"" + spec.negatingDenial + "\" asserts the feed "
               + "is not the thing its label says it is; got: " + sentence)

        // -- the reason is the author's claim, not a missing field --
        //
        // **The reason this pins changed with the op clock, and the change is
        // the point.** It used to require "Timestamps do not reach this
        // machine" — a statement that no time was available, which promised a
        // newest-first feed once one was. A time is available now, inside the
        // signed op, and the feed is still not ordered by it: it is the
        // author's own claim, so ordering by it would reward lying. So the
        // sentence must now say the time EXISTS and is not trusted, and a
        // sentence reverting to "not yet" fails here — which is the direction
        // this assertion exists to block, because "not yet" is the softer and
        // more tempting wording.
        verify(sentence.indexOf("their author claimed") >= 0
               && sentence.indexOf("anyone could set") >= 0,
               "the sentence must say WHY the order is not by time, and the why "
               + "is now that the time is self-asserted rather than that none "
               + "exists; got: " + sentence)
        verify(sentence.indexOf("yet") < 0,
               "the denial is a design limit, not a missing feature -- 'yet' "
               + "promises a newest-first feed that is not coming; got: " + sentence)

        // -- and the property the old label asserted is still asserted --
        //
        // "Same order for everyone" claimed CONVERGENCE; "newest first" does
        // not. The property did not change, so something must still carry it,
        // and the denial's third sentence is where it went. Without this the
        // label swap silently drops a claim no other string makes.
        verify(sentence.indexOf("every peer computes identically") >= 0,
               "the convergence the old label asserted must survive the label "
               + "change -- the third sentence is where it now lives; got: "
               + sentence)

        screen.destroy()
    }

    // `feed-view`: "The ordering label names the order rather than a time".
    // Separate from the denial because they are two different strings reached by
    // two different paths — the label is a Repeater delegate over `orderings`,
    // the denial a plain body `Text` — and a test asserting both at once would
    // report the first failure and hide the second.
    function test_the_ordering_label_is_rendered_and_names_a_position_not_a_time() {
        var screen = screenFor(30, false)

        // The model is the source, and what reaches the screen is what matters:
        // a label correct in `orderings` but never rendered discharges nothing.
        compare(screen.orderings.length, 1,
                "core implements exactly one ordering")
        compare(screen.orderings[0].label, spec.orderingLabel,
                "`feed-view` makes \"" + spec.orderingLabel + "\" the label this "
                + "interface uses")

        var rendered = everyStringOn(screen)
        var seen = false
        for (var i = 0; i < rendered.length; ++i) {
            if (rendered[i] === spec.orderingLabel)
                seen = true
        }
        verify(seen, "the label must reach the screen, not merely the model; "
                     + "rendered strings: " + JSON.stringify(rendered))

        screen.destroy()
    }

    // `feed-view`: "The phrase asserting a clock order does not appear" and "No
    // second string undoes the denial" — swept across the populated, empty and
    // failed states, because the requirement is about every state and a sweep of
    // one state is a sweep that a second state's copy can walk straight past.
    //
    // **The forbidden set is the TEMPORAL reading, never the superlative.** It
    // cannot contain bare "newest first": the label says that and `feed-view`
    // permits it. "Most recent first" is forbidden by name in the spec, and it
    // is here for that reason rather than as one entry in a growing list of
    // phrasings — the two shapes below are *a superlative applied to the clock*
    // and *an explicit claim of chronology*, which is the grammatical form the
    // requirement is about.
    //
    // The denial is exempted BY ITS OWN ANCHOR, and that matters: it is the one
    // string on the screen that must talk about the clock, and an exemption
    // keyed on anything looser would exempt a second string that made the claim.
    function test_no_string_in_any_state_asserts_a_chronological_ordering() {
        var forbidden = [
            { needle: "most recent first",
              why: "forbidden by name -- a claim about instants, which the "
                   + "ordering carries none of" },
            { needle: "latest first",
              why: "the bare temporal superlative, with no 'in this forum's "
                   + "order' to make it positional" },
            { needle: "ordered by time", why: "an explicit claim of chronology" },
            { needle: "in time order", why: "an explicit claim of chronology" },
            { needle: "by date", why: "an explicit claim of chronology" },
            { needle: "when posts were written",
              why: "the reading `feed-view` names as forbidden: the ordering "
                   + "says an author had SEEN something, never when either was "
                   + "written" }
        ]

        var cases = [
            { make: function () { return screenFor(30, true) },
              what: "the populated state" },
            { make: function () { return screenFor(0, false) },
              what: "the empty state" },
            { make: function () { return failedScreen() },
              what: "the failed state" }
        ]

        for (var c = 0; c < cases.length; ++c) {
            var screen = cases[c].make()
            var all = everyStringOn(screen)

            // The sweep is only worth its name if it saw something. A corpus
            // that came back empty would pass every assertion below.
            verify(all.length > 0,
                   cases[c].what + " yielded no strings at all, so the sweep "
                   + "below would pass vacuously")

            for (var i = 0; i < all.length; ++i) {
                var lower = all[i].toLowerCase()

                // **The negation check runs on EVERY string, the denial
                // included, and the ordering of these two blocks is the whole
                // reason to say so.** The first version of this sweep put the
                // `continue` above both, and a mutation opening the denial with
                // "Not newest first" — the exact defect `feed-view` gained a
                // direction to stop — passed it: the exemption that lets the
                // denial talk about the clock also excused it from the one
                // check aimed at the denial. So the exemption is scoped to the
                // temporal list below and reaches nothing else.
                verify(lower.indexOf(spec.negatingDenial) < 0,
                       cases[c].what + " must not assert the feed is not newest "
                       + "first -- that negates the label rather than the "
                       + "temporal reading; found: \"" + all[i] + "\"")

                // The denial is the one string that must name the clock, so it
                // alone is exempt from the temporal list. Exempted by its own
                // anchor rather than by anything looser, so a SECOND string
                // making the claim is not exempted with it.
                if (lower.indexOf(spec.denialAnchor) >= 0)
                    continue
                for (var f = 0; f < forbidden.length; ++f) {
                    verify(lower.indexOf(forbidden[f].needle) < 0,
                           cases[c].what + " must not contain \""
                           + forbidden[f].needle + "\" -- " + forbidden[f].why
                           + "; found: \"" + all[i] + "\"")
                }
            }
            screen.destroy()
        }
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
